package main

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"sort"
	"strings"
)

const graphDraftMigration = `BEGIN;
CREATE TABLE IF NOT EXISTS graph_drafts(
 id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), base_revision INTEGER NOT NULL,
 revision INTEGER NOT NULL, status TEXT NOT NULL, actor TEXT NOT NULL, operations BLOB NOT NULL,
 content_digest TEXT NOT NULL, preview_token TEXT NOT NULL DEFAULT '', created TEXT NOT NULL, updated TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS graph_drafts_work ON graph_drafts(work_id, updated);
CREATE TABLE IF NOT EXISTS graph_draft_authorizations(
 work_id TEXT NOT NULL REFERENCES works(id), actor TEXT NOT NULL, revision INTEGER NOT NULL,
 prepare INTEGER NOT NULL, apply INTEGER NOT NULL, scopes BLOB NOT NULL, updated TEXT NOT NULL,
 PRIMARY KEY(work_id, actor));
PRAGMA user_version=24;
COMMIT;`

type GraphDraftOperation struct {
	Kind         string `json:"kind"`
	Prerequisite string `json:"prerequisite"`
	Dependent    string `json:"dependent"`
}

type GraphDraft struct {
	Schema        int                   `json:"schema_version"`
	ID            string                `json:"draft_id"`
	WorkID        string                `json:"work_id"`
	BaseRevision  int                   `json:"base_revision"`
	Revision      int                   `json:"revision"`
	Status        string                `json:"status"`
	Actor         string                `json:"actor"`
	Operations    []GraphDraftOperation `json:"operations"`
	ContentDigest string                `json:"content_digest"`
	PreviewToken  string                `json:"preview_token,omitempty"`
	Created       string                `json:"created_at"`
	Updated       string                `json:"updated_at"`
}

type GraphDraftSaveRequest struct {
	Schema                int                   `json:"schema_version"`
	WorkID                string                `json:"work_id"`
	DraftID               string                `json:"draft_id,omitempty"`
	ExpectedRevision      int                   `json:"expected_revision"`
	ExpectedDraftRevision int                   `json:"expected_draft_revision,omitempty"`
	Operations            []GraphDraftOperation `json:"operations"`
}

type GraphDraftPreviewRequest struct {
	Schema           int    `json:"schema_version"`
	WorkID           string `json:"work_id"`
	DraftID          string `json:"draft_id"`
	ExpectedRevision int    `json:"expected_revision"`
}

type GraphDraftPreview struct {
	Schema           int                       `json:"schema_version"`
	WorkID           string                    `json:"work_id"`
	DraftID          string                    `json:"draft_id"`
	BaseRevision     int                       `json:"base_revision"`
	ContentDigest    string                    `json:"content_digest"`
	PreviewToken     string                    `json:"preview_token"`
	Operations       []GraphDraftOperation     `json:"operations"`
	AffectedTasks    []string                  `json:"affected_tasks"`
	RequiredRight    string                    `json:"required_right"`
	NoImplicitLaunch bool                      `json:"no_implicit_launch"`
	ProofImpact      GraphDraftProofProjection `json:"proof_impact"`
}

type GraphDraftApplyRequest struct {
	Schema           int    `json:"schema_version"`
	WorkID           string `json:"work_id"`
	DraftID          string `json:"draft_id"`
	EventID          string `json:"event_id"`
	ExpectedRevision int    `json:"expected_revision"`
	PreviewToken     string `json:"preview_token"`
	ContentDigest    string `json:"content_digest"`
}

type GraphDraftApplyResult struct {
	Schema        int                       `json:"schema_version"`
	WorkID        string                    `json:"work_id"`
	DraftID       string                    `json:"draft_id"`
	Revision      int                       `json:"revision"`
	EventID       string                    `json:"event_id"`
	Operations    []GraphDraftOperation     `json:"operations"`
	AffectedTasks []string                  `json:"affected_tasks"`
	AppliedAt     string                    `json:"applied_at"`
	ProofImpact   GraphDraftProofProjection `json:"proof_impact"`
}

type graphDraftAuthorization struct {
	Revision int
	Prepare  bool
	Apply    bool
	Scopes   []string
}

