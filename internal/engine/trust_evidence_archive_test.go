//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func archiveFixture(t *testing.T) (*Store, map[string]any) {
	t.Helper()
	s := evidenceFixture(t)
	k := evidencePrepare(t, s, "archive-key", 0)
	evidenceActivate(t, s, k, "archive-key-activate", 1)
	_, anchor, _ := evidenceAnchor(t, s)
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	return s, map[string]any{"schema_version": 1, "event_id": "archive-one", "expected_revision": st.Revision, "expected_sequence": anchor.Sequence, "expected_hash": anchor.Hash, "anchor": anchor, "reason": "archive exact immutable prefix"}
}
func archiveCallAt(t *testing.T, s *Store, op string, body map[string]any, at time.Time) (any, error) {
	t.Helper()
	raw, _ := json.Marshal(body)
	c, err := parseTrustEvidence(op, raw)
	if err != nil {
		return nil, err
	}
	return s.trustArchiveOperation(context.Background(), op, c, raw, trustAdminToken, at)
}
func TestActionsConfianceT6ArchiveLifecycle(t *testing.T) {
	s, body := archiveFixture(t)
	before := secretStateBytes(t, s)
	preview, err := archiveCallAt(t, s, "archive-preview", body, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(before, secretStateBytes(t, s)) {
		t.Fatal("preview changed state")
	}
	dir := filepath.Join(s.root, ".swarm", "trust-archives")
	if _, err := os.Lstat(dir); !os.IsNotExist(err) {
		t.Fatal("preview created archive directory")
	}
	now := time.Now().UTC()
	v, err := archiveCallAt(t, s, "archive-create", body, now)
	if err != nil {
		t.Fatal(err)
	}
	id := v.(map[string]any)["archive_id"].(string)
	if preview.(map[string]any)["archive_id"] != id {
		t.Fatal("preview differs from applied content")
	}
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	a := st.PublicEvidence.Archives[id]
	saved := secretStateBytes(t, s)
	again, err := archiveCallAt(t, s, "archive-create", body, time.Time{})
	if err != nil || hashMust(again) != hashMust(v) || !bytes.Equal(saved, secretStateBytes(t, s)) {
		t.Fatal("replay changed archive or state", err)
	}
	release := map[string]any{"schema_version": 1, "event_id": "archive-release", "expected_revision": st.Revision, "archive_id": id, "reason": "release after minimum conservation"}
	deadline := now.AddDate(0, 0, a.RetentionDays)
	if _, err := archiveCallAt(t, s, "archive-release", release, deadline.Add(-time.Nanosecond)); err == nil {
		t.Fatal("released before minimum hold")
	}
	if !bytes.Equal(saved, secretStateBytes(t, s)) {
		t.Fatal("early refusal changed state")
	}
	reopened := &Store{root: s.root}
	if _, err := archiveCallAt(t, reopened, "archive-show", map[string]any{"archive_id": id}, time.Time{}); err != nil {
		t.Fatal("restart lost archive", err)
	}
	if _, err := archiveCallAt(t, reopened, "archive-release", release, deadline); err != nil {
		t.Fatal("boundary release denied", err)
	}
	if _, err := os.Stat(filepath.Join(dir, id+".json")); err != nil {
		t.Fatal("release deleted copy", err)
	}
	current, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	for key, raw := range st.Originals {
		if !bytes.Equal(raw, current.Originals[key]) {
			t.Fatal("release removed historical original")
		}
	}
	if current.PublicEvidence.Archives[id].ReleasedAt == nil {
		t.Fatal("release not persisted")
	}
}
func TestActionsConfianceT6ArchiveRejectsDamagedFiles(t *testing.T) {
	for _, variant := range []string{"missing", "corrupt", "link", "directory-link"} {
		t.Run(variant, func(t *testing.T) {
			s, body := archiveFixture(t)
			v, err := archiveCallAt(t, s, "archive-create", body, time.Time{})
			if err != nil {
				t.Fatal(err)
			}
			id := v.(map[string]any)["archive_id"].(string)
			dir := filepath.Join(s.root, ".swarm", "trust-archives")
			file := filepath.Join(dir, id+".json")
			raw, err := os.ReadFile(file)
			if err != nil {
				t.Fatal(err)
			}
			switch variant {
			case "missing":
				err = os.Remove(file)
			case "corrupt":
				err = os.WriteFile(file, []byte("{}"), 0600)
			case "link":
				outside := filepath.Join(t.TempDir(), "copy")
				if err = os.WriteFile(outside, raw, 0600); err == nil {
					err = os.Remove(file)
				}
				if err == nil {
					err = os.Symlink(outside, file)
				}
			case "directory-link":
				outside := filepath.Join(t.TempDir(), "moved")
				if err = os.Rename(dir, outside); err == nil {
					err = os.Symlink(outside, dir)
				}
			}
			if err != nil {
				t.Fatal(err)
			}
			before := secretStateBytes(t, s)
			if _, err := archiveCallAt(t, s, "archive-show", map[string]any{"archive_id": id}, time.Time{}); err == nil {
				t.Fatal("damaged archive accepted")
			}
			if _, err := archiveCallAt(t, s, "archive-create", body, time.Time{}); err == nil {
				t.Fatal("replay hid missing or altered file")
			}
			if !bytes.Equal(before, secretStateBytes(t, s)) {
				t.Fatal("damaged read changed state")
			}
		})
	}
}

func TestActionsConfianceT6ArchiveOrphanAndPolicy(t *testing.T) {
	s, body := archiveFixture(t)
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	anchor := body["anchor"].(TrustPublicAnchor)
	bundle := trustArchiveBundle{Schema: 1, Export: evidencePrefix(evidenceExport(&st), anchor.Sequence), Anchor: anchor, PublicKey: st.PublicEvidence.Keys[anchor.KeyID]}
	payload, _ := json.Marshal(bundle)
	dir, err := s.evidenceArchiveDir(true)
	if err != nil {
		t.Fatal(err)
	}
	err = trustPublishPrivate(archiveFile(dir, hash(payload)), payload)
	dir.Close()
	if err != nil {
		t.Fatal(err)
	}
	v, err := archiveCallAt(t, s, "archive-create", body, time.Time{})
	if err != nil {
		t.Fatal("exact orphan recovery denied", err)
	}
	id := v.(map[string]any)["archive_id"].(string)
	st, err = s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	a := st.PublicEvidence.Archives[id]
	change := lifecycleChange(t, "alice", false, st.Revision)
	change.EventID = "shorter-archive-policy"
	change.Policy.EvidenceRetentionDays = 1
	if _, err = s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	st, err = s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	if st.Policy.EvidenceRetentionDays != 1 || st.PublicEvidence.Archives[id].RetentionDays != a.RetentionDays {
		t.Fatal("policy shortened existing archive hold")
	}
	release := map[string]any{"schema_version": 1, "event_id": "shortened-release", "expected_revision": st.Revision, "archive_id": id, "reason": "shortened policy must not release old copy"}
	if _, err := archiveCallAt(t, s, "archive-release", release, a.CreatedAt.AddDate(0, 0, 1)); err == nil {
		t.Fatal("new policy bypassed stored hold")
	}
}
func TestActionsConfianceT6ArchiveConcurrentReplay(t *testing.T) {
	s, body := archiveFixture(t)
	raw, _ := json.Marshal(body)
	results := make(chan error, 8)
	for i := 0; i < 8; i++ {
		go func() {
			_, err := s.trustEvidenceOperation(context.Background(), "archive-create", raw, trustAdminToken, false)
			results <- err
		}()
	}
	for i := 0; i < 8; i++ {
		if err := <-results; err != nil {
			t.Fatal(err)
		}
	}
	st, err := s.loadTrustState()
	if err != nil {
		t.Fatal(err)
	}
	count := 0
	for _, ev := range st.Evidence {
		if ev.Kind == "evidence-archive-create" {
			count++
		}
	}
	if len(st.PublicEvidence.Archives) != 1 || count != 1 {
		t.Fatal("replay produced duplicate publication", count)
	}
}
func TestActionsConfianceT6ArchiveLargeRetention(t *testing.T) {
	a := TrustEvidenceArchive{CreatedAt: time.Date(2026, 1, 1, 0, 0, 0, 1, time.UTC), RetentionDays: int(trustPolicyBounds()["evidence_retention_days"]["max"])}
	if archiveProtection(a, time.Date(9999, 12, 31, 0, 0, 0, 1, time.UTC))["minimum_retention_satisfied"].(bool) {
		t.Fatal("large retention overflowed into release")
	}
}

func TestActionsConfianceT6ArchiveInvalidInputs(t *testing.T) {
	for _, variant := range []string{"wrong-hash", "wrong-sequence", "wrong-signature", "stale-revision"} {
		t.Run(variant, func(t *testing.T) {
			s, body := archiveFixture(t)
			switch variant {
			case "wrong-hash":
				body["expected_hash"] = "invalid"
			case "wrong-sequence":
				body["expected_sequence"] = 0
			case "wrong-signature":
				a := body["anchor"].(TrustPublicAnchor)
				a.Signature = "invalid"
				body["anchor"] = a
			case "stale-revision":
				body["expected_revision"] = 0
			}
			before := secretStateBytes(t, s)
			if _, err := archiveCallAt(t, s, "archive-create", body, time.Time{}); err == nil {
				t.Fatal("invalid archive accepted")
			}
			if !bytes.Equal(before, secretStateBytes(t, s)) {
				t.Fatal("invalid archive mutated state")
			}
			if _, err := os.Lstat(filepath.Join(s.root, ".swarm", "trust-archives")); !os.IsNotExist(err) {
				t.Fatal("invalid request created files")
			}
		})
	}
}
