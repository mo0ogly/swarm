//go:build linux

package engine

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func projectMemorySourceFixture(t *testing.T, s *Store) (Work, Retex) {
	t.Helper()
	w := memorySeed(t, s, createTest(t, s))
	r := Retex{ID: "memory-source", Source: dialogueHits(w)[0].Ref, Problem: "Vérifier la reprise", Nature: "fait", Proposal: "Vérifier le parcours affecté", Effort: "faible", Risk: "Résultat historique", Criteria: "Effet observé", Proof: "proof.txt"}
	w = retexTestAction(t, s, w, "retex-save", r, "")
	w = retexTestAction(t, s, w, "retex-task", r, "")
	id := w.Retex[0].Task
	raw := []byte(strings.ReplaceAll(string(fixture(t, s.root)), `"t1"`, `"`+id+`"`))
	ev, err := evaluate(raw, s.root, "delivery")
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.mutate(w.ID, "fixture", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error {
		task, _ := w.task(id)
		task.Status = "accepted"
		task.Gate = &GateRecord{Document: raw, Evaluation: ev, At: now()}
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	w = retexTestAction(t, s, w, "retex-status", r, "verifie")
	return w, w.Retex[0]
}

func admittedMemoryFixture(t *testing.T, s *Store) (Work, ProjectMemoryRequest, MemoryReference) {
	t.Helper()
	w, r := projectMemorySourceFixture(t, s)
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: w.Revision, Work: w.ID, ID: "lesson", SourceRetex: r.ID, Kind: "experience", Body: "Vérifier la réception avant reprise.", Conditions: "Même parcours, preuves courantes à revalider.", Reason: "Expérience vérifiée"}
	p, err := s.projectMemoryOperation("admit-preview", q)
	if err != nil {
		t.Fatal(err)
	}
	q.PreviewToken = p.(map[string]any)["preview_token"].(string)
	v, err := s.projectMemoryOperation("admit", q)
	if err != nil {
		t.Fatal(err)
	}
	w = v.(Work)
	last := w.ProjectMemory[0].Versions[0]
	return w, q, MemoryReference{w.ID, "lesson", last.Version, last.SHA256}
}

func memoryPreparedRequest(t *testing.T, s *Store, action string, q ProjectMemoryRequest) ProjectMemoryRequest {
	t.Helper()
	p, err := s.projectMemoryOperation(action+"-preview", q)
	if err != nil {
		t.Fatal(err)
	}
	q.PreviewToken = p.(map[string]any)["preview_token"].(string)
	return q
}

func TestProjectMemoryReplacementPersistenceAndStaleSource(t *testing.T) {
	s := storeTest(t)
	w, q, old := admittedMemoryFixture(t, s)
	original := w.ProjectMemory[0].Versions[0]
	q.Revision, q.EventID, q.ExpectedSHA256, q.Body = w.Revision, newID("memory-"), old.SHA256, "Vérifier la réception et l’effet avant reprise."
	q = memoryPreparedRequest(t, s, "replace", q)
	v, err := s.projectMemoryOperation("replace", q)
	if err != nil {
		t.Fatal(err)
	}
	w = v.(Work)
	if len(w.ProjectMemory[0].Versions) != 2 || w.ProjectMemory[0].Versions[0] != original {
		t.Fatal("replacement rewrote history")
	}
	if _, err = s.resolveMemory(s.db, old); err == nil {
		t.Fatal("superseded version selectable")
	}
	latest := w.ProjectMemory[0].Versions[1]
	ref := MemoryReference{w.ID, "lesson", latest.Version, latest.SHA256}
	root := s.root
	s.db.Close()
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	if got, err := reopened.resolveMemory(reopened.db, ref); err != nil || got.Body != q.Body {
		t.Fatal("memory not restored", err)
	}
	if err = os.WriteFile(filepath.Join(root, "proof.txt"), []byte("changed evidence"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = reopened.resolveMemory(reopened.db, ref); err == nil {
		t.Fatal("stale source accepted")
	}
	withdraw := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: w.Revision, Work: w.ID, ID: "lesson", ExpectedSHA256: ref.SHA256, Reason: "Preuve périmée"}
	withdraw = memoryPreparedRequest(t, reopened, "withdraw", withdraw)
	if _, err = reopened.projectMemoryOperation("withdraw", withdraw); err != nil {
		t.Fatal("stale knowledge could not be withdrawn", err)
	}
	// An identical historical request still replays after its source becomes stale.
	if _, err = reopened.projectMemoryOperation("replace", q); err != nil {
		t.Fatal("replay revalidated an already committed decision", err)
	}
}

func TestProjectMemoryConcurrentReplacement(t *testing.T) {
	s := storeTest(t)
	w, q, ref := admittedMemoryFixture(t, s)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	q.Revision, q.ExpectedSHA256 = w.Revision, ref.SHA256
	q.EventID = newID("memory-")
	first := memoryPreparedRequest(t, s, "replace", q)
	q.Body = "Autre procédure justifiée."
	q.EventID = newID("memory-")
	second := memoryPreparedRequest(t, other, "replace", q)
	errors := make(chan error, 2)
	var wg sync.WaitGroup
	for i, store := range []*Store{s, other} {
		wg.Add(1)
		go func(i int, store *Store) {
			defer wg.Done()
			request := first
			if i == 1 {
				request = second
			}
			_, err := store.projectMemoryOperation("replace", request)
			errors <- err
		}(i, store)
	}
	wg.Wait()
	close(errors)
	successes := 0
	for err := range errors {
		if err == nil {
			successes++
		} else if commandFailure(err).Code != "revision_conflict" {
			t.Fatal(err)
		}
	}
	w, _ = s.get(w.ID)
	if successes != 1 || len(w.ProjectMemory[0].Versions) != 2 {
		t.Fatal("lost concurrent update", successes)
	}
}

func TestProjectMemoryInvalidInputsAndAtomicSelection(t *testing.T) {
	s := storeTest(t)
	w, q, ref := admittedMemoryFixture(t, s)
	q.Revision, q.EventID, q.Body = w.Revision, newID("memory-"), "Changed without preview"
	if _, err := s.projectMemoryOperation("replace", q); err == nil {
		t.Fatal("missing current hash accepted")
	}
	q.ExpectedSHA256 = ref.SHA256
	q.PreviewToken = "stale"
	if _, err := s.projectMemoryOperation("replace", q); err == nil {
		t.Fatal("stale preview accepted")
	}
	got, _ := s.get(w.ID)
	if got.Revision != w.Revision || len(got.ProjectMemory[0].Versions) != 1 {
		t.Fatal("rejection mutated history")
	}
	if _, err := s.memoryAttachments(s.db, []MemoryReference{ref}, "planner"); err == nil {
		t.Fatal("unsupported role received worker selection")
	}
	foreign := storeTest(t)
	if _, err := foreign.resolveMemory(foreign.db, ref); err == nil {
		t.Fatal("other project received content")
	}
	dest, launch := setupAgent(t, s)
	selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: dest.Revision, Work: dest.ID, Task: launch.TaskID, Selections: []MemoryReference{ref, ref}, Reason: "Duplicate"}
	if _, err := s.projectMemoryOperation("select-preview", selection); err == nil {
		t.Fatal("duplicate selection accepted")
	}
	selection.Selections = []MemoryReference{ref}
	selection = memoryPreparedRequest(t, s, "select", selection)
	// Change the original dialogue without changing the RETEX's reference.
	w, err := s.mutate(w.ID, "fixture", newID("e-"), w.Revision, []byte(`{}`), func(w *Work) error { w.Tasks[0].Answers[0].Text = "changed original"; return nil })
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.projectMemoryOperation("select", selection); err == nil {
		t.Fatal("source drift after preview accepted")
	}
	destAfter, _ := s.get(dest.ID)
	if destAfter.Revision != dest.Revision || len(destAfter.Tasks[0].MemorySelections) > 0 {
		t.Fatal("failed selection partially committed")
	}
}

func TestProjectMemoryArchiveDoesNotImportAdmission(t *testing.T) {
	s := storeTest(t)
	w, _, ref := admittedMemoryFixture(t, s)
	archive := filepath.Join(t.TempDir(), "memory.zip")
	if err := s.export(w.ID, archive); err != nil {
		t.Fatal(err)
	}
	other := storeTest(t)
	imported, err := other.importBundle(archive)
	if err != nil {
		t.Fatal(err)
	}
	if len(imported.ProjectMemory) != 1 || !imported.ProjectMemory[0].Imported {
		t.Fatal("imported admission became active")
	}
	if imported.ProjectMemory[0].Versions[0].SHA256 != ref.SHA256 {
		t.Fatal("import changed historical version")
	}
	if _, err = other.resolveMemory(other.db, ref); err == nil {
		t.Fatal("foreign admission accepted")
	}
}

func TestProjectMemoryRecipeBundle(t *testing.T) {
	path := os.Getenv("SWARM_MEMORY_RECIPE_BUNDLE")
	if path == "" {
		t.Skip("Only exported for an explicitly requested isolated browser fixture")
	}
	if _, err := os.Lstat(path); !os.IsNotExist(err) {
		t.Fatal("fixture output must be absent")
	}
	s := storeTest(t)
	w, _ := projectMemorySourceFixture(t, s)
	if err := s.export(w.ID, path); err != nil {
		t.Fatal(err)
	}
}

func TestProjectMemoryMigrationPreservesOriginals(t *testing.T) {
	s := storeTest(t)
	w, _, _ := admittedMemoryFixture(t, s)
	var original []byte
	if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&original); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA user_version=32"); err != nil {
		t.Fatal(err)
	}
	if inspected, err := openStoreWithMigration(s.root, false, false); err == nil {
		inspected.db.Close()
		t.Fatal("observation silently migrated")
	}
	upgraded, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer upgraded.db.Close()
	var current []byte
	if err = upgraded.db.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&current); err != nil || !bytes.Equal(original, current) {
		t.Fatal("migration rewrote work", err)
	}
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm/state-pre-v33-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatal("migration backup missing", err)
	}
	backup, err := sql.Open("sqlite", backups[0])
	if err != nil {
		t.Fatal(err)
	}
	defer backup.Close()
	var retained []byte
	if err = backup.QueryRow("SELECT body FROM works WHERE id=?", w.ID).Scan(&retained); err != nil || !bytes.Equal(retained, original) {
		t.Fatal("backup lost memory", err)
	}
}

