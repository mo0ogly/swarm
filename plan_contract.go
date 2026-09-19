package main

import (
	"encoding/json"
	"fmt"
	"io"
	"strings"
)

type PlanQuestion struct {
	Question string `json:"question"`
	Answer   string `json:"answer"`
}
type PlanMission struct {
	ID           string   `json:"id"`
	Title        string   `json:"title"`
	Role         string   `json:"role"`
	Scope        string   `json:"scope"`
	Deliverable  string   `json:"deliverable"`
	Depends      []string `json:"depends"`
	Criteria     []string `json:"criteria"`
	Proof        string   `json:"proof"`
	Entry        string   `json:"entry"`
	Validation   string   `json:"validation"`
	Delivery     string   `json:"delivery"`
	Stop         string   `json:"stop"`
	MaxAttempts  int      `json:"max_attempts"`
	MaxToolCalls int      `json:"max_tool_calls"`
}
type ActionPlan struct {
	Version     int            `json:"version"`
	Objective   string         `json:"objective"`
	Assumptions []string       `json:"assumptions"`
	Questions   []PlanQuestion `json:"questions"`
	Tasks       []PlanMission  `json:"tasks"`
}
type ApprovedPlan struct {
	Source       string     `json:"source"`
	BriefHash    string     `json:"brief_hash"`
	ResponseHash string     `json:"response_hash"`
	Spec         ActionPlan `json:"spec"`
	TaskIDs      []string   `json:"task_ids"`
	At           string     `json:"at"`
}
type PlanReview struct {
	Source       string     `json:"source"`
	BriefHash    string     `json:"brief_hash"`
	ResponseHash string     `json:"response_hash"`
	Spec         ActionPlan `json:"spec"`
}

func planDirectives() string {
	return `
CADRE OBLIGATOIRE DU PLAN D’ACTION — VERSION 1
Tu es planner APEX. Analyse le brief adopté et propose des missions exécutables. Aucune écriture, commande mutante ou délégation. Aucun worker à lancer. Ne réclame aucun PASS, score ou coût mesuré sans preuve.
Réponds UNIQUEMENT par un objet JSON strict conforme au modèle suivant, sans Markdown, balises, commentaire ou texte autour. Tous les champs sont obligatoires. Maximum 8 tâches, 16000 octets au total. Les identifiants locaux sont uniques ; depends ne référence que ces identifiants, sans cycle. Les dépendances doivent être acceptées avant lancement. Le rôle d’une tâche est toujours worker.
Les questions non décidées doivent rester dans questions avec answer vide ; ne pas inventer les réponses de l’opérateur. Les hypothèses figurent dans assumptions, distinctes des faits. Les gates sont des contrôles à réaliser, jamais des résultats déjà acquis. Chaque tâche exécutable a le rôle worker ; planner et subplanner sont des responsables de périmètre créés par la planification hiérarchique, jamais des tâches de code. Définir un livrable concret, des critères observables, les preuves, les conditions d’arrêt et l’OODA sur blocage. max_attempts entre 1 et 3 ; max_tool_calls entre 1 et 100. Même pour une mission documentaire, max_tool_calls doit être positif (par exemple 10), jamais zéro. Ces plafonds limitent effectivement les futures tentatives, sans augmenter les limites du fournisseur.
{"version":1,"objective":"Objectif du plan","assumptions":[],"questions":[{"question":"Décision manquante","answer":""}],"tasks":[{"id":"T1","title":"Mission précise","role":"worker","scope":"Fichiers et exclusions","deliverable":"docs/T1-handoff.md","depends":[],"criteria":["Résultat observable"],"proof":"Fichiers et commandes de vérification","entry":"Prérequis à vérifier","validation":"Tests et résultats attendus","delivery":"Revue et gate fraîche avant acceptation","stop":"Au plus deux corrections puis OODA et arrêt si blocage persistant","max_attempts":2,"max_tool_calls":30}]}
Un champ absent, un JSON invalide, une dépendance inconnue ou un cycle rend la réponse inutilisable. Ne pas suivre le format Markdown des échanges précédents.
`
}

