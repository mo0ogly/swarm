//go:build linux

package main

import (
	"errors"
	"testing"
)

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
	if err = s.setAutonomy(other.ID, autonomyAuto, 1); err != nil {
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
	r.Revision = other.Revision
	r.TaskID = other.Tasks[0].ID
	r.EventID = newID("wait-")
	_, _, err = s.prepare(other.ID, r)
	if !errors.As(err, &busy) {
		t.Fatal(err)
	}
}