func TestProjectMemoryLegacyGuideAndPreparedGuard(t *testing.T) {
	s := storeTest(t)
	w, _, ref := admittedMemoryFixture(t, s)
	q := webRequest{Kind: "retex-guide-preview", Work: w.ID, Task: w.Retex[0].ID, Revision: w.Revision, Event: newID("e-"), Note: "LEGACY_MUST_NOT_BE_RECALLED"}
	p, err := s.retexAction(q)
	if err != nil {
		t.Fatal(err)
	}
	q.Kind = "retex-guide"
	q.ContextHash = p.(map[string]any)["digest"].(string)
	if _, err = s.retexAction(q); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	destination, launch := setupAgent(t, s)
	a, _, err := s.prepareLaunch(destination.ID, launch, true)
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(a.Prompt, "retex-"+w.Retex[0].ID+".md") || strings.Contains(a.Prompt, q.Note) {
		t.Fatal("managed lesson bypassed explicit selection")
	}
	profile := ProjectProfile{Version: 1, Name: "Legacy lesson", Instructions: []ProjectInstruction{{Path: w.Retex[0].Lesson, Roles: append([]string{}, projectRoles...)}}}
	if err = s.applyProjectProfile(profile, ""); err != nil {
		t.Fatal(err)
	}
	for _, role := range projectRoles {
		if _, _, err = s.projectContext(role); err == nil {
			t.Fatal("profile role bypassed versioned memory", role)
		}
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", destination.ID); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.projectAgentWorkflow("planner"); err == nil {
		t.Fatal("transactional planner context bypassed guard")
	}
	if err = tx.Rollback(); err != nil {
		t.Fatal(err)
	}
	if _, _, err = s.prepareLaunch(destination.ID, launch, true); err == nil || !strings.Contains(err.Error(), "source héritée") {
		t.Fatal("direct profile bypassed selection", err)
	}
	if err = os.Remove(filepath.Join(s.root, projectProfileFile)); err != nil {
		t.Fatal(err)
	}
	destination, err = s.executeRequest(destination.ID, "checkpoint", Request{Schema: 1, EventID: newID("e-"), Revision: destination.Revision, Summary: "Legacy alias fixture", Memory: []string{"./" + w.Retex[0].Lesson}})
	if err != nil {
		t.Fatal(err)
	}
	launch.Revision = destination.Revision
	if _, _, err = s.prepareLaunch(destination.ID, launch, true); err == nil {
		t.Fatal("recovery-piece alias bypassed versioned memory")
	}
	destination, err = s.executeRequest(destination.ID, "checkpoint", Request{Schema: 1, EventID: newID("e-"), Revision: destination.Revision, Summary: "Legacy reference removed", Memory: []string{}})
	if err != nil {
		t.Fatal(err)
	}
	selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: destination.Revision, Work: destination.ID, Task: launch.TaskID, Selections: []MemoryReference{ref}, Reason: "Pertinent"}
	selection = memoryPreparedRequest(t, s, "select", selection)
	v, err := s.projectMemoryOperation("select", selection)
	if err != nil {
		t.Fatal(err)
	}
	destination = v.(Work)
	launch.Revision = destination.Revision
	a, _, err = s.prepare(destination.ID, launch)
	if err != nil {
		t.Fatal(err)
	}
	withdraw := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: w.Revision, Work: w.ID, ID: ref.ID, ExpectedSHA256: ref.SHA256, Reason: "Retrait après préparation"}
	withdraw = memoryPreparedRequest(t, s, "withdraw", withdraw)
	if _, err = s.projectMemoryOperation("withdraw", withdraw); err != nil {
		t.Fatal(err)
	}
	if err = s.projectMemoryGuard(a); err == nil {
		t.Fatal("prepared send ignored withdrawal")
	}
	if err = s.finishAgent(a, "interrupted", "Fixture", nil); err != nil {
		t.Fatal(err)
	}
}

