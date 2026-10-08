package engine

import (
	"database/sql"
	"encoding/json"
	"fmt"
)

// v23 adds hierarchical (project/mission/role/task) admin overrides for
// RunLimits, with an append-only history so every change stays reviewable
// and reversible without a second source of truth (same Store/db as budgets,
// quotas, pricing and provider admin).
const runLimitsConfigMigration = `BEGIN;
CREATE TABLE IF NOT EXISTS run_limits_config(scope TEXT NOT NULL, mission_id TEXT NOT NULL DEFAULT '', scope_key TEXT NOT NULL DEFAULT '',
 revision INTEGER NOT NULL, body BLOB NOT NULL, actor TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '', updated TEXT NOT NULL,
 PRIMARY KEY(scope,mission_id,scope_key));
CREATE TABLE IF NOT EXISTS run_limits_history(seq INTEGER PRIMARY KEY AUTOINCREMENT, scope TEXT NOT NULL, mission_id TEXT NOT NULL DEFAULT '',
 scope_key TEXT NOT NULL DEFAULT '', revision INTEGER NOT NULL, body BLOB NOT NULL, request BLOB NOT NULL, actor TEXT NOT NULL, reason TEXT NOT NULL DEFAULT '',
 at TEXT NOT NULL, event_id TEXT NOT NULL, rollback_of INTEGER, UNIQUE(scope,mission_id,scope_key,event_id));
CREATE INDEX IF NOT EXISTS run_limits_history_scope ON run_limits_history(scope,mission_id,scope_key,revision);
PRAGMA user_version=23;
COMMIT;`

// Hierarchical scopes for RunLimits administration, most general first.
// Resolution overlays non-zero fields in this order: project < mission < role < task.
// Zero in an override means "inherit from the enclosing scope" (same convention as RunLimits itself).
const (
	ScopeProject = "project"
	ScopeMission = "mission"
	ScopeRole    = "role"
	ScopeTask    = "task"
)

// RunLimitsConfigEntry is the current stored override for one scope.
type RunLimitsConfigEntry struct {
	Scope     string    `json:"scope"`
	MissionID string    `json:"mission_id,omitempty"`
	ScopeKey  string    `json:"scope_key,omitempty"`
	Values    RunLimits `json:"values"`
	Revision  int       `json:"revision"`
	Actor     string    `json:"actor"`
	Reason    string    `json:"reason"`
	Updated   string    `json:"updated"`
}

// RunLimitsConfigChange is an operator request to set (or clear, with zero
// values) the override at one scope. expected_revision=0 requires that no
// override exists yet for that scope (first write); any other value must
// match the current stored revision (optimistic concurrency).
type RunLimitsConfigChange struct {
	Schema     int       `json:"schema_version"`
	EventID    string    `json:"event_id"`
	Scope      string    `json:"scope"`
	Mission    string    `json:"mission_id,omitempty"`
	Key        string    `json:"scope_key,omitempty"`
	Revision   int       `json:"expected_revision"`
	Values     RunLimits `json:"values"`
	Reason     string    `json:"reason"`
	RollbackOf int       `json:"rollback_of,omitempty"`
}

// RunLimitsHistoryEntry is one immutable, past-effective revision of a scope.
type RunLimitsHistoryEntry struct {
	Scope      string    `json:"scope"`
	MissionID  string    `json:"mission_id,omitempty"`
	ScopeKey   string    `json:"scope_key,omitempty"`
	Revision   int       `json:"revision"`
	Values     RunLimits `json:"values"`
	Actor      string    `json:"actor"`
	Reason     string    `json:"reason"`
	At         string    `json:"at"`
	RollbackOf int       `json:"rollback_of,omitempty"`
}

