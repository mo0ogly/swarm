//go:build linux

package main

const terminalGuide = `AIDE PAR SUJET
help mission | help taches | help agents | help journaux | help decisions
help budget | help preparation | help contexte | help parite

SE REPÉRER
Flèches : choisir une tâche. Entrée : ouvrir ses actions.
Tab : passer des tâches aux agents et au suivi. d : détails.
t : couleurs claires/sombres. F1 ou ? : aide. q : quitter.
Les couleurs complètent les libellés : bleu = activité, vert = validation,
ambre = attente ou revue, rouge = échec. NO_COLOR garde une vue sans couleur.

LANCER UNE MISSION
1. Sélectionner la tâche et appuyer sur Entrée.
2. Choisir Lancer, puis fournisseur, espace et consigne avec Tab.
3. Lire le périmètre. Entrée sur Confirmer lance le fournisseur.
Échap annule le dialogue ; il n'arrête jamais un agent déjà lancé.

TERMINÉ N'EST PAS ACCEPTÉ
Terminé : le processus s'est arrêté. Son résultat reste à examiner.
l : lire le rapport ; s : soumettre ; g : examiner et charger la gate.
v : consulter les preuves ; a : accepter après revue.
Une gate absente ou périmée bloque l'acceptation : vérifier à nouveau.
f : dérogation motivée ; elle ne transforme pas les contrôles en PASS.

COMPRENDRE UNE ATTENTE
d : détail, dernière activité, motif et prochaine action.
j dans les actions : rechercher les journaux de la tentative sélectionnée.
Arrêter demande l'arrêt ; attendre la confirmation avant Relancer.
Une erreur de lancement reste visible : corriger sa cause, pas répéter à vide.
e : consigner une OODA après une preuve nouvelle ou un blocage.

MÉMOIRE ET RETEX
La recherche des échanges, l'aperçu du contexte et les fiches RETEX sont
accessibles dans le cockpit web : ../wattson.sh swarm web depuis flaskProject.
Les tâches APEX créées depuis un RETEX sont pilotables ici.

BUDGET ET REPRISE
b dans les actions : budget et consommation. Une estimation n'est pas une facture.
u : retrouver les changements depuis la dernière visite.
Fermer la console ne coupe pas les agents : rouvrir conserve le travail.
Après remplacement du binaire : q puis relancer la console.

COMMANDES AVANCÉES
` + consoleHelp

func (s *Store) openTerminalHelp(work string, c *consoleState) {
	if c.dialog != nil && c.dialog.mode == "help" {
		c.dialog = c.helpParent
		c.helpParent = nil
		return
	}
	c.helpParent = c.dialog
	mode := ""
	if c.dialog != nil {
		mode = c.dialog.mode
	}
	context, _ := cliTopicHelp(terminalHelpTopic(mode))
	d := &taskDialog{mode: "help", review: context + "\n" + terminalGuide, task: Task{Title: "Guide du cockpit terminal"}}
	if c.dialog != nil {
		d.task = c.dialog.task
		d.agent = c.dialog.agent
	} else if w, e := s.get(work); e == nil {
		d.task.Title = w.Title
	}
	c.dialog = d
}
