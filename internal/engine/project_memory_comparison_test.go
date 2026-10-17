//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Produces comparable inputs, never a model-quality result or a provider call.
func TestProjectMemoryComparisonContexts(t *testing.T) {
	s := storeTest(t)
	source, retex := projectMemorySourceFixture(t, s)
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: source.Revision, Work: source.ID, ID: "import-proof-procedure", SourceRetex: retex.ID, Kind: "procedure", Body: "Une archive importée conserve des preuves historiques sans installer nécessairement les artefacts dans la racine cible. Avant admission, comparer les empreintes de l'original récupérable. Si elles correspondent, vérifier l'archive, restaurer l'artefact puis évaluer la cible actuelle. Sans original vérifiable, bloquer l'admission et demander une preuve fraîche. Ne pas remplacer une preuve courante différente par une copie non vérifiée ; une ancienne acceptation ne valide pas la cible.", Conditions: "Archive et cible distinctes ; vérifier disponibilité, empreintes et contexte courant. Une restauration n'est possible que depuis un original vérifié.", Reason: "Comparer une décision et sa variante, sans généraliser l'ancienne réussite"}
	q = memoryPreparedRequest(t, s, "admit", q)
	value, err := s.projectMemoryOperation("admit", q)
	if err != nil {
		t.Fatal(err)
	}
	source = value.(Work)
	v := source.ProjectMemory[0].Versions[0]
	ref := MemoryReference{source.ID, q.ID, v.Version, v.SHA256}

	pack := map[string]any{"schema_version": 1, "provenance": "engine-preview-fixture; no provider invocation", "lesson": v}
	cases := []map[string]any{}
	prompts := map[string]string{}
	replays := map[string]any{}
	normalize := func(prompt string, revision int) string {
		t.Helper()
		needle := fmt.Sprintf(" ; révision du travail %d.\n", revision)
		if strings.Count(prompt, needle) != 1 {
			t.Fatal("ambiguous identity revision")
		}
		return strings.Replace(prompt, needle, " ; révision du travail [neutralisée pour comparaison].\n", 1)
	}
	for _, scenario := range []string{"missing-recoverable", "changed-unrecoverable"} {
		t.Run(scenario, func(t *testing.T) {
			dest, launch := setupAgent(t, s)
			facts := map[string]any{"artifact": "proof.txt", "required_sha256": hash([]byte("proof")), "target_state": "missing", "target_sha256": nil, "archive_available": true, "archive_sha256": hash([]byte("proof")), "historical_task_status": "accepted"}
			if scenario == "changed-unrecoverable" {
				facts["target_state"], facts["target_sha256"] = "different", hash([]byte("new candidate"))
				facts["archive_available"], facts["archive_sha256"] = false, nil
			}
			factsJSON, err := json.Marshal(facts)
			if err != nil {
				t.Fatal(err)
			}
			launch.Instruction = "EXERCICE DE DECISION UNIQUEMENT : proposer une action, sans outils, accès aux fichiers, lancement ou modification. La cible décrite ci-dessous est synthétique et distincte de la racine du contexte. Ne pas annoncer d'exécution. Une archive conserve une tâche historiquement acceptée ; décider comment traiter l'admission d'une connaissance dans la cible actuelle. Répondre uniquement par un objet JSON avec exactement : decision (restore_and_evaluate, rebuild_evidence, admit_directly ou accept_task), actions (liste ordonnée parmi verify_archive, restore_artifact, evaluate_current, block_admission, request_fresh_evidence, import_acceptance), admission (after_fresh_pass, blocked ou now), acceptance (unchanged ou accepted), reason (explication non vide). OBSERVATIONS_COURANTES : " + string(factsJSON)
			baseline, created, err := s.prepareLaunch(dest.ID, launch, true)
			if err != nil || created {
				t.Fatal("baseline preview", err)
			}
			baselineRevision := dest.Revision
			unchanged, err := s.get(dest.ID)
			if err != nil || unchanged.Revision != dest.Revision || len(unchanged.Tasks[0].Attempts) != 0 {
				t.Fatal("baseline preview mutated task", err)
			}
			selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: dest.Revision, Work: dest.ID, Task: launch.TaskID, Reason: "Procédure pertinente à confronter aux observations courantes", Selections: []MemoryReference{ref}}
			selection = memoryPreparedRequest(t, s, "select", selection)
			value, err := s.projectMemoryOperation("select", selection)
			if err != nil {
				t.Fatal(err)
			}
			dest = value.(Work)
			launch.Revision = dest.Revision
			treatment, created, err := s.prepareLaunch(dest.ID, launch, true)
			if err != nil || created {
				t.Fatal("treatment preview", err)
			}
			body, err := s.memoryAttachments(s.db, []MemoryReference{ref}, "worker")
			if err != nil {
				t.Fatal(err)
			}
			decision := dest.Tasks[0].MemoryDecision
			delta := body + fmt.Sprintf("Décision de sélection mémoire : %s ; auteur : %s ; motif : %s\n", decision.EventID, decision.Actor, decision.Reason)
			baselineInput := normalize(baseline.Prompt, baselineRevision)
			treatmentInput := normalize(treatment.Prompt, dest.Revision)
			if strings.Count(treatmentInput, delta) != 1 || strings.Replace(treatmentInput, delta, "", 1) != baselineInput {
				t.Fatal("paired inputs differ beyond selected memory and its attribution")
			}
			unchanged, err = s.get(dest.ID)
			if err != nil || unchanged.Revision != dest.Revision || len(unchanged.Tasks[0].Attempts) != 0 {
				t.Fatal("treatment preview mutated task", err)
			}
			for _, entry := range []struct {
				name     string
				agent    Agent
				revision int
			}{{"without", baseline, baselineRevision}, {"with", treatment, dest.Revision}} {
				id := scenario + "-" + entry.name
				if entry.agent.Context.SHA256 != hash([]byte(entry.agent.Prompt)) || !strings.Contains(entry.agent.Prompt, string(factsJSON)) {
					t.Fatal("context or observations not bound")
				}
				input := normalize(entry.agent.Prompt, entry.revision)
				prompts[id+".engine.txt"] = entry.agent.Prompt
				prompts[id+".txt"] = input
				cases = append(cases, map[string]any{"id": id, "scenario": scenario, "treatment": entry.name, "prompt": id + ".txt", "prompt_sha256": hash([]byte(input)), "raw_prompt": id + ".engine.txt", "source_revision": entry.revision, "normalization": "work-revision-only-v1", "context": entry.agent.Context, "facts": facts})
			}
			// Replay the proposed local artifact decision through the actual gate.
			// This is a deterministic control of the fixture, never an LLM result.
			target := t.TempDir()
			if scenario == "changed-unrecoverable" {
				if err := os.WriteFile(filepath.Join(target, "proof.txt"), []byte("new candidate"), 0600); err != nil {
					t.Fatal(err)
				}
			}
			sourceTask, err := source.task(retex.Task)
			if err != nil {
				t.Fatal(err)
			}
			before, beforeErr := evaluate(sourceTask.Gate.Document, target, "delivery")
			if beforeErr == nil && before.Ship {
				t.Fatal("historical acceptance authorized a missing or changed target")
			}
			replay := map[string]any{"before_ship_allowed": before.Ship, "fixture_only": true}
			if scenario == "missing-recoverable" {
				if hash([]byte("proof")) != facts["required_sha256"] {
					t.Fatal("archive digest changed")
				}
				if err := os.WriteFile(filepath.Join(target, "proof.txt"), []byte("proof"), 0600); err != nil {
					t.Fatal(err)
				}
				after, err := evaluate(sourceTask.Gate.Document, target, "delivery")
				if err != nil || !after.Ship {
					t.Fatal("restored target not verified", err)
				}
				replay["after_ship_allowed"] = after.Ship
			} else if raw, err := os.ReadFile(filepath.Join(target, "proof.txt")); err != nil || string(raw) != "new candidate" {
				t.Fatal("variant was overwritten", err)
			}
			replays[scenario] = replay
			t.Logf("matched pair: baseline %d bytes; selected %d bytes; only %d memory/attribution bytes added", len(baseline.Prompt), len(treatment.Prompt), len(delta))
		})
	}
	var agents int
	if err := s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents); err != nil || agents != 0 {
		t.Fatal("comparison preparation launched or stored an agent", agents, err)
	}
	if len(cases) != 4 {
		t.Fatal("incomplete comparison", len(cases))
	}
	if out := os.Getenv("SWARM_MEMORY_COMPARISON_OUT"); out != "" && !t.Failed() {
		if !filepath.IsAbs(out) {
			t.Fatal("comparison output must be absolute")
		}
		if err := os.Mkdir(out, 0700); err != nil {
			t.Fatal("output must be a new directory", err)
		}
		for name, body := range prompts {
			if err := os.WriteFile(filepath.Join(out, name), []byte(body), 0600); err != nil {
				t.Fatal(err)
			}
		}
		pack["cases"] = cases
		pack["gate_replays"] = replays
		raw, err := json.MarshalIndent(pack, "", "  ")
		if err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(out, "manifest.json"), append(raw, '\n'), 0600); err != nil {
			t.Fatal(err)
		}
		t.Log("exported four complete fixture contexts; model effectiveness NOT TESTED")
	}
}
