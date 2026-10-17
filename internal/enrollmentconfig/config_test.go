//go:build linux

package enrollmentconfig

import (
	"bytes"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"strings"
	"testing"
	"time"

	"swarm.local/companion/internal/identitycontract"
)

func fixturePins() Pins {
	seed := sha256.Sum256([]byte("installation config fixture, not a deployed root"))
	key := ed25519.NewKeyFromSeed(seed[:]).Public().(ed25519.PublicKey)
	id := sha256.Sum256([]byte("isolated installation fixture"))
	return Pins{InstallationDigest: hex.EncodeToString(id[:]), PublicKey: base64.RawURLEncoding.EncodeToString(key),
		Principal: identitycontract.Principal{ID: "operator", Kind: "human", Issuer: "https://fixture.invalid", Subject: "subject-exact"}}
}

func fixtureAt() time.Time { return time.Date(2026, 10, 17, 12, 0, 0, 0, time.UTC) }
func fixtureChange(event string, base Head) Change {
	return Change{Schema: 1, EventID: event, Expected: base, Values: Values{Mode: "local", SocketPath: "/run/swarm-fixture/authority.sock"}, Reason: "explicit fixture choice"}
}
func fixtureSnapshot() Snapshot {
	pins := fixturePins()
	return Snapshot{Schema: 1, Pins: pins, History: []Entry{{Change: fixtureChange("initial", Head{}), Actor: pins.Principal, At: fixtureAt()}}}
}

func TestConfigValidatesWireViewsAndProjections(t *testing.T) {
	v, err := view(fixtureSnapshot())
	if err != nil {
		t.Fatal(err)
	}
	if err = v.Snapshot.Pins.Validate(); err != nil {
		t.Fatal(err)
	}
	if err = ValidateView(v); err != nil {
		t.Fatal(err)
	}
	for _, replay := range []bool{false, true} {
		p := Projection{Next: v, Replay: replay}
		if replay {
			p.Base = v.Head
		}
		if err = ValidateProjection(p); err != nil {
			t.Fatal(err)
		}
	}
	for _, name := range []string{"head", "snapshot", "time", "empty"} {
		t.Run(name, func(t *testing.T) {
			bad := v
			bad.Snapshot = owned(v.Snapshot)
			switch name {
			case "head":
				bad.Head.Digest = strings.Repeat("f", 64)
			case "snapshot":
				bad.Snapshot.History[0].Change.Reason = "changed"
			case "time":
				bad.Snapshot.History[0].At = bad.Snapshot.History[0].At.In(time.FixedZone("offset", 3600))
			case "empty":
				bad = View{}
			}
			if ValidateView(bad) == nil {
				t.Fatal("invalid view accepted")
			}
		})
	}
	for _, p := range []Projection{{Next: v, Base: v.Head}, {Next: v, Replay: true}, {Next: v, Base: Head{Revision: 1, Digest: "bad"}, Replay: true}} {
		if ValidateProjection(p) == nil {
			t.Fatal("invalid projection accepted")
		}
	}
	badPins := fixturePins()
	badPins.PublicKey = ""
	if badPins.Validate() == nil {
		t.Fatal("invalid independent pins accepted")
	}
}

func TestConfigCanonicalCodecAndExactRevisions(t *testing.T) {
	snapshot := fixtureSnapshot()
	raw, err := Encode(snapshot)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(raw, []byte(`"revision":"0"`)) {
		t.Fatalf("revision must be a decimal string: %s", raw)
	}
	decoded, err := Decode(raw)
	if err != nil {
		t.Fatal(err)
	}
	decoded.History[0].Change.Values.SocketPath = "/changed.sock"
	if snapshot.History[0].Change.Values.SocketPath == "/changed.sock" {
		t.Fatal("decoded slice aliases producer")
	}
	var head Head
	if err = json.Unmarshal([]byte(`{"revision":"18446744073709551615","digest":"`+strings.Repeat("a", 64)+`"}`), &head); err != nil || head.Revision != ^uint64(0) {
		t.Fatalf("exact uint64: %v %v", head, err)
	}
	if err = json.Unmarshal([]byte(`{"revision":"18446744073709551616","digest":""}`), &head); err == nil {
		t.Fatal("revision overflow accepted")
	}
	if err = json.Unmarshal([]byte(`{"revision":1,"digest":""}`), &head); err == nil {
		t.Fatal("numeric revision accepted")
	}
}

func TestConfigRejectsMalformedOrUnsupportedFrames(t *testing.T) {
	raw, _ := Encode(fixtureSnapshot())
	cases := map[string][]byte{
		"missing-version":  bytes.Replace(raw, []byte(`"schema_version":1,`), nil, 1),
		"future-version":   bytes.Replace(raw, []byte(`"schema_version":1`), []byte(`"schema_version":2`), 1),
		"null-version":     bytes.Replace(raw, []byte(`"schema_version":1`), []byte(`"schema_version":null`), 1),
		"unknown":          append([]byte(`{"unknown":true,`), raw[1:]...),
		"duplicate":        append([]byte(`{"schema_version":1,`), raw[1:]...),
		"folded":           bytes.Replace(raw, []byte(`"schema_version"`), []byte(`"Schema_Version"`), 1),
		"nested-unknown":   bytes.Replace(raw, []byte(`"mode":"local"`), []byte(`"mode":"local","tls_off":true`), 1),
		"nested-duplicate": bytes.Replace(raw, []byte(`"mode":"local"`), []byte(`"mode":"local","mode":"local"`), 1),
		"whitespace":       append([]byte(" "), raw...),
		"trailing":         append(append([]byte(nil), raw...), []byte("{}")...),
		"numeric-revision": bytes.Replace(raw, []byte(`"revision":"0"`), []byte(`"revision":0`), 1),
		"null-history":     bytes.Replace(raw, raw[bytes.Index(raw, []byte(`"history":`)):], []byte(`"history":null}`), 1),
	}
	for name, input := range cases {
		t.Run(name, func(t *testing.T) {
			out, err := Decode(input)
			if err == nil || len(out.History) != 0 || out.Schema != 0 {
				t.Fatalf("partial/successful decode: %#v %v", out, err)
			}
		})
	}
}

