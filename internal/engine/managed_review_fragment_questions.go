//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"strings"
)

// Questions remain attributed to the original immutable inspection. Resolving
// one is a new final-review claim, never a rewrite of that inspection.
type managedFragmentQuestion struct {
	Packet   int    `json:"packet"`
	Artifact int    `json:"artifact"`
	Need     int    `json:"need_index"`
	Text     string `json:"question"`
}

const fragmentQuestionInstructions = `
Chaque unresolved_questions doit recevoir une résolution indépendante. Retourne un objet avec review (l'avis habituel candidate_commit/tasks) et resolutions. Une résolution contient packet, artifact, need_index, verdict (resolved, unknown ou fail), reason et evidence. Pour resolved, cite un extrait original exact visible prouvant la réponse à cette question ; une opinion d'inspection ne suffit pas. Si une seule réserve reste unknown ou fail, aucune tâche ne peut être acceptée. Ne transforme jamais une absence de preuve en conformité.
`

func fragmentDecisionSchema(b managedFragmentFinalEvidence) string {
	if len(b.Questions) == 0 {
		return managedReviewSchema
	}
	return `{"type":"object","additionalProperties":false,"properties":{"review":` + managedReviewSchema + `,"resolutions":{"type":"array","items":{"type":"object","additionalProperties":false,"properties":{"packet":{"type":"integer","minimum":0},"artifact":{"type":"integer","minimum":0},"need_index":{"type":"integer","minimum":0},"verdict":{"type":"string","enum":["resolved","unknown","fail"]},"reason":{"type":"string","minLength":8,"maxLength":1000},"evidence":{"type":"string","maxLength":4000}},"required":["packet","artifact","need_index","verdict","reason","evidence"]}}},"required":["review","resolutions"]}`
}
func parseManagedFragmentDecision(reply string, visible, c managedReviewContext, p managedReviewFragmentPlan, replies []string) (string, []ManagedTaskReview, error) {
	b, err := managedFragmentFinalBundle(c, p, replies)
	if err != nil {
		return "", nil, err
	}
	if len(b.Questions) == 0 {
		return parseManagedReview(reply, visible)
	}
	var response struct {
		Review      json.RawMessage `json:"review"`
		Resolutions []struct {
			Packet   int    `json:"packet"`
			Artifact int    `json:"artifact"`
			Need     int    `json:"need_index"`
			Verdict  string `json:"verdict"`
			Reason   string `json:"reason"`
			Evidence string `json:"evidence"`
		} `json:"resolutions"`
	}
	if len(reply) > managedFragmentReplyLimit {
		return "", nil, fmt.Errorf("résolutions trop grandes")
	}
	if err = strict([]byte(reply), &response); err != nil {
		return "", nil, err
	}
	if len(response.Resolutions) != len(b.Questions) {
		return "", nil, fmt.Errorf("réserves finales non couvertes")
	}
	expected := map[[3]int]bool{}
	for _, q := range b.Questions {
		expected[[3]int{q.Packet, q.Artifact, q.Need}] = true
	}
	impactKeys := map[[3]int]bool{}
	for _, ref := range p.Reused {
		_, inspection, e := parseManagedFragmentInspection(ref.Reply, ref.Original)
		if e != nil {
			return "", nil, e
		}
		var impact managedFragmentFinalEvidence
		addHistoricalImpactQuestion(&impact, ref, inspection)
		q := impact.Questions[0]
		impactKeys[[3]int{q.Packet, q.Artifact, q.Need}] = true
	}
	originals := ""
	for _, s := range visible.Sources {
		originals += "\n" + s.Content
	}
	for _, t := range visible.Tasks {
		controls, _ := json.Marshal(t.Controls)
		originals += "\n" + t.Report + "\n" + string(controls)
	}
	unresolved := false
	for _, r := range response.Resolutions {
		key := [3]int{r.Packet, r.Artifact, r.Need}
		if !expected[key] || len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 1000 || len(r.Evidence) > 4000 {
			return "", nil, fmt.Errorf("résolution inconnue, dupliquée ou invalide")
		}
		delete(expected, key)
		switch r.Verdict {
		case "resolved":
			if impactKeys[key] && !fragmentCurrentChangeEvidence(b.ChangeDiff, r.Evidence) {
				return "", nil, fmt.Errorf("impact historique sans citation d’un changement actuel")
			}
			if len(strings.TrimSpace(r.Evidence)) < 8 || !strings.Contains(originals, r.Evidence) {
				return "", nil, fmt.Errorf("résolution sans preuve originale visible")
			}
		case "unknown", "fail":
			unresolved = true
		default:
			return "", nil, fmt.Errorf("verdict de résolution invalide")
		}
	}
	state, records, err := parseManagedReview(string(response.Review), visible)
	if err != nil {
		return "", nil, err
	}
	if unresolved {
		return "unknown", nil, nil
	}
	return state, records, nil
}

// A historical impact resolution must cite actual added/removed content in the
// current delta, not just an old excerpt, a filename, a producer report or hash.
// Semantic relevance remains the independent reviewer's responsibility.
func fragmentCurrentChangeEvidence(delta, evidence string) bool {
	if len(strings.TrimSpace(evidence)) < 8 || !strings.Contains(delta, evidence) {
		return false
	}
	for _, line := range strings.Split(delta, "\n") {
		if strings.HasPrefix(line, "rename from ") || strings.HasPrefix(line, "rename to ") || strings.HasPrefix(line, "old mode ") || strings.HasPrefix(line, "new mode ") || strings.HasPrefix(line, "new file mode ") || strings.HasPrefix(line, "deleted file mode ") {
			if strings.Contains(evidence, line) {
				return true
			}
		}
		if len(line) < 1 || (line[0] != '+' && line[0] != '-') || strings.HasPrefix(line, "+++") || strings.HasPrefix(line, "---") {
			continue
		}
		if strings.Contains("\n"+evidence+"\n", "\n"+line+"\n") {
			return true
		}
		changed := strings.TrimSpace(line[1:])
		if len(changed) >= 8 && (strings.Contains(changed, evidence) || strings.Contains(evidence, changed)) {
			return true
		}
	}
	return false
}
