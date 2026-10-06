# Complément de revue B02 — service B01 actuel

Le dossier B02 accepté demeure historique et intact. Son empreinte b290e646… était vraie avant les changements B03/B04. Le service courant a SHA256 c396c9d2303e213f88e8416ce5bd8aadfbf926077301c691eab2a389e2eb470f. Le complément ci-dessous reprend intégralement le dossier B01 historique avec son bloc graph_draft.go remplacé par la source actuelle ; les autres blocs sont conservés exactement. Les changements concernent la projection proof_impact et la fraîcheur ; la parité utilise toujours le même service derrière les deux adaptateurs. Les tests B01 frais et la nouvelle recette B02 couvrent ce candidat. Le contenu joint par contrôles documentaires n’est ni un avis indépendant ni une nouvelle preuve fonctionnelle.

# Dossier de revue B01 — sources complètes

Mission w-768ed45d1845fbec90bea04d ; B01 ; tentative a-a4db668c56f16afc0f289f16 ; producteur auto-7070b3180fb50d773f66. Base Git cc3069dc7bb61b90168d21f945cb2eb5e27578ed, checkout sale conservé. Tables dédiées graph_drafts/graph_draft_authorizations, migration v24 sauvegardée ; Work reste le graphe appliqué. Le service revalide droits, portée aval, activité, révision, token et contenu au moment de l’effet. Aucun appel au dispatcher ni création d’agent.

Routes authentifiées : GET/POST /api/v1/graph-drafts ; POST /api/v1/graph-drafts/preview ; POST /api/v1/graph-drafts/apply. Le source et les tests complets suivent. Les intégrations exactes modèle/migration/HTTP sont jointes à la fin. Les contrôles engine_controls fournissent les observations hôte actuelles ; le rapport producteur ne remplace pas ces reçus.

## Source complète — graph_draft.go

```go
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
```

## Source complète — graph_draft_http.go

```go
//go:build linux

package main

import (
        "io"
        "net/http"
)

func registerGraphDraftHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
        decode := func(w http.ResponseWriter, r *http.Request, value any) error {
                raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
                if err != nil {
                        return err
                }
                return strict(raw, value)
        }
        mux.HandleFunc("/api/v1/graph-drafts", func(w http.ResponseWriter, r *http.Request) {
                if r.Method == http.MethodGet {
                        value, err := s.getGraphDraft(operatorIdentity(), r.URL.Query().Get("work"), r.URL.Query().Get("draft"))
                        if err != nil {
                                fail(w, err)
                                return
                        }
                        send(w, value)
                        return
                }
                if r.Method != http.MethodPost {
                        http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftSaveRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.saveGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/graph-drafts/preview", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftPreviewRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.previewGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/graph-drafts/apply", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request GraphDraftApplyRequest
                if err := decode(w, r, &request); err != nil {
                        fail(w, graphDraftError("invalid_input", err.Error()))
                        return
                }
                value, err := s.applyGraphDraft(operatorIdentity(), request)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
}
```

## Tests complets — graph_draft_test.go

