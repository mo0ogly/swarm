//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"unicode/utf8"
)

type memoryWorkScanner interface{ Scan(...any) error }

// QueryRow and streaming Rows use the same identity/revision boundary. This
// validates consistency with the SQL row, never an owner, author or permission.
func scanMemoryWork(row memoryWorkScanner) (Work, error) {
	var id string
	var revision int
	var raw []byte
	if err := row.Scan(&id, &revision, &raw); err != nil {
		return Work{}, fmt.Errorf("Lecture de source mémoire impossible : %w", err)
	}
	if !safeName(id) {
		return Work{}, fmt.Errorf("Identifiant de ligne source mémoire invalide")
	}
	if !utf8.Valid(raw) {
		return Work{}, fmt.Errorf("Source mémoire %q : texte UTF-8 invalide", id)
	}
	var w Work
	if err := json.Unmarshal(raw, &w); err != nil {
		return Work{}, fmt.Errorf("Source mémoire %q : JSON invalide : %w", id, err)
	}
	if revision < 1 || w.Schema != 1 || w.ID != id || w.Revision != revision {
		return Work{}, fmt.Errorf("Source mémoire %q incohérente : schéma, identité ou révision incompatible", id)
	}
	return w, nil
}

func memoryWork(reader memoryReader, id string) (Work, error) {
	if !safeName(id) {
		return Work{}, fmt.Errorf("Travail source invalide")
	}
	return scanMemoryWork(reader.QueryRow("SELECT id,revision,body FROM works WHERE id=?", id))
}

// Used by the memory catalogue only. Global Store readers keep their existing
// contracts. No partial catalogue survives a row/iteration/close failure.
func memoryWorks(reader memoryRowsReader) ([]Work, error) {
	rows, err := reader.Query("SELECT id,revision,body FROM works ORDER BY rowid DESC")
	if err != nil {
		return nil, fmt.Errorf("Sources du catalogue mémoire inaccessibles : %w", err)
	}
	defer rows.Close()
	out := []Work{}
	for rows.Next() {
		w, err := scanMemoryWork(rows)
		if err != nil {
			return nil, err
		}
		out = append(out, w)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	if err := rows.Close(); err != nil {
		return nil, err
	}
	return out, nil
}
