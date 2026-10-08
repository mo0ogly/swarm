//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

func graphDraftHTTPRequest(t *testing.T, handler http.Handler, method, path string, value any) *httptest.ResponseRecorder {
	t.Helper()
	var body bytes.Buffer
	if value != nil {
		if err := json.NewEncoder(&body).Encode(value); err != nil {
			t.Fatal(err)
		}
	}
	req := httptest.NewRequest(method, "http://local.test"+path, &body)
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "graph-token"})
	if method != http.MethodGet {
		req.Header.Set("Origin", "http://local.test")
		req.Header.Set("X-Swarm-CSRF", "graph-token")
	}
	out := httptest.NewRecorder()
	handler.ServeHTTP(out, req)
	return out
}

func TestGraphDraftB01HTTPPreviewApply(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	handler := newWebHandler(s, "local.test", "graph-token")
	save := GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}}
	response := graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts", save)
	if response.Code != http.StatusOK {
		t.Fatalf("save %d %s", response.Code, response.Body.String())
	}
	var draft GraphDraft
	if err := json.Unmarshal(response.Body.Bytes(), &draft); err != nil {
		t.Fatal(err)
	}
	response = graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/preview", GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: draft.ID, ExpectedRevision: w.Revision})
	if response.Code != http.StatusOK {
		t.Fatalf("preview %d %s", response.Code, response.Body.String())
	}
	var preview GraphDraftPreview
	if err := json.Unmarshal(response.Body.Bytes(), &preview); err != nil {
		t.Fatal(err)
	}
	if !preview.NoImplicitLaunch || preview.RequiredRight != "apply_plan" {
		t.Fatalf("contrat preview incomplet: %#v", preview)
	}
	response = graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/apply", GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: draft.ID, EventID: "http-apply", ExpectedRevision: w.Revision, PreviewToken: preview.PreviewToken, ContentDigest: preview.ContentDigest})
	if response.Code != http.StatusOK {
		t.Fatalf("apply %d %s", response.Code, response.Body.String())
	}
	current, _ := s.get(w.ID)
	if !taskDependsOn(t, current, "t2", "t1") {
		t.Fatal("route HTTP sans effet métier")
	}
	var agents int
	if err := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
		t.Fatalf("route HTTP a lancé un agent: %d %v", agents, err)
	}
	response = graphDraftHTTPRequest(t, handler, http.MethodGet, "/api/v1/graph-drafts?work="+w.ID+"&draft="+draft.ID, nil)
	if response.Code != http.StatusOK {
		t.Fatalf("read %d %s", response.Code, response.Body.String())
	}
}

func TestGraphDraftB01HTTPGuardsAndStatus(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	handler := newWebHandler(s, "local.test", "graph-token")
	d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	request := GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "http-conflict", ExpectedRevision: w.Revision + 1, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest}
	response := graphDraftHTTPRequest(t, handler, http.MethodPost, "/api/v1/graph-drafts/apply", request)
	if response.Code != http.StatusConflict {
		t.Fatalf("conflit HTTP=%d body=%s", response.Code, response.Body.String())
	}
	var failure struct {
		Failure CommandError `json:"failure"`
	}
	if err := json.Unmarshal(response.Body.Bytes(), &failure); err != nil || failure.Failure.Code != "revision_conflict" {
		t.Fatalf("code métier absent: %#v %v", failure, err)
	}
	if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), false, false, nil); err != nil {
		t.Fatal(err)
	}
	response = graphDraftHTTPRequest(t, handler, http.MethodGet, "/api/v1/graph-drafts?work="+w.ID+"&draft="+d.ID, nil)
	if response.Code != http.StatusForbidden {
		t.Fatalf("droit HTTP=%d body=%s", response.Code, response.Body.String())
	}
}
