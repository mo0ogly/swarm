package main

import (
	"database/sql"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strconv"
	"strings"
	"syscall"
	"time"
)

const lifecycleMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS mission_lifecycle(
 work_id TEXT PRIMARY KEY REFERENCES works(id), archived_at TEXT NOT NULL,
 archive_path TEXT NOT NULL, archive_digest TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS mission_trash(
 work_id TEXT PRIMARY KEY, title TEXT NOT NULL, revision INTEGER NOT NULL,
 deleted_at TEXT NOT NULL, snapshot BLOB NOT NULL
);
CREATE TABLE IF NOT EXISTS lifecycle_receipts(
 event_id TEXT PRIMARY KEY, work_id TEXT NOT NULL, action TEXT NOT NULL,
 request_digest TEXT NOT NULL, created_at TEXT NOT NULL, response BLOB NOT NULL
);
PRAGMA user_version=13;
COMMIT;`

func migrateLifecycle(db *sql.DB) error {
	_, err := db.Exec(lifecycleMigration)
	return err
}

type LifecycleRequest struct {
	Schema        int    `json:"schema_version"`
	EventID       string `json:"event_id,omitempty"`
	Revision      int    `json:"expected_revision"`
	Action        string `json:"action"`
	RetentionDays int    `json:"retention_days,omitempty"`
	PreviewToken  string `json:"preview_token,omitempty"`
}

type LifecycleImpact struct {
	Kind  string `json:"kind"`
	Count int64  `json:"count"`
	Bytes int64  `json:"bytes"`
}

type LifecyclePreview struct {
	PurgeBefore   string            `json:"purge_before,omitempty"`
	Schema        int               `json:"schema_version"`
	WorkID        string            `json:"work_id"`
	Title         string            `json:"title"`
	Revision      int               `json:"revision"`
	Generation    int64             `json:"lifecycle_generation"`
	Action        string            `json:"action"`
	RetentionDays int               `json:"retention_days,omitempty"`
	Archived      bool              `json:"archived"`
	ArchivePath   string            `json:"archive_path,omitempty"`
	Data          []LifecycleImpact `json:"data"`
	Excluded      []string          `json:"excluded"`
	Blocked       []string          `json:"blocked,omitempty"`
	Confirmation  string            `json:"confirmation"`
	Token         string            `json:"token"`
}

type LifecycleReceipt struct {
	Schema      int               `json:"schema_version"`
	EventID     string            `json:"event_id"`
	WorkID      string            `json:"work_id"`
	Title       string            `json:"title"`
	Revision    int               `json:"revision"`
	Action      string            `json:"action"`
	At          string            `json:"at"`
	ArchivePath string            `json:"archive_path,omitempty"`
	Data        []LifecycleImpact `json:"data"`
	Restorable  bool              `json:"restorable"`
}

type LifecycleMission struct {
	WorkID     string `json:"work_id"`
	Title      string `json:"title"`
	Revision   int    `json:"revision"`
	State      string `json:"state"`
	StateLabel string `json:"state_label"`
	ChangedAt  string `json:"changed_at,omitempty"`
}

func lifecycleImpactLabel(kind string) string {
	labels := map[string]string{
		"mission": "Mission", "historique": "Historique des modifications",
		"agents": "Tentatives des agents", "journaux": "Journaux des agents",
		"journaux_agents": "Journaux des agents", "sorties_terminal": "Sorties du terminal",
		"pilotage_et_recus": "Pilotage et reçus", "assistant": "Échanges avec l’assistant",
		"echanges_assistant": "Échanges avec l’assistant", "preparations_liees": "Préparations liées",
	}
	if label := labels[kind]; label != "" {
		return label
	}
	return "Données internes"
}

// lifecycleMissions is the single inventory used by the web manager and the
// CLI. Deleted missions come only from the recoverable trash; archived missions
// remain in works and are labelled explicitly instead of being silently hidden.
func (s *Store) lifecycleMissions() ([]LifecycleMission, error) {
	rows, err := s.db.Query(`SELECT w.id,w.body,w.revision,l.archived_at
		FROM works w LEFT JOIN mission_lifecycle l ON l.work_id=w.id
		ORDER BY w.rowid DESC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := []LifecycleMission{}
	for rows.Next() {
		var id string
		var raw []byte
		var revision int
		var archived sql.NullString
		if err = rows.Scan(&id, &raw, &revision, &archived); err != nil {
			return nil, err
		}
		var work Work
		if err = json.Unmarshal(raw, &work); err != nil {
			return nil, err
		}
		item := LifecycleMission{WorkID: id, Title: work.Title, Revision: revision, State: "active", StateLabel: "Active"}
		if archived.Valid {
			item.State, item.StateLabel, item.ChangedAt = "archived", "Archivée", archived.String
		}
		out = append(out, item)
	}
	if err = rows.Err(); err != nil {
		return nil, err
	}
	trash, err := s.db.Query("SELECT work_id,title,revision,deleted_at FROM mission_trash ORDER BY deleted_at DESC")
	if err != nil {
		return nil, err
	}
	defer trash.Close()
	for trash.Next() {
		var item LifecycleMission
		if err = trash.Scan(&item.WorkID, &item.Title, &item.Revision, &item.ChangedAt); err != nil {
			return nil, err
		}
		item.State, item.StateLabel = "trash", "Dans la corbeille"
		out = append(out, item)
	}
	return out, trash.Err()
}

