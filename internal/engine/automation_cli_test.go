//go:build linux

package engine

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
