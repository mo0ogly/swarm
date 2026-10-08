//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func runLimitsWebRequest(h http.Handler, method, path, token string, body []byte) *httptest.ResponseRecorder {
	req := httptest.NewRequest(method, "http://local.test"+path, bytes.NewReader(body))
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: token})
	if method == "POST" {
		req.Header.Set("Origin", "http://local.test")
		req.Header.Set("X-Swarm-CSRF", token)
	}
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	return rr
}

// TestRunLimitsAdminHTTPRoundTrip exercises the HTTP adapter end to end
// (REQ-ADM-01/06): empty state, non-writing preview, apply, history and
// rollback, all through the same Store methods run_limits_admin.go tests
// already cover — this test only checks the HTTP wiring, not the rules.
func TestRunLimitsAdminHTTPRoundTrip(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "run-limits-token")

	// Empty state: no override yet for the project scope.
	rr := runLimitsWebRequest(h, "GET", "/api/v1/run-limits?scope=project", "run-limits-token", nil)
	if rr.Code != 200 {
		t.Fatalf("GET show: %d %s", rr.Code, rr.Body.String())
	}
	var shown map[string]any
	if e := json.Unmarshal(rr.Body.Bytes(), &shown); e != nil {
		t.Fatal(e)
	}
	entry := shown["entry"].(map[string]any)
	if entry["revision"].(float64) != 0 {
		t.Fatalf("expected no prior override, got %v", shown)
	}
	if hist, ok := shown["history"].([]any); ok && len(hist) != 0 {
		t.Fatalf("expected empty history, got %v", hist)
	}

	// Preview must reject an out-of-bounds value without writing anything.
	badPreview := []byte(`{"scope":"project","expected_revision":0,"values":{"max_tool_calls":999999}}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/preview", "run-limits-token", badPreview)
	if rr.Code != 400 {
		t.Fatalf("expected rejection of out-of-bounds preview, got %d %s", rr.Code, rr.Body.String())
	}
	rr = runLimitsWebRequest(h, "GET", "/api/v1/run-limits?scope=project", "run-limits-token", nil)
	json.Unmarshal(rr.Body.Bytes(), &shown)
	if shown["entry"].(map[string]any)["revision"].(float64) != 0 {
		t.Fatal("rejected preview must not have written anything")
	}

	// Valid preview: no write yet, but the proposed values are echoed back.
	goodPreview := []byte(`{"scope":"project","expected_revision":0,"values":{"max_tool_calls":42}}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/preview", "run-limits-token", goodPreview)
	if rr.Code != 200 {
		t.Fatalf("preview: %d %s", rr.Code, rr.Body.String())
	}
	var previewed map[string]any
	json.Unmarshal(rr.Body.Bytes(), &previewed)
	if previewed["applied"] != false || !previewed["revision_ok"].(bool) {
		t.Fatalf("unexpected preview result: %v", previewed)
	}
	rr = runLimitsWebRequest(h, "GET", "/api/v1/run-limits?scope=project", "run-limits-token", nil)
	json.Unmarshal(rr.Body.Bytes(), &shown)
	if shown["entry"].(map[string]any)["revision"].(float64) != 0 {
		t.Fatal("preview must never write")
	}

	// Apply: writes revision 1.
	apply := []byte(`{"schema_version":1,"event_id":"web-apply-1","scope":"project","expected_revision":0,"values":{"max_tool_calls":42},"reason":"abaisser le plafond"}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/apply", "run-limits-token", apply)
	if rr.Code != 200 {
		t.Fatalf("apply: %d %s", rr.Code, rr.Body.String())
	}
	var applied map[string]any
	json.Unmarshal(rr.Body.Bytes(), &applied)
	if applied["revision"].(float64) != 1 || applied["values"].(map[string]any)["max_tool_calls"].(float64) != 42 {
		t.Fatalf("unexpected apply result: %v", applied)
	}

	// A stale expected_revision is refused (optimistic concurrency).
	stale := []byte(`{"schema_version":1,"event_id":"web-apply-stale","scope":"project","expected_revision":0,"values":{"max_tool_calls":10},"reason":"stale"}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/apply", "run-limits-token", stale)
	if rr.Code != 409 {
		t.Fatalf("expected 409 on stale revision, got %d %s", rr.Code, rr.Body.String())
	}

	// Second real change: revision 2.
	apply2 := []byte(`{"schema_version":1,"event_id":"web-apply-2","scope":"project","expected_revision":1,"values":{"max_tool_calls":10},"reason":"resserrer encore"}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/apply", "run-limits-token", apply2)
	if rr.Code != 200 {
		t.Fatalf("apply2: %d %s", rr.Code, rr.Body.String())
	}

	// History now has two entries, most recent first.
	rr = runLimitsWebRequest(h, "GET", "/api/v1/run-limits?scope=project", "run-limits-token", nil)
	json.Unmarshal(rr.Body.Bytes(), &shown)
	hist := shown["history"].([]any)
	if len(hist) != 2 {
		t.Fatalf("expected 2 history entries, got %d: %v", len(hist), hist)
	}

	// Rollback to revision 1 creates revision 3 with revision 1's values.
	rollback := []byte(`{"scope":"project","to_revision":1,"event_id":"web-rollback-1","reason":"retour test","expected_revision":2}`)
	rr = runLimitsWebRequest(h, "POST", "/api/v1/run-limits/rollback", "run-limits-token", rollback)
	if rr.Code != 200 {
		t.Fatalf("rollback: %d %s", rr.Code, rr.Body.String())
	}
	var rolled map[string]any
	json.Unmarshal(rr.Body.Bytes(), &rolled)
	if rolled["revision"].(float64) != 3 || rolled["values"].(map[string]any)["max_tool_calls"].(float64) != 42 {
		t.Fatalf("unexpected rollback result: %v", rolled)
	}

	// Effective resolution reflects the rolled-back value for a hypothetical new attempt.
	rr = runLimitsWebRequest(h, "GET", "/api/v1/run-limits/effective?mission_id=&role=&task=", "run-limits-token", nil)
	if rr.Code != 200 {
		t.Fatalf("effective: %d %s", rr.Code, rr.Body.String())
	}
	var eff map[string]any
	json.Unmarshal(rr.Body.Bytes(), &eff)
	if eff["effective"].(map[string]any)["max_tool_calls"].(float64) != 42 {
		t.Fatalf("unexpected effective result: %v", eff)
	}
}

func TestRunLimitsAdminHTTPRejectsCrossSiteWrite(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "run-limits-token")
	apply := []byte(`{"schema_version":1,"event_id":"csrf","scope":"project","expected_revision":0,"values":{"max_tool_calls":42},"reason":"x"}`)
	req := httptest.NewRequest("POST", "http://local.test/api/v1/run-limits/apply", bytes.NewReader(apply))
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "run-limits-token"})
	req.Header.Set("Origin", "http://evil.test")
	req.Header.Set("X-Swarm-CSRF", "run-limits-token")
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	if rr.Code != 403 {
		t.Fatalf("expected cross-site apply to be rejected, got %d %s", rr.Code, rr.Body.String())
	}
}

// TestRunLimitsFieldBoundsMatchValidator keeps the Administration screen's
// displayed bounds (run_limits_web.go's runLimitsFieldBounds, used only for
// the unit/ceiling hint in the UI) honest against the one real authority,
// validRunLimitsOverride: if that function's bounds ever change without
// updating the display table, this test fails loudly instead of the UI
// silently lying about what will be accepted.
func TestRunLimitsFieldBoundsMatchValidator(t *testing.T) {
	set := func(name string, v int) RunLimits {
		l := RunLimits{}
		switch name {
		case "silence_seconds":
			l.SilenceSeconds = v
		case "tool_seconds":
			l.ToolSeconds = v
		case "max_tool_calls":
			l.MaxToolCalls = v
		case "max_repeated_calls":
			l.MaxRepeatedCalls = v
		case "max_consecutive_errors":
			l.MaxConsecutiveErrors = v
		default:
			t.Fatalf("unknown field %s", name)
		}
		return l
	}
	if len(runLimitsFieldBounds) != 5 {
		t.Fatalf("expected 5 displayed fields, got %d", len(runLimitsFieldBounds))
	}
	for _, f := range runLimitsFieldBounds {
		if e := validRunLimitsOverride(set(f.Name, f.Max)); e != nil {
			t.Fatalf("%s at displayed max %d must be accepted by the real validator: %v", f.Name, f.Max, e)
		}
		if e := validRunLimitsOverride(set(f.Name, f.Max+1)); e == nil {
			t.Fatalf("%s above displayed max %d must be refused by the real validator", f.Name, f.Max)
		}
	}
}