type lifecycleCell struct {
	Kind  string `json:"kind"`
	Value string `json:"value"`
}

type lifecycleTable struct {
	Name    string            `json:"name"`
	Columns []string          `json:"columns"`
	Rows    [][]lifecycleCell `json:"rows"`
}

type lifecycleSnapshot struct {
	Schema int              `json:"schema_version"`
	Tables []lifecycleTable `json:"tables"`
}

type lifecycleSpec struct {
	name  string
	where string
	args  []any
}

func lifecycleAction(action string) bool {
	switch action {
	case "archive", "restore", "purge", "delete":
		return true
	}
	return false
}

func lifecycleToken(p LifecyclePreview) string {
	p.Token = ""
	b, _ := json.Marshal(p)
	return hash(b)
}

func (s *Store) lifecyclePreview(work string, r LifecycleRequest) (LifecyclePreview, error) {
	var p LifecyclePreview
	if r.Schema != 1 || !lifecycleAction(r.Action) {
		return p, &CommandError{Code: "invalid_lifecycle", Message: "schema_version ou action de cycle de vie invalide"}
	}
	if r.Action == "purge" && (r.RetentionDays < 1 || r.RetentionDays > 36500) {
		return p, &CommandError{Code: "invalid_retention", Message: "retention_days doit être compris entre 1 et 36500 ; aucune purge globale implicite"}
	}
	w, err := s.get(work)
	if err != nil {
		if r.Action == "restore" {
			return s.trashRestorePreview(work, r)
		}
		return p, err
	}
	if r.Revision != w.Revision {
		return p, &CommandError{Code: "revision_conflict", Message: fmt.Sprintf("révision périmée : attendue %d, courante %d ; relire la mission", r.Revision, w.Revision), Retryable: true}
	}
	p = LifecyclePreview{Schema: 1, WorkID: w.ID, Title: w.Title, Revision: w.Revision, Action: r.Action,
		RetentionDays: r.RetentionDays, Data: []LifecycleImpact{}, Excluded: lifecycleExclusions()}
	if err = s.db.QueryRow("SELECT count(*) FROM lifecycle_receipts WHERE work_id=?", work).Scan(&p.Generation); err != nil {
		return p, err
	}
	var archivedPath string
	err = s.db.QueryRow("SELECT archive_path FROM mission_lifecycle WHERE work_id=?", work).Scan(&archivedPath)
	p.Archived = err == nil
	if err != nil && !errors.Is(err, sql.ErrNoRows) {
		return p, err
	}
	p.ArchivePath = archivedPath
	if r.Action == "archive" && p.Archived {
		p.Blocked = append(p.Blocked, "mission déjà archivée ; utiliser restore avant une nouvelle archive")
	}
	if r.Action == "restore" && !p.Archived {
		p.Blocked = append(p.Blocked, "mission ni archivée ni placée dans la corbeille")
	}
	if r.Action == "archive" || r.Action == "purge" || r.Action == "delete" {
		blocked, guardErr := s.lifecycleBlockers(s.db, work)
		if guardErr != nil {
			return p, guardErr
		}
		p.Blocked = append(p.Blocked, blocked...)
	}
	if r.Action == "purge" {
		p.PurgeBefore = time.Now().UTC().Truncate(24 * time.Hour).Add(-time.Duration(r.RetentionDays) * 24 * time.Hour).Format(time.RFC3339Nano)
		p.Data, err = lifecyclePurgeImpact(s.db, work, p.PurgeBefore)
	} else {
		p.Data, err = lifecycleMissionImpact(s.db, work)
	}
	if err != nil {
		return p, err
	}
	switch r.Action {
	case "archive":
		p.Confirmation = "Créer une archive ZIP exportable et masquer logiquement la mission, sans supprimer ses données."
	case "restore":
		p.Confirmation = "Rendre de nouveau active la mission archivée, ou restaurer toutes ses données depuis la corbeille."
	case "purge":
		p.Confirmation = fmt.Sprintf("Supprimer seulement les journaux et sorties datés de plus de %d jours ; reçus, verdicts et preuves restent conservés.", r.RetentionDays)
	case "delete":
		p.Confirmation = "Déplacer toutes les données internes de la mission dans la corbeille récupérable ; aucun fichier du projet ne sera effacé."
	}
	p.Token = lifecycleToken(p)
	return p, nil
}

func lifecycleExclusions() []string {
	return []string{
		"sources et fichiers du projet, y compris ceux simplement référencés par un rapport",
		"rapports, preuves et reçus de validation sur le système de fichiers",
		"données des autres missions",
	}
}

