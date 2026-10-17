//go:build linux

package engine

import "fmt"

// Existing launch transport capacity, also applied after recovery is appended.
// This is not a new provider limit, retention rule or execution budget.
const agentContextTransportBytes = 128000

// Bind the current receipt to the complete prompt without mutating a manifest
// already retained by an earlier stage or caller. Selection metadata is kept.
func bindAgentContext(a *Agent) error {
	if len(a.Prompt) > agentContextTransportBytes {
		return fmt.Errorf("Contexte supérieur à %d octets : réduire les pièces jointes ou le brief.", agentContextTransportBytes)
	}
	manifest := ContextManifest{}
	if a.Context != nil {
		manifest = *a.Context
	}
	manifest.SHA256, manifest.Bytes = hash([]byte(a.Prompt)), len(a.Prompt)
	a.Context = &manifest
	return nil
}

// Preserve the prepared receipt and validate the complete new input before
// changing the caller's record. No provider has been started at this point.
func appendAgentRecoveryContext(a *Agent, recovery string) error {
	current := *a
	if current.PreparedContext == nil && current.Context != nil {
		prepared := *current.Context
		current.PreparedContext = &prepared
	}
	current.Prompt += recovery
	if err := bindAgentContext(&current); err != nil {
		return err
	}
	*a = current
	return nil
}
