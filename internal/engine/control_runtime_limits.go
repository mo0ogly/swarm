//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"
)

// The existing authenticated Admin role scope supplies a runtime allowance;
// commands, criteria and historical policy/acceptance receipts stay unchanged.
const validationRuntimeRole = "validation"
const validationGoRuntimeRole = "validation-go-test"

func (s *Store) validationRuntimeLimits(work string) (RunLimitsConfigEntry, string, error) {
	return validationRuntimeLimitsWith(s.db, work)
}

func validationRuntimeLimitsWith(q rowQuerier, work string) (RunLimitsConfigEntry, string, error) {
	cfg, err := currentRunLimitsConfigWith(q, ScopeRole, work, validationRuntimeRole)
	if err != nil {
		return cfg, "", err
	}
	inner, err := currentRunLimitsConfigWith(q, ScopeRole, work, validationGoRuntimeRole)
	if err != nil {
		return cfg, "", err
	}
	if inner.Values.ToolSeconds > 0 {
		if cfg.Values.ToolSeconds == 0 || inner.Values.ToolSeconds >= cfg.Values.ToolSeconds {
			return cfg, "", fmt.Errorf("budget validation-go-test explicitement inférieur au budget validation requis")
		}
		raw, err := json.Marshal([]RunLimitsConfigEntry{cfg, inner})
		if err != nil {
			return cfg, "", err
		}
		return cfg, hash(raw), nil
	}
	if cfg.Values.ToolSeconds == 0 {
		return cfg, "", nil
	}
	raw, err := json.Marshal(cfg)
	if err != nil {
		return cfg, "", err
	}
	return cfg, hash(raw), nil
}
func (s *Store) runConfiguredValidationControl(work, root string, c ValidationControl) (ValidationControlResult, []byte, int) {
	cfg, _, err := s.validationRuntimeLimits(work)
	if err != nil {
		return ValidationControlResult{ID: c.ID, Command: append([]string(nil), c.Command...), ExitCode: -1, OutputHash: hash(nil), Summary: "budget de contrôle inaccessible : " + err.Error(), Started: now(), Finished: now()}, nil, 0
	}
	effective := c
	if cfg.Values.ToolSeconds > 0 {
		effective.Timeout = cfg.Values.ToolSeconds
	}
	inner, err := currentRunLimitsConfigWith(s.db, ScopeRole, work, validationGoRuntimeRole)
	if err != nil {
		return ValidationControlResult{ID: c.ID, Command: append([]string(nil), c.Command...), ExitCode: -1, OutputHash: hash(nil), Summary: "budget interne de contrôle inaccessible : " + err.Error(), Started: now(), Finished: now()}, nil, 0
	}
	if inner.Values.ToolSeconds > 0 && goTestControl(c.Command) {
		effective.Command = configuredGoTestDeadline(c.Command, inner.Values.ToolSeconds)
	}
	result, output, total := runValidationControlCaptured(root, effective)
	if inner.Values.ToolSeconds > 0 && goTestControl(c.Command) {
		result.CommandRuntimeLimits = &inner
	}
	if cfg.Values.ToolSeconds > 0 {
		result.RuntimeLimits = &cfg
	}
	return result, output, total
}

// A fresh explicit runtime allowance is a changed precondition, never a
// permission to repeat a control unchanged or to erase its rejected result.
func (s *Store) freshControlRuntimeBudget(w Work, a Agent, item ManagedAttempt, r PlanningRequest) (string, error) {
	if !r.ConfirmRecovery {
		return "", fmt.Errorf("confirmation explicite de reprise du contrôle requise")
	}
	cfg, digest, err := s.validationRuntimeLimits(w.ID)
	if err != nil {
		return "", err
	}
	if digest == "" || cfg.Revision < 1 || cfg.Actor == "" {
		return "", fmt.Errorf("nouveau budget administratif explicite requis")
	}
	inner, err := currentRunLimitsConfigWith(s.db, ScopeRole, w.ID, validationGoRuntimeRole)
	if err != nil {
		return "", err
	}
	parts := controlFailurePattern.FindStringSubmatch(item.Detail)
	if inner.Values.ToolSeconds > 0 && parts != nil {
		checked, err := w.task(parts[2])
		if err != nil {
			return "", err
		}
		for _, control := range checked.ValidationPolicy.Controls {
			if control.ID == parts[1] && goTestControl(control.Command) && inner.Updated > cfg.Updated {
				cfg = inner
			}
		}
	}
	updated, err := time.Parse(time.RFC3339Nano, cfg.Updated)
	if err != nil {
		return "", err
	}
	events, err := s.events(w.ID)
	if err != nil {
		return "", err
	}
	var failureAt time.Time
	for _, event := range events {
		if event.Kind != "managed.conflict" {
			continue
		}
		var failure struct {
			Agent  string `json:"agent"`
			Reason string `json:"reason"`
		}
		if json.Unmarshal(event.Payload, &failure) != nil || failure.Agent != a.ID || failure.Reason != item.Detail {
			continue
		}
		at, err := time.Parse(time.RFC3339Nano, event.At)
		if err != nil {
			return "", err
		}
		if at.After(failureAt) {
			failureAt = at
		}
	}
	if failureAt.IsZero() || !updated.After(failureAt) {
		return "", fmt.Errorf("budget inchangé depuis le refus attribué ; aucune répétition du contrôle")
	}
	return digest, nil
}

// The explicit Admin adapter changes only the Go test harness deadline. The
// approved control remains immutable; the result captures the effective argv.
func goTestControl(command []string) bool {
	return len(command) > 1 && command[0] == "go" && command[1] == "test"
}
func configuredGoTestDeadline(command []string, seconds int) []string {
	result := append([]string(nil), command...)
	if !goTestControl(result) || seconds <= 0 {
		return result
	}
	duration := strconv.Itoa(seconds) + "s"
	forwarded := false
	found := false
	argsIndex := -1
	for i := 2; i < len(result); i++ {
		if result[i] == "-args" {
			forwarded = true
			argsIndex = i
			continue
		}
		flag := "-timeout"
		if forwarded {
			flag = "-test.timeout"
		}
		if result[i] == flag && i+1 < len(result) {
			result[i+1] = duration
			found = true
			i++
			continue
		}
		if strings.HasPrefix(result[i], flag+"=") {
			result[i] = flag + "=" + duration
			found = true
		}
	}
	if !found {
		flag := "-timeout=" + duration
		if argsIndex >= 0 {
			result = append(result[:argsIndex], append([]string{flag}, result[argsIndex:]...)...)
		} else {
			result = append(result, flag)
		}
	}
	return result
}
