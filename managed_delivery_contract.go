//go:build linux

package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Producer declarations are a completeness check, never an acceptance verdict.
// They are read from the immutable result/candidate and checked before a paid review.
type ManagedDelivery struct {
	Version  int                 `json:"version"`
	Task     string              `json:"task"`
	Attempt  string              `json:"attempt"`
	Contract string              `json:"contract"`
	Outcome  string              `json:"outcome"`
	Criteria []DeliveryCriterion `json:"criteria"`
}

type DeliveryCriterion struct {
	Index    int      `json:"index"`
	Status   string   `json:"status"`
	Reason   string   `json:"reason"`
	Controls []string `json:"controls"`
	Evidence []string `json:"evidence"`
}

func managedDeliveryInstructions(t *Task, attempt string) string {
	manifest := ManagedDelivery{Version: 1, Task: t.ID, Attempt: attempt, Contract: reviewContract(t), Outcome: "partial"}
	for i := range t.Criteria {
		ids := []string{}
		if t.ValidationPolicy != nil {
			for _, c := range t.ValidationPolicy.Controls {
				for _, index := range c.Criteria {
					if index == i+1 {
						ids = append(ids, c.ID)
						break
					}
				}
			}
		}
		manifest.Criteria = append(manifest.Criteria, DeliveryCriterion{Index: i + 1, Status: "not_tested", Reason: "À compléter avec le résultat observé", Controls: ids, Evidence: []string{}})
	}
	data, _ := json.MarshalIndent(manifest, "", "  ")
	return "\nBILAN DE LIVRAISON REQUIS : écrire docs/" + t.ID + ".delivery.json dans cette copie, en plus du rapport Markdown. Le moteur contrôle ce bilan avant la revue payante. Garder task/attempt/contract et un élément par critère. outcome=complete seulement si TOUS les critères sont démontrés ; sinon partial ou blocked. status=pass, fail, not_tested ou not_applicable ; expliquer chaque résultat dans reason. controls référence uniquement les identifiants de contrôles autorisés pour ce critère, jamais une commande libre ; evidence référence des fichiers suivis de cette copie (chemins relatifs à la racine Git). Le moteur exécutera ces contrôles : votre déclaration ne prouve pas leur succès. Une preuve absente ou une non-applicabilité conserve un résultat incomplet à examiner. Ne pas réduire un critère au test le plus facile : traiter toutes ses obligations, y compris les interactions navigateur demandées. Ne pas inventer de preuve ni transformer un non-testé en pass. Ce bilan n’est pas une acceptation.\n" + string(data) + "\n"
}

func deliveryIncomplete(reason string) error {
	return &CommandError{Code: "delivery_incomplete", Message: "Livraison incomplète : " + reason + ". Aucun appel de revue lancé ; le responsable doit examiner les éléments manquants avant une reprise autorisée."}
}

func (s *Store) managedDelivery(w Work, t *Task, a Agent, candidate string) (*ManagedDelivery, error) {
	repo := w.Planning.Repository
	bare := filepath.Join(repo.Storage, "repository.git")
	path := filepath.ToSlash(filepath.Join(repo.Subdir, "docs", t.ID+".delivery.json"))
	source, e := candidateReviewSource(bare, candidate, path, 32*1024)
	if errors.Is(e, os.ErrNotExist) {
		if a.DeliveryVersion > 0 {
			return nil, deliveryIncomplete("bilan par critère absent (" + path + ")")
		}
		return nil, nil // A new host must not retroactively change old attempts.
	}
	if e != nil {
		return nil, deliveryIncomplete("bilan illisible : " + e.Error())
	}
	var d ManagedDelivery
	if e = strict([]byte(source.Content), &d); e != nil {
		return nil, deliveryIncomplete("bilan de livraison invalide : " + e.Error())
	}
	if d.Version != 1 || d.Task != t.ID || d.Attempt != a.Attempt || d.Contract != reviewContract(t) {
		return nil, deliveryIncomplete("bilan attribué à une autre tâche, tentative ou consigne")
	}
	if d.Outcome != "complete" && d.Outcome != "partial" && d.Outcome != "blocked" {
		return nil, deliveryIncomplete("état de livraison inconnu")
	}
	if len(d.Criteria) != len(t.Criteria) || t.ValidationPolicy == nil {
		return nil, deliveryIncomplete("critères ou contrôles manquants")
	}
	seen := map[int]bool{}
	for _, c := range d.Criteria {
		if c.Index < 1 || c.Index > len(t.Criteria) || seen[c.Index] {
			return nil, deliveryIncomplete("critère inconnu ou dupliqué")
		}
		seen[c.Index] = true
		if c.Status != "pass" || strings.TrimSpace(c.Reason) == "" {
			return nil, deliveryIncomplete(fmt.Sprintf("critère %d non démontré (%s) : %s", c.Index, c.Status, guardBlock(c.Reason, 300)))
		}
		if len(c.Controls) == 0 || len(c.Evidence) == 0 || len(c.Evidence) > 24 {
			return nil, deliveryIncomplete(fmt.Sprintf("contrôles ou fichiers de preuve manquants pour le critère %d", c.Index))
		}
		for _, id := range c.Controls {
			valid := false
			for _, check := range t.ValidationPolicy.Controls {
				if check.ID == id {
					for _, index := range check.Criteria {
						if index == c.Index {
							valid = true
						}
					}
				}
			}
			if !valid {
				return nil, deliveryIncomplete(fmt.Sprintf("contrôle %s non autorisé pour le critère %d", id, c.Index))
			}
		}
		for _, path := range c.Evidence {
			if path == "" || filepath.IsAbs(path) || filepath.ToSlash(filepath.Clean(path)) != path || strings.HasPrefix(path, "../") || path == "." || strings.ContainsAny(path, "\x00\n\r\\:") || strings.HasPrefix(path, ".git/") || path == ".git" {
				return nil, deliveryIncomplete("chemin de preuve invalide")
			}
			entry, e := managedGit(bare, "ls-tree", "-z", candidate, "--", ":(literal)"+path)
			if e != nil {
				return nil, e
			}
			parts := strings.SplitN(strings.TrimSuffix(entry, "\x00"), "\t", 2)
			if len(parts) != 2 || parts[1] != path || (!strings.HasPrefix(parts[0], "100644 blob ") && !strings.HasPrefix(parts[0], "100755 blob ")) {
				return nil, deliveryIncomplete("fichier de preuve absent ou non régulier : " + path)
			}
		}
	}
	if d.Outcome != "complete" {
		return nil, deliveryIncomplete("le producteur déclare un résultat partiel ou bloqué")
	}
	return &d, nil
}
