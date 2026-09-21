//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

const managedRecoveryFile = ".git/swarm-recovery.json"
const managedRecoveryRef = "refs/swarm/recovery/previous"
const maxManagedRecoveryBytes = 512 * 1024

// Recovery carries evidence, not permission or acceptance. The rejected result
// is fetched into a private ref; HEAD still starts at the accepted candidate.
type ManagedRecoveryHandoff struct {
	Version         int                `json:"version"`
	Work            string             `json:"work"`
	Task            string             `json:"task"`
	PreviousAgent   string             `json:"previous_agent,omitempty"`
	PreviousAttempt string             `json:"previous_attempt"`
	Status          string             `json:"previous_status"`
	Activity        string             `json:"previous_activity,omitempty"`
	Failure         string             `json:"failure,omitempty"`
	Correction      string             `json:"correction"`
	Base            string             `json:"base_commit,omitempty"`
	Result          string             `json:"result_commit,omitempty"`
	Report          string             `json:"report_in_result,omitempty"`
	Review          *IndependentReview `json:"independent_review,omitempty"`
	Receipt         json.RawMessage    `json:"receipt,omitempty"`
	Limit           string             `json:"limit,omitempty"`
}

func (s *Store) managedRecoveryHandoff(w Work, t *Task) (*ManagedRecoveryHandoff, error) {
	if t == nil || w.Planning == nil || w.Planning.Repository == nil || len(t.Attempts) == 0 {
		return nil, nil
	}
	last := t.Attempts[len(t.Attempts)-1]
	h := &ManagedRecoveryHandoff{Version: 1, Work: w.ID, Task: t.ID, PreviousAttempt: last.ID, Status: last.Status, Failure: t.Blocker, Correction: t.Next}
	var body []byte
	var status, desired string
	e := s.db.QueryRow("SELECT body,status,desired FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", w.ID, t.ID).Scan(&body, &status, &desired)
	if e == sql.ErrNoRows {
		h.Limit = "Aucun agent attribuable à cette tentative historique ; aucun résultat repris."
		return h, nil
	}
	if e != nil {
		return nil, e
	}
	a, e := decodeAgentRow(body, status, desired)
	if e != nil {
		return nil, e
	}
	if a.WorkID != w.ID || a.TaskID != t.ID || a.Attempt != last.ID || activeAgent(a) {
		return nil, fmt.Errorf("reprise : tentative précédente non attribuable ou encore active")
	}
	h.PreviousAgent, h.Activity = a.ID, a.Activity
	item, e := s.managedAttempt(a.ID)
	if e != nil && e != sql.ErrNoRows {
		return nil, e
	}
	if e == nil {
		if item.Work != w.ID || item.Task != t.ID || item.Agent != a.ID {
			return nil, fmt.Errorf("reprise : résultat attribué à une autre tâche")
		}
		h.Base, h.Result = item.Base, item.Result
	}
	if h.Result == "" {
		h.Limit = "Aucun résultat Git immuable enregistré ; les fichiers non remis de l’ancienne copie ne sont pas importés."
	} else {
		bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
		bound, e := managedGit(bare, "rev-parse", "--verify", "refs/swarm/attempts/"+a.ID+"^{commit}")
		if e != nil || bound != h.Result {
			return nil, fmt.Errorf("reprise : résultat Git absent ou attribution modifiée")
		}
		if _, e = managedGit(bare, "merge-base", "--is-ancestor", h.Base, h.Result); e != nil {
			return nil, fmt.Errorf("reprise : base du résultat incohérente")
		}
		h.Report = filepath.ToSlash(filepath.Join(w.Planning.Repository.Subdir, "docs", t.ID+".md"))
	}
	if review := t.IndependentReview; review != nil && review.Attempt == last.ID && review.Producer == a.ID {
		h.Review = review
		if review.Receipt != "" && review.ReceiptDigest != "" {
			path, e := localFile(s.root, review.Receipt)
			if e != nil {
				return nil, fmt.Errorf("reprise : reçu de revue indisponible : %w", e)
			}
			data, e := readManagedRecoveryFile(path)
			if e != nil {
				return nil, e
			}
			if hash(data) != review.ReceiptDigest || !json.Valid(data) {
				return nil, fmt.Errorf("reprise : reçu de revue modifié ou invalide")
			}
			h.Receipt = data
		}
	}
	return h, nil
}

