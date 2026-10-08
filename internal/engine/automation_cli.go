//go:build linux

package engine

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"syscall"
	"time"
)

type AutomationService struct{ store *Store }

type AutomationTargetView struct {
	WorkID          string         `json:"work_id"`
	MissionRevision int            `json:"mission_revision"`
	MissionState    string         `json:"mission_validation_state"`
	Authorized      bool           `json:"authorized"`
	Authorization   string         `json:"authorization_reason,omitempty"`
	Profile         map[string]any `json:"profile"`
	Limits          map[string]any `json:"limits"`
	Budget          BudgetView     `json:"budget"`
	Cost            CostTotal      `json:"cost"`
	CostState       string         `json:"cost_state"`
	NextAction      string         `json:"next_action"`
}

type AutomationOccurrenceView struct {
	OccurrenceID string `json:"occurrence_id"`
	RequestID    string `json:"request_id"`
	Revision     int    `json:"revision"`
	IntendedAt   string `json:"intended_at"`
	State        string `json:"state"`
	Reason       string `json:"reason,omitempty"`
	NextAction   string `json:"next_action,omitempty"`
	MissionState string `json:"mission_validation_state"`
}

type AutomationProgramView struct {
	Schedule    AutomationScheduleRecord   `json:"schedule"`
	Target      AutomationTargetView       `json:"target"`
	Occurrences []AutomationOccurrenceView `json:"occurrences"`
}

type AutomationCreateCommand struct {
	Schedule     AutomationScheduleCreate `json:"schedule"`
	PreviewToken string                   `json:"preview_token"`
}

type AutomationStateCommand struct {
	ScheduleID       string `json:"schedule_id"`
	State            string `json:"state"`
	ExpectedRevision int    `json:"expected_revision"`
}

type AutomationCancelCommand struct {
	OccurrenceID     string `json:"occurrence_id"`
	ExpectedRevision int    `json:"expected_revision"`
}

type AutomationConfigCommand struct {
	Schema           int                    `json:"schema_version"`
	ExpectedRevision int                    `json:"expected_revision"`
	Values           AutomationConfigValues `json:"values"`
	Actor            string                 `json:"actor"`
}

type AutomationPreviewView struct {
	AutomationSchedulePreview
	PreviewToken   string               `json:"preview_token"`
	ConfigRevision int                  `json:"config_revision"`
	Target         AutomationTargetView `json:"target"`
}

func (s *Store) automationService() AutomationService { return AutomationService{store: s} }

func automationPreviewToken(request AutomationScheduleCreate, configRevision int) string {
	raw, _ := json.Marshal(struct {
		Request AutomationScheduleCreate `json:"request"`
		Config  int                      `json:"config_revision"`
	}{request, configRevision})
	return hash(raw)
}

func (a AutomationService) target(workID string) (AutomationTargetView, error) {
	w, err := a.store.get(workID)
	if err != nil {
		return AutomationTargetView{}, &CommandError{Code: "unknown_target", Message: "cible inconnue"}
	}
	tx, err := a.store.db.Begin()
	if err != nil {
		return AutomationTargetView{}, err
	}
	authorized, reason, policyErr := automationPolicyTx(tx, workID)
	_ = tx.Rollback()
	if policyErr != nil {
		return AutomationTargetView{}, policyErr
	}
	profile := map[string]any{"provider": "unknown", "role": "unknown"}
	if w.Profile != nil {
		profile = map[string]any{"provider": w.Profile.Provider, "role": w.Profile.Role, "level": w.Profile.Level}
	} else if w.Planning != nil && w.Planning.Provider != "" {
		profile = map[string]any{"provider": w.Planning.Provider, "role": "planning"}
	}
	limits := map[string]any{"max_activations": nil, "remaining_activations": nil, "max_decisions": nil, "remaining_decisions": nil}
	if w.Planning != nil {
		limits = map[string]any{"max_activations": w.Planning.MaxActivations, "remaining_activations": max(0, w.Planning.MaxActivations-w.Planning.Activations), "max_decisions": w.Planning.MaxDecisions, "remaining_decisions": max(0, w.Planning.MaxDecisions-w.Planning.Decisions)}
	}
	budget, err := a.store.budget(workID)
	if err != nil {
		return AutomationTargetView{}, err
	}
	costs, err := a.store.costSummary(workID)
	if err != nil {
		return AutomationTargetView{}, err
	}
	costState := "unknown"
	if costs.WithCost > 0 {
		costState = "partial_reported"
		if costs.Silent == 0 {
			costState = "reported"
		}
	}
	missionState := "not_validated"
	if automationTerminal(w) {
		missionState = "validated_closed"
	}
	next := "enable_when_authorized"
	if authorized {
		next = "preview_then_enable"
	}
	return AutomationTargetView{WorkID: workID, MissionRevision: w.Revision, MissionState: missionState, Authorized: authorized, Authorization: reason, Profile: profile, Limits: limits, Budget: budget, Cost: costs.CostTotal, CostState: costState, NextAction: next}, nil
}

