//go:build linux

package engine

import (
	"context"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

func identityPlanFailure(err error) *CommandError {
	if err == nil {
		return &CommandError{}
	}
	return commandFailure(err)
}

func identityPlanWork(t *testing.T, f agentIdentityRegistryFixture) Work {
	t.Helper()
	w, err := f.store.get(f.permit.scope.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	return w
}

func identityPlanEdit(t *testing.T, f agentIdentityRegistryFixture, edit func(*Work)) Work {
	t.Helper()
	w := identityPlanWork(t, f)
	w, err := f.store.mutate(w.ID, "work.update", newID("plan-edit-"), w.Revision, []byte(`{}`), func(w *Work) error {
		edit(w)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	return w
}

func TestAgentIdentityPlanProjectionBindsContractAndPreservesRuntime(t *testing.T) {
	f := registryFixture(t)
	w := identityPlanWork(t, f)
	raw, err := json.Marshal(w)
	if err != nil {
		t.Fatal(err)
	}
	base, err := agentIdentityRecordedPlanDigest(raw, w.ID, w.Revision)
	if err != nil {
		t.Fatal(err)
	}
	cases := []struct {
		name string
		edit func(*Work)
		same bool
	}{
		{"objective", func(w *Work) { w.Objective += " changed" }, false},
		{"scope", func(w *Work) { w.Scope += " changed" }, false},
		{"owner", func(w *Work) { w.Tasks[1].Owner = "another owner" }, false},
		{"role", func(w *Work) { w.Tasks[1].PlanRole = "planner" }, false},
		{"definition", func(w *Work) { w.Tasks[1].Criteria = []string{"changed"} }, false},
		{"instructions", func(w *Work) { w.Tasks[1].Next += " changed" }, false},
		{"dependencies", func(w *Work) { w.Tasks[1].Depends = []string{w.Tasks[0].ID} }, false},
		{"reservation", func(w *Work) { w.Tasks[1].PlanToolLimit++ }, false},
		{"provider", func(w *Work) { w.Profile = &LaunchProfile{Provider: "another"} }, false},
		{"workspace", func(w *Work) { w.Tasks[1].Profile = &LaunchProfile{Workspace: "another"} }, false},
		{"requirement-owner", func(w *Work) { w.Tasks[1].ScopeID = "other" }, false},
		{"source-plan", func(w *Work) { w.Plans[0].ResponseHash = "changed" }, false},
		{"revision", func(w *Work) { w.Revision++ }, true},
		{"runtime", func(w *Work) {
			w.Updated, w.Summary = "later", "observed result"
			w.Git = GitState{Head: "another commit"}
			w.Tasks[1].Status, w.Tasks[1].Blocker, w.Tasks[1].Response = "blocked", "observed failure", "result"
			w.Tasks[1].Attempts = []Attempt{{ID: "attempt-record", Status: "completed"}}
		}, true},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			var changed Work
			if err := json.Unmarshal(raw, &changed); err != nil {
				t.Fatal(err)
			}
			tc.edit(&changed)
			encoded, err := json.Marshal(changed)
			if err != nil {
				t.Fatal(err)
			}
			digest, err := agentIdentityRecordedPlanDigest(encoded, changed.ID, changed.Revision)
			if err != nil || (digest == base) != tc.same {
				t.Fatalf("projection contract mismatch: same=%t digest=%s base=%s err=%v", tc.same, digest, base, err)
			}
		})
	}
	for _, key := range []string{"future_work_field", "future_task_field"} {
		t.Run(key, func(t *testing.T) {
			var object map[string]json.RawMessage
			if err := json.Unmarshal(raw, &object); err != nil {
				t.Fatal(err)
			}
			if key == "future_work_field" {
				object[key] = json.RawMessage(`9007199254740993`)
			} else {
				var tasks []map[string]json.RawMessage
				if err := json.Unmarshal(object["tasks"], &tasks); err != nil {
					t.Fatal(err)
				}
				tasks[1][key] = json.RawMessage(`9007199254740993`)
				object["tasks"], err = json.Marshal(tasks)
				if err != nil {
					t.Fatal(err)
				}
			}
			encoded, err := json.Marshal(object)
			if err != nil {
				t.Fatal(err)
			}
			digest, err := agentIdentityRecordedPlanDigest(encoded, w.ID, w.Revision)
			if err != nil || digest == base {
				t.Fatal("unknown field silently omitted: ", err)
			}
		})
	}
	first, err := agentIdentityPlanProjectionDigest([]byte(`{"future_number":9007199254740992}`))
	if err != nil {
		t.Fatal(err)
	}
	second, err := agentIdentityPlanProjectionDigest([]byte(`{"future_number":9007199254740993}`))
	if err != nil || first == second {
		t.Fatal("projection rounded large integer values: ", err)
	}
}

func TestAgentIdentityPlanGenerationAndFreshPublication(t *testing.T) {
	f := registryFixture(t)
	cp := registryPublish(t, f)
	before := f.permit.scope
	identityPlanEdit(t, f, func(w *Work) { w.Tasks[1].Status = "blocked" })
	same, err := f.store.readAgentIdentityNextPlanScope(registryContext(t), cp)
	if err != nil || same != before {
		t.Fatal("runtime transition changed plan generation: ", same, err)
	}
	if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); err != nil {
		t.Fatal("runtime transition invalidated authority: ", err)
	}
	identityPlanEdit(t, f, func(w *Work) { w.Objective += " contract amendment" })
	after, err := f.store.readAgentIdentityNextPlanScope(registryContext(t), cp)
	if err != nil || after.PlanVersion != before.PlanVersion+1 || after.PlanDigest == before.PlanDigest {
		t.Fatal("recorded change did not advance generation: ", after, err)
	}
	if _, err = f.store.readAgentIdentityNextPlanScope(registryContext(t), f.permit.base); err == nil {
		t.Fatal("empty checkpoint adopted an existing head")
	}
	next := registryNext(t, f, cp)
	registryApproveFixture(t, &next)
	trustBefore := registryTrustBytes(t, f.store)
	a, b := registryCounts(t, f.store)
	if _, err = f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, next.snapshot), trustRequesterToken, next.permit, nil); identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("old permit was not rejected against the fresh plan: ", err)
	}
	registryAssertUnchanged(t, f.store, trustBefore, a, b)
	// A changed scope needs a distinct root; historical grants stay unchanged.
	root := next.snapshot.Grants["grant-root"]
	root.ID, root.Scope = "grant-root-plan2", after
	next.snapshot.Grants[root.ID] = root
	next.permit.scope, next.permit.roots = after, []string{root.ID}
	registryApproveFixture(t, &next) // fixture, not a real human issuer
	current := registryPublish(t, next)
	_, authority, err := f.store.readAgentIdentityRegistry(registryContext(t), current)
	if err != nil || authority.scope != after || !authority.roots[root.ID] || authority.roots["grant-root"] {
		t.Fatal("fresh publication lost new scope or promoted old root: ", err)
	}
}

