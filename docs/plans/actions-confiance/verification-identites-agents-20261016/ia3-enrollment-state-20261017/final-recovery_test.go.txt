package enrollmentstate

import (
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"math"
	"strings"
	"testing"

	"swarm.local/companion/internal/enrollmentproof"
)

func TestTrustedHeadDetectsConsistentStorageRollback(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	active, _ := appendChange(t, f, Head{}, "enroll", "alice", []byte("original"))
	revoked, _ := appendChange(t, f, active, "revoke", "alice", nil)
	// Test-controlled restoration of BOTH mutable SQL parts to a formerly valid
	// history. Its signatures are authentic; only the independent witness is new.
	tx, err := f.journal.db.BeginTx(ctx, nil)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = tx.Exec(`DELETE FROM enrollment_events WHERE sequence=?`, revoked.Sequence); err != nil {
		t.Fatal(err)
	}
	if _, err = tx.Exec(`UPDATE enrollment_meta SET sequence=?,digest=? WHERE singleton=1`, active.Sequence, active.Digest); err != nil {
		t.Fatal(err)
	}
	if err = tx.Commit(); err != nil {
		t.Fatal(err)
	}
	if err = f.journal.WithActive(ctx, revoked, "alice", func(Record) error { t.Fatal("rollback became current"); return nil }); !errors.Is(err, ErrHead) {
		t.Fatal(err)
	}
	if _, err = f.journal.Apply(ctx, revoked, command(t, f, revoked, "enroll", "bob", nil)); !errors.Is(err, ErrHead) {
		t.Fatal(err)
	}
	// Limit guard: restoring the witness too removes rollback detection. A caller
	// must never learn its expected head from the restored/imported journal.
	if err = f.journal.WithActive(ctx, active, "alice", func(Record) error { return nil }); err != nil {
		t.Fatal(err)
	}
}

func TestAppendFailureRollsBackEventAndCanResume(t *testing.T) {
	f := setup(t)
	ctx := context.Background()
	raw := command(t, f, Head{}, "enroll", "alice", []byte("original"))
	if _, err := f.journal.db.Exec(`CREATE TRIGGER fixture_reject_head BEFORE UPDATE ON enrollment_meta BEGIN SELECT RAISE(ABORT,'fixture injected write failure'); END`); err != nil {
		t.Fatal(err)
	}
	if head, err := f.journal.Apply(ctx, Head{}, raw); err == nil || head != (Head{}) {
		t.Fatal("failed head write returned success", head, err)
	}
	var count int
	var head Head
	if err := f.journal.db.QueryRow(`SELECT count(*) FROM enrollment_events`).Scan(&count); err != nil || count != 0 {
		t.Fatal("partial event survived", count, err)
	}
	if err := f.journal.db.QueryRow(`SELECT sequence,digest FROM enrollment_meta`).Scan(&head.Sequence, &head.Digest); err != nil || head != (Head{}) {
		t.Fatal("head mutated", err)
	}
	if _, err := f.journal.db.Exec(`DROP TRIGGER fixture_reject_head`); err != nil {
		t.Fatal(err)
	}
	head, err := f.journal.Apply(ctx, Head{}, raw)
	if err != nil || head.Sequence != 1 {
		t.Fatal("corrected precondition did not resume", err)
	}
	revoked, revoke := appendChange(t, f, head, "revoke", "alice", nil)
	if replay, err := f.journal.Apply(ctx, head, revoke); err != nil || replay != revoked {
		t.Fatal("exact revocation retry failed", err)
	}
	if _, err := f.journal.Apply(ctx, revoked, command(t, f, revoked, "revoke", "alice", nil)); !errors.Is(err, ErrRevoked) {
		t.Fatal("new duplicate revocation accepted", err)
	}
}

