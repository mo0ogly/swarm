# Dossier de revue B04 — candidat et preuves fiables

## Identité, portée et limites

- Mission / tâche / tentative / producteur : `w-768ed45d1845fbec90bea04d` / `B04` / `a-fe1531641b8c47db6c3cd2c8` / `auto-2a5678c27de0911856ec`.
- Base Git : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed` ; candidat non commité dans un checkout déjà fortement modifié par A01–A04 et B01–B03.
- Portée : req-4 uniquement. Aucun script ou manifeste `execution/b/` modifié ; aucune base de mission active, commande Swarm, acceptation, commit ou push.
- Ce dossier permet la revue sans outils du contrat nouveau et de ses tests. La recette navigateur réelle, la suite complète, `go vet` et la revue indépendante restent au conducteur hôte.

## Contrat complet — `graph_draft_projection.go`

```go
package main

import (
        "encoding/json"
        "sort"
)

// GraphDraftProofProjection identifies proof inputs changed by a graph
// revision. The global work revision is deliberately excluded: presentation
// and administrative changes may advance it without changing tested inputs.
type GraphDraftProofProjection struct {
        Kind          string   `json:"kind"`
        Relevant      bool     `json:"proof_relevant"`
        AffectedTasks []string `json:"affected_tasks"`
        BeforeDigest  string   `json:"before_inputs_sha256"`
        AfterDigest   string   `json:"after_inputs_sha256"`
        Reason        string   `json:"reason"`
}

// MissionAttemptProjection binds a task selection to one concrete agent and
// business attempt. Process and activity are observations, never validation.
type MissionAttemptProjection struct {
        AgentID        string `json:"agent_id,omitempty"`
        AttemptID      string `json:"attempt_id,omitempty"`
        Role           string `json:"role,omitempty"`
        ProcessState   string `json:"process_state,omitempty"`
        Activity       string `json:"activity,omitempty"`
        Validation     string `json:"validation_state"`
        RequestedModel string `json:"requested_model,omitempty"`
        ObservedModel  string `json:"observed_model,omitempty"`
        CostState      string `json:"cost_state"`
}

func taskRelevantInputs(w *Work, t *Task) string {
        if t == nil {
                return ""
        }
        candidate := ""
        if w != nil && w.Planning != nil && w.Planning.Repository != nil {
                candidate = w.Planning.Repository.Candidate
        }
        policy := ""
        if t.ValidationPolicy != nil {
                policy = validationPolicyDigest(*t.ValidationPolicy)
        }
        dependencies := append([]string(nil), t.Depends...)
        sort.Strings(dependencies)
        attempt := ""
        if len(t.Attempts) > 0 {
                attempt = t.Attempts[len(t.Attempts)-1].ID
        }
        raw, _ := json.Marshal(struct {
                ID, Deliverable, Scope, Policy, Candidate, Attempt string
                Criteria, Dependencies, Requirements               []string
        }{t.ID, t.Deliverable, t.ScopeID, policy, candidate, attempt, append([]string(nil), t.Criteria...), dependencies, append([]string(nil), t.Requirements...)})
        return hash(raw)
}

func graphDraftProofProjection(before, after *Work, affected []string) GraphDraftProofProjection {
        ids := append([]string(nil), affected...)
        sort.Strings(ids)
        beforeInputs, afterInputs := map[string]string{}, map[string]string{}
        for _, id := range ids {
                if before != nil {
                        t, _ := before.task(id)
                        beforeInputs[id] = taskRelevantInputs(before, t)
                }
                if after != nil {
                        t, _ := after.task(id)
                        afterInputs[id] = taskRelevantInputs(after, t)
                }
        }
        oldRaw, _ := json.Marshal(beforeInputs)
        newRaw, _ := json.Marshal(afterInputs)
        p := GraphDraftProofProjection{Kind: "administrative", AffectedTasks: ids, BeforeDigest: hash(oldRaw), AfterDigest: hash(newRaw), Reason: "Entrées pertinentes identiques ; la révision administrative ne périme pas la preuve."}
        p.Relevant = p.BeforeDigest != p.AfterDigest
        if p.Relevant {
                p.Kind = "relevant"
                p.Reason = "Le candidat, le contrat de tâche, les dépendances ou la politique de contrôle ont changé ; les reçus antérieurs restent historiques mais ne valent plus acceptation courante."
        }
        return p
}

func projectMissionAttempt(t *Task, agents []Agent) MissionAttemptProjection {
        p := MissionAttemptProjection{Validation: "unknown", CostState: "unknown"}
        if t == nil {
                return p
        }
        p.Validation = t.Status
        for _, a := range agents { // store readers return newest attempts first.
                if a.TaskID != t.ID {
                        continue
                }
                p.AgentID, p.AttemptID, p.Role = a.ID, a.Attempt, a.Role
                p.ProcessState, p.Activity = a.Status, a.Activity
                if a.ModelRoute != nil {
                        p.RequestedModel = a.ModelRoute.Model
                }
                if a.ReportedModel != nil {
                        p.ObservedModel = a.ReportedModel.Model
                }
                if a.Usage != nil && a.Usage.ReportedCost != nil {
                        p.CostState = "reported"
                }
                return p
        }
        if len(t.Attempts) > 0 {
                p.AttemptID = t.Attempts[len(t.Attempts)-1].ID
        }
        return p
}
```

## Tests complets — `graph_draft_projection_test.go`

```go
package main

import (
        "os"
        "path/filepath"
        "testing"
        "time"
)

