package engine

import (
	"fmt"
	"io"
)

// storageRetryCLI exposes the same validated policy used by Store mutations.
func (s *Store) storageRetryCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("swarm storage-retry show|apply [--input configuration.json]")
	}
	switch pos[1] {
	case "show":
		status, err := s.storageRetryStatus()
		if err != nil {
			return err
		}
		return printJSON(out, status)
	case "apply":
		raw, err := readInput(input)
		if err != nil {
			return err
		}
		var policy StorageRetryPolicy
		if err = strict(raw, &policy); err != nil {
			return err
		}
		status, err := s.saveStorageRetryPolicy(policy)
		if err != nil {
			return err
		}
		return printJSON(out, status)
	default:
		return fmt.Errorf("action storage-retry inconnue : %s", pos[1])
	}
}
