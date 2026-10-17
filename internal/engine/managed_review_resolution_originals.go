//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

func managedFragmentResolutionOriginals(visible managedReviewContext) ([]managedReviewFragmentArtifact, error) {
	originals := []managedReviewFragmentArtifact{}
	for _, source := range visible.Sources {
		originals = append(originals, managedReviewFragmentArtifact{Name: source.Path, Kind: "source", Content: source.Content})
	}
	if len(visible.Receipt) != 0 {
		receipt, marshalErr := json.Marshal(visible.Receipt)
		if marshalErr != nil {
			return nil, fmt.Errorf("reçu moteur visible illisible : %w", marshalErr)
		}
		originals = append(originals, managedReviewFragmentArtifact{Name: "candidate-receipt.json", Kind: "context", Content: string(receipt)})
	}
	for _, t := range visible.Tasks {
		originals = append(originals, managedReviewOriginalEvidence(managedReviewContext{}, t)...)
		// The final prompt also supplies these current, authoritative task
		// contract fields. They can answer a question about assigned scope,
		// but are deliberately not added to visible.Sources: parseManagedReview
		// must still require implementation evidence for a business pass.
		// Do not admit delivery claims, binding opinions or historical reviews
		// merely because they occur elsewhere in the prompt metadata.
		contract, _ := json.Marshal(struct {
			Task        string   `json:"task"`
			Title       string   `json:"title"`
			Deliverable string   `json:"deliverable"`
			Criteria    []string `json:"criteria"`
		}{t.Task, t.Title, t.Deliverable, t.Criteria})
		originals = append(originals, managedReviewFragmentArtifact{Name: "task-contract.json", Kind: "context", Content: string(contract)})
	}
	return originals, nil
}
