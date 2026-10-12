//go:build linux

package engine

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

func movedManagedFixture(t *testing.T) (*Store, Work, Agent) {
	t.Helper()
	s, w := managedFixture(t)
	a := managedCompleted(t, s, w, "first", "retained after project move\n")
	w, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	original := s.root
	destination := filepath.Join(t.TempDir(), "moved-project")
	if err = os.Rename(original, destination); err != nil {
		t.Fatal(err)
	}
	if err = os.Symlink(destination, original); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(original, false)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { reopened.db.Close() })
	return reopened, w, a
}

func TestManagedRootAliasRetainsRepositoryCopyAndPublicBundle(t *testing.T) {
	s, w, a := movedManagedFixture(t)
	before, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	snapshot, _ := json.Marshal(before)
	repo, err := s.managedRepository(before)
	if err != nil {
		t.Fatal("owned repository refused after move", err)
	}
	if repo.Storage != w.Planning.Repository.Storage {
		t.Fatal("historical path rewritten")
	}
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if err = verifyManagedCopy(repo, item.Path); err != nil {
		t.Fatal("owned copy refused", err)
	}
	rel, err := filepath.Rel(filepath.Dir(filepath.Dir(filepath.Dir(repo.Storage))), filepath.Join(a.CWD, "docs", "first.md"))
	if err != nil {
		t.Fatal(err)
	}
	data, err := os.ReadFile(filepath.Join(a.CWD, "docs", "first.md"))
	if err != nil {
		t.Fatal(err)
	}
	if err = s.verifyPlanningHandoffArtifacts(a, []ExchangeArtifact{{Path: rel, SHA256: hash(data)}}); err != nil {
		t.Fatal("owned handoff refused", err)
	}
	h := newWebHandler(s, "local.test", "move-token")
	out := httptest.NewRecorder()
	req := httptest.NewRequest("GET", "http://local.test/api/v1/planning-bundle?"+url.Values{"work": {w.ID}}.Encode(), nil)
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "move-token"})
	h.ServeHTTP(out, req)
	if out.Code != 200 || !strings.Contains(out.Body.String(), repo.Candidate) {
		t.Fatal("public bundle unavailable", out.Code, out.Body.String())
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	final, _ := json.Marshal(after)
	if string(final) != string(snapshot) {
		t.Fatal("read/export rewrote mission history")
	}
}

