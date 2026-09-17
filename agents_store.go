package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const agentMigration = `BEGIN;
CREATE TABLE agents(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL,
 cwd TEXT NOT NULL, status TEXT NOT NULL, desired TEXT NOT NULL DEFAULT '', body BLOB NOT NULL, request BLOB NOT NULL);
CREATE UNIQUE INDEX agent_task_active ON agents(work_id,task_id) WHERE status IN ('queued','starting','running','stopping');
CREATE UNIQUE INDEX agent_cwd_active ON agents(cwd) WHERE status IN ('queued','starting','running','stopping');
CREATE TABLE agent_logs(seq INTEGER PRIMARY KEY AUTOINCREMENT, agent_id TEXT NOT NULL REFERENCES agents(id), at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);
CREATE INDEX agent_log_cursor ON agent_logs(agent_id,seq);
CREATE TABLE cockpit_events(seq INTEGER PRIMARY KEY AUTOINCREMENT, work_id TEXT NOT NULL REFERENCES works(id), at TEXT NOT NULL, kind TEXT NOT NULL, message TEXT NOT NULL);
CREATE TABLE cockpit_controls(work_id TEXT PRIMARY KEY REFERENCES works(id), paused INTEGER NOT NULL DEFAULT 0);
CREATE TABLE cockpit_tasks(work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL, priority INTEGER NOT NULL DEFAULT 0, PRIMARY KEY(work_id,task_id));
PRAGMA user_version=2;
COMMIT;`

type Launch struct {
	Mode            string        `json:"mode,omitempty"`
	Limits          *RunLimits    `json:"limits,omitempty"`
	Level           string        `json:"level,omitempty"`
	ModelPolicyHash string        `json:"model_policy_hash,omitempty"`
	PlanBriefHash   string        `json:"plan_brief_hash,omitempty"`
	References      []DialogueRef `json:"references,omitempty"`
	ContextHash     string        `json:"context_hash,omitempty"`
	Brainstorm      bool          `json:"brainstorm,omitempty"`
	Schema          int           `json:"schema_version"`
	EventID         string        `json:"event_id"`
	Revision        int           `json:"expected_revision"`
	TaskID          string        `json:"task_id"`
	Origin          string        `json:"origin,omitempty"`
	Provider        string        `json:"provider"`
	Workspace       string        `json:"workspace"`
	Instruction     string        `json:"instruction"`
	Role            string        `json:"role"`
	Parent          string        `json:"parent,omitempty"`
	Previous        string        `json:"previous,omitempty"`
	Timeout         int           `json:"timeout_seconds"`
	Capture         bool          `json:"capture_output"`
}
type AgentProgress struct {
	Action       string `json:"action,omitempty"`
	Detail       string `json:"detail,omitempty"`
	ToolCalls    int    `json:"tool_calls"`
	ToolResults  int    `json:"tool_results"`
	PendingTools int    `json:"pending_tools"`
	LastTool     string `json:"last_tool,omitempty"`
	LastResult   string `json:"last_result_at,omitempty"`
	Degraded     string `json:"degraded,omitempty"`
}
type Agent struct {
	Mode              string           `json:"mode,omitempty"`
	UnregisteredClaim bool             `json:"-"`
	ModelRoute        *ModelRoute      `json:"model_route,omitempty"`
	Context           *ContextManifest `json:"context,omitempty"`
	Brainstorm        bool             `json:"brainstorm,omitempty"`
	Reply             string           `json:"reply,omitempty"`
	Usage             *Usage           `json:"usage,omitempty"`
	Progress          AgentProgress    `json:"progress"`
	Limits            RunLimits        `json:"limits"`
	ID                string           `json:"id"`
	WorkID            string           `json:"work_id"`
	TaskID            string           `json:"task_id"`
	Attempt           string           `json:"attempt_id"`
	Origin            string           `json:"origin,omitempty"`
	Relay             string           `json:"relay,omitempty"`
	StopKind          string           `json:"stop_kind,omitempty"`
	Provider          string           `json:"provider"`
	Role              string           `json:"role"`
	Parent            string           `json:"parent,omitempty"`
	Previous          string           `json:"previous,omitempty"`
	CWD               string           `json:"workspace"`
	Status            string           `json:"status"`
	Desired           string           `json:"desired,omitempty"`
	Activity          string           `json:"activity"`
	Started           string           `json:"started"`
	Heartbeat         string           `json:"heartbeat,omitempty"`
	Ended             string           `json:"ended,omitempty"`
	ExitCode          *int             `json:"exit_code,omitempty"`
	Supervisor        int              `json:"supervisor_pid,omitempty"`
	SupervisorStamp   string           `json:"supervisor_identity,omitempty"`
	Child             int              `json:"child_pid,omitempty"`
	ChildStamp        string           `json:"child_identity,omitempty"`
	Host              string           `json:"host"`
	Timeout           int              `json:"timeout_seconds"`
	Capture           bool             `json:"capture_output"`
	Prompt            string           `json:"prompt"`
	Command           string           `json:"command"`
	Args              []string         `json:"args"`
	Env               []string         `json:"env_allow"`
}
type AgentLog struct {
	Seq     int64  `json:"seq"`
	AgentID string `json:"agent_id"`
	At      string `json:"at"`
	Kind    string `json:"kind"`
	Message string `json:"message"`
}
type Provider struct {
	InteractiveArgs  []string     `json:"interactive_args,omitempty"`
	ModelPolicy      *ModelPolicy `json:"model_policy,omitempty"`
	AssistantTimeout int          `json:"assistant_timeout_seconds,omitempty"`
	Limits           RunLimits    `json:"limits,omitempty"`
	Command          string       `json:"command"`
	Args             []string     `json:"args"`
	Env              []string     `json:"env_allow"`
}
type Providers struct {
	Schema    int                 `json:"schema_version"`
	Providers map[string]Provider `json:"providers"`
}

