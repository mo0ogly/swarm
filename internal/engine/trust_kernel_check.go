//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"time"
)

type TrustKernelCheckPolicy struct {
	Enabled        trustOIDCEnabled `json:"enabled"`
	TimeoutSeconds int              `json:"timeout_seconds"`
	RingBytes      uint64           `json:"ring_bytes"`
}

func (p *TrustKernelCheckPolicy) UnmarshalJSON(raw []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil {
		return err
	}
	for _, key := range []string{"enabled", "timeout_seconds", "ring_bytes"} {
		v, ok := fields[key]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("diagnostic noyau : champs explicites non nuls requis")
		}
	}
	type plain TrustKernelCheckPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustKernelCheckPolicy(v)
	return nil
}
func trustKernelPolicy(p *TrustKernelCheckPolicy) TrustKernelCheckPolicy {
	if p == nil {
		return TrustKernelCheckPolicy{TimeoutSeconds: 15, RingBytes: 65536}
	}
	return *p
}
func validateTrustKernelPolicy(p *TrustKernelCheckPolicy) error {
	if p == nil {
		return nil
	}
	if p.TimeoutSeconds <= 0 || int64(p.TimeoutSeconds) > int64((1<<63-1)/int64(time.Second)) || p.RingBytes < uint64(os.Getpagesize()) || p.RingBytes > 1<<31 || p.RingBytes&(p.RingBytes-1) != 0 {
		return fmt.Errorf("diagnostic noyau : durée positive représentable et ring puissance de deux entre une page et 2^31 octets requis ; 0 refusé")
	}
	return nil
}

type TrustKernelCheckResult struct {
	EffectivePolicy     TrustKernelCheckPolicy `json:"effective_policy"`
	SchemaVersion       int                    `json:"schema_version"`
	Available           bool                   `json:"available"`
	Source              string                 `json:"source"`
	Scope               string                 `json:"scope"`
	Reason              string                 `json:"reason,omitempty"`
	Diagnostic          string                 `json:"diagnostic,omitempty"`
	Events              []kernelObservation    `json:"events"`
	OwnedPIDs           []int                  `json:"owned_pids"`
	FixtureVerified     bool                   `json:"fixture_verified"`
	GlobalQualification bool                   `json:"global_qualification"`
	Lost                uint64                 `json:"lost"`
	CleanupVerified     bool                   `json:"cleanup_verified"`
	Revision            int                    `json:"revision"`
}

func trustKernelDenied() error {
	return &CommandError{Code: "authorization_required", Message: "diagnostic noyau : politique active et administrateur I3 requis"}
}
func trustKernelUnavailable() error {
	return &CommandError{Code: "kernel_unavailable", Message: "collecte eBPF indisponible ; aucun résultat noyau qualifié"}
}
func trustKernelInterrupted() error {
	return &CommandError{Code: "interrupted", Message: "diagnostic noyau interrompu ; aucun résultat qualifié"}
}

func (s *Store) trustKernelCheckUsing(ctx context.Context, raw []byte, token string, runner func(context.Context, TrustKernelCheckPolicy) (TrustKernelCheckResult, error)) (any, error) {
	var empty map[string]json.RawMessage
	if err := strict(raw, &empty); err != nil {
		return nil, err
	}
	if empty == nil || len(empty) != 0 {
		return nil, fmt.Errorf("diagnostic noyau : objet vide requis")
	}
	var p TrustKernelCheckPolicy
	var revision int
	var authority string
	check := func(st *trustState, initial bool) error {
		id, err := findTrustIdentityAt(st, token, time.Now().UTC())
		if err != nil || !trustSecretAdmin(id) || ctx.Err() != nil {
			return trustKernelDenied()
		}
		current := trustKernelPolicy(st.KernelCheck)
		if !bool(current.Enabled) {
			return trustKernelDenied()
		}
		if initial {
			p = current
			revision = st.Revision
			authority = trustAuthorityDigest(id)
		} else if current != p || revision != st.Revision || authority != trustAuthorityDigest(id) {
			return &CommandError{Code: "revision_conflict", Message: "configuration ou autorité modifiée ; résultat noyau refusé"}
		}
		return nil
	}
	if _, err := s.withTrustReadAvailable(func(st *trustState) (any, error) { return nil, check(st, true) }); err != nil {
		return nil, err
	}
	v, err := runner(ctx, p)
	if err != nil {
		return nil, err
	}
	if ctx.Err() != nil {
		return nil, trustKernelInterrupted()
	}
	if _, err = s.withTrustReadAvailable(func(st *trustState) (any, error) { return nil, check(st, false) }); err != nil {
		return nil, err
	}
	if v.EffectivePolicy != p {
		return nil, fmt.Errorf("politique effective du diagnostic incohérente")
	}
	if err = validateTrustKernelResult(v); err != nil {
		return nil, err
	}
	v.Revision = revision
	return v, nil
}

func printTrustKernelCheck(out io.Writer, value any) error {
	raw, err := json.Marshal(value)
	if err != nil {
		return err
	}
	var v TrustKernelCheckResult
	if err = kernelCheckJSONValue(json.NewDecoder(bytes.NewReader(raw))); err != nil {
		return err
	}
	if err = strict(raw, &v); err != nil {
		return err
	}
	if err = validateTrustKernelResult(v); err != nil {
		return err
	}
	if err = printJSON(out, v); err != nil {
		return err
	}
	if !v.Available {
		return trustKernelUnavailable()
	}
	if !v.FixtureVerified {
		return &CommandError{Code: "kernel_unqualified", Message: "diagnostic eBPF incomplet ; aucune qualification noyau"}
	}
	return nil
}

func validateTrustKernelResult(v TrustKernelCheckResult) error {
	bad := func() error { return fmt.Errorf("réponse du diagnostic noyau invalide") }
	if validateTrustKernelPolicy(&v.EffectivePolicy) != nil || !bool(v.EffectivePolicy.Enabled) {
		return bad()
	}
	if v.SchemaVersion != 1 || v.Source != "ebpf" || v.Scope != "owned_fixture_only" || v.GlobalQualification || v.Events == nil || len(v.OwnedPIDs) != 1 || v.OwnedPIDs[0] <= 0 || !v.CleanupVerified {
		return bad()
	}
	if !v.Available && (v.FixtureVerified || len(v.Events) != 0 || v.Lost != 0) {
		return bad()
	}
	kinds := map[string]bool{}
	var previous, loss, clock uint64
	var boot, start string
	for _, e := range v.Events {
		raw, _ := json.Marshal(e)
		if _, err := decodeKernelObservation(raw); err != nil {
			return err
		}
		if e.PID != v.OwnedPIDs[0] || e.MissionID != "diagnostic" || e.TaskID != "kernel-check" || e.AttemptID != e.CollectorBootID || e.ResourceDigest != hash([]byte(kernelCheckFixture)) || e.Sequence <= previous || e.Sequence-previous-1 != e.LostBefore || e.KtimeNS < clock {
			return bad()
		}
		if (boot != "" && boot != e.CollectorBootID) || (start != "" && start != e.PIDStartTime) {
			return bad()
		}
		boot = e.CollectorBootID
		start = e.PIDStartTime
		clock = e.KtimeNS
		previous = e.Sequence
		loss += e.LostBefore
		kinds[e.EventType] = true
	}
	if loss > v.Lost || (v.FixtureVerified && (v.Lost != 0 || !kinds["process"] || !kinds["file"] || !kinds["network"])) {
		return bad()
	}
	return nil
}
