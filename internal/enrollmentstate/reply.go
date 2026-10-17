package enrollmentstate

import (
	"bytes"
	"encoding/json"
	"errors"
	"strings"
	"unicode"
	"unicode/utf8"
)

// ActiveReplySchema is the only supported read-envelope version. No implicit
// legacy format or downgrade negotiation is accepted. This does not version the
// SQLite schema, signed changes, challenge envelope or authority document.
const ActiveReplySchema = 1

var (
	ErrReply        = errors.New("enrollment state: malformed canonical active reply")
	ErrReplyVersion = errors.New("enrollment state: unsupported active reply schema")
)

// ActiveReply describes state held under the local writer reservation. Its
// explicit version is covered by the signature along with the rest of the bytes.
// A reader still needs authentication, independent context and final freshness.
type ActiveReply struct {
	Schema int    `json:"schema_version"`
	Head   Head   `json:"head"`
	Record Record `json:"record"`
}

func validEnrollmentID(id string) bool {
	return utf8.ValidString(id) && strings.TrimSpace(id) != "" && !strings.ContainsFunc(id, unicode.IsControl)
}

// DecodeActiveReply validates format only, not signature, authority or freshness.
// Call it on an authenticated Statement's payload before interpreting its body.
// A well-formed unsupported version remains an error, even when signed.
func DecodeActiveReply(raw []byte) (ActiveReply, error) {
	var result ActiveReply
	if err := json.Unmarshal(raw, &result); err != nil {
		return ActiveReply{}, ErrReply
	}
	if result.Schema != ActiveReplySchema {
		return ActiveReply{}, ErrReplyVersion
	}
	canonical, err := json.Marshal(result)
	if err != nil || !bytes.Equal(raw, canonical) || result.Head.Sequence <= 0 || !validHead(result.Head) ||
		!validEnrollmentID(result.Record.ID) || result.Record.EnrolledSequence <= 0 || result.Record.EnrolledSequence > result.Head.Sequence || result.Record.Body == nil {
		return ActiveReply{}, ErrReply
	}
	return result, nil
}
