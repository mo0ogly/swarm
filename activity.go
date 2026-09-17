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
	At string `json:"at"`
	// Rang d'écriture dans sa table, préfixé par la source. Deux entrées peuvent
	// partager un horodatage ; sans ce discriminant, le curseur de pagination
	// les saute ensemble et les perd en silence.
	Rank    string `json:"rank"`
	Origin  string `json:"origin"`
	Kind    string `json:"kind"`
	Label   string `json:"label"`
	TaskID  string `json:"task_id,omitempty"`
	Message string `json:"message"`
}

// Clé de pagination : l'horodatage seul ne suffit pas à désigner une entrée.
func (x ActivityEntry) cursor() string { return x.At + "|" + x.Rank }

// Le rang est comparé comme une chaîne : le remplissage par des zéros conserve
// l'ordre numérique, sans quoi 10 se classerait avant 9.
func activityRank(source string, n int64) string {
	return fmt.Sprintf("%s%012d", source, n)
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
	// Vrai quand une source a atteint le plafond de lecture : l'historique
	// remonte moins loin qu'il n'existe, et le fil doit le dire plutôt que de
	// s'arrêter en laissant croire qu'il n'y a plus rien.
	Truncated bool `json:"history_truncated"`
}

// Plafond de lecture par source, aligné sur l'export d'archive.
const activitySourceLimit = 500

// Origine par type d'événement. Le défaut « moteur » SURESTIME l'autonomie :
// une décision humaine mal classée fait croire que le système a agi seul, ce
// que ce fil existe précisément pour démentir. Cette liste doit donc rester
// exhaustive : tout nouveau type d'événement produit par un geste d'opérateur
// s'ajoute ici, faute de quoi il sera attribué au moteur.
//
// `task.update` échappe à cette liste : les deux voies l'écrivent — l'opérateur
// depuis le cockpit, la CLI ou le terminal, et le moteur au départ, au relais
// et à la fin d'une tentative. Son origine se lit donc dans la charge utile,
// où `Request.Origin` nomme le moteur ; l'absence se lit « humain », et les
// points d'entrée externes effacent ce champ.
var activityHumanKinds = map[string]bool{
	"assist-action": true,
	"mission.start": true, "mission-policy": true,
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
	"assist-action": "Action conseillée par l’IA",
	"mission.start": "Configuration de la mission", "mission-policy": "Autorisation de la mission",
	"dispatch": "Ordonnancement", "conductor": "Conduite", "pause": "Départs",
	"autonomy": "Niveau d'autonomie", "budget": "Budget", "priority": "Priorité",
	"decision": "Décision", "decision.moteur": "Décision du moteur",
	"launch.refused": "Lancement refusé",
	"assistant":      "Assistant", "task.update": "Tâche", "gate": "Gate",
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
	lues := map[string]int{}

	rows, e := s.db.Query("SELECT seq,at,kind,message FROM cockpit_events WHERE work_id=? ORDER BY seq DESC LIMIT ?", work, activitySourceLimit+1)
	if e != nil {
		return page, e
	}
	for rows.Next() {
		var seq int64
		var at, kind, message string
		if e = rows.Scan(&seq, &at, &kind, &message); e != nil {
			rows.Close()
			return page, e
		}
		lues["controls"]++
		if lues["controls"] > activitySourceLimit {
			page.Truncated = true
			continue
		}
		entries = append(entries, ActivityEntry{At: at, Rank: activityRank("c", seq), Origin: activityOrigin(kind, activityPayload{}),
			Kind: kind, Label: activityLabel(kind), Message: missionActivityMessage(kind, message)})
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return page, e
	}

	rows, e = s.db.Query("SELECT revision,kind,at,payload FROM events WHERE work_id=? ORDER BY revision DESC LIMIT ?", work, activitySourceLimit+1)
	if e != nil {
		return page, e
	}
	for rows.Next() {
		var revision int64
		var kind, at string
		var raw []byte
		if e = rows.Scan(&revision, &kind, &at, &raw); e != nil {
			rows.Close()
			return page, e
		}
		var p activityPayload
		_ = json.Unmarshal(raw, &p)
		lues["events"]++
		if lues["events"] > activitySourceLimit {
			page.Truncated = true
			continue
		}
		entries = append(entries, ActivityEntry{At: at, Rank: activityRank("e", revision), Origin: activityOrigin(kind, p),
			Kind: kind, Label: activityLabel(kind), TaskID: p.task(), Message: activityMessage(kind, p)})
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return page, e
	}

	// Ordre total sur (horodatage, rang) : deux entrées simultanées restent
	// départageables, ce qu'un tri sur le seul horodatage ne garantit pas.
	sort.Slice(entries, func(i, j int) bool {
		return entries[i].cursor() > entries[j].cursor()
	})

	for _, x := range entries {
		if q.DecisionsOnly && x.Origin != activityHuman {
			continue
		}
		if q.Before != "" && x.cursor() >= q.Before {
			continue
		}
		if len(page.Entries) == q.Limit {
			page.More = true
			break
		}
		page.Entries = append(page.Entries, x)
	}
	// Le curseur ne vaut que s'il reste quelque chose à lire : le rendre sans
	// suite ferait paginer un appelant vers une page vide.
	if n := len(page.Entries); n > 0 && page.More {
		page.Next = page.Entries[n-1].cursor()
	}
	return page, nil
}

func activityOrigin(kind string, p activityPayload) string {
	if kind == "task.update" {
		if p.Origin == conductorAuthor {
			return activityEngine
		}
		return activityHuman
	}
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
		return prefixTask(p.task(), "évaluation enregistrée")
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

func missionActivityMessage(kind, message string) string {
	if kind == "assist-action" {
		var r AssistActionReceipt
		if json.Unmarshal([]byte(message), &r) == nil {
			return r.Message
		}
		return "Résultat de l’action à vérifier"
	}
	if kind != "mission-policy" {
		return message
	}
	var p MissionPolicy
	if json.Unmarshal([]byte(message), &p) != nil {
		return "Autorisation de mission illisible : vérifier les réglages"
	}
	if p.Enabled {
		return "Mission continue autorisée ; les tâches prêtes seront enchaînées"
	}
	return "Mission continue arrêtée ; aucun nouveau départ autorisé"
}
