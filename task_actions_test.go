//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// tentativeFinie prépare un agent terminé (interrupted) sur t1 : la tâche
// repasse en blocked via le règlement du moteur, et le retry devient
// théoriquement possible (le reste des gardes s'applique). Le Work retourné
// est relu : prepareLaunch incrémente la révision.
func tentativeFinie(t *testing.T, s *Store, w Work) Work {
	t.Helper()
	_, r := setupAgent(t, s)
	r.Revision = w.Revision
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	if e = s.finishAgent(a, "interrupted", "tentative de test interrompue", nil); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	return current
}

// rapportSoumis écrit un handoff détectable puis le soumet : seul chemin
// moteur vers « submitted » depuis todo/blocked après une tentative finie.
func rapportSoumis(t *testing.T, s *Store, w Work) Work {
	t.Helper()
	dir := filepath.Join(s.root, "docs")
	if e := os.MkdirAll(dir, 0700); e != nil {
		t.Fatal(e)
	}
	if e := os.WriteFile(filepath.Join(s.root, "docs", "t1-handoff.md"), []byte("revue après tentative"), 0600); e != nil {
		t.Fatal(e)
	}
	if e := s.submitReport(w.ID, "t1", "docs/t1-handoff.md"); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	return current
}

// oracleFor reconstruit la carte kind -> TaskAction depuis le dernier état
// connu du travail ; les tests table-driven l'utilisent pour affirmer sur
// une action précise sans dépendre de l'ordre de la liste.
func oracleFor(t *testing.T, s *Store, w *Work, id string) map[string]TaskAction {
	t.Helper()
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, e := current.task(id)
	if e != nil {
		t.Fatal(e)
	}
	agents, e := s.agents(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	actions := s.taskActions(&current, task, agents)
	out := map[string]TaskAction{}
	for _, a := range actions {
		out[a.Kind] = a
	}
	return out
}

// TestTaskActionsDisponibiliteParEtat vérifie, pour chaque état de tâche,
// quelles actions l'oracle rend disponibles et lesquelles restent fermées
// avec un motif non vide. Table de vérité du plan s28, alignée sur les
// transitions de store.go.
func TestTaskActionsDisponibiliteParEtat(t *testing.T) {
	cases := []struct {
		nom        string
		preparer   func(t *testing.T, s *Store) Work
		dispo      []string
		indispo    []string
		conseillee string
	}{
		{
			nom:        "todo",
			preparer:   func(t *testing.T, s *Store) Work { return taskTest(t, s, createTest(t, s)) },
			dispo:      []string{"start", "submit", "gate", "override", "report", "assign"},
			indispo:    []string{"stop", "retry", "reconcile", "accepted", "reopen"},
			conseillee: "start",
		},
		{
			nom: "blocked",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				return applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Blocker: "blocage de recette", Outcome: "interrupted"})
			},
			dispo:      []string{"start", "submit", "gate", "override", "report", "assign"},
			indispo:    []string{"stop", "retry", "reconcile", "accepted", "reopen"},
			conseillee: "start",
		},
		{
			nom: "submitted_avec_gate",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				w = tentativeFinie(t, s, w)
				// La tentative finie a repassé la tâche en blocked ; une
				// nouvelle soumission la remet en « submitted ».
				w = rapportSoumis(t, s, w)
				return gateTest(t, s, w, fixture(t, s.root))
			},
			dispo:      []string{"start", "retry", "report", "gate", "accepted", "override", "assign"},
			indispo:    []string{"stop", "reconcile", "submit", "reopen"},
			conseillee: "report",
		},
		{
			nom: "submitted_sans_gate",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				w = tentativeFinie(t, s, w)
				// La tentative finie a repassé la tâche en blocked ; une
				// nouvelle soumission la remet en « submitted ».
				return rapportSoumis(t, s, w)
			},
			dispo:      []string{"start", "retry", "report", "gate", "override", "assign"},
			indispo:    []string{"stop", "reconcile", "submit", "accepted", "reopen"},
			conseillee: "report",
		},
		{
			nom: "accepted",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				w = gateTest(t, s, w, fixture(t, s.root))
				return applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
			},
			dispo:      []string{"report", "reopen", "override", "assign"},
			indispo:    []string{"start", "retry", "stop", "reconcile", "submit", "gate", "accepted"},
			conseillee: "reopen",
		},
		{
			nom: "waived",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				if e := s.overrideReviewedTask(w.ID, "t1", "Décision de recette explicite"); e != nil {
					t.Fatal(e)
				}
				current, e := s.get(w.ID)
				if e != nil {
					t.Fatal(e)
				}
				return current
			},
			dispo:      []string{"report", "reopen", "assign"},
			indispo:    []string{"start", "retry", "stop", "reconcile", "submit", "gate", "accepted", "override"},
			conseillee: "reopen",
		},
		{
			nom: "abandoned",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				return applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "abandoned", Outcome: "interrupted"})
			},
			dispo:      []string{"report", "reopen", "assign"},
			indispo:    []string{"start", "retry", "stop", "reconcile", "submit", "gate", "accepted", "override"},
			conseillee: "reopen",
		},
		{
			nom: "rouverte_revalidation",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
				w = gateTest(t, s, w, fixture(t, s.root))
				w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
				// La preuve change : la gate devient périmée, la réouverture
				// positionne alors la revalidation (garde store.go:376).
				if e := os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("modified"), 0600); e != nil {
					t.Fatal(e)
				}
				return applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "todo", Next: "revalidation demandée"})
			},
			dispo:      []string{"start", "report", "submit", "override", "assign"},
			indispo:    []string{"stop", "retry", "reconcile", "gate", "accepted", "reopen"},
			conseillee: "start",
		},
		{
			// À l'inverse du cas todo : avec une tentative terminée, le retry
			// redevient disponible (revue 4, F-1) — c'est le seul chemin de
			// relance cohérent entre console et web.
			nom: "todo_avec_tentative_retry_dispo",
			preparer: func(t *testing.T, s *Store) Work {
				w := taskTest(t, s, createTest(t, s))
				w = tentativeFinie(t, s, w)
				current, e := s.get(w.ID)
				if e != nil {
					t.Fatal(e)
				}
				return applyTest(t, s, current, "task.update", Request{ID: "t1", Status: "todo", Next: "reprise après tentative"})
			},
			dispo:      []string{"start", "retry", "submit", "gate", "override", "report", "assign"},
			indispo:    []string{"stop", "reconcile", "accepted", "reopen"},
			conseillee: "start",
		},
	}

	kinds := []string{"start", "retry", "stop", "reconcile", "report", "submit", "gate", "accepted", "override", "reopen", "assign"}
	for _, tc := range cases {
		t.Run(tc.nom, func(t *testing.T) {
			s := storeTest(t)
			w := tc.preparer(t, s)
			oracle := oracleFor(t, s, &w, "t1")
			for _, k := range kinds {
				a, ok := oracle[k]
				if !ok {
					t.Fatalf("action %s absente de l'oracle", k)
				}
				if strings.TrimSpace(a.Label) == "" {
					t.Fatalf("action %s sans libellé", k)
				}
			}
			for _, k := range tc.dispo {
				if !oracle[k].Disponible {
					t.Errorf("%s devrait être disponible : %s", k, oracle[k].Raison)
				}
			}
			for _, k := range tc.indispo {
				a := oracle[k]
				if a.Disponible {
					t.Errorf("%s devrait être indisponible", k)
				}
				if strings.TrimSpace(a.Raison) == "" {
					t.Errorf("%s indisponible sans motif", k)
				}
			}
			// Exactement une action conseillée, disponible, dans la liste attendue.
			var conseillees []string
			for _, k := range kinds {
				if oracle[k].Conseillee {
					conseillees = append(conseillees, k)
				}
			}
			if len(conseillees) != 1 || conseillees[0] != tc.conseillee {
				t.Fatalf("conseillée = %v, attendu [%s]", conseillees, tc.conseillee)
			}
			if !oracle[tc.conseillee].Disponible {
				t.Errorf("action conseillée %s indisponible : %s", tc.conseillee, oracle[tc.conseillee].Raison)
			}
		})
	}
}

