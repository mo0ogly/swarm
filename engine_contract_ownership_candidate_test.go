//go:build linux

package main

import (
	"encoding/json"
	"path/filepath"
	"strings"
	"testing"
)

// TestEngineContractOwnershipRequirementsAreOwnedExactlyOnce demonstrates
// unique requirement ownership both at construction time (two scopes cannot
// list the same requirement) and at delegation time (a requirement already
// handed to one child cannot be delegated a second time to a sibling): it is
// moved, never copied.
func TestEngineContractOwnershipRequirementsAreOwnedExactlyOnce(t *testing.T) {
	s, w := planningFixture(t)

	duplicated := w
	duplicated.Planning = &PlanningState{Version: w.Planning.Version, MaxTasks: w.Planning.MaxTasks, MaxDecisions: w.Planning.MaxDecisions, MaxActivations: w.Planning.MaxActivations, Scopes: []PlanningScope{
		{ID: "root", Revision: 1, Requirements: []string{"req-1"}},
		{ID: "duplicate", Parent: "root", Revision: 1, Requirements: []string{"req-1"}},
	}}
	if err := validatePlanningState(&duplicated); err == nil || !strings.Contains(err.Error(), "possédée plusieurs fois") {
		t.Fatalf("two scopes were allowed to own the same requirement at once: %v", err)
	}

	w, decision := planningClaim(t, s, w, "root")
	decision.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Premier porteur", Requirements: []string{"req-1"}}}
	w, err := s.planningChange(w.ID, "decide", decision)
	if err != nil {
		t.Fatal(err)
	}
	root, _ := w.Planning.scope("root")
	if len(root.Requirements) != 0 {
		t.Fatalf("root retained the requirement it had just delegated away: %+v", root)
	}
	child, _ := w.Planning.scope("child")
	if len(child.Requirements) != 1 || child.Requirements[0] != "req-1" {
		t.Fatalf("child did not receive exclusive ownership of req-1: %+v", child)
	}

	w = planningDo(t, s, w, "resume", PlanningRequest{Reason: "Tenter une seconde délégation du même besoin"})
	w, second := planningClaim(t, s, w, "root")
	second.Operations = []PlanningOperation{{Kind: "delegate", ID: "sibling", Title: "Second porteur", Requirements: []string{"req-1"}}}
	if _, err = s.planningChange(w.ID, "decide", second); err == nil || !strings.Contains(err.Error(), "exigence répétée ou non possédée") {
		t.Fatalf("a requirement already owned by another scope was delegated a second time: %v", err)
	}
}

// TestEngineContractOwnershipConcurrentClaimIsExclusiveAndRecoversAfterResume
// covers the concurrency limit of a hierarchical mission: a scope has at most
// one active planner at a time, a second concurrent claim is refused, and the
// refusal never mutates the mission. Recovery only happens through the
// explicit pause/resume revocation, never by silently allowing a duplicate.
func TestEngineContractOwnershipConcurrentClaimIsExclusiveAndRecoversAfterResume(t *testing.T) {
	s, w := planningFixture(t)
	root, err := w.Planning.scope("root")
	if err != nil {
		t.Fatal(err)
	}
	first := PlanningRequest{Schema: 1, EventID: newID("claim-"), Revision: w.Revision, Scope: "root", ScopeRevision: root.Revision, Holder: "owner-a", LeaseSeconds: 60}
	w, err = s.planningChange(w.ID, "claim", first)
	if err != nil {
		t.Fatal(err)
	}
	root, _ = w.Planning.scope("root")
	if root.Holder != "owner-a" {
		t.Fatalf("first planner did not acquire the scope: %+v", root)
	}

	second := PlanningRequest{Schema: 1, EventID: newID("claim-"), Revision: w.Revision, Scope: "root", ScopeRevision: root.Revision, Holder: "owner-b", LeaseSeconds: 60}
	if _, err = s.planningChange(w.ID, "claim", second); err == nil || !strings.Contains(err.Error(), "possède déjà ce périmètre") {
		t.Fatalf("a second concurrent planner claim was not refused: %v", err)
	}
	unchanged, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if unchanged.Revision != w.Revision {
		t.Fatal("refused concurrent claim mutated the mission")
	}

	w = planningDo(t, s, unchanged, "resume", PlanningRequest{Reason: "Libérer le périmètre pour un nouveau porteur"})
	root, _ = w.Planning.scope("root")
	if root.Holder != "" {
		t.Fatalf("resume did not revoke the outstanding lease: %+v", root)
	}
	w, err = s.planningChange(w.ID, "claim", PlanningRequest{Schema: 1, EventID: newID("claim-"), Revision: w.Revision, Scope: "root", ScopeRevision: root.Revision, Holder: "owner-b", LeaseSeconds: 60})
	if err != nil {
		t.Fatal(err)
	}
	root, _ = w.Planning.scope("root")
	if root.Holder != "owner-b" {
		t.Fatalf("the recovered scope was not claimable by the waiting owner: %+v", root)
	}
}

