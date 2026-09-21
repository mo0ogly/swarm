//go:build linux

package main

import (
	"fmt"
	"strings"
	"time"
)

const (
	recoveryProviderLimit  = "provider_limit"
	recoveryTransient      = "transient"
	recoveryEnvironment    = "environment"
	recoveryConflict       = "conflict"
	recoveryBusiness       = "business"
	recoveryUnknown        = "unknown"
	recoveryExecutionLimit = "execution_limit"

	recoveryDispositionRetry        = "retry"
	recoveryDispositionWait         = "wait"
	recoveryDispositionCorrection   = "authorized-correction"
	recoveryDispositionIntervention = "intervention"
)

const recoveryDelay = time.Second

type recoveryAssessment struct {
	Category         string
	CauseFingerprint string
	OperationID      string
	NextEligibleAt   time.Time
	Disposition      string
	Reason           string
}

func recoveryForLaunch(r Launch, previous Agent) RecoveryState {
	operation := r.recoveryOperation
	if operation == "" && previous.ID != "" {
		operation = previous.Recovery.OperationID
		if operation == "" {
			operation = previous.ID
		}
	}
	if operation == "" {
		operation = r.EventID
	}
	used := 0
	if previous.ID != "" {
		used = previous.Recovery.AutomaticUsed
	}
	if r.Origin == originConductor {
		used++
	}
	if used > maxAutomaticAttempts {
		used = maxAutomaticAttempts
	}
	state := RecoveryState{
		Category: r.recoveryCategory, CauseFingerprint: r.recoveryCause,
		OperationID: operation, AutomaticUsed: used, AutomaticMax: maxAutomaticAttempts,
		BudgetRemaining: maxAutomaticAttempts - used, NextEligibleAt: r.recoveryNext,
	}
	if previous.ID != "" && r.recoveryCategory == "" {
		state.Category = recoveryCategoryFor(previous)
		state.CauseFingerprint = recoveryCauseFingerprint(previous, state.Category)
	}
	switch state.Category {
	case recoveryBusiness:
		state.Disposition = recoveryDispositionCorrection
	case recoveryTransient, recoveryConflict:
		state.Disposition = recoveryDispositionRetry
	case recoveryEnvironment, recoveryProviderLimit:
		state.Disposition = recoveryDispositionWait
	}
	return state
}

func recoveryCategoryFor(a Agent) string {
	if a.ProviderCooldown != nil {
		return recoveryProviderLimit
	}
	diagnostic := fallbackAttemptDiagnostic(a)
	// The terminal stop cause takes priority over earlier incidental tool errors.
	// A missing file during exploration is not why a 100-call run was stopped.
	if a.StopKind == "garde" && diagnostic.LimitReached {
		return recoveryExecutionLimit
	}
	hasCheck := false
	for _, item := range diagnostic.Items {
		switch item.Category {
		case "environment", "configuration":
			return recoveryEnvironment
		case "check":
			hasCheck = true
		}
	}
	text := strings.ToLower(recoveryCauseText(a))
	for _, marker := range []string{"revision conflict", "revision_confl", "révision périm", "concurrent modification", "merge conflict", "conflit d’écriture", "conflit d'ecriture"} {
		if strings.Contains(text, marker) {
			return recoveryConflict
		}
	}
	for _, marker := range []string{"temporarily unavailable", "temporary failure", "temporaire", "connection reset", "connexion réinitialisée", "connection refused", "timeout", "timed out", "délai dépassé", "code 502", "code 503", "code 504", "unexpected eof"} {
		if strings.Contains(text, marker) {
			return recoveryTransient
		}
	}
	if hasCheck {
		return recoveryBusiness
	}
	return recoveryUnknown
}

func recoveryCauseText(a Agent) string {
	parts := []string{a.Activity}
	for _, item := range fallbackAttemptDiagnostic(a).Items {
		parts = append(parts, item.Category, item.Cause)
		parts = append(parts, item.Traces...)
	}
	return strings.Join(parts, " | ")
}

func recoveryCauseFingerprint(a Agent, category string) string {
	parts := []string{category}
	diagnostic := fallbackAttemptDiagnostic(a)
	for _, item := range diagnostic.Items {
		parts = append(parts, item.Category)
		parts = append(parts, item.Traces...)
	}
	if len(parts) == 1 {
		parts = append(parts, a.Activity)
	}
	canonical := strings.ToLower(strings.Join(strings.Fields(strings.Join(parts, " | ")), " "))
	return hash([]byte(category + "|" + canonical))
}

