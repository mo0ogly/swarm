//go:build linux

package engine

import (
	"crypto/sha256"
	"encoding/json"
	"fmt"
	"time"
)

type failedCall struct {
	signature [32]byte
	completed int
}

// attemptMetricsVersion marks AgentProgress rows carrying the categorised,
// cumulative bilan counters below. Attempts persisted before this version
// have no such counters: a report must show them as unknown, never as zero.
const attemptMetricsVersion = 1

type loopGuard struct {
	signatures              map[string][32]byte
	failures                []failedCall
	limits                  RunLimits
	lastOutput              time.Time
	outputSeen              bool
	calls, repeated, errors int
	lastSignature           [32]byte
	seen                    map[string]bool
	pending                 map[string]time.Time
	contexts                map[string]toolFailure
	toolFailures            []toolFailure
	reason                  string
	degraded                string
	completed               int
	lastTool                string
	lastAction              string
	actionDetail            string
	lastResult              string
	// Cumulative bilan counters (REQ-QW3): unlike errors/repeated above, these
	// never reset on success or on lost visibility — they report what was
	// actually observed, not a current streak.
	reads, writes, unclassified int
	totalErrors, totalRepeats   int
	sigSeen                     map[[32]byte]int
}

func newLoopGuard(l RunLimits) *loopGuard {
	return &loopGuard{signatures: map[string][32]byte{}, limits: l, lastOutput: time.Now(), seen: map[string]bool{}, pending: map[string]time.Time{}, contexts: map[string]toolFailure{}, sigSeen: map[[32]byte]int{}}
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
	if id != "" {
		g.signatures[id] = sig
	}
	if g.calls > 0 && sig == g.lastSignature {
		g.repeated++
	} else {
		g.repeated = 1
	}
	switch toolCategory(name) {
	case "read":
		g.reads++
	case "write":
		g.writes++
	default:
		g.unclassified++
	}
	// Total repeats: every identical (name,input) signature seen again after
	// id-dedup above, not only adjacent ones — distinct from g.repeated which
	// is the current consecutive streak used for the loop-limit decision.
	if n, ok := g.sigSeen[sig]; ok {
		g.totalRepeats++
		g.sigSeen[sig] = n + 1
	} else {
		g.sigSeen[sig] = 1
	}
	g.lastTool = terminalText(name)
	g.lastAction, g.actionDetail = describeOperation(name, input)
	if id != "" {
		g.contexts[id] = toolFailure{name: terminalText(name), action: g.lastAction, detail: g.actionDetail}
	}
	g.lastSignature = sig
	g.calls++
	if !g.limits.observing() && (g.calls > g.limits.MaxToolCalls || (g.calls == g.limits.MaxToolCalls && id == "")) {
		g.reason = "Limite d'appels d'outils atteinte"
	}
	if !g.limits.observing() && (g.repeated >= g.limits.MaxRepeatedCalls) {
		g.reason = "Limite de répétitions identiques atteinte"
	}
}
func (g *loopGuard) result(id string, failed bool, technical ...string) {
	if _, ok := g.pending[id]; !ok {
		return
	}
	delete(g.pending, id)
	context := g.contexts[id]
	delete(g.contexts, id)
	g.completed++
	// The last authorized tool may finish; never discard its result at start.
	if !g.limits.observing() && (g.calls >= g.limits.MaxToolCalls && len(g.pending) == 0) {
		g.reason = "Limite d'appels d'outils atteinte"
	}
	g.lastResult = now()
	sig, tracked := g.signatures[id]
	delete(g.signatures, id)
	recent := g.failures[:0]
	count := 0
	for _, f := range g.failures {
		if g.completed-f.completed >= 4*g.limits.MaxRepeatedCalls || (tracked && !failed && f.signature == sig) {
			continue
		}
		recent = append(recent, f)
		if tracked && f.signature == sig {
			count++
		}
	}
	g.failures = recent
	if tracked && failed {
		g.failures = append(g.failures, failedCall{sig, g.completed})
		if !g.limits.observing() && (count+1 >= g.limits.MaxRepeatedCalls) {
			g.reason = "Limite d'échecs identiques entrelacés atteinte"
		}
	}

	if failed {
		detail := ""
		if len(technical) > 0 {
			detail = technical[0]
		}
		context.technical = operationText(detail, 600)
		g.toolFailures = append(g.toolFailures, context)
		g.errors++
		g.totalErrors++
	} else {
		g.errors = 0
	}
	if !g.limits.observing() && (g.errors >= g.limits.MaxConsecutiveErrors) {
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
				technical, _ := json.Marshal(b["content"])
				g.result(id, failed, string(technical))
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
		case "file_change", "web_search":
			input := item["changes"]
			if typ == "web_search" {
				input = item["query"]
			}
			g.call(id, typ, input, now)
			if kind == "item.completed" {
				g.result(id, item["status"] == "failed", fmt.Sprint(item["error"]))
			}
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
				technical := fmt.Sprint(item["error"])
				if technical == "<nil>" {
					technical = fmt.Sprint(item["aggregated_output"])
				}
				g.result(id, failed, technical)
			}
		}
	}
}
func (g *loopGuard) check(now time.Time) string {
	if g.limits.observing() {
		return ""
	}
	if g.reason != "" {
		return g.reason
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

// monitoring separates observable output and tool state from provider vitality.
// Silence is evidence about the stream only: it never proves progress, failure,
// or completion and therefore cannot end an attempt before its total deadline.
func (g *loopGuard) monitoring(at time.Time) (output, tool, vitality string) {
	output, tool, vitality = "none", "idle", "unknown"
	if g.outputSeen {
		output = "recent"
		if at.Sub(g.lastOutput) >= time.Duration(g.limits.SilenceSeconds)*time.Second {
			output = "silent"
		}
	}
	if g.degraded != "" {
		tool = "unknown"
	} else if len(g.pending) > 0 {
		tool = "waiting"
	}
	return
}

func (g *loopGuard) loseVisibility(reason string) {
	if g.degraded == "" {
		g.degraded = reason
	}
	// Missing messages invalidate consecutive-error/repetition assumptions.
	g.errors = 0
	g.repeated = 0
	g.failures = nil
	g.signatures = map[string][32]byte{}
	g.contexts = map[string]toolFailure{}
}
func (g *loopGuard) summary() AgentProgress {
	output, tool, vitality := g.monitoring(time.Now())
	lastOutput := ""
	if g.outputSeen {
		lastOutput = g.lastOutput.UTC().Format(time.RFC3339Nano)
	}
	return AgentProgress{Action: g.lastAction, Detail: g.actionDetail, ToolCalls: g.calls, ToolResults: g.completed, PendingTools: len(g.pending), LastTool: g.lastTool, LastResult: g.lastResult, LastOutput: lastOutput, OutputState: output, ToolState: tool, Vitality: vitality, Degraded: g.degraded,
		MetricsVersion: attemptMetricsVersion, Reads: g.reads, Writes: g.writes, Unclassified: g.unclassified, Errors: g.totalErrors, Repeats: g.totalRepeats}
}
func (g *loopGuard) diagnostic(agent, attempt, stopReason string) AttemptDiagnostic {
	if stopReason == "" {
		stopReason = g.reason
	}
	return buildAttemptDiagnostic(agent, attempt, g.toolFailures, stopReason, g.limits.MaxConsecutiveErrors)
}
