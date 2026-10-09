//go:build linux

package engine

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestWebTrainingAssetsAndAuthentication(t *testing.T) {
	s := storeTest(t)
	h := newWebHandler(s, "local.test", "training-test-capability")
	request := func(path string, auth bool, rangeHeader string) *httptest.ResponseRecorder {
		req := httptest.NewRequest("GET", "http://local.test"+path, nil)
		if auth {
			req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "training-test-capability"})
		}
		if rangeHeader != "" {
			req.Header.Set("Range", rangeHeader)
		}
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, req)
		return rr
	}
	if rr := request("/training/casa-pizza/tutoriels/index.html", false, ""); rr.Code == 200 {
		t.Fatal("training bypasses cockpit authentication")
	}
	for _, path := range []string{"tutoriels/media/graph-overview-fr.mp4", "tutoriels/media/graph-overview-fr-poster.png", "tutoriels/media/graph-overview-fr.vtt", "tutoriels/swarm-logo.png", "tutoriels/", "tutoriels/player.js", "tutoriels/player.css", "tutoriels/frames/03-minimum.png", "Formation_Swarm_Casa_Pizza.pdf", "Swarm_Casa_Pizza_Training_EN.pdf", "Formation_Swarm_Casa_Pizza.docx", "Swarm_Casa_Pizza_Training_EN.docx", "Kit_Formation_Swarm_Casa_Pizza.zip", "Swarm_Casa_Pizza_Training_Kit_EN.zip"} {
		rr := request("/training/casa-pizza/"+path, true, "")
		if rr.Code != 200 || rr.Body.Len() == 0 {
			t.Fatalf("%s: %d", path, rr.Code)
		}
		if strings.Contains(rr.Header().Get("Content-Security-Policy"), "unsafe-inline") {
			t.Fatal("training weakens CSP")
		}
	}
	if rr := request("/training/casa-pizza/tutoriels/index.html", true, ""); rr.Code != http.StatusMovedPermanently {
		t.Fatal("expected canonical directory redirect")
	}
	page := request("/training/casa-pizza/tutoriels/", true, "").Body.String()
	if strings.Contains(page, "<style>") || strings.Contains(page, "<script>") {
		t.Fatal("player cannot run under cockpit CSP")
	}
	rr := request("/training/casa-pizza/tutoriels/media/03-client-fr.mp4", true, "bytes=0-99")
	if rr.Code != 206 || rr.Body.Len() != 100 {
		t.Fatalf("video seek unsupported: %d, %d", rr.Code, rr.Body.Len())
	}
	for _, path := range []string{"projet-pizza/app.py", "tutoriels/build_media.py", ".swarm/web-session"} {
		if rr := request("/training/casa-pizza/"+path, true, ""); rr.Code != 404 {
			t.Fatalf("unexpected embedded file %s: %d", path, rr.Code)
		}
	}
}
