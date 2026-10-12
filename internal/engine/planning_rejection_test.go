//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestPlanningRejectedProposalReturnsToOwnerWithIntactEvidence(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("existing")}
	w = planningDo(t, s, w, "decide", r)
	var err error
	w, err = s.mutate(w.ID, "test.input", "new-input", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Criteria = append(w.Criteria, "Final retrospective")
		w.Planning.Scopes[0].Requirements = append(w.Planning.Scopes[0].Requirements, "req-2")
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "next-input", Scope: "root", Kind: "operator_resume", Message: "Create a new task without recreating existing tasks."})
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	script := filepath.Join(t.TempDir(), "claude")
	provider := func(p PlanningProposal) {
		t.Helper()
		body, _ := json.Marshal(map[string]any{"type": "result", "structured_output": p})
		if err := os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+string(body)+"'\n"), 0700); err != nil {
			t.Fatal(err)
		}
		config, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": {Command: script}}})
		if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), config, 0600); err != nil {
			t.Fatal(err)
		}
	}
	duplicate := planningTask("existing")
	duplicate.Requirements = []string{"req-2"}
	proposal := PlanningProposal{Inputs: []string{"next-input"}, Reason: "Mistaken duplicate creation", Operations: []PlanningOperation{duplicate}}
	provider(proposal)
	w, err = s.mutate(w.ID, "test.provider", "select-provider", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Planning.Provider = "test"
		raw, _ := json.Marshal(Provider{Command: script})
		w.Planning.ProviderDigest = hash(raw)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	beforeActs, beforeDecisions := w.Planning.Activations, w.Planning.Decisions
	if err = s.planningStep(w.ID); err == nil {
		t.Fatal("invalid proposal accepted")
	}
	failed, _ := s.get(w.ID)
	if failed.Planning.Failure != "" || failed.Planning.Scopes[0].Holder != "" || len(failed.Tasks) != 1 || failed.Planning.Activations != beforeActs+1 || failed.Planning.Decisions != beforeDecisions {
		t.Fatal("rejection globally stopped plan or mutated contract", failed.Planning)
	}
	var diagnostic PlanningEvent
	for _, event := range failed.Planning.Inbox {
		if event.Kind == "planning_rejected" {
			diagnostic = event
		}
	}
	if diagnostic.ID == "" || diagnostic.Scope != "root" || diagnostic.Handoff == nil || diagnostic.Decision != "" {
		t.Fatal("no durable owner diagnostic", diagnostic)
	}
	report, err := s.planningReport(diagnostic)
	if err != nil || report == nil || !strings.Contains(report.Text, "existing") || !strings.Contains(report.Text, "tâche déjà présente") {
		t.Fatal("failed operation not attributed", report, err)
	}
	var receipt map[string]json.RawMessage
	if err = json.Unmarshal([]byte(report.Text), &receipt); err != nil {
		t.Fatal(err)
	}
	var failedOperation struct {
		Proposal *PlanningOperation `json:"proposal"`
	}
	if err = json.Unmarshal(receipt["failed_operation"], &failedOperation); err != nil || failedOperation.Proposal == nil || failedOperation.Proposal.ID != duplicate.ID || failedOperation.Proposal.Next != duplicate.Next {
		t.Fatal("owner cannot receive the exact operation it must correct", failedOperation, err)
	}
	var rawRef ExchangeArtifact
	if err = json.Unmarshal(receipt["raw_response"], &rawRef); err != nil {
		t.Fatal(err)
	}
	raw, err := os.ReadFile(filepath.Join(s.root, rawRef.Path))
	expected, _ := json.Marshal(proposal)
	var adapted any
	if err := json.Unmarshal(expected, &adapted); err != nil {
		t.Fatal(err)
	}
	expected, _ = json.Marshal(adapted)
	if err != nil || hash(raw) != rawRef.SHA256 || string(raw) != string(expected) {
		t.Fatal("original response altered", string(raw), err)
	}
	second, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer second.db.Close()
	failed, err = second.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	// Identical bad operation and unchanged relevant state create no fresh proof.
	provider(proposal)
	if err = second.planningStep(w.ID); err == nil {
		t.Fatal("repeated invalid proposal accepted")
	}
	repeated, _ := second.get(w.ID)
	if repeated.Planning.Failure != "" || repeated.Planning.Scopes[0].RejectionWait == nil {
		t.Fatal("repeat not scoped", repeated.Planning)
	}
	if err = second.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	held, _ := second.get(w.ID)
	if held.Revision != repeated.Revision || held.Planning.Activations != beforeActs+2 {
		t.Fatal("identical rejection spent another call")
	}
	owner, _ := held.Planning.scope("root")
	_, err = second.planningChange(held.ID, "claim", PlanningRequest{Schema: 1, EventID: "unchanged-public-claim", Revision: held.Revision, Scope: "root", ScopeRevision: owner.Revision, Holder: "bypass", LeaseSeconds: 60})
	if err == nil || commandFailure(err).Code != "rejection_requires_change" {
		t.Fatal("public claim bypassed unchanged rejection", err)
	}
	archive := filepath.Join(t.TempDir(), "rejections.zip")
	if err := second.export(held.ID, archive); err != nil {
		t.Fatal(err)
	}
	importedStore := storeTest(t)
	imported, err := importedStore.importBundle(archive)
	if err != nil {
		t.Fatal(err)
	}
	for _, event := range held.Planning.Inbox {
		if event.Kind == "planning_rejected" {
			// Imported proofs remain archived, never promoted to live candidate evidence.
			if _, err := importedStore.planningReport(event); err == nil {
				t.Fatal("import silently activated archived proof")
			}
			if len(event.Artifacts) != 2 {
				t.Fatal("both diagnostic and response must be declared", event)
			}
			for _, ref := range event.Artifacts {
				data, err := os.ReadFile(filepath.Join(importedStore.root, ".swarm", "imports", held.ID, ref.SHA256))
				if err != nil || hash(data) != ref.SHA256 {
					t.Fatal("rejection original lost", ref.Path, err)
				}
			}
		}
	}
	for _, ref := range []ExchangeArtifact{*diagnostic.Handoff, rawRef} {
		got, err := os.ReadFile(filepath.Join(importedStore.root, ".swarm", "imports", held.ID, ref.SHA256))
		if err != nil || hash(got) != ref.SHA256 {
			t.Fatal("rejected original lost in export/import", ref.Path, err)
		}
	}
	if imported.Planning.Scopes[0].RejectionWait == nil {
		t.Fatal("scoped wait lost in export/import")
	}
	manifest, err := os.ReadFile(filepath.Join(importedStore.root, ".swarm", "imports", held.ID, "manifest.json"))
	if err != nil {
		t.Fatal(err)
	}
	var bundle Bundle
	if err := json.Unmarshal(manifest, &bundle); err != nil {
		t.Fatal(err)
	}
	refusals := 0
	for _, event := range held.Planning.Inbox {
		if event.Kind == "planning_rejected" {
			refusals++
			for _, ref := range event.Artifacts {
				if bundle.Files[ref.Path] != ref.SHA256 {
					t.Fatal("artifact path attribution lost", ref)
				}
			}
		}
	}
	if refusals != 2 {
		t.Fatal("repeated refusal original not retained", refusals)
	}
	// Public resumption supplies a new input; it does not refund the two calls.
	held = planningDo(t, second, held, "resume", PlanningRequest{Scope: "root", Reason: "Use a genuinely new task identifier and preserve both rejected responses."})
	rawContext, delivery, err := second.planningDeliveryContext(held, "root", 64000)
	if err != nil {
		t.Fatal(err)
	}
	if len(delivery.Events) == 0 || delivery.Events[0] != repeated.Planning.Scopes[0].RejectionWait.Event || !strings.Contains(string(rawContext), "existing_task_ids") {
		t.Fatal("diagnostic/inventory not delivered first", string(rawContext))
	}
	final := planningTask("final-retex")
	final.Requirements = []string{"req-2"}
	provider(PlanningProposal{Inputs: delivery.Events, Reason: "Correct duplicate using current owner diagnostic", Operations: []PlanningOperation{final}})
	if err = second.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	done, _ := second.get(w.ID)
	if len(done.Tasks) != 2 || done.Tasks[1].ID != "final-retex" || done.Planning.Decisions != beforeDecisions+1 || done.Planning.Activations != beforeActs+3 || done.Planning.Scopes[0].RejectionWait != nil {
		t.Fatal("recovery lost history or acceptance boundary", done.Planning)
	}
	if done.Tasks[1].Status == "accepted" {
		t.Fatal("recovery fabricated acceptance")
	}
}