// TestEngineContractOwnershipRefusesParentClosureBeforeChildrenValidated fills
// a gap left untested elsewhere: closing a parent scope while its delegated
// child is still open must fail, without mutating the mission, and only
// succeed once the child has been validated with fresh proof and closed.
func TestEngineContractOwnershipRefusesParentClosureBeforeChildrenValidated(t *testing.T) {
	s, w := planningFixture(t)
	// Establish the reviewer/checks fixture before any delegation happens: it
	// resets scope 0's requirements to the work's full criteria, which must
	// still match root's real (undelegated) ownership at this point.
	organizedFixtureStore(t, s)
	w, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, rootDecision := planningClaim(t, s, w, "root")
	rootDecision.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Sous-périmètre", Requirements: []string{"req-1"}, Next: "Valider req-1"}}
	w, err = s.planningChange(w.ID, "decide", rootDecision)
	if err != nil {
		t.Fatal(err)
	}
	if len(w.Planning.Scopes) != 2 {
		t.Fatalf("delegation did not create the expected child scope: %+v", w.Planning.Scopes)
	}

	// Root wakes itself to attempt a premature close while its child is open.
	w = planningDo(t, s, w, "resume", PlanningRequest{Reason: "Contrôler une clôture prématurée du parent"})
	w, closeAttempt := planningClaim(t, s, w, "root")
	closeAttempt.Operations = []PlanningOperation{{Kind: "close"}}
	preClose := w.Revision
	if _, err = s.planningChange(w.ID, "decide", closeAttempt); err == nil || !strings.Contains(err.Error(), "périmètre enfant non terminé") {
		t.Fatalf("the parent scope closed before its child was validated: %v", err)
	}
	unchanged, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if unchanged.Revision != preClose {
		t.Fatal("the refused closure mutated the mission")
	}

	// The failed decide left root's lease held; recover it before continuing.
	w = planningDo(t, s, unchanged, "resume", PlanningRequest{Reason: "Libérer le périmètre racine pour poursuivre"})

	w, childDecision := planningClaim(t, s, w, "child")
	childDecision.Operations = []PlanningOperation{planningTask("t1")}
	w, err = s.planningChange(w.ID, "decide", childDecision)
	if err != nil {
		t.Fatal(err)
	}
	raw := fixture(t, s.root)
	w = gateTest(t, s, w, raw)
	w, err = s.mutate(w.ID, "test.accept-child-proof", "accept-child-proof", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].Status = "accepted"
		w.Tasks[0].Attempts = []Attempt{{ID: "fixture-production", Status: "completed"}}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	approveReportFixture(t, s, w.ID, "t1", "fixture-producer", "proof.txt")
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}

	w, childClose := planningClaim(t, s, w, "child")
	childClose.Operations = []PlanningOperation{{Kind: "close"}}
	w, err = s.planningChange(w.ID, "decide", childClose)
	if err != nil {
		t.Fatal(err)
	}
	child, _ := w.Planning.scope("child")
	root, _ := w.Planning.scope("root")
	if child.State != "closed" || root.State != "ready" {
		t.Fatalf("closing the validated child did not wake its parent owner: child=%+v root=%+v", child, root)
	}

	w, rootClose := planningClaim(t, s, w, "root")
	rootClose.Operations = []PlanningOperation{{Kind: "close"}}
	w, err = s.planningChange(w.ID, "decide", rootClose)
	if err != nil {
		t.Fatalf("the parent close was refused after every child was validated: %v", err)
	}
	root, _ = w.Planning.scope("root")
	if root.State != "closed" {
		t.Fatalf("root did not close once its only child was validated: %+v", root)
	}
}

