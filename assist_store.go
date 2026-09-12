package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"strings"
	"time"
)

const assistMigration = `BEGIN;
CREATE TABLE assist_turns(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), created_at TEXT NOT NULL, status TEXT NOT NULL, body BLOB NOT NULL);
CREATE INDEX assist_turns_work ON assist_turns(work_id, created_at);
CREATE UNIQUE INDEX assist_one_active ON assist_turns(work_id) WHERE status IN ('pending','running');
CREATE TABLE assist_previews(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), created_at TEXT NOT NULL, body BLOB NOT NULL);
CREATE TABLE assist_reservations(turn_id TEXT PRIMARY KEY REFERENCES assist_turns(id), work_id TEXT NOT NULL REFERENCES works(id), amount REAL NOT NULL, state TEXT NOT NULL);
PRAGMA user_version=4;
COMMIT;`

const maxAssistQuestion = 2000
const assistTurnHistory = 40

// A page assistant turn is a single bounded question. It is deliberately not a
// Work task: a question must not create a unit of work, must not change the
// validation state of the work, and must be answerable while an agent holds the
// workspace. Everything else is reused — provider catalogue, run limits, budget
// reserve policy, local persistence.
type AssistRequest struct {
	Level       string          `json:"level,omitempty"`
	EventID     string          `json:"event_id"`
	Revision    int             `json:"expected_revision"`
	Coordinates PageCoordinates `json:"coordinates"`
	TemplateID  string          `json:"template_id"`
	Question    string          `json:"question"`
	Provider    string          `json:"provider"`
	ContextHash string          `json:"context_hash,omitempty"`
}

type AssistPreview struct {
	Turn    AssistTurn `json:"turn"`
	Prompt  string     `json:"prompt"`
	Bytes   int        `json:"prompt_bytes"`
	Warning string     `json:"injection_warning,omitempty"`
}

func (s *Store) assistTurn(id string) (AssistTurn, error) {
	var t AssistTurn
	var body []byte
	e := s.db.QueryRow("SELECT body FROM assist_turns WHERE id=?", id).Scan(&body)
	if e != nil {
		return t, e
	}
	return t, json.Unmarshal(body, &t)
}

func (s *Store) assistTurns(work string) ([]AssistTurn, error) {
	rows, e := s.db.Query("SELECT body FROM (SELECT rowid,body FROM assist_turns WHERE work_id=? ORDER BY rowid DESC LIMIT ?) ORDER BY rowid", work, assistTurnHistory)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []AssistTurn{}
	for rows.Next() {
		var body []byte
		var t AssistTurn
		if e = rows.Scan(&body); e != nil {
			return nil, e
		}
		if e = json.Unmarshal(body, &t); e != nil {
			return nil, e
		}
		out = append(out, t)
	}
	return out, rows.Err()
}

func (s *Store) saveAssistTurn(t AssistTurn) error {
	body, e := json.Marshal(t)
	if e != nil {
		return e
	}
	_, e = s.db.Exec("UPDATE assist_turns SET status=?,body=? WHERE id=? AND status IN ('pending','running')", t.Status, body, t.ID)
	return e
}