func graphDraftError(code, message string) error {
	return &CommandError{Code: code, Message: message, Retryable: code == "revision_conflict" || code == "preview_stale" || code == "active_scope_conflict"}
}

func normalizeGraphOperations(operations []GraphDraftOperation) ([]GraphDraftOperation, error) {
	if len(operations) == 0 || len(operations) > 100 {
		return nil, graphDraftError("invalid_input", "operations doit contenir entre 1 et 100 modifications")
	}
	out := append([]GraphDraftOperation(nil), operations...)
	seen := map[string]bool{}
	edges := map[string]string{}
	for i := range out {
		op := &out[i]
		op.Kind = strings.TrimSpace(op.Kind)
		op.Prerequisite = strings.TrimSpace(op.Prerequisite)
		op.Dependent = strings.TrimSpace(op.Dependent)
		if (op.Kind != "add_dependency" && op.Kind != "remove_dependency") || !safeName(op.Prerequisite) || !safeName(op.Dependent) || op.Prerequisite == op.Dependent {
			return nil, graphDraftError("invalid_input", fmt.Sprintf("opération %d invalide", i+1))
		}
		key := op.Kind + "\x00" + op.Prerequisite + "\x00" + op.Dependent
		if seen[key] {
			return nil, graphDraftError("invalid_input", "opération répétée dans le brouillon")
		}
		seen[key] = true
		edge := op.Prerequisite + "\x00" + op.Dependent
		if previous, ok := edges[edge]; ok && previous != op.Kind {
			return nil, graphDraftError("invalid_input", "ajout et retrait contradictoires pour la même dépendance")
		}
		edges[edge] = op.Kind
	}
	return out, nil
}

func graphOperationsDigest(operations []GraphDraftOperation) string {
	raw, _ := json.Marshal(operations)
	return hash(raw)
}

func scanGraphDraft(row interface{ Scan(...any) error }) (GraphDraft, error) {
	var d GraphDraft
	var raw []byte
	err := row.Scan(&d.ID, &d.WorkID, &d.BaseRevision, &d.Revision, &d.Status, &d.Actor, &raw, &d.ContentDigest, &d.PreviewToken, &d.Created, &d.Updated)
	if err == nil {
		d.Schema = 1
		err = json.Unmarshal(raw, &d.Operations)
	}
	return d, err
}

func graphAuthorization(tx *sql.Tx, work, actor string) (graphDraftAuthorization, error) {
	var a graphDraftAuthorization
	var prepare, apply int
	var scopes []byte
	err := tx.QueryRow("SELECT revision,prepare,apply,scopes FROM graph_draft_authorizations WHERE work_id=? AND actor=?", work, actor).Scan(&a.Revision, &prepare, &apply, &scopes)
	if errors.Is(err, sql.ErrNoRows) {
		// Swarm is a local operator application. An explicit row overrides this
		// initial owner capability and therefore makes revocation durable.
		return graphDraftAuthorization{Revision: 0, Prepare: actor == operatorIdentity(), Apply: actor == operatorIdentity()}, nil
	}
	if err != nil {
		return a, err
	}
	a.Prepare, a.Apply = prepare != 0, apply != 0
	if err = json.Unmarshal(scopes, &a.Scopes); err != nil {
		return a, err
	}
	return a, nil
}

func (s *Store) setGraphDraftAuthorization(work, actor string, prepare, apply bool, scopes []string) error {
	if !safeName(work) || strings.TrimSpace(actor) == "" {
		return graphDraftError("invalid_input", "mission et acteur requis")
	}
	for _, scope := range scopes {
		if !safeName(scope) {
			return graphDraftError("invalid_input", "périmètre invalide")
		}
	}
	raw, _ := json.Marshal(scopes)
	_, err := s.db.Exec(`INSERT INTO graph_draft_authorizations(work_id,actor,revision,prepare,apply,scopes,updated)
	 VALUES(?,?,1,?,?,?,?) ON CONFLICT(work_id,actor) DO UPDATE SET revision=revision+1,prepare=excluded.prepare,apply=excluded.apply,scopes=excluded.scopes,updated=excluded.updated`, work, actor, prepare, apply, raw, now())
	return err
}