func (s *Store) providers() (Providers, error) {
	var p Providers
	path, e := localFile(s.root, ".swarm/providers.json")
	if e != nil {
		return p, fmt.Errorf("configurer les fournisseurs : swarm providers init (%w)", e)
	}
	b, e := os.ReadFile(path)
	if e != nil {
		return p, e
	}
	if len(b) > 65536 {
		return p, fmt.Errorf("configuration trop grande")
	}
	e = strict(b, &p)
	if e != nil {
		return p, e
	}
	if p.Schema != 1 {
		return p, fmt.Errorf("schema fournisseurs inconnu")
	}
	return p, nil
}

// The SQL claim is authoritative; legacy bodies may still say queued.
func decodeAgentRow(body []byte, status, desired string) (Agent, error) {
	var a Agent
	if err := json.Unmarshal(body, &a); err != nil {
		return a, err
	}
	a.UnregisteredClaim = status == "starting" && a.Status == "queued" && a.Supervisor == 0 && a.Child == 0 && a.Heartbeat == ""
	a.Status = status
	a.Desired = desired
	return a, nil
}
func (s *Store) agent(id string) (Agent, error) {
	var body []byte
	var status, desired string
	if err := s.db.QueryRow("SELECT body,status,desired FROM agents WHERE id=?", id).Scan(&body, &status, &desired); err != nil {
		return Agent{}, err
	}
	return decodeAgentRow(body, status, desired)
}
func (s *Store) agents(work string) ([]Agent, error) {
	rows, err := s.db.Query("SELECT body,status,desired FROM agents WHERE work_id=? ORDER BY rowid DESC LIMIT 200", work)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	out := []Agent{}
	for rows.Next() {
		var body []byte
		var status, desired string
		if err = rows.Scan(&body, &status, &desired); err != nil {
			return nil, err
		}
		a, e := decodeAgentRow(body, status, desired)
		if e != nil {
			return nil, e
		}
		out = append(out, a)
	}
	return out, rows.Err()
}
func (s *Store) logs(id string, after int64) ([]AgentLog, error) {
	query := "SELECT seq,agent_id,at,kind,message FROM (SELECT * FROM agent_logs WHERE agent_id=? AND seq>? ORDER BY seq DESC LIMIT 200) ORDER BY seq"
	if after != 0 {
		query = "SELECT seq,agent_id,at,kind,message FROM agent_logs WHERE agent_id=? AND seq>? ORDER BY seq LIMIT 200"
		if after < 0 {
			after = 0
		}
	}
	rows, e := s.db.Query(query, id, after)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []AgentLog{}
	for rows.Next() {
		var l AgentLog
		if e = rows.Scan(&l.Seq, &l.AgentID, &l.At, &l.Kind, &l.Message); e != nil {
			return nil, e
		}
		out = append(out, l)
	}
	return out, rows.Err()
}
func boundedLogMessage(message string) string {
	message = terminalText(message)
	if len(message) > 2048 {
		message = message[:2048] + " [tronqué]"
	}
	return message
}
func (s *Store) log(id, kind, message string) error {
	return s.logBatch(id, []AgentLog{{AgentID: id, At: now(), Kind: kind, Message: boundedLogMessage(message)}})
}
func (s *Store) logBatch(id string, logs []AgentLog) error {
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	for _, entry := range logs {
		if _, e = tx.Exec("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?)", id, entry.At, entry.Kind, entry.Message); e != nil {
			return e
		}
	}
	if _, e = tx.Exec("DELETE FROM agent_logs WHERE agent_id=? AND seq NOT IN (SELECT seq FROM agent_logs WHERE agent_id=? ORDER BY seq DESC LIMIT 2000)", id, id); e != nil {
		return e
	}
	return tx.Commit()
}

