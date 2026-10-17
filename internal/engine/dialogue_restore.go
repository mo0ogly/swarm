//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
)

type DialogueRestoreChoice struct {
	Checkpoint string `json:"checkpoint_sha256"`
	Facts      string `json:"facts_sha256"`
	Confirm    bool   `json:"confirm"`
	Reason     string `json:"reason"`
}

type DialogueRestoreRequest struct {
	Schema      int    `json:"schema_version"`
	Event       string `json:"event_id"`
	Revision    int    `json:"expected_revision"`
	Checkpoint  string `json:"checkpoint_sha256"`
	Facts       string `json:"facts_sha256"`
	Confirm     bool   `json:"confirm"`
	Reason      string `json:"reason"`
	Instruction string `json:"instruction"`
}

// A distinct execution and reservation keep previous costs and attempts intact.
// The checkpoint, not this public receipt, contains the cumulative private guard.
type DialogueRestoreReceipt struct {
	SourceAgent      string `json:"source_agent"`
	SourceAttempt    string `json:"source_attempt"`
	SourceState      string `json:"source_state_sha256"`
	Checkpoint       string `json:"checkpoint_sha256"`
	Facts            string `json:"facts_sha256"`
	RootAgent        string `json:"root_agent"`
	PriorTurns       int    `json:"prior_turns"`
	Reason           string `json:"reason"`
	Actor            string `json:"actor,omitempty"`
	At               string `json:"at,omitempty"`
	DecisionRecorded bool   `json:"decision_recorded"`
}

type dialogueRestoreSource struct {
	agent      Agent
	checkpoint *dialogueCheckpoint
	turns      int
}

func dialogueSourceSHA(a Agent) string { raw, _ := json.Marshal(a); return hash(raw) }

func (s *Store) resumeDialogue(id string, r DialogueRestoreRequest, preview bool) (Agent, bool, error) {
	a, err := s.agent(id)
	if err != nil {
		return Agent{}, false, err
	}
	limits := a.Limits
	launch := Launch{Schema: r.Schema, EventID: r.Event, Revision: r.Revision, TaskID: a.TaskID, Provider: a.Provider, Role: a.Role, Mode: "dialogue", Workspace: a.CWD, Previous: a.ID, Parent: a.Parent, Instruction: r.Instruction, Capture: a.Capture, Timeout: a.Timeout, Limits: &limits,
		DialogueRestore: &DialogueRestoreChoice{Checkpoint: r.Checkpoint, Facts: r.Facts, Confirm: r.Confirm, Reason: r.Reason}}
	if a.ModelRoute != nil {
		launch.Level = a.ModelRoute.Level
		launch.ModelPolicyHash = a.ModelRoute.PolicyHash
	}
	if preview {
		return s.prepareLaunch(a.WorkID, launch, true)
	}
	return s.prepare(a.WorkID, launch)
}

func (s *Store) prepareDialogueRestore(w Work, r Launch) (*dialogueRestoreSource, error) {
	if r.DialogueRestore == nil {
		return nil, nil
	}
	choice := r.DialogueRestore
	if !choice.Confirm || len(strings.TrimSpace(choice.Reason)) < 8 || len(choice.Reason) > 2000 || len(strings.TrimSpace(r.Instruction)) < 20 || len(choice.Checkpoint) != 64 || len(choice.Facts) != 64 || r.Previous == "" || r.Mode != "dialogue" || r.Brainstorm || r.Origin == originConductor {
		return nil, fmt.Errorf("Reprise de conversation : confirmation, motif, nouvelle consigne et empreintes examinées requis")
	}
	if w.Planning != nil && w.Planning.Repository != nil {
		return nil, fmt.Errorf("Reprise de conversation : espace Git géré incompatible ; utiliser la reprise de production")
	}
	a, err := s.agent(r.Previous)
	if err != nil {
		return nil, err
	}
	t, err := w.task(r.TaskID)
	if err != nil {
		return nil, err
	}
	if a.WorkID != w.ID || a.TaskID != r.TaskID || a.Mode != "dialogue" || a.Role != r.Role || a.Provider != r.Provider || a.CWD != r.Workspace || a.ID == r.EventID {
		return nil, fmt.Errorf("Reprise de conversation : tentative source incompatible")
	}
	if a.Context == nil || a.Workflow == nil {
		return nil, fmt.Errorf("Reprise de conversation : contexte source non attribué")
	}
	p, err := s.dialogueRecovery(w, t, &a)
	if err != nil {
		return nil, err
	}
	if p == nil || p.Path != "review_boundary" || p.FactsSHA256 != choice.Facts || p.Boundary.SHA256 != choice.Checkpoint {
		return nil, &CommandError{Code: "preview_stale", Message: "Reprise de conversation : faits modifiés ou frontière non confirmée ; relire le diagnostic"}
	}
	if a.Workflow != nil {
		if err = s.actionSkillsGuard(a.Workflow.Skills); err != nil {
			return nil, err
		}
		if err = s.projectContextGuard(a.Workflow.Project, a.Role); err != nil {
			return nil, err
		}
	}
	boundary, c, err := s.readDialogueBoundary(a)
	if err != nil {
		return nil, err
	}
	if c == nil || !boundary.Confirmed || c.SHA256 != choice.Checkpoint || c.Phase != "confirmed" {
		return nil, &CommandError{Code: "preview_stale", Message: "Reprise de conversation : frontière source modifiée"}
	}
	if !c.Guard.Limits.observing() && (c.Guard.Calls >= c.Guard.Limits.MaxToolCalls || c.Guard.Repeated >= c.Guard.Limits.MaxRepeatedCalls || c.Guard.Errors >= c.Guard.Limits.MaxConsecutiveErrors) {
		return nil, fmt.Errorf("Reprise de conversation : garde cumulée épuisée")
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=?", a.ID).Scan(&count); err != nil {
		return nil, err
	}
	if a.DialogueRestore != nil {
		count += a.DialogueRestore.PriorTurns
	}
	if count < 1 || count >= 20 {
		return nil, fmt.Errorf("Reprise de conversation : limite cumulée de 20 échanges atteinte")
	}
	return &dialogueRestoreSource{a, c, count}, nil
}

