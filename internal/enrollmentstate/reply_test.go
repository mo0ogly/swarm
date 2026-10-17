package enrollmentstate

import (
	"context"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"reflect"
	"strings"
	"testing"

	"swarm.local/companion/internal/enrollmentproof"
)

func TestActiveReplyCurrentVersionAndEmptyOpaqueBody(t *testing.T) {
	f := setup(t)
	head, _ := appendChange(t, f, Head{}, "enroll", "opaque-empty", nil)
	v, err := enrollmentproof.NewVerifier(f.key.Public().(ed25519.PublicKey), f.project)
	if err != nil {
		t.Fatal(err)
	}
	ch, err := v.NewChallenge(sha256.Sum256([]byte("version fixture")))
	if err != nil {
		t.Fatal(err)
	}
	raw, err := f.journal.SignActive(context.Background(), head, "opaque-empty", ch.Request(), f.key)
	if err != nil {
		t.Fatal(err)
	}
	statement, err := v.Verify(ch, raw)
	if err != nil {
		t.Fatal(err)
	}
	r, err := DecodeActiveReply(statement.Payload())
	if err != nil || r.Schema != ActiveReplySchema || r.Head != head || r.Record.ID != "opaque-empty" || r.Record.EnrolledSequence != 1 || r.Record.Body == nil || len(r.Record.Body) != 0 {
		t.Fatal("producer and reader disagree", r, err)
	}
	// Opaque empty bytes are valid journal data, never an authority document.
}

func TestActiveReplyStrictDecoderRejectsVersionAndShapeErrors(t *testing.T) {
	good := ActiveReply{Schema: ActiveReplySchema, Head: Head{Sequence: 2, Digest: strings.Repeat("a", 64)}, Record: Record{ID: "alice", Body: []byte("owned"), EnrolledSequence: 1}}
	for _, kind := range []string{"zero", "future", "negative", "bad-head", "zero-head", "empty-id", "control-id", "zero-generation", "future-generation", "null-body"} {
		t.Run(kind, func(t *testing.T) {
			r := good
			version := false
			switch kind {
			case "zero":
				r.Schema = 0
				version = true
			case "future":
				r.Schema = 2
				version = true
			case "negative":
				r.Schema = -1
				version = true
			case "bad-head":
				r.Head.Digest = "unverified"
			case "zero-head":
				r.Head = Head{}
			case "empty-id":
				r.Record.ID = ""
			case "control-id":
				r.Record.ID = "alice\x00"
			case "zero-generation":
				r.Record.EnrolledSequence = 0
			case "future-generation":
				r.Record.EnrolledSequence = 3
			case "null-body":
				r.Record.Body = nil
			}
			raw, _ := json.Marshal(r)
			got, err := DecodeActiveReply(raw)
			if err == nil || !reflect.DeepEqual(got, ActiveReply{}) {
				t.Fatal("invalid frame returned data", kind, err)
			}
			if version && !errors.Is(err, ErrReplyVersion) {
				t.Fatal("version cause lost", err)
			}
		})
	}
	raw, _ := json.Marshal(good)
	for name, bad := range map[string][]byte{
		"omitted": []byte(strings.Replace(string(raw), `"schema_version":1,`, "", 1)), "null": []byte(strings.Replace(string(raw), `"schema_version":1`, `"schema_version":null`, 1)), "string": []byte(strings.Replace(string(raw), `"schema_version":1`, `"schema_version":"1"`, 1)),
		"duplicate": []byte(strings.Replace(string(raw), `{`, `{"schema_version":1,`, 1)), "folded": []byte(strings.Replace(string(raw), `"schema_version"`, `"Schema_Version"`, 1)), "extra": []byte(strings.Replace(string(raw), `{`, `{"ignored":true,`, 1)), "whitespace": append([]byte(" "), raw...), "trailing": append(append([]byte(nil), raw...), []byte(`{}`)...),
	} {
		t.Run(name, func(t *testing.T) {
			if got, err := DecodeActiveReply(bad); err == nil || !reflect.DeepEqual(got, ActiveReply{}) {
				t.Fatal("noncanonical frame returned data", err)
			}
		})
	}
	decoded, err := DecodeActiveReply(raw)
	if err != nil {
		t.Fatal(err)
	}
	decoded.Record.Body[0] = '!'
	again, err := DecodeActiveReply(raw)
	if err != nil || string(again.Record.Body) != "owned" {
		t.Fatal("readers share a writable body", err)
	}
}
