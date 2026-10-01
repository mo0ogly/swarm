//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"sort"
)

type MissionWait struct {
	Task   string `json:"task"`
	Title  string `json:"title"`
	State  string `json:"state"`
	Reason string `json:"reason"`
}
type RecoveryPreview struct {
	Work       string   `json:"work"`
	Task       string   `json:"task"`
	Revision   int      `json:"revision"`
	Agent      string   `json:"agent,omitempty"`
	Kept       []string `json:"kept"`
	Redone     []string `json:"redone"`
	Correction string   `json:"correction"`
	Criteria   []string `json:"criteria"`
	Limits     string   `json:"limits"`
}
type MissionChange struct {
	Revision int    `json:"revision"`
	At       string `json:"at"`
	Category string `json:"category"`
	Label    string `json:"label"`
	Task     string `json:"task,omitempty"`
	Title    string `json:"title,omitempty"`
}
type MissionChanges struct {
	From  int             `json:"from_revision"`
	To    int             `json:"to_revision"`
	First bool            `json:"first_visit"`
	More  bool            `json:"more"`
	Items []MissionChange `json:"items"`
}
type SpendingRow struct {
	Kind         string    `json:"kind"`
	ID           string    `json:"id"`
	Label        string    `json:"label"`
	Calls        int       `json:"recorded_calls"`
	Tools        int       `json:"observed_tool_calls"`
	UnknownTools int       `json:"attempts_with_incomplete_tool_measurement"`
	Input        int64     `json:"input_tokens"`
	Output       int64     `json:"output_tokens"`
	MissingUsage int       `json:"calls_without_usage"`
	Cost         CostTotal `json:"cost"`
}
type MissionSpending struct {
	Rows     []SpendingRow `json:"rows"`
	Controls int           `json:"recorded_control_executions"`
	Retries  int           `json:"worker_retries"`
	Note     string        `json:"note"`
}

func (s *Store) taskWaits(w *Work, t *Task) []MissionWait {
	out := []MissionWait{}
	for _, id := range t.Depends {
		parent, _ := w.task(id)
		if s.acceptedFresh(w, parent, map[string]bool{}) {
			continue
		}
		item := MissionWait{Task: id, Title: id, State: "missing", Reason: "Prérequis absent du plan."}
		if parent != nil {
			item.Title = parent.Title
			item.State = parent.Status
			item.Reason = "Ce résultat n’est pas encore validé."
			if parent.Status == "accepted" {
				item.State = "stale"
				item.Reason = "Validation ancienne : les preuves actuelles doivent être vérifiées à nouveau."
			}
		}
		out = append(out, item)
	}
	return out
}

func recoveryPreviewFor(w Work, t *Task, a *Agent) RecoveryPreview {
	p := RecoveryPreview{Work: w.ID, Task: t.ID, Revision: w.Revision,
		Kept:       []string{"Historique des tentatives, rapports et avis précédents.", "Critères et dépendances du plan."},
		Redone:     []string{"Vérification des conditions de lancement et du candidat courant.", "Contrôles et revue requis pour la nouvelle tentative."},
		Correction: t.Next, Criteria: append([]string{}, t.Criteria...),
		Limits: "Aucun budget ni plafond n’est augmenté. Ce résumé n’autorise aucun départ et ne valide aucun résultat."}
	if a != nil {
		p.Agent = a.ID
		p.Kept = append(p.Kept, "Fournisseur et espace de la tentative sélectionnée, sous réserve des règles du lancement.")
		p.Redone = append(p.Redone, "Une nouvelle tentative d’agent ; l’ancienne reste dans l’historique.")
	}
	if t.CorrectiveRecovery != nil && t.CorrectiveRecovery.Instruction != "" {
		p.Correction = t.CorrectiveRecovery.Instruction
	}
	if w.Planning != nil && w.Planning.Repository != nil {
		p.Kept = append(p.Kept, "Résultat précédent réutilisable seulement après contrôle du dossier de reprise ; ancienne copie conservée.")
	}
	return p
}

func (s *Store) recoveryPreview(work, task, agent string) (RecoveryPreview, error) {
	w, e := s.get(work)
	if e != nil {
		return RecoveryPreview{}, e
	}
	t, e := w.task(task)
	if e != nil {
		return RecoveryPreview{}, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return RecoveryPreview{}, e
	}
	for i := range agents {
		a := &agents[i]
		if a.TaskID == task && (agent == "" || a.ID == agent) {
			return recoveryPreviewFor(w, t, a), nil
		}
	}
	if agent != "" {
		return RecoveryPreview{}, fmt.Errorf("tentative hors tâche ou introuvable")
	}
	return recoveryPreviewFor(w, t, nil), nil
}