func TestPlanningRejectionWaitIgnoresUnrelatedActivity(t *testing.T) {
	w := Work{Criteria: []string{"A", "B"}, Planning: &PlanningState{Scopes: []PlanningScope{{ID: "root"}, {ID: "a", Parent: "root"}, {ID: "b", Parent: "root"}}, Reviewer: &ReviewerConfig{Calls: 3, MaxCalls: 20}, Inbox: []PlanningEvent{{ID: "source-a", Scope: "a", Kind: "operator_resume"}}}, Tasks: []Task{{ID: "task-a", ScopeID: "a", Next: "A"}, {ID: "task-b", ScopeID: "b", Next: "B"}}}
	owner, _ := w.Planning.scope("a")
	owner.RejectionWait = &PlanningRejectionWait{Event: "diagnostic", StateSHA256: planningRejectionState(w, owner)}
	w.Revision++
	w.Planning.Activations++
	w.Planning.Decisions++
	w.Planning.Reviewer.Calls++
	owner.Revision++
	owner.Generation++
	owner.Until = now()
	owner.LastModel = &RoleModel{At: now()}
	w.Tasks[1].Next = "Unrelated B activity"
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "diagnostic", Scope: "a", Kind: "planning_rejected"}, PlanningEvent{ID: "source-b", Scope: "b", Kind: "operator_resume"})
	if !planningRejectionUnchanged(w, owner) {
		t.Fatal("lease, counters or other scope create false progress")
	}
	w.Tasks[0].Next = "Corrected owned instruction"
	if planningRejectionUnchanged(w, owner) {
		t.Fatal("owned instruction not recognized")
	}
	w.Tasks[0].Next = "A"
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "source-new-a", Scope: "a", Kind: "operator_resume"})
	if planningRejectionUnchanged(w, owner) {
		t.Fatal("new public owned input not recognized")
	}
}

