package main

import (
	"fmt"
	"os"
	"sort"
	"strings"
)

// Derived on every read, never persisted as a substitute for checking the files.
type TaskValidation struct {
	State    string       `json:"state"`
	Fresh    bool         `json:"fresh"`
	Blockers []string     `json:"blockers"`
	Owner    string       `json:"owner"`
	Next     string       `json:"next"`
	Evidence TaskEvidence `json:"evidence"`
}
type WorkValidation struct {
	// State is a stable business code. Label is presentation text and may be
	// localized by a CLI client; consumers must never branch on Label.
	State      string                    `json:"state"`
	Label      string                    `json:"label"`
	Validated  int                       `json:"validated"`
	Historical int                       `json:"historical"`
	Stale      int                       `json:"stale"`
	Tasks      map[string]TaskValidation `json:"tasks"`
}

func (s *Store) validationState(w *Work) WorkValidation {
	s = s.readScope()
	v := WorkValidation{Tasks: map[string]TaskValidation{}}
	v.State, _, _ = s.workStatusCode(*w)
	v.Label = workStatusLabel(v.State)
	memo := map[string]bool{}
	digests := map[string]string{}
	for i := range w.Tasks {
		t := &w.Tasks[i]
		x := TaskValidation{State: t.Status, Fresh: s.acceptedFreshMemo(w, t, map[string]bool{}, memo), Blockers: []string{}, Owner: t.Owner}
		if x.Owner == "" {
			x.Owner = "Responsable à désigner"
		}
		if t.Status == "accepted" || t.Status == "waived" {
			v.Historical++
			if x.Fresh && t.Status == "accepted" {
				v.Validated++
			}
			if !x.Fresh {
				x.State = "stale"
				v.Stale++
			}
		}
		if t.Gate != nil && !s.validGate(t) {
			keys := []string{}
			for p := range t.Gate.Evaluation.Artifacts {
				keys = append(keys, p)
			}
			sort.Strings(keys)
			for _, p := range keys {
				f, e := localFile(s.root, p)
				if e != nil {
					x.Blockers = append(x.Blockers, "Preuve absente ou inaccessible : "+p)
					continue
				}
				digest, known := digests[f]
				if !known {
					b, err := os.ReadFile(f)
					if err == nil {
						digest = hash(b)
					}
					digests[f] = digest
				}
				if digest != t.Gate.Evaluation.Artifacts[p] {
					x.Blockers = append(x.Blockers, "Preuve modifiée : "+p)
				}
			}
			if len(x.Blockers) == 0 {
				x.Blockers = append(x.Blockers, "Gate absente, refusée ou contrôles obligatoires non conformes")
			}
		} else if t.Status == "accepted" && t.Gate == nil {
			x.Blockers = append(x.Blockers, "Gate manquante")
		}
		for _, id := range t.Depends {
			d, _ := w.task(id)
			if !s.acceptedFreshMemo(w, d, map[string]bool{}, memo) {
				x.Blockers = append(x.Blockers, "Dépendance non validée actuellement : "+id)
			}
		}
		if len(x.Blockers) > 0 {
			x.Next = "Rouvrir la tâche si elle est acceptée ; exécuter les contrôles affectés, soumettre un nouveau rapport et ses preuves, enregistrer la gate puis accepter après revue. Traiter les dépendances en premier."
		}
		x.Evidence = s.taskEvidence(w, t, x.Fresh)
		v.Tasks[t.ID] = x
	}
	return v
}
func validationDetails(v TaskValidation) string {
	return fmt.Sprintf("Responsable : %s\n%s\nProchaine action : %s", v.Owner, strings.Join(v.Blockers, "\n"), v.Next)
}
