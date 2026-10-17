//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"time"
)

type TrustKernelObservePolicy struct {
	Enabled        trustOIDCEnabled `json:"enabled"`
	WindowSeconds  int              `json:"window_seconds"`
	TimeoutSeconds int              `json:"timeout_seconds"`
	RingBytes      uint64           `json:"ring_bytes"`
	MaxEvents      int              `json:"max_events"`
}

func (p *TrustKernelObservePolicy) UnmarshalJSON(raw []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil {
		return err
	}
	for _, k := range []string{"enabled", "window_seconds", "timeout_seconds", "ring_bytes", "max_events"} {
		v, ok := fields[k]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("observation noyau : champs explicites non nuls requis")
		}
	}
	if err := kernelCheckJSONValue(json.NewDecoder(bytes.NewReader(raw))); err != nil {
		return err
	}
	type plain TrustKernelObservePolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustKernelObservePolicy(v)
	return nil
}
func kernelObservePolicy(p *TrustKernelObservePolicy) TrustKernelObservePolicy {
	if p == nil {
		return TrustKernelObservePolicy{WindowSeconds: 5, TimeoutSeconds: 15, RingBytes: 65536, MaxEvents: 128}
	}
	return *p
}
func validateKernelObservePolicy(p *TrustKernelObservePolicy) error {
	if p == nil {
		return nil
	}
	base := TrustKernelCheckPolicy{Enabled: p.Enabled, TimeoutSeconds: p.TimeoutSeconds, RingBytes: p.RingBytes}
	if err := validateTrustKernelPolicy(&base); err != nil {
		return err
	}
	if p.WindowSeconds <= 0 || p.WindowSeconds >= p.TimeoutSeconds || p.MaxEvents <= 0 || int64(p.MaxEvents) > trustPolicyBounds()["kernel_observation_max_events"]["max"] {
		return fmt.Errorf("observation noyau : fenêtre positive inférieure au délai total et maximum d’événements positif requis ; 0 refusé")
	}
	return nil
}

type kernelObserveAttribution struct {
	AgentID string `json:"agent_id"`
	Mission string `json:"mission_id"`
	Task    string `json:"task_id"`
	Attempt string `json:"attempt_id"`
	PID     int    `json:"pid"`
	Start   string `json:"pid_start_time"`
	Digest  string `json:"resource_digest"`
}

func kernelObserveContext(l kernelObservationLease) kernelObserveAttribution {
	return kernelObserveAttribution{l.AgentID, l.Mission, l.Task, l.Attempt, l.PID, l.Start, l.ResourceDigest}
}

type TrustKernelObserveResult struct {
	SchemaVersion       int                      `json:"schema_version"`
	Available           bool                     `json:"available"`
	Source              string                   `json:"source"`
	Scope               string                   `json:"scope"`
	Attribution         kernelObserveAttribution `json:"attribution"`
	Policy              TrustKernelObservePolicy `json:"effective_policy"`
	Revision            int                      `json:"revision"`
	Events              []kernelObservation      `json:"events"`
	Attempts            uint64                   `json:"attempts,string"`
	Emitted             uint64                   `json:"emitted,string"`
	KernelLost          uint64                   `json:"kernel_lost,string"`
	Omitted             uint64                   `json:"omitted,string"`
	CleanupVerified     bool                     `json:"cleanup_verified"`
	Complete            bool                     `json:"complete"`
	GlobalQualification bool                     `json:"global_qualification"`
	StopReason          string                   `json:"stop_reason"`
	Diagnostic          string                   `json:"diagnostic,omitempty"`
}

func kernelObserveFixed(p TrustKernelObservePolicy, l kernelObservationLease) TrustKernelObserveResult {
	return TrustKernelObserveResult{SchemaVersion: 1, Source: "ebpf", Scope: "registered_main_thread", Attribution: kernelObserveContext(l), Policy: p, Events: []kernelObservation{}}
}

type kernelObserveCommand struct {
	AgentID string `json:"agent_id"`
}