// TestEngineContractOwnershipRootChildGrandchildDelegationBoundsDepthAndIsolatesCopies
// exercises a real three-level ownership chain (root -> child -> leaf), the
// bounded depth limit (a fourth level is refused), the resulting worker's
// isolated ownership, and that isolated executor copy paths are structurally
// distinct per task and per attempt.
func TestEngineContractOwnershipRootChildGrandchildDelegationBoundsDepthAndIsolatesCopies(t *testing.T) {
	s, w := planningFixture(t)
	w, rootDecision := planningClaim(t, s, w, "root")
	rootDecision.Operations = []PlanningOperation{{Kind: "delegate", ID: "child", Title: "Sous-périmètre", Requirements: []string{"req-1"}, Next: "Déléguer à la feuille"}}
	w, err := s.planningChange(w.ID, "decide", rootDecision)
	if err != nil {
		t.Fatal(err)
	}
	w, childDecision := planningClaim(t, s, w, "child")
	childDecision.Operations = []PlanningOperation{{Kind: "delegate", ID: "leaf", Title: "Périmètre feuille", Requirements: []string{"req-1"}, Next: "Confier l’exécution"}}
	w, err = s.planningChange(w.ID, "decide", childDecision)
	if err != nil {
		t.Fatal(err)
	}

	w, leafDecision := planningClaim(t, s, w, "leaf")
	tooDeep := leafDecision
	tooDeep.Operations = []PlanningOperation{{Kind: "delegate", ID: "too-deep", Title: "Niveau interdit", Requirements: []string{"req-1"}}}
	if _, err = s.planningChange(w.ID, "decide", tooDeep); err == nil || !strings.Contains(err.Error(), "profondeur") {
		t.Fatalf("a fourth ownership level (root/child/leaf/too-deep) was accepted: %v", err)
	}

	leafDecision.Operations = []PlanningOperation{planningTask("worker")}
	w, err = s.planningChange(w.ID, "decide", leafDecision)
	if err != nil {
		t.Fatal(err)
	}
	worker, err := w.task("worker")
	if err != nil || worker.ScopeID != "leaf" || worker.PlanRole != "worker" {
		t.Fatalf("the delegated executable task is not an isolated worker under its own scope: %+v err=%v", worker, err)
	}

	repo := &ManagedRepository{Storage: t.TempDir(), Subdir: "svc"}
	firstAttempt := managedCopyPath(repo, "worker", 1)
	retryAttempt := managedCopyPath(repo, "worker", 2)
	otherWorker := managedCopyPath(repo, "worker-b", 1)
	if firstAttempt == retryAttempt || firstAttempt == otherWorker || retryAttempt == otherWorker {
		t.Fatalf("executor copies are not structurally distinct: %s / %s / %s", firstAttempt, retryAttempt, otherWorker)
	}
	copiesRoot := filepath.Join(repo.Storage, "copies")
	for _, path := range []string{firstAttempt, retryAttempt, otherWorker} {
		if !strings.HasPrefix(path, copiesRoot+string(filepath.Separator)) {
			t.Fatalf("copy path escapes the managed storage root %q: %s", copiesRoot, path)
		}
	}
}

// TestEngineContractOwnershipForbidsLateralParentLinkAndNonWorkerLaunch checks
// that hierarchical missions forbid both forms of lateral communication: a
// direct executor-to-executor help exchange, and the Agent.Parent side
// channel at launch time. It also confirms the runtime launch guard, not just
// the offline validator, refuses to launch a responsable (a non-worker plan
// role) as a coding executor.
func TestEngineContractOwnershipForbidsLateralParentLinkAndNonWorkerLaunch(t *testing.T) {
	s, w := planningFixture(t)
	w, decision := planningClaim(t, s, w, "root")
	decision.Operations = []PlanningOperation{planningTask("one")}
	w, err := s.planningChange(w.ID, "decide", decision)
	if err != nil {
		t.Fatal(err)
	}

	_, _, err = s.sendExchange(w.ID, ExchangeSend{Schema: 1, EventID: "ownership-crosstalk", Kind: "help_request", AgentID: "worker-one", TaskID: "one", AttemptID: "attempt-one", RecipientTask: "one", RecipientRole: "worker", Need: "contourner le responsable", Timeout: 60})
	if err == nil || !strings.Contains(err.Error(), "responsable via planning handoff") {
		t.Fatalf("direct lateral cross-talk between executors was accepted: %v", err)
	}

	launchStore := storeTest(t)
	launchWork, launch := setupAgent(t, launchStore)
	organizedFixtureStore(t, launchStore)
	launchWork, err = launchStore.get(launchWork.ID)
	if err != nil {
		t.Fatal(err)
	}
	launch.Revision = launchWork.Revision
	launch.Parent = "another-worker"
	if _, _, err = launchStore.prepare(launchWork.ID, launch); err == nil || !strings.Contains(err.Error(), "communication directe entre exécutants interdite") {
		t.Fatalf("the Agent.Parent lateral side channel was accepted at launch: %v", err)
	}

	// The normal mutate path already fails closed on this (validatePlanningState
	// rejects a non-worker plan role on an executable task). Simulate an
	// imported or historical payload that bypassed that check, to confirm the
	// runtime launch guard independently refuses it too, not only the offline
	// validator relied on by TestCursorContractHierarchicalWorkersCannotCrossTalkOrBecomePlanners.
	corrupted := launchWork
	corrupted.Tasks = append([]Task(nil), launchWork.Tasks...)
	corrupted.Tasks[0].PlanRole = "planner"
	raw, err := json.Marshal(corrupted)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = launchStore.db.Exec("UPDATE works SET body=? WHERE id=?", raw, launchWork.ID); err != nil {
		t.Fatal(err)
	}
	launch.Parent = ""
	launch.EventID = newID("agent-")
	if _, _, err = launchStore.prepare(launchWork.ID, launch); err == nil || !strings.Contains(err.Error(), "seuls les exécutants peuvent lancer une tâche") {
		t.Fatalf("a responsable (non-worker plan role) was launched as a coding executor: %v", err)
	}
}

// The authorized ownership control must execute these obligations too. Calling
// the existing behavioral tests preserves their assertions instead of duplicating
// them or relying on a separate, unreceipted full-suite claim.
func TestEngineContractOwnershipPlannerAndHandoffEvidence(t *testing.T) {
	t.Run("planner_without_coding_tools", TestCursorContractPlannerIsToolFreeAndRootOwnsNeed)
	t.Run("handoff_to_owner_and_reactivation", TestCursorContractRecursiveDelegationHandoffAndAdaptation)
}
