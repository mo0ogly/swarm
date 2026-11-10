package enrollmentstate

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sync"
	"testing"

	"swarm.local/companion/internal/enrollmentproof"
)

type fixture struct {
	journal *Journal
	key     ed25519.PrivateKey
	project [32]byte
	path    string
}

func setup(t *testing.T) fixture {
	t.Helper()
	pub, key, err := ed25519.GenerateKey(rand.Reader)
	if err != nil {
		t.Fatal(err)
	}
	project := sha256.Sum256([]byte("explicit project fixture"))
	path := filepath.Join(t.TempDir(), "enrollment.sqlite")
	j, err := Open(context.Background(), path, pub, project)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { j.Close() })
	return fixture{j, key, project, path}
}

func command(t *testing.T, f fixture, base Head, operation, id string, body []byte) []byte {
	t.Helper()
	raw, err := SignChange(f.key, f.project, base, operation, id, body)
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func appendChange(t *testing.T, f fixture, base Head, operation, id string, body []byte) (Head, []byte) {
	t.Helper()
	raw := command(t, f, base, operation, id, body)
	head, err := f.journal.Apply(context.Background(), base, raw)
	if err != nil {
		t.Fatal(err)
	}
	return head, raw
}

func TestDurableEnrollmentRevocationAndExactRetry(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	head, raw := appendChange(t, f, Head{}, "enroll", "issuer-1", []byte(`{"authority":"opaque fixture"}`))
	if retry, err := f.journal.Apply(ctx, Head{}, raw); err != nil || retry != head {
		t.Fatal("response loss duplicated or lost exact event: ", err)
	}
	var body []byte
	if err := f.journal.WithActive(ctx, head, "issuer-1", func(r Record) error { body = r.Body; return nil }); err != nil || !bytes.Contains(body, []byte("opaque fixture")) {
		t.Fatal(err)
	}
	body[0] = '!'
	if err := f.journal.WithActive(ctx, head, "issuer-1", func(r Record) error {
		if r.Body[0] != '{' {
			t.Fatal("reader retained alias")
		}
		return nil
	}); err != nil {
		t.Fatal(err)
	}
	if err := f.journal.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := Open(ctx, f.path, f.key.Public().(ed25519.PublicKey), f.project)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.Close()
	f.journal = reopened
	if err = reopened.WithActive(ctx, head, "issuer-1", func(Record) error { return nil }); err != nil {
		t.Fatal("reopen lost active record: ", err)
	}
	revoked, _ := appendChange(t, f, head, "revoke", "issuer-1", nil)
	called := false
	if err = reopened.WithActive(ctx, revoked, "issuer-1", func(Record) error { called = true; return nil }); !errors.Is(err, ErrRevoked) || called {
		t.Fatal("revocation did not withdraw active view: ", err)
	}
	if err = reopened.WithActive(ctx, head, "issuer-1", func(Record) error { return nil }); !errors.Is(err, ErrHead) {
		t.Fatal("old witness restored authority: ", err)
	}
	if _, err = reopened.Apply(ctx, revoked, command(t, f, revoked, "enroll", "issuer-1", []byte("replacement"))); !errors.Is(err, ErrExists) {
		t.Fatal("revoked ID reactivated: ", err)
	}
	if _, err = reopened.Apply(ctx, Head{}, raw); !errors.Is(err, ErrHead) {
		t.Fatal("old enrollment replay bypassed newer revocation: ", err)
	}
	var count int
	if err = reopened.db.QueryRow(`SELECT count(*) FROM enrollment_events`).Scan(&count); err != nil || count != 2 {
		t.Fatal("history changed: ", count, err)
	}
}

func TestJournalRejectsTamperAndForeignContext(t *testing.T) {
	for _, kind := range []string{"changed-body", "missing-event", "rolled-back-head", "foreign-project", "foreign-key", "noncanonical", "bad-signature"} {
		t.Run(kind, func(t *testing.T) {
			f := setup(t)
			ctx := context.Background()
			head, raw := appendChange(t, f, Head{}, "enroll", "alice", []byte("original"))
			switch kind {
			case "changed-body":
				_, _ = f.journal.db.Exec(`UPDATE enrollment_events SET body=?`, bytes.Replace(raw, []byte("b3JpZ2luYWw"), []byte("Zm9yZ2Vk"), 1))
			case "missing-event":
				_, _ = f.journal.db.Exec(`DELETE FROM enrollment_events`)
			case "rolled-back-head":
				_, _ = f.journal.db.Exec(`UPDATE enrollment_meta SET sequence=0,digest=''`)
			case "foreign-project":
				if _, err := Open(ctx, f.path, f.key.Public().(ed25519.PublicKey), sha256.Sum256([]byte("other"))); !errors.Is(err, ErrContext) {
					t.Fatal(err)
				}
				return
			case "foreign-key":
				pub, _, _ := ed25519.GenerateKey(rand.Reader)
				if _, err := Open(ctx, f.path, pub, f.project); !errors.Is(err, ErrContext) {
					t.Fatal(err)
				}
				return
			case "noncanonical":
				if _, err := f.journal.Apply(ctx, Head{}, append([]byte(" "), raw...)); !errors.Is(err, ErrChange) {
					t.Fatal(err)
				}
				return
			case "bad-signature":
				var c change
				if err := json.Unmarshal(raw, &c); err != nil {
					t.Fatal(err)
				}
				c.Signature = "not a signature"
				bad, _ := json.Marshal(c)
				if _, err := f.journal.Apply(ctx, Head{}, bad); !errors.Is(err, ErrChange) {
					t.Fatal(err)
				}
				return
			}
			called := false
			if err := f.journal.WithActive(ctx, head, "alice", func(Record) error { called = true; return nil }); err == nil || called {
				t.Fatal("tampered storage gained active view")
			}
		})
	}
}

func TestSignedActiveReplyAndRevocation(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	head, _ := appendChange(t, f, Head{}, "enroll", "alice", []byte("opaque authority"))
	verifier, err := enrollmentproof.NewVerifier(f.key.Public().(ed25519.PublicKey), f.project)
	if err != nil {
		t.Fatal(err)
	}
	challenge, err := verifier.NewChallenge(sha256.Sum256([]byte("resolved request fixture")))
	if err != nil {
		t.Fatal(err)
	}
	raw, err := f.journal.SignActive(ctx, head, "alice", challenge.Request(), f.key)
	if err != nil {
		t.Fatal(err)
	}
	statement, err := verifier.Verify(challenge, raw)
	if err != nil {
		t.Fatal(err)
	}
	var active ActiveReply
	if err = json.Unmarshal(statement.Payload(), &active); err != nil || active.Head != head || active.Record.ID != "alice" || !bytes.Equal(active.Record.Body, []byte("opaque authority")) {
		t.Fatal("signed current payload lost content: ", err)
	}
	revoked, _ := appendChange(t, f, head, "revoke", "alice", nil)
	newChallenge, _ := verifier.NewChallenge(sha256.Sum256([]byte("new request")))
	if reply, err := f.journal.SignActive(ctx, revoked, "alice", newChallenge.Request(), f.key); !errors.Is(err, ErrRevoked) || len(reply) != 0 {
		t.Fatal("revoked record returned a signed active response: ", err)
	}
	// The old authentic response above remains historical. A separate verifier
	// cannot infer revocation from its signature: final-use integration is open.
}

func TestActiveReadAndRevocationSerializeLocally(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	head, _ := appendChange(t, f, Head{}, "enroll", "alice", []byte("body"))
	revocation := command(t, f, head, "revoke", "alice", nil)
	entered, release, done := make(chan struct{}), make(chan struct{}), make(chan error, 1)
	go func() {
		done <- f.journal.WithActive(ctx, head, "alice", func(Record) error { close(entered); <-release; return nil })
	}()
	<-entered
	if _, err := f.journal.Apply(ctx, head, revocation); err == nil {
		close(release)
		t.Fatal("revocation committed during protected active callback")
	}
	close(release)
	if err := <-done; err != nil {
		t.Fatal(err)
	}
	revoked, err := f.journal.Apply(ctx, head, revocation)
	if err != nil {
		t.Fatal(err)
	}
	if err = f.journal.WithActive(ctx, revoked, "alice", func(Record) error { t.Fatal("active callback entered after revocation"); return nil }); !errors.Is(err, ErrRevoked) {
		t.Fatal(err)
	}
}

func TestConcurrentIdenticalChangeHasOneEvent(t *testing.T) {
	f := setup(t)
	raw := command(t, f, Head{}, "enroll", "alice", []byte("body"))
	var wg sync.WaitGroup
	var mu sync.Mutex
	success := 0
	var failures []string
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if _, err := f.journal.Apply(context.Background(), Head{}, raw); err == nil {
				mu.Lock()
				success++
				mu.Unlock()
			} else {
				mu.Lock()
				failures = append(failures, err.Error())
				mu.Unlock()
			}
		}()
	}
	wg.Wait()
	if success == 0 {
		t.Fatal("no writer succeeded: ", failures)
	}
	var count int
	if err := f.journal.db.QueryRow(`SELECT count(*) FROM enrollment_events`).Scan(&count); err != nil || count != 1 {
		t.Fatal(count, err)
	}
}