// Build the exact turn that would be sent, without recording anything. The
// operator reads this before confirming; the hash binds the confirmation to it.
func (s *Store) assistPreview(work string, r AssistRequest) (AssistPreview, error) {
	var p AssistPreview
	tpl, ok := assistTemplate(r.TemplateID)
	if !ok {
		return p, fmt.Errorf("Gabarit inconnu : choisir un gabarit proposé.")
	}
	question := strings.TrimSpace(r.Question)
	if question == "" {
		question = tpl.Question
	}
	if len(question) > maxAssistQuestion {
		return p, fmt.Errorf("Question limitée à %d octets.", maxAssistQuestion)
	}
	if !safeName(r.Provider) {
		return p, fmt.Errorf("Fournisseur invalide.")
	}
	providers, e := s.providers()
	if e != nil {
		return p, e
	}
	if _, ok := providers.Providers[r.Provider]; !ok {
		return p, fmt.Errorf("Fournisseur non configuré.")
	}
	provider := providers.Providers[r.Provider]
	_, route, e := resolveModel(provider, r.Level, "page")
	if e != nil {
		return p, e
	}
	timeout := provider.AssistantTimeout
	if timeout == 0 {
		timeout = 300
	}
	if timeout < 1 || timeout > 300 {
		return p, fmt.Errorf("Délai d’assistance configurable : 1 à 300 secondes.")
	}
	providerRaw, _ := json.Marshal(provider)
	w, e := s.get(work)
	if e != nil {
		return p, e
	}
	if r.Revision != 0 && r.Revision != w.Revision {
		return p, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis relire le contexte.", Retryable: true}
	}
	ctx, e := s.pageContext(work, r.Coordinates)
	if e != nil {
		return p, e
	}
	prompt, e := buildAssistPrompt(ctx, tpl, question)
	if e != nil {
		return p, e
	}
	p.Turn = AssistTurn{ModelRoute: route, TimeoutSeconds: timeout, ProviderDigest: hash(providerRaw), Coordinates: r.Coordinates, RequestHash: assistRequestHash(r), WorkID: work, Status: "preview", CreatedAt: ctx.CapturedAt, Question: question,
		TemplateID: tpl.ID, PromptVer: assistPromptVersion, Provider: r.Provider, Context: ctx, Prompt: prompt}
	p.Prompt = prompt
	p.Bytes = len(prompt)
	for _, f := range ctx.Facts {
		if f.Kind == "texte_non_fiable" && looksLikeInjection(f.Value) {
			p.Warning = "Le contexte contient du texte imitant une consigne (fait " + f.ID + "). Il est transmis comme donnée observée et ne doit pas être suivi."
			break
		}
	}
	if safeName(r.EventID) {
		raw, _ := json.Marshal(p)
		result, err := s.db.Exec("INSERT INTO assist_previews(id,work_id,created_at,body) VALUES(?,?,?,?) ON CONFLICT(id) DO UPDATE SET body=excluded.body,created_at=excluded.created_at WHERE work_id=excluded.work_id", r.EventID, work, now(), raw)
		if err != nil {
			return p, err
		}
		n, _ := result.RowsAffected()
		if n != 1 {
			return p, fmt.Errorf("Identifiant d’aperçu déjà utilisé par un autre travail.")
		}
		_, _ = s.db.Exec("DELETE FROM assist_previews WHERE created_at < datetime('now','-1 day')")
		_, _ = s.db.Exec("DELETE FROM assist_previews WHERE work_id=? AND id NOT IN (SELECT id FROM assist_previews WHERE work_id=? ORDER BY created_at DESC,rowid DESC LIMIT 100)", work, work)
	}
	return p, nil
}

