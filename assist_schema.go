//go:build linux

package main

import (
	_ "embed"
	"encoding/json"
	"os"
	"path/filepath"
)

//go:embed contracts/assistant-answer.v1.schema.json
var assistAnswerSchema []byte

// The provider receives the same contract as the validator, narrowed to this
// observation. This guides generation; server-side validation remains mandatory.
func assistantStructuredProvider(p Provider, t AssistTurn, dir string) (Provider, error) {
	var schema map[string]any
	if e := json.Unmarshal(assistAnswerSchema, &schema); e != nil {
		return p, e
	}
	delete(schema, "$schema")
	delete(schema, "$id")
	delete(schema, "title")
	props := schema["properties"].(map[string]any)
	props["version"] = map[string]any{"type": "integer", "enum": []int{1}}
	props["template_id"] = map[string]any{"type": "string", "enum": []string{t.TemplateID}}
	props["context_hash"] = map[string]any{"type": "string", "enum": []string{t.Context.Hash}}
	facts := []string{}
	for _, f := range t.Context.Facts {
		facts = append(facts, f.ID)
	}
	actions := []string{}
	for _, a := range t.Context.Actions {
		if a.Available {
			actions = append(actions, a.ID)
		}
	}
	for _, name := range []string{"facts", "next_steps"} {
		item := props[name].(map[string]any)["items"].(map[string]any)["properties"].(map[string]any)
		item["source_ids"].(map[string]any)["items"] = map[string]any{"type": "string", "enum": facts}
		if name == "next_steps" {
			if len(actions) == 0 {
				props[name].(map[string]any)["maxItems"] = 0
			} else {
				item["action_id"] = map[string]any{"type": "string", "enum": actions}
			}
		}
	}
	b, e := json.Marshal(schema)
	if e != nil {
		return p, e
	}
	if filepath.Base(p.Command) == "codex" {
		path := filepath.Join(dir, "answer-schema.json")
		if e = os.WriteFile(path, b, 0600); e != nil {
			return p, e
		}
		p.Args = append(p.Args[:len(p.Args)-1], "--output-schema", path, "-")
	} else {
		p.Args = append(p.Args, "--json-schema", string(b))
	}
	return p, nil
}
