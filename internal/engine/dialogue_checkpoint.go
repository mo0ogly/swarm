//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"fmt"
	"maps"
	"math"
	"sort"
	"time"
)

// This is a boundary snapshot, not permission to restart a provider. An active
// turn may have effects after its start snapshot that the engine has not seen.
type dialogueCheckpoint struct {
	Schema   int                `json:"schema_version"`
	Agent    string             `json:"agent"`
	Turn     string             `json:"turn"`
	Project  string             `json:"project_sha256"`
	Launch   string             `json:"launch_sha256"`
	Session  string             `json:"session_id"`
	Phase    string             `json:"phase"`
	Failure  string             `json:"failure,omitempty"`
	At       string             `json:"at"`
	Guard    dialogueGuardState `json:"guard"`
	Usage    *Usage             `json:"usage,omitempty"`
	Model    *ReportedModel     `json:"reported_model,omitempty"`
	Cooldown *ProviderCooldown  `json:"provider_cooldown,omitempty"`
	SHA256   string             `json:"sha256"`
}

type dialogueFailureState struct {
	Name, Action, Detail, Technical, Category string
}

type dialogueFailedCall struct {
	Signature [32]byte
	Completed int
}

type dialogueSignatureCount struct {
	Signature [32]byte
	Count     int
}

// Keep all decision state, not only the AgentProgress projection. Fixed-size
// signatures are values; encoding map[[32]byte]int directly is unsupported.
type dialogueGuardState struct {
	Limits                      RunLimits
	Signatures                  map[string][32]byte
	Failures                    []dialogueFailedCall
	LastOutput                  time.Time
	OutputSeen                  bool
	Calls, Repeated, Errors     int
	LastSignature               [32]byte
	Seen                        map[string]bool
	Pending                     map[string]time.Time
	Contexts                    map[string]dialogueFailureState
	ToolFailures                []dialogueFailureState
	EnvironmentBlocked          bool
	Reason, Degraded            string
	Completed                   int
	LastTool, LastAction        string
	ActionDetail, LastResult    string
	Reads, Writes, Unclassified int
	TotalErrors, TotalRepeats   int
	SignatureCounts             []dialogueSignatureCount
}

func checkpointFailure(f toolFailure) dialogueFailureState {
	return dialogueFailureState{f.name, f.action, f.detail, f.technical, f.category}
}

func (f dialogueFailureState) restore() toolFailure {
	return toolFailure{f.Name, f.Action, f.Detail, f.Technical, f.Category}
}

func checkpointGuard(g *loopGuard) dialogueGuardState {
	v := dialogueGuardState{Limits: g.limits, Signatures: g.signatures, LastOutput: g.lastOutput, OutputSeen: g.outputSeen,
		Calls: g.calls, Repeated: g.repeated, Errors: g.errors, LastSignature: g.lastSignature, Seen: g.seen, Pending: g.pending,
		Contexts: map[string]dialogueFailureState{}, EnvironmentBlocked: g.environmentBlocked, Reason: g.reason, Degraded: g.degraded,
		Completed: g.completed, LastTool: g.lastTool, LastAction: g.lastAction, ActionDetail: g.actionDetail, LastResult: g.lastResult,
		Reads: g.reads, Writes: g.writes, Unclassified: g.unclassified, TotalErrors: g.totalErrors, TotalRepeats: g.totalRepeats}
	for _, f := range g.failures {
		v.Failures = append(v.Failures, dialogueFailedCall{f.signature, f.completed})
	}
	for id, f := range g.contexts {
		v.Contexts[id] = checkpointFailure(f)
	}
	for _, f := range g.toolFailures {
		v.ToolFailures = append(v.ToolFailures, checkpointFailure(f))
	}
	for signature, count := range g.sigSeen {
		v.SignatureCounts = append(v.SignatureCounts, dialogueSignatureCount{signature, count})
	}
	sort.Slice(v.SignatureCounts, func(i, j int) bool {
		return bytes.Compare(v.SignatureCounts[i].Signature[:], v.SignatureCounts[j].Signature[:]) < 0
	})
	return v
}

