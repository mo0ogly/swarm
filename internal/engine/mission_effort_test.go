//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestMissionEffortDurationUnknownsAndReadOnlyCLI(t *testing.T) {
	const start = "2026-10-04T08:00:00Z"
	for _, pair := range [][2]string{{"", start}, {start, ""}, {"bad", start}, {start, "2026-10-04T07:59:59Z"}} {
		if recordedProcessDuration(pair[0], pair[1]) != nil {
			t.Fatal("invented duration", pair)
		}
	}
	if v := recordedProcessDuration(start, start); v == nil || *v != 0 {
		t.Fatal("known zero lost", v)
	}
	s := storeTest(t)
	w := createTest(t, s)
	for _, id := range []string{"effort", "unknown"} {
		w = applyTest(t, s, w, "task.add", Request{ID: id, Title: id, Deliverable: "fixture", Criteria: []string{"fixture"}, Owner: "worker", Next: "fixture"})
	}
	// Explicit synthetic review history and provider telemetry in an isolated Store.
	w.Tasks[0].IndependentReview = &IndependentReview{ID: "fixture-current"}
	w.Tasks[0].PreviousReviews = []IndependentReview{{ID: "fixture-old"}}
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	cost := 0.25
	agents := []Agent{
		{ID: "old", WorkID: w.ID, TaskID: "effort", Attempt: "old", CWD: s.root, Status: "interrupted", Started: start, Ended: "2026-10-04T08:02:00Z", Progress: AgentProgress{ToolCalls: 4}},
		{ID: "new", WorkID: w.ID, TaskID: "effort", Attempt: "new", CWD: s.root, Status: "completed", Previous: "old", Started: "2026-10-04T08:01:30Z", Ended: "2026-10-04T08:02:00Z", Progress: AgentProgress{MetricsVersion: 1, ToolCalls: 7, Reads: 3, Writes: 2, Unclassified: 2}, Usage: &Usage{Input: 100, Output: 20, ReportedCost: &cost}},
		{ID: "missing", WorkID: w.ID, TaskID: "unknown", Attempt: "missing", CWD: s.root, Status: "completed", Mode: "terminal"},
	}
	for _, a := range agents {
		body, _ := json.Marshal(a)
		if _, e := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, a.WorkID, a.TaskID, a.CWD, a.Status, body, []byte(`{}`)); e != nil {
			t.Fatal(e)
		}
	}
	spending, e := s.missionSpending(w, agents)
	if e != nil {
		t.Fatal(e)
	}
	by := map[string]SpendingRow{}
	for _, r := range spending.Rows {
		by[r.ID] = r
	}
	known, unknown := by["effort"], by["unknown"]
	if known.DurationMS == nil || *known.DurationMS != 150000 || known.Tools != 11 || known.Reviews != 2 || known.TaskRetries != 1 || known.Cost.Reported != cost || known.Cost.Silent != 1 {
		t.Fatalf("bad task total: %+v", known)
	}
	if unknown.DurationMS != nil || unknown.MissingDurations != 1 || unknown.UnknownTools != 1 || unknown.Cost.WithCost != 0 {
		t.Fatalf("unknown became zero: %+v", unknown)
	}
	if spending.ObservedAt == "" || spending.Source == "" {
		t.Fatal("missing provenance")
	}
	before, _ := s.get(w.ID)
	eventsBefore, _ := s.events(w.ID)
	output := map[string]string{}
	for _, lang := range []string{"fr", "en"} {
		t.Setenv("SWARM_LANG", lang)
		var text bytes.Buffer
		if e = missionCLI(s, []string{"mission", "spending", w.ID}, "", false, &text); e != nil {
			t.Fatal(e)
		}
		if !strings.Contains(text.String(), "150.000 s") || !strings.Contains(text.String(), uiText("  Appels d’outils non rapportés.")) {
			t.Fatal(text.String())
		}
		if lang == "en" && (strings.Contains(text.String(), "USD rapportés") || strings.Contains(text.String(), "sans coût rapporté")) {
			t.Fatal(text.String())
		}
		output[lang] = text.String()
	}
	var cli bytes.Buffer
	if e = missionCLI(s, []string{"mission", "spending", w.ID}, "", true, &cli); e != nil {
		t.Fatal(e)
	}
	var cliData MissionSpending
	if e = json.Unmarshal(cli.Bytes(), &cliData); e != nil {
		t.Fatal(e)
	}
	const host, token = "127.0.0.1:18999", "effort-fixture"
	req := httptest.NewRequest(http.MethodGet, "http://"+host+"/api/v1/snapshot?work="+w.ID, nil)
	req.Host = host
	req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
	response := httptest.NewRecorder()
	newWebHandler(s, host, token).ServeHTTP(response, req)
	var web struct {
		Mission MissionStatus `json:"mission"`
	}
	if response.Code != 200 || json.Unmarshal(response.Body.Bytes(), &web) != nil {
		t.Fatal(response.Body.String())
	}
	x, _ := json.Marshal(cliData.Rows)
	y, _ := json.Marshal(web.Mission.Spending.Rows)
	if !bytes.Equal(x, y) {
		t.Fatal("HTTP CLI diverge", string(x), string(y))
	}
	after, _ := s.get(w.ID)
	eventsAfter, _ := s.events(w.ID)
	if after.Revision != before.Revision || len(eventsAfter) != len(eventsBefore) {
		t.Fatal("read changed history")
	}
	t.Log("isolated engine: effort=150000ms (overlapping process sum, not elapsed mission time), tools=11, reviews=2, retries=1, reported_cost=0.25 USD, one missing cost; unknown task duration=null, tools unreported; CLI FR/EN and HTTP identical; history unchanged")
	if dir := os.Getenv("SWARM_QW7_EVIDENCE"); dir != "" {
		if e = os.MkdirAll(dir, 0700); e != nil {
			t.Fatal(e)
		}
		exportRoot := filepath.Join(dir, "store", ".swarm")
		if e = os.MkdirAll(exportRoot, 0700); e != nil {
			t.Fatal(e)
		}
		exportDB := filepath.Join(exportRoot, "state.db")
		if e = os.Remove(exportDB); e != nil && !os.IsNotExist(e) {
			t.Fatal(e)
		}
		if _, e = s.db.Exec("VACUUM INTO ?", exportDB); e != nil {
			t.Fatal(e)
		}
		if e = os.WriteFile(filepath.Join(dir, "work-id.txt"), []byte(w.ID), 0600); e != nil {
			t.Fatal(e)
		}
		data, _ := json.MarshalIndent(spending, "", "  ")
		if e = os.WriteFile(filepath.Join(dir, "effort-fixture.json"), data, 0600); e != nil {
			t.Fatal(e)
		}
		for lang, text := range output {
			if e = os.WriteFile(filepath.Join(dir, "cli-"+lang+".txt"), []byte(text), 0600); e != nil {
				t.Fatal(e)
			}
		}
	}
}