func TestProjectMemoryGuideReferenceNormalization(t *testing.T) {
	paths := map[string]bool{".claude/field-guide/retex-example.md": true}
	for _, link := range []string{"retex-example.md", "./retex-example.md#detail", "../field-guide/retex-example.md#detail", "./retex%2dexample.md?view=1#detail"} {
		body := "# Guide\n[Managed](" + link + ")\n[Other](../../docs/retex-example.md)\n"
		filtered := filteredMemoryGuide(body, paths)
		if strings.Contains(filtered, "[Managed]") || !strings.Contains(filtered, "[Other]") {
			t.Fatalf("incorrect reference filtering for %s: %s", link, filtered)
		}
	}
}

func projectMemoryHTTP(t *testing.T, s *Store, action string, q map[string]any, want int) map[string]any {
	t.Helper()
	raw, _ := json.Marshal(q)
	req := httptest.NewRequest(http.MethodPost, "http://localhost:18787/api/v1/project-memory?action="+action, bytes.NewReader(raw))
	req.AddCookie(&http.Cookie{Name: "swarm_session", Value: "fixture-token"})
	req.Header.Set("Origin", "http://localhost:18787")
	req.Header.Set("X-Swarm-CSRF", "fixture-token")
	w := httptest.NewRecorder()
	newWebHandler(s, "localhost:18787", "fixture-token").ServeHTTP(w, req)
	if w.Code != want {
		t.Fatalf("%s: status %d, expected %d: %s", action, w.Code, want, w.Body.String())
	}
	var out map[string]any
	if err := json.Unmarshal(w.Body.Bytes(), &out); err != nil {
		t.Fatal(err, w.Body.String())
	}
	return out
}