func requireGraphRight(a graphDraftAuthorization, apply bool) error {
	if (!apply && !a.Prepare) || (apply && !a.Apply) {
		return graphDraftError("authorization_required", "autorisation de modification du plan requise")
	}
	return nil
}

func (s *Store) saveGraphDraft(actor string, request GraphDraftSaveRequest) (GraphDraft, error) {
	var empty GraphDraft
	if request.Schema != 1 || !safeName(request.WorkID) {
		return empty, graphDraftError("invalid_input", "schema_version=1 et work_id valide requis")
	}
	operations, err := normalizeGraphOperations(request.Operations)
	if err != nil {
		return empty, err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return empty, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", request.WorkID); err != nil {
		return empty, err
	}
	var workRevision int
	if err = tx.QueryRow("SELECT revision FROM works WHERE id=?", request.WorkID).Scan(&workRevision); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return empty, graphDraftError("unknown_work", "mission inconnue")
		}
		return empty, err
	}
	auth, err := graphAuthorization(tx, request.WorkID, actor)
	if err != nil {
		return empty, err
	}
	if err = requireGraphRight(auth, false); err != nil {
		return empty, err
	}
	if request.ExpectedRevision != workRevision {
		return empty, graphDraftError("revision_conflict", fmt.Sprintf("révision périmée : attendue %d, courante %d", request.ExpectedRevision, workRevision))
	}
	raw, _ := json.Marshal(operations)
	at := now()
	if request.DraftID == "" {
		d := GraphDraft{Schema: 1, ID: newID("draft-"), WorkID: request.WorkID, BaseRevision: workRevision, Revision: 1, Status: "editing", Actor: actor, Operations: operations, ContentDigest: graphOperationsDigest(operations), Created: at, Updated: at}
		_, err = tx.Exec("INSERT INTO graph_drafts(id,work_id,base_revision,revision,status,actor,operations,content_digest,preview_token,created,updated) VALUES(?,?,?,?,?,?,?,?,?,?,?)", d.ID, d.WorkID, d.BaseRevision, d.Revision, d.Status, d.Actor, raw, d.ContentDigest, "", d.Created, d.Updated)
		if err != nil {
			return empty, err
		}
		if err = tx.Commit(); err != nil {
			return empty, err
		}
		return d, nil
	}
	if !safeName(request.DraftID) {
		return empty, graphDraftError("invalid_input", "draft_id invalide")
	}
	d, err := scanGraphDraft(tx.QueryRow("SELECT id,work_id,base_revision,revision,status,actor,operations,content_digest,preview_token,created,updated FROM graph_drafts WHERE id=? AND work_id=?", request.DraftID, request.WorkID))
	if errors.Is(err, sql.ErrNoRows) {
		return empty, graphDraftError("invalid_input", "brouillon inconnu")
	}
	if err != nil {
		return empty, err
	}
	if d.Actor != actor {
		return empty, graphDraftError("authorization_required", "seul l’auteur peut modifier ce brouillon")
	}
	if d.Status == "applied" {
		return empty, graphDraftError("already_applied", "brouillon déjà appliqué")
	}
	if d.Revision != request.ExpectedDraftRevision {
		return empty, graphDraftError("draft_conflict", "révision du brouillon périmée")
	}
	d.Revision++
	d.Status, d.Operations, d.ContentDigest, d.PreviewToken, d.Updated = "editing", operations, graphOperationsDigest(operations), "", at
	_, err = tx.Exec("UPDATE graph_drafts SET revision=?,status=?,operations=?,content_digest=?,preview_token='',updated=? WHERE id=? AND revision=?", d.Revision, d.Status, raw, d.ContentDigest, d.Updated, d.ID, request.ExpectedDraftRevision)
	if err != nil {
		return empty, err
	}
	if err = tx.Commit(); err != nil {
		return empty, err
	}
	return d, nil
}

