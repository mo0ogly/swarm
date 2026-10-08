//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"os"
	"path/filepath"
	"sync"
	"testing"
)

func performanceRequest() GraphPerformanceChange {
	return GraphPerformanceChange{Schema: 1, EventID: "targets-1", Revision: 0, Reason: "Keyboard target for isolated test", Values: GraphPerformanceValues{KeyboardP95: 137, Samples: 7, Loads: []GraphPerformanceLoad{{Cards: 50, RenderP95: 1234}, {Cards: 200, RenderP95: 1500}}}}
}
func TestGraphPerformancePersistencePreviewReplayAndConflict(t *testing.T) {
	s := storeTest(t)
	original, err := s.graphPerformanceConfig()
	if err != nil {
		t.Fatal(err)
	}
	var defaults GraphPerformanceValues
	if err = strict(graphPerformanceDefaults, &defaults); err != nil {
		t.Fatal(err)
	}
	if original.Values.KeyboardP95 != defaults.KeyboardP95 || original.Revision != 0 {
		t.Fatal(original)
	}
	req := performanceRequest()
	proposed, err := s.changeGraphPerformance(req, true)
	if err != nil || proposed.Revision != 1 {
		t.Fatal(proposed, err)
	}
	if _, err = os.Stat(filepath.Join(s.root, ".swarm", "graph-performance.json")); !os.IsNotExist(err) {
		t.Fatal("preview wrote configuration", err)
	}
	saved, err := s.changeGraphPerformance(req, false)
	if err != nil {
		t.Fatal(err)
	}
	again, err := s.changeGraphPerformance(req, false)
	if err != nil || len(again.History) != 1 {
		t.Fatal("replay", again, err)
	}
	req.Values.KeyboardP95++
	if _, err = s.changeGraphPerformance(req, false); err == nil {
		t.Fatal("conflicting event accepted")
	}
	req.EventID = "stale"
	if _, err = s.changeGraphPerformance(req, false); err == nil {
		t.Fatal("stale revision accepted")
	}
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	restored, err := other.graphPerformanceConfig()
	if err != nil || restored.Values.KeyboardP95 != saved.Values.KeyboardP95 || len(restored.History) != 1 {
		t.Fatal(restored, err)
	}
	req = performanceRequest()
	req.EventID = "rollback"
	req.Revision = 1
	req.Values = defaults
	rolled, err := other.changeGraphPerformance(req, false)
	if err != nil || len(rolled.History) != 2 || rolled.Values.KeyboardP95 != defaults.KeyboardP95 {
		t.Fatal(rolled, err)
	}
	if saved.Values.KeyboardP95 != 137 {
		t.Fatal("frozen snapshot changed")
	}
}
func TestGraphPerformanceConcurrentInvalidAndSymlink(t *testing.T) {
	s := storeTest(t)
	var wg sync.WaitGroup
	results := make(chan error, 2)
	for _, id := range []string{"one", "two"} {
		wg.Add(1)
		go func(id string) {
			defer wg.Done()
			r := performanceRequest()
			r.EventID = id
			_, err := s.changeGraphPerformance(r, false)
			results <- err
		}(id)
	}
	wg.Wait()
	close(results)
	success := 0
	for err := range results {
		if err == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatalf("concurrent success=%d", success)
	}
	c, err := s.graphPerformanceConfig()
	if err != nil || len(c.History) != 1 {
		t.Fatal(c, err)
	}
	invalid := []GraphPerformanceValues{{KeyboardP95: 0, Samples: 5, Loads: []GraphPerformanceLoad{{50, 1000}}}, {KeyboardP95: 100, Samples: 0, Loads: []GraphPerformanceLoad{{50, 1000}}}, {KeyboardP95: 100, Samples: 5, Loads: []GraphPerformanceLoad{{50, 1000}, {50, 2000}}}, {KeyboardP95: 100, Samples: 5, Loads: nil}}
	for _, v := range invalid {
		r := performanceRequest()
		r.EventID = "invalid"
		r.Revision = 1
		r.Values = v
		if _, err := s.changeGraphPerformance(r, false); err == nil {
			t.Fatal("invalid accepted", v)
		}
	}
	path := filepath.Join(s.root, ".swarm", "graph-performance.json")
	if err = os.Remove(path); err != nil {
		t.Fatal(err)
	}
	outside := filepath.Join(t.TempDir(), "outside")
	if err = os.WriteFile(outside, []byte(`{}`), 0600); err != nil {
		t.Fatal(err)
	}
	if err = os.Symlink(outside, path); err != nil {
		t.Fatal(err)
	}
	if _, err = s.graphPerformanceConfig(); err == nil {
		t.Fatal("symlink read")
	}
	if _, err = s.changeGraphPerformance(performanceRequest(), false); err == nil {
		t.Fatal("symlink write")
	}
}
func TestGraphPerformanceCLIHTTPParityAuthAndPreview(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "performance-token")
	if rr := runLimitsWebRequest(h, "GET", "/api/v1/graph-performance", "", nil); rr.Code == 200 {
		t.Fatal("unauthenticated GET")
	}
	req := performanceRequest()
	raw, _ := json.Marshal(req)
	rr := runLimitsWebRequest(h, "POST", "/api/v1/graph-performance/preview", "performance-token", raw)
	if rr.Code != 200 {
		t.Fatal(rr.Code, rr.Body.String())
	}
	c, _ := s.graphPerformanceConfig()
	if c.Revision != 0 {
		t.Fatal("HTTP preview wrote")
	}
	rr = runLimitsWebRequest(h, "POST", "/api/v1/graph-performance/apply", "performance-token", raw)
	if rr.Code != 200 {
		t.Fatal(rr.Code, rr.Body.String())
	}
	var out bytes.Buffer
	if err := s.runLimitsCLI([]string{"run-limits", "performance", "show"}, "", &out); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(out.Bytes(), &c); err != nil || c.Values.KeyboardP95 != 137 {
		t.Fatal(c, err)
	}
	req.EventID = "http-stale"
	raw, _ = json.Marshal(req)
	rr = runLimitsWebRequest(h, http.MethodPost, "/api/v1/graph-performance/apply", "performance-token", raw)
	if rr.Code != 409 {
		t.Fatal(rr.Code, rr.Body.String())
	}
	raw = []byte(`{"unknown":true}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/graph-performance/apply", "performance-token", raw)
	if rr.Code != 400 {
		t.Fatal(rr.Code, rr.Body.String())
	}
}
