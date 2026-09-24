//go:build linux

package main

import (
	"encoding/json"
	"fmt"
)

type managedFragmentFinalCall struct {
	Phase        string `json:"phase"`
	CallID       string `json:"call_id"`
	PromptDigest string `json:"prompt_sha256"`
	State        string `json:"state"`
	Reply        string `json:"reply,omitempty"`
	ReplyDigest  string `json:"reply_sha256,omitempty"`
}
type managedFragmentFinalJournal struct {
	ResumeCalls      []string                   `json:"resume_calls,omitempty"`
	Version          int                        `json:"version"`
	InspectionDigest string                     `json:"inspection_journal_sha256"`
	Calls            []managedFragmentFinalCall `json:"calls"`
}

// Rebuild every final prompt and reparse every original reply. The result is
// review evidence only: publication still needs Store identities and controls.
func validateManagedFragmentFinalJournal(f managedFragmentFinalJournal, j managedFragmentJournal, c managedReviewContext, p managedReviewFragmentPlan, prefix string) (string, []ManagedTaskReview, error) {
	raw, _ := json.Marshal(j)
	if f.Version != 1 || f.InspectionDigest != hash(raw) || len(f.Calls) > p.ReservedFinalCalls+len(f.ResumeCalls) {
		return "", nil, fmt.Errorf("journal final périmé ou budget dépassé")
	}
	authorized := map[string]bool{}
	for _, id := range f.ResumeCalls {
		if id == "" || authorized[id] {
			return "", nil, fmt.Errorf("reprise finale dupliquée")
		}
		found := false
		for _, call := range f.Calls {
			if call.CallID == id && call.State == "interrupted" {
				found = true
			}
		}
		if !found {
			return "", nil, fmt.Errorf("reprise finale sans interruption")
		}
		authorized[id] = true
	}
	reusable, err := validateManagedFragmentJournal(j, p, j.Attempt, j.ProviderDigest)
	if err != nil {
		return "", nil, err
	}
	replies := make([]string, len(p.Packets))
	for i := range replies {
		reply, ok := reusable[i]
		if !ok {
			return "", nil, fmt.Errorf("inspection finale incomplète")
		}
		replies[i] = reply
	}
	if _, err = managedFragmentFinalBundle(c, p, replies); err != nil {
		return "", nil, err
	}
	seen := map[string]bool{}
	for _, entry := range j.Entries {
		seen[entry.CallID] = true
	}
	var refs []managedFragmentEvidenceRef
	phase := "selection"
	state := "pending"
	var records []ManagedTaskReview
	previousCall := ""
	for _, call := range f.Calls {
		if state == "interrupted" && !authorized[previousCall] {
			return "", nil, fmt.Errorf("reprise finale non autorisée")
		}
		previousCall = call.CallID
		if call.CallID == "" || seen[call.CallID] || call.Phase != phase || (state != "pending" && state != "interrupted" && state != "ready") {
			return "", nil, fmt.Errorf("appel final dupliqué ou hors séquence")
		}
		seen[call.CallID] = true
		var prompt string
		var visible managedReviewContext
		if phase == "selection" {
			prompt, err = managedFragmentRequestPrompt(prefix, c, p, replies)
		} else {
			prompt, visible, err = managedFragmentDecisionPrompt(prefix, c, p, replies, refs)
		}
		if err != nil {
			return "", nil, err
		}
		if call.PromptDigest != hash([]byte(prompt)) {
			return "", nil, fmt.Errorf("consigne finale modifiée")
		}
		switch call.State {
		case "reserved", "interrupted":
			if call.Reply != "" || call.ReplyDigest != "" {
				return "", nil, fmt.Errorf("réponse finale non attribuée")
			}
			state = call.State
		default:
			if call.ReplyDigest == "" || hash([]byte(call.Reply)) != call.ReplyDigest {
				return "", nil, fmt.Errorf("réponse finale modifiée")
			}
			if phase == "selection" {
				request, e := parseManagedFragmentFinalRequest(call.Reply, prefix, c, p, replies)
				if e != nil {
					return "", nil, e
				}
				if request.State != call.State {
					return "", nil, fmt.Errorf("état de sélection différent de la réponse")
				}
				state = request.State
				refs = request.References
				if state == "ready" {
					phase = "decision"
				}
			} else {
				if len(call.Reply) > managedFragmentReplyLimit {
					return "", nil, fmt.Errorf("avis final trop grand")
				}
				state, records, err = parseManagedReview(call.Reply, visible)
				if err != nil {
					return "", nil, err
				}
				if state != call.State {
					return "", nil, fmt.Errorf("verdict final différent de la réponse")
				}
			}
		}
	}
	return state, records, nil
}
