package main

import (
	"bytes"
	"database/sql"
	"path/filepath"
	"strings"
	"testing"
)

func TestInspectionCLIRefusesImplicitStorageUpgrade(t *testing.T) {
	s, w := storeTest(t), Work{}
	w = createTest(t, s)
	if _, err := s.db.Exec("PRAGMA user_version=21"); err != nil {
		t.Fatal(err)
	}
	for _, args := range [][]string{{"work", "show", w.ID}, {"work", "list"}, {"planning", "show", w.ID}, {"mission", "status", w.ID}, {"agent", "list", w.ID}, {"agent", "logs", "absent"}, {"prepare", "list"}, {"providers", "show"}} {
		var out, errOut bytes.Buffer
		code := run(append([]string{"--root", s.root, "--json"}, args...), &out, &errOut)
		if code != 2 || !strings.Contains(out.String()+errOut.String(), "storage_upgrade_required") {
			t.Fatalf("%v: code%d out%s err%s", args, code, out.String(), errOut.String())
		}
		var version int
		if err := s.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != 21 {
			t.Fatalf("inspection migrated storage: %d %v", version, err)
		}
	}
	backups, _ := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v22-*.db"))
	if len(backups) != 0 {
		t.Fatal("inspection produced migration backup")
	}
	var out, errOut bytes.Buffer
	if code := run([]string{"--root", s.root, "init"}, &out, &errOut); code != 0 {
		t.Fatal(code, out.String(), errOut.String())
	}
	var version int
	if err := s.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion {
		t.Fatal(version, err)
	}
	out.Reset()
	errOut.Reset()
	if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &out, &errOut); code != 0 {
		t.Fatal(code, errOut.String())
	}
}

func TestInspectionDoesNotCreateMissingStore(t *testing.T) {
	root := t.TempDir()
	if s, err := openStoreWithMigration(root, false, false); err == nil {
		s.db.Close()
		t.Fatal("missing store created")
	}
	db, err := sql.Open("sqlite", "file:"+filepath.Join(root, ".swarm", "state.db")+"?mode=ro")
	if err == nil {
		defer db.Close()
		if err = db.Ping(); err == nil {
			t.Fatal("database exists")
		}
	}
}