func TestCanonicalChangesAndSignerInputValidation(t *testing.T) {
	f := setup(t)
	raw := command(t, f, Head{}, "enroll", "alice", nil)
	for _, kind := range []string{"unknown", "duplicate", "folded", "null", "trailing", "wrong-type", "foreign-key", "foreign-project", "wrong-domain"} {
		t.Run(kind, func(t *testing.T) {
			bad := append([]byte(nil), raw...)
			switch kind {
			case "unknown":
				bad = []byte(strings.Replace(string(raw), `{`, `{"extra":"",`, 1))
			case "duplicate":
				bad = []byte(strings.Replace(string(raw), `{`, `{"id":"alice",`, 1))
			case "folded":
				bad = []byte(strings.Replace(string(raw), `"id":`, `"ID":`, 1))
			case "null":
				bad = []byte(strings.Replace(string(raw), `"body":""`, `"body":null`, 1))
			case "trailing":
				bad = append(bad, []byte(`{}`)...)
			case "wrong-type":
				bad = []byte(strings.Replace(string(raw), `"sequence":1`, `"sequence":"1"`, 1))
			case "foreign-key":
				_, key, _ := ed25519.GenerateKey(rand.Reader)
				bad, _ = SignChange(key, f.project, Head{}, "enroll", "alice", nil)
			case "foreign-project":
				bad, _ = SignChange(f.key, sha256.Sum256([]byte("other project")), Head{}, "enroll", "alice", nil)
			case "wrong-domain":
				var c change
				if err := json.Unmarshal(raw, &c); err != nil {
					t.Fatal(err)
				}
				c.Protocol = "another-protocol"
				bad, _ = json.Marshal(c)
			}
			if head, err := f.journal.Apply(context.Background(), Head{}, bad); err == nil || head != (Head{}) {
				t.Fatal("invalid change succeeded", kind, err)
			}
		})
	}
	for _, kind := range []string{"key", "head", "overflow", "operation", "empty-id", "control-id", "invalid-utf8", "revoke-body"} {
		t.Run(kind, func(t *testing.T) {
			key, base, operation, id, body := f.key, Head{}, "enroll", "alice", []byte(nil)
			switch kind {
			case "key":
				key = nil
			case "head":
				base = Head{-1, ""}
			case "overflow":
				base = Head{math.MaxInt64, strings.Repeat("a", 64)}
			case "operation":
				operation = "reactivate"
			case "empty-id":
				id = "  "
			case "control-id":
				id = "alice\x00"
			case "invalid-utf8":
				id = string([]byte{255})
			case "revoke-body":
				operation, body = "revoke", []byte("not empty")
			}
			if out, err := SignChange(key, f.project, base, operation, id, body); err == nil || len(out) != 0 {
				t.Fatal("invalid signer input accepted", err)
			}
		})
	}
	var count int
	if err := f.journal.db.QueryRow(`SELECT count(*) FROM enrollment_events`).Scan(&count); err != nil || count != 0 {
		t.Fatal("invalid requests mutated history", count, err)
	}
}

func TestCallbackFailureAndCancellationReleaseReservation(t *testing.T) {
	f := setup(t)
	head, _ := appendChange(t, f, Head{}, "enroll", "alice", nil)
	want := errors.New("consumer rejected opaque payload")
	if err := f.journal.WithActive(context.Background(), head, "alice", func(Record) error { return want }); !errors.Is(err, want) {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	if err := f.journal.WithActive(ctx, head, "alice", func(Record) error { cancel(); return nil }); !errors.Is(err, context.Canceled) {
		t.Fatal("canceled transaction succeeded", err)
	}
	appendChange(t, f, head, "revoke", "alice", nil)
}

func TestSignedActiveRefusesWrongContextWithoutBytes(t *testing.T) {
	f := setup(t)
	head, _ := appendChange(t, f, Head{}, "enroll", "alice", nil)
	for _, kind := range []string{"foreign-key", "foreign-project", "malformed-request", "stale-head", "canceled"} {
		t.Run(kind, func(t *testing.T) {
			key, project, expected := f.key, f.project, head
			ctx := context.Background()
			switch kind {
			case "foreign-key":
				_, key, _ = ed25519.GenerateKey(rand.Reader)
			case "foreign-project":
				project = sha256.Sum256([]byte("other"))
			case "stale-head":
				expected = Head{}
			case "canceled":
				canceled, cancel := context.WithCancel(ctx)
				cancel()
				ctx = canceled
			}
			v, err := enrollmentproof.NewVerifier(key.Public().(ed25519.PublicKey), project)
			if err != nil {
				t.Fatal(err)
			}
			challenge, err := v.NewChallenge(sha256.Sum256([]byte("request")))
			if err != nil {
				t.Fatal(err)
			}
			request := challenge.Request()
			if kind == "malformed-request" {
				request.Nonce = "invalid"
			}
			if reply, err := f.journal.SignActive(ctx, expected, "alice", request, key); err == nil || len(reply) != 0 {
				t.Fatal("invalid context produced signed bytes", err)
			}
		})
	}
	if err := f.journal.WithActive(context.Background(), head, "alice", func(Record) error { return nil }); err != nil {
		t.Fatal("refusals altered authority", err)
	}
}
