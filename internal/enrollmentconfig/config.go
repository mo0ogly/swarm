//go:build linux

// Package enrollmentconfig stores desired installation configuration, not
// effective service state or an identity capability. Trusted bootstrap supplies
// pins and the authorizer independently of projects and request bodies. The
// Coordinator adds a local durable witness. Its independent deployment/protection,
// the protected service and public adapters remain separate obligations.
package enrollmentconfig

import (
	"bytes"
	"context"
	"crypto/ed25519"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"time"

	"golang.org/x/sys/unix"
	"swarm.local/companion/internal/identitycontract"
)

// Linux sockaddr_un requires a terminating NUL for a filesystem socket. This is
// a transport representation bound, not an operator policy or waiting limit.
const LocalSocketPathMaxBytes = len(unix.RawSockaddrUnix{}.Path) - 1

var (
	ErrFormat        = errors.New("enrollment config: invalid canonical configuration")
	ErrVersion       = errors.New("enrollment config: unsupported schema version")
	ErrMode          = errors.New("enrollment config: remote mode not implemented")
	ErrPins          = errors.New("enrollment config: independent bootstrap pins do not match")
	ErrHead          = errors.New("enrollment config: independent checkpoint does not match")
	ErrConflict      = errors.New("enrollment config: expected revision has changed")
	ErrEvent         = errors.New("enrollment config: event reused or no longer current")
	ErrUnauthorized  = errors.New("enrollment config: authenticated bootstrap authority required")
	ErrBusy          = errors.New("enrollment config: configuration reservation busy")
	ErrNotConfigured = errors.New("enrollment config: explicit bootstrap required")
	ErrExists        = errors.New("enrollment config: installation already configured")
	ErrPath          = errors.New("enrollment config: unsafe owned storage path or file")
	ErrStorage       = errors.New("enrollment config: storage operation failed")
	ErrIndeterminate = errors.New("enrollment config: replacement observed but durable outcome uncertain")
)

// Pins are public bootstrap data, not obtained from a config file being opened.
// Root/principal migration is intentionally not a regular configuration change.
type Pins struct {
	InstallationDigest string                     `json:"installation_digest"`
	PublicKey          string                     `json:"authority_public_key"`
	Principal          identitycontract.Principal `json:"bootstrap_principal"`
}

func (p Pins) valid() bool {
	key, err := base64.RawURLEncoding.Strict().DecodeString(p.PublicKey)
	return err == nil && len(key) == ed25519.PublicKeySize && base64.RawURLEncoding.EncodeToString(key) == p.PublicKey &&
		identitycontract.Digest(p.InstallationDigest) && p.Principal.Valid() && p.Principal.Kind == "human"
}

// Validate checks representation only; pins still require independent bootstrap.
func (p Pins) Validate() error {
	if !p.valid() {
		return ErrPins
	}
	return nil
}

type Values struct {
	Mode       string `json:"mode"`
	SocketPath string `json:"local_socket_path"`
}

func (v Values) validate() error {
	if v.Mode == "remote" {
		return ErrMode
	}
	if v.Mode != "local" || !identitycontract.Text(v.SocketPath) || !filepath.IsAbs(v.SocketPath) ||
		filepath.Clean(v.SocketPath) != v.SocketPath || v.SocketPath == "/" || len(v.SocketPath) > LocalSocketPathMaxBytes {
		return ErrFormat
	}
	return nil
}

// Decimal strings preserve all uint64 values across JavaScript clients.
type Head struct {
	Revision uint64 `json:"revision,string"`
	Digest   string `json:"digest"`
}

func (h Head) valid() bool {
	return h.Revision == 0 && h.Digest == "" || h.Revision > 0 && identitycontract.Digest(h.Digest)
}

type Change struct {
	Schema   int    `json:"schema_version"`
	EventID  string `json:"event_id"`
	Expected Head   `json:"expected_head"`
	Values   Values `json:"values"`
	Reason   string `json:"reason"`
}

func (c Change) validate() error {
	if c.Schema != 1 {
		return ErrVersion
	}
	if !identitycontract.Text(c.EventID) || !identitycontract.Text(c.Reason) || !c.Expected.valid() {
		return ErrFormat
	}
	return c.Values.validate()
}

// EncodeChange and DecodeChange are the shared wire boundary for future setup,
// CLI, API and Admin adapters. They grant no right to call Create or Apply.
func EncodeChange(c Change) ([]byte, error) {
	if err := c.validate(); err != nil {
		return nil, err
	}
	return json.Marshal(c)
}

func DecodeChange(raw []byte) (Change, error) {
	var c Change
	if err := json.Unmarshal(raw, &c); err != nil {
		return Change{}, fmt.Errorf("%w: %w", ErrFormat, err)
	}
	canonical, err := EncodeChange(c)
	if err != nil {
		return Change{}, err
	}
	if !bytes.Equal(raw, canonical) {
		return Change{}, ErrFormat
	}
	return c, nil
}

