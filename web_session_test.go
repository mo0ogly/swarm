//go:build linux

package main

import (
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestWebSessionStablePrivateAndScoped(t *testing.T) {
	s := storeTest(t)
	address := "127.0.0.1:18787"
	first, e := s.webSessionToken(address)
	if e != nil {
		t.Fatal(e)
	}
	second, e := s.webSessionToken(address)
	if e != nil || first != second {
		t.Fatal("unstable credential", e)
	}
	other, e := s.webSessionToken("127.0.0.1:18788")
	if e != nil || first == other {
		t.Fatal("unscoped credential", e)
	}
	path := filepath.Join(s.root, ".swarm", "web-session-"+hash([]byte(address))[:16])
	info, e := os.Stat(path)
	if e != nil || info.Mode().Perm() != 0600 {
		t.Fatal("not private", e)
	}
	h := newWebHandler(s, "local.test", first)
	for _, tc := range []struct {
		path   string
		code   int
		target string
	}{
		{"/session/" + first + "?work=w-example", 303, "/?work=w-example"},
		{"/session/wrong?work=w-example", 403, ""},
		{"/session/" + first + "?work=../bad", 400, ""},
	} {
		rr := httptest.NewRecorder()
		h.ServeHTTP(rr, httptest.NewRequest("GET", "http://local.test"+tc.path, nil))
		if rr.Code != tc.code || rr.Header().Get("Location") != tc.target {
			t.Fatalf("redirect: %d %s", rr.Code, rr.Header().Get("Location"))
		}
	}
	if e = os.Chmod(path, 0644); e != nil {
		t.Fatal(e)
	}
	if _, e = s.webSessionToken(address); e == nil {
		t.Fatal("public credential accepted")
	}
	if e = os.Remove(path); e != nil {
		t.Fatal(e)
	}
	if e = os.Symlink("providers.json", path); e != nil {
		t.Fatal(e)
	}
	if _, e = s.webSessionToken(address); e == nil {
		t.Fatal("symlink accepted")
	}
}
