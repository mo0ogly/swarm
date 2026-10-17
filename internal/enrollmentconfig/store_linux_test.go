//go:build linux

package enrollmentconfig

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
	"time"

	"golang.org/x/sys/unix"
	"swarm.local/companion/internal/identitycontract"
)

func fixtureStore(t *testing.T) (*Store, string) {
	t.Helper()
	dir := t.TempDir()
	if err := os.Chmod(dir, 0700); err != nil {
		t.Fatal(err)
	}
	pins := fixturePins()
	s, err := Open(dir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() {
		if err := s.Close(); err != nil {
			t.Error(err)
		}
	})
	return s, dir
}
func mustCreate(t *testing.T, s *Store) View {
	t.Helper()
	v, err := s.Create(context.Background(), fixtureChange("initial", Head{}), fixtureAt())
	if err != nil {
		t.Fatal(err)
	}
	return v
}
func assertZero(t *testing.T, v View, err error, cause error) {
	t.Helper()
	if !errors.Is(err, cause) || !reflect.DeepEqual(v, View{}) {
		t.Fatalf("want zero + %v; got %#v %v", cause, v, err)
	}
}

func TestConfigStoreCreatePreviewReopenAndOwnedViews(t *testing.T) {
	s, dir := fixtureStore(t)
	ctx := context.Background()
	initial := fixtureChange("initial", Head{})
	projection, err := s.Preview(ctx, initial, fixtureAt())
	if err != nil || projection.Base != (Head{}) || projection.Next.Head.Revision != 1 {
		t.Fatalf("genesis preview: %#v %v", projection, err)
	}
	if _, err = os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatalf("preview created config: %v", err)
	}
	v, err := s.Apply(ctx, initial, fixtureAt())
	assertZero(t, v, err, ErrNotConfigured)
	first := mustCreate(t, s)
	if first.Head != projection.Next.Head {
		t.Fatal("preview/application diverge")
	}
	raw, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	change := fixtureChange("next", first.Head)
	change.Values.SocketPath = "/run/swarm-fixture/next.sock"
	projection, err = s.Preview(ctx, change, fixtureAt().Add(time.Second))
	if err != nil {
		t.Fatal(err)
	}
	after, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	if string(after) != string(raw) {
		t.Fatal("preview changed persisted bytes")
	}
	projection.Next.Snapshot.History[0].Change.Reason = "mutated projection"
	next, err := s.Apply(ctx, change, fixtureAt().Add(time.Second))
	if err != nil || next.Head.Revision != 2 {
		t.Fatalf("apply: %#v %v", next, err)
	}
	if next.Snapshot.History[0].Change.Reason == "mutated projection" {
		t.Fatal("projection aliases state")
	}
	pins := fixturePins()
	other, err := Open(dir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	read, err := other.Read(ctx, next.Head)
	if err != nil || !reflect.DeepEqual(read, next) {
		t.Fatalf("reopen: %#v %v", read, err)
	}
	read.Snapshot.History[0].Change.Reason = "mutated read"
	read, err = other.Read(ctx, next.Head)
	if err != nil || read.Snapshot.History[0].Change.Reason == "mutated read" {
		t.Fatal("read aliases state")
	}
}

func TestConfigStoreCASReplayAndRollback(t *testing.T) {
	s, dir := fixtureStore(t)
	ctx := context.Background()
	first := mustCreate(t, s)
	old, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	change := fixtureChange("second", first.Head)
	next, err := s.Apply(ctx, change, fixtureAt().Add(time.Second))
	if err != nil {
		t.Fatal(err)
	}
	replay, err := s.Apply(ctx, change, fixtureAt().Add(time.Hour))
	if err != nil || !reflect.DeepEqual(replay, next) {
		t.Fatalf("exact current retry changed history: %v", err)
	}
	for name, c := range map[string]Change{"stale": fixtureChange("stale", first.Head), "conflicting-event": func() Change { x := change; x.Reason = "another reason"; return x }(), "historical-event": fixtureChange("initial", Head{})} {
		t.Run(name, func(t *testing.T) {
			v, err := s.Apply(ctx, c, fixtureAt())
			if err == nil || !reflect.DeepEqual(v, View{}) {
				t.Fatalf("accepted %#v %v", v, err)
			}
		})
	}
	if err = os.WriteFile(filepath.Join(dir, "config.json"), old, 0600); err != nil {
		t.Fatal(err)
	}
	v, err := s.Read(ctx, next.Head)
	assertZero(t, v, err, ErrHead)
	// Restoring the caller's witness too removes this protection; keep that limit.
	if _, err = s.Read(ctx, first.Head); err != nil {
		t.Fatalf("historical matching witness: %v", err)
	}
}

