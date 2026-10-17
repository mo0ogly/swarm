//go:build linux

package enrollmentconfig

import (
	"context"
	"errors"
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
	"time"

	"swarm.local/companion/internal/identitycontract"
)

func fixtureCoordinator(t *testing.T) (*Coordinator, string, string) {
	t.Helper()
	configDir, witnessDir := t.TempDir(), t.TempDir()
	for _, dir := range []string{configDir, witnessDir} {
		if err := os.Chmod(dir, 0700); err != nil {
			t.Fatal(err)
		}
	}
	pins := fixturePins()
	c, err := OpenCoordinated(configDir, witnessDir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := c.Close(); err != nil {
			t.Error(err)
		}
	})
	return c, configDir, witnessDir
}
func coordinatorCreate(t *testing.T, c *Coordinator) View {
	t.Helper()
	v, err := c.Create(context.Background(), fixtureChange("initial", Head{}), fixtureAt())
	if err != nil {
		t.Fatal(err)
	}
	return v
}

func TestCoordinatedBootstrapPreviewApplyAndReopen(t *testing.T) {
	c, dir, wdir := fixtureCoordinator(t)
	ctx := context.Background()
	initial := fixtureChange("initial", Head{})
	p, err := c.Preview(ctx, initial, fixtureAt())
	if err != nil || p.Base != (Head{}) {
		t.Fatalf("preview: %#v %v", p, err)
	}
	for _, path := range []string{filepath.Join(dir, "config.json"), filepath.Join(wdir, "witness.json")} {
		if _, err = os.Stat(path); !os.IsNotExist(err) {
			t.Fatalf("preview wrote %s", path)
		}
	}
	first := coordinatorCreate(t, c)
	if first.Head != p.Next.Head {
		t.Fatal("preview/create diverged")
	}
	next, err := c.Apply(ctx, fixtureChange("next", first.Head), fixtureAt().Add(time.Second))
	if err != nil {
		t.Fatal(err)
	}
	pins := fixturePins()
	other, err := OpenCoordinated(dir, wdir, uint32(os.Geteuid()), pins, c.config.authorize)
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	got, err := other.Read(ctx)
	if err != nil || !reflect.DeepEqual(got, next) {
		t.Fatalf("reopen: %#v %v", got, err)
	}
	got.Snapshot.History[0].Change.Reason = "caller mutation"
	got, err = other.Read(ctx)
	if err != nil || got.Snapshot.History[0].Change.Reason == "caller mutation" {
		t.Fatal("view aliases durable state")
	}
}

func TestCoordinatedRejectsMissingWitnessAndSingleStoreRollback(t *testing.T) {
	c, dir, wdir := fixtureCoordinator(t)
	ctx := context.Background()
	first := coordinatorCreate(t, c)
	oldConfig, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	oldWitness, err := os.ReadFile(filepath.Join(wdir, "witness.json"))
	if err != nil {
		t.Fatal(err)
	}
	_, err = c.Apply(ctx, fixtureChange("next", first.Head), fixtureAt())
	if err != nil {
		t.Fatal(err)
	}
	latestWitness, err := os.ReadFile(filepath.Join(wdir, "witness.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(wdir, "witness.json"), oldWitness, 0600); err != nil {
		t.Fatal(err)
	}
	v, err := c.Read(ctx)
	assertZero(t, v, err, ErrHead)
	if err = os.WriteFile(filepath.Join(wdir, "witness.json"), latestWitness, 0600); err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(dir, "config.json"), oldConfig, 0600); err != nil {
		t.Fatal(err)
	}
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrHead)
	if _, err = c.Preview(ctx, fixtureChange("after-rollback", first.Head), fixtureAt()); !errors.Is(err, ErrHead) {
		t.Fatalf("preview adopted rollback: %v", err)
	}
	// Restoring both stores defeats this local boundary; preserve the limitation.
	if err = os.WriteFile(filepath.Join(wdir, "witness.json"), oldWitness, 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = c.Read(ctx); err != nil {
		t.Fatal(err)
	}
	if err = os.Remove(filepath.Join(wdir, "witness.json")); err != nil {
		t.Fatal(err)
	}
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrWitnessMissing)
	v, err = c.Create(ctx, fixtureChange("new-root", Head{}), fixtureAt())
	assertZero(t, v, err, ErrWitnessMissing)
	if _, err = os.Stat(filepath.Join(wdir, "witness.json")); !os.IsNotExist(err) {
		t.Fatal("witness inferred from config")
	}
}

