# C04 — dossier Go du candidat de reprise 2/2

Go/gardes/tests complets ci-dessous ; UI et navigateur complets dans docs/C04.md. Corrections de liste/SQL/CAS attribuées à la supervision, pas à une revue.

## automation_cli.go

```go
//go:build linux

package main

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
```

## automation_http.go

```go
//go:build linux

package main

import (
        "io"
        "net/http"
        "strconv"
)

func registerAutomationHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
        service := s.automationService()
        decode := func(w http.ResponseWriter, r *http.Request, value any) error {
                raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
                if err != nil {
                        return err
                }
                return strict(raw, value)
        }
        mux.HandleFunc("/api/v1/automation/programs", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodGet {
                        http.Error(w, "GET requis", http.StatusMethodNotAllowed)
                        return
                }
                id := r.URL.Query().Get("id")
                if id != "" {
                        value, err := service.Show(id)
                        if err != nil {
                                fail(w, err)
                                return
                        }
                        send(w, value)
                        return
                }
                value, err := service.List()
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/automation/preview", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var request AutomationScheduleCreate
                if err := decode(w, r, &request); err != nil {
                        fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
                        return
                }
                count := 5
                if raw := r.URL.Query().Get("count"); raw != "" {
                        if value, err := strconv.Atoi(raw); err == nil {
                                count = value
                        }
                }
                value, err := service.Preview(request, count, nil)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/automation/create", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var command AutomationCreateCommand
                if err := decode(w, r, &command); err != nil {
                        fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
                        return
                }
                value, err := service.Create(command, nil)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/automation/state", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var command AutomationStateCommand
                if err := decode(w, r, &command); err != nil {
                        fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
                        return
                }
                value, err := service.SetState(command, nil)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/automation/cancel", func(w http.ResponseWriter, r *http.Request) {
                if r.Method != http.MethodPost {
                        http.Error(w, "POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var command AutomationCancelCommand
                if err := decode(w, r, &command); err != nil {
                        fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
                        return
                }
                value, err := service.Cancel(command)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
        mux.HandleFunc("/api/v1/automation/config", func(w http.ResponseWriter, r *http.Request) {
                if r.Method == http.MethodGet {
                        value, err := s.automationConfig()
                        if err != nil {
                                fail(w, err)
                                return
                        }
                        send(w, value)
                        return
                }
                if r.Method != http.MethodPost {
                        http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
                        return
                }
                var command AutomationConfigCommand
                if err := decode(w, r, &command); err != nil {
                        fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
                        return
                }
                value, err := service.ApplyConfig(command)
                if err != nil {
                        fail(w, err)
                        return
                }
                send(w, value)
        })
}
```


## automation_cli_test.go

