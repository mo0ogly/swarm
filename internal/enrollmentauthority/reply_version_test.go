package enrollmentauthority

import (
	"encoding/json"
	"errors"
	"reflect"
	"strings"
	"testing"

	"swarm.local/companion/internal/enrollmentstate"
)

// Every payload is signed by the pinned fixture root for a fresh real challenge.
// Authenticity must not make an unsupported or malformed read format acceptable.
func TestActiveReplyRejectsAuthenticUnsupportedFormats(t *testing.T) {
	f := setup(t)
	body, err := Encode(f.doc)
	if err != nil {
		t.Fatal(err)
	}
	legacy, err := json.Marshal(struct {
		Head   enrollmentstate.Head   `json:"head"`
		Record enrollmentstate.Record `json:"record"`
	}{f.head, enrollmentstate.Record{ID: f.doc.EnrollmentID, Body: body, EnrolledSequence: 1}})
	if err != nil {
		t.Fatal(err)
	}
	for _, kind := range []string{"absent", "zero", "future", "negative", "null", "string", "duplicate", "folded", "extra"} {
		t.Run(kind, func(t *testing.T) {
			raw := string(legacy)
			switch kind {
			case "zero":
				raw = strings.Replace(raw, `{`, `{"schema_version":0,`, 1)
			case "future":
				raw = strings.Replace(raw, `{`, `{"schema_version":2,`, 1)
			case "negative":
				raw = strings.Replace(raw, `{`, `{"schema_version":-1,`, 1)
			case "null":
				raw = strings.Replace(raw, `{`, `{"schema_version":null,`, 1)
			case "string":
				raw = strings.Replace(raw, `{`, `{"schema_version":"1",`, 1)
			case "duplicate":
				raw = strings.Replace(raw, `{`, `{"schema_version":1,"schema_version":1,`, 1)
			case "folded":
				raw = strings.Replace(raw, `{`, `{"Schema_Version":1,`, 1)
			case "extra":
				raw = strings.Replace(raw, `{`, `{"schema_version":1,"assume_active":true,`, 1)
			}
			got, err := DecodeActive(f.statement(t, []byte(raw)), f.expected, f.at)
			if err == nil || !reflect.DeepEqual(got, Document{}) {
				t.Fatal("authentic unsupported frame became authority", kind, err)
			}
			if !errors.Is(err, ErrDocument) {
				t.Fatal("document error classification lost", err)
			}
			if (kind == "absent" || kind == "zero" || kind == "future" || kind == "negative") && !errors.Is(err, enrollmentstate.ErrReplyVersion) {
				t.Fatal("version cause lost at consumer", err)
			}
		})
	}
}