func TestCoordinatedPendingIntentBeforeConfigCanResume(t *testing.T) {
	c, dir, wdir := fixtureCoordinator(t)
	ctx := context.Background()
	original := c.witness.syncDir
	c.witness.syncDir = func() error { return errors.New("fixture intent fsync failure") }
	v, err := c.Create(ctx, fixtureChange("initial", Head{}), fixtureAt())
	assertZero(t, v, err, ErrIndeterminate)
	if _, err = os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatal("config written before durable intent")
	}
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrPending)
	v, err = c.Recover(ctx)
	assertZero(t, v, err, ErrIndeterminate)
	if _, err = os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatal("recovery skipped intent barrier")
	}
	c.witness.syncDir = original
	v, err = c.Recover(ctx)
	if err != nil || v.Head.Revision != 1 || len(v.Snapshot.History) != 1 {
		t.Fatalf("recovery: %#v %v", v, err)
	}
	if _, err = os.Stat(filepath.Join(wdir, "witness.json")); err != nil {
		t.Fatal(err)
	}
}

func TestCoordinatedConfigVisiblePendingBlocksUntilAuthorizedRecovery(t *testing.T) {
	c, dir, wdir := fixtureCoordinator(t)
	ctx := context.Background()
	first := coordinatorCreate(t, c)
	change := fixtureChange("next", first.Head)
	originalSync := c.config.syncDir
	c.config.syncDir = func() error { return errors.New("fixture config directory sync failure") }
	v, err := c.Apply(ctx, change, fixtureAt().Add(time.Second))
	assertZero(t, v, err, ErrIndeterminate)
	before, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	witnessBefore, err := os.ReadFile(filepath.Join(wdir, "witness.json"))
	if err != nil {
		t.Fatal(err)
	}
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrPending)
	if _, err = c.Preview(ctx, fixtureChange("another", first.Head), fixtureAt()); !errors.Is(err, ErrPending) {
		t.Fatal(err)
	}
	v, err = c.Apply(ctx, fixtureChange("another", first.Head), fixtureAt())
	assertZero(t, v, err, ErrPending)
	originalAuth := c.config.authorize
	denial := errors.New("fixture authority revoked")
	c.config.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return identitycontract.Principal{}, denial
	}
	v, err = c.Recover(ctx)
	assertZero(t, v, err, ErrUnauthorized)
	if !errors.Is(err, denial) {
		t.Fatal("cause lost")
	}
	after, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	witnessAfter, _ := os.ReadFile(filepath.Join(wdir, "witness.json"))
	if string(before) != string(after) || string(witnessBefore) != string(witnessAfter) {
		t.Fatal("denied recovery mutated files")
	}
	c.config.authorize = originalAuth
	c.config.syncDir = originalSync
	v, err = c.Apply(ctx, change, fixtureAt().Add(time.Hour))
	if err != nil || v.Head.Revision != 2 || len(v.Snapshot.History) != 2 || !v.Snapshot.History[1].At.Equal(fixtureAt().Add(time.Second)) {
		t.Fatalf("exact pending retry: %#v %v", v, err)
	}
}

func TestCoordinatedConfirmationUncertainReplaysWithoutNewEvent(t *testing.T) {
	c, _, _ := fixtureCoordinator(t)
	ctx := context.Background()
	first := coordinatorCreate(t, c)
	change := fixtureChange("next", first.Head)
	original := c.witness.syncDir
	calls := 0
	c.witness.syncDir = func() error {
		calls++
		if calls >= 2 {
			return errors.New("fixture confirmation fsync failure")
		}
		return original()
	}
	v, err := c.Apply(ctx, change, fixtureAt())
	assertZero(t, v, err, ErrIndeterminate)
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrIndeterminate)
	c.witness.syncDir = original
	v, err = c.Apply(ctx, change, fixtureAt().Add(time.Hour))
	if err != nil || len(v.Snapshot.History) != 2 {
		t.Fatalf("confirmation retry: %#v %v", v, err)
	}
	if _, err = c.Read(ctx); err != nil {
		t.Fatal(err)
	}
}