```go
package main

import (
        "errors"
        "sync"
        "testing"
)

func graphDraftWork(t *testing.T, s *Store) Work {
        t.Helper()
        w := createTest(t, s)
        w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Amont", Deliverable: "preuve", Criteria: []string{"preuve"}})
        w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Aval", Deliverable: "preuve", Criteria: []string{"preuve"}})
        w = applyTest(t, s, w, "task.add", Request{ID: "t3", Title: "Autre", Deliverable: "preuve", Criteria: []string{"preuve"}})
        return w
}

func createGraphDraftTest(t *testing.T, s *Store, w Work, operations ...GraphDraftOperation) (GraphDraft, GraphDraftPreview) {
        t.Helper()
        d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: operations})
        if err != nil {
                t.Fatal(err)
        }
        p, err := s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
        if err != nil {
                t.Fatal(err)
        }
        return d, p
}

func applyGraphDraftTest(t *testing.T, s *Store, w Work, d GraphDraft, p GraphDraftPreview, event string) GraphDraftApplyResult {
        t.Helper()
        result, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: event, ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
        if err != nil {
                t.Fatal(err)
        }
        return result
}

func graphCode(err error) string {
        var command *CommandError
        if errors.As(err, &command) {
                return command.Code
        }
        return ""
}

func taskDependsOn(t *testing.T, w Work, task, dependency string) bool {
        t.Helper()
        value, err := w.task(task)
        if err != nil {
                t.Fatal(err)
        }
        for _, candidate := range value.Depends {
                if candidate == dependency {
                        return true
                }
        }
        return false
}

func TestGraphDraftB01Persistence(t *testing.T) {
        root := t.TempDir()
        s, err := openStore(root, true)
        if err != nil {
                t.Fatal(err)
        }
        w := graphDraftWork(t, s)
        d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}})
        if err != nil {
                t.Fatal(err)
        }
        before, _ := s.get(w.ID)
        if taskDependsOn(t, before, "t2", "t1") {
                t.Fatal("la sauvegarde du brouillon a modifié le plan")
        }
        s.db.Close()
        s, err = openStore(root, false)
        if err != nil {
                t.Fatal(err)
        }
        t.Cleanup(func() { s.db.Close() })
        loaded, err := s.getGraphDraft(operatorIdentity(), w.ID, d.ID)
        if err != nil || loaded.ContentDigest != d.ContentDigest || loaded.Status != "editing" {
                t.Fatalf("brouillon non durable: %#v %v", loaded, err)
        }
        p, err := s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
        if err != nil {
                t.Fatal(err)
        }
        result := applyGraphDraftTest(t, s, w, d, p, "persist-apply")
        if result.Revision != w.Revision+1 {
                t.Fatalf("révision=%d", result.Revision)
        }
        after, _ := s.get(w.ID)
        if !taskDependsOn(t, after, "t2", "t1") {
                t.Fatal("ajout non appliqué")
        }
        var agents int
        if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
                t.Fatalf("départ implicite: %d %v", agents, err)
        }
        d2, p2 := createGraphDraftTest(t, s, after, GraphDraftOperation{Kind: "remove_dependency", Prerequisite: "t1", Dependent: "t2"})
        applyGraphDraftTest(t, s, after, d2, p2, "persist-remove")
        removed, _ := s.get(w.ID)
        if taskDependsOn(t, removed, "t2", "t1") {
                t.Fatal("retrait non appliqué")
        }
        t.Run("migration-from-v23", func(t *testing.T) {
                legacyRoot := t.TempDir()
                legacy, openErr := openStore(legacyRoot, true)
                if openErr != nil {
                        t.Fatal(openErr)
                }
                if _, openErr = legacy.db.Exec("DROP TABLE graph_draft_authorizations; DROP TABLE graph_drafts; PRAGMA user_version=23"); openErr != nil {
                        t.Fatal(openErr)
                }
                if openErr = legacy.db.Close(); openErr != nil {
                        t.Fatal(openErr)
                }
                migrated, openErr := openStore(legacyRoot, false)
                if openErr != nil {
                        t.Fatal(openErr)
                }
                defer migrated.db.Close()
                var version, tables int
                if openErr = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); openErr != nil {
                        t.Fatal(openErr)
                }
                if openErr = migrated.db.QueryRow("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('graph_drafts','graph_draft_authorizations')").Scan(&tables); openErr != nil {
                        t.Fatal(openErr)
                }
                if version != 24 || tables != 2 {
                        t.Fatalf("migration incomplète: version=%d tables=%d", version, tables)
                }
        })
}

func TestGraphDraftB01InvalidDependencies(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        w = applyTest(t, s, w, "task.update", Request{ID: "t2", Depends: []string{"t1"}})
        cases := []struct {
                name, code string
                op         GraphDraftOperation
        }{
                {"unknown", "unknown_task", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "absent", Dependent: "t3"}},
                {"duplicate", "duplicate_dependency", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}},
                {"cycle", "dependency_cycle", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"}},
        }
        for _, tc := range cases {
                t.Run(tc.name, func(t *testing.T) {
                        d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{tc.op}})
                        if err != nil {
                                t.Fatal(err)
                        }
                        _, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
                        if graphCode(err) != tc.code {
                                t.Fatalf("code=%q err=%v", graphCode(err), err)
                        }
                        current, getErr := s.get(w.ID)
                        if getErr != nil || current.Revision != w.Revision || !taskDependsOn(t, current, "t2", "t1") {
                                t.Fatalf("état altéré après refus: rev=%d err=%v", current.Revision, getErr)
                        }
                })
        }
}

func TestGraphDraftB01ConcurrentApply(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        d1, p1 := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        d2, p2 := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t3"})
        requests := []GraphDraftApplyRequest{
                {Schema: 1, WorkID: w.ID, DraftID: d1.ID, EventID: "concurrent-one", ExpectedRevision: w.Revision, PreviewToken: p1.PreviewToken, ContentDigest: p1.ContentDigest},
                {Schema: 1, WorkID: w.ID, DraftID: d2.ID, EventID: "concurrent-two", ExpectedRevision: w.Revision, PreviewToken: p2.PreviewToken, ContentDigest: p2.ContentDigest},
        }
        var wg sync.WaitGroup
        errs := make([]error, 2)
        for i := range requests {
                wg.Add(1)
                go func(i int) { defer wg.Done(); _, errs[i] = s.applyGraphDraft(operatorIdentity(), requests[i]) }(i)
        }
        wg.Wait()
        passes, conflicts := 0, 0
        for _, err := range errs {
                if err == nil {
                        passes++
                } else if graphCode(err) == "revision_conflict" {
                        conflicts++
                } else {
                        t.Fatalf("erreur inattendue: %v", err)
                }
        }
        if passes != 1 || conflicts != 1 {
                t.Fatalf("passes=%d conflits=%d erreurs=%v", passes, conflicts, errs)
        }
        current, _ := s.get(w.ID)
        if current.Revision != w.Revision+1 {
                t.Fatalf("deuxième effet ou effet perdu, révision=%d", current.Revision)
        }
}

func TestGraphDraftB01Idempotency(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        request := GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "stable-event", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest}
        first, err := s.applyGraphDraft(operatorIdentity(), request)
        if err != nil {
                t.Fatal(err)
        }
        second, err := s.applyGraphDraft(operatorIdentity(), request)
        if err != nil || second.Revision != first.Revision || second.AppliedAt != first.AppliedAt {
                t.Fatalf("rejeu non identique: %#v %v", second, err)
        }
        request.ContentDigest = "different-content"
        if _, err = s.applyGraphDraft(operatorIdentity(), request); graphCode(err) != "event_conflict" {
                t.Fatalf("contenu différent accepté: %v", err)
        }
        current, _ := s.get(w.ID)
        if current.Revision != first.Revision {
                t.Fatal("le rejeu a produit un second effet")
        }
}

func TestGraphDraftB01Authorization(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, true, nil); err != nil {
                t.Fatal(err)
        }
        d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, false, nil); err != nil {
                t.Fatal(err)
        }
        _, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "revoked-event", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
        if graphCode(err) != "authorization_required" {
                t.Fatalf("droit retiré non refusé: %v", err)
        }
        current, _ := s.get(w.ID)
        if current.Revision != w.Revision || taskDependsOn(t, current, "t2", "t1") {
                t.Fatal("effet malgré révocation")
        }
        if err = s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, true, []string{"t2"}); err != nil {
                t.Fatal(err)
        }
        d2, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}, {Kind: "add_dependency", Prerequisite: "t2", Dependent: "t3"}}})
        if err != nil {
                t.Fatal(err)
        }
        _, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d2.ID, ExpectedRevision: w.Revision})
        if graphCode(err) != "authorization_required" {
                t.Fatalf("portée excessive non refusée: %v", err)
        }
}

func TestGraphDraftB01ActiveScope(t *testing.T) {
        t.Run("active-before-preview", func(t *testing.T) {
                s := storeTest(t)
                w := graphDraftWork(t, s)
                w = applyTest(t, s, w, "task.update", Request{ID: "t2", Status: "running"})
                d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}})
                if err != nil {
                        t.Fatal(err)
                }
                _, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
                if graphCode(err) != "active_scope_conflict" {
                        t.Fatalf("périmètre actif non refusé: %v", err)
                }
                current, _ := s.get(w.ID)
                if current.Revision != w.Revision || taskDependsOn(t, current, "t2", "t1") {
                        t.Fatal("état modifié malgré tâche active")
                }
        })
        t.Run("active-after-preview", func(t *testing.T) {
                s := storeTest(t)
                w, launch := setupAgent(t, s)
                w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Autre", Deliverable: "preuve", Criteria: []string{"preuve"}})
                launch.Revision = w.Revision
                d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"})
                if _, created, err := s.prepare(w.ID, launch); err != nil || !created {
                        t.Fatalf("agent actif non préparé: created=%v err=%v", created, err)
                }
                _, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "active-after-preview", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
                if graphCode(err) != "active_scope_conflict" {
                        t.Fatalf("activité apparue après preview non refusée: %v", err)
                }
                current, _ := s.get(w.ID)
                if taskDependsOn(t, current, "t1", "t2") {
                        t.Fatal("état modifié malgré activité concurrente")
                }
        })
}
```