func TestConfigValuesPinsAndHistoryValidation(t *testing.T) {
	cases := map[string]func(*Snapshot){
		"remote-unimplemented": func(s *Snapshot) { s.History[0].Change.Values.Mode = "remote" },
		"unknown-mode":         func(s *Snapshot) { s.History[0].Change.Values.Mode = "automatic" },
		"relative":             func(s *Snapshot) { s.History[0].Change.Values.SocketPath = "authority.sock" },
		"parent":               func(s *Snapshot) { s.History[0].Change.Values.SocketPath = "/run/../authority.sock" },
		"root":                 func(s *Snapshot) { s.History[0].Change.Values.SocketPath = "/" },
		"control":              func(s *Snapshot) { s.History[0].Change.Values.SocketPath = "/run/a\x00sock" },
		"too-long-socket": func(s *Snapshot) {
			s.History[0].Change.Values.SocketPath = "/" + strings.Repeat("x", LocalSocketPathMaxBytes)
		},
		"bad-key":           func(s *Snapshot) { s.Pins.PublicKey = "not-a-key" },
		"padded-key":        func(s *Snapshot) { s.Pins.PublicKey += "=" },
		"bad-installation":  func(s *Snapshot) { s.Pins.InstallationDigest = "" },
		"service-principal": func(s *Snapshot) { s.Pins.Principal.Kind = "service" },
		"wrong-actor":       func(s *Snapshot) { s.History[0].Actor.Subject = "another" },
		"no-clock":          func(s *Snapshot) { s.History[0].At = time.Time{} },
		"empty-reason":      func(s *Snapshot) { s.History[0].Change.Reason = " " },
		"wrong-base":        func(s *Snapshot) { s.History[0].Change.Expected = Head{Revision: 1, Digest: strings.Repeat("a", 64)} },
		"empty-history":     func(s *Snapshot) { s.History = nil },
		"duplicate-event":   func(s *Snapshot) { s.History = append(s.History, s.History[0]) },
	}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			s := fixtureSnapshot()
			mutate(&s)
			if _, err := Encode(s); err == nil {
				t.Fatal("invalid configuration accepted")
			}
			if name == "remote-unimplemented" {
				if _, err := Encode(s); !errors.Is(err, ErrMode) {
					t.Fatalf("remote cause: %v", err)
				}
			}
		})
	}
	s := fixtureSnapshot()
	s.History[0].Change.Values.SocketPath = "/" + strings.Repeat("x", LocalSocketPathMaxBytes-1)
	if _, err := Encode(s); err != nil {
		t.Fatalf("actual Linux socket bound: %v", err)
	}
	s.History[0].At = s.History[0].At.In(time.FixedZone("fixture", 3600))
	raw, err := Encode(s)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Contains(raw, []byte(`"at":"2026-10-17T12:00:00Z"`)) {
		t.Fatal("clock not normalized")
	}
}

func TestConfigChangeUsesSharedStrictWireDecoder(t *testing.T) {
	change := fixtureChange("initial", Head{})
	raw, err := EncodeChange(change)
	if err != nil {
		t.Fatal(err)
	}
	decoded, err := DecodeChange(raw)
	if err != nil || decoded != change {
		t.Fatalf("round trip: %#v %v", decoded, err)
	}
	cases := map[string][]byte{
		"missing-head":         bytes.Replace(raw, []byte(`"expected_head":{"revision":"0","digest":""},`), nil, 1),
		"null-head":            bytes.Replace(raw, []byte(`"expected_head":{"revision":"0","digest":""}`), []byte(`"expected_head":null`), 1),
		"duplicate":            append([]byte(`{"event_id":"initial",`), raw[1:]...),
		"folded":               bytes.Replace(raw, []byte(`"event_id"`), []byte(`"Event_Id"`), 1),
		"unknown-root":         append([]byte(`{"pins":{},`), raw[1:]...),
		"private-key":          append([]byte(`{"private_key":"forbidden",`), raw[1:]...),
		"nested-duplicate":     bytes.Replace(raw, []byte(`"mode":"local"`), []byte(`"mode":"local","mode":"local"`), 1),
		"future-version":       bytes.Replace(raw, []byte(`"schema_version":1`), []byte(`"schema_version":2`), 1),
		"remote-unimplemented": bytes.Replace(raw, []byte(`"mode":"local"`), []byte(`"mode":"remote"`), 1),
		"whitespace":           append([]byte(" "), raw...),
		"trailing":             append(append([]byte(nil), raw...), []byte("{}")...),
	}
	for name, input := range cases {
		t.Run(name, func(t *testing.T) {
			got, err := DecodeChange(input)
			if err == nil || got != (Change{}) {
				t.Fatalf("partial or accepted request: %#v %v", got, err)
			}
		})
	}
}
