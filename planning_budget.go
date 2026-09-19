package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
)

const planningRuntimeMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS planning_calls(id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,scope_id TEXT NOT NULL,estimate REAL NOT NULL,usage BLOB,state TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS managed_attempts(agent_id TEXT PRIMARY KEY,work_id TEXT NOT NULL REFERENCES works(id) ON DELETE CASCADE,task_id TEXT NOT NULL,base_commit TEXT NOT NULL,path TEXT NOT NULL,state TEXT NOT NULL,result_commit TEXT NOT NULL DEFAULT '',detail TEXT NOT NULL DEFAULT '');
PRAGMA user_version=19; COMMIT;`

func planningCommitted(tx *sql.Tx, work string) (float64, error) {
	var value float64
	err := tx.QueryRow("SELECT coalesce(sum(estimate),0) FROM planning_calls WHERE work_id=? AND state!='released'", work).Scan(&value)
	return value, err
}
func reservePlanningCall(tx *sql.Tx, work, scope, id string) error {
	if err := checkLaunchBudget(tx, work, id, false); err != nil {
		return err
	}
	var raw []byte
	reserve := 0.0
	err := tx.QueryRow("SELECT body FROM budgets WHERE work_id=?", work).Scan(&raw)
	if err != nil && err != sql.ErrNoRows {
		return err
	}
	if err == nil {
		var b Budget
		if err = json.Unmarshal(raw, &b); err != nil {
			return err
		}
		if b.Limit > 0 {
			reserve = b.Reserve
		}
	}
	_, err = tx.Exec("INSERT INTO planning_calls(id,work_id,scope_id,estimate,state,created_at) VALUES(?,?,?,?,'estimated',?)", id, work, scope, reserve, now())
	return err
}
func (s *Store) savePlanningUsage(id string, u *Usage) error {
	raw, err := json.Marshal(u)
	if err != nil {
		return err
	}
	_, err = s.db.Exec("UPDATE planning_calls SET usage=?,state='finished' WHERE id=?", raw, id)
	return err
}

func planningAncestors(p *PlanningState, id string) []*PlanningScope {
	out := []*PlanningScope{}
	for id != "" {
		scope, err := p.scope(id)
		if err != nil || len(out) > 20 {
			return nil
		}
		out = append(out, scope)
		id = scope.Parent
	}
	return out
}
func planningScopeTaskCount(w *Work, scope string) int {
	n := 0
	for _, t := range w.Tasks {
		for _, owner := range planningAncestors(w.Planning, t.ScopeID) {
			if owner.ID == scope {
				n++
				break
			}
		}
	}
	return n
}
func checkScopeActivation(p *PlanningState, id string) error {
	for _, scope := range planningAncestors(p, id) {
		limit := scope.ActivationLimit
		if limit == 0 {
			limit = p.MaxActivations
		}
		if scope.Activations >= limit {
			return planningError("budget_exhausted", fmt.Sprintf("budget d’activations du périmètre %s atteint", scope.ID))
		}
	}
	return nil
}
func checkScopeTask(w *Work, id string) error {
	for _, scope := range planningAncestors(w.Planning, id) {
		limit := scope.TaskLimit
		if limit == 0 {
			limit = w.Planning.MaxTasks
		}
		if planningScopeTaskCount(w, scope.ID) >= limit {
			return fmt.Errorf("budget de tâches du périmètre %s atteint", scope.ID)
		}
	}
	return nil
}
