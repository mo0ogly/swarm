//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
)

// Check consistency against the original stored launch. Neither this row nor
// its hashes authenticate an actor able to rewrite both it and the agent body.
// Identity admission and atomic source/effect commitment remain separate work.
func (s *Store) checkedMemorySummaryReceipt(a Agent) (*MemorySummaryReceipt, error) {
	invalid := func() (*MemorySummaryReceipt, error) {
		return nil, fmt.Errorf("reçu extractif incompatible avec la demande confirmée : nouvel examen requis")
	}
	var work, task string
	var raw []byte
	if err := s.db.QueryRow("SELECT work_id,task_id,request FROM agents WHERE id=?", a.ID).Scan(&work, &task, &raw); err != nil {
		return nil, fmt.Errorf("demande du reçu extractif inaccessible : %w", err)
	}
	var request Launch
	if err := strict(raw, &request); err != nil {
		return nil, fmt.Errorf("demande du reçu extractif invalide : %w", err)
	}
	if request.Summary == nil {
		if (a.Context != nil && a.Context.Summary != nil) || (a.PreparedContext != nil && a.PreparedContext.Summary != nil) {
			return invalid()
		}
		return nil, nil // No new receipt is required for legacy non-summary runs.
	}
	if request.Schema != 1 || work != a.WorkID || task != a.TaskID || request.EventID != a.ID || request.TaskID != a.TaskID ||
		request.Revision < 1 || a.Context == nil || a.Context.Summary == nil {
		return invalid()
	}
	receipt := *a.Context.Summary
	prepared := a.PreparedContext
	if prepared == nil {
		prepared = a.Context
	}
	if prepared.Summary == nil || request.ContextHash == "" || prepared.SHA256 != request.ContextHash ||
		prepared.Bytes < 0 || prepared.Bytes > len(a.Prompt) || hash([]byte(a.Prompt[:prepared.Bytes])) != prepared.SHA256 ||
		a.Context.Bytes != len(a.Prompt) || a.Context.SHA256 != hash([]byte(a.Prompt)) {
		return invalid()
	}
	if a.PreparedContext != nil && (prepared.Summary.CheckedBeforeStart != "" || prepared.Summary.CheckedSourceSHA != "") {
		return invalid()
	}
	// The prepared receipt has no start-check fields. A current receipt may
	// have a previous dated observation, which is replaced only after recheck.
	original := *prepared.Summary
	original.CheckedBeforeStart, original.CheckedSourceSHA = "", ""
	receipt.CheckedBeforeStart, receipt.CheckedSourceSHA = "", ""
	if !reflect.DeepEqual(original, receipt) || !reflect.DeepEqual(receipt.Selection, *request.Summary) ||
		receipt.Work != work || receipt.Task != task || receipt.PreparedRevision != request.Revision ||
		receipt.QueuedRevision != request.Revision+1 {
		return invalid()
	}
	q := MemorySummarySourceRequest{Schema: 1, Work: work, Task: task, Revision: receipt.QueuedRevision, Dialogues: request.Summary.Dialogues}
	raw, err := json.Marshal(q)
	if err != nil {
		return nil, err
	}
	packet, err := s.memorySummarySource(raw)
	if err != nil {
		return nil, fmt.Errorf("sélection extractive périmée avant départ : %w", err)
	}
	if packet.SHA256 != receipt.QueuedSourceSHA {
		return nil, fmt.Errorf("sources du reçu extractif modifiées avant départ : nouvel examen requis")
	}
	var payload memorySummarySources
	if err := json.Unmarshal([]byte(packet.Payload), &payload); err != nil {
		return nil, err
	}
	body, expected, err := memorySummaryExtract(*request.Summary, payload.Sources)
	if err != nil {
		return nil, fmt.Errorf("reconstruction du reçu extractif impossible : %w", err)
	}
	expected.Work, expected.Task, expected.PreparedRevision = work, task, request.Revision
	expected.QueuedRevision, expected.QueuedSourceSHA = receipt.QueuedRevision, packet.SHA256
	if !reflect.DeepEqual(receipt, *expected) || !strings.HasSuffix(a.Prompt[:prepared.Bytes], body) {
		return invalid()
	}
	expected.CheckedBeforeStart, expected.CheckedSourceSHA = now(), packet.SHA256
	return expected, nil
}
