//go:build linux

package enrollmentconfig

import (
	"bytes"
	"testing"
)

func TestWitnessStrictCodecAndIntentBinding(t *testing.T) {
	pins := fixturePins()
	entry := Entry{Change: fixtureChange("initial", Head{}), Actor: pins.Principal, At: fixtureAt()}
	target, err := entryHead(pins, entry)
	if err != nil {
		t.Fatal(err)
	}
	w := witnessRecord{Schema: 1, Pins: pins, Committed: Head{}, Pending: &witnessIntent{Entry: entry, Target: target}}
	raw, err := encodeWitness(w)
	if err != nil {
		t.Fatal(err)
	}
	decoded, err := decodeWitness(raw)
	if err != nil || decoded.Pending.Target != target {
		t.Fatalf("round trip: %#v %v", decoded, err)
	}
	cases := map[string][]byte{
		"unknown":          append([]byte(`{"unknown":true,`), raw[1:]...),
		"duplicate":        append([]byte(`{"schema_version":1,`), raw[1:]...),
		"future":           bytes.Replace(raw, []byte(`"schema_version":1`), []byte(`"schema_version":2`), 1),
		"null-version":     bytes.Replace(raw, []byte(`"schema_version":1`), []byte(`"schema_version":null`), 1),
		"folded":           bytes.Replace(raw, []byte(`"schema_version"`), []byte(`"Schema_Version"`), 1),
		"numeric-revision": bytes.Replace(raw, []byte(`"revision":"0"`), []byte(`"revision":0`), 1),
		"trailing":         append(append([]byte(nil), raw...), []byte("{}")...),
		"whitespace":       append([]byte(" "), raw...),
	}
	for name, input := range cases {
		t.Run(name, func(t *testing.T) {
			got, err := decodeWitness(input)
			if err == nil || got.Schema != 0 || got.Pending != nil {
				t.Fatalf("partial/successful witness: %#v %v", got, err)
			}
		})
	}
	for _, name := range []string{"wrong-target", "wrong-base", "wrong-actor", "empty-committed"} {
		t.Run(name, func(t *testing.T) {
			x := w
			intent := *w.Pending
			x.Pending = &intent
			switch name {
			case "wrong-target":
				x.Pending.Target.Revision++
			case "wrong-base":
				x.Committed = target
			case "wrong-actor":
				x.Pending.Entry.Actor.Subject = "other"
			case "empty-committed":
				x.Pending = nil
			}
			if _, err := encodeWitness(x); err == nil {
				t.Fatal("invalid intent accepted")
			}
		})
	}
}