```go
//go:build linux

package main

import (
        "bytes"
        "encoding/json"
        "errors"
        "net/http"
        "net/http/httptest"
        "os"
        "path/filepath"
        "strings"
        "sync"
        "testing"
        "time"
)

func automationC04Input(t *testing.T, value any) string {
        t.Helper()
        raw, err := json.Marshal(value)
        if err != nil {
                t.Fatal(err)
        }
        path := filepath.Join(t.TempDir(), "input.json")
        if err = os.WriteFile(path, raw, 0600); err != nil {
                t.Fatal(err)
        }
        return path
}

func automationC04HTTP(t *testing.T, handler http.Handler, method, path string, value any) *httptest.ResponseRecorder {
        t.Helper()
        var body bytes.Buffer
        if value != nil {
                if err := json.NewEncoder(&body).Encode(value); err != nil {
                        t.Fatal(err)
                }
        }
        request := httptest.NewRequest(method, "http://local.test"+path, &body)
        request.Host = "local.test"
        request.AddCookie(&http.Cookie{Name: "swarm_session", Value: "automation-token"})
        if method != http.MethodGet {
                request.Header.Set("Origin", "http://local.test")
                request.Header.Set("X-Swarm-CSRF", "automation-token")
        }
        response := httptest.NewRecorder()
        handler.ServeHTTP(response, request)
        return response
}

func TestAutomationC04CLIHTTPParity(t *testing.T) {
        s, w := automationC01Fixture(t)
        request := automationC02Create(w, "c04-parity", "once", "2099-01-01T09:00:00", "", "UTC", 1)
        var cliOut, cliErr bytes.Buffer
        code := run([]string{"--root", s.root, "--json", "automation", "preview", "--input", automationC04Input(t, request)}, &cliOut, &cliErr)
        if code != 0 {
                t.Fatalf("CLI preview code=%d stderr=%s", code, cliErr.String())
        }
        var cliPreview AutomationPreviewView
        if err := json.Unmarshal(cliOut.Bytes(), &cliPreview); err != nil {
                t.Fatal(err)
        }
        handler := newWebHandler(s, "local.test", "automation-token")
        response := automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/preview?count=5", request)
        if response.Code != http.StatusOK {
                t.Fatalf("HTTP preview=%d %s", response.Code, response.Body.String())
        }
        var httpPreview AutomationPreviewView
        if err := json.Unmarshal(response.Body.Bytes(), &httpPreview); err != nil {
                t.Fatal(err)
        }
        if cliPreview.PreviewToken != httpPreview.PreviewToken || len(cliPreview.OccurrencesUTC) != 1 || cliPreview.CreatesEnabled || cliPreview.Target.CostState != "unknown" || !cliPreview.Target.Authorized {
                t.Fatalf("parité/projection preview invalide: CLI=%+v HTTP=%+v", cliPreview, httpPreview)
        }
        cliOut.Reset()
        cliErr.Reset()
        command := AutomationCreateCommand{Schedule: request, PreviewToken: cliPreview.PreviewToken}
        code = run([]string{"--root", s.root, "--json", "automation", "create", "--input", automationC04Input(t, command)}, &cliOut, &cliErr)
        if code != 0 {
                t.Fatalf("CLI create code=%d stderr=%s", code, cliErr.String())
        }
        var created AutomationProgramView
        if err := json.Unmarshal(cliOut.Bytes(), &created); err != nil {
                t.Fatal(err)
        }
        if created.Schedule.State != "disabled" || created.Schedule.ScheduleID == "" {
                t.Fatalf("création non désactivée: %+v", created)
        }
        response = automationC04HTTP(t, handler, http.MethodGet, "/api/v1/automation/programs?id="+created.Schedule.ScheduleID, nil)
        if response.Code != http.StatusOK {
                t.Fatalf("HTTP show=%d %s", response.Code, response.Body.String())
        }
        var shown AutomationProgramView
        if err := json.Unmarshal(response.Body.Bytes(), &shown); err != nil {
                t.Fatal(err)
        }
        if shown.Schedule != created.Schedule || shown.Target.MissionState != "not_validated" {
                t.Fatalf("CLI/HTTP divergent: created=%+v shown=%+v", created, shown)
        }
        listed, err := s.automationService().List()
        if err != nil || len(listed) != 1 || listed[0].Schedule.ScheduleID != created.Schedule.ScheduleID {
                t.Fatalf("nonempty list failed: %+v %v", listed, err)
        }
        base := automationC02Time(t, "2098-12-31T08:00:00Z")
        enabled, err := s.automationService().SetState(AutomationStateCommand{ScheduleID: created.Schedule.ScheduleID, State: "enabled", ExpectedRevision: created.Schedule.Revision}, func() time.Time { return base })
        if err != nil {
                t.Fatal(err)
        }
        if err = s.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2099-01-01T09:00:10Z") }); err != nil {
                t.Fatal(err)
        }
        withOccurrence, err := s.automationService().Show(enabled.Schedule.ScheduleID)
        if err != nil || len(withOccurrence.Occurrences) != 1 || withOccurrence.Occurrences[0].MissionState == "validated_closed" {
                t.Fatalf("occurrence non projetée séparément: %+v %v", withOccurrence, err)
        }
        occurrence := withOccurrence.Occurrences[0]
        response = automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/cancel", AutomationCancelCommand{OccurrenceID: occurrence.OccurrenceID, ExpectedRevision: occurrence.Revision})
        if response.Code != http.StatusOK || !strings.Contains(response.Body.String(), `"state":"cancelled"`) {
                t.Fatalf("annulation en attente refusée: %d %s", response.Code, response.Body.String())
        }
        currentConfig, err := s.automationConfig()
        if err != nil {
                t.Fatal(err)
        }
        values := currentConfig.Values
        values.DueGraceSeconds++
        cliOut.Reset()
        cliErr.Reset()
        code = run([]string{"--root", s.root, "--json", "automation", "params", "apply", "--input", automationC04Input(t, AutomationConfigCommand{Schema: 1, ExpectedRevision: currentConfig.Revision, Values: values})}, &cliOut, &cliErr)
        if code != 0 {
                t.Fatalf("CLI params apply=%d %s", code, cliErr.String())
        }
        response = automationC04HTTP(t, handler, http.MethodGet, "/api/v1/automation/config", nil)
        if response.Code != http.StatusOK {
                t.Fatalf("HTTP config=%d %s", response.Code, response.Body.String())
        }
        var config AutomationConfig
        if err = json.Unmarshal(response.Body.Bytes(), &config); err != nil || config.Revision != currentConfig.Revision+1 || config.Values.DueGraceSeconds != values.DueGraceSeconds {
                t.Fatalf("configuration CLI/HTTP divergente: %+v %v", config, err)
        }
        var agents int
        if err := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
                t.Fatalf("preview/create a lancé un agent: %d %v", agents, err)
        }
}

func TestAutomationC04BilingualHelp(t *testing.T) {
        previous, present := os.LookupEnv("SWARM_LANG")
        defer func() {
                if present {
                        _ = os.Setenv("SWARM_LANG", previous)
                } else {
                        _ = os.Unsetenv("SWARM_LANG")
                }
        }()
        for _, test := range []struct{ lang, expected string }{{"fr", "Créer désactivé"}, {"en", "Create disabled"}} {
                if err := os.Setenv("SWARM_LANG", test.lang); err != nil {
                        t.Fatal(err)
                }
                var out, stderr bytes.Buffer
                if code := run([]string{"automation", "help"}, &out, &stderr); code != 0 || !strings.Contains(out.String(), test.expected) {
                        t.Fatalf("aide %s absente: code=%d out=%q err=%q", test.lang, code, out.String(), stderr.String())
                }
        }
}

func TestAutomationC04InvalidInputs(t *testing.T) {
        s, w := automationC01Fixture(t)
        handler := newWebHandler(s, "local.test", "automation-token")
        request := automationC02Create(w, "c04-invalid", "once", "2099-01-01T09:00:00", "", "UTC", 1)
        response := automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/create", AutomationCreateCommand{Schedule: request, PreviewToken: "invented"})
        if response.Code != http.StatusConflict || !strings.Contains(response.Body.String(), "preview_stale") {
                t.Fatalf("création sans aperçu acceptée: %d %s", response.Code, response.Body.String())
        }
        response = automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/preview", map[string]any{"schema_version": 1, "unknown": true})
        if response.Code != http.StatusBadRequest || !strings.Contains(response.Body.String(), "invalid_input") {
                t.Fatalf("champ étranger accepté: %d %s", response.Code, response.Body.String())
        }
        preview, err := s.automationService().Preview(request, 1, nil)
        if err != nil {
                t.Fatal(err)
        }
        created, err := s.automationService().Create(AutomationCreateCommand{Schedule: request, PreviewToken: preview.PreviewToken}, nil)
        if err != nil {
                t.Fatal(err)
        }
        response = automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/state", AutomationStateCommand{ScheduleID: created.Schedule.ScheduleID, State: "paused", ExpectedRevision: 99})
        if response.Code != http.StatusConflict || !strings.Contains(response.Body.String(), "revision_conflict") {
                t.Fatalf("conflit non exposé: %d %s", response.Code, response.Body.String())
        }
        var after AutomationScheduleRecord
        after, err = s.automationSchedule(created.Schedule.ScheduleID)
        if err != nil || after.State != "disabled" || after.Revision != created.Schedule.Revision {
                t.Fatalf("conflit a écrasé le programme: %+v %v", after, err)
        }
}

func TestAutomationC04SessionCSRFAndClaimedCancellation(t *testing.T) {
        s, w := automationC01Fixture(t)
        handler := newWebHandler(s, "local.test", "automation-token")

        unauthenticated := httptest.NewRequest(http.MethodGet, "http://local.test/api/v1/automation/programs", nil)
        unauthenticated.Host = "local.test"
        response := httptest.NewRecorder()
        handler.ServeHTTP(response, unauthenticated)
        if response.Code != http.StatusForbidden {
                t.Fatalf("lecture sans session autorisée: %d %s", response.Code, response.Body.String())
        }

        configBefore, err := s.automationConfig()
        if err != nil {
                t.Fatal(err)
        }
        values := configBefore.Values
        values.DueGraceSeconds++
        var body bytes.Buffer
        if err = json.NewEncoder(&body).Encode(AutomationConfigCommand{Schema: 1, ExpectedRevision: configBefore.Revision, Values: values}); err != nil {
                t.Fatal(err)
        }
        missingCSRF := httptest.NewRequest(http.MethodPost, "http://local.test/api/v1/automation/config", &body)
        missingCSRF.Host = "local.test"
        missingCSRF.Header.Set("Origin", "http://local.test")
        missingCSRF.AddCookie(&http.Cookie{Name: "swarm_session", Value: "automation-token"})
        response = httptest.NewRecorder()
        handler.ServeHTTP(response, missingCSRF)
        if response.Code != http.StatusForbidden {
                t.Fatalf("mutation sans CSRF autorisée: %d %s", response.Code, response.Body.String())
        }
        configAfter, err := s.automationConfig()
        if err != nil || configAfter.Revision != configBefore.Revision || configAfter.Values != configBefore.Values {
                t.Fatalf("mutation refusée a modifié les réglages: avant=%+v après=%+v err=%v", configBefore, configAfter, err)
        }

        request := automationC02Create(w, "c04-claimed", "once", "2099-01-01T09:00:00", "", "UTC", 1)
        preview, err := s.automationService().Preview(request, 1, nil)
        if err != nil {
                t.Fatal(err)
        }
        created, err := s.automationService().Create(AutomationCreateCommand{Schedule: request, PreviewToken: preview.PreviewToken}, nil)
        if err != nil {
                t.Fatal(err)
        }
        base := automationC02Time(t, "2098-12-31T08:00:00Z")
        if _, err = s.automationService().SetState(AutomationStateCommand{ScheduleID: created.Schedule.ScheduleID, State: "enabled", ExpectedRevision: created.Schedule.Revision}, func() time.Time { return base }); err != nil {
                t.Fatal(err)
        }
        if err = s.tickAutomationSchedules(func() time.Time { return automationC02Time(t, "2099-01-01T09:00:10Z") }); err != nil {
                t.Fatal(err)
        }
        program, err := s.automationService().Show(created.Schedule.ScheduleID)
        if err != nil || len(program.Occurrences) != 1 {
                t.Fatalf("occurrence attendue: %+v err=%v", program, err)
        }
        occurrence := program.Occurrences[0]
        if claimed, owned, claimErr := s.claimAutomationRequest(occurrence.RequestID, "c04-driver", time.Now(), defaultAutomationRequestConfig()); claimErr != nil || !owned || claimed.State != "claimed" {
                t.Fatalf("prise en charge préalable: %+v owned=%v err=%v", claimed, owned, claimErr)
        }
        program, err = s.automationService().Show(created.Schedule.ScheduleID)
        if err != nil || len(program.Occurrences) != 1 || program.Occurrences[0].State != "claimed" {
                t.Fatalf("occurrence prise en charge non relue: %+v err=%v", program, err)
        }
        occurrence = program.Occurrences[0]
        response = automationC04HTTP(t, handler, http.MethodPost, "/api/v1/automation/cancel", AutomationCancelCommand{OccurrenceID: occurrence.OccurrenceID, ExpectedRevision: occurrence.Revision})
        if response.Code != http.StatusBadRequest || !strings.Contains(response.Body.String(), "effect_started") {
                t.Fatalf("annulation après prise en charge non refusée: %d %s", response.Code, response.Body.String())
        }
        var state string
        var revision int
        if err = s.db.QueryRow("SELECT state,revision FROM automation_occurrences WHERE occurrence_id=?", occurrence.OccurrenceID).Scan(&state, &revision); err != nil || state != "claimed" || revision != occurrence.Revision {
                t.Fatalf("refus d'annulation a modifié l'occurrence: state=%s revision=%d err=%v", state, revision, err)
        }
}

func TestAutomationC04EmptyLists(t *testing.T) {
        s, _ := automationC01Fixture(t)
        list, err := s.automationService().List()
        if err != nil {
                t.Fatal(err)
        }
        raw, err := json.Marshal(list)
        if err != nil || string(raw) != "[]" {
                t.Fatalf("empty list must be [] for web rendering, got %s: %v", raw, err)
        }
}
func TestAutomationC04ConfigConcurrentConflict(t *testing.T) {
        s, _ := automationC01Fixture(t)
        current, err := s.automationConfig()
        if err != nil {
                t.Fatal(err)
        }
        start := make(chan struct{})
        results := make(chan error, 12)
        var wg sync.WaitGroup
        for i := 0; i < 12; i++ {
                wg.Add(1)
                go func(i int) {
                        defer wg.Done()
                        <-start
                        values := current.Values
                        values.DueGraceSeconds += i + 1
                        _, err := s.automationService().ApplyConfig(AutomationConfigCommand{Schema: 1, ExpectedRevision: current.Revision, Values: values})
                        results <- err
                }(i)
        }
        close(start)
        wg.Wait()
        close(results)
        successes := 0
        for err := range results {
                if err == nil {
                        successes++
                } else {
                        var e *CommandError
                        if !errors.As(err, &e) || e.Code != "revision_conflict" {
                                t.Fatalf("unexpected error %v", err)
                        }
                }
        }
        if successes != 1 {
                t.Fatalf("same revision overwrote concurrent edits: %d successes", successes)
        }
        after, err := s.automationConfig()
        if err != nil || after.Revision != current.Revision+1 || len(after.History) != len(current.History)+1 {
                t.Fatalf("history/revision lost: %+v %v", after, err)
        }
}
```