// Use the launch transaction: Store's single database connection cannot be
// reentered while its writer reservation is held.
func (s *Store) recheckDialogueRestore(tx *sql.Tx, w Work, r Launch, source *dialogueRestoreSource) error {
	if source == nil {
		return nil
	}
	var raw []byte
	var status, desired, latest string
	if err := tx.QueryRow("SELECT body,status,desired FROM agents WHERE id=?", source.agent.ID).Scan(&raw, &status, &desired); err != nil {
		return err
	}
	a, err := decodeAgentRow(raw, status, desired)
	if err != nil {
		return err
	}
	if err = tx.QueryRow("SELECT id FROM agents WHERE work_id=? AND task_id=? ORDER BY rowid DESC LIMIT 1", w.ID, r.TaskID).Scan(&latest); err != nil {
		return err
	}
	t, err := w.task(r.TaskID)
	if err != nil {
		return err
	}
	if latest != a.ID || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].ID != a.Attempt || dialogueSourceSHA(a) != dialogueSourceSHA(source.agent) || !recoveryProcessEnded(a, hostIdentity(), processStamp(a.Child)) {
		return &CommandError{Code: "preview_stale", Message: "Reprise de conversation : tentative source modifiée ; relire le diagnostic"}
	}
	if err = s.dialogueMemorySelectionGuard(tx, a, t.MemorySelections); err != nil {
		return err
	}
	b, c, err := s.readDialogueBoundaryWith(tx, a)
	if err != nil {
		return err
	}
	if !b.Confirmed || c.SHA256 != source.checkpoint.SHA256 {
		return fmt.Errorf("Reprise de conversation : frontière source modifiée")
	}
	var count int
	if err = tx.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=?", a.ID).Scan(&count); err != nil {
		return err
	}
	if a.DialogueRestore != nil {
		count += a.DialogueRestore.PriorTurns
	}
	if count != source.turns {
		return fmt.Errorf("Reprise de conversation : historique des échanges modifié")
	}
	return nil
}

func (source *dialogueRestoreSource) receipt(choice *DialogueRestoreChoice, preview bool) *DialogueRestoreReceipt {
	if source == nil {
		return nil
	}
	root := source.agent.ID
	if source.agent.DialogueRestore != nil {
		root = source.agent.DialogueRestore.RootAgent
	}
	r := &DialogueRestoreReceipt{SourceAgent: source.agent.ID, SourceAttempt: source.agent.Attempt, SourceState: dialogueSourceSHA(source.agent), Checkpoint: source.checkpoint.SHA256, Facts: choice.Facts, RootAgent: root, PriorTurns: source.turns, Reason: strings.TrimSpace(choice.Reason)}
	if !preview {
		r.At = now()
		r.Actor = operatorIdentity()
		r.DecisionRecorded = true
	}
	return r
}

