//go:build linux

package main

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestWebAttributedReportRead(t *testing.T) {
	for _, mode := range []string{"incomplete", "review_rejected"} {
		t.Run(mode, func(t *testing.T) {
			s, w := managedFixture(t)
			a := managedCompleted(t, s, w, "first", "preserved result\n")
			if mode == "incomplete" {
				a.DeliveryVersion = 1
				if err := s.saveAgent(a); err != nil {
					t.Fatal(err)
				}
			} else {
				managedReviewMode(t, s, "fail")
			}
			if err := s.integrateManagedAttempt(a); err != nil {
				t.Fatal(err)
			}
			w, _ = s.get(w.ID)
			reports := s.taskReportsForWork(w.ID, "first")
			if len(reports) != 1 {
				t.Fatal(reports)
			}
			h := newWebHandler(s, "local.test", "token")
			read := func(work, task, path string) *httptest.ResponseRecorder {
				t.Helper()
				q := url.Values{"work": {work}, "task": {task}, "path": {path}}
				r := httptest.NewRequest(http.MethodGet, "http://local.test/api/v1/report?"+q.Encode(), nil)
				r.AddCookie(&http.Cookie{Name: "swarm_session", Value: "token"})
				out := httptest.NewRecorder()
				h.ServeHTTP(out, r)
				return out
			}
			out := read(w.ID, "first", reports[0])
			var content map[string]string
			if out.Code != 200 || json.Unmarshal(out.Body.Bytes(), &content) != nil || !strings.Contains(content["text"], "Rapport nouveau first") {
				t.Fatalf("listed report unreadable: %d %s", out.Code, out.Body.String())
			}
			task, _ := w.task("first")
			console := &consoleState{dialog: &taskDialog{task: *task, mode: "actions", row: 10}}
			s.dialogKey(w.ID, console, "enter")
			if console.dialog.mode != "report" || !strings.Contains(console.dialog.review, content["text"]) {
				t.Fatal("CLI report action does not expose the same report", console.dialog.review)
			}
			context, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{"first"}, Report: reports[0]})
			if err != nil || !strings.Contains(context.Facts[len(context.Facts)-1].Value, "Rapport nouveau first") {
				t.Fatal("assistant cannot read the attributed report", err)
			}
			if _, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{"second"}, Report: reports[0]}); err == nil {
				t.Fatal("assistant read another task's report")
			}
			for _, pair := range [][2]string{{"", ""}, {w.ID, "second"}, {"absent-work", "first"}} {
				if denied := read(pair[0], pair[1], reports[0]); denied.Code != 403 {
					t.Fatalf("unbound report exposed: %d", denied.Code)
				}
			}
			private := filepath.Join(s.root, "private.txt")
			if err := os.WriteFile(private, []byte("private material"), 0600); err != nil {
				t.Fatal(err)
			}
			if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
				t.Fatal(err)
			}
			if err := os.Symlink("../private.txt", filepath.Join(s.root, "docs/first.md")); err != nil {
				t.Fatal(err)
			}
			for _, path := range []string{"private.txt", "docs/first.md"} {
				if denied := read(w.ID, "first", path); denied.Code != 403 || strings.Contains(denied.Body.String(), "private material") {
					t.Fatalf("private file exposed: %d", denied.Code)
				}
				if _, err := s.pageContext(w.ID, PageCoordinates{PageID: "tasks", Selected: []string{"first"}, Report: path}); err == nil {
					t.Fatal("assistant read private file through a report coordinate", path)
				}
			}
			if mode == "review_rejected" {
				if err := os.WriteFile(filepath.Join(s.root, reports[0]), []byte("tampered"), 0600); err != nil {
					t.Fatal(err)
				}
				if denied := read(w.ID, "first", reports[0]); denied.Code != 403 {
					t.Fatalf("modified reviewed report exposed as current: %d", denied.Code)
				}
			}
			after, _ := s.get(w.ID)
			if after.Revision != w.Revision || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || len(after.Tasks[0].Attempts) != 1 {
				t.Fatal("reading report mutated mission")
			}
		})
	}
}
