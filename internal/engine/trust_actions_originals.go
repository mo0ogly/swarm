//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
)

func trustCommandOriginal(cmd TrustCommand) ([]byte, error) {
	if len(cmd.Identities) > 0 || cmd.VerificationKey != "" || cmd.Export != nil || cmd.Anchor != nil {
		return nil, fmt.Errorf("champs de configuration ou de vérification interdits dans une commande métier")
	}
	if len(cmd.original) == 0 {
		return json.Marshal(cmd)
	}
	var decoded TrustCommand
	if err := strict(cmd.original, &decoded); err != nil {
		return nil, err
	}
	if hashMust(decoded) != hashMust(cmd) {
		return nil, fmt.Errorf("original différent de la commande validée")
	}
	return append([]byte{}, cmd.original...), nil
}

func verifyTrustOriginals(exp TrustProofExport) error {
	used := map[string]bool{}
	for _, ev := range exp.Evidence {
		raw := exp.Originals[ev.OriginalHash]
		if len(raw) == 0 || hash(raw) != ev.OriginalHash || !json.Valid(raw) {
			return fmt.Errorf("original absent ou altéré à la séquence %d", ev.Sequence)
		}
		used[ev.OriginalHash] = true
	}
	if len(used) != len(exp.Originals) {
		return fmt.Errorf("original non lié à la chaîne de preuve")
	}
	return nil
}

func trustStoredReply(st *trustState, event string) (json.RawMessage, error) {
	raw := st.EventReplies[event]
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil || len(fields) == 0 {
		return nil, fmt.Errorf("réponse originale absente ou invalide")
	}
	return json.RawMessage(append([]byte{}, raw...)), nil
}
