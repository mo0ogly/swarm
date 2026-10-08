//go:build linux

package engine

import "testing"

func TestReportedModelRejectsGuessesAndInvalidDeclarations(t *testing.T) {
	for _, data := range []map[string]any{
		{"type": "result", "model": "invented"},
		{"type": "system", "subtype": "thinking_tokens", "model": "invented"},
		{"type": "system", "subtype": "init", "model": "bad\nmodel"},
		{"type": "assistant", "message": map[string]any{"content": "model: invented"}},
	} {
		if got := providerReportedModel(data); got != nil {
			t.Fatalf("unexpected observation: %+v", got)
		}
	}
	sink := &outputSink{}
	if sink.reportedModelSnapshot() != nil {
		t.Fatal("unobserved model invented")
	}
	sink.line([]byte(`{"type":"assistant","message":{"model":"reported-v2"}}`))
	got := sink.reportedModelSnapshot()
	if got == nil || got.Model != "reported-v2" || got.At == "" || got.Source != "provider assistant/message" {
		t.Fatalf("declaration lost: %+v", got)
	}
	got.Model = "modified snapshot"
	if sink.reportedModelSnapshot().Model != "reported-v2" {
		t.Fatal("snapshot aliases live observation")
	}
}

func TestReportedModelRealProcessPreservesRequestedModel(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	request.Instruction = "TEST_REPORTED_MODEL"
	a, _, err := s.prepare(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	// Isolated test configuration: the actual child provider deliberately declares
	// a different model. This is a protocol fixture, never a real Claude claim.
	a.ModelRoute = &ModelRoute{Model: "requested-model"}
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	if err = s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	got, err := s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	if got.Status != "completed" || got.ModelRoute.Model != "requested-model" || got.ReportedModel == nil || got.ReportedModel.Model != "provider-reported-model" || got.ReportedModel.Source != "provider system/init" {
		t.Fatalf("requested and reported conflated: %+v", got)
	}
}
