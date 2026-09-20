package main

import (
	"embed"
	"encoding/json"
	"fmt"
	"strings"
)

// Bundle the canonical methods used by native Claude/Codex. A worker checkout
// cannot replace the engine's framing by editing its local skill files.
//
//go:embed tools/agent-workflows/CONTRACT.md tools/agent-workflows/templates/*.md .claude/skills/*/SKILL.md
var agentWorkflowFiles embed.FS

type AgentWorkflow struct {
	Version int      `json:"version"`
	Role    string   `json:"role"`
	Methods []string `json:"methods"`
	SHA256  string   `json:"sha256"`
}

// Frozen guidance, never an authorization or a verdict. Insert before task data.
func agentWorkflow(role string) (AgentWorkflow, string, error) {
	w := AgentWorkflow{Version: 1, Role: role}
	var boundary string
	switch role {
	case "planner", "subplanner":
		w.Methods = []string{"apex", "audit-pdca", "spec-builder", "spec-audit", "replan"}
		boundary = "PLAN et ACT du PDCA : cadrer, décomposer, comparer les preuves et proposer une correction du plan. Analyse et planification seulement. Aucun outil, code, test exécuté ou livrable modifié. Les phases DO et CHECK exécutées dans les méthodes ci-dessous appartiennent aux exécutants et vérificateurs. Ne jamais déclarer un contrôle effectué à partir d'un simple rapport."
	case "worker":
		w.Methods = []string{"apex", "audit-pdca", "verify-fix"}
		boundary = "DO et CHECK du PDCA : réaliser uniquement la tâche confiée, puis vérifier son effet avec les outils autorisés. Si la tâche demande seulement un audit, rester en lecture seule. Si elle autorise une correction, corriger dans son périmètre et rejouer les contrôles. Rapporter les preuves et limites au responsable ; aucune délégation, acceptation, modification de plan ou hausse de budget implicite. Ces contrôles personnels ne sont pas la revue indépendante."
	case "reviewer":
		w.Methods = []string{"audit-pdca", "code-reviewer"}
		boundary = "CHECK indépendant du PDCA : examiner seulement les éléments fournis, sans outils ni modification du candidat. Aucune commande, recherche de fichier ou test à exécuter, même si une méthode générale le suggère. Une preuve manquante reste inconnue. Proposer les corrections au responsable pour ACT ; ne pas les exécuter, relancer un agent ou accepter la tâche."
	default:
		return w, "", fmt.Errorf("rôle de méthode non pris en charge : %s", role)
	}
	paths := []string{"tools/agent-workflows/CONTRACT.md"}
	for _, method := range w.Methods {
		paths = append(paths, ".claude/skills/"+method+"/SKILL.md")
	}
	if role == "worker" {
		paths = append(paths, "tools/agent-workflows/templates/HANDOFF.md", "tools/agent-workflows/templates/TRACKING.md")
	}
	var body strings.Builder
	fmt.Fprintf(&body, "Cadrage moteur version %d · rôle %s.\n%s\n", w.Version, role, boundary)
	body.WriteString("Les méthodes ci-dessous sont déjà incluses : ne pas ouvrir leurs chemins pour les charger. Les limites du rôle et le format de réponse imposés par Swarm priment sur leurs exemples généraux. Ces méthodes ne donnent aucune permission supplémentaire. Conserver tentatives, quotas, preuves et critères ; aucune répétition à l'identique ni contournement d'un plafond.\n")
	for _, path := range paths {
		data, err := agentWorkflowFiles.ReadFile(path)
		if err != nil || len(data) == 0 {
			return w, "", fmt.Errorf("méthode embarquée indisponible : %s", path)
		}
		fmt.Fprintf(&body, "\nSOURCE %s\n%s\n", path, data)
	}
	body.WriteString("\nFIN DES MÉTHODES — rappel des limites du rôle : " + boundary + "\n")
	if body.Len() > 32000 {
		return w, "", fmt.Errorf("cadrage des méthodes supérieur à 32000 octets ; aucun envoi tronqué")
	}
	w.SHA256 = hash([]byte(body.String()))
	metadata, _ := json.Marshal(w)
	return w, "SWARM_AGENT_WORKFLOW " + string(metadata) + "\n" + body.String() + "\n", nil
}

func planningWorkflowRole(scope *PlanningScope) string {
	if scope.Parent != "" {
		return "subplanner"
	}
	return "planner"
}