## Complément moteur supervision — images réellement jointes Codex

### independent_review_images.go complet
```go
//go:build linux

package main

import (
        "bytes"
        "encoding/base64"
        "encoding/json"
        "fmt"
        "image"
        _ "image/jpeg"
        _ "image/png"
        "io"
        "os"
        "path/filepath"
        "strings"
)

// Only operator-declared, receipt-bound screenshots are attached. A report's
// links never authorize reading or transmitting more files.
type reviewImage struct {
        Path      string `json:"path"`
        SHA256    string `json:"sha256"`
        MediaType string `json:"media_type"`
        Data      []byte `json:"-"`
}

func (s *Store) independentReviewImages(t *Task, artifacts map[string]string) ([]reviewImage, error) {
        var images []reviewImage
        seen := map[string]bool{}
        total := 0
        if t.ValidationPolicy == nil {
                return images, nil
        }
        for _, control := range t.ValidationPolicy.Controls {
                for _, name := range control.Inputs {
                        ext := strings.ToLower(filepath.Ext(name))
                        if ext != ".png" && ext != ".jpg" && ext != ".jpeg" {
                                continue
                        }
                        if seen[name] {
                                continue
                        }
                        seen[name] = true
                        if !strings.HasPrefix(name, "docs/screenshots/") {
                                return nil, fmt.Errorf("capture de revue hors docs/screenshots : %s", name)
                        }
                        if len(images) >= 20 {
                                return nil, fmt.Errorf("revue visuelle supérieure à 20 images ; découper la revue")
                        }
                        path, err := safeReport(s.root, name)
                        if err != nil {
                                return nil, err
                        }
                        f, err := os.Open(path)
                        if err != nil {
                                return nil, err
                        }
                        raw, err := io.ReadAll(io.LimitReader(f, 5*1024*1024+1))
                        f.Close()
                        if err != nil {
                                return nil, err
                        }
                        total += len(raw)
                        if len(raw) > 5*1024*1024 || total > 10*1024*1024 {
                                return nil, fmt.Errorf("captures de revue trop volumineuses ; découper la revue")
                        }
                        if artifacts[name] == "" || artifacts[name] != hash(raw) {
                                return nil, fmt.Errorf("capture non liée au contrôle courant : %s", name)
                        }
                        config, format, err := image.DecodeConfig(bytes.NewReader(raw))
                        if err != nil || config.Width <= 0 || config.Height <= 0 || int64(config.Width)*int64(config.Height) > 20000000 {
                                return nil, fmt.Errorf("capture de revue invalide : %s", name)
                        }
                        media := map[string]string{"png": "image/png", "jpeg": "image/jpeg"}[format]
                        if media == "" {
                                return nil, fmt.Errorf("format de capture non pris en charge : %s", name)
                        }
                        images = append(images, reviewImage{Path: name, SHA256: hash(raw), MediaType: media, Data: raw})
                }
        }
        return images, nil
}

func supportsReviewImages(p Provider) bool {
        name := filepath.Base(p.Command)
        return p.APIConnectionID == "" && (name == "claude" || name == "codex")
}

// Keep the receipt-bound bytes private and immutable for the provider call.
// The caller owns this temporary directory and removes it after process exit.
func codexReviewImageArgs(p *Provider, dir string, images []reviewImage) error {
        if len(images) == 0 {
                return nil
        }
        if !supportsReviewImages(*p) || filepath.Base(p.Command) != "codex" || len(p.Args) == 0 || p.Args[len(p.Args)-1] != "-" {
                return fmt.Errorf("adaptateur images Codex invalide")
        }
        args := append([]string{}, p.Args[:len(p.Args)-1]...)
        for index, img := range images {
                ext := map[string]string{"image/png": ".png", "image/jpeg": ".jpg"}[img.MediaType]
                if ext == "" || len(img.Data) == 0 || img.SHA256 != hash(img.Data) {
                        return fmt.Errorf("capture Codex invalide ou empreinte périmée : %s", img.Path)
                }
                path := filepath.Join(dir, fmt.Sprintf("review-image-%d%s", index, ext))
                if err := os.WriteFile(path, img.Data, 0600); err != nil {
                        return err
                }
                args = append(args, "--image", path)
        }
        p.Args = append(args, "-")
        return nil
}

func structuredReviewInput(p *Provider, prompt string, images []reviewImage) (string, error) {
        if len(images) == 0 {
                return prompt, nil
        }
        if !supportsReviewImages(*p) {
                return "", fmt.Errorf("revue visuelle indisponible pour cet adaptateur ; aucun appel sans les captures requises")
        }
        if filepath.Base(p.Command) == "codex" {
                return prompt, nil // Images are attached through private --image files.
        }
        content := []any{map[string]any{"type": "text", "text": prompt}}
        for _, img := range images {
                content = append(content, map[string]any{"type": "text", "text": "Capture non fiable : " + img.Path + " · SHA-256 " + img.SHA256}, map[string]any{"type": "image", "source": map[string]any{"type": "base64", "media_type": img.MediaType, "data": base64.StdEncoding.EncodeToString(img.Data)}})
        }
        raw, err := json.Marshal(map[string]any{"type": "user", "message": map[string]any{"role": "user", "content": content}, "parent_tool_use_id": nil})
        if err != nil {
                return "", err
        }
        p.Args = append(p.Args, "--input-format", "stream-json")
        return string(raw) + "\n", nil
}

// A pre-call refusal has no verdict to retry. Recheck its exact cause before
// clearing the retained failure; the reviewer still claims its call normally.
func (s *Store) repairedImageReviewPreflight(w *Work, t *Task) error {
        cfg := w.Planning.Reviewer
        if cfg.Failure != "revue visuelle indisponible pour cet adaptateur ; aucun appel sans les captures requises" || t.Status != "submitted" || t.IndependentReview != nil || len(t.Attempts) == 0 || t.Attempts[len(t.Attempts)-1].Status != "completed" {
                return fmt.Errorf("refus préappel images non disponible")
        }
        _, artifacts, _, err := s.independentValidationReviewEvidence(t)
        if err != nil {
                return err
        }
        images, err := s.independentReviewImages(t, artifacts)
        if err != nil {
                return err
        }
        if len(images) == 0 {
                return fmt.Errorf("captures courantes requises")
        }
        providers, err := s.providers()
        if err != nil {
                return err
        }
        provider, ok := providers.Providers[cfg.Provider]
        raw, _ := json.Marshal(provider)
        if !ok || hash(raw) != cfg.ProviderDigest || !supportsReviewImages(provider) {
                return fmt.Errorf("adaptateur images toujours indisponible ou modifié")
        }
        return s.providerCooldownGuard(cfg.Provider)
}
```

