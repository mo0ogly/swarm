package main

import "fmt"

// Limits are frozen in each launch; configuration changes affect new attempts only.
type RunLimits struct {
	SilenceSeconds       int `json:"silence_seconds"`
	ToolSeconds          int `json:"tool_seconds"`
	MaxToolCalls         int `json:"max_tool_calls"`
	MaxRepeatedCalls     int `json:"max_repeated_calls"`
	MaxConsecutiveErrors int `json:"max_consecutive_errors"`
}

func (l RunLimits) normalized() (RunLimits, error) {
	fields := []struct {
		p        *int
		def, max int
	}{{&l.SilenceSeconds, 180, 86400}, {&l.ToolSeconds, 300, 86400}, {&l.MaxToolCalls, 100, 10000}, {&l.MaxRepeatedCalls, 4, 100}, {&l.MaxConsecutiveErrors, 3, 100}}
	for _, f := range fields {
		if *f.p == 0 {
			*f.p = f.def
		}
		if *f.p < 1 || *f.p > f.max {
			return l, fmt.Errorf("limite d'exécution hors bornes : 1..%d (0 = défaut)", f.max)
		}
	}
	return l, nil
}
func executionDirectives(root, workspace string, l RunLimits) string {
	return fmt.Sprintf(`
CADRE D'EXÉCUTION — tâche bornée
Racine du projet : %s
Workspace de cette tentative : %s
Utiliser les chemins fournis et la CLI Swarm ; ne pas écrire directement dans .swarm/state.db.
Lire uniquement les fichiers nécessaires. Rechercher avec rg dans le périmètre ; pas de find / ni de balayage global du poste. Les dépendances système connues peuvent être lues par chemin précis.
Vérifier le répertoire courant avant une commande ; préférer les chemins absolus. Fixer un délai explicite pour chaque outil et commande, au plus %d secondes ; découper les opérations longues.
Ne pas répéter une commande identique après échec sans nouvelle preuve. Après deux corrections infructueuses, faire une OODA et produire un handoff bloqué. Une erreur de montage ou permission doit être signalée, pas contournée par une recherche plus large.
Arrêter dès que le livrable et les critères sont vérifiés. Sinon, arrêter sur blocage persistant, budget atteint ou preuve inaccessible. Ne pas poursuivre les tâches suivantes automatiquement.
Budget superviseur : %d appels d'outils, %d appels identiques consécutifs, %d erreurs d'outils consécutives, %d secondes sans sortie fournisseur, %d secondes par outil observable. Ces limites interrompent la tentative et ne valent jamais validation.
Conserver tôt les mesures et un handoff dans un fichier du projet : observations, commandes, résultats, limites, prochaine action. Le superviseur peut interrompre avant le dernier message.
Ne pas déclarer un diagnostic certain sans reproduction ; comparer les builds avec les mêmes options. Ne pas fabriquer de preuve ni de mesure. La capture des logs n'est pas une autorisation d'exposer des secrets.
`, root, workspace, l.ToolSeconds, l.MaxToolCalls, l.MaxRepeatedCalls, l.MaxConsecutiveErrors, l.SilenceSeconds, l.ToolSeconds)
}