// Typed events only: never classify prose or treat an historical event as
// current acceptance. Unknown kinds stay in the full activity feed.
func missionChangeCategory(kind string, raw []byte) (string, string) {
	var p struct {
		Status string `json:"status"`
		State  string `json:"state"`
	}
	_ = json.Unmarshal(raw, &p)
	switch kind {
	case "task.update":
		switch p.Status {
		case "submitted":
			return "result", "Résultat soumis pour examen"
		case "accepted":
			return "result", "Acceptation enregistrée"
		case "blocked":
			return "block", "Blocage enregistré"
		}
	case "review.result":
		if p.State == "passed" {
			return "result", "Avis indépendant enregistré"
		}
		return "block", "Vérification à examiner"
	case "review.failure", "planning.proof-stale":
		return "block", "Preuve ou vérification à examiner"
	case "task.auto-validation":
		if p.State == "blocked" {
			return "block", "Contrôle en échec"
		}
		return "result", "Contrôles enregistrés"
	case "task.auto-validation-reviewed":
		return "result", "Acceptation après contrôles et revue"
	case "decision", "decision.moteur", "planning.decide", "planning.authorize-recovery", "planning.extend-attempt":
		return "decision", "Décision enregistrée"
	}
	return "", ""
}

func (s *Store) missionChanges(w Work, v Visit) (MissionChanges, error) {
	out := MissionChanges{From: v.Revision, To: w.Revision, First: v.At == "", Items: []MissionChange{}}
	if out.First {
		return out, nil
	}
	rows, e := s.db.Query("SELECT revision,kind,at,payload FROM events WHERE work_id=? AND revision>? AND revision<=? ORDER BY revision DESC LIMIT 201", w.ID, v.Revision, w.Revision)
	if e != nil {
		return out, e
	}
	defer rows.Close()
	count := 0
	for rows.Next() {
		var rev int
		var kind, at string
		var raw []byte
		if e = rows.Scan(&rev, &kind, &at, &raw); e != nil {
			return out, e
		}
		count++
		if count > 200 {
			out.More = true
			break
		}
		category, label := missionChangeCategory(kind, raw)
		if category == "" {
			continue
		}
		var p activityPayload
		_ = json.Unmarshal(raw, &p)
		id := p.task()
		if kind == "task.auto-validation" {
			id = ""
			var record AutomaticValidation
			_ = json.Unmarshal(raw, &record)
			for _, t := range w.Tasks {
				for _, a := range t.Attempts {
					if a.ID == record.Attempt {
						id = t.ID
					}
				}
			}
		}
		// Review IDs and receipt IDs do not name tasks. Resolve only identities
		// explicitly persisted on the current task or its review history.
		if kind == "review.result" {
			id = ""
			var review IndependentReview
			_ = json.Unmarshal(raw, &review)
			for _, t := range w.Tasks {
				if t.IndependentReview != nil && t.IndependentReview.ID == review.ID {
					id = t.ID
				}
				for _, r := range t.PreviousReviews {
					if r.ID == review.ID {
						id = t.ID
					}
				}
			}
		}
		title := ""
		if t, _ := w.task(id); t != nil {
			title = t.Title
		} else {
			id = ""
		}
		out.Items = append(out.Items, MissionChange{rev, at, category, label, id, title})
	}
	return out, rows.Err()
}

