//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
)

type RecoveryCriterionFocus struct {
	Index                int    `json:"index"`
	Expected             string `json:"expected"`
	HistoricalVerdict    string `json:"historical_verdict,omitempty"`
	VerificationRequired bool   `json:"verification_required"`
}
type RecoveryTaskFocus struct {
	Revision  int                      `json:"work_revision"`
	Next      string                   `json:"next_action"`
	Blocker   string                   `json:"blocker,omitempty"`
	Criteria  []RecoveryCriterionFocus `json:"criteria"`
	Review    string                   `json:"historical_review,omitempty"`
	Candidate string                   `json:"historical_candidate,omitempty"`
	Report    string                   `json:"historical_report,omitempty"`
	Bounded   bool                     `json:"bounded_excerpt"`
}

// Preserve useful pointers, never raw reports, sibling results or current validity.
func recoveryTaskFocus(tx *sql.Tx, prior Agent, work, task string) (*RecoveryTaskFocus, error) {
	var raw []byte
	if err := tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&raw); err != nil {
		return nil, err
	}
	var w Work
	if err := json.Unmarshal(raw, &w); err != nil {
		return nil, err
	}
	t, _ := w.task(task)
	if t == nil {
		return nil, nil
	}
	f := &RecoveryTaskFocus{Revision: w.Revision, Next: operationText(t.Next, 600), Blocker: operationText(t.Blocker, 600), Criteria: []RecoveryCriterionFocus{}}
	// Only the review attributed to the preceding attempt is carried across.
	var review *IndependentReview
	if r := t.IndependentReview; r != nil && r.Attempt == prior.Attempt && r.Producer == prior.ID {
		review = r
		f.Review = r.ID
		f.Candidate = r.CandidateSHA
		f.Report = operationText(r.Report, 240)
	}
	for i, expected := range t.Criteria {
		if i >= 12 {
			f.Bounded = true
			break
		}
		c := RecoveryCriterionFocus{Index: i + 1, Expected: operationText(expected, 300), VerificationRequired: true}
		if review != nil {
			for _, v := range review.Criteria {
				if v.Index == i+1 {
					c.HistoricalVerdict = v.Verdict
					break
				}
			}
		}
		f.Criteria = append(f.Criteria, c)
	}
	return f, nil
}