func TestInvalidChangesAndCanceledReadsDoNotMutate(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	head, _ := appendChange(t, f, Head{}, "enroll", "x'; DROP TABLE enrollment_events;--", []byte("original"))
	for _, op := range []string{"revoke-missing", "enroll-duplicate"} {
		t.Run(op, func(t *testing.T) {
			operation, id := "revoke", "missing"
			body := []byte(nil)
			want := ErrMissing
			if op == "enroll-duplicate" {
				operation, id, body, want = "enroll", "x'; DROP TABLE enrollment_events;--", []byte("new"), ErrExists
			}
			if _, err := f.journal.Apply(ctx, head, command(t, f, head, operation, id, body)); !errors.Is(err, want) {
				t.Fatal(err)
			}
		})
	}
	canceled, cancel := context.WithCancel(ctx)
	cancel()
	if err := f.journal.WithActive(canceled, head, "missing", func(Record) error { t.Fatal("canceled callback ran"); return nil }); !errors.Is(err, context.Canceled) {
		t.Fatal(err)
	}
	var count int
	if err := f.journal.db.QueryRow(`SELECT count(*) FROM enrollment_events`).Scan(&count); err != nil || count != 1 {
		t.Fatal(count, err)
	}
	// The backing file is a disposable fixture, never the project mission DB.
	if info, err := os.Stat(f.path); err != nil || info.Size() == 0 {
		t.Fatal(err)
	}
}
