//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"
)

// Fil d'activité : ce que le moteur a fait seul et ce qu'un humain a décidé,
// dans l'ordre du temps. Les deux tables sont déjà remplies ; cet oracle ne
// collecte rien de neuf et n'interprète rien. Un motif absent le reste.
const (
	activityEngine = "moteur"
	activityHuman  = "vous"
)

type ActivityEntry struct {
	At      string `json:"at"`
	Origin  string `json:"origin"`
	Kind    string `json:"kind"`
	Label   string `json:"label"`
	TaskID  string `json:"task_id,omitempty"`
	Message string `json:"message"`
}

type activityQuery struct {
	Before        string // curseur : horodatage strictement antérieur
	Limit         int
	DecisionsOnly bool
}

type ActivityPage struct {
	Entries []ActivityEntry `json:"entries"`
	Next    string          `json:"next_cursor"`
	More    bool            `json:"more"`
}

// Origine par type d'événement. Le défaut « moteur » SURESTIME l'autonomie :
// une décision humaine mal classée fait croire que le système a agi seul, ce
// que ce fil existe précisément pour démentir. Cette liste doit donc rester
// exhaustive : tout nouveau type d'événement produit par un geste d'opérateur
// s'ajoute ici, faute de quoi il sera attribué au moteur.
//
// Limite connue : le type seul ne portera pas l'origine indéfiniment.
// `task.update` est déjà écrit par les deux voies — l'ordonnanceur
// (dispatcher.go) et l'opérateur (cockpit_service.go). La donnée discriminante
// existe pourtant en base et n'est pas lue ici : `Launch.Origin` vaut
// « conducteur » quand c'est l'ordonnanceur qui lance.
var activityHumanKinds = map[string]bool{
	"pause": true, "autonomy": true, "priority": true, "budget": true,
	"decision": true, "assistant": true, "ooda": true, "checkpoint": true,
	"gate": true, "work.create": true, "task.add": true,
	"task.submit": true, "task.override": true,
	"plan.adopt": true, "brief.adopt": true,
}

// Les retours d'expérience sont saisis depuis le cockpit ; leurs types sont
// construits dynamiquement à partir du préfixe (web_server.go), donc aucune
// liste ne peut les énumérer.
const activityRetexPrefix = "retex-"

var activityLabels = map[string]string{
	"dispatch": "Ordonnancement", "conductor": "Conduite", "pause": "Départs",
	"autonomy": "Niveau d'autonomie", "budget": "Budget", "priority": "Priorité",
	"decision": "Décision", "launch.refused": "Lancement refusé",
	"assistant": "Assistant", "task.update": "Tâche", "gate": "Gate",
	"agent.start": "Tentative", "ooda": "Boucle OODA", "checkpoint": "Point d'étape",
	"work.create": "Travail", "task.add": "Tâche ajoutée",
	"task.submit": "Rapport soumis", "task.override": "Dérogation",
	"plan.adopt": "Plan adopté", "brief.adopt": "Brief adopté",
	"retex-save": "Retour d'expérience", "retex-qualify": "Retour d'expérience",
	"retex-status": "Retour d'expérience", "retex-task": "Retour d'expérience",
}

func activityLabel(kind string) string {
	if l, ok := activityLabels[kind]; ok {
		return l
	}
	if strings.HasPrefix(kind, activityRetexPrefix) {
		return "Retour d'expérience"
	}
	return kind
}

// La table events porte deux formes de charge utile : les mutations du travail
// sérialisent une Request, qui nomme la tâche « id », tandis que le lancement
// d'agent sérialise un Launch, qui la nomme « task_id ». Les deux orthographes
// sont réellement enregistrées ; les lire toutes les deux évite d'attribuer une
// tentative à aucune tâche, sans rien deviner de ce qui n'a pas été écrit.
type activityPayload struct {
	Request
	TaskID string `json:"task_id,omitempty"`
}

func (p activityPayload) task() string {
	if p.ID != "" {
		return p.ID
	}
	return p.TaskID
}

func (s *Store) activity(work string, q activityQuery) (ActivityPage, error) {
	// Même politique que queryLogs (log_query.go) : une demande excessive est
	// ramenée au plafond, pas à la plus petite page.
	if q.Limit <= 0 {
		q.Limit = 50
	}
	q.Limit = min(200, max(1, q.Limit))
	page := ActivityPage{Entries: []ActivityEntry{}}
	entries := []ActivityEntry{}

	rows, e := s.db.Query("SELECT at,kind,message FROM cockpit_events WHERE work_id=? ORDER BY seq DESC LIMIT 500", work)
	if e != nil {
		return page, e
	}
	for rows.Next() {
		var at, kind, message string
		if e = rows.Scan(&at, &kind, &message); e != nil {
			rows.Close()
			return page, e
		}
		entries = append(entries, ActivityEntry{At: at, Origin: activityOrigin(kind),
			Kind: kind, Label: activityLabel(kind), Message: message})
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return page, e
	}

	rows, e = s.db.Query("SELECT kind,at,payload FROM events WHERE work_id=? ORDER BY revision DESC LIMIT 500", work)
	if e != nil {
		return page, e
	}
	for rows.Next() {
		var kind, at string
		var raw []byte
		if e = rows.Scan(&kind, &at, &raw); e != nil {
			rows.Close()
			return page, e
		}
		var p activityPayload
		_ = json.Unmarshal(raw, &p)
		entries = append(entries, ActivityEntry{At: at, Origin: activityOrigin(kind),
			Kind: kind, Label: activityLabel(kind), TaskID: p.task(), Message: activityMessage(kind, p)})
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return page, e
	}

	// Une entrée sans horodatage exploitable part en fin plutôt que de prendre
	// une place qu'elle ne mérite pas.
	sort.SliceStable(entries, func(i, j int) bool {
		if entries[i].At == entries[j].At {
			return entries[i].Kind < entries[j].Kind
		}
		return entries[i].At > entries[j].At
	})

	for _, x := range entries {
		if q.DecisionsOnly && x.Origin != activityHuman {
			continue
		}
		if q.Before != "" && x.At >= q.Before {
			continue
		}
		if len(page.Entries) == q.Limit {
			page.More = true
			break
		}
		page.Entries = append(page.Entries, x)
	}
	if n := len(page.Entries); n > 0 {
		page.Next = page.Entries[n-1].At
	}
	return page, nil
}

func activityOrigin(kind string) string {
	if activityHumanKinds[kind] || strings.HasPrefix(kind, activityRetexPrefix) {
		return activityHuman
	}
	return activityEngine
}

// Message métier lisible, construit uniquement à partir de champs enregistrés.
// Une tâche non nommée dans la charge utile reste anonyme : le fil dit ce qui
// s'est produit sans prêter l'événement à une tâche qui n'y figure pas.
func activityMessage(kind string, p activityPayload) string {
	task := p.task()
	switch kind {
	case "task.update":
		if p.Status == "" {
			return prefixTask(task, "mise à jour")
		}
		message := prefixTask(task, uiStatus(p.Status))
		if p.Blocker != "" {
			message += " · " + p.Blocker
		}
		return message
	case "gate":
		return "évaluation enregistrée"
	case "agent.start":
		return prefixTask(task, "tentative lancée")
	case "ooda":
		return "décision : " + p.Decision
	case "checkpoint":
		return p.Summary
	}
	if p.Title != "" {
		return p.Title
	}
	return activityLabel(kind)
}

func prefixTask(task, message string) string {
	if task == "" {
		return message
	}
	return fmt.Sprintf("%s : %s", task, message)
}
