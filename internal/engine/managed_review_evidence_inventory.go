//go:build linux

package engine

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
)

// This is a receipt of supplied context, not an interpretation of the review or
// a claim that the model understood it. Never project mutable worker files.
type reviewEvidenceSource struct {
	Path   string `json:"path"`
	Blob   string `json:"git_blob"`
	SHA256 string `json:"sha256"`
	Bytes  int    `json:"bytes"`
}

type reviewEvidenceInventory struct {
	ResultEvent string                 `json:"result_event"`
	Attempt     string                 `json:"attempt"`
	Scope       string                 `json:"scope,omitempty"`
	Decision    string                 `json:"original_decision,omitempty"`
	Review      string                 `json:"review_id"`
	Candidate   string                 `json:"candidate_commit"`
	Context     string                 `json:"context_sha256"`
	Sources     []reviewEvidenceSource `json:"supplied_sources"`
	Unavailable string                 `json:"unavailable,omitempty"`
}

func (s *Store) reviewEvidenceInventory(r *IndependentReview) reviewEvidenceInventory {
	result := reviewEvidenceInventory{ResultEvent: reviewPlanningEventID(r), Attempt: r.Attempt, Review: r.ID, Candidate: r.CandidateSHA, Context: r.ContextDigest, Sources: []reviewEvidenceSource{}}
	if r.Context == "" || r.ContextDigest == "" {
		result.Unavailable = "Inventaire historique non enregistré ; ne pas déduire que des sources ont été jointes."
		return result
	}
	name, err := localFile(s.root, r.Context)
	var context managedReviewContext
	if err == nil {
		context, err = readAttestedReviewContext(name, r.ContextDigest)
	}
	if err != nil {
		result.Unavailable = fmt.Sprintf("Contexte de revue non attesté : %v ; aucune source attestée.", err)
		return result
	}
	if context.Candidate != r.CandidateSHA {
		result.Unavailable = "Contexte de revue attribué à un autre candidat ; aucune source attestée."
		return result
	}
	for _, source := range context.Sources {
		if source.Bytes != len(source.Content) || source.Digest != hash([]byte(source.Content)) {
			result.Sources = []reviewEvidenceSource{}
			result.Unavailable = fmt.Sprintf("Source de contexte incohérente : %s ; aucune source attestée.", source.Path)
			return result
		}
		result.Sources = append(result.Sources, reviewEvidenceSource{source.Path, source.Blob, source.Digest, source.Bytes})
	}
	return result
}

// Review contexts are already admitted and content-addressed by the review
// pipeline. Recovery handoff admission limits do not apply to their inventory.
// Authenticate the whole regular file before decoding, then authenticate the
// exact decoded stream again. The second pass cannot grow beyond the verified
// file size and must consume EOF (including trailing bytes).
func readAttestedReviewContext(name, digest string) (managedReviewContext, error) {
	var context managedReviewContext
	f, err := os.Open(name)
	if err != nil {
		return context, err
	}
	defer f.Close()
	info, err := f.Stat()
	if err != nil {
		return context, err
	}
	if !info.Mode().IsRegular() || info.Size() <= 0 {
		return context, fmt.Errorf("original vide ou non régulier")
	}
	h := sha256.New()
	n, err := io.Copy(h, io.LimitReader(f, info.Size()+1))
	if err != nil {
		return context, err
	}
	if n != info.Size() || hex.EncodeToString(h.Sum(nil)) != digest {
		return context, fmt.Errorf("empreinte originale différente ou original modifié")
	}
	if _, err = f.Seek(0, io.SeekStart); err != nil {
		return context, err
	}
	h.Reset()
	decoder := json.NewDecoder(io.TeeReader(io.LimitReader(f, n+1), h))
	if err = decoder.Decode(&context); err != nil {
		return managedReviewContext{}, fmt.Errorf("JSON original invalide : %w", err)
	}
	var extra json.RawMessage
	if err = decoder.Decode(&extra); err != io.EOF {
		return managedReviewContext{}, fmt.Errorf("données supplémentaires après le contexte original")
	}
	if hex.EncodeToString(h.Sum(nil)) != digest {
		return managedReviewContext{}, fmt.Errorf("original modifié pendant sa lecture")
	}
	return context, nil
}

func (s *Store) planningReviewEvidence(w Work, events []PlanningEvent) map[string]reviewEvidenceInventory {
	result := map[string]reviewEvidenceInventory{}
	for _, event := range events {
		if event.Kind != "independent_review" {
			continue
		}
		task, err := w.task(event.Task)
		if err != nil || task.ScopeID != event.Scope || task.IndependentReview == nil {
			continue
		}
		r := task.IndependentReview
		if reviewPlanningEventID(r) != event.ID || r.Attempt != event.Attempt || !currentTaskAttempt(task, event.Attempt) {
			continue // A latest inventory cannot be attributed to an older refusal.
		}
		inventory := s.reviewEvidenceInventory(r)
		inventory.Scope = event.Scope
		inventory.Decision = event.Decision
		result[task.ID] = inventory
	}
	return result
}

// Current terminal review knowledge remains available after acknowledgement,
// independently of retry authority. Only exact originals in the owned scope
// qualify. The caller includes them only alongside fresh admitted inputs; they
// never become input_events, wake a scope, or enter retry authorization receipts.
func planningAcknowledgedReviewEvidence(w Work, scope string) []PlanningEvent {
	result := []PlanningEvent{}
	if w.Planning == nil {
		return result
	}
	for _, event := range w.Planning.Inbox {
		if event.Scope != scope || event.Decision == "" || event.Kind != "independent_review" {
			continue
		}
		task, err := w.task(event.Task)
		if err != nil || task.ScopeID != scope || task.Status != "blocked" || task.IndependentReview == nil {
			continue
		}
		r := task.IndependentReview
		if (r.State != "unknown" && r.State != "changes_requested" && r.State != "error") || !currentTaskAttempt(task, r.Attempt) || r.Attempt != event.Attempt || reviewPlanningEventID(r) != event.ID {
			continue
		}
		result = append(result, event)
	}
	return result
}
