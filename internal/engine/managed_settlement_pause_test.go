//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"reflect"
	"testing"
)

// The public retry may queue a retained result while paused. Reconciliation
// must keep that intent, rather than turn a deliberate pause into a new failure.
// The subprocess reviewer is a fixture; this does not prove real model autonomy.
func TestManagedSettlementRetainsPublicRetryUntilBothPausesResume(t *testing.T) {
	for _, mode := range []string{"planning", "mission", "both"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a := unpaidReviewFixture(t)
			item, err := s.managedAttempt(a.ID)
			if err != nil {
				t.Fatal(err)
			}
			w, err = s.mutate(w.ID, "test.recovered", "fixture-retained-result", w.Revision, []byte(`{}`), func(current *Work) error {
				task, e := current.task(a.TaskID)
				if e != nil {
					return e
				}
				task.RecoveredResult = &RecoveredResult{Agent: a.ID, Attempt: a.Attempt, Result: item.Result, ProcessStatus: a.Status}
				return nil
			})
			if err != nil {
				t.Fatal(err)
			}
			mux := http.NewServeMux()
			s.registerPlanning(mux, func(out http.ResponseWriter, v any) { json.NewEncoder(out).Encode(v) }, func(out http.ResponseWriter, e error) { http.Error(out, e.Error(), 400) })
			change := func(action, event string) Work {
				t.Helper()
				current, e := s.get(w.ID)
				if e != nil {
					t.Fatal(e)
				}
				req := PlanningRequest{Schema: 1, EventID: event, Revision: current.Revision, Task: a.TaskID, Reason: "Retain existing result and resume only after both pauses are cleared."}
				raw, e := json.Marshal(req)
				if e != nil {
					t.Fatal(e)
				}
				out := httptest.NewRecorder()
				mux.ServeHTTP(out, httptest.NewRequest("POST", "/api/v1/planning?work="+w.ID+"&action="+action, bytes.NewReader(raw)))
				if out.Code != http.StatusOK {
					t.Fatalf("%s: %d %s", action, out.Code, out.Body.String())
				}
				var result Work
				if e = json.Unmarshal(out.Body.Bytes(), &result); e != nil {
					t.Fatal(e)
				}
				return result
			}
			if mode != "mission" {
				w = change("pause", "pause-planning")
			}
			if mode != "planning" {
				if err = s.pause(w.ID, true); err != nil {
					t.Fatal(err)
				}
			}
			armed := change("retry-review", "public-retry-while-paused")
			before, err := s.managedAttempt(a.ID)
			if err != nil || before.State != "integrating" {
				t.Fatal(before, err)
			}
			for i := 0; i < 2; i++ {
				if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil {
					t.Fatal("pause became settlement error", err)
				}
			}
			current, err := s.get(w.ID)
			after, itemErr := s.managedAttempt(a.ID)
			if err != nil || itemErr != nil || !reflect.DeepEqual(armed, current) || !reflect.DeepEqual(before, after) || managedReviewCalls(t, s) != 0 {
				t.Fatalf("paused intent changed or spent a call: work=%v item=%v calls=%d errors=%v/%v", reflect.DeepEqual(armed, current), reflect.DeepEqual(before, after), managedReviewCalls(t, s), err, itemErr)
			}
			if mode == "both" {
				if err = s.pause(w.ID, false); err != nil {
					t.Fatal(err)
				}
				if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil {
					t.Fatal(err)
				}
				current, _ = s.get(w.ID)
				after, _ = s.managedAttempt(a.ID)
				if !reflect.DeepEqual(armed, current) || !reflect.DeepEqual(before, after) || managedReviewCalls(t, s) != 0 {
					t.Fatal("clearing just one pause executed review")
				}
			}
			if mode != "mission" {
				w = change("resume", "resume-planning")
			}
			if mode == "mission" {
				if err = s.pause(w.ID, false); err != nil {
					t.Fatal(err)
				}
			}
			if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil {
				t.Fatal(err)
			}
			current, err = s.get(w.ID)
			task, taskErr := current.task(a.TaskID)
			if err != nil || taskErr != nil || task.Status != "accepted" || task.IndependentReview == nil || managedReviewCalls(t, s) != 1 || len(task.Attempts) != 1 {
				t.Fatalf("retained result did not resume: task=%+v errors=%v/%v", task, err, taskErr)
			}
			if err = s.reconcileKnownMissionResult(a, "fixture-conductor"); err != nil || managedReviewCalls(t, s) != 1 {
				t.Fatal("repeat settlement spent another call", err)
			}
		})
	}
}