func addMeasuredUsage(row *SpendingRow, u *Usage) {
	if u == nil {
		row.MissingUsage++
		row.Cost.Silent++
		return
	}
	row.Input += u.Input
	row.Output += u.Output
	if u.ReportedCost == nil {
		row.Cost.Silent++
	} else {
		row.Cost.WithCost++
		row.Cost.Reported += *u.ReportedCost
	}
}
func (s *Store) missionSpending(w Work, agents []Agent) (MissionSpending, error) {
	out := MissionSpending{Rows: []SpendingRow{}, Note: "Les appels d’outils, les appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n’est pas un zéro ; les relances internes du fournisseur ne sont pas toutes observables. Une réservation d’appel ne prouve pas son envoi au fournisseur."}
	by := map[string]*SpendingRow{}
	rowFor := func(kind, id, label string) *SpendingRow {
		key := kind + ":" + id
		if by[key] == nil {
			by[key] = &SpendingRow{Kind: kind, ID: id, Label: label}
		}
		return by[key]
	}
	for _, a := range agents {
		label := a.TaskID
		if t, _ := w.task(a.TaskID); t != nil {
			label = t.Title
		}
		row := rowFor("worker", a.TaskID, label)
		row.Calls++
		row.Tools += a.Progress.ToolCalls
		if a.Mode == "terminal" || a.Progress.Degraded != "" {
			row.UnknownTools++
		}
		if a.Previous != "" {
			out.Retries++
		}
		addMeasuredUsage(row, a.Usage)
	}
	rows, e := s.db.Query("SELECT scope_id,usage FROM planning_calls WHERE work_id=? AND state!='released' ORDER BY created_at,id", w.ID)
	if e != nil {
		return out, e
	}
	for rows.Next() {
		var scope string
		var raw []byte
		if e = rows.Scan(&scope, &raw); e != nil {
			rows.Close()
			return out, e
		}
		kind, label := "planner", "Responsable : "+scope
		if scope == "reviewer" {
			kind, label = "reviewer", "Vérificateur indépendant"
		}
		row := rowFor(kind, scope, label)
		row.Calls++
		var u Usage
		if len(raw) > 0 && string(raw) != "null" {
			if e = json.Unmarshal(raw, &u); e != nil {
				rows.Close()
				return out, e
			}
			addMeasuredUsage(row, &u)
		} else {
			addMeasuredUsage(row, nil)
		}
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return out, e
	}
	controls, e := s.db.Query("SELECT payload FROM events WHERE work_id=? AND kind='task.auto-validation'", w.ID)
	if e != nil {
		return out, e
	}
	for controls.Next() {
		var raw []byte
		if e = controls.Scan(&raw); e != nil {
			controls.Close()
			return out, e
		}
		var record AutomaticValidation
		if e = json.Unmarshal(raw, &record); e != nil {
			controls.Close()
			return out, e
		}
		for _, c := range record.Controls {
			if c.Executed {
				out.Controls++
			}
		}
	}
	e = controls.Err()
	controls.Close()
	if e != nil {
		return out, e
	}
	keys := []string{}
	for k := range by {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		out.Rows = append(out.Rows, *by[k])
	}
	return out, nil
}

func printMissionChanges(out io.Writer, c MissionChanges) {
	fmt.Fprintln(out, uiText("Depuis votre dernière visite"))
	if c.First {
		fmt.Fprintln(out, uiText("Première visite : aucun repère précédent. L’historique complet reste disponible."))
		return
	}
	if c.More {
		fmt.Fprintln(out, uiText("Extrait des 200 derniers événements ; consultez le fil complet pour les plus anciens."))
	}
	if len(c.Items) == 0 {
		fmt.Fprintln(out, uiText("Aucun nouveau résultat, blocage ou décision dans cet extrait."))
	}
	for _, x := range c.Items {
		fmt.Fprintf(out, "r%d · %s · %s · %s\n", x.Revision, x.At, uiText(x.Label), x.Title)
	}
	fmt.Fprintln(out, uiText("Ces événements sont historiques ; l’état actuel de la tâche fait foi."))
}
func printMissionSpending(out io.Writer, c MissionSpending) {
	fmt.Fprintln(out, uiText("Où vont les appels et les coûts ?"))
	for _, r := range c.Rows {
		label := r.Label
		if r.Kind != "worker" {
			label = uiEngineText(label)
		}
		fmt.Fprintf(out, uiText("%s · %d tentatives ou appels enregistrés · %d appels d’outils observés · %d mesures d’outils incomplètes · %d/%d jetons entrée/sortie rapportés · %d usages absents · %s\n"), label, r.Calls, r.Tools, r.UnknownTools, r.Input, r.Output, r.MissingUsage, uiEngineText(r.Cost.Text()))
	}
	fmt.Fprintf(out, uiText("Moteur : %d contrôles enregistrés · %d reprises d’agents\n"), c.Controls, c.Retries)
	fmt.Fprintln(out, uiText(c.Note))
}
func printRecoveryPreview(out io.Writer, p RecoveryPreview) {
	fmt.Fprintln(out, uiText("Avant une relance"))
	for _, section := range []struct {
		label string
		items []string
	}{{"Ce qui sera conservé", p.Kept}, {"Ce qui sera refait", p.Redone}, {"Critères inchangés", p.Criteria}} {
		fmt.Fprintln(out, uiText(section.label))
		for _, item := range section.items {
			fmt.Fprintln(out, "- "+uiEngineText(item))
		}
	}
	fmt.Fprintln(out, uiText("Correction attendue"))
	fmt.Fprintln(out, p.Correction)
	fmt.Fprintln(out, uiText(p.Limits))
}