func activeAgent(a Agent) bool {
	return a.Status == "queued" || a.Status == "starting" || a.Status == "running" || a.Status == "stopping"
}
func observedAgent(a Agent) string {
	if !activeAgent(a) {
		return a.Status
	}
	if a.Host != hostIdentity() {
		return "unknown/other-host"
	}
	if a.Supervisor > 0 && processStamp(a.Supervisor) != a.SupervisorStamp {
		return "unknown/supervisor-lost"
	}
	if a.Heartbeat == "" {
		return a.Status + "/unconfirmed"
	}
	t, e := time.Parse(time.RFC3339Nano, a.Heartbeat)
	if e != nil || time.Since(t) > 10*time.Second {
		return "unknown/no-heartbeat"
	}
	return a.Status
}
func (s *Store) saveAgent(a Agent) error {
	// Only the supervisor writes body; controls live separately to prevent lost stop requests.
	b, _ := json.Marshal(a)
	_, e := s.db.Exec("UPDATE agents SET body=?,status=? WHERE id=?", b, a.Status, a.ID)
	return e
}
func (s *Store) desired(id string) (string, error) {
	var d string
	e := s.db.QueryRow("SELECT desired FROM agents WHERE id=?", id).Scan(&d)
	return d, e
}
func (s *Store) pause(work string, paused bool) error {
	if _, e := s.get(work); e != nil {
		return e
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	if _, e = tx.Exec("INSERT INTO cockpit_controls(work_id,paused) VALUES(?,?) ON CONFLICT(work_id) DO UPDATE SET paused=excluded.paused", work, paused); e != nil {
		return e
	}
	if _, e = tx.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), "pause", fmt.Sprintf("opérateur local : départs suspendus=%t", paused)); e != nil {
		return e
	}
	return tx.Commit()
}
func (s *Store) paused(work string) bool {
	var p bool
	_ = s.db.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&p)
	return p
}

func (s *Store) prepare(work string, r Launch) (Agent, bool, error) {
	return s.prepareLaunch(work, r, false)
}

