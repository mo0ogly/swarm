//go:build linux

package main

import (
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"time"
)

type loopGuard struct {
	limits                  RunLimits
	lastOutput              time.Time
	calls, repeated, errors int
	lastSignature           [32]byte
	seen                    map[string]bool
	pending                 map[string]time.Time
	reason                  string
	degraded                string
	completed               int
	lastTool                string
	lastAction              string
	actionDetail            string
	lastResult              string
}

func newLoopGuard(l RunLimits) *loopGuard {
	return &loopGuard{limits: l, lastOutput: time.Now(), seen: map[string]bool{}, pending: map[string]time.Time{}}
}
func (g *loopGuard) call(id, name string, input any, now time.Time) {
	if id != "" && g.seen[id] {
		return
	}
	if id != "" {
		g.seen[id] = true
		g.pending[id] = now
	}
	b, _ := json.Marshal([]any{name, input})
	sig := sha256.Sum256(b)
	if g.calls > 0 && sig == g.lastSignature {
		g.repeated++
	} else {
		g.repeated = 1
	}
	g.lastTool = terminalText(name)
	g.lastAction, g.actionDetail = describeOperation(name, input)
	g.lastSignature = sig
	g.calls++
	if g.calls >= g.limits.MaxToolCalls {
		g.reason = "Limite d'appels d'outils atteinte"
	}
	if g.repeated >= g.limits.MaxRepeatedCalls {
		g.reason = "Limite de répétitions identiques atteinte"
	}
}
func (g *loopGuard) result(id string, failed bool) {
	if _, ok := g.pending[id]; !ok {
		return
	}
	delete(g.pending, id)
	g.completed++
	g.lastResult = now()
	if failed {
		g.errors++
	} else {
		g.errors = 0
	}
	if g.errors >= g.limits.MaxConsecutiveErrors {
		g.reason = "Limite d'erreurs d'outils consécutives atteinte"
	}
}
func (g *loopGuard) observe(d map[string]any, now time.Time) {
	if g.reason != "" {
		return
	}
	kind, _ := d["type"].(string)
	if kind == "assistant" || kind == "user" {
		m, _ := d["message"].(map[string]any)
		content, _ := m["content"].([]any)
		for _, v := range content {
			b, ok := v.(map[string]any)
			if !ok {
				continue
			}
			typ, _ := b["type"].(string)
			if typ == "tool_use" {
				id, _ := b["id"].(string)
				name, _ := b["name"].(string)
				g.call(id, name, b["input"], now)
			}
			if typ == "tool_result" {
				id, _ := b["tool_use_id"].(string)
				failed, _ := b["is_error"].(bool)
				g.result(id, failed)
			}
			if g.reason != "" {
				return
			}
		}
	}
	if kind == "item.started" || kind == "item.completed" {
		item, _ := d["item"].(map[string]any)
		typ, _ := item["type"].(string)
		id, _ := item["id"].(string)
		switch typ {
		case "command_execution", "mcp_tool_call":
			if kind == "item.started" {
				input := item["command"]
				if typ == "mcp_tool_call" {
					input = []any{item["server"], item["tool"], item["arguments"]}
				}
				g.call(id, typ, input, now)
			} else {
				failed := item["status"] == "failed"
				if code, ok := item["exit_code"].(float64); ok && code != 0 {
					failed = true
				}
				if item["error"] != nil {
					failed = true
				}
				g.result(id, failed)
			}
		}
	}
}
func (g *loopGuard) check(now time.Time) string {
	if g.reason != "" {
		return g.reason
	}
	if now.Sub(g.lastOutput) >= time.Duration(g.limits.SilenceSeconds)*time.Second {
		return "Délai sans sortie fournisseur atteint"
	}
	if g.degraded != "" {
		return ""
	}
	for _, started := range g.pending {
		if now.Sub(started) >= time.Duration(g.limits.ToolSeconds)*time.Second {
			return fmt.Sprintf("Délai d'outil observable atteint (%ds)", g.limits.ToolSeconds)
		}
	}
	return ""
}

func (g *loopGuard) loseVisibility(reason string) {
	if g.degraded == "" {
		g.degraded = reason
	}
	// Missing messages invalidate consecutive-error/repetition assumptions.
	g.errors = 0
	g.repeated = 0
}
func (g *loopGuard) summary() AgentProgress {
	return AgentProgress{Action: g.lastAction, Detail: g.actionDetail, ToolCalls: g.calls, ToolResults: g.completed, PendingTools: len(g.pending), LastTool: g.lastTool, LastResult: g.lastResult, Degraded: g.degraded}
}
