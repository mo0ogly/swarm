package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"unicode/utf8"
)

type workRowScanner interface{ Scan(...any) error }
type workRowReader interface{ QueryRow(string, ...any) *sql.Row }
type workRowsReader interface {
	Query(string, ...any) (*sql.Rows, error)
}

// Validate a persisted row, not a new in-memory Work or an actor's authority.
// Legacy optional fields remain readable; only the storage envelope is checked.
func scanWorkRow(row workRowScanner) (Work, error) {
	var id string
	var revision int
	var raw []byte
	if err := row.Scan(&id, &revision, &raw); err != nil {
		return Work{}, fmt.Errorf("Lecture du travail impossible : %w", err)
	}
	if !safeName(id) {
		return Work{}, fmt.Errorf("Identifiant de ligne de travail invalide")
	}
	if !utf8.Valid(raw) {
		return Work{}, fmt.Errorf("Travail %q : texte UTF-8 invalide", id)
	}
	var w Work
	if err := json.Unmarshal(raw, &w); err != nil {
		return Work{}, fmt.Errorf("Travail %q : JSON invalide : %w", id, err)
	}
	if revision < 1 || w.Schema != 1 || w.ID != id || w.Revision != revision {
		return Work{}, fmt.Errorf("Travail %q incohérent : schéma, identité ou révision incompatible", id)
	}
	return w, nil
}

func readWorkRow(reader workRowReader, id string) (Work, error) {
	if !safeName(id) {
		return Work{}, fmt.Errorf("Identifiant du travail invalide")
	}
	w, err := scanWorkRow(reader.QueryRow("SELECT id,revision,body FROM works WHERE id=?", id))
	if err != nil {
		return Work{}, fmt.Errorf("Travail %q : %w", id, err)
	}
	return w, nil
}

func listWorkRows(reader workRowsReader) ([]Work, error) {
	rows, err := reader.Query("SELECT id,revision,body FROM works ORDER BY rowid DESC")
	if err != nil {
		return nil, fmt.Errorf("Lecture des travaux impossible : %w", err)
	}
	defer rows.Close()
	out := []Work{}
	for rows.Next() {
		w, err := scanWorkRow(rows)
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
