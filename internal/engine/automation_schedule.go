package engine

import (
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	resources "swarm.local/companion"
	"syscall"
	"time"
)

const automationScheduleMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS automation_schedules(
 schedule_id TEXT PRIMARY KEY,
 event_id TEXT NOT NULL UNIQUE,
 name TEXT NOT NULL,
 target_work_id TEXT NOT NULL REFERENCES works(id),
 action TEXT NOT NULL CHECK(action='request_resume'),
 kind TEXT NOT NULL CHECK(kind IN ('once','daily','weekly')),
 timezone TEXT NOT NULL,
 start_local TEXT NOT NULL,
 until_local TEXT NOT NULL,
 max_occurrences INTEGER NOT NULL,
 missed_policy TEXT NOT NULL CHECK(missed_policy='skip'),
 concurrency_policy TEXT NOT NULL CHECK(concurrency_policy='coalesce'),
 state TEXT NOT NULL CHECK(state IN ('disabled','enabled','paused','archived')),
 revision INTEGER NOT NULL,
 next_at TEXT NOT NULL,
 emitted_count INTEGER NOT NULL,
 last_observed_at TEXT NOT NULL,
 created_at TEXT NOT NULL,
 updated_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS automation_schedule_journal(
 journal_id TEXT PRIMARY KEY,
 schedule_id TEXT NOT NULL REFERENCES automation_schedules(schedule_id),
 intended_at TEXT NOT NULL,
 observed_at TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('skipped','submitted','coalesced','held')),
 request_id TEXT NOT NULL,
 source TEXT NOT NULL,
 reason TEXT NOT NULL,
 UNIQUE(schedule_id,intended_at,state)
);
CREATE TABLE IF NOT EXISTS automation_request_origins(
 request_id TEXT NOT NULL REFERENCES automation_requests(request_id),
 schedule_id TEXT NOT NULL REFERENCES automation_schedules(schedule_id),
 intended_at TEXT NOT NULL,
 source TEXT NOT NULL,
 reason TEXT NOT NULL,
 PRIMARY KEY(request_id,schedule_id,intended_at)
);
CREATE TABLE IF NOT EXISTS automation_causal_recoveries(
 recovery_id TEXT PRIMARY KEY,
 request_id TEXT NOT NULL REFERENCES automation_requests(request_id),
 previous_reason TEXT NOT NULL,
 evidence_digest TEXT NOT NULL,
 evidence_kind TEXT NOT NULL,
 observed_at TEXT NOT NULL,
 UNIQUE(request_id,evidence_digest)
);
CREATE INDEX IF NOT EXISTS automation_schedules_due ON automation_schedules(state,next_at);
CREATE INDEX IF NOT EXISTS automation_schedule_journal_schedule ON automation_schedule_journal(schedule_id,observed_at);
PRAGMA user_version=26;
COMMIT;`

var automationConfigDefaults = resources.Automation

type AutomationConfigValues struct {
	ClaimLeaseSeconds              int `json:"claim_lease_seconds"`
	DueGraceSeconds                int `json:"due_grace_seconds"`
	MaxPreviewOccurrences          int `json:"max_preview_occurrences"`
	MaxScheduleOccurrences         int `json:"max_schedule_occurrences"`
	ExternalMaxPayloadBytes        int `json:"external_max_payload_bytes"`
	ExternalRateLimit              int `json:"external_rate_limit"`
	ExternalRateWindowSeconds      int `json:"external_rate_window_seconds"`
	ExternalTimestampWindowSeconds int `json:"external_timestamp_window_seconds"`
}

type AutomationConfigRevision struct {
	Revision int                    `json:"revision"`
	Values   AutomationConfigValues `json:"values"`
	Actor    string                 `json:"actor"`
	At       string                 `json:"at"`
}

type AutomationConfig struct {
	Schema   int                        `json:"schema_version"`
	Revision int                        `json:"revision"`
	Values   AutomationConfigValues     `json:"values"`
	History  []AutomationConfigRevision `json:"history"`
}

func validateAutomationConfigValues(v AutomationConfigValues) bool {
	return v.ClaimLeaseSeconds >= 5 && v.ClaimLeaseSeconds <= 300 && v.DueGraceSeconds >= 0 && v.DueGraceSeconds <= 3600 && v.MaxPreviewOccurrences >= 1 && v.MaxPreviewOccurrences <= 100 && v.MaxScheduleOccurrences >= 1 && v.MaxScheduleOccurrences <= 10000 && v.ExternalMaxPayloadBytes >= 256 && v.ExternalMaxPayloadBytes <= 1048576 && v.ExternalRateLimit >= 1 && v.ExternalRateLimit <= 10000 && v.ExternalRateWindowSeconds >= 1 && v.ExternalRateWindowSeconds <= 3600 && v.ExternalTimestampWindowSeconds >= 1 && v.ExternalTimestampWindowSeconds <= 3600
}

func validateAutomationConfig(c AutomationConfig) error {
	if c.Schema != 1 || c.Revision < 0 || c.Revision != len(c.History) || !validateAutomationConfigValues(c.Values) {
		return fmt.Errorf("configuration automation invalide : schema 1 et bornes opérationnelles C01-C03 requises")
	}
	for i, h := range c.History {
		if h.Revision != i+1 || !validateAutomationConfigValues(h.Values) {
			return fmt.Errorf("historique de configuration automation invalide")
		}
	}
	if c.Revision > 0 && c.History[len(c.History)-1].Values != c.Values {
		return fmt.Errorf("valeurs et historique de configuration automation divergents")
	}
	return nil
}

func (s *Store) automationConfig() (AutomationConfig, error) {
	var c AutomationConfig
	raw := automationConfigDefaults
	path := filepath.Join(s.root, ".swarm", "automation.json")
	if st, err := os.Lstat(path); err == nil {
		if !st.Mode().IsRegular() || st.Size() > 1048576 {
			return c, fmt.Errorf("automation.json doit être un fichier local borné")
		}
		if raw, err = os.ReadFile(path); err != nil {
			return c, err
		}
	} else if !os.IsNotExist(err) {
		return c, err
	}
	if err := strict(raw, &c); err != nil {
		return c, err
	}
	if err := validateAutomationConfig(c); err != nil {
		return c, err
	}
	return c, nil
}

func (c AutomationConfig) requestConfig() AutomationRequestConfig {
	return AutomationRequestConfig{Version: 1, ClaimLeaseSeconds: c.Values.ClaimLeaseSeconds}
}

type AutomationScheduleSpec struct {
	Kind           string `json:"kind"`
	LocalTime      string `json:"local_time"`
	UntilLocal     string `json:"until_local,omitempty"`
	MaxOccurrences int    `json:"max_occurrences"`
}

type AutomationScheduleCreate struct {
	Schema            int                    `json:"schema_version"`
	EventID           string                 `json:"event_id"`
	Name              string                 `json:"name"`
	TargetWorkID      string                 `json:"target_work_id"`
	Action            string                 `json:"action"`
	Timezone          string                 `json:"timezone"`
	Schedule          AutomationScheduleSpec `json:"schedule"`
	MissedPolicy      string                 `json:"missed_policy"`
	ConcurrencyPolicy string                 `json:"concurrency_policy"`
}

type AutomationScheduleRecord struct {
	ScheduleID        string `json:"schedule_id"`
	EventID           string `json:"event_id"`
	Name              string `json:"name"`
	TargetWorkID      string `json:"target_work_id"`
	Action            string `json:"action"`
	Kind              string `json:"kind"`
	Timezone          string `json:"timezone"`
	StartLocal        string `json:"start_local"`
	UntilLocal        string `json:"until_local,omitempty"`
	MaxOccurrences    int    `json:"max_occurrences"`
	MissedPolicy      string `json:"missed_policy"`
	ConcurrencyPolicy string `json:"concurrency_policy"`
	State             string `json:"state"`
	Revision          int    `json:"revision"`
	NextAt            string `json:"next_at,omitempty"`
	EmittedCount      int    `json:"emitted_count"`
	LastObservedAt    string `json:"last_observed_at,omitempty"`
	CreatedAt         string `json:"created_at"`
	UpdatedAt         string `json:"updated_at"`
}

type AutomationSchedulePreview struct {
	Timezone            string   `json:"timezone"`
	OccurrencesUTC      []string `json:"occurrences_utc"`
	MissedPolicy        string   `json:"missed_policy"`
	ConcurrencyPolicy   string   `json:"concurrency_policy"`
	DSTPolicy           string   `json:"dst_policy"`
	ClockBackwardPolicy string   `json:"clock_backward_policy"`
	CreatesEnabled      bool     `json:"creates_enabled"`
}

type automationClock func() time.Time

func realAutomationClock() time.Time { return time.Now() }

const automationLocalLayout = "2006-01-02T15:04:05"

func resolveAutomationLocal(value string, loc *time.Location) (time.Time, error) {
	want, err := time.Parse(automationLocalLayout, value)
	if err != nil {
		return time.Time{}, &CommandError{Code: "invalid_schedule", Message: "date locale attendue au format YYYY-MM-DDTHH:MM:SS"}
	}
	candidate := time.Date(want.Year(), want.Month(), want.Day(), want.Hour(), want.Minute(), want.Second(), 0, loc)
	if candidate.In(loc).Format(automationLocalLayout) != value {
		return time.Time{}, &CommandError{Code: "nonexistent_local_time", Message: "heure locale inexistante pendant une transition DST ; choisir un instant explicite différent"}
	}
	matches := 0
	for minute := -180; minute <= 180; minute++ {
		probe := candidate.Add(time.Duration(minute) * time.Minute)
		if probe.In(loc).Format(automationLocalLayout) == value {
			matches++
		}
	}
	if matches > 1 {
		return time.Time{}, &CommandError{Code: "ambiguous_local_time", Message: "heure locale ambiguë pendant une transition DST ; choisir un instant non ambigu"}
	}
	return candidate.UTC(), nil
}

func validateAutomationScheduleInput(r AutomationScheduleCreate, cfg AutomationConfig) (*time.Location, error) {
	if r.Schema != 1 || !safeName(r.EventID) || !safeName(r.TargetWorkID) || strings.TrimSpace(r.Name) == "" || len([]byte(r.Name)) > 200 || r.Action != "request_resume" || r.MissedPolicy != "skip" || r.ConcurrencyPolicy != "coalesce" {
		return nil, &CommandError{Code: "invalid_input", Message: "schema, event_id, nom, cible, request_resume, missed_policy=skip et concurrency_policy=coalesce requis"}
	}
	loc, err := time.LoadLocation(r.Timezone)
	if err != nil || r.Timezone == "Local" {
		return nil, &CommandError{Code: "invalid_timezone", Message: "fuseau IANA explicite requis ; UTC est accepté"}
	}
	spec := r.Schedule
	if spec.Kind != "once" && spec.Kind != "daily" && spec.Kind != "weekly" {
		return nil, &CommandError{Code: "invalid_schedule", Message: "kind doit valoir once, daily ou weekly"}
	}
	if spec.Kind == "once" {
		if spec.MaxOccurrences != 1 || spec.UntilLocal != "" {
			return nil, &CommandError{Code: "invalid_schedule", Message: "once exige max_occurrences=1 et aucun until_local"}
		}
	} else if spec.MaxOccurrences < 1 || spec.MaxOccurrences > cfg.Values.MaxScheduleOccurrences || spec.UntilLocal == "" {
		return nil, &CommandError{Code: "invalid_schedule", Message: "récurrence bornée : until_local et max_occurrences dans la limite configurée requis"}
	}
	start, err := resolveAutomationLocal(spec.LocalTime, loc)
	if err != nil {
		return nil, err
	}
	if spec.UntilLocal != "" {
		until, resolveErr := resolveAutomationLocal(spec.UntilLocal, loc)
		if resolveErr != nil {
			return nil, resolveErr
		}
		if until.Before(start) {
			return nil, &CommandError{Code: "invalid_schedule", Message: "until_local doit suivre local_time"}
		}
	}
	return loc, nil
}

func automationOccurrence(spec AutomationScheduleSpec, loc *time.Location, index int) (time.Time, error) {
	base, err := time.Parse(automationLocalLayout, spec.LocalTime)
	if err != nil {
		return time.Time{}, err
	}
	switch spec.Kind {
	case "daily":
		base = base.AddDate(0, 0, index)
	case "weekly":
		base = base.AddDate(0, 0, 7*index)
	case "once":
		if index > 0 {
			return time.Time{}, sql.ErrNoRows
		}
	}
	local := base.Format(automationLocalLayout)
	if spec.UntilLocal != "" && local > spec.UntilLocal {
		return time.Time{}, sql.ErrNoRows
	}
	return resolveAutomationLocal(local, loc)
}

func previewAutomationSchedule(r AutomationScheduleCreate, after time.Time, count int, cfg AutomationConfig) (AutomationSchedulePreview, error) {
	preview := AutomationSchedulePreview{Timezone: r.Timezone, MissedPolicy: r.MissedPolicy, ConcurrencyPolicy: r.ConcurrencyPolicy, DSTPolicy: "reject_ambiguous_or_nonexistent", ClockBackwardPolicy: "hold_until_last_observed", CreatesEnabled: false}
	loc, err := validateAutomationScheduleInput(r, cfg)
	if err != nil {
		return preview, err
	}
	if count < 1 || count > cfg.Values.MaxPreviewOccurrences {
		return preview, &CommandError{Code: "invalid_input", Message: "nombre de prévisualisations hors limite configurée"}
	}
	for i := 0; i < r.Schedule.MaxOccurrences && len(preview.OccurrencesUTC) < count; i++ {
		occurrence, occurrenceErr := automationOccurrence(r.Schedule, loc, i)
		if errors.Is(occurrenceErr, sql.ErrNoRows) {
			break
		}
		if occurrenceErr != nil {
			return preview, occurrenceErr
		}
		if occurrence.After(after) {
			preview.OccurrencesUTC = append(preview.OccurrencesUTC, occurrence.Format(time.RFC3339))
		}
	}
	return preview, nil
}

func scanAutomationSchedule(row interface{ Scan(...any) error }) (AutomationScheduleRecord, error) {
	var r AutomationScheduleRecord
	err := row.Scan(&r.ScheduleID, &r.EventID, &r.Name, &r.TargetWorkID, &r.Action, &r.Kind, &r.Timezone, &r.StartLocal, &r.UntilLocal, &r.MaxOccurrences, &r.MissedPolicy, &r.ConcurrencyPolicy, &r.State, &r.Revision, &r.NextAt, &r.EmittedCount, &r.LastObservedAt, &r.CreatedAt, &r.UpdatedAt)
	return r, err
}

const automationScheduleSelect = `SELECT schedule_id,event_id,name,target_work_id,action,kind,timezone,start_local,until_local,max_occurrences,missed_policy,concurrency_policy,state,revision,next_at,emitted_count,last_observed_at,created_at,updated_at FROM automation_schedules`

func (s *Store) automationSchedule(id string) (AutomationScheduleRecord, error) {
	return scanAutomationSchedule(s.db.QueryRow(automationScheduleSelect+" WHERE schedule_id=?", id))
}

func (s *Store) createAutomationSchedule(r AutomationScheduleCreate, clock automationClock) (AutomationScheduleRecord, error) {
	var zero AutomationScheduleRecord
	cfg, err := s.automationConfig()
	if err != nil {
		return zero, err
	}
	if clock == nil {
		clock = realAutomationClock
	}
	nowAt := clock().UTC()
	preview, err := previewAutomationSchedule(r, nowAt, 1, cfg)
	if err != nil {
		return zero, err
	}
	if len(preview.OccurrencesUTC) == 0 {
		return zero, &CommandError{Code: "schedule_expired", Message: "aucune occurrence future dans la borne demandée"}
	}
	if _, err = s.get(r.TargetWorkID); err != nil {
		return zero, &CommandError{Code: "unknown_target", Message: "cible inconnue"}
	}
	scheduleID := "schedule-" + hash([]byte(r.EventID))[:24]
	stamp := nowAt.Format(time.RFC3339Nano)
	_, err = s.db.Exec(`INSERT INTO automation_schedules(schedule_id,event_id,name,target_work_id,action,kind,timezone,start_local,until_local,max_occurrences,missed_policy,concurrency_policy,state,revision,next_at,emitted_count,last_observed_at,created_at,updated_at)
 VALUES(?,?,?,?,?,?,?,?,?,?,?,?, 'disabled',1,?,0,'',?,?)`, scheduleID, r.EventID, r.Name, r.TargetWorkID, r.Action, r.Schedule.Kind, r.Timezone, r.Schedule.LocalTime, r.Schedule.UntilLocal, r.Schedule.MaxOccurrences, r.MissedPolicy, r.ConcurrencyPolicy, preview.OccurrencesUTC[0], stamp, stamp)
	if err != nil {
		if existing, readErr := scanAutomationSchedule(s.db.QueryRow(automationScheduleSelect+" WHERE event_id=?", r.EventID)); readErr == nil {
			if existing.Name != r.Name || existing.TargetWorkID != r.TargetWorkID || existing.Action != r.Action || existing.Kind != r.Schedule.Kind || existing.Timezone != r.Timezone || existing.StartLocal != r.Schedule.LocalTime || existing.UntilLocal != r.Schedule.UntilLocal || existing.MaxOccurrences != r.Schedule.MaxOccurrences || existing.MissedPolicy != r.MissedPolicy || existing.ConcurrencyPolicy != r.ConcurrencyPolicy {
				return zero, &CommandError{Code: "event_conflict", Message: "event_id déjà utilisé pour un programme différent"}
			}
			return existing, nil
		}
		return zero, err
	}
	return s.automationSchedule(scheduleID)
}

func scheduleSpecFromRecord(r AutomationScheduleRecord) AutomationScheduleSpec {
	return AutomationScheduleSpec{Kind: r.Kind, LocalTime: r.StartLocal, UntilLocal: r.UntilLocal, MaxOccurrences: r.MaxOccurrences}
}

func nextAutomationOccurrence(r AutomationScheduleRecord, after time.Time) (time.Time, int, error) {
	loc, err := time.LoadLocation(r.Timezone)
	if err != nil {
		return time.Time{}, 0, err
	}
	spec := scheduleSpecFromRecord(r)
	for i := r.EmittedCount; i < r.MaxOccurrences; i++ {
		occurrence, occurrenceErr := automationOccurrence(spec, loc, i)
		if errors.Is(occurrenceErr, sql.ErrNoRows) {
			break
		}
		if occurrenceErr != nil {
			return time.Time{}, i, occurrenceErr
		}
		if occurrence.After(after) {
			return occurrence, i, nil
		}
	}
	return time.Time{}, r.MaxOccurrences, sql.ErrNoRows
}

func (s *Store) setAutomationScheduleState(id, state string, expectedRevision int, clock automationClock) (AutomationScheduleRecord, error) {
	var zero AutomationScheduleRecord
	if state != "enabled" && state != "paused" && state != "archived" {
		return zero, &CommandError{Code: "invalid_input", Message: "état demandé invalide"}
	}
	if clock == nil {
		clock = realAutomationClock
	}
	record, err := s.automationSchedule(id)
	if err != nil {
		return zero, err
	}
	unlock, err := s.lockAutomationTarget(record.TargetWorkID)
	if err != nil {
		return zero, err
	}
	defer unlock()
	record, err = s.automationSchedule(id)
	if err != nil {
		return zero, err
	}
	if record.Revision != expectedRevision {
		return zero, &CommandError{Code: "revision_conflict", Message: "programme modifié ; recharger avant confirmation"}
	}
	if record.State == "archived" {
		return zero, &CommandError{Code: "schedule_archived", Message: "un programme archivé ne peut pas être réactivé"}
	}
	next := record.NextAt
	if state == "enabled" {
		tx, beginErr := s.db.Begin()
		if beginErr != nil {
			return zero, beginErr
		}
		authorized, code, policyErr := automationPolicyTx(tx, record.TargetWorkID)
		_ = tx.Rollback()
		if policyErr != nil {
			return zero, policyErr
		}
		if !authorized {
			return zero, &CommandError{Code: code, Message: "autorisation actuelle requise pour activer le programme"}
		}
		occurrence, _, nextErr := nextAutomationOccurrence(record, clock().UTC())
		if nextErr != nil {
			return zero, &CommandError{Code: "schedule_expired", Message: "aucune occurrence future à activer"}
		}
		next = occurrence.Format(time.RFC3339)
	}
	if state == "archived" {
		next = ""
	}
	result, err := s.db.Exec(`UPDATE automation_schedules SET state=?,revision=revision+1,next_at=?,updated_at=? WHERE schedule_id=? AND revision=?`, state, next, clock().UTC().Format(time.RFC3339Nano), id, expectedRevision)
	if err != nil {
		return zero, err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return zero, &CommandError{Code: "revision_conflict", Message: "programme modifié ; recharger avant confirmation"}
	}
	return s.automationSchedule(id)
}

func automationJournalID(schedule, intended, state string) string {
	sum := sha256.Sum256([]byte(schedule + "|" + intended + "|" + state))
	return "schedule-event-" + hex.EncodeToString(sum[:12])
}

func (s *Store) journalAutomationSchedule(schedule, intended string, observed time.Time, state, request, source, reason string) error {
	_, err := s.db.Exec(`INSERT OR IGNORE INTO automation_schedule_journal(journal_id,schedule_id,intended_at,observed_at,state,request_id,source,reason) VALUES(?,?,?,?,?,?,?,?)`, automationJournalID(schedule, intended, state), schedule, intended, observed.UTC().Format(time.RFC3339Nano), state, request, source, reason)
	return err
}

func (s *Store) activeAutomationRequest(target string) (AutomationRequestRecord, error) {
	return scanAutomationRequest(s.db.QueryRow(automationRequestSelect+" WHERE r.target_work_id=? AND r.state IN ('received','waiting','claimed') ORDER BY r.created_at LIMIT 1", target))
}

func (s *Store) advanceAutomationSchedule(record AutomationScheduleRecord, intended time.Time, observed time.Time) error {
	record.EmittedCount++
	record.LastObservedAt = observed.UTC().Format(time.RFC3339Nano)
	record.NextAt = ""
	next, _, err := nextAutomationOccurrence(record, intended)
	state := record.State
	if err == nil {
		record.NextAt = next.Format(time.RFC3339)
	} else if errors.Is(err, sql.ErrNoRows) {
		state = "archived"
	} else {
		return err
	}
	result, err := s.db.Exec(`UPDATE automation_schedules SET state=?,revision=revision+1,next_at=?,emitted_count=?,last_observed_at=?,updated_at=? WHERE schedule_id=? AND revision=? AND next_at=?`, state, record.NextAt, record.EmittedCount, record.LastObservedAt, record.LastObservedAt, record.ScheduleID, record.Revision, intended.Format(time.RFC3339))
	if err != nil {
		return err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return &CommandError{Code: "revision_conflict", Message: "occurrence déjà prise en charge par un autre conducteur"}
	}
	return nil
}

func (s *Store) claimAutomationScheduleTick(record *AutomationScheduleRecord) (bool, error) {
	result, err := s.db.Exec(`UPDATE automation_schedules SET revision=revision+1 WHERE schedule_id=? AND revision=? AND state='enabled' AND next_at=?`, record.ScheduleID, record.Revision, record.NextAt)
	if err != nil {
		return false, err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return false, nil
	}
	record.Revision++
	return true, nil
}

func (s *Store) lockAutomationTarget(target string) (func(), error) {
	if !safeName(target) {
		return nil, &CommandError{Code: "invalid_input", Message: "cible invalide"}
	}
	dir := filepath.Join(s.root, ".swarm", "automation-locks")
	if err := os.MkdirAll(dir, 0700); err != nil {
		return nil, err
	}
	file, err := os.OpenFile(filepath.Join(dir, target+".lock"), os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if err != nil {
		return nil, err
	}
	if err = syscall.Flock(int(file.Fd()), syscall.LOCK_EX); err != nil {
		file.Close()
		return nil, err
	}
	return func() { _ = syscall.Flock(int(file.Fd()), syscall.LOCK_UN); _ = file.Close() }, nil
}

func (s *Store) tickAutomationSchedules(clock automationClock) error {
	if clock == nil {
		clock = realAutomationClock
	}
	cfg, err := s.automationConfig()
	if err != nil {
		return err
	}
	observed := clock().UTC()
	rows, err := s.db.Query(automationScheduleSelect + " WHERE state='enabled' ORDER BY schedule_id")
	if err != nil {
		return err
	}
	var due []AutomationScheduleRecord
	for rows.Next() {
		record, scanErr := scanAutomationSchedule(rows)
		if scanErr != nil {
			rows.Close()
			return scanErr
		}
		due = append(due, record)
	}
	if err = rows.Close(); err != nil {
		return err
	}
	for _, record := range due {
		err = func() error {
			unlock, lockErr := s.lockAutomationTarget(record.TargetWorkID)
			if lockErr != nil {
				return lockErr
			}
			defer unlock()
			current, readErr := s.automationSchedule(record.ScheduleID)
			if readErr != nil {
				return readErr
			}
			record = current
			if record.State != "enabled" {
				return nil
			}
			if record.LastObservedAt != "" {
				last, parseErr := time.Parse(time.RFC3339Nano, record.LastObservedAt)
				if parseErr == nil && observed.Before(last) {
					if err = s.journalAutomationSchedule(record.ScheduleID, record.NextAt, observed, "held", "", "clock", "clock_moved_backward"); err != nil {
						return err
					}
					return nil
				}
			}
			for record.State == "enabled" && record.NextAt != "" {
				intended, parseErr := time.Parse(time.RFC3339, record.NextAt)
				if parseErr != nil {
					return parseErr
				}
				if intended.After(observed) {
					break
				}
				claimed, claimErr := s.claimAutomationScheduleTick(&record)
				if claimErr != nil {
					return claimErr
				}
				if !claimed {
					break
				}
				source := "schedule:" + record.ScheduleID
				if observed.Sub(intended) > time.Duration(cfg.Values.DueGraceSeconds)*time.Second {
					if err = s.journalAutomationSchedule(record.ScheduleID, intended.Format(time.RFC3339), observed, "skipped", "", source, "missed_policy_skip"); err != nil {
						return err
					}
					if err = s.advanceAutomationSchedule(record, intended, observed); err != nil {
						return err
					}
					record, err = s.automationSchedule(record.ScheduleID)
					if err != nil {
						return err
					}
					continue
				}
				active, activeErr := s.activeAutomationRequest(record.TargetWorkID)
				if activeErr == nil {
					_, err = s.db.Exec(`INSERT OR IGNORE INTO automation_request_origins(request_id,schedule_id,intended_at,source,reason) VALUES(?,?,?,?,?)`, active.RequestID, record.ScheduleID, intended.Format(time.RFC3339), source, "mission_active_coalesced")
					if err == nil {
						err = s.journalAutomationSchedule(record.ScheduleID, intended.Format(time.RFC3339), observed, "coalesced", active.RequestID, source, "mission_active_coalesced")
					}
				} else if errors.Is(activeErr, sql.ErrNoRows) {
					key := "schedule-" + hash([]byte(record.ScheduleID + "|" + intended.Format(time.RFC3339)))[:24]
					request := AutomationResumeRequest{Schema: 1, IdempotencyKey: key, Source: source, TargetWorkID: record.TargetWorkID, Action: record.Action}
					request.ContentDigest = automationResumeDigest(request)
					var submitted AutomationRequestRecord
					submitted, _, err = s.submitAutomationResume(request)
					if err == nil {
						_, err = s.db.Exec(`INSERT OR IGNORE INTO automation_request_origins(request_id,schedule_id,intended_at,source,reason) VALUES(?,?,?,?,?)`, submitted.RequestID, record.ScheduleID, intended.Format(time.RFC3339), source, "scheduled_occurrence")
					}
					if err == nil {
						err = s.journalAutomationSchedule(record.ScheduleID, intended.Format(time.RFC3339), observed, "submitted", submitted.RequestID, source, submitted.Reason)
					}
				} else {
					err = activeErr
				}
				if err != nil {
					return err
				}
				if err = s.advanceAutomationSchedule(record, intended, observed); err != nil {
					return err
				}
				break
			}
			return nil
		}()
		if err != nil {
			return err
		}
	}
	return nil
}

type AutomationCausalRecovery struct {
	Schema         int    `json:"schema_version"`
	RequestID      string `json:"request_id"`
	PreviousReason string `json:"previous_reason"`
	EvidenceKind   string `json:"evidence_kind"`
	EvidenceDigest string `json:"evidence_digest"`
}

func (s *Store) recoverAutomationRequestCausally(r AutomationCausalRecovery, clock automationClock) (AutomationRequestRecord, error) {
	var zero AutomationRequestRecord
	if _, err := hex.DecodeString(r.EvidenceDigest); r.Schema != 1 || !safeName(r.RequestID) || r.PreviousReason == "" || !safeName(r.EvidenceKind) || len(r.EvidenceDigest) != 64 || err != nil {
		return zero, &CommandError{Code: "invalid_input", Message: "demande, cause précédente et preuve SHA-256 vérifiée requises"}
	}
	if clock == nil {
		clock = realAutomationClock
	}
	record, err := s.automationRequest(r.RequestID)
	if err != nil {
		return zero, err
	}
	if record.Reason != r.PreviousReason || (record.State != "waiting" && record.State != "rejected") {
		return zero, &CommandError{Code: "causal_change_required", Message: "la cause courante ne correspond pas à la reprise demandée"}
	}
	var prior int
	if err = s.db.QueryRow("SELECT count(*) FROM automation_causal_recoveries WHERE request_id=? AND evidence_digest=?", r.RequestID, r.EvidenceDigest).Scan(&prior); err != nil {
		return zero, err
	}
	if prior != 0 {
		return zero, &CommandError{Code: "causal_change_required", Message: "cette preuve a déjà servi ; un changement pertinent nouveau est requis"}
	}
	tx, err := s.db.Begin()
	if err != nil {
		return zero, err
	}
	defer tx.Rollback()
	if _, err = tx.Exec("UPDATE works SET revision=revision WHERE id=?", record.TargetWorkID); err != nil {
		return zero, err
	}
	var workRaw []byte
	if err = tx.QueryRow("SELECT body FROM works WHERE id=?", record.TargetWorkID).Scan(&workRaw); err != nil {
		return zero, err
	}
	var work Work
	if err = json.Unmarshal(workRaw, &work); err != nil {
		return zero, err
	}
	provider := ""
	if work.Planning != nil {
		provider = work.Planning.Provider
	}
	if provider == "" && work.Profile != nil {
		provider = work.Profile.Provider
	}
	if provider != "" {
		if err = s.providerCooldownGuardAt(provider, clock().UTC()); err != nil {
			return zero, err
		}
	}
	authorized, code, err := automationPolicyTx(tx, record.TargetWorkID)
	if err != nil {
		return zero, err
	}
	if !authorized {
		return zero, &CommandError{Code: code, Message: "la condition actuelle reste bloquante ; budget et autorisation ne sont pas relevés"}
	}
	if record.Reason == "workspace_wait" {
		busy, busyErr := s.automationWorkspaceBusyTx(tx, record.TargetWorkID)
		if busyErr != nil {
			return zero, busyErr
		}
		if busy {
			return zero, &CommandError{Code: "workspace_wait", Message: "l’espace reste occupé"}
		}
	}
	recoveryID := "recovery-" + hash([]byte(r.RequestID + "|" + r.EvidenceDigest))[:24]
	if _, err = tx.Exec(`INSERT INTO automation_causal_recoveries(recovery_id,request_id,previous_reason,evidence_digest,evidence_kind,observed_at) VALUES(?,?,?,?,?,?)`, recoveryID, r.RequestID, r.PreviousReason, r.EvidenceDigest, r.EvidenceKind, clock().UTC().Format(time.RFC3339Nano)); err != nil {
		return zero, err
	}
	if err = updateAutomationStateTx(tx, r.RequestID, "received", "", "causal-recovery", "claim"); err != nil {
		return zero, err
	}
	if _, err = tx.Exec("UPDATE automation_occurrences SET claimed_by='',lease_until='' WHERE request_id=?", r.RequestID); err != nil {
		return zero, err
	}
	if err = tx.Commit(); err != nil {
		return zero, err
	}
	return s.automationRequest(r.RequestID)
}

func automationEvidenceDigest(value string) string {
	sum := sha256.Sum256([]byte(value))
	return hex.EncodeToString(sum[:])
}