func (v dialogueGuardState) restore() (*loopGuard, error) {
	for _, n := range []int{v.Calls, v.Repeated, v.Errors, v.Completed, v.Reads, v.Writes, v.Unclassified, v.TotalErrors, v.TotalRepeats} {
		if n < 0 {
			return nil, fmt.Errorf("Compteur de dialogue invalide")
		}
	}
	if v.Completed > v.Calls || v.Reads+v.Writes+v.Unclassified != v.Calls || len(v.Pending) > v.Calls-v.Completed || v.Signatures == nil || v.Seen == nil || v.Pending == nil || v.Contexts == nil {
		return nil, fmt.Errorf("Gardes de dialogue incohérentes")
	}
	g := newLoopGuard(v.Limits)
	g.signatures, g.lastOutput, g.outputSeen = maps.Clone(v.Signatures), v.LastOutput, v.OutputSeen
	g.calls, g.repeated, g.errors, g.lastSignature = v.Calls, v.Repeated, v.Errors, v.LastSignature
	g.seen, g.pending = maps.Clone(v.Seen), maps.Clone(v.Pending)
	g.environmentBlocked, g.reason, g.degraded = v.EnvironmentBlocked, v.Reason, v.Degraded
	g.completed, g.lastTool, g.lastAction = v.Completed, v.LastTool, v.LastAction
	g.actionDetail, g.lastResult = v.ActionDetail, v.LastResult
	g.reads, g.writes, g.unclassified = v.Reads, v.Writes, v.Unclassified
	g.totalErrors, g.totalRepeats = v.TotalErrors, v.TotalRepeats
	for _, f := range v.Failures {
		if f.Completed < 0 || f.Completed > v.Completed {
			return nil, fmt.Errorf("Historique d’échecs de dialogue invalide")
		}
		g.failures = append(g.failures, failedCall{f.Signature, f.Completed})
	}
	for id, started := range v.Pending {
		if !v.Seen[id] || started.IsZero() {
			return nil, fmt.Errorf("Outil de dialogue en attente invalide")
		}
	}
	for id, f := range v.Contexts {
		g.contexts[id] = f.restore()
	}
	for _, f := range v.ToolFailures {
		g.toolFailures = append(g.toolFailures, f.restore())
	}
	totalSignatures := 0
	for _, item := range v.SignatureCounts {
		if item.Count < 1 || g.sigSeen[item.Signature] != 0 {
			return nil, fmt.Errorf("Historique de répétition de dialogue invalide")
		}
		g.sigSeen[item.Signature] = item.Count
		totalSignatures += item.Count
	}
	if totalSignatures != v.Calls || v.Repeated > v.Calls || v.Errors > v.TotalErrors {
		return nil, fmt.Errorf("Historique cumulé de dialogue incomplet")
	}
	return g, nil
}

func dialogueLaunchSHA(a Agent) string {
	b, _ := json.Marshal(struct {
		ID, Work, Task, Attempt, Provider, Role, Mode, Workspace, Command, Prompt string
		Args, Env                                                                 []string
		Limits                                                                    RunLimits
		Timeout                                                                   int
		Restore                                                                   *DialogueRestoreReceipt `json:",omitempty"`
	}{a.ID, a.WorkID, a.TaskID, a.Attempt, a.Provider, a.Role, a.Mode, a.CWD, a.Command, hash([]byte(a.Prompt)), a.Args, a.Env, a.Limits, a.Timeout, a.DialogueRestore})
	return hash(b)
}

func (v dialogueCheckpoint) digest() string {
	v.SHA256 = ""
	b, _ := json.Marshal(v)
	return hash(b)
}

func (v dialogueCheckpoint) restoreGuard(a Agent, root string) (*loopGuard, error) {
	if v.Schema != 1 || v.Agent != a.ID || v.Project != hash([]byte(root)) || v.Launch != dialogueLaunchSHA(a) || v.SHA256 == "" || v.SHA256 != v.digest() || !preparationKey(v.Turn) || v.Guard.Limits != a.Limits {
		return nil, fmt.Errorf("État de dialogue incompatible ou altéré ; aucune reprise autorisée")
	}
	if _, err := time.Parse(time.RFC3339Nano, v.At); err != nil || v.Phase != "running" && v.Phase != "confirmed" && v.Phase != "interrupted" {
		return nil, fmt.Errorf("Frontière de dialogue invalide")
	}
	if v.Session != "" && !validDialogueSession(v.Session) {
		return nil, fmt.Errorf("Identité de session fournisseur invalide")
	}
	if u := v.Usage; u != nil {
		invalid := u.Input < 0 || u.Output < 0
		for _, cached := range []*int64{u.CacheRead, u.CacheCreation, u.CachedInput} {
			invalid = invalid || cached != nil && *cached < 0
		}
		if u.ReportedCost != nil {
			invalid = invalid || *u.ReportedCost < 0 || math.IsNaN(*u.ReportedCost) || math.IsInf(*u.ReportedCost, 0)
		}
		if invalid {
			return nil, fmt.Errorf("Usage de dialogue invalide")
		}
	}
	if v.Phase == "confirmed" && (v.Session == "" || v.Failure != "" || v.Guard.Reason != "" || v.Guard.Degraded != "" || len(v.Guard.Pending) != 0) {
		return nil, fmt.Errorf("Fin de dialogue sans confirmation cohérente")
	}
	return v.Guard.restore()
}
