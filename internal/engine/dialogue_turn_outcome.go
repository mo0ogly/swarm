//go:build linux

package engine

import "strings"

// A process exit is checked by the wrapper. This receipt separately qualifies
// the provider stream before the supervisor confirms a completed dialogue turn.
type dialogueTurnOutcome struct {
	provider  string
	session   string
	completed bool
	failure   string
}

func (o *dialogueTurnOutcome) observe(data map[string]any) {
	if o.failure != "" {
		return
	}
	if err := structuredProviderFailure(data); err != nil {
		o.failure = err.Error() // Existing classifier excludes private error text.
		return
	}
	if o.completed {
		o.failure = "Événement reçu après la fin fournisseur ; tour non confirmé"
		return
	}
	kind, _ := data["type"].(string)
	var candidate string
	if kind == "thread.started" {
		candidate, _ = data["thread_id"].(string)
	} else if kind == "system" || kind == "result" {
		candidate, _ = data["session_id"].(string)
	}
	if candidate != "" {
		if !preparationKey(candidate) || strings.HasPrefix(candidate, "-") || o.session != "" && o.session != candidate {
			o.failure = "Identité de conversation incohérente ; tour non confirmé"
			return
		}
		o.session = candidate
	}
	switch kind {
	case "turn.completed":
		if o.provider != "codex" {
			o.failure = "Fin fournisseur incompatible avec l’adaptateur de dialogue"
			return
		}
		o.completed = true
	case "result":
		failed, hasError := data["is_error"]
		if o.provider != "claude" || data["subtype"] != "success" || hasError && failed != false {
			o.failure = "Résultat fournisseur sans succès explicite ; tour non confirmé"
			return
		}
		o.completed = true
	}
}

func (o dialogueTurnOutcome) confirmation(session, previous string) string {
	if o.failure != "" {
		return o.failure
	}
	if !o.completed {
		return "Fin fournisseur positive absente ; tour non confirmé"
	}
	if o.session == "" || session != o.session || previous != "" && previous != session {
		return "Identité de conversation incohérente ; tour non confirmé"
	}
	return ""
}
