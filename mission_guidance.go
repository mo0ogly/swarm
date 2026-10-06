package main

import "sort"

// A read-only next step, shared by the web and CLI. It grants no authority.
type MissionGuidance struct {
	What    string               `json:"what"`
	Next    string               `json:"next"`
	Actor   string               `json:"actor"`
	Primary MissionPrimaryAction `json:"primary"`
}
type MissionPrimaryAction struct {
	Kind   string `json:"kind"`
	Label  string `json:"label"`
	Effect string `json:"effect"`
	Task   string `json:"task,omitempty"`
	Tone   string `json:"tone"`
}

func missionTaskPrimary(t MissionTask) MissionPrimaryAction {
	a := MissionPrimaryAction{Kind: "task", Task: t.ID, Label: t.Label, Tone: "attention", Effect: "Ouvre les preuves et les actions autorisées ; aucune reprise immédiate."}
	switch {
	case t.AttemptLimitReached:
		a.Label = "Examiner les tentatives et les refus"
	case t.State == "review":
		a.Label = "Examiner le résultat"
		if t.ValidationMode == "automatic" {
			a.Label = "Voir les contrôles en cours"
			a.Tone = "info"
		}
	case t.Action == "prepare":
		a.Label = "Autoriser le plan dans Préparer"
	case t.Action == "configure":
		a.Label = "Configurer le lancement"
		a.Tone = "info"
		a.Effect = "Ouvre les réglages ; aucun agent ne part avant confirmation."
	case t.Action == "start":
		a.Effect = "Ouvre la confirmation ; le moteur revérifie les conditions avant le départ."
		a.Tone = "info"
	case t.Action == "inspect" && t.State == "intervention":
		a.Label = "Diagnostiquer et préparer la reprise"
	}
	return a
}

// Planning failures affect new decisions, not actions already represented by
// the dispatcher or result lifecycle. Keep those actions visible in both UIs.
func missionPlanningOwnsNextStep(d MissionStatus) bool {
	if d.Running > 0 || d.ActiveAgents > 0 || d.Review > 0 {
		return false
	}
	for _, task := range d.Tasks {
		switch task.State {
		case "running", "review", "intervention", "configure", "ready", "manual":
			return false
		}
	}
	return true
}

// Closed responsibility retains diagnostics for history, without requiring recovery.
func missionPlanningOpen(w Work) bool {
	if w.Planning == nil {
		return false
	}
	root, _ := w.Planning.scope("root")
	return root == nil || root.State != "closed"
}

func missionGuidance(w Work, d MissionStatus) MissionGuidance {
	g := MissionGuidance{What: d.Understanding.What, Next: d.Understanding.NextStep, Actor: d.Understanding.Actor}
	set := func(kind, label, effect, tone string) {
		g.Primary = MissionPrimaryAction{Kind: kind, Label: label, Effect: effect, Tone: tone}
	}
	switch {
	case d.Runtime.State == "blocked":
		set("runtime", "Diagnostic du stockage", "Ouvre le diagnostic de la machine ; aucun départ n’est autorisé.", "attention")
	case !d.Organization.Ready:
		set("organization", "Préparer l’organisation", "Affiche les rôles et les conditions manquantes.", "attention")
	case missionPlanningOpen(w) && w.Planning.Failure != "" && missionPlanningOwnsNextStep(d):
		g.What = "Le responsable n’a pas terminé sa décision."
		g.Next = "Examinez le diagnostic du responsable avant de reprendre la planification."
		set("planning", "Voir les décisions", "Ouvre les décisions et les retours du responsable.", "attention")
	case missionPlanningOpen(w) && w.Planning.Paused:
		set("planning-resume", "Reprendre la planification", "Demande la reprise des décisions dans les limites déjà autorisées.", "attention")
	default:
		needs := []MissionTask{}
		for _, t := range d.Tasks {
			if t.State == "review" || t.State == "intervention" {
				needs = append(needs, t)
			}
		}
		sort.SliceStable(needs, func(i, j int) bool { return needs[i].Impact > needs[j].Impact })
		if len(needs) > 0 {
			g.Primary = missionTaskPrimary(needs[0])
			return g
		}
		for _, t := range d.Tasks {
			if t.State == "configure" {
				g.Primary = missionTaskPrimary(t)
				return g
			}
		}
		for _, t := range d.Tasks {
			if t.State == "running" {
				set("follow", "Suivre cette tâche", "Ouvre le journal de cette tâche ; l’agent poursuit son travail.", "info")
				g.Primary.Task = t.ID
				return g
			}
		}
		if w.Planning != nil {
			for _, scope := range w.Planning.Scopes {
				if scope.State != "closed" {
					if !d.Authorized {
						set("start", "Lancer la mission", "Ouvre l’aperçu et la confirmation du lancement.", "info")
					} else {
						set("planning", "Voir les décisions", "Ouvre les décisions et les retours du responsable.", "info")
					}
					return g
				}
			}
		}
		closed := d.Total > 0
		for _, t := range d.Tasks {
			closed = closed && (t.State == "validated" || t.State == "waived" || t.State == "abandoned")
		}
		switch {
		case closed:
			set("results", "Voir les résultats", "Affiche les résultats et leur validation actuelle.", "attention")
			if d.Validated == d.Total {
				g.Primary.Tone = "succes"
			}
		case d.Total == 0:
			set("prepare", "Préparer les tâches", "Ouvre la préparation du besoin ; aucun agent n’est lancé.", "info")
		case d.Paused:
			set("resume", "Reprendre la mission", "Autorise les prochains départs dans les limites existantes.", "info")
		case !d.Authorized:
			set("start", "Lancer la mission", "Ouvre l’aperçu et la confirmation du lancement.", "info")
		case !d.Enabled:
			set("supervision", "Comprendre la reprise", "Affiche l’état du conducteur et les conditions de reprise.", "attention")
		default:
			set("results", "Voir ce qui attend", "Affiche les dépendances et les conditions du prochain départ.", "info")
		}
	}
	return g
}
