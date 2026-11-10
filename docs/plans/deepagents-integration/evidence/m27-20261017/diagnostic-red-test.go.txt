//go:build linux

package engine

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"reflect"
	"strings"
	"testing"
)

func TestMemoryWorkRejectsInvalidSourceRows(t *testing.T) {
	for _, scenario := range []string{"identity", "revision", "zero-revision", "schema", "null", "array", "malformed", "invalid-utf8"} {
		t.Run(scenario, func(t *testing.T) {
			// A minimal SQL reader fixture, not a migrated mission. Production
			// identity triggers are never removed to inject malformed bytes.
			db, err := sql.Open("sqlite", ":memory:")
			if err != nil {
				t.Fatal(err)
			}
			defer db.Close()
			db.SetMaxOpenConns(1)
			if _, err := db.Exec("CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB)"); err != nil {
				t.Fatal(err)
			}
			valid := Work{Schema: 1, ID: "source", Revision: 7, Title: "Mémoire conservée"}
			value := valid
			revision := valid.Revision
			switch scenario {
			case "identity":
				value.ID = "other"
			case "revision":
				revision++
			case "zero-revision":
				value.Revision, revision = 0, 0
			case "schema":
				value.Schema = 2
			}
			raw, _ := json.Marshal(value)
			switch scenario {
			case "null":
				raw = []byte("null")
			case "array":
				raw = []byte("[]")
			case "malformed":
				raw = []byte(`{"schema_version":1,"id":"source",`)
			case "invalid-utf8":
				raw = bytes.Replace(raw, []byte("Mémoire"), []byte{0xff}, 1)
			}
			if _, err := db.Exec("INSERT INTO works(id,revision,body) VALUES(?,?,?)", valid.ID, revision, raw); err != nil {
				t.Fatal(err)
			}
			got, err := memoryWork(db, valid.ID)
			if err == nil || !reflect.DeepEqual(got, Work{}) {
				t.Fatal("inconsistent source row returned a usable Work", scenario, got.ID, got.Revision, err)
			}
			s := &Store{db: db, root: t.TempDir()}
			if paths, err := s.memoryLegacyPaths(db); err == nil || paths != nil {
				t.Fatal("legacy guide accepted an inconsistent source row", paths, err)
			}
			var retained []byte
			var storedRevision int
			if err := db.QueryRow("SELECT revision,body FROM works WHERE id=?", valid.ID).Scan(&storedRevision, &retained); err != nil || storedRevision != revision || !bytes.Equal(raw, retained) {
				t.Fatal("rejected read changed its source", err)
			}
			clean, _ := json.Marshal(valid)
			if _, err := db.Exec("UPDATE works SET revision=?,body=? WHERE id=?", valid.Revision, clean, valid.ID); err != nil {
				t.Fatal(err)
			}
			if got, err := memoryWork(db, valid.ID); err != nil || !reflect.DeepEqual(got, valid) {
				t.Fatal("valid source cannot be read after recovery", err)
			}
			if paths, err := s.memoryLegacyPaths(db); err != nil || len(paths) != 0 {
				t.Fatal("valid source guide cannot be read after recovery", paths, err)
			}
		})
	}
}

func TestMemoryWorkDoesNotReturnPartialRowsAndClosesReaders(t *testing.T) {
	s := storeTest(t)
	older, _, _ := admittedMemoryFixture(t, s)
	newer, _, _ := admittedMemoryFixture(t, s)
	// Catalogue scans newest first: a valid row precedes the invalid older
	// row. Guide scans insertion order: a valid older row precedes newer.
	for _, id := range []string{older.ID, newer.ID} {
		var raw []byte
		var revision int
		if err := s.db.QueryRow("SELECT revision,body FROM works WHERE id=?", id).Scan(&revision, &raw); err != nil {
			t.Fatal(err)
		}
		if _, err := s.db.Exec("UPDATE works SET revision=revision+1 WHERE id=?", id); err != nil {
			t.Fatal(err)
		}
		if works, err := memoryWorks(s.db); err == nil || works != nil {
			t.Fatal("catalogue returned a partial collection", works, err)
		}
		if paths, err := s.memoryLegacyPaths(s.db); err == nil || paths != nil {
			t.Fatal("guide returned partial paths", paths, err)
		}
		// One database connection: successful readback also proves both error
		// paths released their cursor instead of blocking the next operation.
		var retained []byte
		if err := s.db.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&retained); err != nil || !bytes.Equal(raw, retained) {
			t.Fatal("row error changed body or left its cursor borrowed", err)
		}
		if _, err := s.db.Exec("UPDATE works SET revision=? WHERE id=?", revision, id); err != nil {
			t.Fatal(err)
		}
	}
	works, err := memoryWorks(s.db)
	if err != nil || len(works) != 2 || works[0].ID != newer.ID || works[1].ID != older.ID {
		t.Fatal("valid catalogue order changed", works, err)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	defer tx.Rollback()
	if got, err := memoryWork(tx, older.ID); err != nil || got.ID != older.ID {
		t.Fatal("transactional source reader", err)
	}
	if _, err := s.memoryLegacyPaths(tx); err != nil {
		t.Fatal("transactional guide reader", err)
	}
}

