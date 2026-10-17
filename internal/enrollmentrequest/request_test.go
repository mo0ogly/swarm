package enrollmentrequest

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/rand"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"path/filepath"
	"reflect"
	"strings"
	"sync"
	"testing"
	"time"

	"swarm.local/companion/internal/enrollmentauthority"
	"swarm.local/companion/internal/enrollmentproof"
	"swarm.local/companion/internal/enrollmentstate"
	"swarm.local/companion/internal/identitycontract"
)

func hash(raw []byte) string { sum := sha256.Sum256(raw); return hex.EncodeToString(sum[:]) }
func example() Publication {
	at := time.Date(2026, 10, 17, 9, 0, 0, 0, time.UTC)
	snapshot := []byte(`{"schema_version":1,"revision":1,"grants":{},"instances":{}}`)
	return Publication{Schema: 1, Purpose: PublicationPurpose, ProjectDigest: strings.Repeat("a", 64),
		Principal:     identitycontract.Principal{ID: "alice", Kind: "human", Issuer: "https://[::1]/issuer", Subject: "employee[eu]"},
		SessionDigest: hash([]byte("opaque session fixture")), SessionVerifiedAt: at, SessionExpiresAt: at.Add(time.Hour),
		Scope: identitycontract.Scope{WorkID: "work-1", PlanVersion: 3, PlanDigest: strings.Repeat("b", 64)}, TrustRevision: 2,
		Enrollment: Enrollment{ID: "enrollment-1", Generation: 1, Head: enrollmentstate.Head{Sequence: 1, Digest: strings.Repeat("c", 64)}, DocumentDigest: strings.Repeat("d", 64)},
		EventID:    "publication-1", SnapshotRevision: 1, SnapshotDigest: hash(snapshot), Snapshot: snapshot, Roots: []string{"root-1", "root-2"}, PreviousRegistryHead: ""}
}
func TestFullRequestRoundTripAndOwnedCopies(t *testing.T) {
	p := example()
	prepared, err := Prepare(p)
	if err != nil {
		t.Fatal(err)
	}
	digest, err := prepared.Digest()
	if err != nil {
		t.Fatal(err)
	}
	copyHandle := prepared
	p.Snapshot[0] = '!'
	p.Roots[0] = "changed"
	raw := prepared.Bytes()
	original := append([]byte(nil), raw...)
	raw[0] = '!'
	exported, err := prepared.Publication()
	if err != nil {
		t.Fatal(err)
	}
	exported.Roots[0] = "changed"
	exported.Snapshot[0] = '!'
	if !bytes.Equal(copyHandle.Bytes(), original) {
		t.Fatal("request changed through a retained alias")
	}
	decoded, err := Decode(original)
	if err != nil {
		t.Fatal(err)
	}
	again, _ := decoded.Digest()
	if again != digest {
		t.Fatal("wire lost canonical identity")
	}
	plain, _ := decoded.Publication()
	if plain.Roots[0] != "root-1" || plain.Snapshot[0] != '{' {
		t.Fatal("readers share writable storage")
	}
	if bytes.Contains(original, []byte("opaque session fixture")) {
		t.Fatal("raw session content retained")
	}
	var reconstructed Prepared
	encoded, _ := json.Marshal(prepared)
	if err = json.Unmarshal(encoded, &reconstructed); err != nil {
		t.Fatal(err)
	}
	if _, err = reconstructed.Digest(); !errors.Is(err, ErrRequest) {
		t.Fatal("JSON reconstructed a private prepared handle", err)
	}
}
func TestEveryResolvedDimensionChangesTheRequestDigest(t *testing.T) {
	base, _ := Prepare(example())
	old, _ := base.Digest()
	mutations := map[string]func(*Publication){
		"project": func(p *Publication) { p.ProjectDigest = strings.Repeat("e", 64) }, "principal": func(p *Publication) { p.Principal.ID = "bob" }, "issuer": func(p *Publication) { p.Principal.Issuer += "/" }, "subject": func(p *Publication) { p.Principal.Subject += "x" },
		"session": func(p *Publication) { p.SessionDigest = strings.Repeat("e", 64) }, "verified-at": func(p *Publication) { p.SessionVerifiedAt = p.SessionVerifiedAt.Add(time.Nanosecond) }, "session-expiry": func(p *Publication) { p.SessionExpiresAt = p.SessionExpiresAt.Add(time.Nanosecond) },
		"work": func(p *Publication) { p.Scope.WorkID = "work-2" }, "plan-generation": func(p *Publication) { p.Scope.PlanVersion++ }, "plan-digest": func(p *Publication) { p.Scope.PlanDigest = strings.Repeat("e", 64) }, "trust": func(p *Publication) { p.TrustRevision++ },
		"enrollment-id": func(p *Publication) { p.Enrollment.ID = "other" }, "enrollment-generation": func(p *Publication) { p.Enrollment.Generation = 2; p.Enrollment.Head.Sequence = 2 }, "enrollment-head-sequence": func(p *Publication) { p.Enrollment.Head.Sequence++ }, "enrollment-head-digest": func(p *Publication) { p.Enrollment.Head.Digest = strings.Repeat("e", 64) }, "enrollment-body": func(p *Publication) { p.Enrollment.DocumentDigest = strings.Repeat("e", 64) },
		"event": func(p *Publication) { p.EventID = "publication-2" }, "snapshot-revision": func(p *Publication) { p.SnapshotRevision++ },
		"snapshot-content": func(p *Publication) {
			p.Snapshot = []byte(`{"schema_version":1,"revision":1,"grants":{},"instances":{},"extra":"retained"}`)
			p.SnapshotDigest = hash(p.Snapshot)
		},
		"root": func(p *Publication) { p.Roots[0] = "root-3" }, "root-order": func(p *Publication) { p.Roots[0], p.Roots[1] = p.Roots[1], p.Roots[0] }, "previous-registry-head": func(p *Publication) { p.PreviousRegistryHead = strings.Repeat("e", 64) },
	}
	for name, mutate := range mutations {
		t.Run(name, func(t *testing.T) {
			p := example()
			mutate(&p)
			got, err := Prepare(p)
			if err != nil {
				t.Fatal(err)
			}
			digest, err := got.Digest()
			if err != nil || digest == old {
				t.Fatal("request omitted changed dimension", err)
			}
		})
	}
}
func TestRequestRejectsInvalidAndNoncanonicalInputs(t *testing.T) {
	mutations := map[string]func(*Publication){
		"schema": func(p *Publication) { p.Schema = 2 }, "purpose": func(p *Publication) { p.Purpose = "agent.create" }, "project": func(p *Publication) { p.ProjectDigest = "" }, "service": func(p *Publication) { p.Principal.Kind = "service" },
		"session": func(p *Publication) { p.SessionDigest = "unverified" }, "zero-observation": func(p *Publication) { p.SessionVerifiedAt = time.Time{} }, "expired-session": func(p *Publication) { p.SessionExpiresAt = p.SessionVerifiedAt },
		"scope": func(p *Publication) { p.Scope.PlanVersion = 0 }, "trust": func(p *Publication) { p.TrustRevision = 0 }, "enrollment-id": func(p *Publication) { p.Enrollment.ID = "" }, "generation": func(p *Publication) { p.Enrollment.Generation = 0 }, "future-generation": func(p *Publication) { p.Enrollment.Generation = 2 }, "genesis-enrollment": func(p *Publication) { p.Enrollment.Head = enrollmentstate.Head{} }, "document-digest": func(p *Publication) { p.Enrollment.DocumentDigest = "" },
		"event": func(p *Publication) { p.EventID = "../event" }, "revision": func(p *Publication) { p.SnapshotRevision = 0 }, "snapshot-empty": func(p *Publication) { p.Snapshot = nil; p.SnapshotDigest = hash(nil) }, "snapshot-not-json": func(p *Publication) { p.Snapshot = []byte("broken"); p.SnapshotDigest = hash(p.Snapshot) }, "snapshot-null": func(p *Publication) { p.Snapshot = []byte("null"); p.SnapshotDigest = hash(p.Snapshot) }, "snapshot-digest": func(p *Publication) { p.SnapshotDigest = strings.Repeat("e", 64) },
		"roots-empty": func(p *Publication) { p.Roots = nil }, "roots-duplicate": func(p *Publication) { p.Roots[1] = p.Roots[0] }, "root-path": func(p *Publication) { p.Roots[0] = "../root" }, "checkpoint": func(p *Publication) { p.PreviousRegistryHead = "unverified" },
	}
	for name, mutate := range mutations {
		t.Run(name, func(t *testing.T) {
			p := example()
			mutate(&p)
			if prepared, err := Prepare(p); !errors.Is(err, ErrRequest) || len(prepared.Bytes()) != 0 {
				t.Fatal("invalid request prepared", err)
			}
		})
	}
	p, _ := Prepare(example())
	raw := p.Bytes()
	for name, bad := range map[string][]byte{
		"unknown": []byte(strings.Replace(string(raw), `{`, `{"approved":true,`, 1)), "duplicate": []byte(strings.Replace(string(raw), `{`, `{"schema_version":1,`, 1)),
		"nested-duplicate": []byte(strings.Replace(string(raw), `"principal_id":"alice"`, `"principal_id":"alice","principal_id":"alice"`, 1)), "folded": []byte(strings.Replace(string(raw), `"subject"`, `"Subject"`, 1)),
		"null": []byte(strings.Replace(string(raw), `"previous_registry_head_digest":""`, `"previous_registry_head_digest":null`, 1)), "omitted": []byte(strings.Replace(string(raw), `,"previous_registry_head_digest":""`, "", 1)),
		"trailing": append(append([]byte(nil), raw...), []byte(`{}`)...), "whitespace": append([]byte(" "), raw...),
	} {
		t.Run(name, func(t *testing.T) {
			if got, err := Decode(bad); !errors.Is(err, ErrRequest) || len(got.Bytes()) != 0 {
				t.Fatal("noncanonical request accepted", err)
			}
		})
	}
}
func TestChallengeBindingAndConcurrentValueCopies(t *testing.T) {
	p, _ := Prepare(example())
	pub, _, _ := ed25519.GenerateKey(rand.Reader)
	projectBytes, _ := hex.DecodeString(example().ProjectDigest)
	var project [32]byte
	copy(project[:], projectBytes)
	v, _ := enrollmentproof.NewVerifier(pub, project)
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		copyHandle := p
		wg.Add(1)
		go func() {
			defer wg.Done()
			ch, err := copyHandle.NewChallenge(v)
			if err != nil {
				t.Error(err)
				return
			}
			if err = copyHandle.Bind(ch.Request()); err != nil {
				t.Error(err)
			}
		}()
	}
	wg.Wait()
	other, _ := enrollmentproof.NewVerifier(pub, sha256.Sum256([]byte("other")))
	if ch, err := p.NewChallenge(other); !errors.Is(err, ErrBinding) || ch != nil {
		t.Fatal("challenge for foreign project returned", err)
	}
	ch, _ := p.NewChallenge(v)
	request := ch.Request()
	request.RequestDigest = strings.Repeat("e", 64)
	if err := p.Bind(request); !errors.Is(err, ErrBinding) {
		t.Fatal(err)
	}
	var zero Prepared
	if _, err := zero.NewChallenge(v); !errors.Is(err, ErrRequest) {
		t.Fatal(err)
	}
}
func TestRequestChecksExactAuthorityDocumentAndExplicitSessionWindow(t *testing.T) {
	pub, key, _ := ed25519.GenerateKey(rand.Reader)
	project := sha256.Sum256([]byte("isolated request project"))
	p := example()
	p.ProjectDigest = hex.EncodeToString(project[:])
	doc := enrollmentauthority.Document{Schema: 1, EnrollmentID: p.Enrollment.ID, Generation: 1, ProjectDigest: p.ProjectDigest, Principal: p.Principal, Scope: p.Scope, TrustRevision: p.TrustRevision, Permissions: []identitycontract.Permission{{Operation: "agent.create", Resource: "task/child"}}, NotBefore: p.SessionVerifiedAt.Add(-time.Minute), ExpiresAt: p.SessionExpiresAt.Add(time.Hour)}
	body, err := enrollmentauthority.Encode(doc)
	if err != nil {
		t.Fatal(err)
	}
	p.Enrollment.DocumentDigest = hash(body)
	j, err := enrollmentstate.Open(context.Background(), filepath.Join(t.TempDir(), "enroll.sqlite"), pub, project)
	if err != nil {
		t.Fatal(err)
	}
	defer j.Close()
	raw, _ := enrollmentauthority.SignEnrollment(key, project, enrollmentstate.Head{}, doc)
	p.Enrollment.Head, err = j.Apply(context.Background(), enrollmentstate.Head{}, raw)
	if err != nil {
		t.Fatal(err)
	}
	prepared, err := Prepare(p)
	if err != nil {
		t.Fatal(err)
	}
	v, _ := enrollmentproof.NewVerifier(pub, project)
	ch, err := prepared.NewChallenge(v)
	if err != nil {
		t.Fatal(err)
	}
	reply, err := j.SignActive(context.Background(), p.Enrollment.Head, doc.EnrollmentID, ch.Request(), key)
	if err != nil {
		t.Fatal(err)
	}
	statement, err := v.Verify(ch, reply)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := prepared.CheckActive(statement, p.SessionVerifiedAt); err != nil || !reflect.DeepEqual(got.Permissions, doc.Permissions) {
		t.Fatal("bound authority rejected", err)
	}
	for name, at := range map[string]time.Time{"before-observation": p.SessionVerifiedAt.Add(-time.Nanosecond), "at-expiry": p.SessionExpiresAt, "zero": {}} {
		t.Run(name, func(t *testing.T) {
			if got, err := prepared.CheckActive(statement, at); !errors.Is(err, ErrSession) || !reflect.DeepEqual(got, enrollmentauthority.Document{}) {
				t.Fatal("session window bypassed", err)
			}
		})
	}
	// Even an authentic, structurally valid document for the same ID/generation
	// must match the exact independently expected body, including its ceilings.
	altered := doc
	altered.Permissions = []identitycontract.Permission{{Operation: "effect.execute", Resource: "external/target"}}
	newBody, _ := enrollmentauthority.Encode(altered)
	payload, _ := json.Marshal(enrollmentstate.ActiveReply{Schema: enrollmentstate.ActiveReplySchema, Head: p.Enrollment.Head, Record: enrollmentstate.Record{ID: doc.EnrollmentID, Body: newBody, EnrolledSequence: 1}})
	ch, _ = prepared.NewChallenge(v)
	reply, _ = enrollmentproof.SignResponse(ch.Request(), payload, key)
	statement, err = v.Verify(ch, reply)
	if err != nil {
		t.Fatal(err)
	}
	if got, err := prepared.CheckActive(statement, p.SessionVerifiedAt); !errors.Is(err, ErrBinding) || !reflect.DeepEqual(got, enrollmentauthority.Document{}) {
		t.Fatal("different signed document accepted", err)
	}
}
