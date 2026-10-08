//go:build linux

package main

import (
	"encoding/json"
	"reflect"
	"testing"
)

func TestManagedFragmentFinalTransportLossless(t *testing.T) {
	c, p, replies := finalBundleFixture(t)
	b, e := managedFragmentFinalBundle(c, p, replies)
	if e != nil {
		t.Fatal(e)
	}
	tr := compactManagedFragmentFinalEvidence(b)
	if tr.Candidate != b.Candidate || tr.ContextDigest != b.ContextDigest || tr.PlanDigest != b.PlanDigest || !reflect.DeepEqual(tr.ReplyDigests, b.ReplyDigests) || len(tr.Rows) != len(b.Evidence) {
		t.Fatal("lost identity")
	}
	expected := []string{"packet", "artifact", "kind", "name", "sha256", "original_excerpt", "inspection_opinion"}
	if !reflect.DeepEqual(tr.Columns, expected) {
		t.Fatal("ambiguous column legend")
	}
	for i, row := range tr.Rows {
		v := b.Evidence[i]
		want := []any{v.Packet, v.Artifact, v.Kind, v.Name, v.Digest, v.Excerpt, v.Opinion}
		if !reflect.DeepEqual(row, want) {
			t.Fatal("lost or reordered evidence", i)
		}
	}
	original, _ := json.Marshal(b)
	compact, _ := json.Marshal(tr)
	if len(compact) >= len(original) {
		t.Fatal("transport did not reduce repeated keys")
	}
	tr.ReplyDigests[0] = "changed"
	tr.Rows[0][5] = "changed"
	if b.ReplyDigests[0] == "changed" || b.Evidence[0].Excerpt == "changed" {
		t.Fatal("transport aliases durable evidence")
	}
}
