//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"io"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestWebAuthenticationOriginAndSharedGuard(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	_, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	h := newWebHandler(s, "local.test", "secret-test-capability")
	request := func(method, path, origin, csrf, host string, auth bool, body []byte) *httptest.ResponseRecorder {
		req := httptest.NewRequest(method, "http://local.test"+path, bytes.NewReader(body))
		req.Host = host
		req.Header.Set("Origin", origin)
		req.Header.Set("X-Swarm-CSRF", csrf)
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret-test-capability"})
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		return rr
	}
	if r := request("GET", "/api/v1/works", "", "", "local.test", false, nil); r.Code != 403 {
		t.Fatal("unauthenticated access", r.Code)
	}
	if r := request("GET", "/api/v1/works", "", "", "evil.test", true, nil); r.Code != 403 {
		t.Fatal("DNS rebinding permitted")
	}
	current, _ := s.get(w.ID)
	raw, _ := json.Marshal(webRequest{Kind: "task", Work: w.ID, Task: "t1", Event: newID("test-"), Revision: current.Revision, Request: Request{Status: "todo"}})
	for _, origin := range []string{"", "http://evil.test"} {
		if r := request("POST", "/api/v1/action", origin, "secret-test-capability", "local.test", true, raw); r.Code != 403 {
			t.Fatal("cross-site mutation", r.Code)
		}
	}
	if r := request("POST", "/api/v1/action", "http://local.test", "wrong", "local.test", true, raw); r.Code != 403 {
		t.Fatal("missing CSRF rejected")
	}
	rr := request("POST", "/api/v1/action", "http://local.test", "secret-test-capability", "local.test", true, raw)
	if rr.Code != 400 || !strings.Contains(rr.Body.String(), "active_agent") {
		t.Fatal("web bypasses shared guard", rr.Code, rr.Body.String())
	}
	if rr.Header().Get("Content-Security-Policy") == "" {
		t.Fatal("missing CSP")
	}
}
func TestWebTaskExposesOracleActions(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	h := newWebHandler(s, "local.test", "secret-test-capability")
	req := httptest.NewRequest("GET", "http://local.test/api/v1/task?work="+w.ID+"&task=t1", nil)
	req.Host = "local.test"
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret-test-capability"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, req)
	if rr.Code != 200 {
		t.Fatal(rr.Code, rr.Body.String())
	}
	var out struct {
		Actions []TaskAction `json:"actions"`
		Task    Task         `json:"task"`
	}
	if e := json.Unmarshal(rr.Body.Bytes(), &out); e != nil {
		t.Fatal(e)
	}
	if len(out.Actions) == 0 {
		t.Fatal("aucune action exposée par /api/v1/task")
	}
	byKind := map[string]TaskAction{}
	for _, a := range out.Actions {
		byKind[a.Kind] = a
		if a.Label == "" {
			t.Fatalf("action %s sans libellé", a.Kind)
		}
		if !a.Disponible && a.Raison == "" {
			t.Fatalf("action %s indisponible sans motif", a.Kind)
		}
	}
	advised := 0
	for _, a := range out.Actions {
		if a.Conseillee && a.Disponible {
			advised++
		}
	}
	if advised != 1 {
		t.Fatalf("attendu exactement une conseillée disponible, %d", advised)
	}
	start := byKind["start"]
	if len(start.Champs) == 0 {
		t.Fatal("start sans champs explicites")
	}
	for _, c := range start.Champs {
		if c.Label == "" || c.Aide == "" {
			t.Fatalf("champ %s sans libellé ou aide", c.Name)
		}
	}
	// Snapshot : la carte task_actions expose le même oracle par tâche.
	req2 := httptest.NewRequest("GET", "http://local.test/api/v1/snapshot?work="+w.ID, nil)
	req2.Host = "local.test"
	req2.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret-test-capability"})
	rr2 := httptest.NewRecorder()
	h.ServeHTTP(rr2, req2)
	if rr2.Code != 200 {
		t.Fatal(rr2.Code, rr2.Body.String())
	}
	var snap struct {
		TaskActions map[string][]TaskAction `json:"task_actions"`
	}
	if e := json.Unmarshal(rr2.Body.Bytes(), &snap); e != nil {
		t.Fatal(e)
	}
	if len(snap.TaskActions["t1"]) != len(out.Actions) {
		t.Fatalf("snapshot %d actions contre %d en détail de tâche", len(snap.TaskActions["t1"]), len(out.Actions))
	}
}

