//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strings"
	"testing"
)

func TestMissionHelpGroundsLastAttemptAndRelay(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	a.Relay = "Rapport absent : docs/t1.md"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	// Other tasks and noisy activity must not hide this task's evidence.
	if err = s.log(a.ID, "lifecycle", "OLD_RELAY_REASON"); err != nil {
		t.Fatal(err)
	}
	for i := 0; i < 201; i++ {
		other := Agent{ID: fmt.Sprintf("older-%d", i), WorkID: w.ID, TaskID: "other", Status: "completed", CWD: s.root}
		raw, _ := json.Marshal(other)
		if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", other.ID, w.ID, other.TaskID, s.root, "completed", raw, []byte("{}")); err != nil {
			t.Fatal(err)
		}
		if err = s.log(a.ID, "activity", "Noise"); err != nil {
			t.Fatal(err)
		}
	}
	ctx, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{r.TaskID}})
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, f := range ctx.Facts {
		if f.Name == "motif_relais_rapport" && strings.Contains(f.Value, "Rapport absent") {
			found = true
		}
	}
	if !found {
		t.Fatal(ctx.Facts)
	}
	logFound := false
	for _, f := range ctx.Facts {
		if strings.Contains(f.Value, "OLD_RELAY_REASON") {
			logFound = true
		}
	}
	if !logFound {
		t.Fatal("useful event hidden by activity")
	}
	old := ctx.Hash
	a.Relay = "Plusieurs rapports concurrents"
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	next, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{r.TaskID}})
	if err != nil || old == next.Hash {
		t.Fatal("relay change must invalidate assistant context", err)
	}
	general, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks"})
	if err != nil {
		t.Fatal(err)
	}
	for _, f := range general.Facts {
		if f.Name == "motif_relais_rapport" {
			t.Fatal("attempt details must stay scoped to selected task")
		}
	}
}
