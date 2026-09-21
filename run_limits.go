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
func executionDirectives(root, workspace, task string, l RunLimits) string {
	return fmt.Sprintf(`
CADRE D'EXÉCUTION — tâche bornée
Seule racine de travail de cette tentative : %s
Toutes les lectures du code, recherches, modifications, commandes Git, tests et rapports se font dans cette racine. Le dépôt hôte, les copies voisines et les chemins historiques cités dans les rapports ne sont pas votre espace de travail. Ne pas y faire cd ni les modifier. Vérifier pwd et git rev-parse --show-toplevel avant de commencer ; résoudre les chemins de code depuis VOTRE copie.
Ne pas écrire directement dans .swarm/state.db. Les commandes de coordination éventuellement fournies désignent le moteur, pas une autre copie de code.
Lire uniquement les fichiers nécessaires. Rechercher avec rg dans le périmètre ; pas de find / ni de balayage global du poste. Les dépendances système connues peuvent être lues par chemin précis.
Vérifier le répertoire courant avant une commande ; préférer les chemins absolus. Fixer un délai explicite pour chaque outil et commande, au plus %d secondes ; découper les opérations longues.
Les échecs identiques entrecoupés d’autres outils sont également comptés dans une fenêtre bornée. Ne pas répéter une commande identique après échec sans nouvelle preuve. Après deux corrections infructueuses, faire une OODA et produire un handoff bloqué. Une erreur de montage ou permission doit être signalée, pas contournée par une recherche plus large.
Arrêter dès que le livrable et les critères sont vérifiés. Sinon, arrêter sur blocage persistant, budget atteint ou preuve inaccessible. Ne pas poursuivre les tâches suivantes automatiquement.
Budget superviseur : %d appels d'outils, %d appels identiques consécutifs, %d erreurs d'outils consécutives, %d secondes sans sortie fournisseur, %d secondes par outil observable. Ces limites interrompent la tentative et ne valent jamais validation.
Créer dès le début docs/%s.md avec les critères encore non vérifiés, puis le mettre à jour après chaque résultat utile et au plus tous les dix appels. Réserver le dernier cinquième du budget aux vérifications et à la remise ; à cette borne, arrêter l’exploration et consigner aussi les critères non testés. Le rapport contient observations, commandes, résultats, limites et prochaine action. Ce fichier est relayé automatiquement pour évaluation s'il est le seul rapport écrit par cette tentative ; son absence, un fichier vide ou plusieurs rapports concurrents laissent la tâche bloquée. Le relais n'est ni une gate ni une acceptation. Le superviseur peut interrompre avant le dernier message.
Ne pas déclarer un diagnostic certain sans reproduction ; comparer les builds avec les mêmes options. Ne pas fabriquer de preuve ni de mesure. La capture des logs n'est pas une autorisation d'exposer des secrets.
`, workspace, l.ToolSeconds, l.MaxToolCalls, l.MaxRepeatedCalls, l.MaxConsecutiveErrors, l.SilenceSeconds, l.ToolSeconds, task)
}

// A mission may tighten provider limits, never silently relax them. Zero inherits.
func (l RunLimits) tightened(request RunLimits) (RunLimits, error) {
	base, err := l.normalized()
	if err != nil {
		return base, err
	}
	if _, err = request.normalized(); err != nil {
		return base, err
	}
	pairs := []struct {
		dst   *int
		value int
	}{
		{&base.SilenceSeconds, request.SilenceSeconds}, {&base.ToolSeconds, request.ToolSeconds},
		{&base.MaxToolCalls, request.MaxToolCalls}, {&base.MaxRepeatedCalls, request.MaxRepeatedCalls},
		{&base.MaxConsecutiveErrors, request.MaxConsecutiveErrors},
	}
	for _, p := range pairs {
		if p.value > *p.dst {
			return base, fmt.Errorf("limite de mission %d supérieure à la limite fournisseur %d", p.value, *p.dst)
		}
		if p.value > 0 {
			*p.dst = p.value
		}
	}
	return base, nil
}

// Zero in older stored attempts supplies no additional ceiling.
func (l RunLimits) cappedBy(previous RunLimits) RunLimits {
	pairs := []struct {
		dst     *int
		ceiling int
	}{
		{&l.SilenceSeconds, previous.SilenceSeconds}, {&l.ToolSeconds, previous.ToolSeconds},
		{&l.MaxToolCalls, previous.MaxToolCalls}, {&l.MaxRepeatedCalls, previous.MaxRepeatedCalls},
		{&l.MaxConsecutiveErrors, previous.MaxConsecutiveErrors},
	}
	for _, p := range pairs {
		if p.ceiling > 0 && p.ceiling < *p.dst {
			*p.dst = p.ceiling
		}
	}
	return l
}
