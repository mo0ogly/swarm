//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"os"
	"path/filepath"
)

func migratePreparationBudget(db *sql.DB, root string, backup bool) error {
	if backup {
		path := filepath.Join(root, ".swarm", newID("state-pre-v11-")+".db")
		f, e := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if e != nil {
			return e
		}
		if e = f.Close(); e != nil {
			return e
		}
		if _, e = db.Exec("VACUUM INTO ?", path); e != nil {
			return e
		}
	}
	_, e := db.Exec(`BEGIN IMMEDIATE;
 CREATE TABLE IF NOT EXISTS preparation_reservations(turn_id TEXT PRIMARY KEY REFERENCES preparation_turns(id), preparation_id TEXT NOT NULL REFERENCES preparations(id), work_id TEXT, amount REAL NOT NULL, work_amount REAL NOT NULL, state TEXT NOT NULL CHECK(state IN ('reserved','estimated','released')));
 CREATE INDEX IF NOT EXISTS preparation_budget_work ON preparation_reservations(work_id,state);
 CREATE INDEX IF NOT EXISTS preparation_budget_prep ON preparation_reservations(preparation_id,state);
 CREATE TABLE IF NOT EXISTS agent_dialogue_turns(id TEXT PRIMARY KEY, agent_id TEXT NOT NULL REFERENCES agents(id), question TEXT NOT NULL, status TEXT NOT NULL, session_id TEXT NOT NULL);
 CREATE UNIQUE INDEX IF NOT EXISTS dialogue_one_running ON agent_dialogue_turns(agent_id) WHERE status='running';
 PRAGMA user_version=11; COMMIT;`)
	return e
}
func preparationWorkCommitted(tx *sql.Tx, work string) (float64, error) {
	var n float64
	e := tx.QueryRow("SELECT coalesce(sum(work_amount),0) FROM preparation_reservations WHERE work_id=? AND state IN ('reserved','estimated')", work).Scan(&n)
	return n, e
}
func reservePreparationBudget(tx *sql.Tx, p Preparation, turn string) error {
	amount, workAmount := 0.0, 0.0
	if p.Budget != nil && p.Budget.Limit > 0 {
		var used float64
		if e := tx.QueryRow("SELECT coalesce(sum(amount),0) FROM preparation_reservations WHERE preparation_id=? AND state IN ('reserved','estimated')", p.ID).Scan(&used); e != nil {
			return e
		}
		if used+p.Budget.Reserve > p.Budget.Limit {
			return preparationError("budget_exhausted", "Budget de préparation insuffisant : ajustez l’enveloppe avant de renvoyer.")
		}
		amount = p.Budget.Reserve
	}
	if p.WorkID != "" {
		var raw []byte
		e := tx.QueryRow("SELECT body FROM budgets WHERE work_id=?", p.WorkID).Scan(&raw)
		if e != nil && e != sql.ErrNoRows {
			return e
		}
		if e == nil {
			var b Budget
			if e = json.Unmarshal(raw, &b); e != nil {
				return e
			}
			if b.Limit > 0 {
				prep, e := preparationWorkCommitted(tx, p.WorkID)
				if e != nil {
					return e
				}
				var used float64
				if e = tx.QueryRow("SELECT coalesce(sum(amount),0) FROM (SELECT amount FROM reservations WHERE work_id=? AND state IN ('reserved','estimated') UNION ALL SELECT amount FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated'))", p.WorkID, p.WorkID).Scan(&used); e != nil {
					return e
				}
				if used+prep+b.Reserve > b.Limit {
					return preparationError("budget_exhausted", "Budget du travail insuffisant : la préparation partage l’enveloppe avec ses agents et son assistant.")
				}
				workAmount = b.Reserve
			}
		}
	}
	_, e := tx.Exec("INSERT INTO preparation_reservations(turn_id,preparation_id,work_id,amount,work_amount,state) VALUES(?,?,?,?,?,'reserved')", turn, p.ID, p.WorkID, amount, workAmount)
	return e
}

// An estimated reservation is never automatically released after a process may
// have started, even if no provider usage was received.
func (s *Store) markPreparationAttempt(id string) error {
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	var status string
	if e = tx.QueryRow("SELECT status FROM preparation_turns WHERE id=?", id).Scan(&status); e != nil {
		return e
	}
	if status != "running" {
		return preparationError("interrupted", "Échange déjà arrêté.")
	}
	if _, e = tx.Exec("UPDATE preparation_reservations SET state='estimated' WHERE turn_id=? AND state='reserved'", id); e != nil {
		return e
	}
	return tx.Commit()
}
func (s *Store) finishPreparationBudget(id, previous, status string, body []byte) (sql.Result, error) {
	tx, e := s.db.Begin()
	if e != nil {
		return nil, e
	}
	defer tx.Rollback()
	result, e := tx.Exec("UPDATE preparation_turns SET status=?,body=? WHERE id=? AND status=?", status, body, id, previous)
	if e != nil {
		return nil, e
	}
	n, e := result.RowsAffected()
	if e != nil {
		return nil, e
	}
	if n == 1 {
		if _, e = tx.Exec("UPDATE preparation_reservations SET state='released' WHERE turn_id=? AND state='reserved'", id); e != nil {
			return nil, e
		}
	}
	if e = tx.Commit(); e != nil {
		return nil, e
	}
	return result, nil
}

type PreparationBudgetView struct {
	BudgetView
	Work       *BudgetView `json:"work_budget,omitempty"`
	Turns      int         `json:"turns"`
	UsageTurns int         `json:"usage_turns"`
	CostTurns  int         `json:"cost_turns"`
	Input      int64       `json:"input_tokens"`
	Output     int64       `json:"output_tokens"`
}

func (s *Store) preparationBudget(id string) (PreparationBudgetView, error) {
	v := PreparationBudgetView{BudgetView: BudgetView{ActualCostScope: "Somme des coûts rapportés par les échanges de cette préparation ; couverture indiquée par cost_turns et turns.", Policy: "Enveloppe estimative, pas une facture. Réservation avant envoi ; estimation engagée dès qu’un appel peut avoir démarré. Le coût rapporté et les jetons ne couvrent que les échanges qui les fournissent."}}
	p, e := s.preparation(id)
	if e != nil {
		return v, e
	}
	if p.Budget != nil {
		v.Budget = *p.Budget
	}
	e = s.db.QueryRow("SELECT coalesce(sum(CASE WHEN state='reserved' THEN amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN amount ELSE 0 END),0) FROM preparation_reservations WHERE preparation_id=?", id).Scan(&v.Reserved, &v.Estimated)
	if e != nil {
		return v, e
	}
	v.Remaining = max(0, v.Budget.Limit-v.Reserved-v.Estimated)
	v.Warning = v.Budget.Limit > 0 && v.Reserved+v.Estimated >= v.Budget.Limit*.8
	turns, e := s.preparationTurns(id)
	if e != nil {
		return v, e
	}
	v.Turns = len(turns)
	cost := 0.0
	for _, t := range turns {
		if t.Usage != nil {
			v.UsageTurns++
			v.Input += t.Usage.Input
			v.Output += t.Usage.Output
			if t.Usage.ReportedCost != nil {
				v.CostTurns++
				cost += *t.Usage.ReportedCost
			}
		}
	}
	if v.CostTurns > 0 {
		v.ActualCost = &cost
	}
	if p.WorkID != "" {
		b, err := s.budget(p.WorkID)
		if err != nil {
			return v, err
		}
		v.Work = &b
	}
	return v, nil
}
