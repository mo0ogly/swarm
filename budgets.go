package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"math"
	"strings"
	"time"
)

type Budget struct {
	Limit     float64 `json:"limit_usd"`
	Reserve   float64 `json:"reserve_per_launch_usd"`
	Source    string  `json:"estimate_source"`
	PriceDate string  `json:"reference_date"`
	Updated   string  `json:"updated"`
	Actor     string  `json:"actor"`
}
type BudgetView struct {
	ActualCostScope string   `json:"actual_cost_scope"`
	Budget          Budget   `json:"budget"`
	Reserved        float64  `json:"reserved_usd"`
	Estimated       float64  `json:"committed_estimate_usd"`
	Remaining       float64  `json:"remaining_usd"`
	Warning         bool     `json:"warning"`
	ActualCost      *float64 `json:"actual_cost_usd"`
	Policy          string   `json:"policy"`
}

func (s *Store) budget(work string) (BudgetView, error) {
	v := BudgetView{ActualCostScope: "Coûts rapportés par les agents seulement ; assistance et préparation exclues. Les estimations couvrent les trois activités.", Policy: "Budget estimatif non garanti. Réservations atomiques avant départ ; les agents actifs continuent. Le réservé n'est pas une dépense : le coût rapporté est indiqué à part."}
	var raw []byte
	e := s.db.QueryRow("SELECT body FROM budgets WHERE work_id=?", work).Scan(&raw)
	if e != nil && e != sql.ErrNoRows {
		return v, e
	}
	if e == nil {
		if e = json.Unmarshal(raw, &v.Budget); e != nil {
			return v, e
		}
	}
	if e = s.db.QueryRow("SELECT coalesce(sum(CASE WHEN state='reserved' THEN amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN amount ELSE 0 END),0) FROM reservations WHERE work_id=?", work).Scan(&v.Reserved, &v.Estimated); e != nil {
		return v, e
	}
	// Page-assistant questions consume the same estimate envelope as launches.
	var assistReserved, assistEstimated float64
	if e = s.db.QueryRow("SELECT coalesce(sum(CASE WHEN state='reserved' THEN amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN amount ELSE 0 END),0) FROM assist_reservations WHERE work_id=?", work).Scan(&assistReserved, &assistEstimated); e != nil {
		return v, e
	}
	var prepReserved, prepEstimated float64
	if e = s.db.QueryRow("SELECT coalesce(sum(CASE WHEN state='reserved' THEN work_amount ELSE 0 END),0),coalesce(sum(CASE WHEN state='estimated' THEN work_amount ELSE 0 END),0) FROM preparation_reservations WHERE work_id=?", work).Scan(&prepReserved, &prepEstimated); e != nil {
		return v, e
	}
	v.Reserved += prepReserved
	v.Estimated += prepEstimated
	v.Reserved += assistReserved
	v.Estimated += assistEstimated
	// Le coût réel vient des fournisseurs, pas des réservations. Il reste nil
	// tant qu'aucune tentative n'a rapporté : un zéro passerait pour une mesure
	// et laisserait croire que rien n'a été dépensé.
	if summary, err := s.costSummary(work); err == nil && summary.WithCost > 0 {
		reported := summary.Reported
		v.ActualCost = &reported
	}
	v.Remaining = max(0, v.Budget.Limit-v.Reserved-v.Estimated)
	v.Warning = v.Budget.Limit > 0 && (v.Reserved+v.Estimated) >= .8*v.Budget.Limit
	return v, nil
}
func validateBudget(b Budget) error {
	if math.IsNaN(b.Limit) || math.IsInf(b.Limit, 0) || math.IsNaN(b.Reserve) || math.IsInf(b.Reserve, 0) || b.Limit < 0 || b.Reserve < 0 {
		return fmt.Errorf("montants finis et positifs requis")
	}
	if b.Limit > 0 {
		if b.Reserve <= 0 || strings.TrimSpace(b.Source) == "" {
			return fmt.Errorf("réservation et source de l’estimation requises")
		}
		if _, e := time.Parse("2006-01-02", b.PriceDate); e != nil {
			return fmt.Errorf("date de référence : AAAA-MM-JJ")
		}
	}
	return nil
}
func (s *Store) setBudget(work string, b Budget) error {
	if e := validateBudget(b); e != nil {
		return e
	}
	b.Updated = now()
	b.Actor = operatorIdentity()
	raw, _ := json.Marshal(b)
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	if _, e = tx.Exec("INSERT INTO budgets(work_id,body) VALUES(?,?) ON CONFLICT(work_id) DO UPDATE SET body=excluded.body", work, raw); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, b.Updated, "budget", fmt.Sprintf("%s : limite estimative %.2f USD ; réservation %.2f USD", b.Actor, b.Limit, b.Reserve)); e != nil {
		return e
	}
	return tx.Commit()
}
func reserveBudget(tx *sql.Tx, work, agent string) error {
	var raw []byte
	e := tx.QueryRow("SELECT body FROM budgets WHERE work_id=?", work).Scan(&raw)
	if e == sql.ErrNoRows {
		return nil
	}
	if e != nil {
		return e
	}
	var b Budget
	if e = json.Unmarshal(raw, &b); e != nil {
		return e
	}
	if b.Limit == 0 {
		return nil
	}
	var committed float64
	if e = tx.QueryRow("SELECT coalesce(sum(amount),0) FROM reservations WHERE work_id=? AND state IN ('reserved','estimated')", work).Scan(&committed); e != nil {
		return e
	}
	var assist float64
	if e = tx.QueryRow("SELECT coalesce(sum(amount),0) FROM assist_reservations WHERE work_id=? AND state IN ('reserved','estimated')", work).Scan(&assist); e != nil {
		return e
	}
	prep, e := preparationWorkCommitted(tx, work)
	if e != nil {
		return e
	}
	committed += assist + prep
	if committed+b.Reserve > b.Limit {
		return &CommandError{Code: "budget_exhausted", Message: "Budget estimatif insuffisant : nouveaux départs suspendus ; examiner le budget."}
	}
	_, e = tx.Exec("INSERT INTO reservations(agent_id,work_id,amount,state) VALUES(?,?,?,'reserved')", agent, work, b.Reserve)
	return e
}