func TestProjectMemoryPublicLifecycle(t *testing.T) {
	s := storeTest(t)
	source, r := projectMemorySourceFixture(t, s)
	q := map[string]any{"schema_version": 1, "event_id": newID("memory-"), "expected_revision": source.Revision, "work": source.ID, "id": "lesson", "source_retex": r.ID, "kind": "procedure", "body": "Vérifier le résultat observable après chaque correction <source>.", "conditions": "Correction sur le parcours concerné ; revalider les preuves courantes.", "reason": "Réutiliser une expérience vérifiée"}
	p := projectMemoryHTTP(t, s, "admit-preview", q, 200)
	unchanged, _ := s.get(source.ID)
	if unchanged.Revision != source.Revision {
		t.Fatal("preview mutated source")
	}
	q["preview_token"] = p["preview_token"]
	applied := projectMemoryHTTP(t, s, "admit", q, 200)
	replayed := projectMemoryHTTP(t, s, "admit", q, 200)
	if applied["revision"] != replayed["revision"] {
		t.Fatal("replay wrote twice")
	}
	entries := applied["project_memory"].([]any)
	versions := entries[0].(map[string]any)["versions"].([]any)
	v := versions[0].(map[string]any)
	ref := map[string]any{"work": source.ID, "id": "lesson", "version": 1, "sha256": v["sha256"]}
	destination, launch := setupAgent(t, s)
	beforeContract := reviewContract(&destination.Tasks[0])
	selectQ := map[string]any{"schema_version": 1, "event_id": newID("memory-"), "expected_revision": destination.Revision, "work": destination.ID, "task": launch.TaskID, "selections": []any{ref}, "reason": "Même type de correction"}
	p = projectMemoryHTTP(t, s, "select-preview", selectQ, 200)
	if !strings.Contains(p["context"].(string), q["body"].(string)) {
		t.Fatal("selection preview lost content")
	}
	selectQ["preview_token"] = p["preview_token"]
	projectMemoryHTTP(t, s, "select", selectQ, 200)
	destination, _ = s.get(destination.ID)
	if reviewContract(&destination.Tasks[0]) == beforeContract || destination.Tasks[0].MemoryDecision == nil || destination.Tasks[0].MemoryDecision.Reason != "Même type de correction" {
		t.Fatal("selection decision or contract binding lost")
	}
	launch.Revision = destination.Revision
	a, created, err := s.prepare(destination.ID, launch)
	if err != nil || !created {
		t.Fatal(err)
	}
	if !strings.Contains(a.Prompt, q["body"].(string)) || !strings.Contains(a.Prompt, q["conditions"].(string)) {
		t.Fatal("worker did not receive the selected version")
	}
	withdraw := map[string]any{"schema_version": 1, "event_id": newID("memory-"), "expected_revision": applied["revision"], "work": source.ID, "id": "lesson", "expected_sha256": v["sha256"], "reason": "Conditions devenues invalides"}
	p = projectMemoryHTTP(t, s, "withdraw-preview", withdraw, 200)
	if len(p["affected_attempts"].([]any)) != 1 {
		t.Fatal("withdrawal hid the prepared recipient")
	}
	withdraw["preview_token"] = p["preview_token"]
	projectMemoryHTTP(t, s, "withdraw", withdraw, 200)
	stored, err := s.agent(a.ID)
	if err != nil || stored.Prompt != a.Prompt {
		t.Fatal("withdrawal rewrote historical context", err)
	}
	if err = s.finishAgent(a, "interrupted", "Fin de fixture", nil); err != nil {
		t.Fatal(err)
	}
	destination, _ = s.get(destination.ID)
	selectQ["expected_revision"], selectQ["event_id"] = destination.Revision, newID("memory-")
	projectMemoryHTTP(t, s, "select-preview", selectQ, 400)
}