func (s *Store) prepareLaunch(work string, r Launch, previewOnly bool) (Agent, bool, error) {
	var a Agent
	if r.Mode != "" && !interactiveMode(r.Mode) {
		return a, false, fmt.Errorf("mode inconnu")
	}
	if interactiveMode(r.Mode) && (r.Brainstorm || r.Origin == originConductor) {
		return a, false, fmt.Errorf("Le terminal nécessite un lancement manuel de tâche.")
	}
	if r.Brainstorm {
		r.TaskID = "brain-" + hash([]byte(r.EventID))[:20]
		r.Role = "planner"
	}
	if r.Schema != 1 || !safeName(r.EventID) || !safeName(r.TaskID) || !safeName(r.Provider) {
		return a, false, fmt.Errorf("schema_version/event_id/task_id/provider invalide")
	}
	raw, _ := json.Marshal(r)
	// Event ID is the session identity; retries never launch the same command twice.
	var existing []byte
	if e := s.db.QueryRow("SELECT request FROM agents WHERE id=?", r.EventID).Scan(&existing); e == nil {
		a, e = s.agent(r.EventID)
		if string(existing) != string(raw) || a.WorkID != work {
			return a, false, fmt.Errorf("event_id déjà utilisé")
		}
		return a, false, e
	} else if e != sql.ErrNoRows {
		return a, false, e
	}
	if r.Timeout == 0 {
		r.Timeout = 1800
	}
	if r.Timeout < 1 || r.Timeout > 86400 {
		return a, false, fmt.Errorf("timeout_seconds : 1 à 86400")
	}
	if r.Role == "" {
		r.Role = "worker"
	}
	if r.Role != "worker" && r.Role != "planner" && r.Role != "subplanner" {
		return a, false, fmt.Errorf("rôle inconnu")
	}
	if len(r.Instruction) > 16000 {
		return a, false, fmt.Errorf("instruction limitée à 16000 octets")
	}
	if r.Workspace == "" {
		r.Workspace = "."
	}
	cwd, e := resolveWorkspace(s.root, r.Workspace)
	if e != nil {
		return a, false, e
	}
	providers, e := s.providers()
	if e != nil {
		return a, false, e
	}
	p, ok := providers.Providers[r.Provider]
	if !ok {
		return a, false, fmt.Errorf("fournisseur non configuré")
	}
	if r.Mode == "terminal" {
		p, e = terminalProvider(p)
		if e != nil {
			return a, false, e
		}
		if r.Limits != nil && terminalStructuredLimits(*r.Limits) {
			return a, false, fmt.Errorf("Limites d’outils incompatibles avec le terminal natif ; utiliser le mode automatisé.")
		}
	}
	if r.Mode == "dialogue" {
		if e = dialogueProvider(p); e != nil {
			return a, false, e
		}
	}
	purpose := "work"
	if r.Brainstorm {
		purpose = "brainstorm"
	}
	p, route, e := resolveModel(p, r.Level, purpose)
	if e != nil {
		return a, false, e
	}
	if r.ModelPolicyHash != "" && (route == nil || r.ModelPolicyHash != route.PolicyHash) {
		return a, false, fmt.Errorf("Politique de modèle modifiée ; examiner à nouveau le choix.")
	}
	missionLimits := RunLimits{}
	if r.Limits != nil {
		missionLimits = *r.Limits
	}
	limits, e := p.Limits.tightened(missionLimits)
	if e != nil {
		return a, false, e
	}
	if !filepath.IsAbs(p.Command) {
		return a, false, fmt.Errorf("exécutable configuré : chemin absolu requis")
	}
	info, e := os.Stat(p.Command)
	if e != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
		return a, false, fmt.Errorf("exécutable fournisseur indisponible")
	}
	tx, e := s.db.Begin()
	if e != nil {
		return a, false, e
	}
	defer tx.Rollback()
	var paused bool
	e = tx.QueryRow("SELECT paused FROM cockpit_controls WHERE work_id=?", work).Scan(&paused)
	if e != nil && e != sql.ErrNoRows {
		return a, false, e
	}
	if paused {
		return a, false, fmt.Errorf("départs suspendus")
	}
	var body []byte
	var w Work
	if e = tx.QueryRow("SELECT body FROM works WHERE id=?", work).Scan(&body); e != nil {
		return a, false, e
	}
	if e = json.Unmarshal(body, &w); e != nil {
		return a, false, e
	}
	if w.Revision != r.Revision {
		return a, false, fmt.Errorf("révision périmée ; relire le travail")
	}
	if r.Brainstorm {
		if r.PlanBriefHash != "" && (w.PlanningBrief == nil || w.PlanningBrief.SHA256 != r.PlanBriefHash) {
			return a, false, fmt.Errorf("Brief modifié : recommencer la préparation du plan.")
		}
		for _, prior := range w.Tasks {
			if prior.Brainstorm && prior.Status == "running" {
				return a, false, fmt.Errorf("Une réponse IA est déjà en préparation ; attendre sa fin ou l’arrêter.")
			}
		}
		if !nonempty(r.Instruction) {
			return a, false, fmt.Errorf("Décrivez la question à explorer avec l’IA.")
		}
		if e = s.apply(&w, "task.add", Request{ID: r.TaskID, Title: "Brainstorming IA · APEX", Deliverable: "Réponse conservée dans Dialogue IA", Criteria: []string{"Faits et hypothèses séparés", "Options comparées et recommandation motivée", "Plan des agents, gates et conditions d’arrêt proposés"}, Next: r.Instruction}); e != nil {
			return a, false, e
		}
		w.Tasks[len(w.Tasks)-1].Brainstorm = true
		w.Tasks[len(w.Tasks)-1].PlanBriefHash = r.PlanBriefHash
		w.Tasks[len(w.Tasks)-1].Question = r.Instruction
	}
	t, e := w.task(r.TaskID)
	if e != nil {
		return a, false, e
	}
	if e = preparationLaunchGuard(tx, work, t.ID); e != nil {
		return a, false, e
	}
	if r.Mode == "terminal" && t.PlanToolLimit > 0 {
		return a, false, fmt.Errorf("Ce plan impose un plafond d’appels d’outils. Le terminal natif ne peut pas le mesurer ; conserver le mode automatisé.")
	}
	if t.PlanMaxAttempts > 0 {
		var attempts int
		if e = tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND task_id=?", work, t.ID).Scan(&attempts); e != nil {
			return a, false, e
		}
		if attempts >= t.PlanMaxAttempts {
			return a, false, fmt.Errorf("Plafond du plan atteint : %d tentatives. Consigner une OODA et revoir le plan avant toute nouvelle mission.", t.PlanMaxAttempts)
		}
		if t.PlanToolLimit > 0 && (limits.MaxToolCalls == 0 || t.PlanToolLimit < limits.MaxToolCalls) {
			limits.MaxToolCalls = t.PlanToolLimit
		}
		r.Role = t.PlanRole
	}
	if t.PlanBriefHash != "" && (w.PlanningBrief == nil || w.PlanningBrief.SHA256 != t.PlanBriefHash) {
		return a, false, fmt.Errorf("Brief modifié : préparer un nouveau plan.")
	}
	if t.Status == "running" {
		return a, false, fmt.Errorf("tâche déjà running : réconcilier et rouvrir avant lancement")
	}
	if r.Parent != "" {
		var wid string
		if e = tx.QueryRow("SELECT work_id FROM agents WHERE id=?", r.Parent).Scan(&wid); e != nil || wid != work {
			return a, false, fmt.Errorf("parent hors travail")
		}
	}
	var previous Agent
	if r.Previous != "" {
		var b []byte
		if e = tx.QueryRow("SELECT body FROM agents WHERE id=?", r.Previous).Scan(&b); e != nil {
			return a, false, e
		}
		if e = json.Unmarshal(b, &previous); e != nil {
			return a, false, e
		}
		if previous.WorkID != work || previous.TaskID != r.TaskID || activeAgent(previous) {
			return a, false, fmt.Errorf("précédent incompatible ou encore actif")
		}
		if r.Mode != previous.Mode {
			return a, false, fmt.Errorf("Une reprise conserve le mode de la tentative précédente.")
		}
		// A retry from any interface keeps the tighter previous ceilings.
		limits = limits.cappedBy(previous.Limits)
		if previous.Timeout > 0 && previous.Timeout < r.Timeout {
			r.Timeout = previous.Timeout
		}
	}
	var blockerID, blockerWork, blockerTask string
	var overlaps int
	if e = tx.QueryRow("SELECT count(*), coalesce(min(id),''), coalesce(min(work_id),''), coalesce(min(task_id),'') FROM agents WHERE status IN ('queued','starting','running','stopping') AND (cwd=? OR instr(cwd, ? || '/')=1 OR instr(?, cwd || '/')=1)", cwd, cwd, cwd).Scan(&overlaps, &blockerID, &blockerWork, &blockerTask); e != nil {
		return a, false, e
	}
	if overlaps > 0 {
		return a, false, fmt.Errorf("workspace occupé ou recouvrant un workspace actif : agent %s (tâche %s, travail %s) ; voir la page de ce travail pour l'arrêter ou attendre sa fin", blockerID, blockerTask, blockerWork)
	}
	originalNext := t.Next
	// Same transaction as the session intent: no orphan running task on launch conflict.
	if e = s.apply(&w, "task.update", Request{ID: r.TaskID, Status: "running", Owner: r.Provider, Origin: conductorAuthor, Next: "Examiner le handoff et les preuves après exécution"}); e != nil {
		return a, false, e
	}
	// Le choix fait une fois devient réutilisable : profil de la tâche, et
	// profil du travail quand il vient d'un opérateur.
	if !previewOnly && !interactiveMode(r.Mode) {
		profile := launchProfile(r, cwd)
		for i := range w.Tasks {
			if w.Tasks[i].ID == r.TaskID {
				w.Tasks[i].Profile = &profile
			}
		}
		if r.Origin != originConductor {
			w.Profile = &profile
		}
	}
	prompt := fmt.Sprintf("Travail: %s\nObjectif: %s\nPérimètre: %s\nRôle: %s\nTâche %s: %s\nLivrable: %s\nCritères: %s\nProchaine action: %s\nCheckpoint: %s\nInstructions complémentaires: %s\n", w.Title, w.Objective, w.Scope, r.Role, t.ID, t.Title, t.Deliverable, strings.Join(t.Criteria, "; "), originalNext, w.Summary, r.Instruction)
	if r.Mode == "terminal" {
		prompt += fmt.Sprintf("\nSession interactive supervisée, durée maximale %d secondes. Les appels d’outils et le coût ne sont pas mesurables dans ce mode ; ne pas prétendre qu’ils sont contrôlés. Respecter les permissions natives du fournisseur. Attendre les instructions de l’opérateur en cas de doute.\n", r.Timeout)
	} else {
		prompt += executionDirectives(s.root, cwd, t.ID, limits)
	}
	if t.Brainstorm {
		if t.PlanBriefHash != "" {
			prompt += brainstormHistory(w, t.ID) + planDirectives()
		} else {
			prompt += brainstormDirectives(t.ID) + brainstormHistory(w, t.ID)
		}
	}
	if w.PlanningBrief != nil {
		prompt += "\nBRIEF COMMUN ADOPTÉ (contexte, pas une preuve de réussite) :\n" + w.PlanningBrief.Text + "\n"
	}
	prompt += "Pièces de reprise à consulter : " + strings.Join(w.Memory, ", ") + "\n"
	prompt += "Respecter les instructions du projet et les permissions du fournisseur. Ne pas marquer accepté ni modifier la base Swarm. Produire un handoff factuel : changements, tests, preuves, risques, écarts et prochaine action.\n"
	if r.Role != "worker" {
		prompt += "Ce rôle planifie seulement ; ne pas modifier le code.\n"
	}
	if previous.ID != "" {
		prompt += "Reprise d'une nouvelle tentative (pas restauration implicite de conversation). Tentative précédente: " + previous.ID + " ; état: " + previous.Status + " ; vérifier les fichiers existants avant tout nouvel effet.\n"
	}
	// Field guide index is explicit, bounded context, not an automatic transcript capture.
	if guidePath, e := localFile(s.root, ".claude/field-guide/index.md"); e == nil {
		if b, e := os.ReadFile(guidePath); e == nil && len(b) <= 16000 {
			prompt += "\nGuide de terrain (instructions de projet prioritaires):\n" + string(b)
		}
	}
	attachments, err := dialogueAttachments(w, r.References)
	if err != nil {
		return a, false, err
	}
	prompt += attachments
	manifest := contextManifest(w, t.ID, r.References, prompt)
	if len(prompt) > 128000 {
		return a, false, fmt.Errorf("Contexte supérieur à 128000 octets : réduire les pièces jointes ou le brief.")
	}
	if r.ContextHash != "" && manifest.SHA256 != r.ContextHash {
		return a, false, fmt.Errorf("Contexte modifié : examiner un nouvel aperçu avant envoi.")
	}
	a = Agent{Mode: r.Mode, ModelRoute: route, Context: &manifest, Brainstorm: t.Brainstorm, Limits: limits, ID: r.EventID, WorkID: work, TaskID: r.TaskID, Origin: launchOrigin(r), Attempt: t.Attempts[len(t.Attempts)-1].ID, Provider: r.Provider, Role: r.Role, Parent: r.Parent, Previous: r.Previous, CWD: cwd, Status: "queued", Activity: "Lancement demandé ; processus non confirmé", Started: now(), Host: hostIdentity(), Timeout: r.Timeout, Capture: r.Capture, Prompt: prompt, Command: p.Command, Args: p.Args, Env: p.Env}
	if r.Mode == "terminal" {
		if len(s.terminalSocket(a.ID)) >= 108 {
			return a, false, fmt.Errorf("Chemin du terminal trop long ; utiliser une racine de projet plus courte.")
		}
		a.Args = append(append([]string{}, a.Args...), a.Prompt)
		a.Limits = RunLimits{}
		a.Capture = true
		a.Progress.Degraded = terminalMonitoring
	}
	if r.Mode == "dialogue" && len(s.terminalSocket(a.ID)) >= 108 {
		return a, false, fmt.Errorf("Chemin du dialogue trop long")
	}
	if previewOnly {
		// Vérifier le budget dans la même transaction, annulée par le defer.
		if e = reserveBudget(tx, work, a.ID); e != nil {
			return a, false, e
		}
		return a, false, nil
	}
	if t.Brainstorm {
		t.Contexts = append(t.Contexts, SavedContext{a.ID, a.Started, prompt, manifest})
	}
	b, _ := json.Marshal(a)
	if _, e = tx.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,body,request) VALUES(?,?,?,?,?,?,?)", a.ID, work, t.ID, cwd, a.Status, b, raw); e != nil {
		return a, false, fmt.Errorf("agent actif sur tâche/workspace ou conflit : %w", e)
	}
	w.Revision++
	w.Updated = now()
	body, _ = json.Marshal(w)
	if _, e = tx.Exec("UPDATE works SET revision=?,body=? WHERE id=? AND revision=?", w.Revision, body, w.ID, r.Revision); e != nil {
		return a, false, e
	}
	if _, e = tx.Exec("INSERT INTO events VALUES(?,?,?,?,?,?,?)", newID("launch-"), work, w.Revision, "agent.start", w.Updated, raw, raw); e != nil {
		return a, false, e
	}
	if _, e = tx.Exec("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?)", a.ID, now(), "command", "Lancement demandé par opérateur local"); e != nil {
		return a, false, e
	}
	if e = reserveBudget(tx, work, a.ID); e != nil {
		return a, false, e
	}
	e = tx.Commit()
	return a, e == nil, e
}

