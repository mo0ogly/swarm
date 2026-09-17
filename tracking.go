package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"strings"
)

const trackingMigration = `BEGIN;
CREATE TABLE decisions(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), body BLOB NOT NULL);
CREATE INDEX decisions_work ON decisions(work_id);
CREATE TABLE session_visits(work_id TEXT NOT NULL REFERENCES works(id), operator TEXT NOT NULL, revision INTEGER NOT NULL, at TEXT NOT NULL, PRIMARY KEY(work_id,operator));
CREATE TABLE budgets(work_id TEXT PRIMARY KEY REFERENCES works(id), body BLOB NOT NULL);
CREATE TABLE reservations(agent_id TEXT PRIMARY KEY REFERENCES agents(id), work_id TEXT NOT NULL REFERENCES works(id), amount REAL NOT NULL, state TEXT NOT NULL);
PRAGMA user_version=3;
COMMIT;`

type Decision struct {
	ID         string `json:"id"`
	TaskID     string `json:"task_id"`
	AgentID    string `json:"agent_id,omitempty"`
	Kind       string `json:"kind"`
	Summary    string `json:"summary"`
	Evidence   string `json:"evidence"`
	Created    string `json:"created"`
	Author     string `json:"author,omitempty"`
	ResolvedAt string `json:"resolved_at,omitempty"`
	Resolution string `json:"resolution,omitempty"`
}

func operatorIdentity() string {
	host, _ := os.Hostname()
	return fmt.Sprintf("%s/uid:%d", host, os.Getuid())
}
func (s *Store) decisions(work string) ([]Decision, error) {
	s = s.readScope()
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	agents, e := s.pilotAgents(work)
	if e != nil {
		return nil, e
	}
	budget, e := s.budget(work)
	if e != nil {
		return nil, e
	}
	in := escalationInputs{work: &w, agents: agents, budget: budget, validation: s.validationState(&w), gateValid: map[string]bool{}}
	// Même règle qu'à l'ordonnancement : une lecture de coût qui échoue ferait
	// disparaître le dépassement de l'écran, pas le dépassement lui-même.
	summary, e := s.costSummary(work)
	if e != nil {
		return nil, e
	}
	in.taskCost = summary.ByTask
	in.reserve = budget.Budget.Reserve
	for i := range w.Tasks {
		in.gateValid[w.Tasks[i].ID] = s.validGate(&w.Tasks[i])
	}
	// L'identifiant reste stable par sujet : une même demande ne réapparaît pas.
	// Les identifiants construits ici sont ceux des sujets actuels ; toute carte
	// enregistrée qui n'en fait plus partie décrit un sujet disparu.
	courants := map[string]bool{}
	// Sujet = tâche + catégorie. Sert à distinguer « une évaluation plus récente
	// a pris la suite » de « la condition a disparu » : dans le second cas la
	// carte reste ouverte, car un garde-fou de revalidation peut subsister.
	sujetsCourants := map[string]bool{}
	for _, x := range buildEscalations(in) {
		d := Decision{ID: hash([]byte(work + "|" + x.TaskID + "|" + x.AgentID + "|" + x.Kind + "|" + x.Version)), TaskID: x.TaskID, AgentID: x.AgentID, Kind: x.Kind, Summary: x.Summary, Evidence: x.Proof, Created: now()}
		courants[d.ID] = true
		sujetsCourants[d.TaskID+"|"+d.Kind] = true
		raw, _ := json.Marshal(d)
		if _, e := s.db.Exec("INSERT OR IGNORE INTO decisions(id,work_id,body) VALUES(?,?,?)", d.ID, work, raw); e != nil {
			return nil, e
		}
	}
	rows, e := s.db.Query("SELECT body FROM decisions WHERE work_id=? ORDER BY rowid", work)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []Decision{}
	for rows.Next() {
		var raw []byte
		var d Decision
		if e = rows.Scan(&raw); e != nil {
			return nil, e
		}
		if e = json.Unmarshal(raw, &d); e != nil {
			return nil, e
		}
		out = append(out, d)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	rows.Close()
	for i := range out {
		d := &out[i]
		if d.Kind != "gate" && d.Kind != "handoff" {
			continue
		}
		state, exists := in.validation.Tasks[d.TaskID]
		// Une gate réenregistrée crée une carte par version. Sans cette
		// distinction, la version précédente était rouverte de force à chaque
		// lecture et ne se refermait jamais : le même sujet s'empilait à
		// l'écran, une fois par évaluation passée.
		courant := courants[d.ID]
		switch {
		case courant && d.Kind == "gate" && exists && len(state.Blockers) > 0:
			d.Evidence = validationDetails(state)
			d.ResolvedAt = ""
		case exists && state.Fresh && d.ResolvedAt == "":
			if err := s.resolveDecision(work, d.ID, "moteur", "Revalidation constatée : acceptation et dépendances actuellement valides."); err != nil {
				return nil, err
			}
			d.Author = "moteur"
			d.ResolvedAt = now()
			d.Resolution = "Revalidation constatée : acceptation et dépendances actuellement valides."
		case !courant && sujetsCourants[d.TaskID+"|"+d.Kind] && d.ResolvedAt == "":
			const motif = "Sujet remplacé : une évaluation plus récente de la même tâche a pris la suite."
			if err := s.supersedeDecision(work, d.ID, motif); err != nil {
				return nil, err
			}
			d.Author = "moteur"
			d.ResolvedAt = now()
			d.Resolution = motif
		// Le sujet a disparu des demandes alors que la tâche reste acceptée :
		// sa dérive est devenue un état, visible sur la tâche, dont personne
		// n'attend d'arbitrage. Sans ce cas, les cartes déjà enregistrées
		// resteraient ouvertes indéfiniment et la règle ne vaudrait que pour
		// les travaux à venir.
		case !courant && !sujetsCourants[d.TaskID+"|"+d.Kind] && d.ResolvedAt == "" && settledDecisionTask(in.work, d.TaskID):
			const repos = "Acceptation en place et aucune tâche non terminée n'en dépend : la dérive des preuves reste visible sur la tâche."
			if err := s.supersedeDecision(work, d.ID, repos); err != nil {
				return nil, err
			}
			d.Author = "moteur"
			d.ResolvedAt = now()
			d.Resolution = repos
		}
	}
	return out, rows.Err()
}
func (s *Store) resolveDecision(work, id, author, note string) error {
	if strings.TrimSpace(author) == "" || len(strings.TrimSpace(note)) < 5 {
		return fmt.Errorf("auteur et décision motivée requis")
	}
	var body []byte
	if err := s.db.QueryRow("SELECT body FROM decisions WHERE work_id=? AND id=?", work, id).Scan(&body); err != nil {
		return err
	}
	var current Decision
	if err := json.Unmarshal(body, &current); err != nil {
		return err
	}
	if current.Kind == "gate" {
		w, err := s.get(work)
		if err != nil {
			return err
		}
		if state, ok := s.validationState(&w).Tasks[current.TaskID]; ok && !state.Fresh {
			return fmt.Errorf("Revalidation obligatoire : un acquittement ne résout pas ce blocage. %s", validationDetails(state))
		}
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var raw []byte
	if e = tx.QueryRow("SELECT body FROM decisions WHERE work_id=? AND id=?", work, id).Scan(&raw); e != nil {
		return e
	}
	var d Decision
	if e = json.Unmarshal(raw, &d); e != nil {
		return e
	}
	if d.ResolvedAt != "" {
		if d.Author == author && d.Resolution == note {
			return nil
		}
		return fmt.Errorf("décision déjà acquittée")
	}
	d.Author = author
	d.Resolution = note
	d.ResolvedAt = now()
	raw, _ = json.Marshal(d)
	if _, e = tx.Exec("UPDATE decisions SET body=? WHERE id=? AND work_id=?", raw, id, work); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, d.ResolvedAt, decisionEventKind(author), d.TaskID+" : "+author+" : "+note); e != nil {
		return e
	}
	return tx.Commit()
}

// supersedeDecision clôt une carte dont le sujet a été repris par une version
// plus récente. Ce n'est pas un acquittement : le blocage reste porté par la
// carte courante, donc le garde-fou de revalidation de resolveDecision — qui
// empêche un humain de faire taire un blocage actif — ne s'applique pas ici et
// n'est pas affaibli. Seul le moteur emprunte ce chemin.
func (s *Store) supersedeDecision(work, id, note string) error {
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var raw []byte
	if e = tx.QueryRow("SELECT body FROM decisions WHERE work_id=? AND id=?", work, id).Scan(&raw); e != nil {
		return e
	}
	var d Decision
	if e = json.Unmarshal(raw, &d); e != nil {
		return e
	}
	if d.ResolvedAt != "" {
		return nil
	}
	d.Author = "moteur"
	d.Resolution = note
	d.ResolvedAt = now()
	raw, _ = json.Marshal(d)
	if _, e = tx.Exec("UPDATE decisions SET body=? WHERE id=? AND work_id=?", raw, id, work); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, d.ResolvedAt, decisionEventKind(engineAuthor), d.TaskID+" : "+engineAuthor+" : "+note); e != nil {
		return e
	}
	return tx.Commit()
}