// Record the turn after an explicit confirmation bound to the previewed context.
func (s *Store) assistAsk(work string, r AssistRequest) (AssistTurn, error) {
	if !safeName(r.EventID) {
		return AssistTurn{}, fmt.Errorf("event_id invalide.")
	}
	if existing, e := s.assistTurn(r.EventID); e == nil {
		if existing.WorkID != work || existing.RequestHash != assistRequestHash(r) {
			return AssistTurn{}, fmt.Errorf("event_id déjà utilisé pour une autre demande.")
		}
		return existing, nil
	} else if e != sql.ErrNoRows {
		return AssistTurn{}, e
	}
	if r.ContextHash == "" {
		return AssistTurn{}, fmt.Errorf("Examiner le contexte de la page avant envoi.")
	}
	var preview AssistPreview
	var previewRaw []byte
	e := s.db.QueryRow("SELECT body FROM assist_previews WHERE id=? AND work_id=?", r.EventID, work).Scan(&previewRaw)
	if e != nil {
		return AssistTurn{}, fmt.Errorf("Examiner le contexte de cette demande avant envoi.")
	}
	if e = json.Unmarshal(previewRaw, &preview); e != nil {
		return AssistTurn{}, e
	}
	if preview.Turn.RequestHash != assistRequestHash(r) {
		return AssistTurn{}, fmt.Errorf("Demande modifiée depuis l’aperçu : examiner à nouveau le contexte.")
	}
	ps, e := s.providers()
	if e != nil {
		return AssistTurn{}, e
	}
	providerRaw, _ := json.Marshal(ps.Providers[r.Provider])
	if hash(providerRaw) != preview.Turn.ProviderDigest {
		return AssistTurn{}, fmt.Errorf("Fournisseur modifié depuis l’aperçu ; examiner à nouveau la demande.")
	}
	current, e := s.pageContext(work, r.Coordinates)
	if e != nil {
		return AssistTurn{}, e
	}
	if current.Hash != r.ContextHash || preview.Turn.Context.Hash != r.ContextHash {
		return AssistTurn{}, fmt.Errorf("Contexte modifié depuis l’aperçu : relire le contexte avant envoi.")
	}
	turns, e := s.assistTurns(work)
	if e != nil {
		return AssistTurn{}, e
	}
	for _, t := range turns {
		if t.active() {
			return AssistTurn{}, fmt.Errorf("Une question est déjà en cours pour ce travail ; attendre sa réponse.")
		}
	}
	turn := preview.Turn
	turn.ID = r.EventID
	turn.Status = "pending"
	turn.CreatedAt = now()
	body, _ := json.Marshal(turn)
	tx, e := s.db.Begin()
	if e != nil {
		return AssistTurn{}, e
	}
	defer tx.Rollback()
	if _, e = tx.Exec("INSERT INTO assist_turns(id,work_id,created_at,status,body) VALUES(?,?,?,?,?)", turn.ID, work, turn.CreatedAt, turn.Status, body); e != nil {
		return AssistTurn{}, e
	}
	if e = reserveAssistBudget(tx, work, turn.ID); e != nil {
		return AssistTurn{}, e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, turn.CreatedAt, "assistant", "Question d’assistance sur la page "+turn.Context.PageID+" ; gabarit "+turn.TemplateID); e != nil {
		return AssistTurn{}, e
	}
	if e = tx.Commit(); e != nil {
		return AssistTurn{}, e
	}
	return turn, nil
}

// Same estimate policy as an agent launch: reserve before starting, settle after.
func reserveAssistBudget(tx *sql.Tx, work, turn string) error {
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
	if committed+assist+b.Reserve > b.Limit {
		return &CommandError{Code: "budget_exhausted", Message: "Budget estimatif insuffisant : la question d’assistance n’est pas envoyée ; examiner le budget.", Retryable: false}
	}
	_, e = tx.Exec("INSERT INTO assist_reservations(turn_id,work_id,amount,state) VALUES(?,?,?,'reserved')", turn, work, b.Reserve)
	return e
}

func (s *Store) settleAssistBudget(turn, state string) error {
	_, e := s.db.Exec("UPDATE assist_reservations SET state=? WHERE turn_id=? AND state='reserved'", state, turn)
	return e
}

// A turn left pending by a stopped cockpit is interrupted, never silently
// retried: the operator decides whether to ask again.
func (s *Store) assistReconcile() error { return s.assistReconcileGrace(0) }
func (s *Store) assistReconcileGrace(grace time.Duration) error {
	rows, e := s.db.Query("SELECT id FROM assist_turns WHERE status IN ('pending','running')")
	if e != nil {
		return e
	}
	ids := []string{}
	for rows.Next() {
		var id string
		if e = rows.Scan(&id); e != nil {
			rows.Close()
			return e
		}
		ids = append(ids, id)
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return e
	}
	for _, id := range ids {
		t, err := s.assistTurn(id)
		if err != nil {
			continue
		}
		if !t.active() {
			continue
		}
		if t.SupervisorPID == 0 && grace > 0 {
			created, e := time.Parse(time.RFC3339Nano, t.CreatedAt)
			if e == nil && time.Since(created) < grace {
				continue
			}
		}
		if t.SupervisorPID > 0 && t.Host == hostIdentity() && t.SupervisorStamp != "" && processStamp(t.SupervisorPID) == t.SupervisorStamp {
			continue
		}
		stopVerifiedAssist(t)
		t.Status = "interrupted"
		t.EndedAt = now()
		t.Refusal = refuse("interrupted", refusalService, "Supervision interrompue ou absente ; aucune réponse enregistrée et aucune relance automatique.", "")
		if err = s.saveAssistTurn(t); err != nil {
			return err
		}
		_ = s.settleAssistBudget(id, "estimated")
	}
	return nil
}

