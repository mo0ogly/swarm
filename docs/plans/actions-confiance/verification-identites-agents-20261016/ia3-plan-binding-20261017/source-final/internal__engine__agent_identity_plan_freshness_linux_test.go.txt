//go:build linux

package engine

import (
	"context"
	"os"
	"path/filepath"
	goruntime "runtime"
	"strings"
	"testing"
	"time"
)

func TestAgentIdentityPlanChangeDuringRuntimeWait(t *testing.T) {
	f := registryFixture(t)
	cp := registryPublish(t, f)
	w := identityPlanWork(t, f)
	other, err := openStore(f.store.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	binding, err := newAgentIdentityRuntime(f.store.root)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { binding.Close() })
	dir, err := os.MkdirTemp("", "swarm-plan-freshness-")
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { os.RemoveAll(dir) })
	if err = os.WriteFile(filepath.Join(dir, "worker.py"), []byte(agentIdentityChannelWorker), 0600); err != nil {
		t.Fatal(err)
	}
	listener := agentIdentityTestListener(t, filepath.Join(dir, "broker.sock"))
	control := agentIdentityTestListener(t, filepath.Join(dir, "control.sock"))
	instance := f.snapshot.Instances[f.actor.InstanceID]
	cage := startAgentIdentityTestCage(t, binding, f.actor, instance.BindingID, f.permit.trustRevision, dir, listener, control)
	cage.command(t, "send", f.request)
	message, err := receiveAgentIdentityMessage(cage.connection, make([]byte, 65536))
	if err != nil {
		t.Fatal(err)
	}
	defer message.Close()
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	binding.mu.Lock()
	locked := true
	defer func() {
		if locked {
			binding.mu.Unlock()
		}
	}()
	type outcome struct {
		decision agentIdentityDecision
		err      error
	}
	decided := make(chan outcome, 1)
	go func() {
		d, err := f.store.evaluateAgentIdentityRegistryMessage(ctx, cp, message, binding, nil, time.Time{})
		decided <- outcome{d, err}
	}()
	// A bounded local Go-runtime observation establishes that source reading
	// reached the runtime lock. No new production observer or test-only hook.
	stack := make([]byte, 1<<20)
	for !strings.Contains(string(stack[:goruntime.Stack(stack, true)]), "(*agentIdentityRuntime).withCaller") {
		if err = ctx.Err(); err != nil {
			t.Fatal("decision did not reach the held runtime lock: ", err)
		}
		time.Sleep(time.Millisecond)
	}
	written := make(chan error, 1)
	go func() {
		_, err := other.mutate(w.ID, "work.update", "plan-during-runtime-wait", w.Revision, []byte(`{}`), func(w *Work) error {
			w.Objective += " changed during runtime wait"
			return nil
		})
		written <- err
	}()
	committedBeforeUnlock := false
	select {
	case err = <-written:
		if err != nil {
			t.Fatal(err)
		}
		committedBeforeUnlock = true
	case <-time.After(100 * time.Millisecond):
		// A reserved SQLite writer must keep this mutation pending until the
		// decision is made. Its eventual commit/effect are asserted below.
	}
	binding.mu.Unlock()
	locked = false
	result := <-decided
	if !committedBeforeUnlock {
		if err = <-written; err != nil {
			t.Fatal(err)
		}
	}
	if _, err = cage.connection.Write([]byte{1}); err != nil {
		t.Fatal(err)
	}
	cage.completed(t)
	t.Logf("plan_committed_before_runtime_unlock=%t decision_code=%s", committedBeforeUnlock, identityPlanFailure(result.err).Code)
	if committedBeforeUnlock && identityPlanFailure(result.err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("decision used the old plan after its source changed during runtime wait: ", result.err)
	}
	if !committedBeforeUnlock && (result.err != nil || result.decision.Actor != f.actor) {
		t.Fatal("serialized decision was not attributed/allowed before the plan change: ", result.err)
	}
	if _, _, err = f.store.readAgentIdentityRegistry(registryContext(t), cp); identityPlanFailure(err).Code != "agent_identity_plan_mismatch" {
		t.Fatal("committed amendment had no observed authority effect: ", err)
	}
}