func (a AutomationService) Preview(request AutomationScheduleCreate, count int, clock automationClock) (AutomationPreviewView, error) {
	if clock == nil {
		clock = realAutomationClock
	}
	config, err := a.store.automationConfig()
	if err != nil {
		return AutomationPreviewView{}, err
	}
	preview, err := previewAutomationSchedule(request, clock().UTC(), count, config)
	if err != nil {
		return AutomationPreviewView{}, err
	}
	target, err := a.target(request.TargetWorkID)
	if err != nil {
		return AutomationPreviewView{}, err
	}
	return AutomationPreviewView{AutomationSchedulePreview: preview, PreviewToken: automationPreviewToken(request, config.Revision), ConfigRevision: config.Revision, Target: target}, nil
}

func (a AutomationService) Create(command AutomationCreateCommand, clock automationClock) (AutomationProgramView, error) {
	config, err := a.store.automationConfig()
	if err != nil {
		return AutomationProgramView{}, err
	}
	if command.PreviewToken == "" || command.PreviewToken != automationPreviewToken(command.Schedule, config.Revision) {
		return AutomationProgramView{}, &CommandError{Code: "preview_stale", Message: "aperçu requis ou périmé ; prévisualiser à nouveau avant création"}
	}
	record, err := a.store.createAutomationSchedule(command.Schedule, clock)
	if err != nil {
		return AutomationProgramView{}, err
	}
	return a.Show(record.ScheduleID)
}

func (a AutomationService) List() ([]AutomationProgramView, error) {
	rows, err := a.store.db.Query(automationScheduleSelect + " ORDER BY updated_at DESC,schedule_id")
	if err != nil {
		return nil, err
	}
	records := []AutomationScheduleRecord{}
	for rows.Next() {
		record, scanErr := scanAutomationSchedule(rows)
		if scanErr != nil {
			rows.Close()
			return nil, scanErr
		}
		records = append(records, record)
	}
	err = rows.Err()
	rows.Close()
	if err != nil {
		return nil, err
	}
	out := []AutomationProgramView{}
	for _, record := range records {
		view, showErr := a.showRecord(record)
		if showErr != nil {
			return nil, showErr
		}
		out = append(out, view)
	}
	return out, nil
}

func (a AutomationService) Show(id string) (AutomationProgramView, error) {
	if !safeName(id) {
		return AutomationProgramView{}, &CommandError{Code: "invalid_input", Message: "identifiant de programme invalide"}
	}
	record, err := a.store.automationSchedule(id)
	if errors.Is(err, sql.ErrNoRows) {
		return AutomationProgramView{}, &CommandError{Code: "unknown_schedule", Message: "programme inconnu"}
	}
	if err != nil {
		return AutomationProgramView{}, err
	}
	return a.showRecord(record)
}

func (a AutomationService) showRecord(record AutomationScheduleRecord) (AutomationProgramView, error) {
	target, err := a.target(record.TargetWorkID)
	if err != nil {
		return AutomationProgramView{}, err
	}
	rows, err := a.store.db.Query(`SELECT o.occurrence_id,o.request_id,o.revision,x.intended_at,r.state,r.reason,r.next_action
 FROM automation_request_origins x JOIN automation_requests r ON r.request_id=x.request_id
 JOIN automation_occurrences o ON o.request_id=r.request_id WHERE x.schedule_id=? ORDER BY x.intended_at DESC`, record.ScheduleID)
	if err != nil {
		return AutomationProgramView{}, err
	}
	defer rows.Close()
	occurrences := []AutomationOccurrenceView{}
	for rows.Next() {
		var item AutomationOccurrenceView
		if err = rows.Scan(&item.OccurrenceID, &item.RequestID, &item.Revision, &item.IntendedAt, &item.State, &item.Reason, &item.NextAction); err != nil {
			return AutomationProgramView{}, err
		}
		if item.State == "rejected" && item.Reason == "cancelled_by_operator" {
			item.State = "cancelled"
		}
		item.MissionState = target.MissionState
		occurrences = append(occurrences, item)
	}
	return AutomationProgramView{Schedule: record, Target: target, Occurrences: occurrences}, rows.Err()
}