func (s *Store) trashRestorePreview(work string, r LifecycleRequest) (LifecyclePreview, error) {
	var p LifecyclePreview
	var title string
	var revision int
	var raw []byte
	err := s.db.QueryRow("SELECT title,revision,snapshot FROM mission_trash WHERE work_id=?", work).Scan(&title, &revision, &raw)
	if err != nil {
		return p, fmt.Errorf("mission %s introuvable dans les missions actives et la corbeille", work)
	}
	if r.Revision != revision {
		return p, &CommandError{Code: "revision_conflict", Message: fmt.Sprintf("révision de corbeille périmée : attendue %d, courante %d", r.Revision, revision), Retryable: true}
	}
	var snap lifecycleSnapshot
	if err = json.Unmarshal(raw, &snap); err != nil {
		return p, fmt.Errorf("corbeille illisible : %w", err)
	}
	p = LifecyclePreview{Schema: 1, WorkID: work, Title: title, Revision: revision, Action: "restore", Data: snapshotImpact(snap), Excluded: lifecycleExclusions(), Confirmation: "Restaurer toutes les données internes de la mission depuis la corbeille ; les fichiers du projet sont inchangés."}
	if err = s.db.QueryRow("SELECT count(*) FROM lifecycle_receipts WHERE work_id=?", work).Scan(&p.Generation); err != nil {
		return p, err
	}
	p.Token = lifecycleToken(p)
	return p, nil
}

func (s *Store) lifecycleApply(work string, r LifecycleRequest) (LifecycleReceipt, error) {
	var zero LifecycleReceipt
	if r.Schema != 1 || !safeName(r.EventID) || !lifecycleAction(r.Action) || r.PreviewToken == "" {
		return zero, &CommandError{Code: "invalid_lifecycle", Message: "schema_version, event_id, action et preview_token sont requis"}
	}
	raw, _ := json.Marshal(r)
	digest := hash(raw)
	if saved, found, err := s.lifecycleReceipt(r.EventID, work, r.Action, digest); err != nil || found {
		return saved, err
	}
	lock, err := s.lifecycleLock()
	if err != nil {
		return zero, err
	}
	defer func() { _ = syscall.Flock(int(lock.Fd()), syscall.LOCK_UN); _ = lock.Close() }()

	p, err := s.lifecyclePreview(work, r)
	if err != nil {
		return zero, err
	}
	if p.Token != r.PreviewToken {
		return zero, &CommandError{Code: "preview_conflict", Message: "l’aperçu confirmé n’est plus courant ; relire avant de confirmer", Retryable: true}
	}
	if len(p.Blocked) != 0 {
		return zero, &CommandError{Code: "lifecycle_blocked", Message: strings.Join(p.Blocked, " ; "), Retryable: true}
	}
	var archivePath string
	if r.Action == "archive" {
		archivePath, err = s.createLifecycleArchive(p)
		if err != nil {
			return zero, err
		}
	}
	receipt := LifecycleReceipt{Schema: 1, EventID: r.EventID, WorkID: work, Title: p.Title, Revision: p.Revision, Action: r.Action, At: now(), ArchivePath: archivePath, Data: p.Data, Restorable: r.Action == "archive" || r.Action == "delete" || r.Action == "restore"}
	tx, err := s.db.Begin()
	if err != nil {
		return zero, err
	}
	defer tx.Rollback()
	// Acquire SQLite's write lock before the final reads. This closes the window
	// in which another process could reserve an agent or change the revision
	// between the guard and the destructive statement.
	if r.Action == "restore" {
		_, err = tx.Exec("UPDATE mission_trash SET revision=revision WHERE work_id=?", work)
		if err == nil {
			_, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", work)
		}
	} else {
		_, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", work)
	}
	if err != nil {
		return zero, err
	}
	// Every guard and the revision are checked again in the same write transaction
	// as the mutation. BEGIN is serialized by the store's single connection.
	if r.Action == "restore" {
		err = s.applyRestore(tx, work, r, p, receipt)
	} else {
		err = s.checkLifecycleCurrent(tx, work, r, p)
		if err == nil {
			switch r.Action {
			case "archive":
				rel, _ := filepath.Rel(s.root, archivePath)
				var digestValue string
				digestValue, err = fileDigest(archivePath)
				if err == nil {
					_, err = tx.Exec("INSERT INTO mission_lifecycle(work_id,archived_at,archive_path,archive_digest) VALUES(?,?,?,?)", work, receipt.At, filepath.ToSlash(rel), digestValue)
				}
			case "purge":
				err = applyLifecyclePurge(tx, work, p.PurgeBefore)
			case "delete":
				err = applyLifecycleDelete(tx, work, p.Title, p.Revision, receipt.At)
			}
		}
	}
	if err != nil {
		if archivePath != "" {
			_ = os.Remove(archivePath)
		}
		return zero, err
	}
	response, _ := json.Marshal(receipt)
	if _, err = tx.Exec("INSERT INTO lifecycle_receipts(event_id,work_id,action,request_digest,created_at,response) VALUES(?,?,?,?,?,?)", r.EventID, work, r.Action, digest, receipt.At, response); err != nil {
		if archivePath != "" {
			_ = os.Remove(archivePath)
		}
		return zero, err
	}
	if err = tx.Commit(); err != nil {
		if archivePath != "" {
			_ = os.Remove(archivePath)
		}
		return zero, err
	}
	return receipt, nil
}

