//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"testing"
)

// Mesure du nombre de gestes humains nécessaires pour mener trois tâches de
// « à faire » à « acceptée », sur un parcours identique.
//
// Geste humain = appel au moteur qu'aucun automatisme ne peut produire à la
// place de l'opérateur. L'exécution de la tentative elle-même n'en est pas un.
// Les deux parcours ne diffèrent que par un point : l'agent écrit ou non son
// rapport là où la consigne le lui demande. Aucun drapeau ne désactive le
// conducteur : sans rapport, il refuse, ce qui reproduit exactement le
// comportement antérieur à SC-17.

type interruptionCount struct {
	t     *testing.T
	steps int
}

func (c *interruptionCount) human(label string, act func() error) {
	c.t.Helper()
	c.steps++
	if e := act(); e != nil {
		c.t.Fatalf("geste humain %q (n°%d) en échec : %v", label, c.steps, e)
	}
}

func parcoursProvider(t *testing.T, s *Store, writeReport bool) Providers {
	t.Helper()
	script := filepath.Join(s.root, "provider.sh")
	body := `#!/bin/sh
prompt=$(cat)
cible=$(printf '%s' "$prompt" | grep -o 'docs/[A-Za-z0-9_-]*\.md' | head -n 1)
if [ -n "$SWARM_ECRIT_RAPPORT" ] && [ -n "$cible" ]; then
  mkdir -p "$SWARM_RACINE/docs"
  printf 'Handoff : tests exécutés, preuves listées, prochaine action.\n' > "$SWARM_RACINE/$cible"
fi
echo "tentative terminée"
`
	if e := os.WriteFile(script, []byte(body), 0700); e != nil {
		t.Fatal(e)
	}
	t.Setenv("SWARM_RACINE", s.root)
	if writeReport {
		t.Setenv("SWARM_ECRIT_RAPPORT", "1")
	} else {
		os.Unsetenv("SWARM_ECRIT_RAPPORT")
	}
	p := Providers{Schema: 1, Providers: map[string]Provider{
		"fixture": {Command: script, Env: []string{"SWARM_RACINE", "SWARM_ECRIT_RAPPORT"}},
	}}
	raw, _ := json.Marshal(p)
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}
	return p
}

func gateDocument(t *testing.T, s *Store, task string) []byte {
	t.Helper()
	proof := "proof-" + task + ".txt"
	if e := os.WriteFile(filepath.Join(s.root, proof), []byte("preuve "+task), 0600); e != nil {
		t.Fatal(e)
	}
	return []byte(fmt.Sprintf(`{"method_version":"2","scope_id":%q,"artifacts":{%q:%q},"domains":{"quality":100},"checks":[{"id":"q","domain":"quality","mandatory":true,"gate":"delivery","penalty":10,"max_penalty":100,"severity":"minor"}],"results":[{"id":"q","status":"PASS","count":0,"evidence":[%q]}]}`,
		task, proof, hash([]byte("preuve "+task)), proof))
}

