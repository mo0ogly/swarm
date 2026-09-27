//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestReviewCostAvailableWithoutBudgetAndWithoutMutation(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	_, err := s.mutate(w.ID, "test.budget", "budget", w.Revision, []byte(`{}`), func(w *Work) error { w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls; return nil })
	if err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	item, _ := s.managedAttempt(a.ID)
	p := filepath.Join(t.TempDir(), "request.json")
	os.WriteFile(p, []byte(`{"task_id":"`+a.TaskID+`"}`), 0600)
	var out bytes.Buffer
	if err = s.planningCLI([]string{"planning", "review-cost", w.ID}, p, &out); err != nil {
		t.Fatal(err)
	}
	var v ManagedReviewCost
	if err = json.Unmarshal(out.Bytes(), &v); err != nil {
		t.Fatal(err)
	}
	if v.FitsBudget || v.CallsAvailable != 0 || v.CallsRequired != v.Inspections+2 || v.Reusable != 0 || v.ReuseReason == "" {
		t.Fatal(v)
	}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(w http.ResponseWriter, v any) { json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, httptest.NewRequest("GET", "/api/v1/planning?work="+w.ID+"&task="+a.TaskID+"&action=review-cost", nil))
	var httpV ManagedReviewCost
	json.Unmarshal(rec.Body.Bytes(), &httpV)
	if rec.Code != 200 || !reflect.DeepEqual(v, httpV) {
		t.Fatal(rec.Code, rec.Body.String())
	}
	after, _ := s.get(w.ID)
	afterItem, _ := s.managedAttempt(a.ID)
	if !reflect.DeepEqual(before, after) || !reflect.DeepEqual(item, afterItem) || managedReviewCalls(t, s) != 0 {
		t.Fatal("estimate mutated mission or spent a call")
	}
}