func (s *Store) getGraphDraft(actor, work, id string) (GraphDraft, error) {
	var empty GraphDraft
	if !safeName(work) || !safeName(id) {
		return empty, graphDraftError("invalid_input", "work_id et draft_id valides requis")
	}
	tx, err := s.db.Begin()
	if err != nil {
		return empty, err
	}
	defer tx.Rollback()
	auth, err := graphAuthorization(tx, work, actor)
	if err != nil {
		return empty, err
	}
	if err = requireGraphRight(auth, false); err != nil {
		return empty, err
	}
	d, err := scanGraphDraft(tx.QueryRow("SELECT id,work_id,base_revision,revision,status,actor,operations,content_digest,preview_token,created,updated FROM graph_drafts WHERE id=? AND work_id=?", id, work))
	if errors.Is(err, sql.ErrNoRows) {
		return empty, graphDraftError("invalid_input", "brouillon inconnu")
	}
	if err != nil {
		return empty, err
	}
	if d.Actor != actor {
		return empty, graphDraftError("authorization_required", "brouillon hors autorisation")
	}
	return d, nil
}

func graphState(work *Work, operations []GraphDraftOperation) (map[string][]string, []string, error) {
	deps := map[string][]string{}
	known := map[string]bool{}
	for _, task := range work.Tasks {
		known[task.ID] = true
		deps[task.ID] = append([]string(nil), task.Depends...)
	}
	changed := map[string]bool{}
	for _, op := range operations {
		if !known[op.Prerequisite] || !known[op.Dependent] {
			return nil, nil, graphDraftError("unknown_task", "une extrémité de dépendance est inconnue")
		}
		found := false
		for _, value := range deps[op.Dependent] {
			if value == op.Prerequisite {
				found = true
				break
			}
		}
		if op.Kind == "add_dependency" {
			if found {
				return nil, nil, graphDraftError("duplicate_dependency", "dépendance déjà présente")
			}
			deps[op.Dependent] = append(deps[op.Dependent], op.Prerequisite)
		} else {
			if !found {
				return nil, nil, graphDraftError("invalid_input", "dépendance à retirer absente")
			}
			kept := deps[op.Dependent][:0]
			for _, value := range deps[op.Dependent] {
				if value != op.Prerequisite {
					kept = append(kept, value)
				}
			}
			deps[op.Dependent] = kept
		}
		changed[op.Dependent] = true
	}
	state := map[string]int{}
	var visit func(string) error
	visit = func(id string) error {
		if state[id] == 1 {
			return graphDraftError("dependency_cycle", "la proposition crée un cycle de dépendances")
		}
		if state[id] == 2 {
			return nil
		}
		state[id] = 1
		for _, dependency := range deps[id] {
			if err := visit(dependency); err != nil {
				return err
			}
		}
		state[id] = 2
		return nil
	}
	for id := range deps {
		if err := visit(id); err != nil {
			return nil, nil, err
		}
	}
	// A dependency edit affects its dependent and every downstream consumer.
	for progress := true; progress; {
		progress = false
		for id, taskDeps := range deps {
			if changed[id] {
				continue
			}
			for _, dependency := range taskDeps {
				if changed[dependency] {
					changed[id], progress = true, true
					break
				}
			}
		}
	}
	affected := make([]string, 0, len(changed))
	for id := range changed {
		affected = append(affected, id)
	}
	sort.Strings(affected)
	return deps, affected, nil
}

func graphScopeAllowed(auth graphDraftAuthorization, affected []string) bool {
	if len(auth.Scopes) == 0 {
		return true
	}
	allowed := map[string]bool{}
	for _, id := range auth.Scopes {
		allowed[id] = true
	}
	for _, id := range affected {
		if !allowed[id] {
			return false
		}
	}
	return true
}

func graphActiveGuard(tx *sql.Tx, work *Work, affected []string) error {
	active := map[string]bool{}
	for _, task := range work.Tasks {
		if task.Status == "running" {
			active[task.ID] = true
		}
	}
	rows, err := tx.Query("SELECT task_id FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", work.ID)
	if err != nil {
		return err
	}
	defer rows.Close()
	for rows.Next() {
		var id string
		if err = rows.Scan(&id); err != nil {
			return err
		}
		active[id] = true
	}
	if err = rows.Err(); err != nil {
		return err
	}
	for _, id := range affected {
		if active[id] {
			return graphDraftError("active_scope_conflict", "la modification touche une tâche ou un périmètre actif : "+id)
		}
	}
	return nil
}