func TestConfigStoreAuthorizationAndRejectedChangesDoNotMutate(t *testing.T) {
	s, dir := fixtureStore(t)
	ctx := context.Background()
	first := mustCreate(t, s)
	before, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	pins := fixturePins()
	calls := 0
	s.authorize = func(_ context.Context, current *Snapshot, c Change) (identitycontract.Principal, error) {
		calls++
		current.History[0].Change.Reason = "hostile callback copy"
		return identitycontract.Principal{}, errors.New("fixture refusal")
	}
	v, err := s.Apply(ctx, fixtureChange("denied", first.Head), fixtureAt())
	assertZero(t, v, err, ErrUnauthorized)
	if _, err = s.Preview(ctx, fixtureChange("denied", first.Head), fixtureAt()); !errors.Is(err, ErrUnauthorized) {
		t.Fatal(err)
	}
	s.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		p := pins.Principal
		p.Subject = "other human"
		return p, nil
	}
	v, err = s.Apply(ctx, fixtureChange("other-human", first.Head), fixtureAt())
	assertZero(t, v, err, ErrUnauthorized)
	s.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	}
	cancelled, cancel := context.WithCancel(ctx)
	cancel()
	v, err = s.Apply(cancelled, fixtureChange("cancelled", first.Head), fixtureAt())
	assertZero(t, v, err, context.Canceled)
	invalid := fixtureChange("remote", first.Head)
	invalid.Values.Mode = "remote"
	v, err = s.Apply(ctx, invalid, fixtureAt())
	assertZero(t, v, err, ErrMode)
	after, _ := os.ReadFile(filepath.Join(dir, "config.json"))
	if string(before) != string(after) || calls != 2 {
		t.Fatalf("refusal mutated state/callback calls %d", calls)
	}
	foreign := pins
	foreign.InstallationDigest = strings.Repeat("b", 64)
	other, err := Open(dir, uint32(os.Geteuid()), foreign, s.authorize)
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	v, err = other.Read(ctx, first.Head)
	assertZero(t, v, err, ErrPins)
}

func TestConfigStorePersistenceUncertaintyAndExactRecovery(t *testing.T) {
	s, _ := fixtureStore(t)
	ctx := context.Background()
	first := mustCreate(t, s)
	change := fixtureChange("uncertain", first.Head)
	projection, err := s.Preview(ctx, change, fixtureAt().Add(time.Second))
	if err != nil {
		t.Fatal(err)
	}
	original := s.syncDir
	s.syncDir = func() error { return errors.New("fixture directory fsync failure") }
	v, err := s.Apply(ctx, change, fixtureAt().Add(time.Second))
	assertZero(t, v, err, ErrIndeterminate)
	s.syncDir = original
	observed, err := s.Read(ctx, projection.Next.Head)
	if err != nil {
		t.Fatal(err)
	}
	recovered, err := s.Apply(ctx, change, fixtureAt().Add(time.Hour))
	if err != nil || !reflect.DeepEqual(recovered, observed) || len(recovered.Snapshot.History) != 2 {
		t.Fatalf("uncertain retry: %#v %v", recovered, err)
	}
}

func TestConfigStoreConcurrentHandlesRejectBusyThenStale(t *testing.T) {
	s, dir := fixtureStore(t)
	ctx := context.Background()
	first := mustCreate(t, s)
	pins := fixturePins()
	entered := make(chan struct{})
	release := make(chan struct{})
	var once sync.Once
	defer once.Do(func() { close(release) })
	s.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		close(entered)
		<-release
		return pins.Principal, nil
	}
	other, err := Open(dir, uint32(os.Geteuid()), pins, func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	defer other.Close()
	done := make(chan error, 1)
	go func() { _, err := s.Apply(ctx, fixtureChange("winner", first.Head), fixtureAt()); done <- err }()
	<-entered
	v, err := other.Apply(ctx, fixtureChange("loser", first.Head), fixtureAt())
	assertZero(t, v, err, ErrBusy)
	once.Do(func() { close(release) })
	if err = <-done; err != nil {
		t.Fatal(err)
	}
	v, err = other.Apply(ctx, fixtureChange("loser", first.Head), fixtureAt())
	assertZero(t, v, err, ErrConflict)
}

