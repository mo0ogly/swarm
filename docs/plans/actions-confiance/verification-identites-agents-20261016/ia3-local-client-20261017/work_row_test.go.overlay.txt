package engine

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"reflect"
	"testing"
)

func TestWorkRowStoreGetRejectsInvalidRows(t *testing.T) {
	for _, scenario := range []string{"identity", "revision", "schema", "null", "invalid-utf8"} {
		t.Run(scenario, func(t *testing.T) {
			db, err := sql.Open("sqlite", ":memory:")
			if err != nil {
				t.Fatal(err)
			}
			defer db.Close()
			db.SetMaxOpenConns(1)
			if _, err := db.Exec("CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB)"); err != nil {
				t.Fatal(err)
			}
			w := Work{Schema: 1, ID: "source", Revision: 1, Title: "Stored source"}
			columnRevision := w.Revision
			switch scenario {
			case "identity":
				w.ID = "foreign"
			case "revision":
				columnRevision++
			case "schema":
				w.Schema = 2
			}
			raw, _ := json.Marshal(w)
			if scenario == "null" {
				raw = []byte("null")
			}
			if scenario == "invalid-utf8" {
				raw = bytes.Replace(raw, []byte("Stored"), []byte{0xff}, 1)
			}
			if _, err := db.Exec("INSERT INTO works(id,revision,body) VALUES(?,?,?)", "source", columnRevision, raw); err != nil {
				t.Fatal(err)
			}
			s := &Store{db: db}
			if got, err := s.get("source"); err == nil || !reflect.DeepEqual(got, Work{}) {
				t.Fatal("Store.get returned an inconsistent row", got.ID, got.Revision, err)
			}
		})
	}
}

func TestWorkRowStoreListRejectsPartialResult(t *testing.T) {
	s := storeTest(t)
	older := createTest(t, s)
	_ = createTest(t, s) // Valid row is visited first by newest-first list.
	if _, err := s.db.Exec("UPDATE works SET revision=revision+1 WHERE id=?", older.ID); err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	if got, err := s.list(); err == nil || got != nil {
		t.Fatal("Store.list returned a partial or inconsistent collection", got, err)
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("list refusal changed storage or leaked its cursor")
	}
}

func TestWorkRowCommonMutationRejectsBeforeCallbacks(t *testing.T) {
	for _, scenario := range []string{"fresh", "replay"} {
		t.Run(scenario, func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			events, err := s.events(w.ID)
			if err != nil || len(events) != 1 {
				t.Fatal("creation fixture", events, err)
			}
			id, kind, event, revision, request := w.ID, "fixture.check", newID("e-"), w.Revision, []byte(`{}`)
			if scenario == "replay" {
				id, revision, event = "", 0, events[0].ID
				if err := s.db.QueryRow("SELECT kind,request FROM events WHERE id=?", event).Scan(&kind, &request); err != nil {
					t.Fatal(err)
				}
			}
			wrong := w
			wrong.ID = "foreign-source"
			raw, _ := json.Marshal(wrong)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
				t.Fatal(err)
			}
			before := recoverySnapshotTest(t, s)
			called, hooked := false, false
			got, err := s.mutateWithHookOnce(id, kind, event, revision, request, func(*Work) error {
				called = true
				return nil
			}, func(*sql.Tx, *Work) error {
				hooked = true
				return nil
			})
			if err == nil || called || hooked || !reflect.DeepEqual(got, Work{}) {
				t.Fatal("inconsistent mutation/replay reached a callback or returned data", scenario, called, hooked, got.ID, err)
			}
			if before != recoverySnapshotTest(t, s) {
				t.Fatal("mutation refusal changed state")
			}
		})
	}
}

func TestWorkRowPublicReadersRefuseIncoherentWork(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	if _, err := s.db.Exec("UPDATE works SET revision=revision+1 WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	for _, args := range [][]string{{"work", "show", w.ID}, {"work", "list"}} {
		var stdout, stderr bytes.Buffer
		if code := run(append([]string{"--root", s.root, "--json"}, args...), &stdout, &stderr); code != 2 || stdout.Len() != 0 || stderr.Len() == 0 {
			t.Fatal("inconsistent work published by CLI", args, code, stdout.String(), stderr.String())
		}
	}
	request := httptest.NewRequest(http.MethodGet, "http://localhost:18787/api/v1/works", nil)
	request.AddCookie(&http.Cookie{Name: "swarm_session", Value: "fixture-token"})
	response := httptest.NewRecorder()
	newWebHandler(s, "localhost:18787", "fixture-token").ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest {
		t.Fatal("inconsistent work published by HTTP", response.Code, response.Body.String())
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("public inspection mutated state")
	}
	if _, err := s.db.Exec("UPDATE works SET revision=? WHERE id=?", w.Revision, w.ID); err != nil {
		t.Fatal(err)
	}
	var stdout, stderr bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &stdout, &stderr); code != 0 || !bytes.Contains(stdout.Bytes(), []byte(w.ID)) {
		t.Fatal("valid CLI inspection failed after fixture recovery", code, stderr.String())
	}
}

func TestWorkRowReplayReturnsCurrentConsistentState(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	events, err := s.events(w.ID)
	if err != nil || len(events) != 1 {
		t.Fatal(events, err)
	}
	var kind string
	var raw []byte
	if err := s.db.QueryRow("SELECT kind,request FROM events WHERE id=?", events[0].ID).Scan(&kind, &raw); err != nil {
		t.Fatal(err)
	}
	current, err := s.executeRequest(w.ID, "work.update", Request{Schema: 1, EventID: newID("e-"), Revision: w.Revision, Title: "Current title after creation"})
	if err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	called := false
	got, err := s.mutateWithHookOnce("", kind, events[0].ID, 0, raw, func(*Work) error { called = true; return nil }, func(*sql.Tx, *Work) error { called = true; return nil })
	if err != nil || called || !reflect.DeepEqual(got, current) || before != recoverySnapshotTest(t, s) {
		t.Fatal("exact replay lost current state or repeated a mutation", got.Revision, current.Revision, called, err)
	}
}

func TestWorkRowReadEnvelopePreservesLegacyAndCauses(t *testing.T) {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	db.SetMaxOpenConns(1)
	if _, err := db.Exec("CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB)"); err != nil {
		t.Fatal(err)
	}
	raw := []byte(`{"schema_version":1,"id":"legacy","revision":3,"title":"Historical source","tasks":[],"future_optional":{"value":"retained"}}`)
	if _, err := db.Exec("INSERT INTO works(id,revision,body) VALUES(?,?,?)", "legacy", 3, raw); err != nil {
		t.Fatal(err)
	}
	s := &Store{db: db}
	got, err := s.get("legacy")
	if err != nil || got.ID != "legacy" || got.Revision != 3 || got.Title != "Historical source" || got.Tasks == nil {
		t.Fatal("legacy optional fields caused a read migration or rejection", got, err)
	}
	var retained []byte
	if err := db.QueryRow("SELECT body FROM works WHERE id=?", "legacy").Scan(&retained); err != nil || !bytes.Equal(raw, retained) {
		t.Fatal("read rewrote optional fields", err)
	}
	if got, err := s.get("absent"); !errors.Is(err, sql.ErrNoRows) || !reflect.DeepEqual(got, Work{}) {
		t.Fatal("missing row cause lost", got, err)
	}
	if got, err := s.get("../unsafe"); err == nil || !reflect.DeepEqual(got, Work{}) {
		t.Fatal("unsafe key became a work", got, err)
	}
}