func (s *Store) trustKernelObserveUsing(ctx context.Context, raw []byte, token string, run func(context.Context, TrustKernelObservePolicy, kernelObservationLease, func() error) (TrustKernelObserveResult, error)) (any, error) {
	var command kernelObserveCommand
	if err := strict(raw, &command); err != nil {
		return nil, err
	}
	if !safeName(command.AgentID) {
		return nil, fmt.Errorf("observation noyau : agent_id enregistré requis")
	}
	var policy TrustKernelObservePolicy
	var revision int
	var authority string
	check := func(initial bool) error {
		if ctx.Err() != nil {
			return trustKernelInterrupted()
		}
		_, err := s.withTrustReadAvailable(func(st *trustState) (any, error) {
			id, e := findTrustIdentityAt(st, token, time.Now().UTC())
			current := kernelObservePolicy(st.KernelObservation)
			if e != nil || !trustSecretAdmin(id) || !bool(current.Enabled) {
				return nil, trustKernelDenied()
			}
			if initial {
				policy = current
				revision = st.Revision
				authority = trustAuthorityDigest(id)
			} else if current != policy || revision != st.Revision || authority != trustAuthorityDigest(id) {
				return nil, &CommandError{Code: "revision_conflict", Message: "configuration ou autorité modifiée ; observation refusée"}
			}
			return nil, nil
		})
		return err
	}
	if err := check(true); err != nil {
		return nil, err
	}
	source, err := s.kernelAttemptObservationSource(command.AgentID)
	if err != nil {
		return nil, err
	}
	fresh := func() error {
		if err := check(false); err != nil {
			return err
		}
		current, err := s.kernelAttemptObservationSource(command.AgentID)
		if err != nil {
			return err
		}
		if current != source {
			return fmt.Errorf("source d’observation modifiée")
		}
		return nil
	}
	result, err := run(ctx, policy, source, fresh)
	if err != nil {
		return nil, err
	}
	if err = fresh(); err != nil {
		return nil, err
	}
	if err = validateKernelObserveResult(result, policy, source); err != nil {
		return nil, err
	}
	result.Revision = revision
	return result, nil
}

func validateKernelObserveResult(v TrustKernelObserveResult, p TrustKernelObservePolicy, l kernelObservationLease) error {
	bad := func() error { return fmt.Errorf("résultat d’observation noyau incohérent") }
	if validateKernelObservePolicy(&p) != nil || !bool(p.Enabled) || v.SchemaVersion != 1 || v.Source != "ebpf" || v.Scope != "registered_main_thread" || v.Policy != p || v.Attribution != kernelObserveContext(l) || v.Events == nil || v.Complete || v.GlobalQualification || !v.CleanupVerified || len(v.Events) > p.MaxEvents {
		return bad()
	}
	if l.AgentID == "" || l.Mission == "" || l.Task == "" || l.Attempt == "" || l.PID <= 0 || l.Start == "" || !kernelObservationHex(l.ResourceDigest, 32) {
		return bad()
	}
	if !v.Available {
		if len(v.Events) != 0 || v.Attempts != 0 || v.Emitted != 0 || v.KernelLost != 0 || v.Omitted != 0 || v.StopReason != "unavailable" {
			return bad()
		}
		return nil
	}
	if v.StopReason != "window" && v.StopReason != "event_limit" {
		return bad()
	}
	if v.StopReason == "event_limit" && len(v.Events) != p.MaxEvents {
		return bad()
	}
	if v.Attempts < v.Emitted || v.KernelLost != v.Attempts-v.Emitted || v.Emitted < uint64(len(v.Events)) || v.Omitted != v.Emitted-uint64(len(v.Events)) {
		return bad()
	}
	var sequence, clock, lost uint64
	boot := ""
	for _, e := range v.Events {
		raw, err := json.Marshal(e)
		if err != nil {
			return err
		}
		if _, err = decodeKernelObservation(raw); err != nil {
			return err
		}
		if boot == "" {
			boot = e.CollectorBootID
		}
		if e.CollectorBootID != boot || e.PID != l.PID || e.PIDStartTime != l.Start || e.MissionID != l.Mission || e.TaskID != l.Task || e.AttemptID != l.Attempt || e.ResourceDigest != l.ResourceDigest || e.Sequence <= sequence || e.KtimeNS < clock || e.LostBefore != e.Sequence-sequence-1 {
			return bad()
		}
		if ^uint64(0)-lost < e.LostBefore {
			return bad()
		}
		lost += e.LostBefore
		sequence = e.Sequence
		clock = e.KtimeNS
	}
	if lost > v.KernelLost {
		return bad()
	}
	return nil
}
func printKernelObserve(out io.Writer, value any) error {
	raw, err := json.Marshal(value)
	if err != nil {
		return err
	}
	var fields map[string]json.RawMessage
	if err = json.Unmarshal(raw, &fields); err != nil {
		return err
	}
	for _, k := range []string{"schema_version", "available", "source", "scope", "attribution", "effective_policy", "revision", "events", "attempts", "emitted", "kernel_lost", "omitted", "cleanup_verified", "complete", "global_qualification", "stop_reason"} {
		v, ok := fields[k]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("réponse d’observation incomplète")
		}
	}
	var v TrustKernelObserveResult
	if err = strict(raw, &v); err != nil {
		return err
	}
	a := v.Attribution
	l := kernelObservationLease{AgentID: a.AgentID, Mission: a.Mission, Task: a.Task, Attempt: a.Attempt, PID: a.PID, Start: a.Start, ResourceDigest: a.Digest}
	if err = validateKernelObserveResult(v, v.Policy, l); err != nil {
		return err
	}
	if err = printJSON(out, v); err != nil {
		return err
	}
	if !v.Available {
		return trustKernelUnavailable()
	}
	return nil
}