func (s *Store) lifecycleReceipt(event, work, action, digest string) (LifecycleReceipt, bool, error) {
	var r LifecycleReceipt
	var oldWork, oldAction, oldDigest string
	var raw []byte
	err := s.db.QueryRow("SELECT work_id,action,request_digest,response FROM lifecycle_receipts WHERE event_id=?", event).Scan(&oldWork, &oldAction, &oldDigest, &raw)
	if errors.Is(err, sql.ErrNoRows) {
		return r, false, nil
	}
	if err != nil {
		return r, false, err
	}
	if oldWork != work || oldAction != action || oldDigest != digest {
		return r, false, &CommandError{Code: "event_conflict", Message: "event_id déjà utilisé pour un contenu différent"}
	}
	err = json.Unmarshal(raw, &r)
	return r, true, err
}

func (s *Store) lifecycleLock() (*os.File, error) {
	f, err := os.OpenFile(filepath.Join(s.root, ".swarm", "automatic-validation.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return nil, err
	}
	if err = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		f.Close()
		return nil, &CommandError{Code: "active_control", Message: "un contrôle de validation est actif ; attendre sa fin", Retryable: true}
	}
	return f, nil
}

func (s *Store) checkLifecycleCurrent(tx *sql.Tx, work string, r LifecycleRequest, p LifecyclePreview) error {
	var revision int
	if err := tx.QueryRow("SELECT revision FROM works WHERE id=?", work).Scan(&revision); err != nil {
		return err
	}
	if revision != r.Revision || revision != p.Revision {
		return &CommandError{Code: "revision_conflict", Message: "mission modifiée pendant la confirmation ; relire l’aperçu", Retryable: true}
	}
	if r.Action == "purge" {
		current, impactErr := lifecyclePurgeImpact(tx, work, p.PurgeBefore)
		if impactErr != nil {
			return impactErr
		}
		before, _ := json.Marshal(p.Data)
		after, _ := json.Marshal(current)
		if string(before) != string(after) {
			return &CommandError{Code: "preview_conflict", Message: "historique modifié ; relire l’aperçu", Retryable: true}
		}
	}
	if r.Action == "archive" || r.Action == "purge" || r.Action == "delete" {
		blocked, err := s.lifecycleBlockers(tx, work)
		if err != nil {
			return err
		}
		if len(blocked) != 0 {
			return &CommandError{Code: "lifecycle_blocked", Message: strings.Join(blocked, " ; "), Retryable: true}
		}
	}
	return nil
}

func (s *Store) archived(work string) (bool, error) {
	var n int
	err := s.db.QueryRow("SELECT count(*) FROM mission_lifecycle WHERE work_id=?", work).Scan(&n)
	return n != 0, err
}

type queryRower interface {
	QueryRow(query string, args ...any) *sql.Row
}

func (s *Store) lifecycleBlockers(q queryRower, work string) ([]string, error) {
	checks := []struct {
		message string
		query   string
	}{
		{"agent actif ou intention de départ/arrêt réservée", "SELECT count(*) FROM agents WHERE work_id=? AND (status IN ('queued','starting','running','stopping') OR desired!='')"},
		{"budget d’agent réservé", "SELECT count(*) FROM reservations WHERE work_id=? AND state IN ('reserved','estimated')"},
		{"assistant ou contrôle actif", "SELECT count(*) FROM assist_turns WHERE work_id=? AND status IN ('pending','running')"},
		{"budget assistant réservé", "SELECT count(*) FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated')"},
		{"préparation active ou budget de préparation réservé", "SELECT count(*) FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')"},
		{"dialogue de préparation actif", "SELECT count(*) FROM preparation_turns WHERE preparation_id IN (SELECT id FROM preparations WHERE work_id=?) AND status IN ('pending','running')"},
	}
	out := []string{}
	for _, check := range checks {
		var n int
		if err := q.QueryRow(check.query, work).Scan(&n); err != nil {
			return nil, err
		}
		if n > 0 {
			out = append(out, check.message)
		}
	}
	var heartbeat, stopped string
	err := q.QueryRow("SELECT m.heartbeat_at,m.stopped_at FROM mission_supervision m LEFT JOIN cockpit_controls c ON c.work_id=m.work_id WHERE m.work_id=? AND coalesce(c.autonomy,'autonome') NOT IN ('manuel','assiste') AND coalesce(c.paused,0)=0 AND coalesce((SELECT json_extract(message,'$.enabled') FROM cockpit_events WHERE work_id=m.work_id AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1),0)=1", work).Scan(&heartbeat, &stopped)
	if err != nil && !errors.Is(err, sql.ErrNoRows) {
		return nil, err
	}
	if err == nil && stopped == "" {
		if at, parseErr := time.Parse(time.RFC3339Nano, heartbeat); parseErr == nil && time.Since(at) <= missionConductorStaleAfter && time.Since(at) >= -missionClockFutureTolerance {
			out = append(out, "conducteur de mission actif")
		}
	}
	return out, nil
}

