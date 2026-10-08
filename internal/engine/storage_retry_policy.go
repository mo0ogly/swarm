package engine

import (
	"fmt"
	"os"
	"path/filepath"
	resources "swarm.local/companion"
)

var storageRetryDefaults = resources.StorageRetry

type StorageRetryPolicy struct {
	Schema  int `json:"schema_version"`
	Retries int `json:"busy_retries"`
	DelayMS int `json:"busy_retry_delay_ms"`
}

// CLI and server share this operator configuration. It only retries database
// transactions, never provider processes or objective validation failures.
func (s *Store) storageRetryPolicy() (StorageRetryPolicy, error) {
	var p StorageRetryPolicy
	raw := storageRetryDefaults
	path := filepath.Join(s.root, ".swarm", "storage-retry.json")
	st, err := os.Lstat(path)
	if err == nil {
		if !st.Mode().IsRegular() || st.Size() > 4096 {
			return p, fmt.Errorf("storage-retry.json : fichier local borné requis")
		}
		raw, err = os.ReadFile(path)
	} else if os.IsNotExist(err) {
		err = nil
	}
	if err != nil {
		return p, err
	}
	if err = strict(raw, &p); err != nil {
		return p, err
	}
	if p.Schema != 1 || p.Retries < 0 || p.Retries > 10 || p.DelayMS < 0 || p.DelayMS > 1000 {
		return p, fmt.Errorf("storage-retry.json : schema_version=1, busy_retries=0..10, busy_retry_delay_ms=0..1000 requis")
	}
	return p, nil
}
