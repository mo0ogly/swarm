//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

func (s *Store) validateManagedFinalRestartArchive(r IndependentReview, p managedReviewFragmentPlan, j managedFragmentJournal) error {
	a := r.FragmentJournal
	if a.FinalJournal == "" && a.FinalJournalDigest == "" {
		return nil
	}
	if r.State != "error" || (r.FailureCode != reviewFragmentProtocolError && r.FailureCode != reviewFragmentInterruptedError) {
		return fmt.Errorf("reprise complète réservée à un incident de protocole ou de moteur")
	}
	path, err := safeReport(s.root, a.FinalJournal)
	if err != nil {
		return err
	}
	raw, err := os.ReadFile(path)
	if err != nil || !citationHashValid(a.FinalJournalDigest) || hash(raw) != a.FinalJournalDigest {
		return fmt.Errorf("journal final original modifié")
	}
	var f managedFragmentFinalJournal
	if err = strict(raw, &f); err != nil {
		return err
	}
	jraw, _ := json.Marshal(j)
	if f.Version < 1 || f.Version > managedFragmentFinalProtocolVersion || f.InspectionDigest != hash(jraw) || len(f.Calls) == 0 || len(f.Calls) > p.ReservedFinalCalls+len(f.ResumeCalls) {
		return fmt.Errorf("historique final non attribué")
	}
	authorized := map[string]bool{}
	for _, id := range f.ResumeCalls {
		if id == "" || authorized[id] {
			return fmt.Errorf("reprise historique dupliquée")
		}
		authorized[id] = true
	}
	seen := map[string]bool{}
	interrupted := map[string]bool{}
	for _, entry := range j.Entries {
		seen[entry.CallID] = true
	}
	phase := "selection"
	for i, call := range f.Calls {
		if !safeName(call.CallID) || seen[call.CallID] || call.Phase != phase || !citationHashValid(call.PromptDigest) {
			return fmt.Errorf("appel final historique non attribué")
		}
		seen[call.CallID] = true
		switch call.State {
		case "interrupted":
			interrupted[call.CallID] = true
			matches, err := filepath.Glob(filepath.Join(s.root, filepath.Dir(r.Context), call.CallID+"-raw-*.json"))
			if err != nil || len(matches) > 1 {
				return fmt.Errorf("originaux historiques ambigus")
			}
			for _, name := range matches {
				rel, err := filepath.Rel(s.root, name)
				if err != nil {
					return err
				}
				path, err := safeReport(s.root, rel)
				if err != nil {
					return err
				}
				original, err := os.ReadFile(path)
				if err != nil || filepath.Base(path) != call.CallID+"-raw-"+hash(original)+".json" {
					return fmt.Errorf("original historique de réponse modifié")
				}
			}
			if call.Reply != "" || call.ReplyDigest != "" || (i < len(f.Calls)-1 && !authorized[call.CallID]) {
				return fmt.Errorf("interruption historique invalide")
			}
		case "ready":
			if phase != "selection" || hash([]byte(call.Reply)) != call.ReplyDigest {
				return fmt.Errorf("sélection historique modifiée")
			}
			var request managedFragmentFinalRequest
			if err = strict([]byte(call.Reply), &request); err != nil {
				return err
			}
			planRaw, _ := json.Marshal(p)
			if request.State != "ready" || request.Candidate != p.Candidate || request.ContextDigest != p.ContextDigest || request.PlanDigest != hash(planRaw) {
				return fmt.Errorf("sélection historique non attribuée")
			}
			phase = "decision"
		default:
			return fmt.Errorf("réservation active ou verdict sémantique : reprise complète refusée")
		}
	}
	for id := range authorized {
		if !interrupted[id] {
			return fmt.Errorf("reprise historique sans appel original")
		}
	}
	if f.Calls[len(f.Calls)-1].State != "interrupted" {
		return fmt.Errorf("aucune interruption finale à diagnostiquer")
	}
	return nil
}