func assessRecovery(a Agent, task Task, at time.Time) recoveryAssessment {
	category := a.Recovery.ObservedCategory
	if category == "" {
		category = recoveryCategoryFor(a)
	}
	cause := a.Recovery.ObservedCauseFingerprint
	if cause == "" {
		cause = recoveryCauseFingerprint(a, category)
	}
	operation := a.Recovery.OperationID
	if operation == "" {
		operation = a.ID
	}
	result := recoveryAssessment{Category: category, CauseFingerprint: cause, OperationID: operation, Disposition: recoveryDispositionIntervention}
	if category == recoveryExecutionLimit {
		result.Reason = "limite d’exécution atteinte : examiner le travail conservé et les preuves manquantes avant une reprise autorisée"
		return result
	}
	if category == recoveryProviderLimit && a.ProviderCooldown != nil && a.ProviderCooldown.active(at) {
		result.Disposition = recoveryDispositionWait
		result.Reason = a.ProviderCooldown.message()
		if a.ProviderCooldown.ResetAt > 0 {
			result.NextEligibleAt = time.Unix(a.ProviderCooldown.ResetAt, 0)
		}
		return result
	}
	if category == recoveryEnvironment {
		result.Disposition = recoveryDispositionWait
		result.Reason = "défaut d’environnement : relance automatique retenue ; aucune relance automatique sans nouvelle vérification technique"
		return result
	}
	if category == recoveryUnknown {
		result.Reason = "cause inconnue ou signal absent ; aucune reprise automatique sûre"
		return result
	}
	used := a.Recovery.AutomaticUsed
	if used >= maxAutomaticAttempts {
		result.Reason = fmt.Sprintf("budget de reprise épuisé : %d/%d tentatives automatiques", used, maxAutomaticAttempts)
		return result
	}
	if a.Recovery.CauseFingerprint != "" && a.Recovery.CauseFingerprint == cause {
		result.Reason = "cause commune inchangée après reprise ; boucle automatique arrêtée"
		return result
	}
	if category == recoveryBusiness && task.PlanMaxAttempts <= 0 {
		result.Reason = "échec métier : aucun cycle de correction explicitement autorisé par le plan"
		return result
	}
	result.NextEligibleAt = recoveryEligibleAt(a, category)
	if at.Before(result.NextEligibleAt) {
		result.Disposition = recoveryDispositionWait
		result.Reason = "reprise bornée différée jusqu’au " + result.NextEligibleAt.UTC().Format(time.RFC3339Nano)
		return result
	}
	result.Disposition = recoveryDispositionRetry
	if category == recoveryBusiness {
		result.Disposition = recoveryDispositionCorrection
		result.Reason = "correction métier autorisée par la borne de tentatives du plan"
	} else if category == recoveryConflict {
		result.Reason = "conflit : relire l’état courant et recalculer avant tout effet"
	} else {
		result.Reason = "incident transitoire : reprise avec la même identité d’opération"
	}
	return result
}

// finalizeRecoveryState freezes the observed cause and the next time boundary
// before the terminal agent row is saved. A restarted conductor therefore
// makes the same decision; it does not reconstruct a fresh budget in memory.
func finalizeRecoveryState(a *Agent) {
	if a.Recovery.OperationID == "" {
		a.Recovery.OperationID = a.ID
	}
	if a.Recovery.AutomaticMax == 0 {
		a.Recovery.AutomaticMax = maxAutomaticAttempts
	}
	a.Recovery.BudgetRemaining = max(0, a.Recovery.AutomaticMax-a.Recovery.AutomaticUsed)
	if a.Status != "failed" && a.Status != "interrupted" || a.Status == "interrupted" && a.StopKind == originOperator {
		return
	}
	category := recoveryCategoryFor(*a)
	a.Recovery.ObservedCategory = category
	a.Recovery.ObservedCauseFingerprint = recoveryCauseFingerprint(*a, category)
	eligible := recoveryEligibleAt(*a, category)
	if !eligible.IsZero() {
		a.Recovery.NextEligibleAt = eligible.UTC().Format(time.RFC3339Nano)
	}
	switch category {
	case recoveryTransient, recoveryConflict:
		a.Recovery.Disposition = recoveryDispositionRetry
	case recoveryEnvironment, recoveryProviderLimit:
		a.Recovery.Disposition = recoveryDispositionWait
	default:
		a.Recovery.Disposition = recoveryDispositionIntervention
	}
}

func recoveryEligibleAt(a Agent, category string) time.Time {
	if category == recoveryExecutionLimit {
		return time.Time{}
	}
	if category == recoveryProviderLimit && a.ProviderCooldown != nil {
		if a.ProviderCooldown.ResetAt > 0 {
			return time.Unix(a.ProviderCooldown.ResetAt, 0)
		}
		return time.Time{}
	}
	ended, err := time.Parse(time.RFC3339Nano, a.Ended)
	if err != nil {
		return time.Time{}
	}
	delay := recoveryDelay
	if category == recoveryConflict || category == recoveryBusiness {
		delay *= 2
	}
	return ended.Add(delay)
}

func recoveryInstruction(category, operation, cause string) string {
	base := fmt.Sprintf("Reprise Swarm bornée. Identité d’opération : %s. Empreinte de cause : %s. ", operation, cause)
	switch category {
	case recoveryTransient:
		return base + "Vérifier l’effet déjà produit avant de reprendre la même opération ; ne pas le dupliquer."
	case recoveryConflict:
		return base + "Relire l’état courant et recalculer la décision ; ne jamais écraser la version concurrente."
	case recoveryBusiness:
		return base + "Appliquer uniquement la correction autorisée par le plan, puis rejouer les contrôles."
	}
	return base
}