type Usage struct {
	CacheRead     *int64   `json:"cache_read_input_tokens,omitempty"`
	CacheCreation *int64   `json:"cache_creation_input_tokens,omitempty"`
	CachedInput   *int64   `json:"cached_input_tokens,omitempty"`
	ReportedCost  *float64 `json:"provider_reported_cost_usd,omitempty"`
	Input         int64    `json:"input_tokens"`
	Output        int64    `json:"output_tokens"`
	Source        string   `json:"source"`
	Scope         string   `json:"scope"`
	At            string   `json:"at"`
}

func providerUsage(data map[string]any) *Usage {
	kind, _ := data["type"].(string)
	if kind == "" && data["subtype"] == "success" && data["terminal_reason"] == "completed" {
		if session, ok := data["session_id"].(string); ok && session != "" {
			kind = "terminal_result"
		}
	}
	if kind != "result" && kind != "turn.completed" && kind != "terminal_result" {
		return nil
	}
	m, ok := data["usage"].(map[string]any)
	if !ok {
		return nil
	}
	parse := func(v any) (int64, bool) {
		n, ok := v.(float64)
		return int64(n), ok && !math.IsNaN(n) && !math.IsInf(n, 0) && n >= 0 && n <= 9e15 && n == math.Trunc(n)
	}
	in, a := parse(m["input_tokens"])
	out, b := parse(m["output_tokens"])
	if !a || !b {
		return nil
	}
	u := &Usage{Input: in, Output: out, Source: "événement fournisseur " + kind, Scope: "champs bruts du dernier événement ; caches séparés, sans somme ni cumul de session", At: now()}
	for key, dest := range map[string]**int64{"cache_read_input_tokens": &u.CacheRead, "cache_creation_input_tokens": &u.CacheCreation, "cached_input_tokens": &u.CachedInput} {
		if n, ok := parse(m[key]); ok {
			value := n
			*dest = &value
		}
	}
	if n, ok := data["total_cost_usd"].(float64); ok && !math.IsNaN(n) && !math.IsInf(n, 0) && n >= 0 {
		u.ReportedCost = &n
	}
	return u
}
