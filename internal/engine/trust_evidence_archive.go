//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"golang.org/x/sys/unix"
)

type TrustEvidenceArchive struct {
	ID             string     `json:"archive_id"`
	Sequence       int        `json:"sequence"`
	Hash           string     `json:"hash"`
	KeyID          string     `json:"key_id"`
	CreatedAt      time.Time  `json:"created_at"`
	CreatedBy      string     `json:"created_by"`
	RetentionDays  int        `json:"retention_days"`
	PolicyRevision int        `json:"policy_revision"`
	ReleasedAt     *time.Time `json:"released_at,omitempty"`
	ReleasedBy     string     `json:"released_by,omitempty"`
}
type trustArchiveBundle struct {
	Schema    int               `json:"schema_version"`
	Export    TrustProofExport  `json:"export"`
	Anchor    TrustPublicAnchor `json:"anchor"`
	PublicKey TrustEvidenceKey  `json:"public_key_at_archive"`
}

func trustArchiveID(id string) bool {
	raw, err := hex.DecodeString(id)
	return err == nil && len(raw) == 32
}
func archiveProtection(a TrustEvidenceArchive, now time.Time) map[string]any {
	seconds := now.Unix() - a.CreatedAt.Unix()
	if now.Nanosecond() < a.CreatedAt.Nanosecond() {
		seconds--
	}
	elapsed := seconds >= 0 && seconds/86400 >= int64(a.RetentionDays)
	return map[string]any{"retention_days": a.RetentionDays, "retention_started_at": a.CreatedAt, "minimum_retention_satisfied": elapsed, "protected": a.ReleasedAt == nil, "released_at": a.ReleasedAt, "originals_deleted": false}
}
func evidencePrefix(e TrustProofExport, n int) TrustProofExport {
	p := TrustProofExport{Schema: e.Schema, Simulation: e.Simulation, Evidence: e.Evidence[:n], Originals: map[string][]byte{}}
	for _, ev := range p.Evidence {
		p.Originals[ev.OriginalHash] = e.Originals[ev.OriginalHash]
	}
	return p
}
func (s *Store) evidenceArchiveDir(create bool) (*os.File, error) {
	state, _, _ := s.trustPaths()
	fd, err := unix.Open(filepath.Dir(state), unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC|unix.O_NOFOLLOW, 0)
	if err != nil {
		return nil, err
	}
	defer unix.Close(fd)
	if create {
		if err = unix.Mkdirat(fd, "trust-archives", 0700); err != nil && err != unix.EEXIST {
			return nil, err
		}
	}
	child, err := unix.Openat(fd, "trust-archives", unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC|unix.O_NOFOLLOW, 0)
	if err != nil {
		return nil, err
	}
	f := os.NewFile(uintptr(child), "trust-archives")
	info, err := f.Stat()
	if err != nil || info.Mode().Perm()&0077 != 0 {
		f.Close()
		return nil, trustEvidenceInvalid()
	}
	return f, nil
}
func archiveFile(dir *os.File, id string) string {
	return fmt.Sprintf("/proc/self/fd/%d/%s.json", dir.Fd(), id)
}
func (s *Store) readEvidenceArchive(a TrustEvidenceArchive) (trustArchiveBundle, error) {
	var b trustArchiveBundle
	dir, err := s.evidenceArchiveDir(false)
	if err != nil {
		return b, err
	}
	defer dir.Close()
	raw, err := trustReadPrivate(archiveFile(dir, a.ID), 2*trustStateMaxBytes)
	if err != nil {
		return b, err
	}
	if hash(raw) != a.ID || strict(raw, &b) != nil || b.Schema != 1 || b.Anchor.Sequence != a.Sequence || b.Anchor.Hash != a.Hash || b.Anchor.KeyID != a.KeyID {
		return b, trustEvidenceInvalid()
	}
	if _, err = verifyPublicEvidence(TrustEvidenceCommand{Export: &b.Export, Anchor: &b.Anchor, TrustedKey: &b.PublicKey}); err != nil {
		return b, err
	}
	return b, nil
}
func (s *Store) trustArchiveOperation(ctx context.Context, op string, c TrustEvidenceCommand, raw []byte, token string, at time.Time) (any, error) {
	return s.withTrustLock(op == "archive-create" || op == "archive-release", func(st *trustState) (any, error) {
		now := at
		if now.IsZero() {
			now = time.Now().UTC()
		}
		id, err := findTrustIdentityAt(st, token, now)
		if err != nil || !trustSecretAdmin(id) || ctx.Err() != nil {
			return nil, trustEvidenceDenied()
		}
		r := evidenceRegistry(st)
		if op == "archive-show" {
			a, ok := r.Archives[c.ArchiveID]
			if !trustArchiveID(c.ArchiveID) || !ok {
				return nil, trustEvidenceInvalid()
			}
			b, err := s.readEvidenceArchive(a)
			if err != nil {
				return nil, err
			}
			return map[string]any{"archive": a, "bundle": b, "protection": archiveProtection(a, now), "current_key_state": r.Keys[a.KeyID].State}, nil
		}
		digest := hashMust(struct {
			Operation string
			Command   TrustEvidenceCommand
		}{op, c})
		if prior, ok := r.Replies[c.EventID]; ok {
			if prior.Digest != digest {
				return nil, &CommandError{Code: "event_conflict", Message: "événement d’archive réutilisé avec un autre contenu"}
			}
			if prior.Authority != trustAuthorityDigest(id) {
				return nil, trustEvidenceDenied()
			}
			var value map[string]any
			if json.Unmarshal(prior.Value, &value) != nil {
				return nil, trustEvidenceInvalid()
			}
			if op == "archive-create" {
				archiveID, typed := value["archive_id"].(string)
				if !typed {
					return nil, trustEvidenceInvalid()
				}
				a, ok := r.Archives[archiveID]
				if !ok {
					return nil, trustEvidenceInvalid()
				}
				if _, err := s.readEvidenceArchive(a); err != nil {
					return nil, err
				}
			}
			return value, nil
		}
		if c.ExpectedRevision == nil || *c.ExpectedRevision != st.Revision {
			return nil, &CommandError{Code: "revision_conflict", Message: "configuration modifiée ; rechargez avant de confirmer"}
		}
		var a TrustEvidenceArchive
		if op == "archive-release" {
			var ok bool
			a, ok = r.Archives[c.ArchiveID]
			if !ok || a.ReleasedAt != nil {
				return nil, trustEvidenceInvalid()
			}
			if _, err := s.readEvidenceArchive(a); err != nil {
				return nil, err
			}
			if !archiveProtection(a, now)["minimum_retention_satisfied"].(bool) {
				return nil, &CommandError{Code: "authorization_required", Message: "archive protégée : durée minimale de conservation non échue"}
			}
			a.ReleasedAt = &now
			a.ReleasedBy = id.Subject
		} else {
			if c.Anchor == nil || c.ExpectedSequence == nil || c.Anchor.Sequence < 1 || c.Anchor.Sequence > len(st.Evidence) || *c.ExpectedSequence != c.Anchor.Sequence || c.ExpectedHash != c.Anchor.Hash {
				return nil, trustEvidenceInvalid()
			}
			key, ok := r.Keys[c.Anchor.KeyID]
			if !ok {
				return nil, trustEvidenceInvalid()
			}
			exp := evidencePrefix(evidenceExport(st), c.Anchor.Sequence)
			if _, err := verifyPublicEvidence(TrustEvidenceCommand{Export: &exp, Anchor: c.Anchor, TrustedKey: &key}); err != nil {
				return nil, err
			}
			b := trustArchiveBundle{Schema: 1, Export: exp, Anchor: *c.Anchor, PublicKey: key}
			payload, err := json.Marshal(b)
			if err != nil || len(payload) > 2*trustStateMaxBytes {
				return nil, trustEvidenceInvalid()
			}
			a = TrustEvidenceArchive{ID: hash(payload), Sequence: c.Anchor.Sequence, Hash: c.Anchor.Hash, KeyID: key.ID, CreatedAt: now, CreatedBy: id.Subject, RetentionDays: st.Policy.EvidenceRetentionDays, PolicyRevision: st.Revision}
			if op == "archive-preview" {
				return map[string]any{"archive_id": a.ID, "sequence": a.Sequence, "retention_days": a.RetentionDays, "bytes": len(payload), "applied": false}, nil
			}
			if _, exists := r.Archives[a.ID]; exists {
				return nil, &CommandError{Code: "operation_pending", Message: "archive déjà conservée ; relisez son reçu"}
			}
			dir, err := s.evidenceArchiveDir(true)
			if err != nil {
				return nil, err
			}
			defer dir.Close()
			path := archiveFile(dir, a.ID)
			existing, err := trustReadPrivate(path, 2*trustStateMaxBytes)
			if os.IsNotExist(err) {
				err = trustPublishPrivate(path, payload)
			} else if err == nil && !bytes.Equal(existing, payload) {
				err = trustEvidenceInvalid()
			}
			if err != nil {
				return nil, err
			}
		}
		if ctx.Err() != nil {
			return nil, ctx.Err()
		}
		if r.Archives == nil {
			r.Archives = map[string]TrustEvidenceArchive{}
		}
		r.Archives[a.ID] = a
		value := map[string]any{"archive_id": a.ID, "sequence": a.Sequence, "retention_days": a.RetentionDays, "created_at": a.CreatedAt, "released_at": a.ReleasedAt, "originals_deleted": false}
		original := hash(raw)
		st.Originals[original] = append([]byte{}, raw...)
		appendTrustEvidence(st, "evidence-"+op, id, a.ID, hashMust(value), original, "applied", c.Reason, now)
		reply, _ := json.Marshal(value)
		r.Replies[c.EventID] = trustEvidenceReply{Digest: digest, Authority: trustAuthorityDigest(id), Value: reply}
		return value, nil
	})
}