func lifecycleMissionImpact(q queryRower, work string) ([]LifecycleImpact, error) {
	queries := []struct{ kind, query string }{
		{"mission", "SELECT count(*),coalesce(sum(length(body)),0) FROM works WHERE id=?"},
		{"historique", "SELECT count(*),coalesce(sum(length(payload)+length(request)),0) FROM events WHERE work_id=?"},
		{"agents", "SELECT count(*),coalesce(sum(length(body)+length(request)),0) FROM agents WHERE work_id=?"},
		{"journaux", "SELECT count(*),coalesce(sum(length(message)),0) FROM agent_logs WHERE agent_id IN (SELECT id FROM agents WHERE work_id=?)"},
		{"sorties_terminal", "SELECT count(*),coalesce(sum(length(data)),0) FROM terminal_events WHERE agent_id IN (SELECT id FROM agents WHERE work_id=?)"},
		{"pilotage_et_recus", "SELECT count(*),coalesce(sum(length(message)),0) FROM cockpit_events WHERE work_id=?"},
		{"assistant", "SELECT count(*),coalesce(sum(length(body)),0) FROM assist_turns WHERE work_id=?"},
		{"preparations_liees", "SELECT count(*),coalesce(sum(length(body)),0) FROM preparations WHERE work_id=?"},
	}
	out := make([]LifecycleImpact, 0, len(queries))
	for _, item := range queries {
		var impact LifecycleImpact
		impact.Kind = item.kind
		if err := q.QueryRow(item.query, work).Scan(&impact.Count, &impact.Bytes); err != nil {
			return nil, err
		}
		out = append(out, impact)
	}
	return out, nil
}

func lifecyclePurgeImpact(q queryRower, work, cutoff string) ([]LifecycleImpact, error) {
	queries := []struct {
		kind, query string
		args        []any
	}{
		{"journaux_agents", "SELECT count(*),coalesce(sum(length(message)),0) FROM agent_logs WHERE agent_id IN (SELECT id FROM agents WHERE work_id=?) AND julianday(at)<julianday(?)", []any{work, cutoff}},
		{"sorties_terminal", "SELECT count(*),coalesce(sum(length(data)),0) FROM terminal_events WHERE agent_id IN (SELECT id FROM agents WHERE work_id=? AND status NOT IN ('queued','starting','running','stopping') AND julianday(json_extract(body,'$.ended'))<julianday(?))", []any{work, cutoff}},
		{"echanges_assistant", "SELECT count(*),coalesce(sum(length(body)),0) FROM assist_turns WHERE work_id=? AND status NOT IN ('pending','running') AND julianday(created_at)<julianday(?)", []any{work, cutoff}},
	}
	out := []LifecycleImpact{}
	for _, item := range queries {
		v := LifecycleImpact{Kind: item.kind}
		if err := q.QueryRow(item.query, item.args...).Scan(&v.Count, &v.Bytes); err != nil {
			return nil, err
		}
		out = append(out, v)
	}
	return out, nil
}

func applyLifecyclePurge(tx *sql.Tx, work, cutoff string) error {
	statements := []struct {
		query string
		args  []any
	}{
		{"DELETE FROM agent_logs WHERE agent_id IN (SELECT id FROM agents WHERE work_id=?) AND julianday(at)<julianday(?)", []any{work, cutoff}},
		{"DELETE FROM terminal_events WHERE agent_id IN (SELECT id FROM agents WHERE work_id=? AND status NOT IN ('queued','starting','running','stopping') AND julianday(json_extract(body,'$.ended'))<julianday(?))", []any{work, cutoff}},
		{"DELETE FROM assist_reservations WHERE turn_id IN (SELECT id FROM assist_turns WHERE work_id=? AND status NOT IN ('pending','running') AND julianday(created_at)<julianday(?))", []any{work, cutoff}},
		{"DELETE FROM assist_turns WHERE work_id=? AND status NOT IN ('pending','running') AND julianday(created_at)<julianday(?)", []any{work, cutoff}},
	}
	for _, statement := range statements {
		if _, err := tx.Exec(statement.query, statement.args...); err != nil {
			return err
		}
	}
	return nil
}

func (s *Store) createLifecycleArchive(p LifecyclePreview) (string, error) {
	dir, err := s.lifecycleDir("archives")
	if err != nil {
		return "", err
	}
	path := filepath.Join(dir, fmt.Sprintf("%s-r%d-%s.zip", p.WorkID, p.Revision, p.Token[:12]))
	if err = s.export(p.WorkID, path); err != nil {
		return "", err
	}
	return path, nil
}

