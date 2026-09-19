package main

import (
	"fmt"
	"sort"
	"strings"
)

var cliHelpTopics = map[string]string{
	"planification": `CONFIER UN BESOIN À UNE ÉQUIPE
Sur une mission vide : swarm planning enable TRAVAIL --input activation.json.
L’activation fixe le fournisseur, les limites, le dépôt Git et les contrôles par
exigence. Les tâches créées héritent de ces contrôles autorisés.
Puis swarm mission start TRAVAIL --input profil.json ; garder web ou mission watch actif.
Suivi : swarm planning show TRAVAIL ; swarm planning history TRAVAIL.
Pause/reprise des décisions : planning pause|resume TRAVAIL --input requete.json.
Une reprise ne supprime ni limites, ni échecs, ni contrôles.
Livraison : swarm planning bundle TRAVAIL /chemin/absolu/resultat.bundle.
Le dépôt d’origine est préservé ; swarm-result désigne la révision vérifiée.
Les copies Git séparent les fichiers, sans constituer une sandbox système.
Exemple complet : docs/plans/swarm-architecture-implementation/IMPLEMENTATION.md.`,
	"pilotage": `PILOTER LE TRAVAIL
À quoi ça sert : repérer les tâches en cours et les décisions attendues.
Comment faire : swarm console TRAVAIL ; flèches pour choisir, Entrée pour agir.
swarm mission status TRAVAIL affiche les raisons d’attente et les prochaines actions.
Si vous hésitez : une tâche terminée n’est pas forcément validée. Ouvrez son rapport.
Rôles : planner = planificateur ; subplanner = responsable de branche ;
worker = exécutant. Un rôle absent n’est pas déduit du titre de la tâche.
À compléter : livrable, critères ou consigne absents. En attente : conditions du
moteur non réunies. À résoudre : examiner le blocage et la dernière tentative.
Le graphe web distingue rôle (icône et libellé), état (contour) et prochaine étape.
Le graphe avec flèches, repli et orientation reste disponible dans le web.
La réorganisation graphique avec simulation des changements n’est pas disponible.`,
	"mission": `POURSUIVRE LA MISSION
À quoi ça sert : autoriser l’enchaînement des tâches prêtes.
Le mode autonome exige une organisation hiérarchique et des validations explicites.
Sans elles, start et les réservations automatiques sont refusés ; status explique
les éléments manquants. Les missions historiques restent consultables et manuelles.
Une recette isolée ou un binaire installé ne prouve pas la clôture de votre mission.
Examiner d’abord le verdict moteur : swarm mission preview TRAVAIL --input profil.json.
Puis : swarm mission start TRAVAIL --input profil.json, puis
swarm mission watch TRAVAIL pour maintenir le conducteur actif sans serveur web.
start enregistre l’autorisation mais ne prétend pas qu’un conducteur est présent.
status sépare autorisation, santé du Conducteur Swarm, dernière et prochaine
vérifications, dernière action persistée et agents actifs. Il ne décrit pas une
supervision externe Codex et n’anime aucun compte à rebours dans le terminal.
Après arrêt de watch ou du serveur, l’autorisation reste conservée mais la supervision
devient absente ; une reprise réconcilie les tentatives avant tout nouveau départ.
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
	"validations": `CONFIGURER LES VALIDATIONS
À quoi ça sert : choisir explicitement revue humaine ou contrôles structurés.
Dans le web : Tâches > Configurer les validations. Associez chaque critère à un
contrôle objectif, examinez portée, limites et effet, puis confirmez l’aperçu.
Pour un critère qualitatif (lisibilité, pertinence, qualité), gardez le mode humain.
CLI : swarm validation preview TRAVAIL --task TÂCHE --input politique.json --json
puis recopiez preview_token sans changer le document, ajoutez event_id et lancez :
swarm validation apply TRAVAIL --task TÂCHE --input politique.json --json
Document : schema_version, expected_revision, task_id, intent replace et policy.
Retrait : intent remove, sans policy. Un nouvel aperçu est requis après toute
modification ou révision concurrente. Une pause suspend les contrôles automatiques.
Jamais une suggestion IA, un journal ou un handoff ne vaut autorisation.`,
	"cycle-vie": `ARCHIVER, PURGER OU SUPPRIMER UNE MISSION
À quoi ça sert : ranger une mission, réduire ses anciens journaux ou la placer
dans une corbeille récupérable sans jamais effacer les sources du projet.
Listez d’abord les missions actives, archivées et récupérables : swarm lifecycle list.
Commencez toujours par : swarm lifecycle preview TRAVAIL ACTION --input requete.json.
ACTION vaut archive, restore, purge ou delete. Recopiez preview_token, ajoutez
un event_id puis lancez lifecycle apply avec le même document et la même révision.
archive crée un ZIP exportable ; restore réactive une archive ou une corbeille.
purge exige retention_days et conserve événements, reçus, verdicts et fichiers.
delete retire les seules données SQLite de la mission vers la corbeille interne.
La suppression demande de vérifier le nom de la mission dans le web. En CLI,
l’aperçu affiche ce nom et son effet avant que vous prépariez la commande apply.
Agents, intentions, conducteur, dialogue ou contrôle actifs provoquent un refus.
Un double envoi avec le même event_id rend le même reçu ; un contenu différent
est refusé. Après tout changement, demandez un nouvel aperçu.`,
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
Le web et mission status lisent le même verdict persistant : autorisation,
Conducteur Swarm observé, erreur, vérification relative, dernière action et activité agent.
Écarts : graphe interactif et repli, assistant IA contextuel avec propositions
d’actions, recherche des échanges et parcours RETEX sont accessibles dans le web.
Le terminal propose ses propres dialogues et raccourcis : la parité UX n’est pas totale.`,
}

func cliTopicHelp(topic string) (string, error) {
	topic = strings.ToLower(strings.TrimSpace(topic))
	aliases := map[string]string{"planning": "planification", "management": "pilotage", "tasks": "taches", "validation": "validations", "lifecycle": "cycle-vie", "logs": "journaux", "context": "contexte", "parity": "parite"}
	if target, ok := aliases[topic]; ok {
		topic = target
	}
	if topic == "" {
		keys := make([]string, 0, len(cliHelpTopics))
		for k := range cliHelpTopics {
			keys = append(keys, k)
		}
		sort.Strings(keys)
		return uiText("AIDE SWARM — choisissez un sujet\n\nswarm aide SUJET\nSujets : ") + strings.Join(keys, ", ") + uiText("\n\nConsole : help SUJET ; F1 ouvre l’aide de la fenêtre courante.\nPréparation : /aide SUJET ; /aide affiche les commandes.\nL’aide ne lance aucune action.\n"), nil
	}
	text, ok := cliHelpTopics[topic]
	if !ok {
		return "", fmt.Errorf("sujet d’aide inconnu : %s ; swarm aide liste les sujets", topic)
	}
	return uiText(text) + "\n", nil
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
