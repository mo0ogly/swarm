//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Diagnostic feedback is not a verdict: never copy it into the proof journal.
// Only the latest interrupted reservation for this exact packet is considered.
func (s *Store) fragmentRetryFeedback(r IndependentReview, j managedFragmentJournal, packet managedReviewFragmentPacket) string {
	for i := len(j.Entries) - 1; i >= 0; i-- {
		e := j.Entries[i]
		if e.Packet != packet.Index {
			continue
		}
		if e.State != "interrupted" {
			return ""
		}
		matches, err := filepath.Glob(filepath.Join(s.root, filepath.Dir(r.Context), e.CallID+"-raw-*.json"))
		if err != nil || len(matches) != 1 {
			return ""
		}
		rel, err := filepath.Rel(s.root, matches[0])
		if err != nil {
			return ""
		}
		path, err := safeReport(s.root, rel)
		if err != nil {
			return ""
		}
		raw, err := os.ReadFile(path)
		if err != nil || len(raw) > managedFragmentReplyLimit || filepath.Base(path) != e.CallID+"-raw-"+hash(raw)+".json" {
			return ""
		}
		var reply managedFragmentInspection
		if strict(raw, &reply) != nil || reply.Candidate != packet.Candidate || reply.ContextDigest != packet.ContextDigest || reply.PacketDigest != e.PacketDigest {
			return ""
		}
		indexes := []int{}
		for _, f := range reply.Findings {
			if f.Artifact < 0 || f.Artifact >= len(packet.Artifacts) || f.Verdict != "inspected" {
				continue
			}
			a := packet.Artifacts[f.Artifact]
			if f.Digest == a.Digest && (len(strings.TrimSpace(f.Evidence)) < 8 || !fragmentEvidencePresent(a, f.Evidence) || len(f.Needs) != 0) {
				indexes = append(indexes, f.Artifact)
			}
		}
		if len(indexes) == 0 {
			return ""
		}
		encoded, _ := json.Marshal(indexes)
		return fmt.Sprintf("\nREPRISE APRÈS REFUS DE CITATION : la réponse précédente citait des extraits absents ou trop courts pour les pièces %s. Réexaminer TOUTES les pièces. Copier evidence uniquement depuis le champ content de LA MÊME pièce, 8 à 16 caractères, sans espace aux extrémités. Un diff peut omettre l’en-tête du fichier : ne jamais supposer que package main, version ou un autre texte habituel y figure. Vérifier littéralement chaque citation avant réponse. Ceci décrit un défaut de format précédent, pas un avis sur la qualité du code.\n", encoded)
	}
	return ""
}