func (s *Store) lifecycleDir(name string) (string, error) {
	base := filepath.Join(s.root, ".swarm", "lifecycle")
	for _, dir := range []string{base, filepath.Join(base, name)} {
		if st, err := os.Lstat(dir); err == nil {
			if !st.IsDir() || st.Mode()&os.ModeSymlink != 0 {
				return "", fmt.Errorf("répertoire de cycle de vie non local : %s", dir)
			}
		} else if !os.IsNotExist(err) {
			return "", err
		} else if err = os.Mkdir(dir, 0700); err != nil {
			return "", err
		}
	}
	return filepath.Join(base, name), nil
}

func fileDigest(path string) (string, error) {
	b, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return hash(b), nil
}

func lifecycleSpecs(tx *sql.Tx, work string) ([]lifecycleSpec, error) {
	agents, preps := []string{}, []string{}
	for _, target := range []struct {
		query string
		out   *[]string
	}{{"SELECT id FROM agents WHERE work_id=? ORDER BY id", &agents}, {"SELECT id FROM preparations WHERE work_id=? ORDER BY id", &preps}} {
		rows, err := tx.Query(target.query, work)
		if err != nil {
			return nil, err
		}
		for rows.Next() {
			var id string
			if err = rows.Scan(&id); err != nil {
				rows.Close()
				return nil, err
			}
			*target.out = append(*target.out, id)
		}
		rows.Close()
	}
	agentWhere, agentArgs := inClause("agent_id", agents)
	prepWhere, prepArgs := inClause("preparation_id", preps)
	turnWhere := "preparation_id IN (SELECT id FROM preparations WHERE work_id=?)"
	return []lifecycleSpec{
		{"works", "id=?", []any{work}}, {"events", "work_id=?", []any{work}},
		{"agents", "work_id=?", []any{work}}, {"agent_logs", agentWhere, agentArgs},
		{"terminal_events", agentWhere, agentArgs}, {"agent_dialogue_turns", agentWhere, agentArgs},
		{"cockpit_events", "work_id=?", []any{work}}, {"cockpit_controls", "work_id=?", []any{work}},
		{"cockpit_tasks", "work_id=?", []any{work}}, {"decisions", "work_id=?", []any{work}},
		{"session_visits", "work_id=?", []any{work}}, {"budgets", "work_id=?", []any{work}},
		{"reservations", "work_id=?", []any{work}}, {"assist_turns", "work_id=?", []any{work}},
		{"assist_previews", "work_id=?", []any{work}}, {"assist_reservations", "work_id=?", []any{work}},
		{"mission_supervision", "work_id=?", []any{work}},
		{"mission_coordination_events", "work_id=?", []any{work}},
		{"agent_exchanges", "work_id=?", []any{work}},
		{"exchange_acknowledgements", "work_id=?", []any{work}},
		{"workspace_turns", "work_id=?", []any{work}},
		{"workspace_integrations", "work_id=?", []any{work}},
		{"planning_calls", "work_id=?", []any{work}}, {"managed_attempts", "work_id=?", []any{work}},
		{"preparations", "work_id=?", []any{work}}, {"preparation_documents", prepWhere, prepArgs},
		{"preparation_commands", prepWhere, prepArgs}, {"preparation_turns", turnWhere, []any{work}},
		{"preparation_reservations", "work_id=? OR " + prepWhere, append([]any{work}, prepArgs...)},
		{"preparation_launch_locks", "work_id=?", []any{work}}, {"mission_lifecycle", "work_id=?", []any{work}},
	}, nil
}

func inClause(column string, values []string) (string, []any) {
	if len(values) == 0 {
		return "1=0", nil
	}
	marks, args := make([]string, len(values)), make([]any, len(values))
	for i, value := range values {
		marks[i], args[i] = "?", value
	}
	return column + " IN (" + strings.Join(marks, ",") + ")", args
}

func snapshotMission(tx *sql.Tx, work string) (lifecycleSnapshot, error) {
	specs, err := lifecycleSpecs(tx, work)
	if err != nil {
		return lifecycleSnapshot{}, err
	}
	snap := lifecycleSnapshot{Schema: 1, Tables: []lifecycleTable{}}
	for _, spec := range specs {
		table, err := snapshotTable(tx, spec)
		if err != nil {
			return snap, err
		}
		snap.Tables = append(snap.Tables, table)
	}
	return snap, nil
}

