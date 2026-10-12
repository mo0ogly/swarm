//go:build linux

package engine

import (
	"fmt"
	"strings"
)

type PreparedSkillRefresh struct {
	Confirm bool
	Reason  string
	Skills  []ActionSkillSelection
}

// Engine-produced receipt; external launch requests cannot grant this exception.
type PreparedSkillRefreshReceipt struct {
	Prepared string                 `json:"prepared_id"`
	Original string                 `json:"original_request_sha256"`
	Previous []ActionSkillSelection `json:"previous_skills"`
	Reason   string                 `json:"reason"`
	Actor    string                 `json:"actor"`
}

func samePreparedSkillPaths(old, next []ActionSkillSelection) bool {
	if len(old) == 0 || len(old) != len(next) {
		return false
	}
	for i := range old {
		if old[i].Path != next[i].Path {
			return false
		}
	}
	return true
}

func (s *Store) refreshPreparedSkills(r Launch, choice PreparedSkillRefresh) (Launch, error) {
	if !choice.Confirm || len(strings.TrimSpace(choice.Reason)) < 8 || len(choice.Reason) > 2000 || !samePreparedSkillPaths(r.Skills, choice.Skills) {
		return r, fmt.Errorf("confirmation, motif et mêmes chemins de skills préparés requis")
	}
	changed := false
	for i, selection := range choice.Skills {
		item, _, err := s.readActionSkill(selection.Path)
		if err != nil {
			return r, err
		}
		if selection.SHA256 == "" || item.SHA256 != selection.SHA256 {
			return r, fmt.Errorf("empreinte actuelle du skill requise : %s", selection.Path)
		}
		changed = changed || selection.SHA256 != r.Skills[i].SHA256
	}
	if !changed {
		return r, fmt.Errorf("aucun skill préparé modifié ; utiliser la reprise ordinaire")
	}
	r.SkillRefresh = &PreparedSkillRefreshReceipt{Prepared: r.EventID, Original: preparedRequestDigest(r), Previous: append([]ActionSkillSelection(nil), r.Skills...), Reason: strings.TrimSpace(choice.Reason), Actor: operatorIdentity()}
	r.Skills = append([]ActionSkillSelection(nil), choice.Skills...)
	r.preparedSkillRefresh = true
	return r, nil
}