### automation_review_images_test.go complet
```go
//go:build linux

package main

import (
        "bytes"
        "encoding/json"
        "fmt"
        "os"
        "path/filepath"
        "strings"
        "testing"
        "time"
)

func TestAutomationC04CodexReviewImages(t *testing.T) {
        dir := t.TempDir()
        raw := screenshotFixture(t, dir, "source.png")
        img := reviewImage{Path: "docs/screenshots/c04.png", SHA256: hash(raw), MediaType: "image/png", Data: raw}
        provider := Provider{Command: "/usr/bin/codex", Args: []string{"exec", "--json", "--sandbox", "read-only", "-"}}
        input, err := structuredReviewInput(&provider, "Examine pixels without tools", []reviewImage{img})
        if err != nil || input != "Examine pixels without tools" {
                t.Fatalf("prompt changed: %q %v", input, err)
        }
        private := t.TempDir()
        if err = codexReviewImageArgs(&provider, private, []reviewImage{img, img}); err != nil {
                t.Fatal(err)
        }
        if provider.Args[len(provider.Args)-1] != "-" {
                t.Fatal("stdin prompt lost")
        }
        count := 0
        for index, arg := range provider.Args {
                if arg != "--image" {
                        continue
                }
                count++
                path := provider.Args[index+1]
                data, err := os.ReadFile(path)
                info, statErr := os.Stat(path)
                if err != nil || statErr != nil || !bytes.Equal(raw, data) || info.Mode().Perm() != 0600 || filepath.Dir(path) != private {
                        t.Fatalf("image bytes/privacy not preserved: %s %v %v", path, err, statErr)
                }
        }
        if count != 2 {
                t.Fatalf("image count=%d", count)
        }
        bad := img
        bad.SHA256 = hash([]byte("changed"))
        if err = codexReviewImageArgs(&provider, private, []reviewImage{bad}); err == nil {
                t.Fatal("stale bytes admitted")
        }
        if supportsReviewImages(Provider{Command: "/usr/bin/codex", APIConnectionID: "api"}) {
                t.Fatal("unverified API image support advertised")
        }
}

func TestAutomationC04CodexReviewImagesProcess(t *testing.T) {
        dir := t.TempDir()
        raw := screenshotFixture(t, dir, "source.png")
        capture := filepath.Join(dir, "observed.json")
        command := filepath.Join(dir, "codex")
        script := fmt.Sprintf(`#!/usr/bin/python3
