package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
)

type AssistActionPlan struct {
	ID             string     `json:"id"`
	Turn           string     `json:"turn"`
	Step           int        `json:"step"`
	Work           string     `json:"work"`
	Revision       int        `json:"revision"`
	Action         PageAction `json:"action"`
	Effect         string     `json:"effect"`
	Path           string     `json:"path,omitempty"`
	Digest         string     `json:"digest,omitempty"`
	ProviderDigest string     `json:"provider_digest,omitempty"`
	Launch         *Launch    `json:"launch,omitempty"`
}
type AssistActionReceipt struct {
	ID      string `json:"id"`
	Turn    string `json:"turn"`
	Step    int    `json:"step"`
	Status  string `json:"status"`
	Message string `json:"message"`
}

func (s *Store) assistActionPlan(work, turnID string, index int) (AssistActionPlan, error) {
	var p AssistActionPlan
	t, e := s.assistTurn(turnID)
	if e != nil {
		return p, e
	}
	if t.WorkID != work || t.Imported || t.Status != "completed" || t.Refusal != nil || t.Answer == nil || index < 0 || index >= len(t.Answer.NextSteps) {
		return p, fmt.Errorf("Proposition IA indisponible ou hors travail")
	}
	if refusal := checkAnswer(t.Answer, t.Context, t.TemplateID); refusal != nil {
		return p, fmt.Errorf("Proposition non conforme : %s", refusal.Message)
	}
	ctx, e := s.pageContext(work, t.Coordinates)
	if e != nil {
		return p, e
	}
	if ctx.Hash != t.Context.Hash {
		return p, fmt.Errorf("La situation a changé : actualisez l’explication avant d’agir")
	}
	a, ok := ctx.action(t.Answer.NextSteps[index].ActionID)
	if !ok || !a.Available {
		return p, fmt.Errorf("Action non autorisée actuellement")
	}
	w, e := s.get(work)
	if e != nil {
		return p, e
	}
	p = AssistActionPlan{Turn: turnID, Step: index, Work: work, Revision: w.Revision, Action: a}
	switch a.Operation {
	case "task.submit":
		reports := s.taskReports(a.Target)
		if len(reports) != 1 {
			return p, fmt.Errorf("Il faut choisir un rapport : %d candidats détectés. Ouvrez les actions de la tâche", len(reports))
		}
		p.Path = reports[0]
		var id string
		e = s.db.QueryRow("SELECT id FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", work, a.Target).Scan(&id)
		if e == nil {
			agent, err := s.agent(id)
			if err != nil {
				return p, err
			}
			report, reason := s.provenReport(a.Target, agent.Started)
			if report == "" {
				return p, fmt.Errorf("Rapport à examiner : %s", reason)
			}
			p.Path = report
		} else if e != sql.ErrNoRows {
			return p, e
		}
		path, e := safeReport(s.root, p.Path)
		if e != nil {
			return p, e
		}
		raw, e := os.ReadFile(path)
		if e != nil {
			return p, e
		}
		if len(raw) == 0 {
			return p, fmt.Errorf("Rapport vide")
		}
		p.Digest = hash(raw)
		p.Effect = "Présenter le compte rendu pour examen. Sa validation reste à faire."
	case "task.start":
		task, e := w.task(a.Target)
		if e != nil {
			return p, e
		}
		profile := task.Profile
		if profile == nil {
			profile = w.Profile
		}
		if profile == nil {
			return p, fmt.Errorf("Configurer d’abord le fournisseur et l’espace de travail de cette tâche")
		}
		ps, e := s.providers()
		if e != nil {
			return p, e
		}
		provider, ok := ps.Providers[profile.Provider]
		if !ok {
			return p, fmt.Errorf("Fournisseur absent")
		}
		raw, _ := json.Marshal(provider)
		p.ProviderDigest = hash(raw)
		instruction := profile.Instruction
		if task.Profile == nil {
			instruction = "Mission : " + task.Title + "\nLivrable : " + task.Deliverable + "\nProchaine action : " + task.Next + "\nConsignes communes : " + instruction
		}
		p.Launch = &Launch{ProviderDigest: p.ProviderDigest, Schema: 1, Revision: w.Revision, TaskID: task.ID, Provider: profile.Provider, Role: profile.Role, Workspace: profile.Workspace, Instruction: instruction, Level: profile.Level, Timeout: profile.Timeout, Capture: profile.Capture, Limits: profile.Limits}
		p.Effect = "Lancer « " + task.Title + " » avec " + profile.Provider + ". Les réglages de cette tâche sont conservés."
	case "agent.stop":
		p.Effect = "Demander à l’agent de s’arrêter. Sa fin sera confirmée dans la tâche."
	case "agent.reconcile":
		p.Effect = "Vérifier si l’agent a terminé et mettre son état à jour. La mission pourra ensuite reprendre."
	default:
		return p, fmt.Errorf("Cette action nécessite le formulaire habituel : %s", a.Label)
	}
	raw, _ := json.Marshal(p)
	p.ID = "assist-act-" + hash(raw)[:32]
	if p.Launch != nil {
		p.Launch.EventID = p.ID
		if _, _, e = s.prepareLaunch(work, *p.Launch, true); e != nil {
			return p, e
		}
	}
	return p, nil
}
func (s *Store) assistActionReceipt(work, id string) (AssistActionReceipt, error) {
	var r AssistActionReceipt
	var raw string
	e := s.db.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='assist-action' AND json_extract(message,'$.id')=? ORDER BY seq DESC LIMIT 1", work, id).Scan(&raw)
	if e != nil {
		return r, e
	}
	e = json.Unmarshal([]byte(raw), &r)
	return r, e
}
func (s *Store) applyAssistAction(work, turn string, step int, id string) (AssistActionReceipt, error) {
	if !safeName(id) {
		return AssistActionReceipt{}, fmt.Errorf("Identifiant de proposition invalide")
	}
	// A receipt is checked before freshness: execution changes its own context.
	if old, e := s.assistActionReceipt(work, id); e == nil {
		if old.Turn != turn || old.Step != step {
			return old, fmt.Errorf("Proposition différente")
		}
		return old, nil
	} else if e != sql.ErrNoRows {
		return old, e
	}
	p, e := s.assistActionPlan(work, turn, step)
	if e != nil {
		return AssistActionReceipt{}, e
	}
	if p.ID != id {
		return AssistActionReceipt{}, fmt.Errorf("Les paramètres ont changé : examinez la proposition actualisée")
	}
	r := AssistActionReceipt{ID: id, Turn: turn, Step: step, Status: "pending", Message: "Application engagée. Si cette réponse persiste, vérifiez l’état de la tâche ; aucune répétition automatique."}
	raw, _ := json.Marshal(r)
	// Atomic durable claim: at most one executor, including across web servers.
	result, e := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) SELECT ?,?,'assist-action',? WHERE NOT EXISTS (SELECT 1 FROM cockpit_events WHERE work_id=? AND kind='assist-action' AND json_extract(message,'$.id')=?)", work, now(), string(raw), work, id)
	if e != nil {
		return r, e
	}
	n, _ := result.RowsAffected()
	if n == 0 {
		return s.assistActionReceipt(work, id)
	}
	seq, _ := result.LastInsertId()
	switch p.Action.Operation {
	case "task.submit":
		e = s.submitReportVerified(work, p.Action.Target, p.Path, p.Revision, "", id, p.Digest)
	case "task.start":
		var a Agent
		var created bool
		a, created, e = s.prepare(work, *p.Launch)
		if e == nil && created {
			e = s.spawnAgent(a)
		}
	case "agent.stop", "agent.reconcile":
		kind := "stop"
		if p.Action.Operation == "agent.reconcile" {
			kind = "reconcile"
		}
		_, e = s.webAction(webRequest{Kind: kind, Work: work, Agent: p.Action.Target, Revision: p.Revision})
	}
	r.Status = "done"
	r.Message = p.Effect + " Action enregistrée ; état relu dans le pilotage."
	if e != nil {
		r.Status = "attention"
		r.Message = "L’action n’a pas été confirmée : " + e.Error() + ". Vérifiez l’état avant toute nouvelle demande."
	}
	raw, _ = json.Marshal(r)
	if _, saveErr := s.db.Exec("UPDATE cockpit_events SET message=? WHERE seq=?", string(raw), seq); saveErr != nil {
		return r, saveErr
	}
	return r, nil
}