func TestConfigStoreRejectsUnsafeDirectoriesAndFiles(t *testing.T) {
	pins := fixturePins()
	auth := func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return pins.Principal, nil
	}
	for _, name := range []string{"directory-symlink", "writable-directory", "wrong-owner", "nil-authorizer", "writable-parent", "lock-symlink"} {
		t.Run(name, func(t *testing.T) {
			base := t.TempDir()
			if err := os.Chmod(base, 0700); err != nil {
				t.Fatal(err)
			}
			dir := base
			owner := uint32(os.Geteuid())
			authorize := Authorizer(auth)
			switch name {
			case "directory-symlink":
				dir = filepath.Join(t.TempDir(), "alias")
				if err := os.Symlink(base, dir); err != nil {
					t.Fatal(err)
				}
			case "writable-directory":
				if err := os.Chmod(dir, 0755); err != nil {
					t.Fatal(err)
				}
			case "wrong-owner":
				owner++
			case "nil-authorizer":
				authorize = nil
			case "writable-parent":
				if err := os.Chmod(base, 0777); err != nil {
					t.Fatal(err)
				}
				dir = filepath.Join(base, "private")
				if err := os.Mkdir(dir, 0700); err != nil {
					t.Fatal(err)
				}
			case "lock-symlink":
				outside := filepath.Join(t.TempDir(), "outside")
				if err := os.WriteFile(outside, []byte("untouched"), 0600); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(outside, filepath.Join(dir, "config.lock")); err != nil {
					t.Fatal(err)
				}
			}
			s, err := Open(dir, owner, pins, authorize)
			if err == nil {
				if s != nil {
					s.Close()
				}
				t.Fatal("unsafe boundary accepted")
			}
		})
	}
	for _, name := range []string{"config-symlink", "public-config", "hardlink", "fifo", "malformed"} {
		t.Run(name, func(t *testing.T) {
			s, dir := fixtureStore(t)
			first := mustCreate(t, s)
			path := filepath.Join(dir, "config.json")
			switch name {
			case "config-symlink":
				outside := filepath.Join(t.TempDir(), "outside")
				raw, _ := os.ReadFile(path)
				if err := os.WriteFile(outside, raw, 0600); err != nil {
					t.Fatal(err)
				}
				if err := os.Remove(path); err != nil {
					t.Fatal(err)
				}
				if err := os.Symlink(outside, path); err != nil {
					t.Fatal(err)
				}
			case "public-config":
				if err := os.Chmod(path, 0644); err != nil {
					t.Fatal(err)
				}
			case "hardlink":
				if err := os.Link(path, filepath.Join(dir, "alias.json")); err != nil {
					t.Fatal(err)
				}
			case "fifo":
				if err := os.Remove(path); err != nil {
					t.Fatal(err)
				}
				if err := unix.Mkfifo(path, 0600); err != nil {
					t.Fatal(err)
				}
			case "malformed":
				if err := os.WriteFile(path, []byte(`{}`), 0600); err != nil {
					t.Fatal(err)
				}
			}
			v, err := s.Read(context.Background(), first.Head)
			if err == nil || !reflect.DeepEqual(v, View{}) {
				t.Fatalf("unsafe read: %#v %v", v, err)
			}
		})
	}
}

func TestConfigStoreRejectsPermissionsChangedAfterOpen(t *testing.T) {
	s, dir := fixtureStore(t)
	first := mustCreate(t, s)
	before, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err = os.Chmod(dir, 0770); err != nil {
		t.Fatal(err)
	}
	v, err := s.Apply(context.Background(), fixtureChange("after-chmod", first.Head), fixtureAt())
	assertZero(t, v, err, ErrPath)
	after, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil || string(before) != string(after) {
		t.Fatalf("unsafe directory mutation: %v", err)
	}
}

