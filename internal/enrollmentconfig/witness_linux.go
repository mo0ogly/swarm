//go:build linux

package enrollmentconfig

import (
	"bytes"
	"encoding/json"
	"errors"
)

var (
	ErrWitnessMissing = errors.New("enrollment config: independent durable witness missing")
	ErrWitness        = errors.New("enrollment config: invalid canonical witness or intent")
	ErrPending        = errors.New("enrollment config: pending intent requires authorized recovery")
)

type witnessIntent struct {
	Entry  Entry `json:"entry"`
	Target Head  `json:"target"`
}

type witnessRecord struct {
	Schema    int            `json:"schema_version"`
	Pins      Pins           `json:"pins"`
	Committed Head           `json:"committed"`
	Pending   *witnessIntent `json:"pending"`
}

func encodeWitness(w witnessRecord) ([]byte, error) {
	if w.Schema != 1 {
		return nil, ErrVersion
	}
	if !w.Pins.valid() || !w.Committed.valid() {
		return nil, ErrWitness
	}
	if w.Pending == nil {
		if w.Committed.Revision == 0 {
			return nil, ErrWitness
		}
	} else {
		intent := *w.Pending
		w.Pending = &intent
		if err := intent.Entry.Change.validate(); err != nil {
			return nil, err
		}
		if w.Committed.Revision == ^uint64(0) || intent.Entry.Change.Expected != w.Committed ||
			intent.Entry.Actor != w.Pins.Principal || intent.Entry.At.IsZero() {
			return nil, ErrWitness
		}
		w.Pending.Entry.At = intent.Entry.At.UTC()
		target, err := entryHead(w.Pins, w.Pending.Entry)
		if err != nil {
			return nil, err
		}
		if target != intent.Target {
			return nil, ErrWitness
		}
	}
	return json.Marshal(w)
}

func decodeWitness(raw []byte) (witnessRecord, error) {
	var w witnessRecord
	if err := json.Unmarshal(raw, &w); err != nil {
		return witnessRecord{}, errors.Join(ErrWitness, err)
	}
	canonical, err := encodeWitness(w)
	if err != nil {
		return witnessRecord{}, err
	}
	if !bytes.Equal(raw, canonical) {
		return witnessRecord{}, ErrWitness
	}
	return w, nil
}

func (c *Coordinator) loadWitness() (*witnessRecord, error) {
	raw, err := c.witness.readDocument("witness.json")
	if errors.Is(err, ErrNotConfigured) {
		return nil, ErrWitnessMissing
	}
	if err != nil {
		return nil, err
	}
	w, err := decodeWitness(raw)
	if err != nil {
		return nil, err
	}
	if w.Pins != c.config.pins {
		return nil, ErrPins
	}
	return &w, nil
}

func (c *Coordinator) saveWitness(w witnessRecord, create bool) error {
	raw, err := encodeWitness(w)
	if err != nil {
		return err
	}
	return c.witness.persistDocument("witness.json", raw, create)
}