func TestPlanningLateRejectionCannotRevokeAppliedDecision(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("existing")}
	w = planningDo(t, s, w, "decide", r)
	if err := s.planningRejected(w.ID, "root", r.Generation, "late-claim", `{"late":true}`, &planningOperationError{Kind: "task", ID: "existing", Err: os.ErrExist}); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || after.Planning.Failure != "" || after.Planning.Decisions != w.Planning.Decisions || len(after.Planning.Inbox) != len(w.Planning.Inbox) {
		t.Fatal("late rejection changed applied decision")
	}
}

func TestPlanningRejectionRacesWithValidDecision(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("fresh")}
	second, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer second.db.Close()
	start := make(chan struct{})
	done := make(chan error, 2)
	go func() { <-start; _, err := s.planningChange(w.ID, "decide", r); done <- err }()
	go func() {
		<-start
		done <- second.planningRejected(w.ID, "root", r.Generation, "racing-claim", `{"operations":[]}`, &planningOperationError{Kind: "task", ID: "old", Err: os.ErrExist})
	}()
	close(start)
	<-done
	<-done
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	diagnostics := 0
	for _, event := range after.Planning.Inbox {
		if event.Kind == "planning_rejected" {
			diagnostics++
		}
	}
	owner, _ := after.Planning.scope("root")
	if owner.Holder != "" || after.Planning.Failure != "" || after.Planning.Activations != w.Planning.Activations {
		t.Fatal("race revoked wrong owner or changed consumption")
	}
	if after.Planning.Decisions == w.Planning.Decisions+1 {
		if len(after.Tasks) != 1 || diagnostics != 0 {
			t.Fatal("late rejection corrupted valid decision")
		}
	} else if after.Planning.Decisions == w.Planning.Decisions {
		if len(after.Tasks) != 0 || diagnostics != 1 {
			t.Fatal("decision escaped rejection generation boundary")
		}
	} else {
		t.Fatal("multiple decisions applied")
	}
}

