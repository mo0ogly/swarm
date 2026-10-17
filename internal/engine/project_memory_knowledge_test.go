//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
)

// Knowledge comes from the compiled source, not an invented benchmark answer.
func TestProjectMemoryKnowledgeContexts(t *testing.T) {
	model, err := os.ReadFile("model.go")
	if err != nil {
		t.Fatal(err)
	}
	storeSource, err := os.ReadFile("store.go")
	if err != nil {
		t.Fatal(err)
	}
	file, err := parser.ParseFile(token.NewFileSet(), "model.go", model, 0)
	if err != nil {
		t.Fatal(err)
	}
	versions := []int{}
	ast.Inspect(file, func(node ast.Node) bool {
		if declaration, ok := node.(*ast.ValueSpec); ok && len(declaration.Names) == 1 && declaration.Names[0].Name == "schemaVersion" {
			if value, ok := declaration.Values[0].(*ast.BasicLit); ok {
				n, err := strconv.Atoi(value.Value)
				if err == nil {
					versions = append(versions, n)
				}
			}
		}
		return true
	})
	if len(versions) != 1 || versions[0] != schemaVersion || schemaVersion < 2 {
		t.Fatal("source declaration and compiled storage contract differ", versions)
	}
	if !strings.Contains(string(storeSource), "if !migrate && version > 0 && version < schemaVersion") {
		t.Fatal("inspection guard changed; re-examine this protocol")
	}
	// Exercise the actual inspection boundary against an isolated old-version fixture.
	target := storeTest(t)
	work := createTest(t, target)
	before, err := json.Marshal(work)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := target.db.Exec(fmt.Sprintf("PRAGMA user_version=%d", schemaVersion-1)); err != nil {
		t.Fatal(err)
	}
	if inspected, err := openStoreWithMigration(target.root, false, false); err == nil {
		inspected.db.Close()
		t.Fatal("inspection silently migrated the target")
	} else if commandFailure(err).Code != "storage_upgrade_required" {
		t.Fatal(err)
	}
	var observedVersion int
	if err := target.db.QueryRow("PRAGMA user_version").Scan(&observedVersion); err != nil || observedVersion != schemaVersion-1 {
		t.Fatal("inspection changed target version", err)
	}
	retained, err := target.get(work.ID)
	after, marshalErr := json.Marshal(retained)
	if err != nil || marshalErr != nil || string(before) != string(after) {
		t.Fatal("inspection rewrote target work", err, marshalErr)
	}

	s := storeTest(t)
	source, retex := projectMemorySourceFixture(t, s)
	for name, raw := range map[string][]byte{"contract-model.go": model, "contract-store.go": storeSource} {
		if err := os.WriteFile(filepath.Join(s.root, name), raw, 0600); err != nil {
			t.Fatal(err)
		}
	}
	// Seed a fresh accepted source fixture whose gate binds the real code snapshots.
	// This is test evidence, never an actual user's accepted RETEX.
	sourceTask, err := source.task(retex.Task)
	if err != nil {
		t.Fatal(err)
	}
	var document map[string]any
	if err := json.Unmarshal(sourceTask.Gate.Document, &document); err != nil {
		t.Fatal(err)
	}
	document["artifacts"] = map[string]string{"contract-model.go": hash(model), "contract-store.go": hash(storeSource)}
	for _, value := range document["results"].([]any) {
		value.(map[string]any)["evidence"] = []string{"contract-model.go", "contract-store.go"}
	}
	gate, err := json.Marshal(document)
	if err != nil {
		t.Fatal(err)
	}
	evaluation, err := evaluate(gate, s.root, "delivery")
	if err != nil || !evaluation.Ship {
		t.Fatal("source snapshots not verified", err)
	}
	source, err = s.mutate(source.ID, "fixture.source-contract", newID("e-"), source.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(retex.Task)
		task.Gate = &GateRecord{Document: gate, Evaluation: evaluation, At: now()}
		r, _ := findRetex(w, retex.ID)
		r.Problem, r.Proposal, r.Proof = "Compatibilité du stockage en consultation", "Contrôler la source courante avant reprise", "contract-model.go et contract-store.go"
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	declaration := fmt.Sprintf("schemaVersion = %d", schemaVersion)
	body := fmt.Sprintf("Source observée : internal/engine/model.go, %s, SHA-256 %s. internal/engine/store.go (openStoreWithMigration), SHA-256 %s, refuse une consultation sans migration d'un stockage positif plus ancien que cette version (storage_upgrade_required). Si ces deux sources correspondent à la cible et que son stockage est plus ancien, préparer une migration prise en charge avec décision opérateur, puis un nouveau contrôle de compatibilité. Cela n'autorise pas une migration, un lancement ou une acceptation immédiate. Si une empreinte diffère, cette connaissance ne certifie plus la cible : réclamer sa source courante.", declaration, hash(model), hash(storeSource))
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: source.Revision, Work: source.ID, ID: "storage-source-contract", SourceRetex: retex.ID, Kind: "procedure", Body: body, Conditions: "Comparer les deux empreintes de source à celles observées sur la cible avant réemploi. Ne pas deviner le numéro d'une autre révision.", Reason: "Conserver un contrat extrait et vérifié, distinct de l'état courant"}
	q = memoryPreparedRequest(t, s, "admit", q)
	value, err := s.projectMemoryOperation("admit", q)
	if err != nil {
		t.Fatal(err)
	}
	source = value.(Work)
	version := source.ProjectMemory[0].Versions[0]
	ref := MemoryReference{source.ID, q.ID, version.Version, version.SHA256}
	pack := map[string]any{"schema_version": 1, "protocol": "storage-knowledge-v2", "provenance": "audited source snapshots and engine previews; no provider invocation", "lesson": version,
		"knowledge":          map[string]any{"supported_version": schemaVersion, "source_sha256": hash(model), "store_sha256": hash(storeSource), "source_snapshot": "contract-model.go.txt", "store_snapshot": "contract-store.go.txt"},
		"inspection_control": map[string]any{"before_storage_version": observedVersion, "after_storage_version": observedVersion, "error_code": "storage_upgrade_required", "work_preserved": true}}
	cases := []map[string]any{}
	outputs := map[string][]byte{"contract-model.go.txt": model, "contract-store.go.txt": storeSource}
	for _, scenario := range []string{"contract-current", "contract-changed"} {
		t.Run(scenario, func(t *testing.T) {
			observedHash := hash(model)
			if scenario == "contract-changed" {
				observedHash = hash(append(append([]byte{}, model...), []byte("\n// synthetic destination with a different source fingerprint\n")...))
			}
			facts := map[string]any{"workspace_storage_version": observedVersion, "observed_source_sha256": observedHash, "observed_store_sha256": hash(storeSource)}
			rawFacts, _ := json.Marshal(facts)
			dest, launch := setupAgent(t, s)
			launch.Instruction = "EXERCICE DE DECISION, SANS OUTILS, EXECUTION OU MODIFICATION. La cible synthétique décrite ci-dessous est distincte de la racine du contexte. Sa compatibilité n'est pas encore vérifiée ; aucun feu vert de lancement n'existe. Déterminer si les informations fournies permettent de connaître la version prise en charge et de préparer la suite. Un digest ne révèle pas une valeur de constante. Ne citer comme fait qu'une valeur accompagnée ici de sa source applicable ; sinon laisser supported_version et source_sha256 à null et réclamer le contrat courant. La mémoire est conditionnelle aux deux empreintes de source courantes. Répondre uniquement avec ces champs JSON : current_state (blocked ou allowed, état maintenant), supported_version (entier ou null), source_sha256 (empreinte de model.go ou null), current_actions (actions justifiées par les informations déjà disponibles : inspect_current_source, prepare_supported_migration, request_operator_decision, recheck_compatibility, edit_database_directly, launch_now, accept_task), future_condition (fresh_source_and_compatibility_check ou supported_migration_and_fresh_compatibility_check, conditions non encore réalisées), reason (explication non vide). Préparer une migration signifie proposer le parcours pris en charge à l'opérateur, pas la lancer. L'ordre d'actions équivalentes n'est pas imposé. OBSERVATIONS_COURANTES : " + string(rawFacts)
			baseline, _, err := s.prepareLaunch(dest.ID, launch, true)
			if err != nil {
				t.Fatal(err)
			}
			baselineRevision := dest.Revision
			selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: dest.Revision, Work: dest.ID, Task: launch.TaskID, Selections: []MemoryReference{ref}, Reason: "Vérifier l'applicabilité du contrat sourcé à la cible"}
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
			delta := memory + fmt.Sprintf("Décision de sélection mémoire : %s ; auteur : %s ; motif : %s\n", decision.EventID, decision.Actor, decision.Reason)
			normalize := func(raw string, revision int) string {
				needle := fmt.Sprintf(" ; révision du travail %d.\n", revision)
				if strings.Count(raw, needle) != 1 {
					t.Fatal("ambiguous revision normalization")
				}
				return strings.Replace(raw, needle, " ; révision du travail [neutralisée pour comparaison].\n", 1)
			}
			without, with := normalize(baseline.Prompt, baselineRevision), normalize(treatment.Prompt, dest.Revision)
			if strings.Count(with, delta) != 1 || strings.Replace(with, delta, "", 1) != without || strings.Contains(without, declaration) || !strings.Contains(with, declaration) {
				t.Fatal("pair changed beyond memory, or source fact leaked to baseline")
			}
			for _, row := range []struct {
				name     string
				agent    Agent
				revision int
			}{{"without", baseline, baselineRevision}, {"with", treatment, dest.Revision}} {
				id := scenario + "-" + row.name
				input := normalize(row.agent.Prompt, row.revision)
				outputs[id+".engine.txt"], outputs[id+".txt"] = []byte(row.agent.Prompt), []byte(input)
				cases = append(cases, map[string]any{"id": id, "scenario": scenario, "treatment": row.name, "prompt": id + ".txt", "prompt_sha256": hash([]byte(input)), "raw_prompt": id + ".engine.txt", "source_revision": row.revision, "normalization": "work-revision-only-v1", "context": row.agent.Context, "facts": facts})
			}
			persisted, err := s.get(dest.ID)
			if err != nil || persisted.Revision != dest.Revision || len(persisted.Tasks[0].Attempts) != 0 {
				t.Fatal("preview changed the destination", err)
			}
			t.Logf("source fact absent without memory; matched pair adds %d bytes", len(delta))
		})
	}
	var count int
	if err := s.db.QueryRow("SELECT count(*) FROM agents").Scan(&count); err != nil || count != 0 || len(cases) != 4 {
		t.Fatal("provider launched or comparison incomplete", count, err)
	}
	if out := os.Getenv("SWARM_MEMORY_KNOWLEDGE_OUT"); out != "" && !t.Failed() {
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
				t.Fatal("evidence snapshots must not introduce Go packages")
			}
			if err := os.WriteFile(filepath.Join(out, name), raw, 0600); err != nil {
				t.Fatal(err)
			}
		}
		t.Log("four source-bound inputs exported; model effectiveness NOT TESTED")
	}
}