// Fold the provider reply into a validated answer or an explicit refusal.
func (s *Store) settleAssistTurn(turn AssistTurn, reply string, usage *Usage, failure *AssistRefusal) error {
	turn.EndedAt = now()
	turn.RawBytes = len(reply)
	if usage != nil {
		turn.Usage = &AssistUsageNote{Provider: turn.Provider, Usage: usage, Note: "Jetons déclarés par le fournisseur ; aucun coût facturé n’en est déduit."}
	}
	switch {
	case failure != nil:
		turn.Status = "failed"
		turn.Refusal = failure
	default:
		answer, refusal, repaired := validateAssistantReply(reply, turn.Context, turn.TemplateID)
		turn.Repaired = repaired
		if refusal != nil {
			turn.Status = "refused"
			turn.Refusal = refusal
		} else {
			turn.Status = "answered"
			turn.Answer = answer
			turn.AnswerBytes = answerBytes(answer)
		}
	}
	if e := s.saveAssistTurn(turn); e != nil {
		return e
	}
	state := "estimated"
	return s.settleAssistBudget(turn.ID, state)
}

func assistRequestHash(r AssistRequest) string {
	r.ContextHash = ""
	raw, _ := json.Marshal(r)
	return hash(raw)
}

func (s *Store) cancelAssist(work, id string) error {
	t, e := s.assistTurn(id)
	if e != nil {
		return e
	}
	if t.WorkID != work {
		return fmt.Errorf("Question hors travail.")
	}
	if !t.active() {
		return nil
	}
	t.Status = "interrupted"
	t.EndedAt = now()
	t.Refusal = refuse("cancelled", refusalService, "Question annulée par l’opérateur ; aucune relance automatique.", "")
	if e = s.saveAssistTurn(t); e != nil {
		return e
	}
	return s.settleAssistBudget(id, "estimated")
}
func (s *Store) assistCurrentTurns(work string) ([]AssistTurn, error) {
	s = s.readScope()
	if e := s.assistReconcileGrace(10 * time.Second); e != nil {
		return nil, e
	}
	turns, e := s.assistTurns(work)
	if e != nil {
		return nil, e
	}
	w, e := s.get(work)
	if e != nil {
		return nil, e
	}
	cache := map[string]PageContext{}
	for i := range turns {
		if turns[i].Imported || turns[i].Context.Revision != w.Revision {
			turns[i].Stale = true
			continue
		}
		keyBytes, _ := json.Marshal(turns[i].Coordinates)
		key := string(keyBytes)
		c, ok := cache[key]
		var err error
		if !ok {
			c, err = s.pageContext(work, turns[i].Coordinates)
			if err == nil {
				cache[key] = c
			}
		}
		turns[i].Stale = turns[i].Imported || err != nil || c.Hash != turns[i].Context.Hash
	}
	return turns, nil
}

type assistQuerier interface {
	Query(string, ...any) (*sql.Rows, error)
}

func allAssistTurns(q assistQuerier, work string) ([]AssistTurn, error) {
	rows, e := q.Query("SELECT body FROM assist_turns WHERE work_id=? ORDER BY rowid", work)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []AssistTurn{}
	size := 0
	for rows.Next() {
		var raw []byte
		var t AssistTurn
		if e = rows.Scan(&raw); e != nil {
			return nil, e
		}
		size += len(raw)
		if size > 64<<20 {
			return nil, fmt.Errorf("Historique supérieur à 64 Mio ; utiliser une sauvegarde de la base.")
		}
		if e = json.Unmarshal(raw, &t); e != nil {
			return nil, e
		}
		out = append(out, t)
	}
	return out, rows.Err()
}