func TestWebReportCannotEscapeDocumentation(t *testing.T) {
	s := storeTest(t)
	os.Mkdir(filepath.Join(s.root, "docs"), 0700)
	os.WriteFile(filepath.Join(s.root, "private.txt"), []byte("private"), 0600)
	os.Symlink("../private.txt", filepath.Join(s.root, "docs/link.md"))
	h := newWebHandler(s, "local.test", "token")
	r := httptest.NewRequest("GET", "http://local.test/api/v1/report?path=docs/link.md", nil)
	r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, r)
	if rr.Code != 403 || strings.Contains(rr.Body.String(), "private") {
		t.Fatal("report leaked outside docs", rr.Code)
	}
}
func TestWebEventCursorReconnect(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Exercise the actual HTTP stream and Last-Event-ID, without issuing mutations.
	var host string
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(rw http.ResponseWriter, r *http.Request) { newWebHandler(s, host, "token").ServeHTTP(rw, r) }))
	server.Start()
	defer server.Close()
	host = strings.TrimPrefix(server.URL, "http://")
	req, _ := http.NewRequest("GET", server.URL+"/api/v1/events?work="+w.ID, nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
	req.Header.Set("Last-Event-ID", "1")
	server.Client().Timeout = 3 * time.Second
	resp, e := server.Client().Do(req)
	if e != nil {
		t.Fatal(e)
	}
	defer resp.Body.Close()
	buf, e := io.ReadAll(resp.Body)
	n := len(buf)
	if e != nil && e != io.EOF {
		t.Fatal(e)
	}
	if !strings.Contains(string(buf), "retry: 2000\n") {
		t.Fatal("missing reconnect backoff")
	}
	if strings.Contains(string(buf[:n]), "id: 1\n") || !strings.Contains(string(buf[:n]), "id: 2\n") {
		t.Fatal("cursor replay", string(buf[:n]))
	}
}

// Le fil passe par le transport HTTP comme le reste du cockpit : mêmes gardes
// d'authentification, et les paramètres de requête doivent réellement atteindre
// l'oracle — un endpoint qui ignore « decisions » rendrait un fil complet là où
// l'opérateur a demandé les seuls arbitrages.
func TestActivityEndpointAppliesQueryAndGuards(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.controlEvent(w.ID, "dispatch", "t1 : départ automatique"); e != nil {
		t.Fatal(e)
	}
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	h := newWebHandler(s, "local.test", "secret-test-capability")
	get := func(query string, auth bool) *httptest.ResponseRecorder {
		req := httptest.NewRequest("GET", "http://local.test/api/v1/activity?work="+w.ID+query, nil)
		req.Host = "local.test"
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "secret-test-capability"})
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		return rr
	}
	if rr := get("", false); rr.Code != 403 {
		t.Fatalf("le fil doit rester derrière l'authentification, code %d", rr.Code)
	}

	decode := func(rr *httptest.ResponseRecorder) ActivityPage {
		t.Helper()
		if rr.Code != 200 {
			t.Fatalf("code %d : %s", rr.Code, rr.Body.String())
		}
		var page ActivityPage
		if e := json.Unmarshal(rr.Body.Bytes(), &page); e != nil {
			t.Fatalf("réponse illisible : %v — %s", e, rr.Body.String())
		}
		return page
	}

	complet := decode(get("", true))
	moteur := false
	for _, x := range complet.Entries {
		if x.Origin == activityEngine {
			moteur = true
		}
	}
	if !moteur {
		t.Fatalf("le fil complet doit contenir les actions du moteur : %+v", complet.Entries)
	}

	// Le filtre doit être transmis, pas seulement accepté.
	filtre := decode(get("&decisions=1", true))
	for _, x := range filtre.Entries {
		if x.Origin != activityHuman {
			t.Fatalf("decisions=1 ignoré : entrée %q rendue", x.Kind)
		}
	}
	if len(filtre.Entries) >= len(complet.Entries) {
		t.Fatalf("le filtre n'a rien retiré : %d entrées filtrées pour %d au total",
			len(filtre.Entries), len(complet.Entries))
	}

	// La limite aussi, sans quoi la pagination du cockpit serait inopérante.
	borne := decode(get("&limit=1", true))
	if len(borne.Entries) != 1 {
		t.Fatalf("limit=1 ignoré : %d entrées rendues", len(borne.Entries))
	}
	if !borne.More || borne.Next == "" {
		t.Fatalf("une suite existe : More et Next attendus, obtenu %+v", borne)
	}
	suivante := decode(get("&limit=1&before="+url.QueryEscape(borne.Next), true))
	if len(suivante.Entries) != 1 {
		t.Fatalf("page suivante vide : le curseur before n'a pas été transmis")
	}
	if suivante.Entries[0].cursor() == borne.Entries[0].cursor() {
		t.Fatal("la page suivante rend la même entrée : curseur ignoré")
	}
}