func snapshotTable(tx *sql.Tx, spec lifecycleSpec) (lifecycleTable, error) {
	table := lifecycleTable{Name: spec.name, Rows: [][]lifecycleCell{}}
	info, err := tx.Query("PRAGMA table_info(" + spec.name + ")")
	if err != nil {
		return table, err
	}
	for info.Next() {
		var cid, notnull, pk int
		var name, kind string
		var defaultValue any
		if err = info.Scan(&cid, &name, &kind, &notnull, &defaultValue, &pk); err != nil {
			info.Close()
			return table, err
		}
		table.Columns = append(table.Columns, name)
	}
	info.Close()
	rows, err := tx.Query("SELECT * FROM "+spec.name+" WHERE "+spec.where, spec.args...)
	if err != nil {
		return table, err
	}
	defer rows.Close()
	for rows.Next() {
		values, refs := make([]any, len(table.Columns)), make([]any, len(table.Columns))
		for i := range values {
			refs[i] = &values[i]
		}
		if err = rows.Scan(refs...); err != nil {
			return table, err
		}
		encoded := make([]lifecycleCell, len(values))
		for i, value := range values {
			switch v := value.(type) {
			case nil:
				encoded[i] = lifecycleCell{Kind: "null"}
			case int64:
				encoded[i] = lifecycleCell{Kind: "int", Value: fmt.Sprint(v)}
			case float64:
				encoded[i] = lifecycleCell{Kind: "float", Value: fmt.Sprint(v)}
			case []byte:
				encoded[i] = lifecycleCell{Kind: "blob", Value: base64.StdEncoding.EncodeToString(v)}
			default:
				encoded[i] = lifecycleCell{Kind: "text", Value: fmt.Sprint(v)}
			}
		}
		table.Rows = append(table.Rows, encoded)
	}
	return table, rows.Err()
}

func snapshotImpact(s lifecycleSnapshot) []LifecycleImpact {
	out := []LifecycleImpact{}
	for _, table := range s.Tables {
		var size int64
		for _, row := range table.Rows {
			for _, cell := range row {
				size += int64(len(cell.Value))
			}
		}
		if len(table.Rows) != 0 {
			out = append(out, LifecycleImpact{Kind: table.Name, Count: int64(len(table.Rows)), Bytes: size})
		}
	}
	return out
}

func applyLifecycleDelete(tx *sql.Tx, work, title string, revision int, at string) error {
	snap, err := snapshotMission(tx, work)
	if err != nil {
		return err
	}
	raw, err := json.Marshal(snap)
	if err != nil {
		return err
	}
	if _, err = tx.Exec("INSERT INTO mission_trash(work_id,title,revision,deleted_at,snapshot) VALUES(?,?,?,?,?)", work, title, revision, at, raw); err != nil {
		return err
	}
	specs, err := lifecycleSpecs(tx, work)
	if err != nil {
		return err
	}
	for i := len(specs) - 1; i >= 0; i-- {
		spec := specs[i]
		if _, err = tx.Exec("DELETE FROM "+spec.name+" WHERE "+spec.where, spec.args...); err != nil {
			return fmt.Errorf("suppression atomique de %s : %w", spec.name, err)
		}
	}
	return nil
}

func (s *Store) applyRestore(tx *sql.Tx, work string, r LifecycleRequest, p LifecyclePreview, receipt LifecycleReceipt) error {
	var count int
	if err := tx.QueryRow("SELECT count(*) FROM works WHERE id=?", work).Scan(&count); err != nil {
		return err
	}
	if count != 0 {
		var archived int
		if err := tx.QueryRow("SELECT count(*) FROM mission_lifecycle WHERE work_id=?", work).Scan(&archived); err != nil {
			return err
		}
		if archived == 0 {
			return &CommandError{Code: "restore_conflict", Message: "mission active non archivée ; restauration refusée"}
		}
		var revision int
		if err := tx.QueryRow("SELECT revision FROM works WHERE id=?", work).Scan(&revision); err != nil {
			return err
		}
		if revision != r.Revision || p.Token != r.PreviewToken {
			return &CommandError{Code: "revision_conflict", Message: "mission modifiée pendant la confirmation", Retryable: true}
		}
		_, err := tx.Exec("DELETE FROM mission_lifecycle WHERE work_id=?", work)
		return err
	}
	var revision int
	var raw []byte
	if err := tx.QueryRow("SELECT revision,snapshot FROM mission_trash WHERE work_id=?", work).Scan(&revision, &raw); err != nil {
		return err
	}
	if revision != r.Revision || p.Token != r.PreviewToken {
		return &CommandError{Code: "revision_conflict", Message: "corbeille modifiée pendant la confirmation", Retryable: true}
	}
	var snap lifecycleSnapshot
	if err := json.Unmarshal(raw, &snap); err != nil || snap.Schema != 1 {
		return fmt.Errorf("instantané de corbeille invalide")
	}
	for _, table := range snap.Tables {
		if err := restoreTable(tx, table); err != nil {
			return fmt.Errorf("restauration atomique de %s : %w", table.Name, err)
		}
	}
	_, err := tx.Exec("DELETE FROM mission_trash WHERE work_id=?", work)
	return err
}

