//go:build linux

package main

import "testing"

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
	if len(page.Entries) < 2 {
		t.Fatalf("les deux sources doivent apparaître : %+v", page.Entries)
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
