//go:build linux

package main

import (
	"strings"
	"testing"
)

func TestActivityMergesBothSourcesNewestFirst(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Source « moteur » : décisions locales, hors machine à états.
	if e := s.controlEvent(w.ID, "dispatch", "t1 : départ automatique · fixture"); e != nil {
		t.Fatal(e)
	}
	// Source « métier » : mutations du travail, portées par une révision.
	applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})

	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	// Compter les entrées ne prouverait rien : chaque table porte déjà, à elle
	// seule, plusieurs entrées et les deux origines. Seul un type qu'une seule
	// table peut produire atteste qu'elle a réellement été lue : « dispatch »
	// n'existe que dans cockpit_events, « task.update » que dans events.
	kinds := map[string]bool{}
	for _, x := range page.Entries {
		kinds[x.Kind] = true
	}
	if !kinds["dispatch"] {
		t.Fatalf("cockpit_events non fusionnée : %+v", page.Entries)
	}
	if !kinds["task.update"] {
		t.Fatalf("events non fusionnée : %+v", page.Entries)
	}
	for i := 1; i < len(page.Entries); i++ {
		if page.Entries[i-1].At < page.Entries[i].At {
			t.Fatalf("ordre non décroissant : %s avant %s", page.Entries[i-1].At, page.Entries[i].At)
		}
	}
	origines := map[string]bool{}
	for _, x := range page.Entries {
		origines[x.Origin] = true
	}
	if !origines[activityEngine] || !origines[activityHuman] {
		t.Fatalf("les deux origines doivent être distinguées : %+v", page.Entries)
	}
}

// Le lancement d'agent sérialise un Launch, qui nomme la tâche « task_id » et
// n'a aucun champ « id ». Lire la seule orthographe des mutations laisserait la
// tentative orpheline : ce fil dirait qu'un départ a eu lieu sans dire sur quoi.
func TestActivityNamesTaskOfAgentStart(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	page, e := s.activity(w.ID, activityQuery{Limit: 50})
	if e != nil {
		t.Fatal(e)
	}
	found := false
	for _, x := range page.Entries {
		if x.Kind != "agent.start" {
			continue
		}
		found = true
		if x.TaskID != r.TaskID {
			t.Fatalf("tentative non rattachée à sa tâche : task_id=%q attendu %q", x.TaskID, r.TaskID)
		}
		if !strings.HasPrefix(x.Message, r.TaskID) {
			t.Fatalf("le message doit nommer la tâche %q : %q", r.TaskID, x.Message)
		}
	}
	if !found {
		t.Fatalf("aucune tentative dans le fil : %+v", page.Entries)
	}
}