// parcours mène trois tâches jusqu'à l'acceptation et retourne le nombre de
// gestes humains consommés sous un régime donné.
//
// Les trois régimes correspondent à l'état du produit avant et après chaque
// lot ; ils ne diffèrent que par ce que le moteur fait de lui-même :
//
//	manuel   — état antérieur à SC-17 : l'agent n'écrit pas son rapport là où
//	           la consigne le demande, donc le conducteur refuse de relayer
//	assiste  — SC-17 : le handoff prouvé est relayé
//	autonome — SC-18 : les départs suivants sont ordonnancés
//
// En régime autonome, la décision de départ est prise par `planDispatch`,
// l'oracle réellement livré ; l'exécution passe ensuite par `supervise` au lieu
// du superviseur détaché, qui n'a pas de sens dans un binaire de test. La
// boucle complète de `dispatch` est couverte séparément par
// `TestDispatchLaunchesFromCapturedProfile`.
func parcours(t *testing.T, regime string) int {
	t.Helper()
	s := storeTest(t)
	parcoursProvider(t, s, regime != "manuel")
	count := &interruptionCount{t: t}

	w := createTest(t, s)
	if regime == "autonome" {
		if e := s.setAutonomy(w.ID, autonomyAuto, 1); e != nil {
			t.Fatal(e)
		}
	}
	for i := 1; i <= 3; i++ {
		id := fmt.Sprintf("t%d", i)
		w = applyTest(t, s, w, "task.add", Request{ID: id, Title: "Tâche " + id, Deliverable: "rapport", Criteria: []string{"preuves"}, Owner: "fixture", Next: "lancer"})
	}

	for i := 1; i <= 3; i++ {
		id := fmt.Sprintf("t%d", i)
		workspace := filepath.Join(s.root, "ws-"+id)
		if e := os.MkdirAll(workspace, 0700); e != nil {
			t.Fatal(e)
		}
		var agent Agent
		if regime == "autonome" && i > 1 {
			// Le moteur a lancé de lui-même à la fin de la tentative précédente :
			// on constate son départ au lieu d'en demander un.
			agent = departAutomatique(t, s, w.ID, id)
		} else {
			count.human("lancer "+id, func() error {
				current, e := s.get(w.ID)
				if e != nil {
					return e
				}
				a, _, e := s.prepare(w.ID, Launch{Schema: 1, EventID: newID("agent-"), Revision: current.Revision, TaskID: id, Provider: "fixture", Workspace: workspace, Role: "worker", Capture: true})
				agent = a
				return e
			})
		}
		// Exécution de la tentative : travail de l'agent, pas un geste humain.
		if e := s.supervise(agent.ID); e != nil {
			t.Fatal(e)
		}

		current, e := s.get(w.ID)
		if e != nil {
			t.Fatal(e)
		}
		task, e := current.task(id)
		if e != nil {
			t.Fatal(e)
		}
		if task.Status == "blocked" {
			count.human("relayer le rapport de "+id, func() error {
				report := filepath.Join("docs", id+".md")
				if e := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); e != nil {
					return e
				}
				if _, err := os.Stat(filepath.Join(s.root, report)); err != nil {
					if e := os.WriteFile(filepath.Join(s.root, report), []byte("Handoff repris à la main."), 0600); e != nil {
						return e
					}
				}
				return s.submitReport(w.ID, id, report)
			})
		}

		document := gateDocument(t, s, id)
		count.human("enregistrer la gate de "+id, func() error {
			current, e := s.get(w.ID)
			if e != nil {
				return e
			}
			ev, e := evaluate(document, s.root, "delivery")
			if e != nil {
				return e
			}
			r := Request{Schema: 1, EventID: newID("gate-"), Revision: current.Revision}
			raw, _ := json.Marshal(r)
			_, e = s.mutate(w.ID, "gate", r.EventID, r.Revision, raw, func(x *Work) error {
				for j := range x.Tasks {
					if x.Tasks[j].ID == id {
						x.Tasks[j].Gate = &GateRecord{Name: "delivery " + id, Document: document, Evaluation: ev, At: now()}
					}
				}
				return nil
			})
			return e
		})

		count.human("accepter "+id, func() error {
			current, e := s.get(w.ID)
			if e != nil {
				return e
			}
			w = applyTest(t, s, current, "task.update", Request{ID: id, Status: "accepted", Next: "tâche acceptée"})
			return nil
		})
	}

	final, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	for _, task := range final.Tasks {
		if task.Status != "accepted" {
			t.Fatalf("parcours incomplet : %s reste %q", task.ID, task.Status)
		}
	}
	return count.steps
}

// departAutomatique retrouve la tentative que l'ordonnanceur a créée seul à la
// fin de la précédente. Le superviseur détaché n'a pas de sens dans un binaire
// de test : la tentative est ensuite menée par `supervise`, comme les autres.
func departAutomatique(t *testing.T, s *Store, work, task string) Agent {
	t.Helper()
	agents, e := s.agents(work)
	if e != nil {
		t.Fatal(e)
	}
	for _, a := range agents {
		if a.TaskID == task && a.Origin == originConductor {
			return a
		}
	}
	t.Fatalf("aucun départ automatique enregistré pour %s", task)
	return Agent{}
}

func TestParcoursInterruptionsParRegime(t *testing.T) {
	mesures := map[string]int{}
	for _, regime := range []string{"manuel", "assiste", "autonome"} {
		mesures[regime] = parcours(t, regime)
	}
	t.Logf("gestes humains sur trois tâches menées à l'acceptation : manuel %d · assisté %d · autonome %d",
		mesures["manuel"], mesures["assiste"], mesures["autonome"])
	attendu := map[string]int{"manuel": 12, "assiste": 9, "autonome": 7}
	for regime, n := range attendu {
		if mesures[regime] != n {
			t.Fatalf("régime %s : %d gestes attendus, %d mesurés", regime, n, mesures[regime])
		}
	}
}