// TestTaskActionsChampsExplicites vérifie que chaque action porte des champs
// humains (libellé + aide) et que les intitulés du design sont présents.
func TestTaskActionsChampsExplicites(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	oracle := oracleFor(t, s, &w, "t1")

	attendus := map[string]map[string]string{
		"submit":   {"path": "Chemin du rapport soumis"},
		"gate":     {"path": "Document de gate (preuve d'évaluation)", "name": "Nom de la gate"},
		"override": {"note": "Motif de la dérogation"},
		"assign":   {"owner": "Responsable"},
	}
	for kind, champs := range attendus {
		a := oracle[kind]
		vus := map[string]TaskField{}
		for _, c := range a.Champs {
			vus[c.Name] = c
		}
		for name, label := range champs {
			c, ok := vus[name]
			if !ok {
				t.Errorf("%s : champ %s manquant", kind, name)
				continue
			}
			if c.Label != label {
				t.Errorf("%s.%s : libellé %q, attendu %q", kind, name, c.Label, label)
			}
			if strings.TrimSpace(c.Aide) == "" {
				t.Errorf("%s.%s : aide vide", kind, name)
			}
		}
	}
	// Le nom de la gate est requis.
	nom := oracle["gate"]
	for _, c := range nom.Champs {
		if c.Name == "name" && !c.Requis {
			t.Error("le nom de la gate devrait être requis")
		}
	}
	// Lancement : champs explicites, consigne requise.
	lance := oracle["start"]
	noms := map[string]bool{}
	for _, c := range lance.Champs {
		noms[c.Name] = true
	}
	for _, want := range []string{"provider", "workspace", "instruction"} {
		if !noms[want] {
			t.Errorf("start : champ %s manquant", want)
		}
	}
}

