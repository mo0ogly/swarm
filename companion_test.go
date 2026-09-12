package main

import (
	"archive/zip"
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func storeTest(t *testing.T) *Store {
	t.Helper()
	s, e := openStore(t.TempDir(), true)
	if e != nil {
		t.Fatal(e)
	}
	t.Cleanup(func() { s.db.Close() })
	return s
}
func applyTest(t *testing.T, s *Store, w Work, kind string, r Request) Work {
	t.Helper()
	r.Schema = 1
	r.Revision = w.Revision
	r.EventID = newID("e-")
	b, _ := json.Marshal(r)
	v, e := s.mutate(w.ID, kind, r.EventID, r.Revision, b, func(w *Work) error { return s.apply(w, kind, r) })
	if e != nil {
		t.Fatal(e)
	}
	return v
}
func createTest(t *testing.T, s *Store) Work {
	return applyTest(t, s, Work{}, "work.create", Request{Title: "Reprise claire", Objective: "Préserver le travail", Scope: "projet local", Criteria: []string{"preuves courantes"}, Next: "Ajouter une tâche"})
}
func taskTest(t *testing.T, s *Store, w Work) Work {
	return applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Contrôler les preuves", Deliverable: "rapport de test", Criteria: []string{"tests PASS"}, Owner: "session", Next: "lancer le test"})
}
func fixture(t *testing.T, root string) []byte {
	t.Helper()
	if e := os.WriteFile(filepath.Join(root, "proof.txt"), []byte("proof"), 0600); e != nil {
		t.Fatal(e)
	}
	return []byte(fmt.Sprintf(`{"method_version":"2","scope_id":"t1","artifacts":{"proof.txt":"%s"},"domains":{"quality":90,"security":10},"checks":[{"id":"q","domain":"quality","mandatory":true,"gate":"validation","penalty":10,"max_penalty":100,"severity":"minor"},{"id":"s","domain":"security","mandatory":true,"gate":"validation","penalty":60,"max_penalty":100,"severity":"major"}],"results":[{"id":"q","status":"PASS","count":0,"evidence":["proof.txt"]},{"id":"s","status":"PASS","count":0,"evidence":["proof.txt"]}]}`, hash([]byte("proof"))))
}
func gateTest(t *testing.T, s *Store, w Work, raw []byte) Work {
	t.Helper()
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil {
		t.Fatal(e)
	}
	r := Request{Schema: 1, EventID: newID("e-"), Revision: w.Revision}
	b, _ := json.Marshal(r)
	w, e = s.mutate(w.ID, "gate", r.EventID, r.Revision, b, func(w *Work) error {
		w.Tasks[0].Gate = &GateRecord{Document: raw, Evaluation: ev, At: now()}
		return nil
	})
	if e != nil {
		t.Fatal(e)
	}
	return w
}
func TestResumeDurableAndNoAutostart(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "checkpoint", Request{Summary: "Test préparé, non exécuté", Next: "Exécuter le test ciblé"})
	s.db.Close()
	s2, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer s2.db.Close()
	got, e := s2.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	v, e := s2.render(got)
	if e != nil {
		t.Fatal(e)
	}
	for _, want := range []string{"non confirmée", "Test préparé", "Exécuter le test ciblé", "gate non renseignée"} {
		if !strings.Contains(v, want) {
			t.Fatal(v)
		}
	}
	if got.Revision != w.Revision {
		t.Fatal("resume mutated revision")
	}
	for _, line := range strings.Split(v, "\n") {
		if len([]rune(line)) > 80 {
			t.Fatal("width", line)
		}
	}
	b, _ := os.ReadFile(filepath.Join(s.root, ".swarm", "views", w.ID+".md"))
	if string(b) != v {
		t.Fatal("markdown differs")
	}
}
func TestIdempotencyAndRevision(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	r := Request{Schema: 1, EventID: "repeat", Revision: w.Revision, Summary: "checkpoint"}
	b, _ := json.Marshal(r)
	fn := func(w *Work) error { return s.apply(w, "checkpoint", r) }
	one, e := s.mutate(w.ID, "checkpoint", r.EventID, r.Revision, b, fn)
	if e != nil {
		t.Fatal(e)
	}
	two, e := s.mutate(w.ID, "checkpoint", r.EventID, r.Revision, b, fn)
	if e != nil || two.Revision != one.Revision {
		t.Fatal(two, e)
	}
	if _, e = s.mutate(w.ID, "checkpoint", "other", r.Revision, b, fn); e == nil {
		t.Fatal("stale revision accepted")
	}
	if _, e = s.mutate(w.ID, "checkpoint", r.EventID, r.Revision, []byte(`{}`), fn); e == nil {
		t.Fatal("event collision accepted")
	}
	ev, _ := s.events(w.ID)
	if len(ev) != 2 {
		t.Fatal(len(ev))
	}
}
func TestConcurrentWriters(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	s2, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer s2.db.Close()
	ch := make(chan error, 2)
	var wg sync.WaitGroup
	for _, st := range []*Store{s, s2} {
		wg.Add(1)
		go func(st *Store) {
			defer wg.Done()
			_, e := st.mutate(w.ID, "checkpoint", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error { w.Summary = "writer"; return nil })
			ch <- e
		}(st)
	}
	wg.Wait()
	close(ch)
	ok := 0
	for e := range ch {
		if e == nil {
			ok++
		}
	}
	if ok != 1 {
		t.Fatal("expected one writer", ok)
	}
	v, _ := s.get(w.ID)
	ev, _ := s.events(w.ID)
	if v.Revision != 2 || len(ev) != 2 {
		t.Fatal(v, ev)
	}
}
func TestCrashRollsBack(t *testing.T) {
	if root := os.Getenv("SWARM_TEST_CRASH_ROOT"); root != "" {
		s, e := openStore(root, false)
		if e != nil {
			os.Exit(28)
		}
		tx, e := s.db.Begin()
		if e != nil {
			os.Exit(29)
		}
		_, e = tx.Exec("UPDATE works SET revision=99")
		if e != nil {
			os.Exit(30)
		}
		os.Exit(27)
	}
	s := storeTest(t)
	w := createTest(t, s)
	cmd := exec.Command(os.Args[0], "-test.run=^TestCrashRollsBack$")
	cmd.Env = append(os.Environ(), "SWARM_TEST_CRASH_ROOT="+s.root)
	if e := cmd.Run(); e == nil {
		t.Fatal("expected interruption")
	}
	var rev int
	if e := s.db.QueryRow("SELECT revision FROM works WHERE id=?", w.ID).Scan(&rev); e != nil || rev != 1 {
		t.Fatal(rev, e)
	}
	ev, _ := s.events(w.ID)
	if len(ev) != 1 {
		t.Fatal(ev)
	}
}
func TestAcceptanceFreshnessAndAttempts(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.apply(&w, "task.update", Request{ID: "t1", Status: "accepted"}); e == nil {
		t.Fatal("accepted without evidence")
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Blocker: "test interrompu", Outcome: "interrupted"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, fixture(t, s.root))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	if len(w.Tasks[0].Attempts) != 2 {
		t.Fatal(w)
	}
	if state, _, _ := s.workStatus(w); state != "VALIDÉ" {
		t.Fatal(state)
	}
	os.WriteFile(filepath.Join(s.root, "proof.txt"), []byte("changed"), 0600)
	if state, _, _ := s.workStatus(w); state != "BLOQUÉ" {
		t.Fatal(state)
	}
	v, _ := s.view(w)
	if !strings.Contains(v, "PÉRIMÉES") {
		t.Fatal(v)
	}
}
func TestDependencyAndEmpty(t *testing.T) {
	s := storeTest(t)
	ws, _ := s.list()
	if !strings.Contains(s.listView(ws), "Aucun travail") {
		t.Fatal(ws)
	}
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "client", Deliverable: "test", Criteria: []string{"ok"}, Depends: []string{"t1"}})
	if e := s.apply(&w, "task.update", Request{ID: "t2", Status: "running"}); e == nil {
		t.Fatal("dependency bypass")
	}
	if e := s.apply(&w, "task.add", Request{ID: "t3", Title: "t", Deliverable: "t", Criteria: []string{"ok"}, Depends: []string{"missing"}}); e == nil {
		t.Fatal("missing dependency")
	}
}
func TestExportImportMissingAndNoOverwrite(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = gateTest(t, s, w, fixture(t, s.root))
	p := filepath.Join(t.TempDir(), "work.zip")
	if e := s.export(w.ID, p); e != nil {
		t.Fatal(e)
	}
	other := storeTest(t)
	got, e := other.importBundle(p)
	if e != nil {
		t.Fatal(e)
	}
	if got.ID != w.ID || got.Revision != w.Revision {
		t.Fatal(got)
	}
	if other.validGate(&got.Tasks[0]) {
		t.Fatal("archive certified missing destination file")
	}
	if _, e = other.importBundle(p); e == nil {
		t.Fatal("overwrite accepted")
	}
	fixture(t, other.root)
	if !other.validGate(&got.Tasks[0]) {
		t.Fatal("identical destination not valid")
	}
	ev, _ := other.events(w.ID)
	if len(ev) != w.Revision {
		t.Fatal(ev)
	}
	if e = s.export(w.ID, p); e == nil {
		t.Fatal("overwritten export")
	}
}
func TestRejectHostileArchive(t *testing.T) {
	for _, name := range []string{"../outside", "/tmp/out", "blobs/../../out"} {
		t.Run(name, func(t *testing.T) {
			s := storeTest(t)
			p := filepath.Join(t.TempDir(), "bad.zip")
			f, _ := os.Create(p)
			z := zip.NewWriter(f)
			entry, _ := z.Create(name)
			entry.Write([]byte("bad"))
			z.Close()
			f.Close()
			if _, e := s.importBundle(p); e == nil {
				t.Fatal("unsafe archive accepted")
			}
		})
	}
}
func TestScore94AndPathProtection(t *testing.T) {
	s := storeTest(t)
	raw := fixture(t, s.root)
	raw = bytes.Replace(raw, []byte(`"id":"s","status":"PASS","count":0`), []byte(`"id":"s","status":"FAIL","count":1`), 1)
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil || ev.Quality == nil || *ev.Quality != 94 || ev.Ship {
		t.Fatal(ev, e)
	}
	if _, e = localFile(s.root, "../outside"); e == nil {
		t.Fatal("outside accepted")
	}
	outside := filepath.Join(t.TempDir(), "secret")
	os.WriteFile(outside, []byte("secret"), 0600)
	os.Symlink(outside, filepath.Join(s.root, "link"))
	if _, e = localFile(s.root, "link"); e == nil {
		t.Fatal("symlink escaped")
	}
}
func TestCLIJSONAndExitCodes(t *testing.T) {
	root := t.TempDir()
	var out, err bytes.Buffer
	if code := run([]string{"--root", root, "init"}, &out, &err); code != 0 {
		t.Fatal(err.String())
	}
	raw := fixture(t, root)
	p := filepath.Join(root, "input.json")
	os.WriteFile(p, raw, 0600)
	out.Reset()
	if code := run([]string{"--root", root, "evaluate", "--input", p}, &out, &err); code != 0 {
		t.Fatal(err.String())
	}
	var ev Evaluation
	if e := json.Unmarshal(out.Bytes(), &ev); e != nil || !ev.Ship {
		t.Fatal(ev, e)
	}
	os.WriteFile(p, []byte(`{}`), 0600)
	if code := run([]string{"--root", root, "evaluate", "--input", p}, &out, &err); code != 2 {
		t.Fatal(code)
	}
}