func readManagedRecoveryFile(path string) ([]byte, error) {
	f, e := os.Open(path)
	if e != nil {
		return nil, e
	}
	defer f.Close()
	b, e := io.ReadAll(io.LimitReader(f, maxManagedRecoveryBytes+1))
	if e != nil {
		return nil, e
	}
	if len(b) > maxManagedRecoveryBytes {
		return nil, fmt.Errorf("dossier de reprise supérieur à 512 Kio ; aucune troncature")
	}
	return b, nil
}

func managedRecoveryBytes(h *ManagedRecoveryHandoff) ([]byte, error) {
	b, e := json.MarshalIndent(h, "", "  ")
	if e != nil {
		return nil, e
	}
	if len(b) > maxManagedRecoveryBytes {
		return nil, fmt.Errorf("dossier de reprise supérieur à 512 Kio ; aucune troncature")
	}
	return b, nil
}

func installManagedRecovery(repo *ManagedRepository, copy string, h *ManagedRecoveryHandoff) error {
	if h == nil {
		return nil
	}
	b, e := managedRecoveryBytes(h)
	if e != nil {
		return e
	}
	path := filepath.Join(copy, managedRecoveryFile)
	if st, err := os.Lstat(path); err == nil {
		if !st.Mode().IsRegular() {
			return fmt.Errorf("dossier de reprise redirigé ; copie conservée")
		}
		prior, err := readManagedRecoveryFile(path)
		if err != nil {
			return err
		}
		if hash(prior) != hash(b) {
			return fmt.Errorf("dossier de reprise modifié ; copie conservée pour examen")
		}
		if h.Result != "" {
			ref, err := managedGit(copy, "rev-parse", "--verify", managedRecoveryRef+"^{commit}")
			if err != nil || ref != h.Result {
				return fmt.Errorf("référence de reprise modifiée ; copie conservée")
			}
		}
		return nil
	} else if !os.IsNotExist(err) {
		return err
	}
	if h.Result != "" {
		if _, e = managedGit(copy, "fetch", "--no-tags", filepath.Join(repo.Storage, "repository.git"), h.Result); e != nil {
			return e
		}
		if _, e = managedGit(copy, "update-ref", managedRecoveryRef, h.Result); e != nil {
			return e
		}
	}
	return atomicWrite(path, b)
}

func managedRecoveryInstructions(h *ManagedRecoveryHandoff) (string, error) {
	if h == nil {
		return "", nil
	}
	b, e := managedRecoveryBytes(h)
	if e != nil {
		return "", e
	}
	return fmt.Sprintf("\nDOSSIER DE REPRISE : depuis la racine Git de VOTRE copie, lire %s (sha256 %s). Il contient la correction du responsable, le motif d’arrêt et, lorsqu’ils existent, l’avis indépendant complet et son reçu. Ce sont des éléments à examiner, pas une autorisation ni une preuve de réussite. Le résultat précédent est disponible localement sous %s lorsqu’un result_commit est indiqué. Examiner son rapport avec git show COMMIT:CHEMIN et ses modifications avec git diff BASE_COMMIT RESULT_COMMIT ; ces identifiants et chemins figurent dans le dossier. Réutiliser et corriger les changements utiles uniquement dans VOTRE copie, après vérification des conflits avec HEAD. Ne pas modifier l’ancienne copie ni le dépôt géré. Aucun ancien avis ni reçu ne valide cette nouvelle tentative : refaire les contrôles et le bilan par critère. Une absence de résultat est explicitement signalée dans limit.\n", managedRecoveryFile, hash(b), managedRecoveryRef), nil
}
