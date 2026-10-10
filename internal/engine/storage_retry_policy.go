package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	resources "swarm.local/companion"
)

var storageRetryDefaults = resources.StorageRetry

type StorageRetryPolicy struct {
	Schema        int `json:"schema_version"`
	Retries       int `json:"busy_retries"`
	DelayMS       int `json:"busy_retry_delay_ms"`
	BusyTimeoutMS int `json:"busy_timeout_ms"`
}

type storageRetryDocument struct {
	Schema        int  `json:"schema_version"`
	Retries       int  `json:"busy_retries"`
	DelayMS       int  `json:"busy_retry_delay_ms"`
	BusyTimeoutMS *int `json:"busy_timeout_ms,omitempty"`
}

type StorageRetryStatus struct {
	Configured          StorageRetryPolicy `json:"configured"`
	Effective           StorageRetryPolicy `json:"effective"`
	Source              string             `json:"source"`
	Persisted           bool               `json:"persisted"`
	StorageFailureCause string             `json:"storage_failure_cause"`
	MaximumTotalWaitMS  int                `json:"maximum_total_wait_ms"`
	Note                string             `json:"note"`
}

const legacyStorageBusyTimeoutMS = 5000

// CLI and server share this operator configuration. It only retries database
// transactions, never provider processes or objective validation failures.
func (s *Store) storageRetryPolicy() (StorageRetryPolicy, error) {
	p, _, _, err := s.readStorageRetryPolicy()
	return p, err
}

func (s *Store) readStorageRetryPolicy() (StorageRetryPolicy, string, bool, error) {
	var document storageRetryDocument
	raw := storageRetryDefaults
	path := filepath.Join(s.root, ".swarm", "storage-retry.json")
	source, persisted := "config/storage-retry.json", false
	st, err := os.Lstat(path)
	if err == nil {
		if !st.Mode().IsRegular() || st.Size() > 4096 {
			return StorageRetryPolicy{}, "", false, fmt.Errorf("storage-retry.json : fichier local borné requis")
		}
		raw, err = os.ReadFile(path)
		source, persisted = ".swarm/storage-retry.json", true
	} else if os.IsNotExist(err) {
		err = nil
	}
	if err != nil {
		return StorageRetryPolicy{}, "", false, err
	}
	if err = strict(raw, &document); err != nil {
		return StorageRetryPolicy{}, "", false, err
	}
	busyTimeout := legacyStorageBusyTimeoutMS
	if document.BusyTimeoutMS != nil {
		busyTimeout = *document.BusyTimeoutMS
	}
	p := StorageRetryPolicy{Schema: document.Schema, Retries: document.Retries, DelayMS: document.DelayMS, BusyTimeoutMS: busyTimeout}
	if err = validStorageRetryPolicy(p); err != nil {
		return StorageRetryPolicy{}, "", false, err
	}
	return p, source, persisted, nil
}

func validStorageRetryPolicy(p StorageRetryPolicy) error {
	if p.Schema != 1 || p.Retries < 0 || p.Retries > 10 || p.DelayMS < 0 || p.DelayMS > 1000 || p.BusyTimeoutMS < 1 || p.BusyTimeoutMS > 60000 {
		return fmt.Errorf("storage-retry.json : schema_version=1, busy_retries=0..10, busy_retry_delay_ms=0..1000, busy_timeout_ms=1..60000 requis")
	}
	return nil
}

func storageRetryMaximumWaitMS(p StorageRetryPolicy) int {
	return (p.Retries+1)*p.BusyTimeoutMS + p.Retries*p.DelayMS
}

func (s *Store) storageRetryStatus() (StorageRetryStatus, error) {
	p, source, persisted, err := s.readStorageRetryPolicy()
	if err != nil {
		return StorageRetryStatus{}, err
	}
	// Make the displayed effective value true for this long-lived process too,
	// including when another public CLI invocation changed the persisted file.
	if _, err = s.db.Exec(fmt.Sprintf("PRAGMA busy_timeout=%d", p.BusyTimeoutMS)); err != nil {
		return StorageRetryStatus{}, err
	}
	return StorageRetryStatus{
		Configured: p, Effective: p, Source: source, Persisted: persisted,
		StorageFailureCause: "sqlite_busy", MaximumTotalWaitMS: storageRetryMaximumWaitMS(p),
		Note: "Durée bornée : busy_timeout s'applique à chaque tentative SQLite, puis le délai précède chaque reprise. Aucun secret ni contenu de requête n'est exposé.",
	}, nil
}

func (s *Store) saveStorageRetryPolicy(p StorageRetryPolicy) (StorageRetryStatus, error) {
	if err := validStorageRetryPolicy(p); err != nil {
		return StorageRetryStatus{}, err
	}
	raw, err := json.Marshal(p)
	if err != nil {
		return StorageRetryStatus{}, err
	}
	path := filepath.Join(s.root, ".swarm", "storage-retry.json")
	if err = atomicWrite(path, append(raw, '\n')); err != nil {
		return StorageRetryStatus{}, err
	}
	if err = os.Chmod(path, 0600); err != nil {
		return StorageRetryStatus{}, err
	}
	if _, err = s.db.Exec(fmt.Sprintf("PRAGMA busy_timeout=%d", p.BusyTimeoutMS)); err != nil {
		return StorageRetryStatus{}, err
	}
	return s.storageRetryStatus()
}
