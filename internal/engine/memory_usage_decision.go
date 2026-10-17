//go:build linux

package engine

import (
	"encoding/json"
	"slices"
	"time"
)

// Private inputs from a future protected consumer, not an enrollment or an
// attestation constructed by this evaluator. No public JSON admission exists.
type memoryUsageFrame struct {
	snapshot  agentIdentitySnapshot
	authority agentIdentityAuthority
	requester agentIdentityCaller
	recipient agentIdentityCaller
	at        time.Time
}

type memoryUsageDecisionIDs struct {
	eventID        string
	requesterGrant string
	recipientGrant string
}

// A complete pure verdict, never a consumption receipt or permission to release
// bytes after its authority/source frame has changed. No partial verdict escapes.
type memoryUsageDecision struct {
	bindingDigest string
	requester     []agentIdentityDecision
	recipient     []agentIdentityDecision
	expiresAt     time.Time
}

func evaluateMemorySummaryUsage(root string, w Work, r Launch, target memoryUsageTarget, ids memoryUsageDecisionIDs, frame memoryUsageFrame, previous *memoryUsageDecision) (memoryUsageDecision, error) {
	empty := memoryUsageDecision{}
	binding, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		return empty, err
	}
	if frame.authority.projectRoot != root {
		return empty, agentIdentityError("memory_usage_project_mismatch")
	}
	if frame.recipient.actor != target.Recipient {
		return empty, agentIdentityError("memory_usage_recipient_mismatch")
	}
	requester, err := binding.requests(ids.eventID, ids.requesterGrant)
	if err != nil {
		return empty, err
	}
	if !safeName(ids.recipientGrant) {
		return empty, agentIdentityError("memory_usage_binding_invalid")
	}
	recipient := []agentIdentityRequest{}
	for _, source := range binding.projection.Sources {
		if !source.Transmit {
			continue
		}
		key, err := json.Marshal([]string{"swarm-memory-receive-event-v1", ids.eventID, source.Resource})
		if err != nil {
			return empty, err
		}
		request := agentIdentityRequest{Schema: 1, EventID: "memory-receive-" + hash(key), Scope: target.Scope, GrantID: ids.recipientGrant,
			Operation: "memory.read", Resource: source.Resource, ContentDigest: binding.digest}
		if !request.valid() {
			return empty, agentIdentityError("memory_usage_binding_invalid")
		}
		recipient = append(recipient, request)
	}
	if len(recipient) == 0 {
		return empty, agentIdentityError("memory_usage_sources_invalid")
	}
	current := memoryUsageDecision{bindingDigest: binding.digest}
	appendDecision := func(caller agentIdentityCaller, request agentIdentityRequest, out *[]agentIdentityDecision) error {
		decision, err := evaluateAgentIdentity(frame.snapshot, frame.authority, caller, request, frame.at)
		if err != nil {
			return err
		}
		*out = append(*out, decision)
		if current.expiresAt.IsZero() || decision.ExpiresAt.Before(current.expiresAt) {
			current.expiresAt = decision.ExpiresAt
		}
		return nil
	}
	for _, request := range requester {
		if err = appendDecision(frame.requester, request, &current.requester); err != nil {
			return empty, err
		}
	}
	for _, request := range recipient {
		if err = appendDecision(frame.recipient, request, &current.recipient); err != nil {
			return empty, err
		}
	}
	// Authenticate and reevaluate both actors before comparing the previous proof.
	// This preserves the kernel's refusal causes on revocation/expiry/denial.
	if previous != nil && (previous.bindingDigest != current.bindingDigest || !previous.expiresAt.Equal(current.expiresAt) ||
		!slices.Equal(previous.requester, current.requester) || !slices.Equal(previous.recipient, current.recipient)) {
		return empty, agentIdentityError("memory_usage_decision_stale")
	}
	return current, nil
}
