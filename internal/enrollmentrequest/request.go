// Package enrollmentrequest binds a complete registry-publication proposal to
// an enrollment challenge. Prepared values are immutable owned data, not proof
// that their declared principal, session, plan or witness is currently trusted.
package enrollmentrequest

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"time"

	"swarm.local/companion/internal/enrollmentauthority"
	"swarm.local/companion/internal/enrollmentproof"
	"swarm.local/companion/internal/enrollmentstate"
	"swarm.local/companion/internal/identitycontract"
)

const (
	PublicationPurpose = "agent_identity_registry_publication"
	protocol           = "swarm-enrollment-publication-request-v1"
)

var (
	ErrRequest = errors.New("enrollment request: invalid canonical publication")
	ErrBinding = errors.New("enrollment request: context or document mismatch")
	ErrSession = errors.New("enrollment request: outside explicit observed session window")
)

type Enrollment struct {
	ID             string               `json:"id"`
	Generation     int64                `json:"generation"`
	Head           enrollmentstate.Head `json:"head"`
	DocumentDigest string               `json:"document_digest"`
}

// Publication carries the full snapshot bytes, not a projection or a supplied
// digest alone. Snapshot record semantics are checked by the engine producer.
// Roots keep their exact order, as in the existing registry publication head.
type Publication struct {
	Schema               int                        `json:"schema_version"`
	Purpose              string                     `json:"purpose"`
	ProjectDigest        string                     `json:"project_context_digest"`
	Principal            identitycontract.Principal `json:"principal"`
	SessionDigest        string                     `json:"session_digest"`
	SessionVerifiedAt    time.Time                  `json:"session_verified_at"`
	SessionExpiresAt     time.Time                  `json:"session_expires_at"`
	Scope                identitycontract.Scope     `json:"scope"`
	TrustRevision        int                        `json:"trust_configuration_revision"`
	Enrollment           Enrollment                 `json:"enrollment"`
	EventID              string                     `json:"event_id"`
	SnapshotRevision     int                        `json:"snapshot_revision"`
	SnapshotDigest       string                     `json:"snapshot_digest"`
	Snapshot             []byte                     `json:"snapshot"`
	Roots                []string                   `json:"root_grants"`
	PreviousRegistryHead string                     `json:"previous_registry_head_digest"`
}

// Prepared stores immutable encoded bytes. Value copies cannot retain writable
// slices or change a previously computed request digest. JSON cannot populate it.
// Decode can prepare arbitrary valid data; that does not authenticate its source.
type Prepared struct {
	raw    string
	digest [sha256.Size]byte
}

func digestText(raw []byte) string {
	digest := sha256.Sum256(raw)
	return hex.EncodeToString(digest[:])
}

func (p Publication) valid() bool {
	if p.Schema != 1 || p.Purpose != PublicationPurpose || !identitycontract.Digest(p.ProjectDigest) || !p.Principal.Valid() || p.Principal.Kind != "human" ||
		!identitycontract.Digest(p.SessionDigest) || p.SessionVerifiedAt.IsZero() || !p.SessionVerifiedAt.Before(p.SessionExpiresAt) || !p.Scope.Valid() || p.TrustRevision <= 0 ||
		!identitycontract.Text(p.Enrollment.ID) || p.Enrollment.Generation <= 0 || p.Enrollment.Head.Sequence < p.Enrollment.Generation || !identitycontract.Digest(p.Enrollment.Head.Digest) || !identitycontract.Digest(p.Enrollment.DocumentDigest) ||
		!identitycontract.Name(p.EventID) || p.SnapshotRevision <= 0 || !identitycontract.Digest(p.SnapshotDigest) || p.SnapshotDigest != digestText(p.Snapshot) || len(p.Roots) == 0 ||
		(p.PreviousRegistryHead != "" && !identitycontract.Digest(p.PreviousRegistryHead)) {
		return false
	}
	// This is an exact byte binding to a JSON object, not a second implementation
	// of the engine's snapshot schema, grant graph or transition authorization.
	snapshot := bytes.TrimSpace(p.Snapshot)
	if !json.Valid(snapshot) || len(snapshot) == 0 || snapshot[0] != '{' {
		return false
	}
	seen := map[string]bool{}
	for _, id := range p.Roots {
		if !identitycontract.Name(id) || seen[id] {
			return false
		}
		seen[id] = true
	}
	return true
}