// Trois actions d'écriture du cockpit — profil, autonomie, ordonnancement —
// sont arrivées par la porte web sans test ni mention dans l'inventaire. La
// dernière lance des agents : c'est la seule du lot qui engage une dépense.
// Ce test emprunte la porte réelle et vérifie les bornes, pas les libellés.
func TestWebConduiteActionsRespectTheirLimits(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)

	// Profil : enregistré et relu, sans départ.
	if _, e := s.webAction(webRequest{Kind: "profile", Work: w.ID, Task: "t1",
		Provider: r.Provider, Role: "worker", Workspace: r.Workspace,
		Revision: currentRevision(t, s, w.ID)}); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	task, e := current.task("t1")
	if e != nil {
		t.Fatal(e)
	}
	if task.Profile == nil || task.Profile.Provider != r.Provider {
		t.Fatalf("le profil de lancement n'a pas été retenu : %+v", task.Profile)
	}

	// Autonomie : appliquée et relisible.
	if _, e := s.webAction(webRequest{Kind: "autonomy", Work: w.ID, Autonomy: autonomyManual, Slots: 3, Revision: currentRevision(t, s, w.ID)}); e != nil {
		t.Fatal(e)
	}
	if got := s.autonomy(w.ID); got != autonomyManual {
		t.Fatalf("niveau d'autonomie non appliqué : %q", got)
	}
	if got := s.slots(w.ID); got != 3 {
		t.Fatalf("nombre de créneaux non appliqué : %d", got)
	}

	// Ordonnancement en manuel : la porte répond, mais ne lance rien.
	out, e := s.webAction(webRequest{Kind: "dispatch", Work: w.ID, Revision: currentRevision(t, s, w.ID)})
	if e != nil {
		t.Fatal(e)
	}
	lances, _ := out.(map[string]any)["launched"].([]string)
	if len(lances) != 0 {
		t.Fatalf("le niveau manuel interdit tout départ automatique : %v", lances)
	}

	// Départs suspendus d'abord : le passage en autonome ordonnance lui-même,
	// et le mesurer après un départ déjà consommé ne prouverait rien.
	if _, e := s.webAction(webRequest{Kind: "pause", Work: w.ID, Revision: currentRevision(t, s, w.ID)}); e != nil {
		t.Fatal(e)
	}
	organizedFixtureStore(t, s)
	if _, e := s.webAction(webRequest{Kind: "autonomy", Work: w.ID, Autonomy: autonomyAuto, Slots: 2, Revision: currentRevision(t, s, w.ID)}); e != nil {
		t.Fatal(e)
	}
	out, e = s.webAction(webRequest{Kind: "dispatch", Work: w.ID, Revision: currentRevision(t, s, w.ID)})
	if e != nil {
		t.Fatal(e)
	}
	lances, _ = out.(map[string]any)["launched"].([]string)
	if len(lances) != 0 {
		t.Fatalf("la suspension prime sur l'ordonnancement : %v", lances)
	}
	if got := taskStatus(t, s, w.ID).Status; got != "todo" {
		t.Fatalf("aucune tentative ne doit être partie pendant la suspension : t1 est %q", got)
	}

	// Reprise : le même ordonnancement part, ce qui prouve que c'est bien la
	// suspension qui le retenait et non une tâche inéligible.
	if _, e := s.webAction(webRequest{Kind: "unpause", Work: w.ID, Revision: currentRevision(t, s, w.ID)}); e != nil {
		t.Fatal(e)
	}
	if got := taskStatus(t, s, w.ID).Status; got != "running" {
		t.Fatalf("la reprise doit relancer l'ordonnancement : t1 est %q", got)
	}
}