func (s *Store) loadDialogueRestore(a Agent) (*dialogueCheckpoint, *AgentProgress, error) {
	r := a.DialogueRestore
	if r == nil {
		return nil, nil, nil
	}
	previous, err := s.agent(r.SourceAgent)
	if err != nil {
		return nil, nil, err
	}
	if !r.DecisionRecorded || r.At == "" || r.Actor == "" || r.SourceAgent != a.Previous || r.SourceAttempt != previous.Attempt || r.SourceState != dialogueSourceSHA(previous) || previous.Provider != a.Provider || previous.CWD != a.CWD || previous.Role != a.Role || previous.Limits != a.Limits || previous.Command != a.Command || !reflect.DeepEqual(previous.Args, a.Args) || !reflect.DeepEqual(previous.Env, a.Env) || !reflect.DeepEqual(previous.ModelRoute, a.ModelRoute) {
		return nil, nil, fmt.Errorf("Reprise de conversation : décision ou lancement incompatible")
	}
	b, c, err := s.readDialogueBoundary(previous)
	if err != nil {
		return nil, nil, err
	}
	if !b.Confirmed || c.SHA256 != r.Checkpoint || r.PriorTurns < 1 || r.PriorTurns >= 20 {
		return nil, nil, fmt.Errorf("Reprise de conversation : frontière source indisponible")
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM agent_dialogue_turns WHERE agent_id=?", previous.ID).Scan(&count); err != nil {
		return nil, nil, err
	}
	root := previous.ID
	if previous.DialogueRestore != nil {
		count += previous.DialogueRestore.PriorTurns
		root = previous.DialogueRestore.RootAgent
	}
	if count != r.PriorTurns || root != r.RootAgent {
		return nil, nil, fmt.Errorf("Reprise de conversation : filiation ou compte des échanges incompatible")
	}
	return c, b.Progress, nil
}

func (s *Store) dialogueRestoreStartGuard(a Agent) error {
	if a.DialogueRestore == nil {
		return nil
	}
	if _, _, err := s.loadDialogueRestore(a); err != nil {
		return err
	}
	w, err := s.get(a.WorkID)
	if err != nil {
		return err
	}
	t, err := w.task(a.TaskID)
	if err != nil {
		return err
	}
	if t.Status != "running" || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].ID != a.Attempt {
		return fmt.Errorf("Reprise de conversation : tentative courante modifiée avant démarrage")
	}
	if err = s.dialogueMemorySelectionGuard(s.db, a, t.MemorySelections); err != nil {
		return err
	}
	providers, err := s.providers()
	if err != nil {
		return err
	}
	p, exists := providers.Providers[a.Provider]
	if !exists || dialogueProvider(p) != nil {
		return fmt.Errorf("Reprise de conversation : fournisseur modifié avant démarrage")
	}
	level := ""
	if a.ModelRoute != nil {
		level = a.ModelRoute.Level
	}
	p, route, err := resolveModel(p, level, "work")
	if err != nil {
		return err
	}
	admin, err := configuredRunLimitsWith(s.db, w.ID, a.Role, t.ID)
	if err != nil {
		return err
	}
	bounded := administrativeDurationThresholds(a.Limits.cappedBy(p.Limits).cappedBy(admin), admin)
	bounded.ObservationMode = admin.ObservationMode
	if p.Command != a.Command || !reflect.DeepEqual(p.Args, a.Args) || !reflect.DeepEqual(p.Env, a.Env) || !reflect.DeepEqual(route, a.ModelRoute) || bounded != a.Limits {
		return fmt.Errorf("Reprise de conversation : contrat modifié avant démarrage")
	}
	cwd, err := resolveWorkspace(s.root, a.CWD)
	if err != nil {
		return err
	}
	if cwd != a.CWD {
		return fmt.Errorf("Reprise de conversation : espace modifié avant démarrage")
	}
	if a.Preflight == nil || a.Preflight.Verification != "verified" {
		return fmt.Errorf("Reprise de conversation : précontrôle vérifié absent")
	}
	return s.recheckPreflight(*a.Preflight, a.Provider, p, a.CWD, Launch{Mode: a.Mode, Level: level})
}

func (s *Store) restoredDialogueMonitor(a Agent) (*dialogueMonitor, error) {
	m := newDialogueMonitor(s, a)
	c, base, err := s.loadDialogueRestore(a)
	if err != nil {
		return nil, err
	}
	if c != nil {
		previous, err := s.agent(a.Previous)
		if err != nil {
			return nil, err
		}
		g, err := c.restoreGuard(previous, s.root)
		if err != nil {
			return nil, err
		}
		m.sink.guard = g
		m.session = c.Session
		m.usage = cloneDialogueUsage(c.Usage)
		m.baseProgress = base
		m.sink.reportedModel = c.Model
	}
	return m, nil
}