func restoreTable(tx *sql.Tx, table lifecycleTable) error {
	if len(table.Rows) == 0 {
		return nil
	}
	allowed := map[string]bool{}
	for _, spec := range []string{"works", "events", "agents", "agent_logs", "terminal_events", "agent_dialogue_turns", "cockpit_events", "cockpit_controls", "cockpit_tasks", "decisions", "session_visits", "budgets", "reservations", "assist_turns", "assist_previews", "assist_reservations", "mission_supervision", "mission_coordination_events", "agent_exchanges", "exchange_acknowledgements", "workspace_turns", "workspace_integrations", "planning_calls", "managed_attempts", "preparations", "preparation_documents", "preparation_commands", "preparation_turns", "preparation_reservations", "preparation_launch_locks", "mission_lifecycle"} {
		allowed[spec] = true
	}
	if !allowed[table.Name] {
		return fmt.Errorf("table non autorisée")
	}
	for _, column := range table.Columns {
		if !safeName(column) {
			return fmt.Errorf("colonne non autorisée")
		}
	}
	marks := strings.TrimRight(strings.Repeat("?,", len(table.Columns)), ",")
	query := "INSERT INTO " + table.Name + "(" + strings.Join(table.Columns, ",") + ") VALUES(" + marks + ")"
	for _, row := range table.Rows {
		if len(row) != len(table.Columns) {
			return fmt.Errorf("ligne incohérente")
		}
		args := make([]any, len(row))
		for i, cell := range row {
			switch cell.Kind {
			case "null":
				args[i] = nil
			case "int":
				value, err := strconv.ParseInt(cell.Value, 10, 64)
				if err != nil {
					return err
				}
				args[i] = value
			case "float":
				var value float64
				if _, err := fmt.Sscan(cell.Value, &value); err != nil {
					return err
				}
				args[i] = value
			case "blob":
				value, err := base64.StdEncoding.DecodeString(cell.Value)
				if err != nil {
					return err
				}
				args[i] = value
			case "text":
				args[i] = cell.Value
			default:
				return fmt.Errorf("type de cellule inconnu")
			}
		}
		if _, err := tx.Exec(query, args...); err != nil {
			return err
		}
	}
	return nil
}

func lifecycleActions() []string {
	out := []string{"archive", "restore", "purge", "delete"}
	sort.Strings(out)
	return out
}

func lifecycleCLI(s *Store, args []string, input string, asJSON bool, out io.Writer) error {
	if len(args) == 2 && args[0] == "lifecycle" && args[1] == "list" {
		missions, err := s.lifecycleMissions()
		if err != nil {
			return err
		}
		if asJSON {
			return printJSON(out, missions)
		}
		fmt.Fprintln(out, "Missions à gérer")
		for _, mission := range missions {
			fmt.Fprintf(out, "- %s — %s (%s), révision %d\n", mission.StateLabel, mission.Title, mission.WorkID, mission.Revision)
		}
		if len(missions) == 0 {
			fmt.Fprintln(out, "Aucune mission active, archivée ou récupérable.")
		}
		fmt.Fprintln(out, "Avant toute action : lifecycle preview ; l’aperçu seul ne modifie rien.")
		return nil
	}
	if len(args) != 4 || args[0] != "lifecycle" || (args[1] != "preview" && args[1] != "apply") || input == "" {
		return fmt.Errorf("usage : lifecycle list | lifecycle preview|apply WORK archive|restore|purge|delete --input requete.json")
	}
	if !lifecycleAction(args[3]) {
		return fmt.Errorf("action inconnue ; valeurs : %s", strings.Join(lifecycleActions(), ", "))
	}
	b, err := readInput(input)
	if err != nil {
		return err
	}
	var request LifecycleRequest
	if err = strict(b, &request); err != nil {
		return err
	}
	if request.Action == "" {
		request.Action = args[3]
	}
	if request.Action != args[3] {
		return fmt.Errorf("action du document différente de la commande")
	}
	if args[1] == "preview" {
		preview, previewErr := s.lifecyclePreview(args[2], request)
		if previewErr != nil {
			return previewErr
		}
		if asJSON {
			return printJSON(out, preview)
		}
		fmt.Fprintf(out, "Aperçu %s — %s (%s), révision %d\n", preview.Action, preview.Title, preview.WorkID, preview.Revision)
		for _, item := range preview.Data {
			fmt.Fprintf(out, "- %s : %d élément(s), %d octet(s)\n", lifecycleImpactLabel(item.Kind), item.Count, item.Bytes)
		}
		for _, exclusion := range preview.Excluded {
			fmt.Fprintln(out, "- Exclu : "+exclusion)
		}
		for _, blocker := range preview.Blocked {
			fmt.Fprintln(out, "- Refus actuel : "+blocker)
		}
		fmt.Fprintln(out, "Effet : "+preview.Confirmation)
		fmt.Fprintln(out, "Jeton à confirmer : "+preview.Token)
		return nil
	}
	receipt, err := s.lifecycleApply(args[2], request)
	if err != nil {
		return err
	}
	if asJSON {
		return printJSON(out, receipt)
	}
	labels := map[string]string{"archive": "Archivage", "restore": "Restauration", "purge": "Purge de l’historique", "delete": "Mise en corbeille"}
	fmt.Fprintf(out, "%s appliqué à %s ; reçu %s ; récupérable : %t\n", labels[receipt.Action], receipt.WorkID, receipt.EventID, receipt.Restorable)
	return nil
}