// Prepare validates and owns the complete input. It applies no TTL, default
// duration, role promotion, snapshot projection or root-order normalization.
func Prepare(p Publication) (Prepared, error) {
	if !p.valid() {
		return Prepared{}, ErrRequest
	}
	p.SessionVerifiedAt, p.SessionExpiresAt = p.SessionVerifiedAt.UTC(), p.SessionExpiresAt.UTC()
	raw, err := json.Marshal(p)
	if err != nil {
		return Prepared{}, ErrRequest
	}
	signed := append([]byte(protocol+"\x00request\x00"), raw...)
	return Prepared{raw: string(raw), digest: sha256.Sum256(signed)}, nil
}

func Decode(raw []byte) (Prepared, error) {
	var p Publication
	if err := json.Unmarshal(raw, &p); err != nil {
		return Prepared{}, ErrRequest
	}
	prepared, err := Prepare(p)
	if err != nil || !bytes.Equal(raw, prepared.Bytes()) {
		return Prepared{}, ErrRequest
	}
	return prepared, nil
}

func (p Prepared) Bytes() []byte { return []byte(p.raw) }

func (p Prepared) Digest() ([sha256.Size]byte, error) {
	if p.raw == "" {
		return [sha256.Size]byte{}, ErrRequest
	}
	return p.digest, nil
}

// Publication returns a new owned decoding; no writable alias of Prepared exists.
func (p Prepared) Publication() (Publication, error) {
	if p.raw == "" {
		return Publication{}, ErrRequest
	}
	var result Publication
	if err := json.Unmarshal([]byte(p.raw), &result); err != nil {
		return Publication{}, ErrRequest
	}
	return result, nil
}

func (p Prepared) Expected() (enrollmentauthority.Expected, error) {
	d, err := p.Publication()
	if err != nil {
		return enrollmentauthority.Expected{}, err
	}
	return enrollmentauthority.Expected{Head: d.Enrollment.Head, EnrollmentID: d.Enrollment.ID, Generation: d.Enrollment.Generation,
		ProjectDigest: d.ProjectDigest, Principal: d.Principal, Scope: d.Scope, TrustRevision: d.TrustRevision, RequestDigest: hex.EncodeToString(p.digest[:])}, nil
}

func (p Prepared) NewChallenge(v *enrollmentproof.Verifier) (*enrollmentproof.Challenge, error) {
	digest, err := p.Digest()
	if err != nil {
		return nil, err
	}
	challenge, err := v.NewChallenge(digest)
	if err != nil {
		return nil, err
	}
	if err = p.Bind(challenge.Request()); err != nil {
		return nil, err
	}
	return challenge, nil
}

// Bind compares the full request digest and project. It does not authenticate
// a peer or validate the cryptographic envelope; SignResponse/Verify do that.
// A service must receive and bind the full descriptor before deciding to sign.
func (p Prepared) Bind(request enrollmentproof.Request) error {
	expected, err := p.Expected()
	if err != nil {
		return err
	}
	if request.ProjectDigest != expected.ProjectDigest || request.RequestDigest != expected.RequestDigest {
		return ErrBinding
	}
	return nil
}

// CheckActive checks the exact returned authority body and explicit session
// window, in addition to DecodeActive's context checks. Neither a declared
// session window nor its digest proves current authentication or non-revocation.
func (p Prepared) CheckActive(statement enrollmentproof.Statement, at time.Time) (enrollmentauthority.Document, error) {
	d, err := p.Publication()
	if err != nil {
		return enrollmentauthority.Document{}, err
	}
	if at.IsZero() || at.Before(d.SessionVerifiedAt) || !at.Before(d.SessionExpiresAt) {
		return enrollmentauthority.Document{}, ErrSession
	}
	expected, err := p.Expected()
	if err != nil {
		return enrollmentauthority.Document{}, err
	}
	authority, err := enrollmentauthority.DecodeActive(statement, expected, at)
	if err != nil {
		return enrollmentauthority.Document{}, err
	}
	body, err := enrollmentauthority.Encode(authority)
	if err != nil || digestText(body) != d.Enrollment.DocumentDigest {
		return enrollmentauthority.Document{}, ErrBinding
	}
	return authority, nil
}
