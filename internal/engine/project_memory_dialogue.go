//go:build linux

package engine

import "fmt"

// A restored provider session retains every ancestor's received context. Read
// the retained lineage without rewriting those contexts or assuming the latest
// selection erased earlier knowledge. The reader may be the launch transaction.
func agentMemoryReferences(reader memoryReader, a Agent) ([]MemoryReference, error) {
	refs := []MemoryReference{}
	seenRefs := map[MemoryReference]bool{}
	seenAgents := map[string]bool{}
	for {
		if seenAgents[a.ID] {
			return nil, fmt.Errorf("Filiation mémoire de conversation cyclique")
		}
		seenAgents[a.ID] = true
		if a.Context != nil {
			local := map[string]bool{}
			for _, ref := range a.Context.Memory {
				key := ref.Work + "/" + ref.ID
				if local[key] {
					return nil, fmt.Errorf("Mémoire sélectionnée plusieurs fois")
				}
				local[key] = true
				if !seenRefs[ref] {
					refs = append(refs, ref)
					seenRefs[ref] = true
				}
			}
		}
		r := a.DialogueRestore
		if r == nil {
			return refs, nil
		}
		if a.Mode != "dialogue" || !r.DecisionRecorded || r.Actor == "" || r.At == "" || r.SourceAgent == "" || r.SourceAgent != a.Previous || r.PriorTurns < 1 || r.PriorTurns >= 20 {
			return nil, fmt.Errorf("Filiation mémoire de conversation invalide")
		}
		var raw []byte
		var status, desired string
		if err := reader.QueryRow("SELECT body,status,desired FROM agents WHERE id=?", r.SourceAgent).Scan(&raw, &status, &desired); err != nil {
			return nil, fmt.Errorf("Source mémoire de conversation inaccessible : %w", err)
		}
		prior, err := decodeAgentRow(raw, status, desired)
		if err != nil {
			return nil, err
		}
		if prior.ID != r.SourceAgent || prior.Attempt != r.SourceAttempt || prior.WorkID != a.WorkID || prior.TaskID != a.TaskID || prior.Mode != "dialogue" || prior.Provider != a.Provider || prior.Role != a.Role || prior.CWD != a.CWD || prior.Context == nil || dialogueSourceSHA(prior) != r.SourceState {
			return nil, fmt.Errorf("Source mémoire de conversation modifiée ou incompatible")
		}
		root := prior.ID
		if prior.DialogueRestore != nil {
			root = prior.DialogueRestore.RootAgent
			if prior.DialogueRestore.PriorTurns >= r.PriorTurns {
				return nil, fmt.Errorf("Filiation mémoire de conversation non décroissante")
			}
		}
		if root != r.RootAgent {
			return nil, fmt.Errorf("Racine mémoire de conversation incompatible")
		}
		a = prior
	}
}

func (s *Store) projectMemoryGuardWith(reader memoryReader, a Agent) error {
	refs, err := agentMemoryReferences(reader, a)
	if err != nil {
		return err
	}
	_, err = s.memoryAttachments(reader, refs, a.Role)
	return err
}

func (s *Store) dialogueMemorySelectionGuard(reader memoryReader, a Agent, selected []MemoryReference) error {
	refs, err := agentMemoryReferences(reader, a)
	if err != nil {
		return err
	}
	if _, err = s.memoryAttachments(reader, refs, a.Role); err != nil {
		return err
	}
	selection := map[MemoryReference]bool{}
	for _, ref := range selected {
		selection[ref] = true
	}
	for _, ref := range refs {
		if !selection[ref] {
			return fmt.Errorf("Mémoire historique absente de la sélection courante : démarrer une nouvelle conversation.")
		}
	}
	return nil
}