func validRunLimitsScope(scope, mission, key string) error {
	switch scope {
	case ScopeProject:
		if mission != "" || key != "" {
			return fmt.Errorf("portée projet : identifiant de mission et clé de portée doivent être vides")
		}
	case ScopeMission:
		if !safeName(mission) || key != "" {
			return fmt.Errorf("portée mission : identifiant de mission requis, clé de portée vide")
		}
	case ScopeRole, ScopeTask:
		if !safeName(mission) || !safeName(key) {
			return fmt.Errorf("portée %s : identifiant de mission et clé de portée requis", scope)
		}
	default:
		return fmt.Errorf("portée inconnue : %s", scope)
	}
	return nil
}

// validRunLimitsOverride accepts an all-zero value (clears/no-op override,
// pure inheritance) or a value where every non-zero field is within the
// same bounds RunLimits.normalized() enforces for a frozen attempt. It
// never silently substitutes a default: an out-of-bounds field is refused.
func validRunLimitsOverride(l RunLimits) error {
	fields := []struct {
		name   string
		v, max int
	}{
		{"observation_mode", l.ObservationMode, 1},
		{"silence_seconds", l.SilenceSeconds, 86400},
		{"tool_seconds", l.ToolSeconds, 86400},
		{"max_tool_calls", l.MaxToolCalls, 10000},
		{"max_repeated_calls", l.MaxRepeatedCalls, 100},
		{"max_consecutive_errors", l.MaxConsecutiveErrors, 100},
	}
	for _, f := range fields {
		if f.v != 0 && (f.v < 1 || f.v > f.max) {
			return fmt.Errorf("limite d'agent hors bornes : %s doit valoir 0 (héritée) ou 1..%d, reçu %d", f.name, f.max, f.v)
		}
	}
	return nil
}

// overlayRunLimits applies override's non-zero fields onto base.
func overlayRunLimits(base, override RunLimits) RunLimits {
	if override.ObservationMode != 0 {
		base.ObservationMode = override.ObservationMode
	}
	if override.SilenceSeconds != 0 {
		base.SilenceSeconds = override.SilenceSeconds
	}
	if override.ToolSeconds != 0 {
		base.ToolSeconds = override.ToolSeconds
	}
	if override.MaxToolCalls != 0 {
		base.MaxToolCalls = override.MaxToolCalls
	}
	if override.MaxRepeatedCalls != 0 {
		base.MaxRepeatedCalls = override.MaxRepeatedCalls
	}
	if override.MaxConsecutiveErrors != 0 {
		base.MaxConsecutiveErrors = override.MaxConsecutiveErrors
	}
	return base
}

// rowQuerier is satisfied by both *sql.DB and *sql.Tx: with the store's
// connection pool capped at one connection (see openStore), a read that
// must happen while a transaction is still open has to run on that same
// *sql.Tx, never on s.db, or it deadlocks waiting for the connection the
// open transaction is holding.
type rowQuerier interface {
	QueryRow(query string, args ...any) *sql.Row
}

func currentRunLimitsConfigWith(q rowQuerier, scope, mission, key string) (RunLimitsConfigEntry, error) {
	e := RunLimitsConfigEntry{Scope: scope, MissionID: mission, ScopeKey: key}
	var raw []byte
	err := q.QueryRow("SELECT revision,body,actor,reason,updated FROM run_limits_config WHERE scope=? AND mission_id=? AND scope_key=?", scope, mission, key).
		Scan(&e.Revision, &raw, &e.Actor, &e.Reason, &e.Updated)
	if err == sql.ErrNoRows {
		return e, nil
	}
	if err != nil {
		return e, err
	}
	if err = json.Unmarshal(raw, &e.Values); err != nil {
		return e, err
	}
	return e, nil
}

func (s *Store) currentRunLimitsConfig(scope, mission, key string) (RunLimitsConfigEntry, error) {
	return currentRunLimitsConfigWith(s.db, scope, mission, key)
}

