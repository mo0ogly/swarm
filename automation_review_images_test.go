//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestAutomationC04CodexReviewImages(t *testing.T) {
	dir := t.TempDir()
	raw := screenshotFixture(t, dir, "source.png")
	img := reviewImage{Path: "docs/screenshots/c04.png", SHA256: hash(raw), MediaType: "image/png", Data: raw}
	provider := Provider{Command: "/usr/bin/codex", Args: []string{"exec", "--json", "--sandbox", "read-only", "-"}}
	input, err := structuredReviewInput(&provider, "Examine pixels without tools", []reviewImage{img})
	if err != nil || input != "Examine pixels without tools" {
		t.Fatalf("prompt changed: %q %v", input, err)
	}
	private := t.TempDir()
	if err = codexReviewImageArgs(&provider, private, []reviewImage{img, img}); err != nil {
		t.Fatal(err)
	}
	if provider.Args[len(provider.Args)-1] != "-" {
		t.Fatal("stdin prompt lost")
	}
	count := 0
	for index, arg := range provider.Args {
		if arg != "--image" {
			continue
		}
		count++
		path := provider.Args[index+1]
		data, err := os.ReadFile(path)
		info, statErr := os.Stat(path)
		if err != nil || statErr != nil || !bytes.Equal(raw, data) || info.Mode().Perm() != 0600 || filepath.Dir(path) != private {
			t.Fatalf("image bytes/privacy not preserved: %s %v %v", path, err, statErr)
		}
	}
	if count != 2 {
		t.Fatalf("image count=%d", count)
	}
	bad := img
	bad.SHA256 = hash([]byte("changed"))
	if err = codexReviewImageArgs(&provider, private, []reviewImage{bad}); err == nil {
		t.Fatal("stale bytes admitted")
	}
	if supportsReviewImages(Provider{Command: "/usr/bin/codex", APIConnectionID: "api"}) {
		t.Fatal("unverified API image support advertised")
	}
}

func TestAutomationC04CodexReviewImagesProcess(t *testing.T) {
	dir := t.TempDir()
	raw := screenshotFixture(t, dir, "source.png")
	capture := filepath.Join(dir, "observed.json")
	command := filepath.Join(dir, "codex")
	script := fmt.Sprintf(`#!/usr/bin/python3
import json,sys,pathlib,hashlib
args=sys.argv[1:]
images=[args[i+1] for i,a in enumerate(args) if a=='--image']
data={'args':args,'prompt':sys.stdin.read(),'images':[{'path':p,'sha256':hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()} for p in images]}
pathlib.Path(%q).write_text(json.dumps(data))
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'{"ok":true}'}}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':1,'output_tokens':1}}))
`, capture)
	if err := os.WriteFile(command, []byte(script), 0700); err != nil {
		t.Fatal(err)
	}
	started := time.Now()
	reply, err := runStructuredProviderImagesClock(Provider{Command: command}, nil, "Inspect actual pixels", `{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}`, []reviewImage{{Path: "docs/screenshots/current.png", SHA256: hash(raw), MediaType: "image/png", Data: raw}}, 10*time.Second, func() bool { return true }, nil, func() time.Duration { return time.Since(started) })
	if err != nil || !strings.Contains(reply, `"ok":true`) {
		t.Fatalf("reply=%s err=%v", reply, err)
	}
	var observed struct {
		Args   []string `json:"args"`
		Prompt string   `json:"prompt"`
		Images []struct {
			Path   string `json:"path"`
			SHA256 string `json:"sha256"`
		} `json:"images"`
	}
	b, err := os.ReadFile(capture)
	if err != nil {
		t.Fatal(err)
	}
	if err = json.Unmarshal(b, &observed); err != nil {
		t.Fatal(err)
	}
	if observed.Prompt != "Inspect actual pixels" || len(observed.Images) != 1 || observed.Images[0].SHA256 != hash(raw) {
		t.Fatalf("actual process attachment lost: %+v", observed)
	}
	args := strings.Join(observed.Args, "|")
	if !strings.Contains(args, "--sandbox|read-only") || !strings.Contains(args, "--ephemeral") || !strings.Contains(args, "--output-schema|") || strings.Contains(args, "resume") {
		t.Fatalf("review guard changed: %s", args)
	}
	if _, err = os.Stat(observed.Images[0].Path); !os.IsNotExist(err) {
		t.Fatalf("temporary image survived provider exit: %v", err)
	}
}

func TestAutomationC04ImagePreflightRecovery(t *testing.T) {
	s, prep := preparedTeam(t)
	w, err := s.get(prep.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	ps, _ := s.providers()
	cfg := w.Planning.Reviewer
	provider := ps.Providers[cfg.Provider]
	provider.Command = filepath.Join(t.TempDir(), "codex")
	if err = os.WriteFile(provider.Command, []byte("#!/bin/sh\nexit 0\n"), 0700); err != nil {
		t.Fatal(err)
	}
	ps.Providers[cfg.Provider] = provider
	raw, _ := json.Marshal(ps)
	if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	raw, _ = json.Marshal(provider)
	cfg.ProviderDigest = hash(raw)
	cfg.Calls = 3
	cfg.Failure = "revue visuelle indisponible pour cet adaptateur ; aucun appel sans les captures requises"
	task := &w.Tasks[0]
	task.Status = "submitted"
	task.Attempts = []Attempt{{ID: "kept-producer", Status: "completed"}}
	name := "docs/screenshots/actual.png"
	image := screenshotFixture(t, s.root, name)
	receipt := "docs/fixture-receipt.json"
	if err = os.WriteFile(filepath.Join(s.root, receipt), []byte("isolated test receipt"), 0600); err != nil {
		t.Fatal(err)
	}
	task.ValidationPolicy = &ValidationPolicy{Mode: "human", Controls: []ValidationControl{{ID: "capture", Command: []string{"fixture-check"}, Inputs: []string{name}}}}
	task.AutoValidation = &AutomaticValidation{Attempt: "kept-producer", Controller: validationController, PolicyDigest: validationPolicyDigest(*task.ValidationPolicy), State: "pending_review", Receipt: receipt, Artifacts: map[string]string{name: hash(image), receipt: hash([]byte("isolated test receipt"))}, Controls: []ValidationControlResult{{ID: "capture", Command: []string{"fixture-check"}, Executed: true, Passed: true, Started: now(), Finished: now()}}}
	raw, _ = json.Marshal(w)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	request := PlanningRequest{Schema: 1, EventID: "image-repaired", Revision: w.Revision, Task: task.ID, Reason: "Installed adapter now supports declared current images", ConfirmReviewErrorRepair: true}
	if err = os.WriteFile(filepath.Join(s.root, name), []byte("corrupt"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = s.planningChange(w.ID, "retry-review", request); err == nil {
		t.Fatal("stale image cleared retained failure")
	}
	if err = os.WriteFile(filepath.Join(s.root, name), image, 0600); err != nil {
		t.Fatal(err)
	}
	next, err := s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal(err)
	}
	if next.Planning.Reviewer.Failure != "" || next.Planning.Reviewer.Calls != 3 || len(next.Tasks[0].Attempts) != 1 || next.Tasks[0].Status != "submitted" || next.Tasks[0].IndependentReview != nil {
		t.Fatal("retry changed budgets, producer or verdict")
	}
	next.Planning.Reviewer.Failure = "unrelated provider error"
	if err = s.repairedImageReviewPreflight(&next, &next.Tasks[0]); err == nil {
		t.Fatal("unrelated failure bypassed")
	}
}