type Visit struct {
	Revision int    `json:"revision"`
	At       string `json:"at"`
	Operator string `json:"operator"`
}

func (s *Store) visit(work, operator string) (Visit, error) {
	v := Visit{Operator: operator}
	e := s.db.QueryRow("SELECT revision,at FROM session_visits WHERE work_id=? AND operator=?", work, operator).Scan(&v.Revision, &v.At)
	if e == sql.ErrNoRows {
		return v, nil
	}
	return v, e
}
func (s *Store) markVisit(work, operator string, revision int) error {
	_, e := s.db.Exec("INSERT INTO session_visits(work_id,operator,revision,at) VALUES(?,?,?,?) ON CONFLICT(work_id,operator) DO UPDATE SET revision=excluded.revision,at=excluded.at", work, operator, revision, now())
	return e
}

func gateDecisionVersion(t Task) string {
	if t.Gate != nil {
		return t.Gate.At
	}
	return "missing"
}

// settledDecisionTask : la tâche que vise cette carte a-t-elle reçu une
// décision humaine qui la clôt ? Une tâche rouverte ne compte pas : son
// garde-fou de revalidation doit rester visible dans les demandes.
func settledDecisionTask(w *Work, id string) bool {
	t, e := w.task(id)
	if e != nil {
		return false
	}
	return settledTask(t.Status)
}

// Le moteur ferme lui-même certaines demandes : revalidation constatée, sujet
// remplacé, dérive devenue un simple état. Le fil d'activité classe par type
// d'événement ; sans type distinct, ces fermetures s'affichaient « vous » et
// prêtaient à l'opérateur des décisions qu'il n'a pas prises — exactement ce
// que ce fil existe pour démentir.
const engineAuthor = "moteur"

func decisionEventKind(author string) string {
	if author == engineAuthor {
		return "decision.moteur"
	}
	return "decision"
}