// configureRunLimits validates and stores a new override for one scope. It
// never touches an already-frozen Agent.Limits value: this table is read
// only when a future attempt resolves its effective limits, never when an
// in-flight attempt was launched. See effectiveRunLimits and RETEX in
// docs/T2-contrat-moteur.md.
func (s *Store) configureRunLimits(r RunLimitsConfigChange) (RunLimitsConfigEntry, error) {
	if r.Values.ObservationMode != 0 && (r.Scope != ScopeMission || !nonempty(r.Reason)) {
		return RunLimitsConfigEntry{}, fmt.Errorf("mode observation : portée mission et motif explicite requis")
	}
	if r.Schema != 1 {
		return RunLimitsConfigEntry{}, fmt.Errorf("version de demande invalide")
	}
	if !safeName(r.EventID) {
		return RunLimitsConfigEntry{}, fmt.Errorf("event_id obligatoire (lettres, chiffres, tirets)")
	}
	if err := validRunLimitsScope(r.Scope, r.Mission, r.Key); err != nil {
		return RunLimitsConfigEntry{}, err
	}
	if err := validRunLimitsOverride(r.Values); err != nil {
		return RunLimitsConfigEntry{}, err
	}
	if len(r.Reason) > 2000 {
		return RunLimitsConfigEntry{}, fmt.Errorf("motif trop long (2000 caractères maximum)")
	}
	reqRaw, err := json.Marshal(r)
	if err != nil {
		return RunLimitsConfigEntry{}, err
	}
	tx, err := s.db.Begin()
	if err != nil {
		return RunLimitsConfigEntry{}, err
	}
	defer tx.Rollback()
	var oldRequest []byte
	err = tx.QueryRow("SELECT request FROM run_limits_history WHERE scope=? AND mission_id=? AND scope_key=? AND event_id=?", r.Scope, r.Mission, r.Key, r.EventID).Scan(&oldRequest)
	if err == nil {
		if string(oldRequest) != string(reqRaw) {
			return RunLimitsConfigEntry{}, &CommandError{Code: "event_conflict", Message: "event_id déjà utilisé pour un contenu différent"}
		}
		// Idempotent replay: same event, same content, no new revision.
		// Must read via tx, not s.db: the pool has a single connection and
		// tx still holds it (see rowQuerier).
		return currentRunLimitsConfigWith(tx, r.Scope, r.Mission, r.Key)
	}
	if err != sql.ErrNoRows {
		return RunLimitsConfigEntry{}, err
	}
	var current int
	err = tx.QueryRow("SELECT revision FROM run_limits_config WHERE scope=? AND mission_id=? AND scope_key=?", r.Scope, r.Mission, r.Key).Scan(&current)
	if err != nil && err != sql.ErrNoRows {
		return RunLimitsConfigEntry{}, err
	}
	if current != r.Revision {
		return RunLimitsConfigEntry{}, &CommandError{Code: "revision_conflict", Message: "La configuration a changé ; relire puis confirmer.", Retryable: true}
	}
	entry := RunLimitsConfigEntry{Scope: r.Scope, MissionID: r.Mission, ScopeKey: r.Key, Values: r.Values, Revision: current + 1, Actor: operatorIdentity(), Reason: r.Reason, Updated: now()}
	raw, err := json.Marshal(entry.Values)
	if err != nil {
		return RunLimitsConfigEntry{}, err
	}
	if _, err = tx.Exec("INSERT INTO run_limits_config(scope,mission_id,scope_key,revision,body,actor,reason,updated) VALUES(?,?,?,?,?,?,?,?) ON CONFLICT(scope,mission_id,scope_key) DO UPDATE SET revision=excluded.revision,body=excluded.body,actor=excluded.actor,reason=excluded.reason,updated=excluded.updated",
		entry.Scope, entry.MissionID, entry.ScopeKey, entry.Revision, raw, entry.Actor, entry.Reason, entry.Updated); err != nil {
		return RunLimitsConfigEntry{}, err
	}
	var rollbackOf any
	if r.RollbackOf != 0 {
		rollbackOf = r.RollbackOf
	}
	if _, err = tx.Exec("INSERT INTO run_limits_history(scope,mission_id,scope_key,revision,body,request,actor,reason,at,event_id,rollback_of) VALUES(?,?,?,?,?,?,?,?,?,?,?)",
		entry.Scope, entry.MissionID, entry.ScopeKey, entry.Revision, raw, reqRaw, entry.Actor, entry.Reason, entry.Updated, r.EventID, rollbackOf); err != nil {
		return RunLimitsConfigEntry{}, err
	}
	if err = tx.Commit(); err != nil {
		return RunLimitsConfigEntry{}, err
	}
	return entry, nil
}

