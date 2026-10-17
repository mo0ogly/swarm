package engine

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

func (s *Store) configurePreparedRepository(work string, r ManagedRepositoryRequest, finish *func(bool) error) (*ManagedRepository, error) {
	if finish == nil || *finish != nil {
		return nil, fmt.Errorf("transaction de préparation du dépôt requise")
	}
	unlock, err := managedLock(s.root, work)
	if err != nil {
		return nil, err
	}
	storage := filepath.Join(s.root, ".swarm", "managed", work)
	created := []string{}
	for _, name := range []string{"repository.git", "repository.json", "repository.json.pending"} {
		path := filepath.Join(storage, name)
		if _, err := os.Lstat(path); os.IsNotExist(err) {
			created = append(created, path)
		} else if err != nil {
			unlock()
			return nil, err
		}
	}
	*finish = func(committed bool) error {
		defer unlock()
		if committed {
			return nil
		}
		var failures []error
		for _, path := range created {
			if err := os.RemoveAll(path); err != nil {
				failures = append(failures, fmt.Errorf("annulation du dépôt préparé : %w", err))
			}
		}
		return errors.Join(failures...)
	}
	return s.configureManagedRepositoryLocked(work, r)
}