type Entry struct {
	Change Change                     `json:"change"`
	Actor  identitycontract.Principal `json:"actor"`
	At     time.Time                  `json:"at"`
}

type Snapshot struct {
	Schema  int     `json:"schema_version"`
	Pins    Pins    `json:"pins"`
	History []Entry `json:"history"`
}

// View is saved configuration data. It says nothing about the effective service,
// confinement, OIDC authentication or final-use authority.
type View struct {
	Snapshot Snapshot `json:"snapshot"`
	Head     Head     `json:"head"`
}

// Projection has a distinct type so a preview is not mistaken for a saved View.
type Projection struct {
	Base   Head `json:"base"`
	Next   View `json:"proposed"`
	Replay bool `json:"current_event_replay"`
}

// ValidateView uses the same canonical history and head calculation as storage.
// It authenticates neither the source of the view nor its current authority.
func ValidateView(v View) error {
	raw, err := json.Marshal(v.Snapshot)
	if err != nil {
		return fmt.Errorf("%w: %w", ErrFormat, err)
	}
	snapshot, err := Decode(raw)
	if err != nil {
		return err
	}
	expected, err := view(snapshot)
	if err != nil {
		return err
	}
	if v.Head != expected.Head {
		return ErrHead
	}
	return nil
}

func ValidateProjection(p Projection) error {
	if err := ValidateView(p.Next); err != nil {
		return err
	}
	if !p.Base.valid() {
		return ErrHead
	}
	if p.Replay {
		if p.Base != p.Next.Head {
			return ErrHead
		}
	} else if p.Next.Snapshot.History[len(p.Next.Snapshot.History)-1].Change.Expected != p.Base {
		return ErrHead
	}
	return nil
}

// Authorizer runs under the configuration reservation with owned copies. It must
// resolve authenticated authority, not echo an identity from Change. It must not
// reenter this Store. Matching a returned principal is not authentication itself.
type Authorizer func(context.Context, *Snapshot, Change) (identitycontract.Principal, error)

func owned(s Snapshot) Snapshot {
	s.History = append([]Entry(nil), s.History...)
	return s
}

func entryHead(p Pins, e Entry) (Head, error) {
	raw, err := json.Marshal(struct {
		Pins  Pins  `json:"pins"`
		Entry Entry `json:"entry"`
	}{p, e})
	if err != nil {
		return Head{}, fmt.Errorf("%w: %w", ErrFormat, err)
	}
	digest := sha256.Sum256(append([]byte("swarm-installation-config-v1\x00entry\x00"), raw...))
	return Head{Revision: e.Change.Expected.Revision + 1, Digest: hex.EncodeToString(digest[:])}, nil
}

func view(s Snapshot) (View, error) {
	if len(s.History) == 0 {
		return View{}, ErrFormat
	}
	head, err := entryHead(s.Pins, s.History[len(s.History)-1])
	if err != nil {
		return View{}, err
	}
	return View{Snapshot: owned(s), Head: head}, nil
}

// Encode validates the entire chain and canonicalizes timestamps on a copy.
func Encode(s Snapshot) ([]byte, error) {
	if s.Schema != 1 {
		return nil, ErrVersion
	}
	if !s.Pins.valid() || len(s.History) == 0 {
		return nil, ErrFormat
	}
	s = owned(s)
	base := Head{}
	events := map[string]bool{}
	for i, e := range s.History {
		if err := e.Change.validate(); err != nil {
			return nil, err
		}
		if e.Change.Expected != base || events[e.Change.EventID] || e.Actor != s.Pins.Principal || e.At.IsZero() || base.Revision == ^uint64(0) {
			return nil, ErrFormat
		}
		events[e.Change.EventID] = true
		e.At = e.At.UTC()
		s.History[i] = e
		next, err := entryHead(s.Pins, e)
		if err != nil {
			return nil, err
		}
		base = next
	}
	raw, err := json.Marshal(s)
	if err != nil {
		return nil, fmt.Errorf("%w: %w", ErrFormat, err)
	}
	return raw, nil
}

// Decode rejects noncanonical, duplicate, unknown, folded, missing or null fields
// at every level, rather than repairing untrusted configuration into authority.
func Decode(raw []byte) (Snapshot, error) {
	var s Snapshot
	if err := json.Unmarshal(raw, &s); err != nil {
		return Snapshot{}, fmt.Errorf("%w: %w", ErrFormat, err)
	}
	canonical, err := Encode(s)
	if err != nil {
		return Snapshot{}, err
	}
	if !bytes.Equal(raw, canonical) {
		return Snapshot{}, ErrFormat
	}
	return s, nil
}