func TestCoordinatedConfigFailureBeforeRenameLeavesIntent(t *testing.T) {
	c, dir, _ := fixtureCoordinator(t)
	ctx := context.Background()
	auth := c.config.authorize
	c.config.authorize = func(ctx context.Context, s *Snapshot, change Change) (identitycontract.Principal, error) {
		if err := os.Chmod(dir, 0500); err != nil {
			return identitycontract.Principal{}, err
		}
		return auth(ctx, s, change)
	}
	v, err := c.Create(ctx, fixtureChange("initial", Head{}), fixtureAt())
	if !errors.Is(err, ErrPath) || !reflect.DeepEqual(v, View{}) {
		t.Fatalf("write failure: %#v %v", v, err)
	}
	if err = os.Chmod(dir, 0700); err != nil {
		t.Fatal(err)
	}
	c.config.authorize = auth
	v, err = c.Read(ctx)
	assertZero(t, v, err, ErrPending)
	v, err = c.Recover(ctx)
	if err != nil || v.Head.Revision != 1 {
		t.Fatalf("resume before rename: %#v %v", v, err)
	}
}

func TestCoordinatedRejectsUnrelatedHistoryDuringRecovery(t *testing.T) {
	c, dir, _ := fixtureCoordinator(t)
	ctx := context.Background()
	first := coordinatorCreate(t, c)
	original := c.config.syncDir
	c.config.syncDir = func() error { return errors.New("fixture sync failure") }
	_, err := c.Apply(ctx, fixtureChange("next", first.Head), fixtureAt())
	if !errors.Is(err, ErrIndeterminate) {
		t.Fatal(err)
	}
	c.config.syncDir = original
	raw, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	s, err := Decode(raw)
	if err != nil {
		t.Fatal(err)
	}
	s.History[1].Change.Reason = "different coherent history"
	raw, err = Encode(s)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(dir, "config.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	v, err := c.Recover(ctx)
	assertZero(t, v, err, ErrHead)
	if err = os.Remove(filepath.Join(dir, "config.json")); err != nil {
		t.Fatal(err)
	}
	v, err = c.Recover(ctx)
	assertZero(t, v, err, ErrNotConfigured)
}

func TestCoordinatedSeparateDirectoriesAndConcurrentHandles(t *testing.T) {
	c, dir, wdir := fixtureCoordinator(t)
	ctx := context.Background()
	pins := fixturePins()
	for _, path := range []string{dir, filepath.Join(dir, "nested")} {
		if path != dir {
			if err := os.Mkdir(path, 0700); err != nil {
				t.Fatal(err)
			}
		}
		other, err := OpenCoordinated(dir, path, uint32(os.Geteuid()), pins, c.config.authorize)
		if err == nil {
			other.Close()
			t.Fatal("same or nested witness accepted")
		}
	}
	first := coordinatorCreate(t, c)
	entered := make(chan struct{})
	release := make(chan struct{})
	var once sync.Once
	defer once.Do(func() { close(release) })
	c.config.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		close(entered)
		<-release
		return pins.Principal, nil
	}
	other, err := OpenCoordinated(dir, wdir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	done := make(chan error, 1)
	go func() { _, err := c.Apply(ctx, fixtureChange("winner", first.Head), fixtureAt()); done <- err }()
	<-entered
	v, err := other.Read(ctx)
	assertZero(t, v, err, ErrBusy)
	once.Do(func() { close(release) })
	if err = <-done; err != nil {
		t.Fatal(err)
	}
	v, err = other.Apply(ctx, fixtureChange("stale", first.Head), fixtureAt())
	assertZero(t, v, err, ErrConflict)
}

func TestCoordinatedCancellationAfterIntentKeepsRecoverableDecision(t *testing.T) {
	c, dir, _ := fixtureCoordinator(t)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	original := c.witness.syncDir
	c.witness.syncDir = func() error { err := original(); cancel(); return err }
	v, err := c.Create(ctx, fixtureChange("initial", Head{}), fixtureAt())
	assertZero(t, v, err, context.Canceled)
	if !errors.Is(err, ErrPending) {
		t.Fatalf("lost pending state: %v", err)
	}
	if _, err = os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatal("canceled before config wrote config")
	}
	c.witness.syncDir = original
	v, err = c.Recover(context.Background())
	if err != nil || v.Head.Revision != 1 || len(v.Snapshot.History) != 1 {
		t.Fatalf("fresh recovery: %#v %v", v, err)
	}
}