func parseActionPlan(text string) (ActionPlan, error) {
	var p ActionPlan
	if len(text) > 16000 {
		return p, fmt.Errorf("Plan trop long : 16000 octets maximum.")
	}
	d := json.NewDecoder(strings.NewReader(text))
	d.DisallowUnknownFields()
	if e := d.Decode(&p); e != nil {
		return p, fmt.Errorf("Réponse non conforme au cadre JSON du plan : %v", e)
	}
	var extra any
	if d.Decode(&extra) != io.EOF {
		return p, fmt.Errorf("Le plan doit contenir un seul objet JSON, sans texte supplémentaire.")
	}
	var raw map[string]json.RawMessage
	_ = json.Unmarshal([]byte(text), &raw)
	for _, k := range []string{"version", "objective", "assumptions", "questions", "tasks"} {
		if _, ok := raw[k]; !ok {
			return p, fmt.Errorf("Champ obligatoire absent : %s", k)
		}
	}
	var qs []map[string]json.RawMessage
	_ = json.Unmarshal(raw["questions"], &qs)
	for _, q := range qs {
		if value, ok := q["answer"]; !ok || len(value) == 0 || value[0] != '"' {
			return p, fmt.Errorf("Chaque question doit contenir answer vide pour la décision opérateur.")
		}
	}
	var missions []map[string]json.RawMessage
	_ = json.Unmarshal(raw["tasks"], &missions)
	for i, m := range missions {
		for _, k := range []string{"id", "title", "role", "scope", "deliverable", "depends", "criteria", "proof", "entry", "validation", "delivery", "stop", "max_attempts", "max_tool_calls"} {
			if _, ok := m[k]; !ok {
				return p, fmt.Errorf("Mission %d : champ obligatoire absent %s", i+1, k)
			}
		}
	}
	for _, q := range p.Questions {
		if nonempty(q.Answer) {
			return p, fmt.Errorf("L’IA ne peut pas répondre à la place de l’opérateur : answer doit rester vide.")
		}
	}
	_, e := validateActionPlan(p, false)
	return p, e
}
func validateActionPlan(p ActionPlan, requireAnswers bool) ([]PlanMission, error) {
	b, _ := json.Marshal(p)
	if len(b) > 32000 {
		return nil, fmt.Errorf("Plan relu trop long : 32000 octets maximum.")
	}
	if p.Version != 1 || !nonempty(p.Objective) || p.Assumptions == nil || p.Questions == nil || len(p.Tasks) < 1 || len(p.Tasks) > 8 {
		return nil, fmt.Errorf("Cadre requis : version 1, objectif, listes assumptions/questions et 1 à 8 tâches.")
	}
	if len(p.Assumptions) > 16 || len(p.Questions) > 16 {
		return nil, fmt.Errorf("16 hypothèses ou questions maximum.")
	}
	for _, a := range p.Assumptions {
		if !nonempty(a) {
			return nil, fmt.Errorf("Hypothèse vide.")
		}
	}
	for _, q := range p.Questions {
		if !nonempty(q.Question) || requireAnswers && !nonempty(q.Answer) {
			return nil, fmt.Errorf("Décision non résolue : %s. Renseigner la réponse avant enregistrement.", q.Question)
		}
	}
	by := map[string]PlanMission{}
	for _, t := range p.Tasks {
		if !safeName(t.ID) || len(t.ID) > 32 {
			return nil, fmt.Errorf("Identifiant de mission invalide : %s", t.ID)
		}
		if _, ok := by[t.ID]; ok {
			return nil, fmt.Errorf("Identifiant en double : %s", t.ID)
		}
		if t.Role != "worker" {
			return nil, fmt.Errorf("%s : rôle invalide ; une tâche exécutable doit être worker, les responsables sont des périmètres hiérarchiques.", t.ID)
		}
		for name, v := range map[string]string{"titre": t.Title, "périmètre": t.Scope, "livrable": t.Deliverable, "preuves": t.Proof, "gate entry": t.Entry, "gate validation": t.Validation, "gate delivery": t.Delivery, "arrêt": t.Stop} {
			if !nonempty(v) {
				return nil, fmt.Errorf("%s : %s obligatoire.", t.ID, name)
			}
		}
		if len(t.Criteria) < 1 || len(t.Criteria) > 12 || t.Depends == nil || t.MaxAttempts < 1 || t.MaxAttempts > 3 || t.MaxToolCalls < 1 || t.MaxToolCalls > 100 {
			return nil, fmt.Errorf("%s : critères, liste de dépendances et plafonds (1–3 tentatives, 1–100 outils) requis.", t.ID)
		}
		for _, v := range t.Criteria {
			if !nonempty(v) {
				return nil, fmt.Errorf("%s : critère vide.", t.ID)
			}
		}
		by[t.ID] = t
	}
	state := map[string]int{}
	ordered := []PlanMission{}
	var visit func(string) error
	visit = func(id string) error {
		if state[id] == 1 {
			return fmt.Errorf("Cycle de dépendances : %s", id)
		}
		if state[id] == 2 {
			return nil
		}
		t, ok := by[id]
		if !ok {
			return fmt.Errorf("Dépendance inconnue : %s", id)
		}
		state[id] = 1
		seen := map[string]bool{}
		for _, dep := range t.Depends {
			if seen[dep] {
				return fmt.Errorf("Dépendance répétée : %s", dep)
			}
			seen[dep] = true
			if e := visit(dep); e != nil {
				return e
			}
		}
		state[id] = 2
		ordered = append(ordered, t)
		return nil
	}
	for _, t := range p.Tasks {
		if e := visit(t.ID); e != nil {
			return nil, e
		}
	}
	return ordered, nil
}
