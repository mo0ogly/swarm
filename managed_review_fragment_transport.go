//go:build linux

package main

// Compact transport changes only repeated JSON field names. Every identity,
// original excerpt and opinion remains present, in the same order, with a
// column legend. Durable journal/final evidence formats are unchanged.
type managedFragmentFinalTransport struct {
	Candidate     string   `json:"candidate_commit"`
	ContextDigest string   `json:"context_sha256"`
	PlanDigest    string   `json:"plan_sha256"`
	ReplyDigests  []string `json:"reply_sha256"`
	Columns       []string `json:"columns"`
	Rows          [][]any  `json:"rows"`
}

func compactManagedFragmentFinalEvidence(b managedFragmentFinalEvidence) managedFragmentFinalTransport {
	out := managedFragmentFinalTransport{Candidate: b.Candidate, ContextDigest: b.ContextDigest, PlanDigest: b.PlanDigest, ReplyDigests: append([]string(nil), b.ReplyDigests...), Columns: []string{"packet", "artifact", "kind", "name", "sha256", "original_excerpt", "inspection_opinion"}, Rows: make([][]any, 0, len(b.Evidence))}
	for _, e := range b.Evidence {
		out.Rows = append(out.Rows, []any{e.Packet, e.Artifact, e.Kind, e.Name, e.Digest, e.Excerpt, e.Opinion})
	}
	return out
}
