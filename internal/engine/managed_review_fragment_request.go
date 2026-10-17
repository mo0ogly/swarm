//go:build linux

package engine

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
func parseManagedFragmentFinalRequest(reply, prefix string, c managedReviewContext, p managedReviewFragmentPlan, replies []string) (request managedFragmentFinalRequest, err error) {
	empty := managedFragmentFinalRequest{}
	if len(reply) > managedFragmentReplyLimit {
		return empty, reviewResponseProtocolError(fmt.Errorf("demande de preuves trop grande"))
	}
	if _, err := managedFragmentFinalBundle(c, p, replies); err != nil {
		return empty, err
	}
	defer func() { err = reviewResponseProtocolError(err) }()
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

func managedFragmentSelectionSchema(p managedReviewFragmentPlan, version int) string {
	if version < 5 {
		return managedFragmentRequestSchema
	}
	var schema map[string]any
	if err := json.Unmarshal([]byte(managedFragmentRequestSchema), &schema); err != nil {
		panic(err)
	}
	properties := schema["properties"].(map[string]any)
	raw, _ := json.Marshal(p)
	for name, value := range map[string]string{"candidate_commit": p.Candidate, "context_sha256": p.ContextDigest, "plan_sha256": hash(raw)} {
		properties[name] = map[string]any{"type": "string", "enum": []string{value}}
	}
	choices := []any{}
	for i, packet := range p.Packets {
		for j, artifact := range packet.Artifacts {
			choices = append(choices, map[string]any{
				"type": "object", "additionalProperties": false,
				"required": []string{"packet", "artifact", "sha256"},
				"properties": map[string]any{
					"packet":   map[string]any{"type": "integer", "enum": []int{i}},
					"artifact": map[string]any{"type": "integer", "enum": []int{j}},
					"sha256":   map[string]any{"type": "string", "enum": []string{artifact.Digest}},
				},
			})
		}
	}
	properties["references"].(map[string]any)["items"] = map[string]any{"anyOf": choices}
	raw, err := json.Marshal(schema)
	if err != nil {
		panic(err)
	}
	return string(raw)
}