func (a AutomationService) SetState(command AutomationStateCommand, clock automationClock) (AutomationProgramView, error) {
	if !safeName(command.ScheduleID) {
		return AutomationProgramView{}, &CommandError{Code: "invalid_input", Message: "schedule_id requis"}
	}
	record, err := a.store.setAutomationScheduleState(command.ScheduleID, command.State, command.ExpectedRevision, clock)
	if err != nil {
		return AutomationProgramView{}, err
	}
	return a.Show(record.ScheduleID)
}

func (a AutomationService) Cancel(command AutomationCancelCommand) (AutomationOccurrenceView, error) {
	if !safeName(command.OccurrenceID) || command.ExpectedRevision < 1 {
		return AutomationOccurrenceView{}, &CommandError{Code: "invalid_input", Message: "occurrence_id et expected_revision requis"}
	}
	tx, err := a.store.db.Begin()
	if err != nil {
		return AutomationOccurrenceView{}, err
	}
	defer tx.Rollback()
	var requestID, state, effectID string
	var revision int
	if err = tx.QueryRow(`SELECT request_id,state,revision,effect_id FROM automation_occurrences WHERE occurrence_id=?`, command.OccurrenceID).Scan(&requestID, &state, &revision, &effectID); errors.Is(err, sql.ErrNoRows) {
		return AutomationOccurrenceView{}, &CommandError{Code: "unknown_occurrence", Message: "occurrence inconnue"}
	}
	if err != nil {
		return AutomationOccurrenceView{}, err
	}
	if revision != command.ExpectedRevision {
		return AutomationOccurrenceView{}, &CommandError{Code: "revision_conflict", Message: "occurrence modifiée ; recharger avant annulation"}
	}
	if state != "received" && state != "waiting" || effectID != "" {
		return AutomationOccurrenceView{}, &CommandError{Code: "effect_started", Message: "annulation refusée : la prise en charge ou l’effet a commencé"}
	}
	stamp := now()
	result, err := tx.Exec(`UPDATE automation_occurrences SET state='rejected',revision=revision+1 WHERE occurrence_id=? AND revision=? AND state IN ('received','waiting') AND effect_id=''`, command.OccurrenceID, revision)
	if err != nil {
		return AutomationOccurrenceView{}, err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return AutomationOccurrenceView{}, &CommandError{Code: "revision_conflict", Message: "occurrence modifiée ; recharger avant annulation"}
	}
	if _, err = tx.Exec(`UPDATE automation_requests SET state='rejected',revision=revision+1,reason='cancelled_by_operator',actor=?,next_action='',updated_at=? WHERE request_id=?`, operatorIdentity(), stamp, requestID); err != nil {
		return AutomationOccurrenceView{}, err
	}
	if err = tx.Commit(); err != nil {
		return AutomationOccurrenceView{}, err
	}
	return AutomationOccurrenceView{OccurrenceID: command.OccurrenceID, RequestID: requestID, Revision: revision + 1, State: "cancelled", Reason: "cancelled_by_operator", MissionState: "not_validated"}, nil
}

