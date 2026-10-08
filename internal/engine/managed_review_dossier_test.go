//go:build linux

package engine

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

func TestReviewDossierPublicExportNoBudgetOrMutation(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	_, err := s.mutate(w.ID, "test.budget", "no-budget", w.Revision, []byte(`{}`), func(w *Work) error { w.Planning.Reviewer.MaxCalls = w.Planning.Reviewer.Calls; return nil })
	if err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	attempt, _ := s.managedAttempt(a.ID)
	expected, err := s.readManagedPreflightEvidence(w.ID, a.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	input := filepath.Join(t.TempDir(), "input.json")
	if err = os.WriteFile(input, []byte(`{"task_id":"`+a.TaskID+`"}`), 0600); err != nil {
		t.Fatal(err)
	}
	var output bytes.Buffer
	if err = s.planningCLI([]string{"planning", "review-dossier", w.ID}, input, &output); err != nil {
		t.Fatal(err)
	}
	var got ManagedReviewDossier
	if err = json.Unmarshal(output.Bytes(), &got); err != nil {
		t.Fatal(err)
	}
	if got.Work != w.ID || got.Task != a.TaskID || got.Revision != before.Revision || got.Version != 1 || !sameDossierJSON(got.Context, expected.Context) {
		t.Fatal("export lost canonical evidence")
	}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(w http.ResponseWriter, v any) { json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
	response := httptest.NewRecorder()
	mux.ServeHTTP(response, httptest.NewRequest("GET", "/api/v1/planning?work="+w.ID+"&task="+a.TaskID+"&action=review-dossier", nil))
	var api ManagedReviewDossier
	if response.Code != 200 || json.Unmarshal(response.Body.Bytes(), &api) != nil || !sameDossierJSON(got, api) {
		t.Fatal("API differs", response.Code)
	}
	if _, err = s.managedReviewDossier(w.ID, "absent"); err == nil {
		t.Fatal("unknown task exported")
	}
	after, _ := s.get(w.ID)
	afterAttempt, _ := s.managedAttempt(a.ID)
	if !reflect.DeepEqual(before, after) || !reflect.DeepEqual(attempt, afterAttempt) || managedReviewCalls(t, s) != 0 {
		t.Fatal("export mutated state or spent a call")
	}
}

// CLI pretty-printing may indent nested json.RawMessage values. Compare the
// canonical JSON evidence, not its presentation whitespace.
func sameDossierJSON(a, b any) bool {
	ar, ae := json.Marshal(a)
	br, be := json.Marshal(b)
	return ae == nil && be == nil && bytes.Equal(ar, br)
}