// TestTaskActionsRunningSansAgent vérifie le cas particulier : tâche running
// mais aucun agent actif observé — ni start ni retry, motif de réconciliation.
func TestTaskActionsRunningSansAgent(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	oracle := oracleFor(t, s, &w, "t1")
	if oracle["start"].Disponible {
		t.Error("start devrait être fermé tant que l'état n'est pas réconcilié")
	}
	if oracle["reconcile"].Disponible {
		t.Error("reconcile exige une tentative ; aucune n'existe ici")
	}
	if !strings.Contains(oracle["start"].Raison, "réconciliez") && !strings.Contains(oracle["start"].Raison, "Réconciliez") {
		t.Errorf("motif de start inattendu : %q", oracle["start"].Raison)
	}
	// Aucune action conseillée ici : réconcilier exige une tentative, start est
	// fermé tant que l'état observé n'est pas réconcilié — les surfaces
	// dégradent en le signalant explicitement.
	for _, a := range oracle {
		if a.Conseillee {
			t.Errorf("action conseillée inattendue en running sans agent : %s", a.Kind)
		}
	}
}

// TestTaskActionsDependanceNonValidee couvre le blocage de départ par
// dépendance : le motif cite la dépendance fautive.
func TestTaskActionsDependanceNonValidee(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Aval", Deliverable: "rapport", Criteria: []string{"revue"}, Depends: []string{"t1"}})
	// t1 reste en todo : la dépendance n'est pas acceptée, le départ de t2
	// doit être bloqué dès l'oracle (sans même tenter task.update, que le
	// moteur refuserait).
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, e := current.task("t2")
	if e != nil {
		t.Fatal(e)
	}
	agents, _ := s.agents(w.ID)
	actions := s.taskActions(&current, task, agents)
	byKind := map[string]TaskAction{}
	for _, a := range actions {
		byKind[a.Kind] = a
	}
	if byKind["start"].Disponible {
		t.Error("start devrait être bloqué par la dépendance t1")
	}
	if !strings.Contains(byKind["start"].Raison, "t1") {
		t.Errorf("motif sans identifiant de dépendance : %q", byKind["start"].Raison)
	}
}

// Le cockpit propose une action au nom de l'oracle : un nom qui n'existe plus
// laisse la liste déroulante vide et le dialogue inutilisable, sans message.
// C'est arrivé après S28, où « todo » est devenu « reopen » côté oracle alors
// que l'écran choisissait encore l'ancien nom. Ce test fige les noms d'actions
// sur lesquels le cockpit s'appuie, hors recette navigateur.
func TestTaskActionKindsUsedByCockpitExist(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	connus := map[string]bool{}
	for _, a := range s.taskActions(&w, &w.Tasks[0], nil) {
		connus[a.Kind] = true
	}
	// Noms écrits en dur dans web/cockpit.js et web/conduite.js.
	for _, kind := range []string{"start", "retry", "stop", "reconcile", "report",
		"submit", "gate", "accepted", "override", "reopen", "assign"} {
		if !connus[kind] {
			t.Errorf("le cockpit référence l'action %q, absente de l'oracle : le dialogue resterait vide", kind)
		}
	}
	if connus["todo"] {
		t.Error("« todo » est un statut, pas une action : le cockpit doit employer « reopen »")
	}
}
