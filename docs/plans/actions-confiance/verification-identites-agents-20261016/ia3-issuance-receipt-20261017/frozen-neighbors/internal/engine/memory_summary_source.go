//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"unicode/utf8"
)

type MemorySummarySourceRequest struct {
	Schema    int           `json:"schema_version"`
	Work      string        `json:"work"`
	Task      string        `json:"task"`
	Revision  int           `json:"expected_revision"`
	Dialogues []DialogueRef `json:"dialogues"`
}

// Payload is a string so its exact Go-produced bytes can be verified by other
// runtimes without assuming identical JSON serialization or escaping rules.
type MemorySummarySourcePacket struct {
	Schema  int    `json:"schema_version"`
	Kind    string `json:"kind"`
	Payload string `json:"payload"`
	SHA256  string `json:"sha256"`
}

type memorySummarySource struct {
	ID       string       `json:"id"`
	Role     string       `json:"role"`
	Kind     string       `json:"kind"`
	Text     string       `json:"text"`
	SHA256   string       `json:"sha256"`
	Dialogue *DialogueRef `json:"dialogue,omitempty"`
}

func (s *Store) memorySummarySource(raw []byte) (MemorySummarySourcePacket, error) {
	var empty MemorySummarySourcePacket
	if len(raw) > projectMemoryTransportBytes {
		return empty, fmt.Errorf("summary source request exceeds memory JSON transport capacity")
	}
	var q MemorySummarySourceRequest
	if err := strict(raw, &q); err != nil {
		return empty, err
	}
	if q.Schema != 1 || q.Revision < 1 || !safeName(q.Work) || !safeName(q.Task) || len(q.Dialogues) == 0 {
		return empty, fmt.Errorf("summary source requires schema 1, work, task, expected revision and explicit dialogues")
	}
	// One row is the complete consistency boundary: task definition and selected
	// answers are resolved from the same stored Work, never from separate reads.
	var body []byte
	var revision int
	if err := s.db.QueryRow("SELECT revision,body FROM works WHERE id=?", q.Work).Scan(&revision, &body); err != nil {
		return empty, err
	}
	if !utf8.Valid(body) {
		return empty, fmt.Errorf("summary source work contains invalid UTF-8")
	}
	var w Work
	if err := json.Unmarshal(body, &w); err != nil {
		return empty, err
	}
	if w.ID != q.Work || w.Revision != revision {
		return empty, fmt.Errorf("summary source stored work identity or revision mismatch")
	}
	return memorySummarySourceForWork(s.root, w, q)
}

// Resolve the same source contract on a caller-owned Work snapshot, including
// the launch reservation transaction; no second database reader is introduced.
func memorySummarySourceForWork(root string, w Work, q MemorySummarySourceRequest) (MemorySummarySourcePacket, error) {
	var empty MemorySummarySourcePacket
	if w.ID != q.Work {
		return empty, fmt.Errorf("summary source work identity mismatch")
	}
	if w.Revision != q.Revision {
		return empty, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	t, err := w.task(q.Task)
	if err != nil {
		return empty, err
	}
	if t.Brainstorm {
		return empty, fmt.Errorf("summary source target must be a work task, not a brainstorm exchange")
	}
	// This operator snapshot is deliberately not a delegated worker context.
	// No sibling task/report, tool output or private guard is loaded.
	contract, err := json.Marshal(struct {
		Project   string   `json:"project_sha256"`
		Title     string   `json:"work_title"`
		Objective string   `json:"work_objective"`
		Scope     string   `json:"work_scope"`
		Criteria  []string `json:"work_criteria"`
		Task      *Task    `json:"task"`
	}{hash([]byte(root)), w.Title, w.Objective, w.Scope, w.Criteria, t})
	if err != nil {
		return empty, err
	}
	sources := []memorySummarySource{{ID: "current-task-snapshot", Role: "user", Kind: "task_snapshot", Text: string(contract), SHA256: hash(contract)}}
	seen := map[DialogueRef]bool{}
	for _, ref := range q.Dialogues {
		if seen[ref] {
			return empty, fmt.Errorf("summary source dialogue selected more than once")
		}
		seen[ref] = true
		hit, err := resolveDialogue(w, ref)
		if err != nil {
			return empty, err
		}
		if !nonempty(hit.Question) || !nonempty(hit.Text) {
			return empty, fmt.Errorf("summary source selected exchange has no stored question or answer")
		}
		refJSON, err := json.Marshal(ref)
		if err != nil {
			return empty, err
		}
		id := "dialogue-" + hash(refJSON)
		questionRef, answerRef := ref, ref
		sources = append(sources,
			memorySummarySource{ID: id + "-question", Role: "user", Kind: "operator_question", Text: hit.Question, SHA256: hash([]byte(hit.Question)), Dialogue: &questionRef},
			memorySummarySource{ID: id + "-answer", Role: "assistant", Kind: "historical_answer", Text: hit.Text, SHA256: hash([]byte(hit.Text)), Dialogue: &answerRef})
	}
	payload, err := json.Marshal(struct {
		Scope struct {
			Work string `json:"work"`
			Task string `json:"task"`
		} `json:"scope"`
		Revision int                   `json:"work_revision"`
		Project  string                `json:"project_sha256"`
		Audience string                `json:"audience"`
		Complete bool                  `json:"provider_history_complete"`
		Sources  []memorySummarySource `json:"sources"`
	}{Scope: struct {
		Work string `json:"work"`
		Task string `json:"task"`
	}{w.ID, t.ID}, Revision: w.Revision, Project: hash([]byte(root)), Audience: "local_operator", Sources: sources})
	if err != nil {
		return empty, err
	}
	return MemorySummarySourcePacket{Schema: 1, Kind: "swarm_summary_source_v1", Payload: string(payload), SHA256: hash(payload)}, nil
}