func TestAgentIdentityPlanRecoveryFinishesHistoryWithoutReactivation(t *testing.T) {
	for _, phase := range []string{"intent_persisted", "sqlite_committed"} {
		t.Run(phase, func(t *testing.T) {
			f := registryFixture(t)
			injected := errors.New("fixture interruption")
			_, err := f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), trustRequesterToken, f.permit, func(at string) error {
				if at == phase {
					return injected
				}
				return nil
			})
			if !errors.Is(err, injected) {
				t.Fatal("publication boundary not reached: ", err)
			}
			identityPlanEdit(t, f, func(w *Work) { w.Objective += " changed while pending" })
			cp, err := f.store.recoverAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), f.permit)
			if err != nil {
				t.Fatal("historical publication cannot recover: ", err)
			}
			if a, b := registryCounts(t, f.store); a != 1 || b != 1 {
				t.Fatal("recovery duplicated history: ", a, b)
			}
			if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
				t.Fatal("historical recovery reactivated changed plan: ", err)
			}
		})
	}
}

func TestAgentIdentityPlanHierarchyTracksRealProducer(t *testing.T) {
	s, w := planningFixture(t)
	before, err := readAgentIdentityRecordedPlan(registryContext(t), s.db, w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w, r := planningClaim(t, s, w, "root")
	claimed, err := readAgentIdentityRecordedPlan(registryContext(t), s.db, w.ID)
	if err != nil || claimed != before {
		t.Fatal("claim/lease changed hierarchical contract: ", before, claimed, err)
	}
	r.Operations = []PlanningOperation{planningTask("first")}
	w = planningDo(t, s, w, "decide", r)
	after, err := readAgentIdentityRecordedPlan(registryContext(t), s.db, w.ID)
	if err != nil || after.PlanVersion != before.PlanVersion+1 || after.PlanDigest == before.PlanDigest {
		t.Fatal("supervisor decision did not advance contract generation: ", after, err)
	}
}

func TestAgentIdentityPlanWaitReadsCommittedContract(t *testing.T) {
	f := registryFixture(t)
	w := identityPlanWork(t, f)
	f.store.db.SetMaxOpenConns(1)
	tx, err := f.store.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	ctx, cancel := context.WithTimeout(context.Background(), 3*time.Second)
	defer cancel()
	done := make(chan error, 1)
	proposal := registryRaw(t, f.snapshot)
	trustBefore := registryTrustBytes(t, f.store)
	waits := f.store.db.Stats().WaitCount
	go func() {
		_, err := f.store.publishAgentIdentityRegistry(ctx, proposal, trustRequesterToken, f.permit, nil)
		done <- err
	}()
	// Observe the SQL wait without competing for the publisher's nonblocking
	// Trust lock. A repeated shared-lock probe could itself reject publication.
	for f.store.db.Stats().WaitCount == waits {
		select {
		case err := <-done:
			t.Fatal("publisher returned before its SQL wait: ", err)
		default:
		}
		if err = ctx.Err(); err != nil {
			t.Fatal("publisher did not reach its wait: ", err)
		}
		time.Sleep(time.Millisecond)
	}
	if _, err = f.store.withTrustReadAvailable(func(*trustState) (any, error) { return nil, nil }); identityPlanFailure(err).Code != "workspace_wait" {
		t.Fatal("SQL wait was not under the Trust lock: ", err)
	}
	w.Revision++
	w.Objective += " committed during wait"
	raw, err := json.Marshal(w)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = tx.Exec(`UPDATE works SET revision=?,body=? WHERE id=?`, w.Revision, raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if err = tx.Commit(); err != nil {
		t.Fatal(err)
	}
	if err = <-done; identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("publisher checked the plan before its wait: ", err)
	}
	registryAssertUnchanged(t, f.store, trustBefore, 0, 0)
}

func TestAgentIdentityPlanMigrationAndStampIntegrity(t *testing.T) {
	f := registryFixture(t)
	w := identityPlanWork(t, f)
	if _, err := f.store.db.Exec(`DROP TRIGGER agent_identity_plan_insert; DROP TRIGGER agent_identity_plan_update_guard; DROP TRIGGER agent_identity_plan_update; DROP TABLE agent_identity_plan_heads; PRAGMA user_version=36`); err != nil {
		t.Fatal(err)
	}
	if err := f.store.db.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err := openStoreWithMigration(f.store.root, false, false); identityPlanFailure(err).Code != "storage_upgrade_required" {
		t.Fatal("inspection implicitly migrated: ", err)
	}
	s, err := openStore(f.store.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer s.db.Close()
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v37-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal("schema 37 backup missing: ", backups, err)
	}
	backup, err := os.Stat(backups[0])
	if err != nil || backup.Mode().Perm() != 0600 {
		t.Fatal("schema 37 backup is not private: ", err)
	}
	got, err := s.get(w.ID)
	if err != nil || !reflect.DeepEqual(got, w) {
		t.Fatal("migration rewrote historical Work: ", err)
	}
	scope, err := s.readAgentIdentityNextPlanScope(registryContext(t), f.permit.base)
	if err != nil || scope.PlanVersion != 1 || scope.PlanDigest != f.permit.scope.PlanDigest {
		t.Fatal("migration failed to initialize a contract stamp: ", scope, err)
	}
	if a, b := registryCounts(t, s); a != 0 || b != 0 {
		t.Fatal("migration granted identity authority: ", a, b)
	}
	if _, err = s.db.Exec(`UPDATE agent_identity_plan_heads SET digest=? WHERE work_id=?`, hash([]byte("tampered stamp")), w.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = s.readAgentIdentityNextPlanScope(registryContext(t), f.permit.base); identityPlanFailure(err).Code != "agent_identity_plan_stamp_changed" {
		t.Fatal("tampered stamp accepted: ", err)
	}
	if _, err = s.db.Exec(`UPDATE works SET body=body WHERE id=?`, w.ID); err == nil {
		t.Fatal("writer ignored a missing/changed prior stamp")
	}
	afterRefusal, err := s.get(w.ID)
	if err != nil || !reflect.DeepEqual(afterRefusal, w) {
		t.Fatal("rejected writer mutated Work", err)
	}
}

func TestAgentIdentityPlanDoesNotReactivateAfterChangeAndRevert(t *testing.T) {
	f := registryFixture(t)
	cp := registryPublish(t, f)
	w, err := f.store.get(f.permit.scope.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	original := w.Objective
	for i, objective := range []string{"temporary changed objective", original} {
		w, err = f.store.mutate(w.ID, "work.update", []string{"change", "revert"}[i], w.Revision, []byte(`{}`), func(w *Work) error {
			w.Objective = objective
			return nil
		})
		if err != nil {
			t.Fatal(err)
		}
	}
	if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); err == nil || identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("returning to old contents reactivated old authority: ", err)
	}
	if err = f.store.db.Close(); err != nil {
		t.Fatal(err)
	}
	s, err := openStore(f.store.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer s.db.Close()
	if _, _, err = s.readAgentIdentityRegistry(registryContext(t), cp); identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("reopening lost the plan generation: ", err)
	}
}

func TestAgentIdentityPlanGenerationRollsBackAndRejectsOverflow(t *testing.T) {
	f := registryFixture(t)
	cp := registryPublish(t, f)
	w := identityPlanWork(t, f)
	changed := w
	changed.Revision++
	changed.Objective += " temporary uncommitted contract"
	raw, err := json.Marshal(changed)
	if err != nil {
		t.Fatal(err)
	}
	tx, err := f.store.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if _, err = tx.Exec(`UPDATE works SET revision=?,body=? WHERE id=?`, changed.Revision, raw, w.ID); err != nil {
		t.Fatal(err)
	}
	inside, err := readAgentIdentityRecordedPlan(registryContext(t), tx, w.ID)
	if err != nil || inside.PlanVersion != f.permit.scope.PlanVersion+1 {
		t.Fatal("trigger is not in the Work transaction: ", inside, err)
	}
	if err = tx.Rollback(); err != nil {
		t.Fatal(err)
	}
	if _, authority, err := f.store.readAgentIdentityRegistry(registryContext(t), cp); err != nil || authority.scope != f.permit.scope {
		t.Fatal("rolled-back contract invalidated authority: ", err)
	}
	if _, err = f.store.db.Exec(`UPDATE agent_identity_plan_heads SET generation=9223372036854775807 WHERE work_id=?`, w.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = f.store.db.Exec(`UPDATE works SET revision=?,body=? WHERE id=?`, changed.Revision, raw, w.ID); err == nil {
		t.Fatal("integer generation overflow accepted")
	}
	after := identityPlanWork(t, f)
	if !reflect.DeepEqual(after, w) {
		t.Fatal("overflow committed a partial Work update")
	}
}

func TestAgentIdentityPlanUsesPreparationConversion(t *testing.T) {
	s := storeTest(t)
	p := readyPreparation(t, s, "")
	p, err := s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
	if err != nil {
		t.Fatal(err)
	}
	scope, err := readAgentIdentityRecordedPlan(registryContext(t), s.db, p.WorkID)
	if err != nil || scope.WorkID != p.WorkID || scope.PlanVersion < 1 || !agentIdentityHashValid(scope.PlanDigest) {
		t.Fatal("converted plan did not reach the shared producer: ", scope, err)
	}
	var snapshots, receipts int
	if err = s.db.QueryRow(`SELECT count(*) FROM agent_identity_snapshots`).Scan(&snapshots); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow(`SELECT count(*) FROM agent_identity_publications`).Scan(&receipts); err != nil || snapshots != 0 || receipts != 0 {
		t.Fatal("preparation conversion minted identity authority: ", snapshots, receipts, err)
	}
}

func TestAgentIdentityPlanCanonicalObjectOrder(t *testing.T) {
	f := registryFixture(t)
	w, err := f.store.get(f.permit.scope.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(w)
	if err != nil {
		t.Fatal(err)
	}
	before, err := agentIdentityRecordedPlanDigest(raw, w.ID, w.Revision)
	if err != nil {
		t.Fatal(err)
	}
	var unordered any
	if err = json.Unmarshal(raw, &unordered); err != nil {
		t.Fatal(err)
	}
	reordered, err := json.MarshalIndent(unordered, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	after, err := agentIdentityRecordedPlanDigest(reordered, w.ID, w.Revision)
	if err != nil || before != after {
		t.Fatal("JSON key order changed a supposedly canonical plan projection: ", err)
	}
}

func TestAgentIdentityPlanRefusesArbitraryScope(t *testing.T) {
	f := registryFixture(t)
	f.permit.scope.PlanDigest = hash([]byte("not the recorded plan"))
	for id, grant := range f.snapshot.Grants {
		grant.Scope = f.permit.scope
		f.snapshot.Grants[id] = grant
	}
	for id, instance := range f.snapshot.Instances {
		instance.Scope = f.permit.scope
		f.snapshot.Instances[id] = instance
	}
	registryApproveFixture(t, &f)
	before := registryTrustBytes(t, f.store)
	a, b := registryCounts(t, f.store)
	_, err := f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), trustRequesterToken, f.permit, nil)
	if err == nil || identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("arbitrary plan digest did not produce the expected refusal: ", err)
	}
	registryAssertUnchanged(t, f.store, before, a, b)
}

func TestAgentIdentityPlanRefusesCurrentContractDrift(t *testing.T) {
	f := registryFixture(t)
	cp := registryPublish(t, f)
	w, err := f.store.get(f.permit.scope.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	_, err = f.store.mutate(w.ID, "work.update", "plan-contract-drift", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Objective = "a changed objective"
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	before := registryTrustBytes(t, f.store)
	a, b := registryCounts(t, f.store)
	if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); err == nil || identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("changed recorded contract remained usable: ", err)
	}
	registryAssertUnchanged(t, f.store, before, a, b)
}

func TestAgentIdentityPlanRefusesMissingRecordedPlan(t *testing.T) {
	f := registryFixture(t)
	w, err := f.store.get(f.permit.scope.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	_, err = f.store.mutate(w.ID, "fixture.clear-plan", "plan-absent", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Plans, w.Planning = nil, nil
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	before := registryTrustBytes(t, f.store)
	a, b := registryCounts(t, f.store)
	_, err = f.store.publishAgentIdentityRegistry(registryContext(t), registryRaw(t, f.snapshot), trustRequesterToken, f.permit, nil)
	if err == nil || identityPlanFailure(err).Code != "agent_identity_plan_unavailable" {
		t.Fatal("missing recorded plan did not produce the expected refusal: ", err)
	}
	registryAssertUnchanged(t, f.store, before, a, b)
}
