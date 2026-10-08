//go:build linux

package engine

import (
	"errors"
	"io/fs"
	"os"
	"path/filepath"
	"testing"
	"time"
)

// D02 exercises only temporary project roots. It never opens or copies the
// repository's live .swarm state.
func TestGraphDeliveryD02Migration(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	if _, err := s.db.Exec("DROP TABLE automation_external_audit; DROP TABLE automation_external_events; PRAGMA user_version=26"); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}

	_, err := openStoreWithMigration(s.root, false, false)
	var command *CommandError
	if !errors.As(err, &command) || command.Code != "storage_upgrade_required" {
		t.Fatalf("inspection silently migrated v26 storage: %v", err)
	}

	migrated, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer migrated.db.Close()
	var version, tables int
	if err = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil {
		t.Fatal(err)
	}
	if err = migrated.db.QueryRow("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('automation_external_events','automation_external_audit')").Scan(&tables); err != nil {
		t.Fatal(err)
	}
	got, err := migrated.get(w.ID)
	if err != nil || version != schemaVersion || tables != 2 || got.Title != w.Title {
		t.Fatalf("v26 migration lost state: version=%d tables=%d work=%+v err=%v", version, tables, got, err)
	}
	backups, err := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v27-*.db"))
	if err != nil || len(backups) != 1 {
		t.Fatalf("migration backup missing: %v %v", backups, err)
	}
	info, err := os.Stat(backups[0])
	if err != nil || info.Mode().Perm() != 0600 || info.Size() == 0 {
		t.Fatalf("migration backup unsafe: info=%v err=%v", info, err)
	}
}

func TestGraphDeliveryD02ArchiveCompatibility(t *testing.T) {
	s, w := automationC01Fixture(t)
	clock := func() time.Time { return automationC02Time(t, "2026-01-01T00:00:00Z") }
	schedule, err := s.createAutomationSchedule(automationC02Create(w, "d02-archive", "once", "2026-02-01T12:00:00", "", "UTC", 1), clock)
	if err != nil {
		t.Fatal(err)
	}
	schedule, err = s.setAutomationScheduleState(schedule.ScheduleID, "enabled", schedule.Revision, clock)
	if err != nil {
		t.Fatal(err)
	}
	archiveV2 := filepath.Join(t.TempDir(), "automation-v2.zip")
	if err = s.export(w.ID, archiveV2); err != nil {
		t.Fatal(err)
	}
	destination := storeTest(t)
	imported, err := destination.importBundle(archiveV2)
	if err != nil || imported.ID != w.ID {
		t.Fatalf("schema 2 archive import failed: work=%+v err=%v", imported, err)
	}
	importedSchedule, err := destination.automationSchedule(schedule.ScheduleID)
	if err != nil || importedSchedule.State != "disabled" || importedSchedule.TargetWorkID != w.ID {
		t.Fatalf("imported program was not retained disabled: %+v err=%v", importedSchedule, err)
	}
	if _, err = destination.importBundle(archiveV2); err == nil {
		t.Fatal("duplicate archive overwrote an existing mission")
	}
	current, err := destination.get(w.ID)
	if err != nil || current.Revision != w.Revision {
		t.Fatalf("duplicate rejection changed imported mission: %+v err=%v", current, err)
	}

	legacySource := storeTest(t)
	legacyWork := lifecycleFixture(t, legacySource)
	archiveV1 := filepath.Join(t.TempDir(), "mission-v1.zip")
	if err = legacySource.export(legacyWork.ID, archiveV1); err != nil {
		t.Fatal(err)
	}
	legacyDestination := storeTest(t)
	legacyImported, err := legacyDestination.importBundle(archiveV1)
	if err != nil || legacyImported.ID != legacyWork.ID || legacyImported.Title != legacyWork.Title {
		t.Fatalf("schema 1 archive compatibility lost: %+v err=%v", legacyImported, err)
	}
}

func TestGraphDeliveryD02BackupRestoreSafety(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	config := filepath.Join(s.root, ".swarm", "providers.json")
	if err := os.WriteFile(config, []byte("{\"schema_version\":1,\"providers\":[]}"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}

	snapshot := t.TempDir()
	if err := copyD02Tree(filepath.Join(s.root, ".swarm"), filepath.Join(snapshot, ".swarm")); err != nil {
		t.Fatal(err)
	}
	changed, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	second := createTest(t, changed)
	if err = changed.db.Close(); err != nil {
		t.Fatal(err)
	}

	rollbackRoot := t.TempDir()
	if err = copyD02Tree(filepath.Join(snapshot, ".swarm"), filepath.Join(rollbackRoot, ".swarm")); err != nil {
		t.Fatal(err)
	}
	restored, err := openStoreWithMigration(rollbackRoot, false, false)
	if err != nil {
		t.Fatal(err)
	}
	got, err := restored.get(w.ID)
	if err != nil || got.Title != w.Title {
		t.Fatalf("rollback lost saved mission: %+v err=%v", got, err)
	}
	if _, err = restored.get(second.ID); err == nil {
		t.Fatal("rollback retained post-backup mission")
	}
	raw, err := os.ReadFile(filepath.Join(rollbackRoot, ".swarm", "providers.json"))
	if err != nil || string(raw) != "{\"schema_version\":1,\"providers\":[]}" {
		t.Fatalf("rollback lost companion configuration: %q err=%v", raw, err)
	}
	if _, err = restored.db.Exec("PRAGMA user_version=28"); err != nil {
		t.Fatal(err)
	}
	if err = restored.db.Close(); err != nil {
		t.Fatal(err)
	}
	if _, err = openStore(rollbackRoot, false); err == nil {
		t.Fatal("future storage schema was accepted")
	}
}

func copyD02Tree(source, destination string) error {
	return filepath.WalkDir(source, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		relative, err := filepath.Rel(source, path)
		if err != nil {
			return err
		}
		target := filepath.Join(destination, relative)
		info, err := entry.Info()
		if err != nil {
			return err
		}
		if info.Mode()&os.ModeSymlink != 0 {
			return errors.New("backup source contains a symbolic link")
		}
		if entry.IsDir() {
			return os.MkdirAll(target, info.Mode().Perm())
		}
		if !info.Mode().IsRegular() {
			return errors.New("backup source contains a non-regular file")
		}
		raw, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		return os.WriteFile(target, raw, info.Mode().Perm())
	})
}