import json,sys,pathlib,hashlib
args=sys.argv[1:]
images=[args[i+1] for i,a in enumerate(args) if a=='--image']
data={'args':args,'prompt':sys.stdin.read(),'images':[{'path':p,'sha256':hashlib.sha256(pathlib.Path(p).read_bytes()).hexdigest()} for p in images]}
pathlib.Path(%q).write_text(json.dumps(data))
print(json.dumps({'type':'item.completed','item':{'type':'agent_message','text':'{"ok":true}'}}))
print(json.dumps({'type':'turn.completed','usage':{'input_tokens':1,'output_tokens':1}}))
`, capture)
        if err := os.WriteFile(command, []byte(script), 0700); err != nil {
                t.Fatal(err)
        }
        started := time.Now()
        reply, err := runStructuredProviderImagesClock(Provider{Command: command}, nil, "Inspect actual pixels", `{"type":"object","properties":{"ok":{"type":"boolean"}},"required":["ok"],"additionalProperties":false}`, []reviewImage{{Path: "docs/screenshots/current.png", SHA256: hash(raw), MediaType: "image/png", Data: raw}}, 10*time.Second, func() bool { return true }, nil, func() time.Duration { return time.Since(started) })
        if err != nil || !strings.Contains(reply, `"ok":true`) {
                t.Fatalf("reply=%s err=%v", reply, err)
        }
        var observed struct {
                Args   []string `json:"args"`
                Prompt string   `json:"prompt"`
                Images []struct {
                        Path   string `json:"path"`
                        SHA256 string `json:"sha256"`
                } `json:"images"`
        }
        b, err := os.ReadFile(capture)
        if err != nil {
                t.Fatal(err)
        }
        if err = json.Unmarshal(b, &observed); err != nil {
                t.Fatal(err)
        }
        if observed.Prompt != "Inspect actual pixels" || len(observed.Images) != 1 || observed.Images[0].SHA256 != hash(raw) {
                t.Fatalf("actual process attachment lost: %+v", observed)
        }
        args := strings.Join(observed.Args, "|")
        if !strings.Contains(args, "--sandbox|read-only") || !strings.Contains(args, "--ephemeral") || !strings.Contains(args, "--output-schema|") || strings.Contains(args, "resume") {
                t.Fatalf("review guard changed: %s", args)
        }
        if _, err = os.Stat(observed.Images[0].Path); !os.IsNotExist(err) {
                t.Fatalf("temporary image survived provider exit: %v", err)
        }
}

```

