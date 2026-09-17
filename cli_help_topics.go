package main

import (
	"fmt"
	"sort"
	"strings"
)

var cliHelpTopics = map[string]string{
	"pilotage": `PILOTER LE TRAVAIL
À quoi ça sert : repérer les tâches en cours et les décisions attendues.
Comment faire : swarm console TRAVAIL ; flèches pour choisir, Entrée pour agir.
swarm mission status TRAVAIL affiche les raisons d’attente et les prochaines actions.
Si vous hésitez : une tâche terminée n’est pas forcément validée. Ouvrez son rapport.
Le graphe avec flèches, repli et orientation reste disponible dans le web.`,
	"mission": `POURSUIVRE LA MISSION
À quoi ça sert : autoriser l’enchaînement des tâches prêtes.
Comment faire : swarm mission start TRAVAIL --input profil.json, puis
swarm mission watch TRAVAIL pour maintenir le conducteur actif sans serveur web.
Le profil décrit provider, workspace et role. Un profil existant peut être réutilisé.
Pause/reprise : swarm mission pause TRAVAIL ; swarm mission resume TRAVAIL.
Si vous hésitez : la pause suspend les départs, pas les agents déjà lancés.
Une dépendance non validée ou un dossier occupé retient la suite.`,
	"taches": `COMPRENDRE ET VALIDER UNE TÂCHE
À quoi ça sert : produire un livrable puis vérifier qu’il répond aux critères.
Comment faire : dans les actions, l lit le rapport, s le soumet, g examine les
contrôles, v affiche les preuves et a accepte après revue.
Si vous hésitez : accepter exige des preuves actuelles. Une dérogation motivée
ne transforme pas un contrôle en succès. Ne relancez pas pour contourner un refus.`,
	"agents": `SUIVRE UN AGENT
À quoi ça sert : examiner une tentative précise et son activité reçue.
Comment faire : swarm agent list TRAVAIL ; swarm agent show AGENT.
Dans la console, ouvrez les détails et les journaux de la tentative choisie.
Si vous hésitez : processus vivant, activité récente et résultat validé sont
trois informations distinctes. Un coût non rapporté n’est pas un coût nul.
Fermer la console ne coupe pas les agents. Arrêter demande une confirmation du moteur.`,
	"journaux": `LIRE LES JOURNAUX
À quoi ça sert : retrouver les événements effectivement reçus d’une tentative.
Comment faire : j dans les actions ; swarm agent logs AGENT [après_seq].
Si vous hésitez : l’absence de nouvelle sortie ne prouve pas un blocage.
Comparez le dernier signal avec l’état du processus. La capture détaillée est facultative.`,
	"decisions": `TRAITER UNE DÉCISION
À quoi ça sert : comprendre ce qui nécessite une intervention.
Comment faire : ouvrez les décisions dans les actions, puis la tâche concernée.
Lisez les preuves et motivez votre choix.
Si vous hésitez : acquitter signifie avoir vu le signal. Cela ne valide pas
la tâche et ne demande pas l’arrêt de l’agent.`,
	"budget": `COMPRENDRE LE BUDGET
À quoi ça sert : suivre les dépenses rapportées et les estimations de départ.
Comment faire : b dans les actions ; swarm prepare budget PRÉPARATION pour la préparation.
Si vous hésitez : une estimation n’est pas une facture ; un coût absent reste inconnu.
Examinez le motif du refus avant de relever une limite.`,
	"preparation": `DU BESOIN AU PLAN
À quoi ça sert : préciser le résultat attendu avant de lancer des agents.
Comment faire : swarm prepare chat [PRÉPARATION] ; /aide liste les commandes.
Décrivez le besoin, examinez le brief, puis /adopter. Préparez le plan, répondez
aux décisions et /verifier avant /creer-missions et /autoriser-missions.
Si vous hésitez : enregistrer ou vérifier ne lance pas les agents.
Les documents et leurs versions sont partagés avec le web dans la même racine.`,
	"contexte": `CHOISIR CE QUI EST ENVOYÉ À L’IA
À quoi ça sert : fournir les documents utiles sans dépasser la limite de contexte.
Comment faire : dans prepare chat, /sources montre les extraits ; /retirer enlève
un extrait ; /contexte recent limite la conversation au dernier échange.
Si vous hésitez : réduisez les documents ou repartez du brief. Aucun envoi tronqué.
/aide détaille la syntaxe des commandes de préparation.`,
	"parite": `CLI ET WEB : CE QUI EST PARTAGÉ
Même racine --root : mêmes tâches, tentatives, documents, preuves et décisions.
Le moteur contrôle les dépendances, la fraîcheur des preuves et les espaces occupés.
Le CLI propose préparation, dialogue, missions, lancement, suivi, arrêt et validation.
Pour la mission continue sans serveur web, garder mission watch TRAVAIL actif.
Écarts : graphe interactif et repli, assistant IA contextuel avec propositions
d’actions, recherche des échanges et parcours RETEX sont accessibles dans le web.
Le terminal propose ses propres dialogues et raccourcis : la parité UX n’est pas totale.`,
}

func cliTopicHelp(topic string) (string, error) {
	topic = strings.ToLower(strings.TrimSpace(topic))
	if topic == "" {
		keys := make([]string, 0, len(cliHelpTopics))
		for k := range cliHelpTopics {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		return "AIDE SWARM — choisissez un sujet\n\nswarm aide SUJET\nSujets : " + strings.Join(keys, ", ") + "\n\nConsole : help SUJET ; F1 ouvre l’aide de la fenêtre courante.\nPréparation : /aide SUJET ; /aide affiche les commandes.\nL’aide ne lance aucune action.\n", nil
	}
	text, ok := cliHelpTopics[topic]
	if !ok {
		return "", fmt.Errorf("sujet d’aide inconnu : %s ; swarm aide liste les sujets", topic)
	}
	return text + "\n", nil
}

func terminalHelpTopic(mode string) string {
	switch mode {
	case "budget":
		return "budget"
	case "logs":
		return "journaux"
	case "decisions", "decision":
		return "decisions"
	case "start", "retry", "stop", "reconcile", "agent", "detail":
		return "agents"
	case "report", "submit", "gate", "gate-load", "accepted", "override", "proofs", "reopen", "actions":
		return "taches"
	default:
		return "pilotage"
	}
}