func TestCoordinatedWitnessSourceRejectsTamperAndUnsafeFiles(t *testing.T) {
	for _, name := range []string{"wrong-pins", "symlink", "permissions", "hardlink", "malformed"} {
		t.Run(name, func(t *testing.T) {
			c, _, wdir := fixtureCoordinator(t)
			coordinatorCreate(t, c)
			path := filepath.Join(wdir, "witness.json")
			cause := ErrPath
			switch name {
			case "wrong-pins":
				raw, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				w, err := decodeWitness(raw)
				if err != nil {
					t.Fatal(err)
				}
				// Use another valid context rather than failing on shape first.
				w.Pins.InstallationDigest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
				raw, err = encodeWitness(w)
				if err != nil {
					t.Fatal(err)
				}
				if err = os.WriteFile(path, raw, 0600); err != nil {
					t.Fatal(err)
				}
				cause = ErrPins
			case "symlink":
				raw, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				outside := filepath.Join(t.TempDir(), "outside.json")
				if err = os.WriteFile(outside, raw, 0600); err != nil {
					t.Fatal(err)
				}
				if err = os.Remove(path); err != nil {
					t.Fatal(err)
				}
				if err = os.Symlink(outside, path); err != nil {
					t.Fatal(err)
				}
			case "permissions":
				if err := os.Chmod(path, 0640); err != nil {
					t.Fatal(err)
				}
			case "hardlink":
				if err := os.Link(path, filepath.Join(wdir, "alias")); err != nil {
					t.Fatal(err)
				}
			case "malformed":
				if err := os.WriteFile(path, []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
				cause = ErrVersion
			}
			v, err := c.Read(context.Background())
			assertZero(t, v, err, cause)
		})
	}
}

func TestCoordinatedRawStoreCannotAdvanceWitness(t *testing.T) {
	c, _, _ := fixtureCoordinator(t)
	first := coordinatorCreate(t, c)
	if _, err := c.config.Apply(context.Background(), fixtureChange("outside-protocol", first.Head), fixtureAt()); err != nil {
		t.Fatal(err)
	}
	v, err := c.Read(context.Background())
	assertZero(t, v, err, ErrHead)
	v, err = c.Recover(context.Background())
	assertZero(t, v, err, ErrHead)
}

func TestCoordinatedStorageErrorIdentifiesFailedStage(t *testing.T) {
	c, _, _ := fixtureCoordinator(t)
	cause := errors.New("fixture I/O failure")
	c.witness.syncDir = func() error { return cause }
	v, err := c.Create(context.Background(), fixtureChange("initial", Head{}), fixtureAt())
	assertZero(t, v, err, ErrIndeterminate)
	if !errors.Is(err, cause) || !strings.Contains(err.Error(), "prepare witness") {
		t.Fatalf("missing failed stage or original cause: %v", err)
	}
}

func TestCoordinatedSubprocessInterruptedIntent(t *testing.T) {
	if os.Getenv("SWARM_WITNESS_FIXTURE_CHILD") == "1" {
		paths := os.Args[len(os.Args)-2:]
		pins := fixturePins()
		c, err := OpenCoordinated(paths[0], paths[1], uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
			return pins.Principal, nil
		})
		if err != nil {
			t.Fatal(err)
		}
		c.witness.syncDir = func() error { return errors.New("fixture intent barrier interrupted") }
		if _, err = c.Create(context.Background(), fixtureChange("initial", Head{}), fixtureAt()); !errors.Is(err, ErrIndeterminate) {
			t.Fatal(err)
		}
		// Exit without Close or test cleanup. The parent must reconstruct from disk.
		os.Exit(0)
	}
	_, dir, wdir := fixtureCoordinator(t)
	cmd := exec.Command(os.Args[0], "-test.run=^TestCoordinatedSubprocessInterruptedIntent$", "-test.timeout=0", "--", dir, wdir)
	cmd.Env = append(os.Environ(), "SWARM_WITNESS_FIXTURE_CHILD=1")
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("child: %v %s", err, out)
	}
	if _, err := os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatal("child wrote config before intent barrier")
	}
	pins := fixturePins()
	fresh, err := OpenCoordinated(dir, wdir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	defer fresh.Close()
	v, err := fresh.Read(context.Background())
	assertZero(t, v, err, ErrPending)
	v, err = fresh.Recover(context.Background())
	if err != nil || v.Head.Revision != 1 || len(v.Snapshot.History) != 1 {
		t.Fatalf("new-process state recovery: %#v %v", v, err)
	}
}
