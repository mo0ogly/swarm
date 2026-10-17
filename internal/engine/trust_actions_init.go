//go:build linux

package engine

import (
	"bytes"
	"crypto/rand"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"syscall"
)

type trustInitialPublication struct {
	Schema      int    `json:"schema_version"`
	State       []byte `json:"state"`
	StateDigest string `json:"state_sha256"`
	Key         []byte `json:"proof_key"`
}

func trustSyncDirectory(path string) error {
	f, err := os.Open(filepath.Dir(path))
	if err != nil {
		return err
	}
	defer f.Close()
	return f.Sync()
}

func trustPublishPrivate(path string, raw []byte) error {
	f, err := os.CreateTemp(filepath.Dir(path), ".trust-init-")
	if err != nil {
		return err
	}
	name := f.Name()
	defer os.Remove(name)
	n, err := f.Write(raw)
	if err == nil && n != len(raw) {
		err = io.ErrShortWrite
	}
	if err == nil {
		err = f.Sync()
	}
	closeErr := f.Close()
	if err != nil {
		return err
	}
	if closeErr != nil {
		return closeErr
	}
	if err = os.Link(name, path); err != nil {
		return err
	}
	if err = os.Remove(name); err != nil {
		return err
	}
	return trustSyncDirectory(path)
}

func trustReadPrivate(path string, limit int) ([]byte, error) {
	f, err := os.OpenFile(path, os.O_RDONLY|syscall.O_NOFOLLOW|syscall.O_NONBLOCK, 0)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return nil, err
	}
	if !info.Mode().IsRegular() || info.Mode().Perm()&0077 != 0 || info.Size() < 1 || info.Size() > int64(limit) {
		return nil, fmt.Errorf("publication initiale privée régulière et bornée requise")
	}
	raw, err := io.ReadAll(io.LimitReader(f, int64(limit)+1))
	if err != nil {
		return nil, err
	}
	if len(raw) > limit {
		return nil, fmt.Errorf("publication initiale trop grande")
	}
	return raw, nil
}

func validateTrustInitialPublication(p trustInitialPublication) error {
	if p.Schema != 1 || len(p.Key) != trustProofKeyBytes || len(p.State) == 0 || len(p.State) > trustStateMaxBytes || hash(p.State) != p.StateDigest {
		return fmt.Errorf("publication initiale non attribuée")
	}
	var st trustState
	if err := strict(p.State, &st); err != nil {
		return err
	}
	if st.Schema != 1 || st.Revision != 1 || len(st.History) != 1 || st.History[0].Revision != 1 || len(st.Identities) < 3 || st.Mandates == nil || st.Actions == nil || st.Receipts == nil || len(st.Mandates) != 0 || len(st.Actions) != 0 || len(st.Receipts) != 0 || len(st.Ledger) != 0 || len(st.Evidence) != 0 || len(st.EventDigests) != 1 || len(st.EventReplies) != 1 {
		return fmt.Errorf("état initial de publication incohérent")
	}
	if err := validateTrustPolicy(st.Policy); err != nil {
		return err
	}
	event := st.History[0].EventID
	digest, digestErr := hex.DecodeString(st.EventDigests[event])
	if event == "" || digestErr != nil || len(digest) != 32 || len(st.EventReplies[event]) == 0 {
		return fmt.Errorf("autorisation initiale de publication absente")
	}
	return nil
}

func (s *Store) trustInitialPath() string {
	state, _, _ := s.trustPaths()
	return state + ".initializing"
}

func (s *Store) resumeTrustInitialPublication(change *TrustConfigChange) error {
	pending := s.trustInitialPath()
	raw, err := trustReadPrivate(pending, trustStateMaxBytes*2)
	if os.IsNotExist(err) {
		return nil
	}
	if err != nil {
		return err
	}
	var p trustInitialPublication
	if err = strict(raw, &p); err != nil {
		return err
	}
	if err = validateTrustInitialPublication(p); err != nil {
		return err
	}
	if change != nil {
		var initial trustState
		if err = strict(p.State, &initial); err != nil {
			return err
		}
		if initial.EventDigests[change.EventID] != hashMust(*change) {
			return &CommandError{Code: "event_conflict", Message: "reprise réservée à la configuration initiale exacte"}
		}
	}
	state, _, keyPath := s.trustPaths()
	current, stateErr := trustReadPrivate(state, trustStateMaxBytes)
	if stateErr == nil && !bytes.Equal(current, p.State) {
		return fmt.Errorf("état de publication initiale remplacé ; conserver les originaux")
	}
	if stateErr != nil && !os.IsNotExist(stateErr) {
		return stateErr
	}
	key, keyErr := s.trustProofKey()
	if keyErr == nil && !bytes.Equal(key, p.Key) {
		return fmt.Errorf("clé de publication initiale remplacée ; conserver les originaux")
	}
	if keyErr != nil && !os.IsNotExist(keyErr) {
		return keyErr
	}
	if os.IsNotExist(keyErr) {
		if err = trustPublishPrivate(keyPath, []byte(hex.EncodeToString(p.Key)+"\n")); err != nil {
			return err
		}
	}
	if os.IsNotExist(stateErr) {
		if err = trustPublishPrivate(state, p.State); err != nil {
			return err
		}
	}
	if err = trustSyncDirectory(state); err != nil {
		return err
	}
	if err = os.Remove(pending); err != nil {
		return err
	}
	return trustSyncDirectory(pending)
}

func (s *Store) publishTrustInitialState(raw []byte) error {
	state, _, key := s.trustPaths()
	for _, path := range []string{state, key, s.trustInitialPath()} {
		if _, err := os.Lstat(path); err == nil {
			return fmt.Errorf("fichier préexistant sans publication initiale attribuée ; conserver et diagnostiquer")
		} else if !os.IsNotExist(err) {
			return err
		}
	}
	p := trustInitialPublication{Schema: 1, State: raw, StateDigest: hash(raw), Key: make([]byte, trustProofKeyBytes)}
	if _, err := rand.Read(p.Key); err != nil {
		return err
	}
	if err := validateTrustInitialPublication(p); err != nil {
		return err
	}
	pending, err := json.Marshal(p)
	if err != nil {
		return err
	}
	if err = trustPublishPrivate(s.trustInitialPath(), pending); err != nil {
		return err
	}
	return s.resumeTrustInitialPublication(nil)
}