func TestManagedRootAliasPreparedCopyIsReused(t *testing.T) {
	s, w := managedFixture(t)
	r := Launch{Schema: 1, EventID: "prepared-before-move", Revision: w.Revision, TaskID: "first", Provider: "managed-review-fixture", Role: "worker", Instruction: "Preserve prepared settings", Timeout: 60}
	copy, err := s.ensureManagedAttempt(w, r)
	if err != nil {
		t.Fatal(err)
	}
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	original := s.root
	destination := filepath.Join(t.TempDir(), "moved-project")
	if err = os.Rename(original, destination); err != nil {
		t.Fatal(err)
	}
	if err = os.Symlink(destination, original); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(destination, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	current, err := reopened.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	record, err := reopened.readPreparedLaunch(current, r.EventID)
	if err != nil {
		t.Fatal(err)
	}
	task, err := current.task(r.TaskID)
	if err != nil {
		t.Fatal(err)
	}
	if err = reopened.preparedLaunchGuard(current, task, record); err != nil {
		t.Fatal("prepared ownership lost", err)
	}
	reused, err := reopened.ensureManagedAttempt(current, r)
	if err != nil || reused != copy {
		t.Fatal("prepared copy was not reused", reused, copy, err)
	}
	var count int
	if err = reopened.db.QueryRow("SELECT COUNT(*) FROM managed_attempts WHERE work_id=?", w.ID).Scan(&count); err != nil || count != 1 {
		t.Fatal("copy duplicated", count, err)
	}
}

func TestManagedRootAliasStillRejectsInnerRedirectedCopy(t *testing.T) {
	s, w, a := movedManagedFixture(t)
	repo, err := s.managedRepository(w)
	if err != nil {
		t.Fatal("alias itself refused", err)
	}
	item, err := s.managedAttempt(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.Rename(item.Path, item.Path+"-preserved"); err != nil {
		t.Fatal(err)
	}
	if err = os.Symlink(repo.Source, item.Path); err != nil {
		t.Fatal(err)
	}
	if err = verifyManagedCopy(repo, item.Path); err == nil {
		t.Fatal("inner redirected copy accepted")
	}
	if err = s.verifyPlanningHandoffArtifacts(a, nil); err == nil {
		t.Fatal("redirected producer accepted")
	}
}

func TestManagedRootAliasPublishesExistingResultAndPreparesNextCopy(t *testing.T) {
	s, w, a := movedManagedFixture(t)
	if err := s.integrateManagedAttempt(a); err != nil {
		t.Fatal("retained result cannot publish", err)
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if current.Tasks[0].Status != "accepted" {
		t.Fatal("result not accepted by fixture review", current.Tasks[0].Blocker)
	}
	r := Launch{Schema: 1, EventID: "next-after-move", Revision: current.Revision, TaskID: "second", Provider: "managed-review-fixture", Role: "worker", Instruction: "Prepare next owned copy", Timeout: 60}
	copy, err := s.ensureManagedAttempt(current, r)
	if err != nil || copy == a.CWD {
		t.Fatal("next copy unavailable", copy, err)
	}
	agent, created, err := s.resumePreparedLaunch(w.ID, r.EventID, current.Revision)
	if err != nil || !created {
		t.Fatal("prepared launch cannot resume", created, err)
	}
	again, created, err := s.resumePreparedLaunch(w.ID, r.EventID, current.Revision)
	if err != nil || created || again.ID != agent.ID {
		t.Fatal("replay lost attribution", again.ID, created, err)
	}
}

func TestManagedRootAliasHandoffRefusesChildRootLinkAndExternalCopyAlias(t *testing.T) {
	s, _, a := movedManagedFixture(t)
	child := filepath.Join(s.root, "redirected-child")
	if err := os.Symlink(s.root, child); err != nil {
		t.Fatal(err)
	}
	bad := a
	bad.CWD = child
	if err := s.verifyPlanningHandoffArtifacts(bad, nil); err == nil {
		t.Fatal("inner child link back to root accepted")
	}
	outside := filepath.Join(t.TempDir(), "copy-link")
	if err := os.Symlink(a.CWD, outside); err != nil {
		t.Fatal(err)
	}
	bad.CWD = outside
	if err := s.verifyPlanningHandoffArtifacts(bad, nil); err == nil {
		t.Fatal("external alias of one copy accepted as project alias")
	}
}

func TestManagedRootAliasStagingRefusesForeignInputAndGitMetadata(t *testing.T) {
	for _, kind := range []string{"input-workspace", "git-metadata"} {
		t.Run(kind, func(t *testing.T) {
			s, w, a := movedManagedFixture(t)
			before, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			calls := before.Planning.Reviewer.Calls
			if kind == "input-workspace" {
				a.CWD = filepath.Join(s.root, "project")
			} else {
				gitDir := filepath.Join(a.CWD, ".git")
				if err = os.Rename(gitDir, gitDir+"-preserved"); err != nil {
					t.Fatal(err)
				}
				if err = os.Symlink(filepath.Join(w.Planning.Repository.Storage, "repository.git"), gitDir); err != nil {
					t.Fatal(err)
				}
			}
			if err = s.stageManagedResult(a); err == nil {
				t.Fatal("foreign custody accepted", kind)
			}
			item, err := s.managedAttempt(a.ID)
			if err != nil || item.Result != "" {
				t.Fatal("result retained before ownership check", item.Result, err)
			}
			after, err := s.get(w.ID)
			if err != nil || after.Revision != before.Revision || after.Planning.Reviewer.Calls != calls {
				t.Fatal("refusal changed publication/review", after.Revision, err)
			}
		})
	}
}