func (s *Store) stopAgent(id string) error {
	a, e := s.agent(id)
	if e != nil {
		return e
	}
	if !activeAgent(a) {
		return fmt.Errorf("agent déjà terminé")
	}
	tx, e := s.db.Begin()
	if e != nil {
		return e
	}
	defer tx.Rollback()
	result, e := tx.Exec("UPDATE agents SET desired='stop' WHERE id=? AND desired!='stop' AND status IN ('queued','starting','running','stopping')", id)
	if e != nil {
		return e
	}
	changed, e := result.RowsAffected()
	if e != nil {
		return e
	}
	if changed == 0 {
		// Repeated stop requests share the persisted intention and do not add
		// duplicate command events, including simultaneous requests from tabs.
		var desired string
		if e = tx.QueryRow("SELECT desired FROM agents WHERE id=?", id).Scan(&desired); e != nil {
			return e
		}
		if desired != "stop" {
			return fmt.Errorf("agent déjà terminé")
		}
		return tx.Commit()
	}
	if _, e = tx.Exec("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?)", id, now(), "command", "Arrêt demandé ; attendre confirmation du superviseur"); e != nil {
		return e
	}
	return tx.Commit()
}

func (s *Store) finishAgent(a Agent, state, message string, code *int) error {
	a.Status = state
	a.Activity = message
	a.Ended = now()
	a.Heartbeat = now()
	a.ExitCode = code
	if a.Mode == "dialogue" {
		if _, e := s.db.Exec("UPDATE agent_dialogue_turns SET status='interrupted' WHERE agent_id=? AND status='running'", a.ID); e != nil {
			return e
		}
	}
	if e := s.saveAgent(a); e != nil {
		return e
	}
	if e := s.log(a.ID, "lifecycle", message); e != nil {
		return e
	}
	stateBudget := "estimated"
	if a.Child == 0 {
		stateBudget = "released"
	}
	if _, e := s.db.Exec("UPDATE reservations SET state=? WHERE agent_id=? AND state='reserved'", stateBudget, a.ID); e != nil {
		return e
	}
	return s.settleAgentTask(a)
}