func blockingB04Control(t *testing.T, name string) (policy *ValidationPolicy, started, release string) {
        t.Helper()
        root := t.TempDir()
        started, release = filepath.Join(root, name+".started"), filepath.Join(root, name+".release")
        body := "import os,time\nopen(" + strconvQuote(started) + ", 'w').close()\nwhile not os.path.exists(" + strconvQuote(release) + "): time.sleep(0.01)\n"
        return automaticPolicy("python3", "-c", body), started, release
}

func strconvQuote(value string) string {
        quoted := "'"
        for _, r := range value {
                if r == '\'' {
                        quoted += "\\'"
                } else {
                        quoted += string(r)
                }
        }
        return quoted + "'"
}

func waitB04Control(t *testing.T, marker string) {
        t.Helper()
        deadline := time.Now().Add(3 * time.Second)
        for time.Now().Before(deadline) {
                if _, err := os.Stat(marker); err == nil {
                        return
                }
                time.Sleep(10 * time.Millisecond)
        }
        t.Fatal("long control did not start")
}

func TestGraphDraftB04RelevantChange(t *testing.T) {
        before := Work{Planning: &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-before"}}, Tasks: []Task{{ID: "source", Status: "accepted"}, {ID: "checked", Status: "accepted", Depends: []string{"source"}}}}
        after := before
        after.Tasks = append([]Task(nil), before.Tasks...)
        after.Planning = &PlanningState{Repository: &ManagedRepository{Candidate: "candidate-after"}}
        p := graphDraftProofProjection(&before, &after, []string{"checked"})
        if !p.Relevant || p.Kind != "relevant" || p.BeforeDigest == p.AfterDigest {
                t.Fatalf("candidate-relevant change preserved stale proof: %+v", p)
        }

        t.Run("long-control-receipt-becomes-stale", func(t *testing.T) {
                policy, started, release := blockingB04Control(t, "relevant")
                s, w, agent, _ := automaticValidationFixture(t, policy, true)
                current, _ := s.get(w.ID)
                done := make(chan struct{})
                go func() { s.conduct(agent, "completed"); close(done) }()
                waitB04Control(t, started)
                current, _ = s.get(w.ID)
                draft, preview := createGraphDraftTest(t, s, current, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t3", Dependent: "t1"})
                applyGraphDraftTest(t, s, current, draft, preview, "relevant-during-control")
                if err := os.WriteFile(release, []byte("release"), 0600); err != nil {
                        t.Fatal(err)
                }
                <-done
                got, _ := s.get(w.ID)
                task, _ := got.task("t1")
                if task.Status == "accepted" || task.AutoValidation != nil || task.Gate != nil {
                        t.Fatalf("stale long control accepted changed inputs: %+v", task)
                }
                receipts, _ := filepath.Glob(filepath.Join(s.root, ".swarm", "validation", w.ID, "t1", "*.json"))
                if len(receipts) == 0 {
                        t.Fatal("stale receipt was not preserved")
                }
        })

        t.Run("attached-receipt-is-kept-but-stale", func(t *testing.T) {
                s := storeTest(t)
                w := graphDraftWork(t, s)
                w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
                w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
                w = gateTest(t, s, w, gateDocument(t, s, "t1"))
                w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
                beforeGate := w.Tasks[0].Gate
                draft, preview := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t3", Dependent: "t1"})
                applyGraphDraftTest(t, s, w, draft, preview, "stale-attached-receipt")
                got, _ := s.get(w.ID)
                task, _ := got.task("t1")
                if task.Gate == nil || task.Gate.At != beforeGate.At || task.EvidenceStaleReason == "" || s.validGate(task) || s.acceptedFresh(&got, task, map[string]bool{}) {
                        t.Fatalf("attached receipt was lost or remained current: %+v", task)
                }
        })
}

func TestGraphDraftB04AdministrativeChange(t *testing.T) {
        before := Work{Revision: 7, Tasks: []Task{{ID: "checked", Title: "Avant", Owner: "worker", Next: "attendre", Deliverable: "docs/result.md", Criteria: []string{"tests"}}}}
        after := before
        after.Revision++
        after.Tasks = append([]Task(nil), before.Tasks...)
        after.Tasks[0].Title, after.Tasks[0].Owner, after.Tasks[0].Next = "Nom affiché", "autre acteur", "consulter le journal"
        p := graphDraftProofProjection(&before, &after, []string{"checked"})
        if p.Relevant || p.Kind != "administrative" || p.BeforeDigest != p.AfterDigest {
                t.Fatalf("presentation-only change invalidated unchanged inputs: %+v", p)
        }

        t.Run("long-control-survives-administrative-revision", func(t *testing.T) {
                policy, started, release := blockingB04Control(t, "administrative")
                s, w, agent, _ := automaticValidationFixture(t, policy, false)
                current, _ := s.get(w.ID)
                done := make(chan struct{})
                go func() { s.conduct(agent, "completed"); close(done) }()
                waitB04Control(t, started)
                current, _ = s.get(w.ID)
                applyTest(t, s, current, "checkpoint", Request{Summary: "Présentation opérateur actualisée", Next: "Consulter le journal"})
                if err := os.WriteFile(release, []byte("release"), 0600); err != nil {
                        t.Fatal(err)
                }
                <-done
                got, _ := s.get(w.ID)
                task, _ := got.task("t1")
                if task.Status != "accepted" || task.AutoValidation == nil || !s.validGate(task) {
                        t.Fatalf("administrative revision discarded unchanged proof inputs: %+v", task)
                }
        })
}

func TestGraphDraftB04UnknownCost(t *testing.T) {
        w := Work{Tasks: []Task{{ID: "checked", Title: "Contrôle", Status: "accepted", Attempts: []Attempt{{ID: "attempt-1"}, {ID: "attempt-2"}}}}}
        agents := []Agent{{ID: "agent-2", TaskID: "checked", Attempt: "attempt-2", Role: "worker", Status: "completed", Activity: "Processus terminé"}, {ID: "agent-1", TaskID: "checked", Attempt: "attempt-1", Role: "worker", Status: "failed"}}
        p := projectMissionAttempt(&w.Tasks[0], agents)
        if p.AgentID != "agent-2" || p.AttemptID != "attempt-2" || p.ProcessState != "completed" || p.Validation != "accepted" || p.CostState != "unknown" {
                t.Fatalf("attempt identity, process, validation or unknown cost conflated: %+v", p)
        }
        ledgers := attemptLedgers(w, agents)
        if len(ledgers) != 2 || ledgers[0].Attempt != "attempt-1" || ledgers[1].Attempt != "attempt-2" || ledgers[1].Role != "worker" || !ledgers[0].MissingUsage || !ledgers[1].MissingUsage || ledgers[0].Cost.Reported != 0 || ledgers[0].Cost.Silent != 1 {
                t.Fatalf("unknown provider consumption rendered as measured zero: %+v", ledgers)
        }
}
```

## Gardes et raccords exacts touchés

### Rattachement d’un contrôle long
La fonction complète, y compris le garde de révision administrative et la vérification des entrées pertinentes, est fournie sans omission dans docs/B04.md, bloc « Contrôle moteur long complet ». Aucun garde n’est remplacé par un résumé.

### Aperçu et application du brouillon — `graph_draft.go`

`GraphDraftPreview` et `GraphDraftApplyResult` exposent désormais
`proof_impact`. L’aperçu construit un état après sans le persister ; l’application
compare l’état avant/après effectivement écrit. Les deux passent par
`graphDraftProofProjection`, avec la même liste triée de tâches affectées.

### Projection mission — `mission_status.go`, `pilotage.go`, `mission_insights.go`

- Chaque `MissionTask` et `pilotage.tasks[TASK]` contient `attempt` construit par
  `projectMissionAttempt` sur la tentative la plus récente.
- Le bilan par tentative expose séparément rôle, modèle demandé, modèle observé,
  état du processus et validation globale de la tâche.
- `CostState` vaut `unknown` tant qu’aucun montant fournisseur n’est rapporté ;
  `CostTotal.Silent` et `MissingUsage` conservent cette absence, sans zéro inventé.

### Sélection et présentation web

`web/pilotage.js` mémorise `agent_id` avec la sélection. `web/pilot-inspector.js`
résout l’agent par cette identité, affiche séparément rôle/processus/activité/
validation, et utilise la projection pour montrer la tentative même si aucune
session agent n’est chargée. L’activité et la fin de processus ne modifient pas le
badge de validation.

La recette produit B03 a été étendue, sans l’exécuter dans ce sandbox : elle crée
une tentative via les opérations CLI publiques dans son root isolé, sélectionne la
tâche, compare l’identité DOM à `pilotage.tasks.t000.attempt`, refuse tout libellé
de validation sur cette seule activité, avance une révision par checkpoint puis
vérifie que sélection et tentative restent identiques. Les assertions publiées sont
`selection_attempt_stable`, `activity_not_validation` et
`unknown_cost_projected`. Le conducteur hôte doit encore les observer dans les
quatre variantes et collecter diagnostics/captures.

## Documentation et i18n

`GUIDE-UTILISATEUR.md` et `docs/en/USER-GUIDE.md` documentent la séparation rôle,
processus, activité, validation, modèles et coût inconnu, ainsi que le contrat de
fraîcheur. Les nouvelles clés sont dans `locales/en.json`; `web/i18n-en.js` est
généré par `npm run i18n:build`.

## Empreintes du candidat au moment du dossier

| Fichier | SHA-256 |
| --- | --- |
| `graph_draft_projection.go` | `a4085936414f083283a6a3dac6738525a7e114954d2d91d5e6917fb0c1485121` |
| `graph_draft_projection_test.go` | `842b9dd702040d4fb491f5f83086a88e768001b2614f23539b4609b3204c4318` |
| `graph_draft.go` | `95679e6c1870bfd119a9ed076a06de0fca04a156d9c86352e7ae0a302b714f64` |
| `automatic_validation.go` | `9cb2b4434bad93f8c466652ced0a51437e65a8dbf7e5e2ca9c4881c19dec030e` |
| `mission_insights.go` | `f786a43b5be5296d05c6aa0f39981612dcc651295788e30daf8d673d67447c57` |
| `mission_status.go` | `7b1b51c1b237c840b9f6958bd787fbf97d1a17ad99e7bf14c2747ddf30702f4c` |
| `pilotage.go` | `f015680cffe488877db801bfa85eecc14a814f3dd4bf7e48e39c340c64d6f02d` |
| `web/pilotage.js` | `ca7ac4271d0983e20a3e0b856bdb5d057db717300dda03de25f28dade45f9592` |
| `web/pilot-inspector.js` | `49a6db25e852e698f04d5e99ee2a1ce28a23cf0eff9fe78bbd6f415a926f7f5a` |
| `tests/graph_draft_ui.cjs` | `d0482bd3c626b3500730a3b3245de75201f6b3dcbaa8d294160fa00a4350f1d5` |

Les empreintes finales de toutes les entrées, le dossier lui-même et le rapport
sont consignés dans `docs/B04.md` après les derniers contrôles. Toute modification
ultérieure d’une entrée partagée invalide ces empreintes et impose l’analyse
d’impact B01–B03 avant contrôle hôte/revue.

## Résultats disponibles et limites

- `go test ./... -run '^TestGraphDraftB04' -count=1` : PASS, exit 0.
- `go test -race ./... -run '^TestGraphDraftB04' -count=1` : PASS final, exit 0.
- Régressions ciblées B01/validation automatique/bilan/statut : PASS, exit 0.
- Syntaxe JS, génération/catalogue i18n, tests frontend purs bilan/graphe : PASS.
- `execution/b/verify.py B04`, suite Go complète, vet, npm complet et navigateur : NOT TESTED par ce worker, réservés au conducteur hôte.
- Revue indépendante et acceptation moteur : NOT TESTED ; ce dossier n’est ni l’une ni l’autre.

## Pièces d’intégration fournies par supervision avant revue

Les sections suivantes sont copiées exactement depuis le candidat courant ; les fonctions de projection et tests complets ci-dessus, ainsi que runAutomaticValidation et la recette produit complète joints au rapport, constituent les pièces de revue. Les résultats hôte restent à établir.

### graph_draft.go : aperçu et application complets
```go
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

### pilotage.go : 1–55
```go
//go:build linux

package main

import (
        "errors"
        "strings"
        "time"
)

// Projection de lecture : mêmes preuves et prédicats que les commandes.
// Aucun verdict de fraîcheur n'est conservé entre deux snapshots.
func (s *Store) pilotage(w *Work, agents []Agent, validation WorkValidation) map[string]any {
        at := now()
        edges := []map[string]any{}
        tasks := map[string]any{}
        memo := map[string]bool{}
        for i := range w.Tasks {
                t := &w.Tasks[i]
                waiting := []string{}
                for _, id := range t.Depends {
                        d, _ := w.task(id)
                        fresh := s.acceptedFreshMemo(w, d, map[string]bool{}, memo)
                        code, label := "dependency_waiting", "Dépendance non validée ou périmée"
                        if fresh {
                                code, label = "dependency_satisfied", "Dépendance validée actuellement"
                        } else {
                                waiting = append(waiting, id)
                        }
                        edges = append(edges, map[string]any{"from_task_id": id, "to_task_id": t.ID, "satisfied_now": fresh, "reason_code": code, "reason_label": label, "evaluated_revision": w.Revision, "evaluated_at": at})
                }
                tasks[t.ID] = map[string]any{"ready": s.assistCanStart(w, t), "waiting_on": waiting, "delivery": validation.Tasks[t.ID], "attempt": projectMissionAttempt(t, agents)}
        }
        health := map[string]any{}
        confirmed, unknown, occupied := 0, 0, 0
        for _, a := range agents {
                desired, _ := s.desired(a.ID)
                health[a.ID] = pilotAgentHealth(a, desired, at)
                if activeAgent(a) {
                        occupied++
                        status := observedAgent(a)
                        if status == "running" || status == "stopping" {
                                confirmed++
                        } else {
                                unknown++
                        }
                }
        }
        var total int
        complete := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&total) == nil
        return map[string]any{"schema_version": 1, "project_view_key": hash([]byte(s.root)), "snapshot_as_of": at,
                "edges": edges, "tasks": tasks, "health": health,
                "summary": map[string]any{"confirmed": confirmed, "unknown": unknown, "occupied": occupied, "total": total, "loaded": len(agents), "complete": complete}}
}


```

### mission_insights.go : 438–477
```go
func missionChangeCategory(kind string, raw []byte) (string, string) {
        var p struct {
                Status string `json:"status"`
                Next   string `json:"next"`
                State  string `json:"state"`
        }
        _ = json.Unmarshal(raw, &p)
        switch kind {
        case "task.update":
                switch p.Status {
                case "submitted":
                        return "result", "Résultat soumis pour examen"
                case "accepted":
                        return "result", "Acceptation enregistrée"
                case "blocked":
                        return "block", "Blocage enregistré"
                }
                if p.Next != "" {
                        return "decision", "Consigne de reprise modifiée"
                }
        case "review.result":
                if p.State == "passed" {
                        return "result", "Avis indépendant enregistré"
                }
                return "block", "Vérification à examiner"
        case "review.failure", "planning.proof-stale":
                return "block", "Preuve ou vérification à examiner"
        case "task.auto-validation":
                if p.State == "blocked" {
                        return "block", "Contrôle en échec"
                }
                return "result", "Contrôles enregistrés"
        case "task.auto-validation-reviewed":
                return "result", "Acceptation après contrôles et revue"
        case "decision", "decision.moteur", "planning.decide", "planning.authorize-recovery", "planning.extend-attempt":
                return "decision", "Décision enregistrée"
        }
        return "", ""
}


```

### web/pilot-inspector.js : 36–139
```js
     const data=await api('/api/v1/agent-detail?'+new URLSearchParams({work,agent:agentId}));
     if(token!==this.generation||work!==requestWork||Pilot.state.selection?.id!==selected.id)return;
     this.loaded=data;a=data.agent;
    }catch(e){if(token===this.generation&&work===requestWork){$('pilot-inspector-title').textContent=tr_web_pilot_inspector_js('Tentative indisponible');$('pilot-inspector-body').textContent=tr_web_pilot_inspector_js('Cette tentative ne peut plus être consultée dans ce travail. Revenez à votre vue pour choisir un autre agent.');this.queueFooter($('pilot-inspector-body'))}return}
   }
  }
  const tid=selected.kind==='task'?selected.id:decision?.task_id||a?.task_id;
  const t=snapshot.work.tasks.find(t=>t.id===tid);
  if(!a&&t&&selected.kind==='task')a=snapshot.agents.find(x=>x.agent.id===(selected.agent_id||snapshot.pilotage?.tasks?.[tid]?.attempt?.agent_id))?.agent||Pilot.taskAgent(tid);
  const h=snapshot.pilotage?.health[a?.id]||(a&&this.loaded?.agent.id===a.id?this.loaded.health:null);
  const validation=snapshot.validation?.tasks[tid],uncertain=Pilot.uncertainExecution(t,a,h);
  const shown=PilotGraph.visible(snapshot.work.tasks,Pilot.state.collapsed);
  const masked=t&&(!shown.has(t.id)||!Pilot.matches(t,a));
  const signature=JSON.stringify([work,selected,t,a?.progress,a?.status,a?.usage,a?.reported_model,(globalThis.SwarmI18n?.engine(h?.process_label) ?? h?.process_label),(globalThis.SwarmI18n?.engine(h?.activity_label) ?? h?.activity_label),validation,decision,masked,Pilot.queue,Pilot.interventions(),snapshot.task_actions?.[tid]]);
  if(signature===Pilot.inspectorKey&&!open)return;Pilot.inspectorKey=signature;const token=++this.generation;
  const body=$('pilot-inspector-body'),focusKey=document.activeElement?.dataset.inspectorAction;
  const activityExpanded=body.querySelector('#pilot-activity-details')?.open===true;
  body.replaceChildren();
  $('pilot-inspector-title').textContent=decision?tr_web_pilot_inspector_js('Intervention à examiner'):t?.status==='submitted'?tr_web_pilot_inspector_js('Résultat à examiner')+(a?' — '+a.provider:''):a?'Agent '+a.provider:Pilot.taskTitle(t);
  const mission=node('section',undefined,'pilot-mission');mission.append(node('p',snapshot.work.title,'pilot-eyebrow'));body.append(mission);if(a)mission.append(ProjectProfiles.badge(a.workflow));if(t&&typeof TaskModels!=='undefined')mission.append(node('p',TaskModels.text(t,a)),Pilot.command(tr_web_pilot_inspector_js('Modèle de la tâche'),()=>TaskModels.open(t.id)));
  if(!t&&!decision){body.append(node('p',tr_web_pilot_inspector_js('Élément supprimé ou indisponible. Aucune commande ne sera exécutée.'),'notice attention'));this.queueFooter(body);return}
  if(t){
   mission.append(node('h3',Pilot.taskTitle(t)));
   if(t.launch_held){
    const plan=snapshot.work.plans?.find(p=>p.source?.startsWith('prep-')&&p.task_ids?.includes(t.id));
    const held=node('section',undefined,'notice attention');held.append(node('p',tr_web_pilot_inspector_js('Mission créée ; son démarrage attend votre autorisation.')));
    if(plan){const link=node('a',tr_web_pilot_inspector_js('Ouvrir la préparation pour autoriser les missions'));link.href='/prepare.html?id='+encodeURIComponent(plan.source);link.dataset.preparation=plan.source;held.append(link)}
    body.append(held);
   }
   const state=validation?.state||t.status;const badge=node('p',uncertain||(state==='running'?tr_web_pilot_inspector_js('Résultat de la tâche : pas encore validé'):tr_web_pilot_inspector_js('Validation actuelle : ')+(labels[state]||state)),'pilot-validation');badge.dataset.state=uncertain?'stale':state;mission.append(badge);
   if(t.status==='submitted')body.append(this.reviewControls(t,a?.id));
   else if(!t.launch_held){const recovery=this.recoveryControls(t,a,h);if(recovery)body.append(recovery)}
   body.append(node('p',snapshot.work.objective));
   const purpose=node('section',undefined,'pilot-section');purpose.classList.add('pilot-purpose');purpose.append(node('h4',tr_web_pilot_inspector_js('Ce qui était demandé')));
   const criteria=node('ul');for(const c of t.criteria||[])criteria.append(node('li',c));purpose.append(criteria);
   body.append(purpose);
  }
  if(decision){
   body.append(node('p',decision.resolved_at?tr_web_pilot_inspector_js('Cette demande a été traitée ailleurs.'):decisionObservation(decision).summary,'notice attention'),node('p',decision.evidence));
   if(!decision.resolved_at)body.append(this.action(tr_web_pilot_inspector_js('Examiner et décider'),()=>openDecision(decision),'decision'));
  }
  if(masked){body.append(node('p',tr_web_pilot_inspector_js('Cette sélection est masquée dans le graphe ou par un filtre.'),'notice info'),this.action(tr_web_pilot_inspector_js('Révéler et retirer les filtres masquants'),()=>Pilot.revealSelection(),'reveal'))}
  if(a){
   body.append(this.action(['terminal','dialogue'].includes(a.mode)?tr_web_pilot_inspector_js('Ouvrir la session interactive'):tr_web_pilot_inspector_js('Voir la session de l’agent'),()=>AgentTerminal.open(a),'terminal'));
   const health=node('section',undefined,'pilot-section');health.id='pilot-current-activity';
   health.append(node('h4',tr_web_pilot_inspector_js('Rôle, processus, activité et validation')));
   const explanation=this.activityExplanation(a,h,uncertain);
   const identity=snapshot.pilotage?.tasks?.[tid]?.attempt||{};
   health.append(node('p',tr_web_pilot_inspector_js('Rôle déclaré : ')+(identity.role||tr_web_pilot_inspector_js('Rôle à préciser'))),node('p',tr_web_pilot_inspector_js('Processus : ')+explanation.state),node('p',tr_web_pilot_inspector_js('Activité : ')+explanation.operation),node('p',tr_web_pilot_inspector_js('Prochaine action : ')+explanation.next),node('p',tr_web_pilot_inspector_js('Validation : ')+(labels[validation?.state||t?.status]||tr_web_pilot_inspector_js('inconnue'))));
   if(t?.deliverable)health.append(node('p',tr_web_pilot_inspector_js('Résultat attendu : ')+t.deliverable));
   health.append(node('p',typeof a.usage?.provider_reported_cost_usd==='number'?tr_web_pilot_inspector_js('Coût de cette tentative : ')+a.usage.provider_reported_cost_usd.toFixed(2)+' USD':tr_web_pilot_inspector_js('Coût inconnu : le fournisseur n’a pas transmis de montant à Swarm.')));
   const details=node('details');details.id='pilot-activity-details';details.open=activityExpanded;
   details.append(node('summary',tr_web_pilot_inspector_js('Voir la commande et les signaux techniques')));
   details.append(node('p',tr_web_pilot_inspector_js('Validation actuelle de la tâche : ')+(t?.status==='running'?tr_web_pilot_inspector_js('pas encore validée'):labels[validation?.state||t?.status]||'inconnue')));
   for(const [title,text]of [[tr_web_pilot_inspector_js('Exécution'),uncertain||(globalThis.SwarmI18n?.engine(h?.process_label) ?? h?.process_label)||tr_web_pilot_inspector_js('Observation indisponible')],[tr_web_pilot_inspector_js('Activité reçue'),(globalThis.SwarmI18n?.engine(h?.activity_label) ?? h?.activity_label)||tr_web_pilot_inspector_js('Non connue')]]){
    const row=node('p');row.append(node('strong',title+' : '),node('span',text));details.append(row);
   }
   details.append(node('p',tr_web_pilot_inspector_js('Dernière commande ou information reçue (peut être abrégée) :')),node('pre',a.progress?.detail||a.progress?.action||tr_web_pilot_inspector_js('Aucun détail disponible.')));
   if(h?.heartbeat)details.append(node('p',tr_web_pilot_inspector_js('Dernier contact avec le superviseur : ')+new Date(h.heartbeat).toLocaleString('fr')));
   if(a.progress?.last_result_at)details.append(node('p',tr_web_pilot_inspector_js('Dernière réponse d’un outil : ')+new Date(a.progress.last_result_at).toLocaleString('fr')));
   if(a.ended)details.append(node('p',tr_web_pilot_inspector_js('Fin observée : ')+new Date(a.ended).toLocaleString('fr')));
   if(a.exit_code!==undefined)details.append(node('p',tr_web_pilot_inspector_js('Code de sortie : ')+a.exit_code));
   health.append(details);body.insertBefore(health,mission.nextSibling);
   if(a.parent||a.previous)body.append(this.relations(a));
  }
  if(t){
   const descendants=PilotGraph.descendants(snapshot.work.tasks,t.id);
   const blockers=snapshot.pilotage?.tasks[t.id]?.waiting_on||[];
   const impacts=node('section',undefined,'pilot-section');
   impacts.append(node('h4',tr_web_pilot_inspector_js('Dépendances et impact')),node('p',descendants.length?descendants.length+tr_web_pilot_inspector_js(' tâches en aval : ')+descendants.join(', ')+'. Elles peuvent avoir d’autres prérequis.':tr_web_pilot_inspector_js('Aucune tâche ne dépend de celle-ci.')));
   if(blockers.length)impacts.append(node('p',tr_web_pilot_inspector_js('Prérequis manquants : ')+blockers.join(', ')));
   for(const message of validation?.blockers||[])impacts.append(node('p',message,'notice attention'));
   body.append(impacts);
   const actions=node('section',undefined,'pilot-section');actions.id='pilot-inspector-commands';
   actions.append(node('h4',tr_web_pilot_inspector_js('Que faire maintenant ?')));
   if(t.status==='submitted')actions.append(node('p',tr_web_pilot_inspector_js('L’exécution a produit un résultat. Lisez le rapport et les preuves avant toute décision de validation.')));
   if(Pilot.queue&&selected.kind==='task'&&!['blocked','submitted'].includes(t.status))actions.append(node('p',tr_web_pilot_inspector_js('Cette tâche ne requiert plus cette intervention. Son état a changé.'),'notice info'));
   if(t.status==='blocked')actions.append(node('p',t.blocker||tr_web_pilot_inspector_js('Examiner le motif de blocage avant toute reprise.')));
   const opts=snapshot.task_actions?.[t.id]||[];
   const primary=opts.find(x=>x.conseillee&&x.disponible);
   if(h?.stop_requested)actions.append(node('p',tr_web_pilot_inspector_js('Arrêt déjà demandé — confirmation attendue.'),'notice info'));
   if(t.status!=='submitted'&&primary&&!(primary.kind==='stop'&&h?.stop_requested))actions.append(this.action(primary.kind==='report'?tr_web_pilot_inspector_js('Ouvrir les conclusions du rapport'):primary.label,()=>this.taskAction(t.id,a?.id,primary.kind),'primary','primary'));
   actions.append(this.action(tr_web_pilot_inspector_js('Toutes les actions autorisées'),()=>taskDialog(t.id,a?.id),'actions'));
   for(const x of opts.filter(x=>!x.disponible&&['start','retry','stop','accepted'].includes(x.kind)))actions.append(node('p',x.label+' : '+x.raison,'pilot-muted'));
   body.append(actions);
   const reports=node('section',undefined,'pilot-section');reports.id='pilot-reports';reports.dataset.review=String(t.status==='submitted');reports.append(node('h4',tr_web_pilot_inspector_js('Rapports et preuves de la tâche')),node('p',tr_web_pilot_inspector_js('Chargement des rapports disponibles…')));body.append(reports);
   api('/api/v1/task?'+new URLSearchParams({work,task:t.id})).then(data=>{
    if(token!==this.generation||work!==requestWork||!reports.isConnected)return;
    reports.replaceChildren(node('h4',tr_web_pilot_inspector_js('Rapports et preuves de la tâche')));
    if(!data.reports?.length)reports.append(node('p',tr_web_pilot_inspector_js('Aucun rapport détecté pour cette tâche.')));
    for(const path of data.reports||[])reports.append(this.action(tr_web_pilot_inspector_js('Ouvrir ')+path.split('/').pop(),()=>this.readReport(path,t.id),'report:'+path));
    reports.append(this.action(tr_web_pilot_inspector_js('Examiner les contrôles de validation'),()=>this.taskAction(t.id,a?.id,'gate'),'gate'));
   }).catch(e=>{if(token===this.generation&&reports.isConnected)reports.append(node('p',tr_web_pilot_inspector_js('Rapports indisponibles : ')+e.message,'notice alert'))});
  }
  this.queueFooter(body);
  const technical=node('details',undefined,'pilot-section');technical.append(node('summary',tr_web_pilot_inspector_js('Identifiants et détails techniques')));
  const projectedAttempt=snapshot.pilotage?.tasks?.[tid]?.attempt||{};
  for(const [name,value]of [[tr_web_pilot_inspector_js('Tâche'),t?.id],[tr_web_pilot_inspector_js('Session agent'),a?.id||projectedAttempt.agent_id],[tr_web_pilot_inspector_js('Tentative métier'),a?.attempt_id||projectedAttempt.attempt_id],[tr_web_pilot_inspector_js('Espace de travail'),a?.workspace],[tr_web_pilot_inspector_js('Livrable attendu'),t?.deliverable]])if(value)technical.append(node('p',name+' : '+value));
  body.append(technical);
  if(open)panel.scrollTop=0;
  else if(focusKey)[...body.querySelectorAll('[data-inspector-action]')].find(n=>n.dataset.inspectorAction===focusKey)?.focus({preventScroll:true});
 },
 activityExplanation(a,h,uncertain=''){
  const progress=a.progress||{},detail=progress.detail||'',action=progress.action||'';

```

### web/pilotage.js : 96–100
```js
 },
 taskAgent(id){const agent=snapshot.pilotage?.tasks?.[id]?.attempt?.agent_id;return snapshot.agents.find(x=>x.agent.id===agent)?.agent||snapshot.agents.find(x=>x.agent.task_id===id)?.agent},
 inspect(kind,id){const a=kind==='agent'?snapshot.agents.find(x=>x.agent.id===id)?.agent:kind==='task'?this.taskAgent(id):null;if(['terminal','dialogue'].includes(a?.mode)){AgentTerminal.open(a);return}this.state.selection={kind,id,agent_id:a?.id||''};this.inspectorKey='';this.save();PilotInspector.render(true)},
 selectedTask(){
  const s=this.state.selection;

```

### Addendum de fraîcheur avant remise bornée

La recette a été complétée par supervision après refus B03 : toucheEspace réelle pour choisir la deuxième tâche, capture inspecteur après sélection de tentative, SHA256 des captures dans sortie hôte. Ces ajouts de preuve ne modifient pas le code produit. L’empreinte courante de tests/graph_draft_ui.cjs est cd21188db227987b6277c2a6b5f064a85fc9c4f03e90894e83117ac4242e6b07. Les empreintes anciennes ci-dessus sont historiques ; la policy et le receipt frais lient les entrées courantes. Aucune capture n’est présentée comme inspectée par le vérificateur Codex qui ne reçoit pas les pixels.

### Empreintes courantes de supervision

- graph_draft_projection.go : a4085936414f083283a6a3dac6738525a7e114954d2d91d5e6917fb0c1485121
- graph_draft_projection_test.go : d1238c84dd92d8a676ffaee5e474590106179e596ea9d9e21c69778bb8dc1818
- graph_draft.go : c396c9d2303e213f88e8416ce5bd8aadfbf926077301c691eab2a389e2eb470f
- automatic_validation.go : f191d733f754a7086696eef4127aa10cd26692cff7c2d1adf4cefbff988bb201
- tests/graph_draft_ui.cjs : cd21188db227987b6277c2a6b5f064a85fc9c4f03e90894e83117ac4242e6b07


## Recette Go complète actuelle — supervision

Même inventaire, seize tranches disjointes concurrentes et timeout240s inchangé. Aucun cas manquant ne passe.

```python
"""Compile once; run every discovered Go test in disjoint bounded processes."""
import concurrent.futures
import json
import re
import subprocess
import tempfile
import time
from pathlib import Path


def partition(names, plan):
    if not names or len(names) != len(set(names)):
        raise ValueError('empty or duplicate test inventory')
    isolated = plan['isolated']
    if len(isolated) != len(set(isolated)) or not set(isolated) <= set(names):
        raise ValueError('isolated test inventory mismatch')
    remaining = set(names) - set(isolated)
    groups = [[] for _ in range(min(plan['groups'], len(remaining)))]
    weights = plan['observed_seconds']
    costs = [0.0 for _ in groups]
    # Greedy scheduling changes order only. Unknown durations have a conservative
    # unit cost; observed durations under load are hints, never acceptance data.
    for name in sorted(remaining, key=lambda n: (-weights.get(n, 1), n)):
        index = min(range(len(groups)), key=lambda i: (costs[i], len(groups[i]), i))
        groups[index].append(name)
        costs[index] += weights.get(name, 1)
    all_groups = [[name] for name in isolated] + groups
    if sorted(name for group in all_groups for name in group) != sorted(names):
        raise ValueError('incomplete or duplicate coverage')
    return all_groups, len(isolated)


def run(group, index, binary, package, timeout):
    pattern = '^(?:' + '|'.join(re.escape(name) for name in group) + ')$'
    command = ['go', 'tool', 'test2json', '-t', '-p', package, str(binary),
               '-test.run', pattern, '-test.count=1', '-test.v=test2json',
               f'-test.timeout={timeout}s']
    started = time.monotonic()
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    observed, passed, skipped = set(), set(), {}
    last = started
    failures = []
    for line in process.stdout:
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            failures.append(line.rstrip())
            continue
        test, action = event.get('Test', ''), event.get('Action')
        if action == 'run' and '/' not in test:
            observed.add(test)
        if action == 'pass' and test and '/' not in test:
            passed.add(test)
        if action == 'skip':
            skipped[test] = list(failures[-5:])
        if action == 'run' and time.monotonic() - last >= 15:
            print(f'SHARD {index} RUN {test} elapsed={time.monotonic()-started:.1f}s', flush=True)
            last = time.monotonic()
        if action == 'output':
            failures.append(event.get('Output', '').rstrip())
            failures = failures[-50:]
        if action == 'fail':
            print(f'SHARD {index} FAIL {test}\n' + '\n'.join(failures), flush=True)
    code = process.wait()
    missing = set(group) - observed
    incomplete = set(group) - passed - set(skipped)
    # Required feature cases may never disappear behind an optional skip.
    required_skips = {name for name in skipped if name.startswith(('TestGraphDraftB', 'TestGraphPerformance', 'TestReviewTokenCache'))}
    print(f'SHARD {index} exit_code={code} elapsed={time.monotonic()-started:.1f}s covered={len(observed)}/{len(group)} passed={len(passed)}', flush=True)
    for name, reason in skipped.items():
        print(json.dumps({'optional_skip': name, 'observed_output': reason}), flush=True)
    if code or missing or incomplete or required_skips:
        print(f'SHARD {index} missing={sorted(missing)} incomplete={sorted(incomplete)} required_skips={sorted(required_skips)}\n' + '\n'.join(failures), flush=True)
        return 1
    return 0


def main():
    plan = json.loads(Path('tests/supervision_go_schedule.json').read_text())
    assert plan['version'] == 1 and 1 <= plan['parallelism'] <= 16
    assert 1 <= plan['groups'] <= 16 and 0 < plan['test_timeout_seconds'] <= 240
    assert all(isinstance(v, (float, int)) and 0 <= v < 10000 for v in plan['observed_seconds'].values())
    packages = subprocess.run(['go', 'list', './...'], capture_output=True, text=True, check=True).stdout.splitlines()
    if len(packages) != 1:
        raise SystemExit('Package inventory changed: update suite coverage before running.')
    with tempfile.TemporaryDirectory(prefix='swarm-go-suite-') as folder:
        binary = Path(folder)/'companion.test'
        subprocess.run(['go', 'test', '-c', '-o', str(binary), packages[0]], check=True)
        listing = subprocess.run([str(binary), '-test.list', '.'], capture_output=True, text=True, check=True)
        names = [name for name in listing.stdout.splitlines() if re.match(r'^(Test|Example|Fuzz)\w*$', name)]
        groups, isolated = partition(names, plan)
        print(f'GO SUITE package={packages[0]} tests={len(names)} shards={len(groups)} coverage=disjoint-complete compile=once', flush=True)
        codes = [run(groups[i], i, binary, packages[0], plan['test_timeout_seconds']) for i in range(isolated)]
        with concurrent.futures.ThreadPoolExecutor(max_workers=plan['parallelism']) as pool:
            codes.extend(pool.map(lambda pair: run(pair[1], pair[0]+isolated, binary, packages[0], plan['test_timeout_seconds']), enumerate(groups[isolated:])))
        if any(codes):
            raise SystemExit(1)
        print(f'PASS full discovered Go suite: {len(names)} tests; all shards exited 0.', flush=True)
        print('Optional Go skips are listed explicitly above, never counted as passed; live-provider probes and opt-in browser recipes are not implied.', flush=True)


if __name__ == '__main__':
    main()
```