### Raccord exact planning_runner.go : répertoire privé nettoyé après processus, schéma et images
```go
        defer os.RemoveAll(dir)
        if filepath.Base(p.Command) == "codex" {
                if err = codexReviewImageArgs(&p, dir, images); err != nil {
                        return "", err
                }
                schemaPath := filepath.Join(dir, "planning-schema.json")
                if err = os.WriteFile(schemaPath, []byte(schema), 0600); err != nil {
                        return "", err
                }
                p.Args = append(p.Args[:len(p.Args)-1], "--output-schema", schemaPath, "-")
        } else {
                p.Args = append(p.Args, "--json-schema", schema)
        }
```

### Annexe guide courant docs/AUTOMATION-PROGRAMS.md
```markdown
## Captures et revue indépendante

Les quatre captures FR/EN sombre/État sont fournies comme fichiers PNG déclarés sous `docs/screenshots/` dans les entrées de contrôle. Le moteur vérifie leurs empreintes, dimensions et tailles avant la revue. Un simple lien dans un rapport ne joint aucune image. Codex reçoit les octets via `--image` dans un répertoire privé temporaire, effacé après le processus ; Claude les reçoit dans le message structuré. La revue garde ses outils désactivés et le même budget. Une adaptation API ou fournisseur non vérifiée est refusée. Les fixtures ne prouvent pas la disponibilité d’un abonnement.

```

### Annexe guide courant docs/en/AUTOMATION-PROGRAMS.md
```markdown
## Screenshots and independent review

The four FR/EN dark/light screenshots are declared PNG files under `docs/screenshots/` in control inputs. The engine validates hashes, dimensions and sizes before review. A report link alone attaches no image. Codex receives the bytes through `--image` in a private temporary directory removed after process exit; Claude receives them in the structured message. Review tools remain disabled and the budget unchanged. Unverified API/provider adapters are refused. Fixtures do not prove subscription availability.

```

### Raccord exact independent_review.go
```go
                if v == nil && cfg.Failure != "" && r.ConfirmReviewErrorRepair {
                        if err := s.repairedImageReviewPreflight(w, t); err != nil {
                                return err
                        }
                        cfg.Failure = ""
                        return nil
                }
```
TestAutomationC04ImagePreflightRecovery complet fourni par sortie de contrôle liée.