func TestConfigStorePreviewRejectsUnrepresentableClock(t *testing.T) {
	s, dir := fixtureStore(t)
	first := mustCreate(t, s)
	change := fixtureChange("bad-clock", first.Head)
	at := time.Date(10000, 1, 1, 0, 0, 0, 0, time.UTC)
	projection, err := s.Preview(context.Background(), change, at)
	if !errors.Is(err, ErrFormat) || !reflect.DeepEqual(projection, Projection{}) {
		t.Fatalf("unrepresentable preview: %#v %v", projection, err)
	}
	v, err := s.Apply(context.Background(), change, at)
	assertZero(t, v, err, ErrFormat)
	if _, err = s.Read(context.Background(), first.Head); err != nil {
		t.Fatal(err)
	}
	if _, err = os.Stat(filepath.Join(dir, "config.json")); err != nil {
		t.Fatal(err)
	}
}

func TestConfigStoreRejectsRestrictiveUmaskWithoutReplacingConfig(t *testing.T) {
	s, dir := fixtureStore(t)
	first := mustCreate(t, s)
	before, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	prior := unix.Umask(0700)
	defer unix.Umask(prior)
	v, err := s.Apply(context.Background(), fixtureChange("masked", first.Head), fixtureAt())
	assertZero(t, v, err, ErrPath)
	after, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil || string(before) != string(after) {
		t.Fatalf("umask caused unreadable replacement: %v", err)
	}
	files, err := os.ReadDir(dir)
	if err != nil || len(files) != 2 {
		t.Fatalf("temporary file retained after refusal: %v %v", files, err)
	}
}

func TestConfigStoreExplicitBootstrapRecoveryAndNoImplicitReset(t *testing.T) {
	s, dir := fixtureStore(t)
	ctx := context.Background()
	initial := fixtureChange("initial", Head{})
	projection, err := s.Preview(ctx, initial, fixtureAt())
	if err != nil {
		t.Fatal(err)
	}
	v, err := s.Read(ctx, projection.Next.Head)
	assertZero(t, v, err, ErrNotConfigured)
	original := s.syncDir
	s.syncDir = func() error { return errors.New("fixture initial directory sync failure") }
	v, err = s.Create(ctx, initial, fixtureAt())
	assertZero(t, v, err, ErrIndeterminate)
	s.syncDir = original
	created, err := s.Create(ctx, initial, fixtureAt().Add(time.Second))
	if err != nil || created.Head != projection.Next.Head || len(created.Snapshot.History) != 1 {
		t.Fatalf("explicit bootstrap recovery: %#v %v", created, err)
	}
	if err = os.Remove(filepath.Join(dir, "config.json")); err != nil {
		t.Fatal(err)
	}
	v, err = s.Apply(ctx, fixtureChange("after-delete", created.Head), fixtureAt())
	assertZero(t, v, err, ErrNotConfigured)
	if _, err = os.Stat(filepath.Join(dir, "config.json")); !os.IsNotExist(err) {
		t.Fatal("missing configuration recreated")
	}
}

func TestConfigStoreReplayReauthorizesAndCancellationInsideAuthorizer(t *testing.T) {
	s, dir := fixtureStore(t)
	first := mustCreate(t, s)
	before, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil {
		t.Fatal(err)
	}
	denied := errors.New("fixture principal no longer authenticated")
	s.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		return identitycontract.Principal{}, denied
	}
	v, err := s.Apply(context.Background(), fixtureChange("initial", Head{}), fixtureAt())
	assertZero(t, v, err, ErrUnauthorized)
	if !errors.Is(err, denied) || errors.Is(err, ErrPins) {
		t.Fatalf("authentication cause lost or misclassified: %v", err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	s.authorize = func(context.Context, *Snapshot, Change) (identitycontract.Principal, error) {
		cancel()
		return fixturePins().Principal, nil
	}
	v, err = s.Apply(ctx, fixtureChange("cancel-inside", first.Head), fixtureAt())
	assertZero(t, v, err, context.Canceled)
	after, err := os.ReadFile(filepath.Join(dir, "config.json"))
	if err != nil || string(before) != string(after) {
		t.Fatalf("refusal changed config: %v", err)
	}
}

func TestConfigStoreCoherentContentTamperRejectedByIndependentHead(t *testing.T) {
	s, dir := fixtureStore(t)
	first := mustCreate(t, s)
	tampered := owned(first.Snapshot)
	tampered.History[0].Change.Values.SocketPath = "/run/another-authority.sock"
	raw, err := Encode(tampered)
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(dir, "config.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
	v, err := s.Read(context.Background(), first.Head)
	assertZero(t, v, err, ErrHead)
}