func (s *Store) settleAgentTask(a Agent) error {
	state := a.Status
	// Process success is not task acceptance. Submission requires an explicit handoff.
	// Failed/interrupted attempts are blocked, never silently retried.
	w, e := s.get(a.WorkID)
	if e != nil {
		return e
	}
	t, e := w.task(a.TaskID)
	if e != nil {
		return e
	}
	if t.Status != "running" || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].ID != a.Attempt {
		return nil
	}
	outcome := "completed"
	if state == "failed" {
		outcome = "failed"
	}
	if state == "interrupted" {
		outcome = "interrupted"
	}
	r := Request{Schema: 1, EventID: newID("finish-"), Revision: w.Revision, ID: t.ID, Status: "blocked", Origin: conductorAuthor, Outcome: outcome, Blocker: a.Activity + " ; handoff et validation requis", Next: "Examiner logs/diff, puis soumettre ou relancer explicitement"}
	b, _ := json.Marshal(r)
	_, e = s.mutate(w.ID, "task.update", r.EventID, r.Revision, b, func(w *Work) error {
		if e := s.apply(w, "task.update", r); e != nil {
			return e
		}
		task, _ := w.task(a.TaskID)
		if task.Brainstorm {
			body, e := readBrainstormReport(s.root, task.ID)
			if nonempty(a.Reply) {
				body = []byte(a.Reply)
				e = nil
			}
			if e != nil {
				task.ResponseError = "Réponse non disponible : " + e.Error() + ". Consulter les journaux ou relancer."
			} else {
				task.Response = string(body)
				task.Answers = append(task.Answers, BrainstormAnswer{Attempt: a.ID, Text: string(body), At: now()})
				task.ResponseError = ""
				task.Next = "Lire la réponse dans Dialogue IA, répondre ou adopter le brief."
			}
		}
		return nil
	})
	if e != nil {
		_ = s.log(a.ID, "warning", "Réconciliation de tâche nécessaire : "+e.Error())
		return e
	}
	// Le règlement a eu lieu : le conducteur peut relayer un handoff prouvé.
	// Un rejeu de fin de tentative sort plus haut et ne relaie donc jamais deux fois.
	s.conduct(a, outcome)
	// Le créneau libéré doit servir sans attendre une action humaine.
	s.dispatchAfterSettle(a.WorkID)
	return nil
}