func (s *Store) runLimitsHistory(scope, mission, key string) ([]RunLimitsHistoryEntry, error) {
	rows, err := s.db.Query("SELECT revision,body,actor,reason,at,coalesce(rollback_of,0) FROM run_limits_history WHERE scope=? AND mission_id=? AND scope_key=? ORDER BY revision DESC", scope, mission, key)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	var out []RunLimitsHistoryEntry
	for rows.Next() {
		var h RunLimitsHistoryEntry
		var raw []byte
		h.Scope, h.MissionID, h.ScopeKey = scope, mission, key
		if err = rows.Scan(&h.Revision, &raw, &h.Actor, &h.Reason, &h.At, &h.RollbackOf); err != nil {
			return nil, err
		}
		if err = json.Unmarshal(raw, &h.Values); err != nil {
			return nil, err
		}
		out = append(out, h)
	}
	return out, rows.Err()
}

// rollbackRunLimits re-applies a past revision's values as a brand new
// revision (never rewrites or deletes history): the audit trail keeps every
// intermediate decision, and the rolled-back-to revision is still valid.
func (s *Store) rollbackRunLimits(scope, mission, key string, toRevision int, eventID, reason string, expectedRevision int) (RunLimitsConfigEntry, error) {
	hist, err := s.runLimitsHistory(scope, mission, key)
	if err != nil {
		return RunLimitsConfigEntry{}, err
	}
	var target *RunLimitsHistoryEntry
	for i := range hist {
		if hist[i].Revision == toRevision {
			target = &hist[i]
			break
		}
	}
	if target == nil {
		return RunLimitsConfigEntry{}, fmt.Errorf("révision historique introuvable : %d", toRevision)
	}
	if reason == "" {
		reason = fmt.Sprintf("retour à la révision %d", toRevision)
	}
	return s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: eventID, Scope: scope, Mission: mission, Key: key, Revision: expectedRevision, Values: target.Values, Reason: reason, RollbackOf: toRevision})
}

// effectiveRunLimits resolves the value a NEW attempt would receive right
// now for (mission, role, task). It re-reads the store on every call: it
// never caches, and it never reaches into any live Agent, so it cannot
// alter an attempt already frozen by an earlier call (REQ-ADM-05).
func (s *Store) effectiveRunLimits(mission, role, task string) (RunLimits, error) {
	base, err := configuredRunLimitsWith(s.db, mission, role, task)
	if err != nil {
		return base, err
	}
	return base.normalized()
}

// Read all scopes on the launch transaction so configuration and reservation
// share one snapshot. Zero fields remain inherited from the provider.
func configuredRunLimitsWith(q rowQuerier, mission, role, task string) (RunLimits, error) {
	base := RunLimits{}
	scopes := []struct {
		scope, mission, key string
	}{
		{ScopeProject, "", ""},
	}
	if mission != "" {
		scopes = append(scopes, struct{ scope, mission, key string }{ScopeMission, mission, ""})
		if role != "" {
			scopes = append(scopes, struct{ scope, mission, key string }{ScopeRole, mission, role})
		}
		if task != "" {
			scopes = append(scopes, struct{ scope, mission, key string }{ScopeTask, mission, task})
		}
	}
	for _, sc := range scopes {
		entry, err := currentRunLimitsConfigWith(q, sc.scope, sc.mission, sc.key)
		if err != nil {
			return base, err
		}
		if entry.Revision > 0 {
			base = overlayRunLimits(base, entry.Values)
		}
	}
	return base, validRunLimitsOverride(base)
}

// Administrative choice only; no change to persisted task contracts or counters.
func (s *Store) executionObserved(work, task string) bool {
	l, err := configuredRunLimitsWith(s.db, work, "worker", task)
	return err == nil && l.observing()
}
