//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// The proposed V3 branches follow the real CLI inspection contract. Every DB
// here belongs to storeTest; no provider, live project or migration is invoked.
func TestProjectMemoryStoragePlanContexts(t *testing.T) {
	model, err := os.ReadFile("model.go")
	if err != nil {
		t.Fatal(err)
	}
	storeSource, err := os.ReadFile("store.go")
	if err != nil {
		t.Fatal(err)
	}
	if schemaVersion < 2 || strings.Count(string(model), fmt.Sprintf("const schemaVersion = %d\n", schemaVersion)) != 1 {
		t.Fatal("source declaration differs from the compiled version")
	}
	for _, guard := range []string{"version < 0 || version > schemaVersion", "!migrate && version > 0 && version < schemaVersion"} {
		if !strings.Contains(string(storeSource), guard) {
			t.Fatal("inspection contract changed; re-examine V3", guard)
		}
	}
	controls := []map[string]any{}
	for _, delta := range []int{-1, 0, 1} {
		t.Run(fmt.Sprintf("cli-version-%d", schemaVersion+delta), func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			snapshot := func() string {
				var body []byte
				if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&body); err != nil {
					t.Fatal(err)
				}
				events, err := s.events(w.ID)
				if err != nil {
					t.Fatal(err)
				}
				raw, err := json.Marshal(events)
				if err != nil {
					t.Fatal(err)
				}
				return string(body) + "\n" + string(raw)
			}
			before := snapshot()
			backupsBefore, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-*.db"))
			if err != nil {
				t.Fatal(err)
			}
			version := schemaVersion + delta
			if _, err := s.db.Exec(fmt.Sprintf("PRAGMA user_version=%d", version)); err != nil {
				t.Fatal(err)
			}
			var stdout, stderr bytes.Buffer
			code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &stdout, &stderr)
			if delta == 0 {
				var result struct {
					Work Work `json:"work"`
				}
				if code != 0 || json.Unmarshal(stdout.Bytes(), &result) != nil || result.Work.ID != w.ID || result.Work.Title != w.Title || result.Work.Revision != w.Revision {
					t.Fatal("same-version CLI did not return the actual work", code, stdout.String(), stderr.String())
				}
			} else if code != 2 || stdout.Len() != 0 || delta < 0 && !strings.Contains(stderr.String(), "storage_upgrade_required") || delta > 0 && !strings.Contains(stderr.String(), "version de stockage non supportée") {
				t.Fatal("CLI did not refuse unsupported inspection", code, stdout.String(), stderr.String())
			}
			var after int
			if err := s.db.QueryRow("PRAGMA user_version").Scan(&after); err != nil || after != version || snapshot() != before {
				t.Fatal("inspection changed version, work or events", after, err)
			}
			backupsAfter, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-*.db"))
			if err != nil || !reflect.DeepEqual(backupsBefore, backupsAfter) {
				t.Fatal("inspection produced a migration backup", err)
			}
			controls = append(controls, map[string]any{"storage_version": version, "exit_code": code, "work_and_events_preserved": true, "storage_version_preserved": true, "migration_backup_created": false})
		})
	}

	s := storeTest(t)
	source, retex := projectMemorySourceFixture(t, s)
	for name, raw := range map[string][]byte{"contract-model.go.txt": model, "contract-store.go.txt": storeSource} {
		if err := os.WriteFile(filepath.Join(s.root, name), raw, 0600); err != nil {
			t.Fatal(err)
		}
	}
	task, err := source.task(retex.Task)
	if err != nil {
		t.Fatal(err)
	}
	var document map[string]any
	if err := json.Unmarshal(task.Gate.Document, &document); err != nil {
		t.Fatal(err)
	}
	document["artifacts"] = map[string]string{"contract-model.go.txt": hash(model), "contract-store.go.txt": hash(storeSource)}
	for _, result := range document["results"].([]any) {
		result.(map[string]any)["evidence"] = []string{"contract-model.go.txt", "contract-store.go.txt"}
	}
	gate, err := json.Marshal(document)
	if err != nil {
		t.Fatal(err)
	}
	evaluation, err := evaluate(gate, s.root, "delivery")
	if err != nil || !evaluation.Ship {
		t.Fatal("source snapshots failed verification", err)
	}
	source, err = s.mutate(source.ID, "fixture.source-contract", newID("e-"), source.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(retex.Task)
		task.Gate = &GateRecord{Document: gate, Evaluation: evaluation, At: now()}
		r, _ := findRetex(w, retex.ID)
		r.Problem, r.Proposal, r.Proof = "Compatibilité du stockage", "Distinguer version ancienne, égale et supérieure", "contract-model.go.txt et contract-store.go.txt"
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	declaration := fmt.Sprintf("schemaVersion = %d", schemaVersion)
	body := fmt.Sprintf("Contrat source vérifié : internal/engine/model.go, %s, SHA-256 %s ; internal/engine/store.go (openStoreWithMigration), SHA-256 %s. Pour une version de stockage positive : si elle est inférieure à la version du CLI, la consultation refuse storage_upgrade_required ; proposer une mise à niveau prise en charge à l'opérateur, sans l'exécuter. Si elle est égale, une consultation est possible sans migration. Si elle est supérieure, ce CLI refuse la version non supportée : demander un CLI compatible, jamais rétrograder. Ces conclusions sont conditionnelles aux deux empreintes courantes identiques. Si elles diffèrent, demander la source courante et ne pas certifier de version. Une branche explicitement conditionnelle ne prouve pas que sa condition est satisfaite. Une compatibilité de stockage ne donne aucune autorisation de lancement métier.", declaration, hash(model), hash(storeSource))
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: source.Revision, Work: source.ID, ID: "storage-source-plan", SourceRetex: retex.ID, Kind: "procedure", Body: body, Conditions: "Comparer les deux empreintes avant réemploi ; distinguer l'action justifiée maintenant et les branches futures.", Reason: "Contrat de code vérifié et trois variantes CLI isolées"}
	q = memoryPreparedRequest(t, s, "admit", q)
	value, err := s.projectMemoryOperation("admit", q)
	if err != nil {
		t.Fatal(err)
	}
	source = value.(Work)
	version := source.ProjectMemory[0].Versions[0]
	ref := MemoryReference{source.ID, q.ID, version.Version, version.SHA256}
	pack := map[string]any{"schema_version": 1, "protocol": "storage-plan-v3", "provenance": "audited source snapshots, isolated CLI controls and engine previews; no provider invocation", "lesson": version, "inspection_controls": controls,
		"knowledge": map[string]any{"supported_version": schemaVersion, "source_sha256": hash(model), "store_sha256": hash(storeSource), "source_snapshot": "contract-model.go.txt", "store_snapshot": "contract-store.go.txt"}}
	outputs := map[string][]byte{"contract-model.go.txt": model, "contract-store.go.txt": storeSource}
	cases := []map[string]any{}
	for _, delta := range []int{-1, 0, 1} {
		for _, freshness := range []string{"current", "changed"} {
			scenario := fmt.Sprintf("storage-%d-%s", schemaVersion+delta, freshness)
			t.Run(scenario, func(t *testing.T) {
				observedHash := hash(model)
				if freshness == "changed" {
					observedHash = hash(append(append([]byte{}, model...), []byte("\n// synthetic destination fingerprint\n")...))
				}
				facts := map[string]any{"workspace_storage_version": schemaVersion + delta, "observed_source_sha256": observedHash, "observed_store_sha256": hash(storeSource)}
				rawFacts, _ := json.Marshal(facts)
				dest, launch := setupAgent(t, s)
				launch.Instruction = "EXERCICE DE PROPOSITION, SANS OUTILS NI EXECUTION. La cible synthétique est distincte de cette racine. Aucun lancement métier n'est autorisé. Un digest ne donne pas la valeur d'une constante. Ne citer une supported_version et source_sha256 que si un contrat applicable est fourni, sinon null et demander la source courante. Comparer les deux empreintes avant réemploi de mémoire. Réponse JSON exacte : current_state (blocked ou allowed, lancement maintenant), supported_version (entier ou null), source_sha256 (empreinte model.go ou null), upgrade_required (unknown, yes, no, unsupported_cli), immediate_actions (liste des seules propositions déjà justifiées : inspect_current_source, propose_supported_upgrade, request_operator_decision, inspect_storage_readonly, request_compatible_cli, edit_database_directly, upgrade_now, downgrade_storage, launch_now, accept_task), conditional_paths (liste facultative de branches {when,then}, différente des actions immédiates), reason (texte non vide). Les conditions possibles sont source_verified_and_storage_older (then propose_supported_upgrade et request_operator_decision), source_verified_and_storage_equal (then inspect_storage_readonly), source_verified_and_storage_newer (then request_compatible_cli), source_verified_upgrade_authorized_and_fresh_check_passed (then inspect_storage_readonly). Une branche ne signifie pas que sa condition est vraie. Proposer une mise à niveau ne l'exécute pas. L'ordre des actions équivalentes est libre. OBSERVATIONS_COURANTES : " + string(rawFacts)
				baseline, _, err := s.prepareLaunch(dest.ID, launch, true)
				if err != nil {
					t.Fatal(err)
				}
				beforeRevision := dest.Revision
				selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: dest.Revision, Work: dest.ID, Task: launch.TaskID, Selections: []MemoryReference{ref}, Reason: "Comparer trois versions et la source applicable"}
				selection = memoryPreparedRequest(t, s, "select", selection)
				value, err := s.projectMemoryOperation("select", selection)
				if err != nil {
					t.Fatal(err)
				}
				dest = value.(Work)
				launch.Revision = dest.Revision
				treatment, _, err := s.prepareLaunch(dest.ID, launch, true)
				if err != nil {
					t.Fatal(err)
				}
				memory, err := s.memoryAttachments(s.db, []MemoryReference{ref}, "worker")
				if err != nil {
					t.Fatal(err)
				}
				decision := dest.Tasks[0].MemoryDecision
				difference := memory + fmt.Sprintf("Décision de sélection mémoire : %s ; auteur : %s ; motif : %s\n", decision.EventID, decision.Actor, decision.Reason)
				normalize := func(raw string, revision int) string {
					needle := fmt.Sprintf(" ; révision du travail %d.\n", revision)
					if strings.Count(raw, needle) != 1 {
						t.Fatal("ambiguous revision normalization")
					}
					return strings.Replace(raw, needle, " ; révision du travail [neutralisée pour comparaison].\n", 1)
				}
				without, with := normalize(baseline.Prompt, beforeRevision), normalize(treatment.Prompt, dest.Revision)
				if strings.Count(with, difference) != 1 || strings.Replace(with, difference, "", 1) != without || strings.Contains(without, declaration) || !strings.Contains(with, declaration) {
					t.Fatal("unmatched pair or source fact leaked to baseline")
				}
				for i, agent := range []Agent{baseline, treatment} {
					name, revision := "without", beforeRevision
					if i == 1 {
						name, revision = "with", dest.Revision
					}
					id, input := scenario+"-"+name, normalize(agent.Prompt, revision)
					outputs[id+".engine.txt"], outputs[id+".txt"] = []byte(agent.Prompt), []byte(input)
					cases = append(cases, map[string]any{"id": id, "scenario": scenario, "treatment": name, "prompt": id + ".txt", "prompt_sha256": hash([]byte(input)), "raw_prompt": id + ".engine.txt", "source_revision": revision, "normalization": "work-revision-only-v1", "context": agent.Context, "facts": facts})
				}
				persisted, err := s.get(dest.ID)
				if err != nil || persisted.Revision != dest.Revision || len(persisted.Tasks[0].Attempts) != 0 {
					t.Fatal("preview mutated the task", err)
				}
				t.Logf("matched pair adds %d bytes of memory and attribution", len(difference))
			})
		}
	}
	var agents int
	if err := s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents); err != nil || agents != 0 || len(cases) != 12 || len(controls) != 3 {
		t.Fatal("incomplete matrix or provider launched", agents, err)
	}
	if out := os.Getenv("SWARM_MEMORY_STORAGE_PLAN_OUT"); out != "" && !t.Failed() {
		if !filepath.IsAbs(out) {
			t.Fatal("output must be absolute")
		}
		if err := os.Mkdir(out, 0700); err != nil {
			t.Fatal("output must be new", err)
		}
		pack["cases"] = cases
		raw, err := json.MarshalIndent(pack, "", "  ")
		if err != nil {
			t.Fatal(err)
		}
		outputs["manifest.json"] = append(raw, '\n')
		for name, raw := range outputs {
			if filepath.Ext(name) == ".go" {
				t.Fatal("snapshots must not introduce a Go package")
			}
			if err := os.WriteFile(filepath.Join(out, name), raw, 0600); err != nil {
				t.Fatal(err)
			}
		}
		t.Log("twelve engine inputs exported; no model invocation")
	}
}
