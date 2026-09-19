//go:build linux

package main

import (
	"errors"
	"os"
	"path/filepath"
	"sync"
	"testing"
)

func TestWorkspaceReservationsAdmitOnlyOneConcurrentWriter(t *testing.T) {
	s := storeTest(t)
	firstWork, first := setupAgent(t, s)
	secondWork, second := setupAgent(t, s)
	workspace := filepath.Join(s.root, "commun")
	if err := os.Mkdir(workspace, 0700); err != nil {
		t.Fatal(err)
	}
	first.Workspace = workspace
	second.Workspace = workspace
	start := make(chan struct{})
	results := make(chan error, 2)
	var group sync.WaitGroup
	for _, launch := range []struct {
		work string
		req  Launch
	}{{firstWork.ID, first}, {secondWork.ID, second}} {
		group.Add(1)
		go func(work string, req Launch) {
			defer group.Done()
			<-start
			_, _, err := s.prepare(work, req)
			results <- err
		}(launch.work, launch.req)
	}
	close(start)
	group.Wait()
	close(results)
	success := 0
	for err := range results {
		if err == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatalf("un seul écrivain concurrent attendu, succès=%d", success)
	}
	active := 0
	for _, work := range []string{firstWork.ID, secondWork.ID} {
		agents, err := s.agents(work)
		if err != nil {
			t.Fatal(err)
		}
		for _, agent := range agents {
			if activeAgent(agent) {
				active++
			}
		}
	}
	if active != 1 {
		t.Fatalf("réservation concurrente incohérente, agents actifs=%d", active)
	}
}

func TestWorkspaceReservationsAllowDisjointWritersAndCleanupKeepsFiles(t *testing.T) {
	s := storeTest(t)
	firstWork, first := setupAgent(t, s)
	secondWork, second := setupAgent(t, s)
	first.Workspace = filepath.Join(s.root, "isole-a")
	second.Workspace = filepath.Join(s.root, "isole-b")
	for _, workspace := range []string{first.Workspace, second.Workspace} {
		if err := os.Mkdir(workspace, 0700); err != nil {
			t.Fatal(err)
		}
		if err := os.WriteFile(filepath.Join(workspace, "conserver.txt"), []byte("preuve"), 0600); err != nil {
			t.Fatal(err)
		}
	}
	firstAgent, created, err := s.prepare(firstWork.ID, first)
	if err != nil || !created {
		t.Fatal(firstAgent, created, err)
	}
	secondAgent, created, err := s.prepare(secondWork.ID, second)
	if err != nil || !created {
		t.Fatal(secondAgent, created, err)
	}

	overlapWork, overlap := setupAgent(t, s)
	overlap.Workspace = s.root
	if _, _, err = s.prepare(overlapWork.ID, overlap); err == nil {
		t.Fatal("un écrivain parent des deux espaces isolés a été admis")
	} else {
		var busy *WorkspaceBusyError
		if !errors.As(err, &busy) {
			t.Fatalf("réservation de dossier non attribuable : %v", err)
		}
	}

	for _, agent := range []Agent{firstAgent, secondAgent} {
		if err = s.finishAgent(agent, "interrupted", "fin de recette M4", nil); err != nil {
			t.Fatal(err)
		}
		if _, err = os.Stat(filepath.Join(agent.CWD, "conserver.txt")); err != nil {
			t.Fatalf("le nettoyage d’état a supprimé le contenu de %s : %v", agent.CWD, err)
		}
	}
}

func TestWorkspaceWaitPreviewNamesActualReservation(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Workspace = s.root
	a, _, err := s.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	other := taskTest(t, s, createTest(t, s))
	request := webRequest{Work: other.ID, Revision: other.Revision, Task: other.Tasks[0].ID, Provider: r.Provider, Workspace: r.Workspace, Role: r.Role}
	result := s.launchEligibility(request)
	if result["reason_code"] != "workspace_busy" {
		t.Fatal(result)
	}
	busy, ok := result["blocker"].(*WorkspaceBusyError)
	if !ok || busy.AgentID != a.ID || busy.WorkID != w.ID || busy.TaskID != r.TaskID || busy.Title != w.Tasks[0].Title {
		t.Fatal(result)
	}
	if result["eligible"] != false {
		t.Fatal(result)
	}
	before := result["next_label"]
	if err = s.setMission(other.ID, true); err != nil {
		t.Fatal(err)
	}
	if err = organizedFixtureStore(t, s).setAutonomy(other.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	result = s.launchEligibility(request)
	if result["next_label"] == before {
		t.Fatal(result)
	}
	if err = s.pause(other.ID, true); err != nil {
		t.Fatal(err)
	}
	paused := s.launchEligibility(request)
	if paused["next_label"] == result["next_label"] || paused["next_label"] == before {
		t.Fatal(paused)
	}
	agents, _ := s.agents(other.ID)
	if len(agents) != 0 {
		t.Fatal(agents)
	}
	if err = s.pause(other.ID, false); err != nil {
		t.Fatal(err)
	}
	other, _ = s.get(other.ID)
	r.Revision = other.Revision
	r.TaskID = other.Tasks[0].ID
	r.EventID = newID("wait-")
	_, _, err = s.prepare(other.ID, r)
	if !errors.As(err, &busy) {
		t.Fatal(err)
	}
}