func (a AutomationService) ApplyConfig(command AutomationConfigCommand) (AutomationConfig, error) {
	lock, err := os.OpenFile(filepath.Join(a.store.root, ".swarm", "automation-config.lock"), os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return AutomationConfig{}, err
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); err != nil {
		return AutomationConfig{}, err
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	current, err := a.store.automationConfig()
	if err != nil {
		return AutomationConfig{}, err
	}
	if command.Schema != 1 || command.ExpectedRevision != current.Revision || !validateAutomationConfigValues(command.Values) {
		if command.ExpectedRevision != current.Revision {
			return AutomationConfig{}, &CommandError{Code: "revision_conflict", Message: "réglages modifiés ; recharger avant confirmation"}
		}
		return AutomationConfig{}, &CommandError{Code: "invalid_input", Message: "réglages opérationnels hors bornes"}
	}
	actor := command.Actor
	if actor == "" {
		actor = operatorIdentity()
	}
	current.Revision++
	current.Values = command.Values
	current.History = append(current.History, AutomationConfigRevision{Revision: current.Revision, Values: command.Values, Actor: actor, At: now()})
	raw, err := json.MarshalIndent(current, "", "  ")
	if err != nil {
		return AutomationConfig{}, err
	}
	raw = append(raw, '\n')
	if err = atomicWrite(filepath.Join(a.store.root, ".swarm", "automation.json"), raw); err != nil {
		return AutomationConfig{}, err
	}
	return current, nil
}

func automationCLIHelp() string {
	lines := []string{
		"PROGRAMMES ET AUTOMATISATION",
		"Lister : swarm automation list",
		"Afficher : swarm automation show PROGRAMME",
		"Prévisualiser : swarm automation preview --input programme.json",
		"Créer désactivé : swarm automation create --input creation.json",
		"Activer, suspendre ou archiver : swarm automation enable|pause|archive PROGRAMME --input revision.json",
		"Annuler une occurrence encore en attente : swarm automation cancel OCCURRENCE --input revision.json",
		"Réglages : swarm automation params show|apply [--input reglages.json]",
		"La création exige le preview_token du même contenu et crée toujours un programme désactivé.",
		"Une occurrence traitée ne signifie pas que la mission est validée. Un coût absent reste inconnu.",
	}
	for i := range lines {
		lines[i] = uiText(lines[i])
	}
	return strings.Join(lines, "\n") + "\n"
}

func (s *Store) automationCLI(pos []string, input string, out io.Writer) error {
	if len(pos) < 2 {
		return &CommandError{Code: "invalid_input", Message: "swarm automation list|show|preview|create|enable|pause|archive|cancel|params"}
	}
	service := s.automationService()
	decode := func(value any) error {
		raw, err := readInput(input)
		if err != nil {
			return err
		}
		return strict(raw, value)
	}
	switch pos[1] {
	case "list":
		if len(pos) != 2 {
			return fmt.Errorf("swarm automation list")
		}
		value, err := service.List()
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "show":
		if len(pos) != 3 {
			return fmt.Errorf("swarm automation show PROGRAMME")
		}
		value, err := service.Show(pos[2])
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "preview":
		if len(pos) != 2 {
			return fmt.Errorf("swarm automation preview --input programme.json")
		}
		var request AutomationScheduleCreate
		if err := decode(&request); err != nil {
			return err
		}
		value, err := service.Preview(request, 5, nil)
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "create":
		if len(pos) != 2 {
			return fmt.Errorf("swarm automation create --input creation.json")
		}
		var command AutomationCreateCommand
		if err := decode(&command); err != nil {
			return err
		}
		value, err := service.Create(command, nil)
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "enable", "pause", "archive":
		if len(pos) != 3 {
			return fmt.Errorf("swarm automation enable|pause|archive PROGRAMME --input revision.json")
		}
		var revision struct {
			ExpectedRevision int `json:"expected_revision"`
		}
		if err := decode(&revision); err != nil {
			return err
		}
		state := map[string]string{"enable": "enabled", "pause": "paused", "archive": "archived"}[pos[1]]
		value, err := service.SetState(AutomationStateCommand{ScheduleID: pos[2], State: state, ExpectedRevision: revision.ExpectedRevision}, nil)
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "cancel":
		if len(pos) != 3 {
			return fmt.Errorf("swarm automation cancel OCCURRENCE --input revision.json")
		}
		var revision struct {
			ExpectedRevision int `json:"expected_revision"`
		}
		if err := decode(&revision); err != nil {
			return err
		}
		value, err := service.Cancel(AutomationCancelCommand{OccurrenceID: pos[2], ExpectedRevision: revision.ExpectedRevision})
		if err != nil {
			return err
		}
		return printJSON(out, value)
	case "params":
		if len(pos) != 3 {
			return fmt.Errorf("swarm automation params show|apply")
		}
		if pos[2] == "show" {
			value, err := s.automationConfig()
			if err != nil {
				return err
			}
			return printJSON(out, value)
		}
		if pos[2] == "apply" {
			var command AutomationConfigCommand
			if err := decode(&command); err != nil {
				return err
			}
			value, err := service.ApplyConfig(command)
			if err != nil {
				return err
			}
			return printJSON(out, value)
		}
		return fmt.Errorf("swarm automation params show|apply")
	default:
		return fmt.Errorf("swarm automation list|show|preview|create|enable|pause|archive|cancel|params")
	}
}

func automationSortedConfigKeys() []string {
	keys := []string{"claim_lease_seconds", "due_grace_seconds", "external_max_payload_bytes", "external_rate_limit", "external_rate_window_seconds", "external_timestamp_window_seconds", "max_preview_occurrences", "max_schedule_occurrences"}
	sort.Strings(keys)
	return keys
}

var _ = os.ErrNotExist
var _ = time.Second
