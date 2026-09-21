package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

// An engine observation, never a worker report, delivery or acceptance proof.
// In particular it does not pretend to snapshot the mutable workspace.
type InterruptionRecord struct {
	Version   int           `json:"version"`
	Work      string        `json:"work"`
	Task      string        `json:"task"`
	Agent     string        `json:"agent"`
	Attempt   string        `json:"attempt"`
	Workspace string        `json:"workspace"`
	Status    string        `json:"status"`
	Reason    string        `json:"reason"`
	Ended     string        `json:"ended"`
	Progress  AgentProgress `json:"observed_progress"`
	Limits    RunLimits     `json:"limits"`
	Notice    string        `json:"notice"`
}

func (s *Store) interruptionRecord(a Agent) (*ExchangeArtifact, error) {
	if a.Status != "failed" && a.Status != "interrupted" {
		return nil, nil
	}
	if !safeName(a.ID) || a.Attempt == "" || a.Ended == "" {
		return nil, fmt.Errorf("identité ou fin de tentative absente")
	}
	dir := filepath.Join(s.root, ".swarm", "interruptions")
	if err := os.MkdirAll(dir, 0700); err != nil {
		return nil, err
	}
	resolved, err := filepath.EvalSymlinks(dir)
	if err != nil || resolved != dir {
		return nil, fmt.Errorf("répertoire des bilans d’arrêt redirigé")
	}
	record := InterruptionRecord{Version: 1, Work: a.WorkID, Task: a.TaskID, Agent: a.ID, Attempt: a.Attempt, Workspace: a.CWD, Status: a.Status, Reason: a.Activity, Ended: a.Ended, Progress: a.Progress, Limits: a.Limits, Notice: "Constat automatique du moteur. Aucun critère validé. Consulter les fichiers conservés dans la copie et les journaux ; leur contenu n’est pas figé par ce bilan. Un rapport du producteur et de nouveaux contrôles restent nécessaires."}
	raw, err := json.MarshalIndent(record, "", "  ")
	if err != nil {
		return nil, err
	}
	path := filepath.Join(dir, a.ID+".json")
	if st, e := os.Lstat(path); e == nil {
		if !st.Mode().IsRegular() {
			return nil, fmt.Errorf("bilan d’arrêt redirigé")
		}
		old, e := os.ReadFile(path)
		if e != nil {
			return nil, e
		}
		if hash(old) != hash(raw) {
			return nil, fmt.Errorf("bilan d’arrêt déjà présent avec un contenu différent")
		}
	} else if !os.IsNotExist(e) {
		return nil, e
	} else if e = atomicWrite(path, raw); e != nil {
		return nil, e
	}
	rel, _ := filepath.Rel(s.root, path)
	return &ExchangeArtifact{Path: filepath.ToSlash(rel), SHA256: hash(raw)}, nil
}