func graphPreviewToken(d GraphDraft, auth graphDraftAuthorization, affected []string) string {
	raw, _ := json.Marshal(struct {
		Work, Draft, Digest string
		Base, Authorization int
		Scopes, Affected    []string
	}{d.WorkID, d.ID, d.ContentDigest, d.BaseRevision, auth.Revision, auth.Scopes, affected})
	return hash(raw)
}

func (s *Store) previewGraphDraft(actor string, request GraphDraftPreviewRequest) (GraphDraftPreview, error) {
	var out GraphDraftPreview
	if request.Schema != 1 || !safeName(request.WorkID) || !safeName(request.DraftID) {
		return out, graphDraftError("invalid_input", "requête de prévisualisation invalide")
	}
	tx, err := s.db.Begin()
	if err != nil {
		return out, err
	}
	defer tx.Rollback()
	var raw []byte
	var revision int
	if err = tx.QueryRow("SELECT revision,body FROM works WHERE id=?", request.WorkID).Scan(&revision, &raw); err != nil {
		return out, err
	}
	if revision != request.ExpectedRevision {
		return out, graphDraftError("revision_conflict", fmt.Sprintf("révision périmée : attendue %d, courante %d", request.ExpectedRevision, revision))
	}
	var work Work
	if err = json.Unmarshal(raw, &work); err != nil {
		return out, err
	}
	d, err := scanGraphDraft(tx.QueryRow("SELECT id,work_id,base_revision,revision,status,actor,operations,content_digest,preview_token,created,updated FROM graph_drafts WHERE id=? AND work_id=?", request.DraftID, request.WorkID))
	if err != nil {
		return out, err
	}
	if d.Actor != actor {
		return out, graphDraftError("authorization_required", "brouillon hors autorisation")
	}
	if d.Status == "applied" {
		return out, graphDraftError("already_applied", "brouillon déjà appliqué")
	}
	if d.BaseRevision != revision {
		return out, graphDraftError("revision_conflict", "la mission a changé depuis la création du brouillon")
	}
	auth, err := graphAuthorization(tx, request.WorkID, actor)
	if err != nil {
		return out, err
	}
	if err = requireGraphRight(auth, false); err != nil {
		return out, err
	}
	deps, affected, err := graphState(&work, d.Operations)
	if err != nil {
		return out, err
	}
	if !graphScopeAllowed(auth, affected) {
		return out, graphDraftError("authorization_required", "la modification dépasse le périmètre autorisé")
	}
	if err = graphActiveGuard(tx, &work, affected); err != nil {
		return out, err
	}
	token := graphPreviewToken(d, auth, affected)
	if _, err = tx.Exec("UPDATE graph_drafts SET status='previewed',preview_token=?,updated=? WHERE id=?", token, now(), d.ID); err != nil {
		return out, err
	}
	if err = tx.Commit(); err != nil {
		return out, err
	}
	after := work
	after.Tasks = append([]Task(nil), work.Tasks...)
	for i := range after.Tasks {
		after.Tasks[i].Depends = append([]string(nil), deps[after.Tasks[i].ID]...)
	}
	return GraphDraftPreview{Schema: 1, WorkID: d.WorkID, DraftID: d.ID, BaseRevision: d.BaseRevision, ContentDigest: d.ContentDigest, PreviewToken: token, Operations: d.Operations, AffectedTasks: affected, RequiredRight: "apply_plan", NoImplicitLaunch: true, ProofImpact: graphDraftProofProjection(&work, &after, affected)}, nil
}

