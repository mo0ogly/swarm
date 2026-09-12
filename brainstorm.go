package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"
	"unicode/utf8"
)

type PlanningBrief struct {
	Task   string `json:"task"`
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Text   string `json:"text"`
	At     string `json:"at"`
}

func brainstormDirectives(id string) string {
	return fmt.Sprintf(`
PRÉPARATION DES AGENTS — BRAINSTORMING IA AVEC APEX
Tu es le planner. Mène toi-même le brainstorming sur la question de l’opérateur.
Reste dans Analyze et Plan d’APEX ; Execute et eXamine sont à proposer, pas à exécuter.
Si présents, lire .claude/skills/apex/SKILL.md et tools/agent-workflows/CONTRACT.md, en respectant ce périmètre de préparation. Aucune délégation, aucun lancement d’agent, aucun changement de code ni commande mutante Swarm. Ne pas écrire de fichier ; le moteur conserve directement ta réponse finale.
Explorer uniquement les documents/fichiers utiles. Distinguer faits avec références, hypothèses et inconnues. Proposer au moins deux options si pertinentes, expliquer leurs avantages, risques et coût qualitatif sans inventer de prix ou de mesures. Recommander une approche avec justification concise. Formuler les questions nécessitant une décision humaine.
Proposer un plan de missions pour les agents : objectif, rôle, périmètre, livrable, dépendances, critères observables, preuves attendues, gates entry/validation/delivery, limites de ressources et conditions d’arrêt. Identifier ce qui peut être parallèle et ce qui doit rester séquentiel. Prévoir une OODA sur blocage ou nouvelle preuve, au plus deux corrections sans réorientation.
Ne pas attribuer un score arbitraire ni déclarer PASS des tests non exécutés. Le rapport doit être revu avant réalisation ; aucune tâche métier n’est acceptée par ce brainstorming.
Répondre directement en Markdown concis (maximum 16000 octets UTF-8). Ne pas utiliser un outil pour écrire la réponse : le moteur enregistre le texte final. Sections si pertinentes : Question, Faits et hypothèses, Options, Recommandation, Questions ouvertes, Missions des agents, Gates et conditions d’arrêt. Terminer par les décisions attendues. Identifiant de l’échange : %s.
`, id)
}

// Freeze the exact reviewed text; future changes to the source cannot silently
// alter what later agents receive. Adoption is not task acceptance or a gate PASS.
func (s *Store) adoptBrief(work, task, path, expectedHash, event string, revision int) (Work, error) {
	raw, _ := json.Marshal(map[string]string{"task": task, "sha256": expectedHash})
	return s.mutate(work, "brief.adopt", event, revision, raw, func(w *Work) error {
		t, e := w.task(task)
		if e != nil {
			return e
		}
		if !t.Brainstorm {
			return fmt.Errorf("Choisir une réponse de préparation IA.")
		}
		if t.Status == "running" {
			return fmt.Errorf("Attendre la fin de la préparation avant adoption.")
		}
		if !nonempty(t.Response) || hash([]byte(t.Response)) != expectedHash {
			return fmt.Errorf("Réponse modifiée ou absente : relire avant adoption.")
		}
		w.PlanningBrief = &PlanningBrief{Task: task, SHA256: expectedHash, Text: t.Response, At: now()}
		return nil
	})
}

func readBrainstormReport(root, task string) ([]byte, error) {
	p, e := localFile(root, "docs/"+task+"-brainstorm.md")
	if e != nil {
		return nil, e
	}
	f, e := os.Open(p)
	if e != nil {
		return nil, e
	}
	defer f.Close()
	b, e := io.ReadAll(io.LimitReader(f, 16001))
	if e != nil {
		return nil, e
	}
	if len(b) > 16000 || !utf8.Valid(b) || !nonempty(string(b)) {
		return nil, fmt.Errorf("rapport UTF-8 vide ou supérieur à 16000 octets")
	}
	return b, nil
}
func brainstormHistory(w Work, current string) string {
	var turns []Task
	for _, t := range w.Tasks {
		if t.Brainstorm && t.ID != current {
			turns = append(turns, t)
		}
	}
	if len(turns) > 4 {
		turns = turns[len(turns)-4:]
	}
	var b strings.Builder
	b.WriteString("\nDIALOGUE : répondre à la nouvelle question en tenant compte des quatre derniers échanges disponibles et du brief adopté. Ne pas refaire un plan entier si une réponse ciblée suffit. Poser les questions utiles dans le rapport ; l’opérateur répondra dans la console.\n")
	for _, t := range turns {
		fmt.Fprintf(&b, "\nOpérateur : %s\nIA : %s\n", t.Question, t.Response)
	}
	return b.String()
}

// Only public final answer events, never reasoning/tool output.
func providerReply(d map[string]any) string {
	text := ""
	if d["type"] == "result" || d["subtype"] == "success" {
		text, _ = d["result"].(string)
	}
	if d["type"] == "item.completed" {
		if item, ok := d["item"].(map[string]any); ok && item["type"] == "agent_message" {
			text, _ = item["text"].(string)
		}
	}
	if !utf8.ValidString(text) {
		return ""
	}
	if len(text) > 16000 {
		text = text[:16000]
		for !utf8.ValidString(text) {
			text = text[:len(text)-1]
		}
		text += "\n[Réponse tronquée à 16000 octets]"
	}
	return text
}