func TestPlanningDuplicateWithinProposalPreservesBothIdentities(t *testing.T) {
	s, w := planningFixture(t)
	w, r := planningClaim(t, s, w, "root")
	r.Operations = []PlanningOperation{planningTask("new-task"), planningTask("new-task")}
	_, err := s.planningChange(w.ID, "decide", r)
	if err == nil || !strings.Contains(err.Error(), "opération 2 task[new-task]") {
		t.Fatal("duplicate location missing", err)
	}
	raw, _ := json.Marshal(PlanningProposal{Inputs: r.Inputs, Reason: r.Reason, Operations: r.Operations})
	if e := s.planningRejected(w.ID, "root", r.Generation, "within-proposal", string(raw), err); e == nil {
		t.Fatal("rejection accepted")
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks) != 0 || after.Planning.Decisions != 0 || after.Planning.Activations != w.Planning.Activations || after.Planning.Failure != "" {
		t.Fatal("partial operation escaped rollback")
	}
	event := after.Planning.Inbox[len(after.Planning.Inbox)-1]
	report, e := s.planningReport(event)
	if e != nil {
		t.Fatal(e)
	}
	var receipt struct {
		Inventory []struct {
			Index int    `json:"index"`
			Kind  string `json:"kind"`
			ID    string `json:"id"`
		} `json:"operation_inventory"`
	}
	if e = json.Unmarshal([]byte(report.Text), &receipt); e != nil || len(receipt.Inventory) != 2 || receipt.Inventory[0].ID != "new-task" || receipt.Inventory[1].ID != "new-task" || receipt.Inventory[1].Index != 2 {
		t.Fatal("same-proposal duplication hidden", report.Text, e)
	}
}

func TestPlanningRejectionWaitAllowsOtherScopeInference(t *testing.T) {
	s, w := planningFixture(t)
	script := filepath.Join(t.TempDir(), "claude")
	body := `{"type":"result","structured_output":{"input_events":["source-b"],"reason":"Other owner remains independently eligible","operations":[]}}`
	if err := os.WriteFile(script, []byte("#!/bin/sh\ncat >\"$0.prompt\"\nprintf '%s\\n' '"+body+"'\n"), 0700); err != nil {
		t.Fatal(err)
	}
	provider := Provider{Command: script}
	config, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"test": provider}})
	if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), config, 0600); err != nil {
		t.Fatal(err)
	}
	var err error
	w, err = s.mutate(w.ID, "test.scopes", "two-owners", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Criteria = append(w.Criteria, "Other requirement")
		w.Planning.Provider = "test"
		raw, _ := json.Marshal(provider)
		w.Planning.ProviderDigest = hash(raw)
		w.Planning.Scopes = []PlanningScope{{ID: "root", State: "waiting", Revision: 1}, {ID: "a", Parent: "root", State: "ready", Revision: 1, Requirements: []string{"req-1"}}, {ID: "b", Parent: "root", State: "ready", Revision: 1, Requirements: []string{"req-2"}}}
		w.Planning.Inbox = []PlanningEvent{{ID: "source-a", Scope: "a", Kind: "operator_resume"}, {ID: "source-b", Scope: "b", Kind: "operator_resume"}}
		owner, _ := w.Planning.scope("a")
		owner.RejectionWait = &PlanningRejectionWait{Event: "prior-rejection", StateSHA256: planningRejectionState(*w, owner)}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	if err = s.planningStep(w.ID); err != nil {
		t.Fatal(err)
	}
	after, _ := s.get(w.ID)
	a, _ := after.Planning.scope("a")
	b, _ := after.Planning.scope("b")
	if after.Planning.Failure != "" || !planningRejectionUnchanged(after, a) || a.Activations != 0 || b.Activations != 1 || after.Planning.Decisions != 1 || after.Planning.Inbox[0].Decision != "" || after.Planning.Inbox[1].Decision == "" {
		t.Fatal("unchanged owner blocked other scope or lost its own input", after.Planning)
	}
}