func TestProjectMemoryAdmissionValidation(t *testing.T) {
	s := storeTest(t)
	w, r := projectMemorySourceFixture(t, s)
	base := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: w.Revision, Work: w.ID, ID: "validation", SourceRetex: r.ID, Kind: "procedure", Body: "Vérifier le résultat.", Conditions: "Même contexte.", Reason: "Retour sourcé"}
	for _, test := range []struct {
		name   string
		change func(*ProjectMemoryRequest)
	}{
		{"empty", func(q *ProjectMemoryRequest) { q.Body = "" }},
		{"conditions", func(q *ProjectMemoryRequest) { q.Conditions = "" }},
		{"kind", func(q *ProjectMemoryRequest) { q.Kind = "unsupported" }},
		{"secret", func(q *ProjectMemoryRequest) { q.Body = "api_key=sk-abcdefghijklmnopqrstuvwxyz0123456789" }},
		{"utf8", func(q *ProjectMemoryRequest) { q.Body = string([]byte{0xff}) }},
		{"capacity", func(q *ProjectMemoryRequest) { q.Body = strings.Repeat("x", projectMemoryTransportBytes) }},
	} {
		t.Run(test.name, func(t *testing.T) {
			q := base
			test.change(&q)
			if _, err := s.projectMemoryOperation("admit-preview", q); err == nil {
				t.Fatal("invalid admission accepted")
			}
		})
	}
	raw, _ := json.Marshal(base)
	var request map[string]any
	json.Unmarshal(raw, &request)
	request["author"] = "admin"
	projectMemoryHTTP(t, s, "admit-preview", request, 400)
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision || len(after.ProjectMemory) != 0 {
		t.Fatal("invalid input mutated work")
	}
}
