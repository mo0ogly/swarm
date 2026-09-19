package main

import (
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const workspaceCoordinationMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS workspace_turns(
 seq INTEGER PRIMARY KEY AUTOINCREMENT,
 work_id TEXT NOT NULL REFERENCES works(id),
 task_id TEXT NOT NULL,
 workspace TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('waiting','acquired','released')),
 agent_id TEXT NOT NULL DEFAULT '',
 requested_at TEXT NOT NULL,
 acquired_at TEXT NOT NULL DEFAULT '',
 released_at TEXT NOT NULL DEFAULT '',
 UNIQUE(work_id,task_id)
);
CREATE INDEX IF NOT EXISTS workspace_turn_waiting ON workspace_turns(state,seq);
CREATE TABLE IF NOT EXISTS workspace_integrations(
 id TEXT PRIMARY KEY,
 request_hash TEXT NOT NULL,
 work_id TEXT NOT NULL REFERENCES works(id),
 exchange_id TEXT NOT NULL REFERENCES agent_exchanges(id),
 canonical_workspace TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('running','integrated','conflict','failed')),
 requested_at TEXT NOT NULL,
 finished_at TEXT NOT NULL DEFAULT '',
 detail BLOB NOT NULL
);
CREATE INDEX IF NOT EXISTS workspace_integration_active ON workspace_integrations(state,canonical_workspace);
PRAGMA user_version=16;
COMMIT;`

func migrateWorkspaceCoordination(db *sql.DB) error {
	_, err := db.Exec(workspaceCoordinationMigration)
	return err
}

type WorkspaceTurn struct {
	Seq         int64  `json:"seq"`
	WorkID      string `json:"work_id"`
	TaskID      string `json:"task_id"`
	Workspace   string `json:"workspace"`
	State       string `json:"state"`
	AgentID     string `json:"agent_id,omitempty"`
	RequestedAt string `json:"requested_at"`
	AcquiredAt  string `json:"acquired_at,omitempty"`
	ReleasedAt  string `json:"released_at,omitempty"`
}

// enqueueWorkspaceTurn keeps the first request position while it waits. A new
// attempt gets a new position only after the previous turn was released.
func (s *Store) enqueueWorkspaceTurn(work, task, workspace string) error {
	tx, err := s.db.Begin()
	if err != nil {
		return err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("DELETE FROM workspace_turns WHERE work_id=? AND task_id=? AND state='released'", work, task); err != nil {
		return err
	}
	if _, err = tx.Exec(`INSERT INTO workspace_turns(work_id,task_id,workspace,state,requested_at)
 VALUES(?,?,?,'waiting',?) ON CONFLICT(work_id,task_id) DO NOTHING`, work, task, workspace, now()); err != nil {
		return err
	}
	return tx.Commit()
}

func scanWorkspaceTurn(row interface{ Scan(...any) error }) (WorkspaceTurn, error) {
	var turn WorkspaceTurn
	err := row.Scan(&turn.Seq, &turn.WorkID, &turn.TaskID, &turn.Workspace, &turn.State,
		&turn.AgentID, &turn.RequestedAt, &turn.AcquiredAt, &turn.ReleasedAt)
	return turn, err
}

func (s *Store) workspaceTurns() ([]WorkspaceTurn, error) {
	rows, err := s.db.Query(`SELECT seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at
		FROM workspace_turns ORDER BY seq`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	turns := []WorkspaceTurn{}
	for rows.Next() {
		turn, scanErr := scanWorkspaceTurn(rows)
		if scanErr != nil {
			return nil, scanErr
		}
		turns = append(turns, turn)
	}
	return turns, rows.Err()
}

func (s *Store) releaseObsoleteWorkspaceTurns(work string, retained map[string]bool) error {
	turns, err := s.workspaceTurns()
	if err != nil {
		return err
	}
	for _, turn := range turns {
		if turn.WorkID != work || turn.State != "waiting" || retained[turn.TaskID] {
			continue
		}
		if _, err = s.db.Exec(`UPDATE workspace_turns SET state='released',released_at=? WHERE seq=? AND state='waiting'`, now(), turn.Seq); err != nil {
			return err
		}
	}
	return nil
}

func workspaceTurnBlockers(turns []WorkspaceTurn, work string) map[string]string {
	blocked := map[string]string{}
	for _, candidate := range turns {
		if candidate.WorkID != work || candidate.State != "waiting" {
			continue
		}
		for _, older := range turns {
			if older.Seq >= candidate.Seq || older.State == "released" || !workspaceOverlap(older.Workspace, candidate.Workspace) {
				continue
			}
			blocked[candidate.TaskID] = fmt.Sprintf("file équitable : tour %d en attente derrière %s/%s (tour %d)", candidate.Seq, older.WorkID, older.TaskID, older.Seq)
			break
		}
	}
	return blocked
}

func claimWorkspaceTurn(tx *sql.Tx, work, task, workspace, agent string) error {
	turn, err := scanWorkspaceTurn(tx.QueryRow(`SELECT seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at
		FROM workspace_turns WHERE work_id=? AND task_id=?`, work, task))
	if err == sql.ErrNoRows {
		if _, err = tx.Exec(`INSERT INTO workspace_turns(work_id,task_id,workspace,state,requested_at) VALUES(?,?,?,'waiting',?)`, work, task, workspace, now()); err != nil {
			return err
		}
		turn, err = scanWorkspaceTurn(tx.QueryRow(`SELECT seq,work_id,task_id,workspace,state,agent_id,requested_at,acquired_at,released_at
			FROM workspace_turns WHERE work_id=? AND task_id=?`, work, task))
	}
	if err != nil {
		return fmt.Errorf("tour d’espace absent : réévaluer la file avant le départ")
	}
	if turn.State != "waiting" || turn.Workspace != workspace {
		return fmt.Errorf("tour d’espace non disponible : réévaluer la file avant le départ")
	}
	var olderWork, olderTask string
	err = tx.QueryRow(`SELECT work_id,task_id FROM workspace_turns
		WHERE seq<? AND state IN ('waiting','acquired')
		AND (workspace=? OR instr(workspace, ? || '/')=1 OR instr(?, workspace || '/')=1)
		ORDER BY seq LIMIT 1`, turn.Seq, workspace, workspace, workspace).Scan(&olderWork, &olderTask)
	if err == nil {
		return fmt.Errorf("file équitable : %s/%s passe avant %s/%s", olderWork, olderTask, work, task)
	}
	if err != sql.ErrNoRows {
		return err
	}
	result, err := tx.Exec(`UPDATE workspace_turns SET state='acquired',agent_id=?,acquired_at=?
		WHERE seq=? AND state='waiting'`, agent, now(), turn.Seq)
	if err != nil {
		return err
	}
	if changed, _ := result.RowsAffected(); changed != 1 {
		return fmt.Errorf("tour d’espace déjà pris")
	}
	return nil
}

func (s *Store) releaseWorkspaceTurn(agent Agent) error {
	result, err := s.db.Exec(`UPDATE workspace_turns SET state='released',released_at=?
		WHERE work_id=? AND task_id=? AND agent_id=? AND state='acquired'`, now(), agent.WorkID, agent.TaskID, agent.ID)
	if err != nil {
		return err
	}
	changed, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if agent.Origin == originConductor && changed != 1 {
		var state, owner string
		err = s.db.QueryRow(`SELECT state,agent_id FROM workspace_turns WHERE work_id=? AND task_id=?`, agent.WorkID, agent.TaskID).Scan(&state, &owner)
		if err == sql.ErrNoRows { // compatibility with attempts created before v16
			return nil
		}
		if err != nil {
			return err
		}
		if state != "released" || owner != agent.ID {
			return fmt.Errorf("libération d’espace non vérifiée pour %s", agent.ID)
		}
	}
	return nil
}

type IntegrationArtifact struct {
	Source       string `json:"source"`
	Target       string `json:"target"`
	BaseSHA256   string `json:"base_sha256,omitempty"`
	ResultSHA256 string `json:"result_sha256"`
}

type IntegrationRequest struct {
	Schema             int                   `json:"schema_version"`
	EventID            string                `json:"event_id"`
	ExchangeID         string                `json:"exchange_id"`
	CanonicalWorkspace string                `json:"canonical_workspace"`
	Artifacts          []IntegrationArtifact `json:"artifacts"`
}

type IntegrationReceipt struct {
	ID                 string                `json:"id"`
	WorkID             string                `json:"work_id"`
	ExchangeID         string                `json:"exchange_id"`
	CanonicalWorkspace string                `json:"canonical_workspace"`
	State              string                `json:"state"`
	RequestedAt        string                `json:"requested_at"`
	FinishedAt         string                `json:"finished_at,omitempty"`
	Artifacts          []IntegrationArtifact `json:"artifacts"`
	Conflicts          []string              `json:"conflicts,omitempty"`
	Error              string                `json:"error,omitempty"`
}

func validSHA256(value string) bool {
	b, err := hex.DecodeString(value)
	return err == nil && len(b) == sha256.Size && value == strings.ToLower(value)
}

func regularDigest(path string) (string, error) {
	f, err := os.Open(path)
	if err != nil {
		return "", err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil || !info.Mode().IsRegular() {
		return "", fmt.Errorf("fichier régulier requis : %s", path)
	}
	h := sha256.New()
	if _, err = io.Copy(h, io.LimitReader(f, 64<<20)); err != nil {
		return "", err
	}
	if info.Size() > 64<<20 {
		return "", fmt.Errorf("fichier supérieur à 64 Mio : %s", path)
	}
	return hex.EncodeToString(h.Sum(nil)), nil
}

func integrationTarget(root, relative string) (string, error) {
	if relative == "" || filepath.IsAbs(relative) || strings.ContainsRune(relative, '\x00') {
		return "", fmt.Errorf("cible relative requise")
	}
	clean := filepath.Clean(relative)
	if clean == "." || clean == ".." || strings.HasPrefix(clean, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("cible hors espace canonique : %s", relative)
	}
	parent, err := filepath.EvalSymlinks(filepath.Join(root, filepath.Dir(clean)))
	if err != nil {
		return "", fmt.Errorf("dossier cible absent : %s", relative)
	}
	rel, err := filepath.Rel(root, parent)
	if err != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("cible hors espace canonique : %s", relative)
	}
	return filepath.Join(parent, filepath.Base(clean)), nil
}

func integrationConflicts(artifacts []IntegrationArtifact, targets []string) []string {
	conflicts := []string{}
	for i, artifact := range artifacts {
		got, digestErr := regularDigest(targets[i])
		switch {
		case os.IsNotExist(digestErr) && artifact.BaseSHA256 == "":
		case digestErr != nil:
			conflicts = append(conflicts, artifact.Target+" : état cible illisible ou non régulier")
		case artifact.BaseSHA256 == "":
			conflicts = append(conflicts, artifact.Target+" : la cible existe déjà")
		case got != artifact.BaseSHA256:
			conflicts = append(conflicts, artifact.Target+" : empreinte de base différente (observée "+got+")")
		}
	}
	return conflicts
}

func (s *Store) integrationReceipt(id string) (IntegrationReceipt, string, error) {
	var receipt IntegrationReceipt
	var detail []byte
	var requestHash string
	err := s.db.QueryRow(`SELECT request_hash,work_id,exchange_id,canonical_workspace,state,requested_at,finished_at,detail
		FROM workspace_integrations WHERE id=?`, id).Scan(&requestHash, &receipt.WorkID, &receipt.ExchangeID,
		&receipt.CanonicalWorkspace, &receipt.State, &receipt.RequestedAt, &receipt.FinishedAt, &detail)
	receipt.ID = id
	if err == nil {
		err = json.Unmarshal(detail, &receipt)
	}
	return receipt, requestHash, err
}

func (s *Store) workspaceIntegrationReceipts(work string) ([]IntegrationReceipt, error) {
	rows, err := s.db.Query(`SELECT detail FROM workspace_integrations WHERE work_id=? ORDER BY rowid`, work)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	receipts := []IntegrationReceipt{}
	for rows.Next() {
		var detail []byte
		var receipt IntegrationReceipt
		if err = rows.Scan(&detail); err != nil {
			return nil, err
		}
		if err = json.Unmarshal(detail, &receipt); err != nil {
			return nil, err
		}
		receipts = append(receipts, receipt)
	}
	return receipts, rows.Err()
}

// integrateWorkspace applies an explicit, SHA-pinned handoff. It never creates
// worktrees and checks every target before the first write. A conflict receipt
// retains the observed state and leaves all canonical files untouched.
func (s *Store) integrateWorkspace(work string, request IntegrationRequest) (IntegrationReceipt, bool, error) {
	var empty IntegrationReceipt
	if request.Schema != 1 || !safeName(request.EventID) || !safeName(request.ExchangeID) || len(request.Artifacts) == 0 || len(request.Artifacts) > 32 {
		return empty, false, fmt.Errorf("requête d’intégration invalide")
	}
	raw, _ := json.Marshal(request)
	requestHash := hash(raw)
	if existing, storedHash, err := s.integrationReceipt(request.EventID); err == nil {
		if storedHash != requestHash || existing.WorkID != work {
			return empty, false, fmt.Errorf("event_id déjà utilisé avec une autre intégration")
		}
		return existing, false, nil
	} else if err != sql.ErrNoRows {
		return empty, false, err
	}
	exchange, err := s.agentExchange(request.ExchangeID)
	if err != nil || exchange.WorkID != work || exchange.Kind != "handoff" || exchange.ResultState != "completed" {
		return empty, false, fmt.Errorf("remise terminée et attribuable requise")
	}
	w, err := s.get(work)
	if err != nil {
		return empty, false, err
	}
	if _, _, err = s.validateExchangeAgent(work, &w, exchange.SourceAgent, exchange.SourceTask, exchange.SourceAttempt); err != nil {
		return empty, false, err
	}
	canonical, err := resolveWorkspace(s.root, request.CanonicalWorkspace)
	if err != nil {
		return empty, false, err
	}
	allowed := map[string]string{}
	for _, artifact := range exchange.Artifacts {
		allowed[artifact.Path] = artifact.SHA256
	}
	targets := make([]string, len(request.Artifacts))
	seenTargets := map[string]bool{}
	for i, artifact := range request.Artifacts {
		if !validSHA256(artifact.ResultSHA256) || (artifact.BaseSHA256 != "" && !validSHA256(artifact.BaseSHA256)) || allowed[artifact.Source] != artifact.ResultSHA256 {
			return empty, false, fmt.Errorf("artefact non couvert par la remise : %s", artifact.Source)
		}
		if _, err = s.artifactDigest(ExchangeArtifact{Path: artifact.Source, SHA256: artifact.ResultSHA256}); err != nil {
			return empty, false, err
		}
		targets[i], err = integrationTarget(canonical, artifact.Target)
		if err != nil {
			return empty, false, err
		}
		if seenTargets[targets[i]] {
			return empty, false, fmt.Errorf("cible dupliquée : %s", artifact.Target)
		}
		seenTargets[targets[i]] = true
	}
	receipt := IntegrationReceipt{ID: request.EventID, WorkID: work, ExchangeID: request.ExchangeID,
		CanonicalWorkspace: canonical, State: "running", RequestedAt: now(), Artifacts: request.Artifacts}
	detail, _ := json.Marshal(receipt)
	// A concurrent integration waits for its bounded serialized turn instead of
	// failing or writing alongside the holder. A crashed holder stays visible as
	// running and is never stolen automatically.
	deadline := time.Now().Add(5 * time.Second)
	for {
		tx, beginErr := s.db.Begin()
		if beginErr != nil {
			return empty, false, beginErr
		}
		var busyID string
		busyErr := tx.QueryRow("SELECT id FROM agents WHERE status IN ('queued','starting','running','stopping') AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1) LIMIT 1", canonical, canonical, canonical).Scan(&busyID)
		if busyErr == nil {
			tx.Rollback()
			return empty, false, fmt.Errorf("espace canonique occupé par l’agent %s", busyID)
		}
		if busyErr != sql.ErrNoRows {
			tx.Rollback()
			return empty, false, busyErr
		}
		var activeID string
		activeErr := tx.QueryRow(`SELECT id FROM workspace_integrations WHERE state='running'
			AND (canonical_workspace=? OR instr(canonical_workspace, ? || '/')=1 OR instr(?, canonical_workspace || '/')=1)
			ORDER BY rowid LIMIT 1`, canonical, canonical, canonical).Scan(&activeID)
		if activeErr == nil {
			_ = tx.Rollback()
			if time.Now().After(deadline) {
				return empty, false, fmt.Errorf("intégration canonique toujours active après attente bornée : %s", activeID)
			}
			time.Sleep(10 * time.Millisecond)
			continue
		}
		if activeErr != sql.ErrNoRows {
			_ = tx.Rollback()
			return empty, false, activeErr
		}
		// The source attempt was validated against this work revision before
		// waiting. Bind that validation to acquiring the integration slot.
		var currentRevision int
		if revisionErr := tx.QueryRow("SELECT revision FROM works WHERE id=?", work).Scan(&currentRevision); revisionErr != nil || currentRevision != w.Revision {
			tx.Rollback()
			return empty, false, fmt.Errorf("mission modifiée pendant l’attente d’intégration ; relire la remise")
		}
		_, insertErr := tx.Exec(`INSERT INTO workspace_integrations(id,request_hash,work_id,exchange_id,canonical_workspace,state,requested_at,finished_at,detail)
			VALUES(?,?,?,?,?,?,?,?,?)`, receipt.ID, requestHash, work, request.ExchangeID, canonical, receipt.State, receipt.RequestedAt, receipt.FinishedAt, detail)
		if insertErr != nil {
			_ = tx.Rollback()
			return empty, false, insertErr
		}
		if commitErr := tx.Commit(); commitErr != nil {
			return empty, false, commitErr
		}
		break
	}
	// Re-read every canonical target only after acquiring the serialized slot.
	// This closes the check/write race with the preceding integration.
	if conflicts := integrationConflicts(request.Artifacts, targets); len(conflicts) > 0 {
		receipt.State, receipt.FinishedAt, receipt.Conflicts = "conflict", now(), conflicts
		detail, _ = json.Marshal(receipt)
		_, updateErr := s.db.Exec(`UPDATE workspace_integrations SET state='conflict',finished_at=?,detail=? WHERE id=? AND state='running'`, receipt.FinishedAt, detail, receipt.ID)
		if updateErr != nil {
			return receipt, true, updateErr
		}
		return receipt, true, &CommandError{Code: "conflict", Message: "intégration arrêtée avant écriture : la base canonique a changé ; consulter le reçu conservé"}
	}
	for i, artifact := range request.Artifacts {
		source, _ := filepath.EvalSymlinks(filepath.Join(s.root, artifact.Source))
		input, openErr := os.Open(source)
		if openErr != nil {
			receipt.State, receipt.Error = "failed", openErr.Error()
			break
		}
		tmp, createErr := os.CreateTemp(filepath.Dir(targets[i]), ".swarm-integration-*")
		if createErr == nil {
			digest := sha256.New()
			var copied int64
			copied, createErr = io.Copy(io.MultiWriter(tmp, digest), io.LimitReader(input, (64<<20)+1))
			if createErr == nil && (copied > 64<<20 || hex.EncodeToString(digest.Sum(nil)) != artifact.ResultSHA256) {
				createErr = fmt.Errorf("source modifiée pendant intégration : %s", artifact.Source)
			}
		}
		input.Close()
		if tmp != nil {
			if syncErr := tmp.Sync(); createErr == nil {
				createErr = syncErr
			}
			if closeErr := tmp.Close(); createErr == nil {
				createErr = closeErr
			}
		}
		if createErr == nil {
			createErr = os.Rename(tmp.Name(), targets[i])
		}
		if createErr != nil {
			if tmp != nil {
				_ = os.Remove(tmp.Name())
			}
			receipt.State, receipt.Error = "failed", createErr.Error()
			break
		}
	}
	if receipt.State == "running" {
		receipt.State = "integrated"
	}
	receipt.FinishedAt = now()
	detail, _ = json.Marshal(receipt)
	_, updateErr := s.db.Exec(`UPDATE workspace_integrations SET state=?,finished_at=?,detail=? WHERE id=? AND state='running'`, receipt.State, receipt.FinishedAt, detail, receipt.ID)
	if updateErr != nil {
		return receipt, true, updateErr
	}
	if receipt.State == "failed" {
		return receipt, true, fmt.Errorf("intégration incomplète : %s", receipt.Error)
	}
	return receipt, true, nil
}