## Tests complets — graph_draft_http_test.go

```go
//go:build linux

package main

import (
        "bytes"
        "encoding/json"
        "net/http"
        "net/http/httptest"
        "testing"
)

func graphDraftHTTPRequest(t *testing.T, handler http.Handler, method, path string, value any) *httptest.ResponseRecorder {
        t.Helper()
        var body bytes.Buffer
        if value != nil {
                if err := json.NewEncoder(&body).Encode(value); err != nil {
                        t.Fatal(err)
                }
        }
        req := httptest.NewRequest(method, "http://local.test"+path, &body)
        req.Host = "local.test"
        req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "graph-token"})
        if method != http.MethodGet {
                req.Header.Set("Origin", "http://local.test")
                req.Header.Set("X-Swarm-CSRF", "graph-token")
        }
        out := httptest.NewRecorder()
        handler.ServeHTTP(out, req)
        return out
}

func TestGraphDraftB01HTTPPreviewApply(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        handler := newWebHandler(s, "local.test", "graph-token")
        save := GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}}
        response := graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts", save)
        if response.Code != http.StatusOK {
                t.Fatalf("save %d %s", response.Code, response.Body.String())
        }
        var draft GraphDraft
        if err := json.Unmarshal(response.Body.Bytes(), &draft); err != nil {
                t.Fatal(err)
        }
        response = graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/preview", GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: draft.ID, ExpectedRevision: w.Revision})
        if response.Code != http.StatusOK {
                t.Fatalf("preview %d %s", response.Code, response.Body.String())
        }
        var preview GraphDraftPreview
        if err := json.Unmarshal(response.Body.Bytes(), &preview); err != nil {
                t.Fatal(err)
        }
        if !preview.NoImplicitLaunch || preview.RequiredRight != "apply_plan" {
                t.Fatalf("contrat preview incomplet: %#v", preview)
        }
        response = graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/apply", GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: draft.ID, EventID: "http-apply", ExpectedRevision: w.Revision, PreviewToken: preview.PreviewToken, ContentDigest: preview.ContentDigest})
        if response.Code != http.StatusOK {
                t.Fatalf("apply %d %s", response.Code, response.Body.String())
        }
        current, _ := s.get(w.ID)
        if !taskDependsOn(t, current, "t2", "t1") {
                t.Fatal("route HTTP sans effet métier")
        }
        var agents int
        if err := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
                t.Fatalf("route HTTP a lancé un agent: %d %v", agents, err)
        }
        response = graphDraftHTTPRequest(t, handler, http.MethodGet, "/api/v1/graph-drafts?work="+w.ID+"&draft="+draft.ID, nil)
        if response.Code != http.StatusOK {
                t.Fatalf("read %d %s", response.Code, response.Body.String())
        }
}

func TestGraphDraftB01HTTPGuardsAndStatus(t *testing.T) {
        s := storeTest(t)
        w := graphDraftWork(t, s)
        handler := newWebHandler(s, "local.test", "graph-token")
        d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
        request := GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "http-conflict", ExpectedRevision: w.Revision + 1, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest}
        response := graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/apply", request)
        if response.Code != http.StatusConflict {
                t.Fatalf("conflit HTTP=%d body=%s", response.Code, response.Body.String())
        }
        var failure struct {
                Failure CommandError `json:"failure"`
        }
        if err := json.Unmarshal(response.Body.Bytes(), &failure); err != nil || failure.Failure.Code != "revision_conflict" {
                t.Fatalf("code métier absent: %#v %v", failure, err)
        }
        if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), false, false, nil); err != nil {
                t.Fatal(err)
        }
        response = graphDraftHTTPRequest(t, handler, http.MethodGet, "/api/v1/graph-drafts?work="+w.ID+"&draft="+d.ID, nil)
        if response.Code != http.StatusForbidden {
                t.Fatalf("droit HTTP=%d body=%s", response.Code, response.Body.String())
        }
}
```