func (s *Store) controlEvent(work, kind, message string) error {
	_, e := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", work, now(), kind, terminalText(message))
	return e
}

// Archived telemetry is inert: never insert it back into the active agents table.
type CockpitHistory struct {
	Decisions []Decision          `json:"decisions,omitempty"`
	Visits    []Visit             `json:"visits,omitempty"`
	Agents    []Agent             `json:"agents"`
	Logs      []AgentLog          `json:"logs"`
	Controls  []map[string]string `json:"controls"`
	Limits    string              `json:"limits"`
}

func cockpitHistory(tx *sql.Tx, work string) (*CockpitHistory, error) {
	h := &CockpitHistory{Agents: []Agent{}, Logs: []AgentLog{}, Controls: []map[string]string{}, Limits: "200 dernières sessions, 200 dernières lignes par session, 500 dernières commandes de contrôle"}
	rows, e := tx.Query("SELECT body FROM agents WHERE work_id=? ORDER BY rowid DESC LIMIT 200", work)
	if e != nil {
		return nil, e
	}
	for rows.Next() {
		var b []byte
		var a Agent
		if e = rows.Scan(&b); e != nil {
			rows.Close()
			return nil, e
		}
		if e = json.Unmarshal(b, &a); e != nil {
			rows.Close()
			return nil, e
		}
		h.Agents = append(h.Agents, a)
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return nil, e
	}
	for _, a := range h.Agents {
		rows, e = tx.Query("SELECT seq,agent_id,at,kind,message FROM (SELECT * FROM agent_logs WHERE agent_id=? ORDER BY seq DESC LIMIT 200) ORDER BY seq", a.ID)
		if e != nil {
			return nil, e
		}
		for rows.Next() {
			var l AgentLog
			if e = rows.Scan(&l.Seq, &l.AgentID, &l.At, &l.Kind, &l.Message); e != nil {
				rows.Close()
				return nil, e
			}
			h.Logs = append(h.Logs, l)
		}
		e = rows.Err()
		rows.Close()
		if e != nil {
			return nil, e
		}
	}
	rows, e = tx.Query("SELECT at,kind,message FROM cockpit_events WHERE work_id=? ORDER BY seq DESC LIMIT 500", work)
	if e != nil {
		return nil, e
	}
	for rows.Next() {
		var at, kind, message string
		if e = rows.Scan(&at, &kind, &message); e != nil {
			rows.Close()
			return nil, e
		}
		h.Controls = append(h.Controls, map[string]string{"at": at, "kind": kind, "message": message})
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return nil, e
	}
	rows, e = tx.Query("SELECT body FROM decisions WHERE work_id=?", work)
	if e != nil {
		return nil, e
	}
	for rows.Next() {
		var raw []byte
		var d Decision
		if e = rows.Scan(&raw); e != nil {
			rows.Close()
			return nil, e
		}
		if e = json.Unmarshal(raw, &d); e != nil {
			rows.Close()
			return nil, e
		}
		h.Decisions = append(h.Decisions, d)
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return nil, e
	}
	rows, e = tx.Query("SELECT operator,revision,at FROM session_visits WHERE work_id=?", work)
	if e != nil {
		return nil, e
	}
	for rows.Next() {
		var v Visit
		if e = rows.Scan(&v.Operator, &v.Revision, &v.At); e != nil {
			rows.Close()
			return nil, e
		}
		h.Visits = append(h.Visits, v)
	}
	e = rows.Err()
	rows.Close()
	return h, e
}

// A workspace is an explicit directory inside the selected project. No arbitrary cwd.
func resolveWorkspace(root, workspace string) (string, error) {
	cwd := workspace
	if cwd == "" {
		cwd = "."
	}
	if !filepath.IsAbs(cwd) {
		cwd = filepath.Join(root, cwd)
	}
	cwd, e := filepath.EvalSymlinks(cwd)
	if e != nil {
		return "", e
	}
	rel, e := filepath.Rel(root, cwd)
	if e != nil || rel == ".." || strings.HasPrefix(rel, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("workspace hors projet")
	}
	info, e := os.Stat(cwd)
	if e != nil || !info.IsDir() {
		return "", fmt.Errorf("workspace non répertoire")
	}
	return cwd, nil
}