func TestReadOnlyShowAndMalformedStatus(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	if e := s.apply(&w, "task.update", Request{ID: "t1", Status: "running blocked"}); e == nil {
		t.Fatal("compound status accepted")
	}
	var out, errs bytes.Buffer
	if code := run([]string{"--root", s.root, "work", "show", w.ID}, &out, &errs); code != 0 {
		t.Fatal(errs.String())
	}
	if _, e := os.Stat(filepath.Join(s.root, ".swarm", "views", w.ID+".md")); !os.IsNotExist(e) {
		t.Fatal("read-only show wrote a view", e)
	}
}
func TestReopenedDependencyInvalidatesAcceptance(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	raw := fixture(t, s.root)
	ev, e := evaluate(raw, s.root, "delivery")
	if e != nil {
		t.Fatal(e)
	}
	w.Tasks[0].Status = "accepted"
	w.Tasks[0].Gate = &GateRecord{Document: raw, Evaluation: ev, At: now()}
	childRaw := bytes.Replace(raw, []byte(`"scope_id":"t1"`), []byte(`"scope_id":"t2"`), 1)
	childEval, _ := evaluate(childRaw, s.root, "delivery")
	w.Tasks = append(w.Tasks, Task{ID: "t2", Status: "accepted", Depends: []string{"t1"}, Gate: &GateRecord{Document: childRaw, Evaluation: childEval, At: now()}})
	if !s.acceptedFresh(&w, &w.Tasks[1], map[string]bool{}) {
		t.Fatal("valid dependency rejected")
	}
	w.Tasks[0].Status = "todo"
	if s.acceptedFresh(&w, &w.Tasks[1], map[string]bool{}) {
		t.Fatal("reopened dependency accepted")
	}
}
func TestImportMissingPieceAndUnknownVersion(t *testing.T) {
	for _, version := range []int{1, 99} {
		s := storeTest(t)
		w := createTest(t, s)
		ev, _ := s.events(w.ID)
		bundle := Bundle{Schema: version, Work: w, Events: ev, Files: map[string]string{"proof": "missing"}}
		b, _ := json.Marshal(bundle)
		path := filepath.Join(t.TempDir(), "bad.zip")
		f, _ := os.Create(path)
		z := zip.NewWriter(f)
		entry, _ := z.Create("manifest.json")
		entry.Write(b)
		z.Close()
		f.Close()
		other := storeTest(t)
		if _, e := other.importBundle(path); e == nil {
			t.Fatal("bad archive accepted")
		}
		ws, _ := other.list()
		if len(ws) != 0 {
			t.Fatal("partial import")
		}
	}
}

func TestAbandonedIsNotCompletion(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "abandoned"})
	state, a, n := s.workStatus(w)
	if state != "ABANDONNÉ" || a != 0 || n != 1 {
		t.Fatal(state, a, n)
	}
	if _, e := evaluate(append(fixture(t, s.root), []byte(` {}`)...), s.root, "delivery"); e == nil {
		t.Fatal("trailing JSON accepted")
	}
}