func (s *Store) applyGraphDraft(actor string, request GraphDraftApplyRequest) (GraphDraftApplyResult, error) {
	var out GraphDraftApplyResult
	if request.Schema != 1 || !safeName(request.WorkID) || !safeName(request.DraftID) || !safeName(request.EventID) {
		return out, graphDraftError("invalid_input", "requête d’application invalide")
	}
	requestRaw, _ := json.Marshal(request)
	tx, err := s.db.Begin()
	if err != nil {
		return out, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", request.WorkID); err != nil {
		return out, err
	}
	var oldWork, oldKind string
	var oldPayload, oldRequest []byte
	err = tx.QueryRow("SELECT work_id,kind,payload,request FROM events WHERE id=?", request.EventID).Scan(&oldWork, &oldKind, &oldPayload, &oldRequest)
	if err == nil {
		if oldWork != request.WorkID || oldKind != "graph_draft.apply" || string(oldRequest) != string(requestRaw) {
			return out, graphDraftError("event_conflict", "event_id déjà utilisé pour un contenu différent")
		}
		if err = json.Unmarshal(oldPayload, &out); err != nil {
			return out, err
		}
		return out, nil
	}
	if !errors.Is(err, sql.ErrNoRows) {
		return out, err
	}
	var raw []byte
	var revision int
	if err = tx.QueryRow("SELECT revision,body FROM works WHERE id=?", request.WorkID).Scan(&revision, &raw); err != nil {
		return out, err
	}
	var work Work
	if err = json.Unmarshal(raw, &work); err != nil {
		return out, err
	}
	d, err := scanGraphDraft(tx.QueryRow("SELECT id,work_id,base_revision,revision,status,actor,operations,content_digest,preview_token,created,updated FROM graph_drafts WHERE id=? AND work_id=?", request.DraftID, request.WorkID))
	if err != nil {
		return out, err
	}
	if d.Actor != actor {
		return out, graphDraftError("authorization_required", "brouillon hors autorisation")
	}
	if d.Status != "previewed" || d.PreviewToken == "" || d.PreviewToken != request.PreviewToken || d.ContentDigest != request.ContentDigest {
		return out, graphDraftError("preview_stale", "prévisualisation absente ou périmée")
	}
	auth, err := graphAuthorization(tx, request.WorkID, actor)
	if err != nil {
		return out, err
	}
	if err = requireGraphRight(auth, true); err != nil {
		return out, err
	}
	deps, affected, err := graphState(&work, d.Operations)
	if err != nil {
		return out, err
	}
	if !graphScopeAllowed(auth, affected) {
		return out, graphDraftError("authorization_required", "la modification dépasse le périmètre autorisé")
	}
	if err = graphActiveGuard(tx, &work, affected); err != nil {
		return out, err
	}
	if revision != request.ExpectedRevision || d.BaseRevision != revision {
		return out, graphDraftError("revision_conflict", fmt.Sprintf("révision périmée : attendue %d, courante %d", request.ExpectedRevision, revision))
	}
	if graphPreviewToken(d, auth, affected) != request.PreviewToken {
		return out, graphDraftError("preview_stale", "autorisation ou portée modifiée depuis la prévisualisation")
	}
	before := work
	before.Tasks = append([]Task(nil), work.Tasks...)
	for i := range work.Tasks {
		work.Tasks[i].Depends = deps[work.Tasks[i].ID]
		for _, id := range affected {
			if work.Tasks[i].ID == id && work.Tasks[i].Gate != nil {
				work.Tasks[i].EvidenceStaleReason = "Dépendances ou contrat du graphe modifiés après le contrôle ; reçu historique conservé, nouvelle validation requise."
				break
			}
		}
	}
	work.Revision++
	work.Updated = now()
	workRaw, _ := json.Marshal(work)
	result, err := tx.Exec("UPDATE works SET revision=?,body=? WHERE id=? AND revision=?", work.Revision, workRaw, work.ID, request.ExpectedRevision)
	if err != nil {
		return out, err
	}
	if count, _ := result.RowsAffected(); count != 1 {
		return out, graphDraftError("revision_conflict", "la mission a changé pendant l’application")
	}
	out = GraphDraftApplyResult{Schema: 1, WorkID: work.ID, DraftID: d.ID, Revision: work.Revision, EventID: request.EventID, Operations: d.Operations, AffectedTasks: affected, AppliedAt: work.Updated, ProofImpact: graphDraftProofProjection(&before, &work, affected)}
	payload, _ := json.Marshal(out)
	if _, err = tx.Exec("UPDATE graph_drafts SET status='applied',revision=revision+1,updated=? WHERE id=? AND status='previewed'", work.Updated, d.ID); err != nil {
		return out, err
	}
	if _, err = tx.Exec("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)", request.EventID, work.ID, work.Revision, "graph_draft.apply", work.Updated, payload, requestRaw); err != nil {
		return out, err
	}
	if err = tx.Commit(); err != nil {
		return out, err
	}
	return out, nil
}
