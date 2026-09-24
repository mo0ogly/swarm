//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"strings"
)

type managedFragmentFinalRequest struct {
	Candidate     string                       `json:"candidate_commit"`
	ContextDigest string                       `json:"context_sha256"`
	PlanDigest    string                       `json:"plan_sha256"`
	State         string                       `json:"state"`
	Reason        string                       `json:"reason"`
	References    []managedFragmentEvidenceRef `json:"references"`
}

const managedFragmentRequestSchema = `{"type":"object","additionalProperties":false,"properties":{"candidate_commit":{"type":"string"},"context_sha256":{"type":"string"},"plan_sha256":{"type":"string"},"state":{"type":"string","enum":["ready","unknown"]},"reason":{"type":"string","minLength":8,"maxLength":1000},"references":{"type":"array","maxItems":64,"items":{"type":"object","additionalProperties":false,"properties":{"packet":{"type":"integer","minimum":0},"artifact":{"type":"integer","minimum":0},"sha256":{"type":"string"}},"required":["packet","artifact","sha256"]}}},"required":["candidate_commit","context_sha256","plan_sha256","state","reason","references"]}`

// Selection is an independent evidence request, never a task verdict. Return
// ready only when the exact requested originals can enter the final message.
func parseManagedFragmentFinalRequest(reply, prefix string, c managedReviewContext, p managedReviewFragmentPlan, replies []string) (managedFragmentFinalRequest, error) {
	empty := managedFragmentFinalRequest{}
	if len(reply) > managedFragmentReplyLimit {
		return empty, fmt.Errorf("demande de preuves trop grande")
	}
	if _, err := managedFragmentFinalBundle(c, p, replies); err != nil {
		return empty, err
	}
	var r managedFragmentFinalRequest
	if err := strict([]byte(reply), &r); err != nil {
		return empty, err
	}
	raw, _ := json.Marshal(p)
	if r.Candidate != c.Candidate || r.ContextDigest != p.ContextDigest || r.PlanDigest != hash(raw) || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 1000 || len(r.References) > 64 || (r.State != "ready" && r.State != "unknown") {
		return empty, fmt.Errorf("demande finale périmée ou invalide")
	}
	if r.State == "unknown" {
		// Keep uncertainty durable without falsely treating requested missing proof
		// as an executable selection. Unknown never authorizes the next call.
		return r, nil
	}
	if _, _, err := managedFragmentDecisionPrompt(prefix, c, p, replies, r.References); err != nil {
		return empty, err
	}
	return r, nil
}
