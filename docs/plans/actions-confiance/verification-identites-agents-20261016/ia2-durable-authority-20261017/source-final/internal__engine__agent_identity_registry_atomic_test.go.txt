//go:build linux

package engine

import (
	"os"
	"path/filepath"
	"testing"
)

func TestAgentIdentityRegistryAtomicWriteRequiresSyncableDirectory(t *testing.T) {
	if os.Geteuid() == 0 {
		t.Fatal("this permission reproducer requires an unprivileged test account")
	}
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")
	if err := os.Chmod(dir, 0300); err != nil {
		t.Fatal(err)
	}
	defer os.Chmod(dir, 0700)
	// Creation/rename can succeed here, but opening the directory for fsync
	// cannot. A successful durable write would therefore be an unsupported claim.
	if err := atomicWrite(path, []byte("new-state")); err == nil {
		t.Fatal("write reported success without opening a directory that can be synchronized")
	}
	if _, err := os.Stat(path); !os.IsNotExist(err) {
		t.Fatalf("failed durability precondition mutated target: %v", err)
	}
}

func TestAgentIdentityRegistryAtomicWritePreservesReplacementContract(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "state.json")
	for _, value := range []string{"initial", "replacement"} {
		if err := atomicWrite(path, []byte(value)); err != nil {
			t.Fatal(err)
		}
		raw, err := os.ReadFile(path)
		if err != nil || string(raw) != value {
			t.Fatalf("wrong durable replacement: %q %v", raw, err)
		}
		info, err := os.Stat(path)
		if err != nil || info.Mode().Perm() != 0600 {
			t.Fatalf("private file mode lost: %v %v", info, err)
		}
	}
	entries, err := os.ReadDir(dir)
	if err != nil || len(entries) != 1 || entries[0].Name() != "state.json" {
		t.Fatalf("temporary files leaked: %v %v", entries, err)
	}
}
