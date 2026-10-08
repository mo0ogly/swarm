package engine

import (
	"strings"
	"unicode"
)

// ReportedModel is a provider assertion, never a resolved configuration or
// proof that a particular model executed. Missing declarations stay unknown.
type ReportedModel struct {
	Model  string `json:"model"`
	Source string `json:"source"`
	At     string `json:"at"`
}

func providerReportedModel(data map[string]any) *ReportedModel {
	var model, source string
	switch data["type"] {
	case "system":
		if data["subtype"] != "init" {
			return nil
		}
		model, _ = data["model"].(string)
		source = "provider system/init"
	case "assistant":
		message, _ := data["message"].(map[string]any)
		model, _ = message["model"].(string)
		source = "provider assistant/message"
	default:
		return nil
	}
	model = strings.TrimSpace(model)
	if model == "" || len(model) > 200 || strings.IndexFunc(model, unicode.IsControl) >= 0 {
		return nil
	}
	return &ReportedModel{Model: model, Source: source, At: now()}
}

func (w *outputSink) reportedModelSnapshot() *ReportedModel {
	w.mu.Lock()
	defer w.mu.Unlock()
	if w.reportedModel == nil {
		return nil
	}
	m := *w.reportedModel
	return &m
}