func TestMemoryWorkMissingSourceAndSQLFailures(t *testing.T) {
	s := storeTest(t)
	if got, err := memoryWork(s.db, "absent"); !errors.Is(err, sql.ErrNoRows) || !reflect.DeepEqual(got, Work{}) {
		t.Fatal("missing row cause or empty result lost", got, err)
	}
	if got, err := memoryWork(s.db, "../unsafe"); err == nil || !reflect.DeepEqual(got, Work{}) {
		t.Fatal("unsafe source ID admitted", got, err)
	}
	if got, err := memoryWorks(s.db); err != nil || got == nil || len(got) != 0 {
		t.Fatal("empty catalogue is not a genuine successful empty result", got, err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	if got, err := memoryWork(s.db, "absent"); err == nil || !reflect.DeepEqual(got, Work{}) {
		t.Fatal("source SQL failure became success", got, err)
	}
	if got, err := memoryWorks(s.db); err == nil || got != nil {
		t.Fatal("catalogue SQL failure became empty success", got, err)
	}
	if got, err := s.memoryLegacyPaths(s.db); err == nil || got != nil {
		t.Fatal("guide SQL failure became empty success", got, err)
	}
}

func TestMemoryWorkRefusalIdentifiesSafeStoredSource(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	if _, err := s.db.Exec("UPDATE works SET revision=revision+1 WHERE id=?", w.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := memoryWork(s.db, w.ID); err == nil || !strings.Contains(err.Error(), w.ID) {
		t.Fatal("source refusal does not identify its stored row", err)
	}
}

func TestMemoryWorkRefusalDoesNotEchoInvalidStoredKey(t *testing.T) {
	db, err := sql.Open("sqlite", ":memory:")
	if err != nil {
		t.Fatal(err)
	}
	defer db.Close()
	db.SetMaxOpenConns(1)
	if _, err := db.Exec("CREATE TABLE works(id TEXT PRIMARY KEY,revision INTEGER,body BLOB)"); err != nil {
		t.Fatal(err)
	}
	key := "unsafe/credential-private-value"
	raw, _ := json.Marshal(Work{Schema: 1, ID: key, Revision: 1})
	if _, err := db.Exec("INSERT INTO works(id,revision,body) VALUES(?,?,?)", key, 1, raw); err != nil {
		t.Fatal(err)
	}
	if got, err := memoryWorks(db); err == nil || got != nil || strings.Contains(err.Error(), "credential-private-value") {
		t.Fatal("invalid stored identifier was published as a diagnostic", got, err)
	}
}

func TestMemoryWorkIdentityMismatchRefusesPublicCatalog(t *testing.T) {
	s := storeTest(t)
	w, _, _ := admittedMemoryFixture(t, s)
	original, _ := json.Marshal(w)
	w.ID = "foreign-source"
	raw, _ := json.Marshal(w)
	// Valid JSON corruption in an isolated migrated fixture. The SQL key
	// remains unchanged; no trigger is disabled or live database modified.
	var id string
	if err := s.db.QueryRow("SELECT id FROM works LIMIT 1").Scan(&id); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, id); err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	var stdout, stderr bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "memory", "list"}, &stdout, &stderr)
	if code != 2 || stdout.Len() != 0 || stderr.Len() == 0 {
		t.Fatal("CLI catalog published an inconsistent source", code, stdout.String(), stderr.String())
	}
	request := httptest.NewRequest(http.MethodGet, "http://localhost:18787/api/v1/project-memory", nil)
	request.AddCookie(&http.Cookie{Name: "swarm_session", Value: "fixture-token"})
	response := httptest.NewRecorder()
	newWebHandler(s, "localhost:18787", "fixture-token").ServeHTTP(response, request)
	if response.Code != http.StatusBadRequest || strings.Contains(response.Body.String(), "Vérifier la réception avant reprise") {
		t.Fatal("HTTP catalog published an inconsistent source", response.Code, response.Body.String())
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("catalog refusal mutated storage")
	}
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", original, id); err != nil {
		t.Fatal(err)
	}
	stdout.Reset()
	stderr.Reset()
	if code := run([]string{"--root", s.root, "--json", "memory", "list"}, &stdout, &stderr); code != 0 || !strings.Contains(stdout.String(), "Vérifier la réception avant reprise") {
		t.Fatal("public catalog cannot recover with a consistent source", code, stderr.String())
	}
}
