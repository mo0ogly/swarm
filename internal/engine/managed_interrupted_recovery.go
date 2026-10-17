//go:build linux

package engine

import (
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

func (s *Store) retainInterruptedSources(w Work, a Agent) (*ExchangeArtifact, error) {
	if w.Planning == nil || w.Planning.Repository == nil {
		return nil, nil
	}
	item, err := s.managedAttempt(a.ID)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	if item.Result != "" {
		return nil, nil
	}
	unlock, err := managedLock(s.root, w.ID)
	if err != nil {
		return nil, err
	}
	defer unlock()
	if _, err = s.managedInterruptedCheckpoint(w, a, item); err != nil {
		return nil, err
	}
	path := filepath.Join(w.Planning.Repository.Storage, "interrupted-"+a.ID+".json")
	raw, err := readManagedRecoveryFile(path)
	if err != nil {
		return nil, err
	}
	rel, err := filepath.Rel(s.root, path)
	if err != nil {
		return nil, err
	}
	return &ExchangeArtifact{Path: filepath.ToSlash(rel), SHA256: hash(raw)}, nil
}

type ManagedInterruptedCheckpoint struct {
	Version int    `json:"version"`
	Work    string `json:"work"`
	Task    string `json:"task"`
	Agent   string `json:"agent"`
	Attempt string `json:"attempt"`
	Base    string `json:"base_commit"`
	Commit  string `json:"commit"`
	Tree    string `json:"tree"`
}

func (s *Store) managedInterruptedCheckpoint(w Work, a Agent, item ManagedAttempt) (*ManagedInterruptedCheckpoint, error) {
	if !safeName(a.ID) || !safeName(a.Attempt) || w.Planning == nil || w.Planning.Repository == nil {
		return nil, fmt.Errorf("reprise : identité ou dépôt du checkpoint absent")
	}
	if !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return nil, fmt.Errorf("reprise : fin du producteur non confirmée")
	}
	repo := w.Planning.Repository
	if item.Work != w.ID || item.Task != a.TaskID || item.Agent != a.ID || item.Result != "" || a.CWD != filepath.Join(item.Path, repo.Subdir) {
		return nil, fmt.Errorf("reprise : copie interrompue non attribuable")
	}
	if err := verifyManagedCopy(repo, item.Path); err != nil {
		return nil, err
	}
	path := filepath.Join(repo.Storage, "interrupted-"+a.ID+".json")
	bare := filepath.Join(repo.Storage, "repository.git")
	ref := "refs/swarm/interruptions/" + a.ID
	if st, err := os.Lstat(path); err == nil {
		if !st.Mode().IsRegular() {
			return nil, fmt.Errorf("reprise : checkpoint redirigé")
		}
		raw, err := readManagedRecoveryFile(path)
		if err != nil {
			return nil, err
		}
		var c ManagedInterruptedCheckpoint
		if err = strict(raw, &c); err != nil {
			return nil, err
		}
		decoded, oidErr := hex.DecodeString(c.Commit)
		if c.Version != 1 || c.Work != w.ID || c.Task != a.TaskID || c.Agent != a.ID || c.Attempt != a.Attempt || c.Base != item.Base || oidErr != nil || (len(decoded) != 20 && len(decoded) != 32) {
			return nil, fmt.Errorf("reprise : attribution du checkpoint modifiée")
		}
		bound, err := managedGit(bare, "rev-parse", "--verify", ref+"^{commit}")
		if err != nil || bound != c.Commit {
			return nil, fmt.Errorf("reprise : référence du checkpoint modifiée")
		}
		tree, err := managedGit(bare, "rev-parse", "--verify", c.Commit+"^{tree}")
		if err != nil || tree != c.Tree {
			return nil, fmt.Errorf("reprise : arbre du checkpoint modifié")
		}
		if _, err = managedGit(bare, "merge-base", "--is-ancestor", c.Base, c.Commit); err != nil {
			return nil, fmt.Errorf("reprise : base du checkpoint incohérente")
		}
		return &c, nil
	} else if !os.IsNotExist(err) {
		return nil, err
	}
	priorRef, err := managedGit(bare, "for-each-ref", "--format=%(objectname)", ref)
	if err != nil {
		return nil, err
	}
	if priorRef != "" {
		return nil, fmt.Errorf("reprise : checkpoint incomplet, référence conservée ; réconcilier son attribution avant relance")
	}
	if _, err = managedGit(item.Path, "add", "-A", "--", "."); err != nil {
		return nil, err
	}
	tree, err := managedGit(item.Path, "write-tree")
	if err != nil {
		return nil, err
	}
	commit, err := managedGit(item.Path, "commit-tree", tree, "-p", item.Base, "-m", "Checkpoint interrompu de "+a.ID)
	if err != nil {
		return nil, err
	}
	if _, err = managedGit(bare, "fetch", "--no-tags", item.Path, commit); err != nil {
		return nil, err
	}
	if _, err = managedGit(bare, "update-ref", ref, commit); err != nil {
		return nil, err
	}
	c := &ManagedInterruptedCheckpoint{Version: 1, Work: w.ID, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, Base: item.Base, Commit: commit, Tree: tree}
	raw, err := json.MarshalIndent(c, "", "  ")
	if err != nil {
		return nil, err
	}
	if err = atomicWrite(path, raw); err != nil {
		return nil, err
	}
	return c, nil
}

func (h *ManagedRecoveryHandoff) reusableCommit() string {
	if h.Result != "" {
		return h.Result
	}
	if h.Checkpoint != nil {
		return h.Checkpoint.Commit
	}
	return ""
}
