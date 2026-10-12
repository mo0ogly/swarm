//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"testing"
	"time"
)

// Two real subprocess calls through the engine; deterministic provider doubles,
// not evidence of two autonomous real-provider supervisors.
func TestCursorEngineT3ConcurrentScopesAndSharedReservations(t *testing.T) {
	for _, pause := range []bool{false, true} {
		name := "complete"
		if pause {
			name = "pause"
		}
		t.Run(name, func(t *testing.T) {
			s := storeTest(t)
			w := applyTest(t, s, Work{}, "work.create", Request{Title: "Concurrent planning", Objective: "Independent scopes share budget", Scope: "isolated fixture", Criteria: []string{"left", "right", "spare"}, Next: "Delegate independently"})
			dir := t.TempDir()
			script := filepath.Join(dir, "claude")
			program := `#!/usr/bin/env python3
import json,sys,pathlib,time
prompt=sys.stdin.read();ctx=None;decoder=json.JSONDecoder()
for i,c in enumerate(prompt):
 if c!='{':continue
 try:
  value,end=decoder.raw_decode(prompt[i:])
  if isinstance(value,dict) and isinstance(value.get('scope'),dict) and 'events' in value and i+end==len(prompt):ctx=value;break
 except (ValueError,TypeError):pass
if ctx is None:raise SystemExit('missing engine context')
root=pathlib.Path(__file__).parent;scope=ctx['scope']['id']
(root/('active-'+scope)).write_text('provider entered')
while not (root/'release').exists():time.sleep(.01)
operation={'kind':'task','id':scope+'-task','title':'Scoped discovered task','requirements':ctx['scope']['requirements'],'deliverable':'proof.txt','criteria':['scoped proof'],'depends':[],'next':'Verify the scoped behavior'}
reply={'input_events':[e['id'] for e in ctx['events']],'reason':'Independent scope discovery persisted','operations':[operation]}
print(json.dumps({'type':'result','result':json.dumps(reply)}))
`
			if err := os.WriteFile(script, []byte(program), 0700); err != nil {
				t.Fatal(err)
			}
			raw, _ := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": {Command: script}}})
			if err := os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			w = planningDo(t, s, w, "enable", PlanningRequest{Provider: "fixture", MaxTasks: 10, MaxDecisions: 10, MaxActivations: 3})
			w, r := planningClaim(t, s, w, "root")
			r.Operations = []PlanningOperation{{Kind: "delegate", ID: "left", Title: "Left", Requirements: []string{"req-1"}, MaxActivations: 1}, {Kind: "delegate", ID: "right", Title: "Right", Requirements: []string{"req-2"}, MaxActivations: 1}, {Kind: "delegate", ID: "spare", Title: "Spare", Requirements: []string{"req-3"}, MaxActivations: 1}}
			w, err := s.planningChange(w.ID, "decide", r)
			if err != nil {
				t.Fatal(err)
			}
			result := make(chan error, 2)
			defer os.WriteFile(filepath.Join(dir, "release"), []byte("cleanup"), 0600)
			waitEntry := func(scope string) {
				t.Helper()
				deadline := time.Now().Add(8 * time.Second)
				for time.Now().Before(deadline) {
					if _, err := os.Stat(filepath.Join(dir, "active-"+scope)); err == nil {
						return
					}
					time.Sleep(10 * time.Millisecond)
				}
				t.Fatal("provider never entered", scope)
			}
			go func() { result <- s.planningStep(w.ID) }()
			waitEntry("left")
			go func() { result <- s.planningStep(w.ID) }()
			waitEntry("right")
			active, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			left, _ := active.Planning.scope("left")
			right, _ := active.Planning.scope("right")
			root, _ := active.Planning.scope("root")
			if left.Holder == "" || right.Holder == "" || left.Holder == right.Holder || left.Activations != 1 || right.Activations != 1 || root.Activations != 3 || active.Planning.Activations != 3 {
				t.Fatal("two distinct active reservations or own/parent accounting missing")
			}
			var calls int
			if err = s.db.QueryRow("SELECT count(*) FROM planning_calls WHERE work_id=?", w.ID).Scan(&calls); err != nil || calls != 3 {
				t.Fatal("shared call reservation mismatch", calls, err)
			}
			spare, _ := active.Planning.scope("spare")
			if _, err = s.planningChange(w.ID, "claim", PlanningRequest{Schema: 1, EventID: "spare-over-budget", Revision: active.Revision, Scope: spare.ID, ScopeRevision: spare.Revision, Holder: "third-owner", LeaseSeconds: 60}); err == nil {
				t.Fatal("shared activation budget bypassed")
			}
			unchanged, _ := s.get(w.ID)
			if !reflect.DeepEqual(unchanged, active) {
				t.Fatal("refused reservation changed work")
			}
			if pause {
				active = planningDo(t, s, active, "pause", PlanningRequest{})
			} else {
				if err = os.WriteFile(filepath.Join(dir, "release"), []byte("finish both"), 0600); err != nil {
					t.Fatal(err)
				}
			}
			for i := 0; i < 2; i++ {
				select {
				case err := <-result:
					if !pause && err != nil {
						t.Fatal(err)
					}
				case <-time.After(8 * time.Second):
					t.Fatal("provider call did not settle")
				}
			}
			after, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			if after.Planning.Activations != 3 {
				t.Fatal("settlement refunded or added an activation")
			}
			if pause {
				if len(after.Tasks) != 0 || !after.Planning.Paused {
					t.Fatal("revoked supervisor decision was applied")
				}
				after = planningDo(t, s, after, "resume", PlanningRequest{})
			} else {
				if len(after.Tasks) != 2 {
					t.Fatal("concurrent decisions lost or duplicated tasks", len(after.Tasks))
				}
				for _, id := range []string{"left", "right"} {
					task, e := after.task(id + "-task")
					if e != nil || task.ScopeID != id {
						t.Fatal("decision escaped its scope", id, e)
					}
				}
			}
			if err = s.planningStep(w.ID); err != nil {
				t.Fatal(err)
			}
			replay, _ := s.get(w.ID)
			if !reflect.DeepEqual(after, replay) {
				t.Fatal("unchanged replay spent another call or changed tasks")
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			durable, err := reopened.get(w.ID)
			if err != nil || !reflect.DeepEqual(durable, replay) {
				t.Fatal("reservations/tasks lost across reopen", err)
			}
		})
	}
}
