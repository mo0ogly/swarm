//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"reflect"
	"strings"
)

type managedReviewFrozenWorkflow struct {
	Version  int           `json:"version"`
	Workflow AgentWorkflow `json:"workflow"`
	Method   string        `json:"method"`
	Prefix   string        `json:"prefix"`
}

func (s *Store) managedFragmentWorkflowPrefix(r IndependentReview) (string, error) {
	anchor := r.FragmentJournal
	if anchor == nil {
		return "", fmt.Errorf("ancrage des consignes absent")
	}
	if anchor.WorkflowPrompt == "" && anchor.WorkflowPromptDigest == "" {
		_, method, err := s.projectAgentWorkflow("reviewer")
		if err != nil {
			return "", err
		}
		return managedReviewPrefix(method), nil
	}
	path, err := safeReport(s.root, anchor.WorkflowPrompt)
	if err != nil {
		return "", err
	}
	raw, err := os.ReadFile(path)
	if err != nil || !citationHashValid(anchor.WorkflowPromptDigest) || hash(raw) != anchor.WorkflowPromptDigest {
		return "", fmt.Errorf("consignes originales modifiées ou absentes")
	}
	var frozen managedReviewFrozenWorkflow
	if err := strict(raw, &frozen); err != nil {
		return "", err
	}
	method, _ := json.Marshal([]any{frozen.Workflow, frozen.Method})
	if frozen.Version != 1 || r.Workflow == nil || !reflect.DeepEqual(*r.Workflow, frozen.Workflow) || hash(method) != anchor.WorkflowDigest || frozen.Method == "" || !strings.HasPrefix(frozen.Prefix, frozen.Method) {
		return "", fmt.Errorf("consignes originales non attribuées à la revue")
	}
	return frozen.Prefix, nil
}