## Intégrations complètes ajoutées par supervision avant nouvelle revue

Diff exact contre le checkout sauvegardé au lancement (base Git si fichier alors inchangé). Les fichiers entiers sont liés aux contrôles publics.

### model.go

```diff
--- baseline/model.go
+++ model.go
@@ -10,9 +10,9 @@
        "strings"
        "time"
 )

-const schemaVersion = 23
+const schemaVersion = 24

 type ManualOverride struct {
        Reason         string `json:"reason"`
        Actor          string `json:"actor"`
```

### store.go

```diff
--- baseline/store.go
+++ store.go
@@ -299,8 +299,18 @@
                if _, e = db.Exec(runLimitsConfigMigration); e != nil {
                        return fail(e)
                }
        }
+       if version < 24 {
+               if version != 0 {
+                       if _, e = db.Exec("VACUUM INTO ?", filepath.Join(dir, newID("state-pre-v24-")+".db")); e != nil {
+                               return fail(e)
+                       }
+               }
+               if _, e = db.Exec(graphDraftMigration); e != nil {
+                       return fail(e)
+               }
+       }
        if e = os.Chmod(path, 0600); e != nil {
                return fail(e)
        }
        return s, nil
```

### web_server.go

```diff
--- baseline/web_server.go
+++ web_server.go
@@ -373,20 +373,28 @@
        }
        fail := func(w http.ResponseWriter, e error) {
                w.Header().Set("Content-Type", "application/json; charset=utf-8")
                status := 400
-               if commandFailure(e).Code == "revision_conflict" {
+               code := commandFailure(e).Code
+               if code == "revision_conflict" || code == "preview_stale" || code == "active_scope_conflict" || code == "event_conflict" || code == "draft_conflict" || code == "already_applied" {
                        status = 409
                }
-               if commandFailure(e).Code == "storage_unavailable" {
+               if code == "authorization_required" {
+                       status = http.StatusForbidden
+               }
+               if code == "unknown_work" || code == "unknown_task" {
+                       status = http.StatusNotFound
+               }
+               if code == "storage_unavailable" {
                        status = http.StatusInsufficientStorage
                }
                w.WriteHeader(status)
                send(w, map[string]any{"error": commandFailure(e).Message, "failure": commandFailure(e)})
        }
        s.registerPlanning(mux, send, fail)
        s.registerProviderAdmin(mux, send, fail)
        s.registerRunLimitsAdmin(mux, send, fail)
+       registerGraphDraftHTTP(s, mux, send, fail)
        s.registerPreparations(mux)
        s.registerTerminals(mux, send, fail)
        mux.HandleFunc("/api/v1/runtime-health", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != "GET" {
```
