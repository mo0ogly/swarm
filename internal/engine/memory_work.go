//go:build linux

package engine

import (
	"fmt"
)

// Keep memory's error context while sharing the general Work row boundary.
func scanMemoryWork(row workRowScanner) (Work, error) {
	w, err := scanWorkRow(row)
	if err != nil {
		return Work{}, fmt.Errorf("Source mémoire : %w", err)
	}
	return w, nil
}

func memoryWork(reader memoryReader, id string) (Work, error) {
	w, err := readWorkRow(reader, id)
	if err != nil {
		return Work{}, fmt.Errorf("Source mémoire : %w", err)
	}
	return w, nil
}

// No partial catalogue survives a row/iteration/close failure.
func memoryWorks(reader memoryRowsReader) ([]Work, error) {
	works, err := listWorkRows(reader)
	if err != nil {
		return nil, fmt.Errorf("Sources du catalogue mémoire inaccessibles : %w", err)
	}
	return works, nil
}
