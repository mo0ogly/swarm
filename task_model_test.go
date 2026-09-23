//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func taskModelFixture(t *testing.T) (*Store, Work, Provider) {
	t.Helper()
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	cmd := filepath.Join(t.TempDir(), "claude")
	if e := os.WriteFile(cmd, []byte("#!/bin/sh\ncat >/dev/null\n"), 0700); e != nil {
		t.Fatal(e)
	}
	p := Provider{Command: cmd, Args: []string{"-p", "--output-format", "stream-json", "--verbose"}}
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"claude": p}})
	if e := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); e != nil {
		t.Fatal(e)
	}
	return s, w, p
}
func TestTaskModelSelectionLaunchAndGuards(t *testing.T) {
	for _, level := range []string{"standard", "exigeant"} {
		t.Run(level, func(t *testing.T) {
			s, w, p := taskModelFixture(t)
			_, route, e := resolveModel(p, level, "work")
			if e != nil {
				t.Fatal(e)
			}
			r := TaskModelRequest{Schema: 1, EventID: "model-change", Revision: w.Revision, Task: "t1", Provider: "claude", Level: level, PolicyHash: route.PolicyHash}
			if _, e = s.previewTaskModel(w.ID, r); e != nil {
				t.Fatal(e)
			}
			before, _ := s.get(w.ID)
			if before.Revision != w.Revision {
				t.Fatal("preview wrote")
			}
			w, e = s.configureTaskModel(w.ID, r)
			if e != nil {
				t.Fatal(e)
			}
			replay, e := s.configureTaskModel(w.ID, r)
			if e != nil || replay.Revision != w.Revision {
				t.Fatal(e)
			}
			other, e := openStore(s.root, false)
			if e != nil {
				t.Fatal(e)
			}
			defer other.db.Close()
			saved, _ := other.get(w.ID)
			task, _ := saved.task("t1")
			if task.ModelSelection.Route.Model != route.Model {
				t.Fatal("not persisted")
			}
			inherited := dispatchTest([]Task{*task}, nil)
			prof := profileFor(inherited, *task)
			if prof.Provider != "claude" || prof.Level != level || inherited.profile.Provider != "fixture" {
				t.Fatal("inheritance mutated")
			}
			launch := Launch{Schema: 1, EventID: "model-launch", Revision: w.Revision, TaskID: "t1", Provider: "claude", Level: level}
			a, _, e := s.prepare(w.ID, launch)
			if e != nil {
				t.Fatal(e)
			}
			if a.ModelRoute.Model != route.Model || !strings.Contains(strings.Join(a.Args, " "), "--model "+route.Model) {
				t.Fatalf("wrong actual arguments %+v", a)
			}
			current, _ := s.get(w.ID)
			r.Revision = current.Revision
			r.EventID = "change-active"
			r.Inherit = true
			if _, e = s.configureTaskModel(w.ID, r); e == nil {
				t.Fatal("active task changed")
			}
		})
	}
}
func TestTaskModelPolicyChangeAndInheritance(t *testing.T) {
	s, w, p := taskModelFixture(t)
	_, route, _ := resolveModel(p, "exigeant", "work")
	r := TaskModelRequest{Schema: 1, EventID: "select", Revision: w.Revision, Task: "t1", Provider: "claude", Level: "exigeant", PolicyHash: route.PolicyHash}
	w, e := s.configureTaskModel(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	r.EventID = "stale"
	if _, e = s.configureTaskModel(w.ID, r); e == nil {
		t.Fatal("stale accepted")
	}
	p.ModelPolicy = effectiveModelPolicy(p)
	p.ModelPolicy.PageLevel = "standard"
	raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"claude": p}})
	os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600)
	_, _, e = s.prepare(w.ID, Launch{Schema: 1, EventID: "outdated", Revision: w.Revision, TaskID: "t1", Provider: "claude", Level: "exigeant"})
	if e == nil || !strings.Contains(e.Error(), "Politique") {
		t.Fatalf("policy silently changed: %v", e)
	}
	r.Revision = w.Revision
	r.EventID = "inherit"
	r.Inherit = true
	w, e = s.configureTaskModel(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	task, _ := w.task("t1")
	if task.ModelSelection != nil || len(task.Attempts) != 0 {
		t.Fatal("inherit changed history")
	}
}

func TestTaskModelConcurrentChangesAndTwoTaskDispatch(t *testing.T) {
	s, w, p := taskModelFixture(t)
	_, route, _ := resolveModel(p, "standard", "work")
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	results := make(chan error, 2)
	for i, store := range []*Store{s, other} {
		go func(i int, store *Store) {
			_, e := store.configureTaskModel(w.ID, TaskModelRequest{Schema: 1, EventID: []string{"first", "second"}[i], Revision: w.Revision, Task: "t1", Provider: "claude", Level: "standard", PolicyHash: route.PolicyHash})
			results <- e
		}(i, store)
	}
	ok, conflict := 0, 0
	for range 2 {
		e := <-results
		if e == nil {
			ok++
		} else if commandFailure(e).Code == "revision_conflict" {
			conflict++
		} else {
			t.Fatal(e)
		}
	}
	if ok != 1 || conflict != 1 {
		t.Fatal(ok, conflict)
	}
	tasks := []Task{{ID: "sonnet-task", Status: "todo", Profile: &LaunchProfile{Provider: "claude", Role: "worker", Workspace: "/tmp/sonnet"}, ModelSelection: &TaskModel{Provider: "claude", Route: ModelRoute{Level: "standard", Model: "sonnet"}}}, {ID: "opus-task", Status: "todo", Profile: &LaunchProfile{Provider: "claude", Role: "worker", Workspace: "/tmp/opus"}, ModelSelection: &TaskModel{Provider: "claude", Route: ModelRoute{Level: "exigeant", Model: "opus"}}}}
	decisions, _ := planDispatch(dispatchTest(tasks, nil))
	if len(decisions) != 2 {
		t.Fatal(decisions)
	}
	levels := map[string]string{}
	for _, d := range decisions {
		levels[d.TaskID] = d.Profile.Level
	}
	if levels["sonnet-task"] != "standard" || levels["opus-task"] != "exigeant" {
		t.Fatal(levels)
	}
}
