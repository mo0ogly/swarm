package main

import (
	"database/sql"
	"fmt"
	"time"
)

const missionSupervisionMigration = `BEGIN;
CREATE TABLE IF NOT EXISTS mission_supervision(
 work_id TEXT PRIMARY KEY REFERENCES works(id),
 conductor_id TEXT NOT NULL,
 source TEXT NOT NULL,
 started_at TEXT NOT NULL,
 heartbeat_at TEXT NOT NULL,
 reconciled_at TEXT NOT NULL,
 last_check_at TEXT NOT NULL,
 last_error TEXT NOT NULL,
 next_check_at TEXT NOT NULL,
 stopped_at TEXT NOT NULL
);
PRAGMA user_version=12;
COMMIT;`

// A conductor is considered present only while it renews this persisted lease.
// The generous threshold avoids announcing a failure during a normal slow cycle,
// while still turning a crashed service into an explicit absence.
// A cycle may include a provider preflight (up to ten seconds) and several
// atomic reservations. Keep the lease comfortably above that bounded work,
// while still allowing an unattended mission to recover after a crash.
const missionConductorStaleAfter = 30 * time.Second

// A timestamp slightly ahead of the reader can come from formatting or a very
// small clock adjustment. Beyond one polling interval, however, wall-clock
// ordering no longer proves that the lease is alive. Fail closed instead of
// keeping an apparently active conductor until the clock catches up.
const missionClockFutureTolerance = missionPollInterval

type MissionSupervisionAction struct {
	At       string `json:"at"`
	Relative string `json:"relative"`
	Actor    string `json:"actor"`
	Kind     string `json:"kind"`
	Summary  string `json:"summary"`
}

type MissionSupervision struct {
	State             string                    `json:"state"`
	Source            string                    `json:"source,omitempty"`
	ObservedAt        string                    `json:"observed_at,omitempty"`
	LastCheckAt       string                    `json:"last_check_at,omitempty"`
	LastCheckRelative string                    `json:"last_check_relative,omitempty"`
	ReconciledAt      string                    `json:"reconciled_at,omitempty"`
	LastError         string                    `json:"last_error,omitempty"`
	NextCheckAt       string                    `json:"next_check_at,omitempty"`
	NextCheckRelative string                    `json:"next_check_relative,omitempty"`
	Late              bool                      `json:"late"`
	ClockIssue        string                    `json:"clock_issue,omitempty"`
	LastAction        *MissionSupervisionAction `json:"last_action,omitempty"`
}

func migrateMissionSupervision(db *sql.DB) error {
	_, err := db.Exec(missionSupervisionMigration)
	return err
}

func supervisionNext(at time.Time) string {
	return at.Add(missionPollInterval).UTC().Format(time.RFC3339Nano)
}

func (s *Store) beginMissionSupervision(work, conductor, source string, at time.Time) error {
	owned, err := s.claimMissionSupervision(work, conductor, source, at)
	if err != nil {
		return err
	}
	if !owned {
		return fmt.Errorf("mission déjà conduite par un autre processus")
	}
	return nil
}

// claimMissionSupervision is the single ownership decision. A live lease is
// never overwritten; an expired/stopped lease is replaced with a compare-and-
// swap so concurrent conductors cannot both believe they acquired it.
func (s *Store) claimMissionSupervision(work, conductor, source string, at time.Time) (bool, error) {
	stamp := at.UTC().Format(time.RFC3339Nano)
	tx, err := s.db.Begin()
	if err != nil {
		return false, err
	}
	defer tx.Rollback()
	var current, heartbeat, stopped string
	err = tx.QueryRow("SELECT conductor_id,heartbeat_at,stopped_at FROM mission_supervision WHERE work_id=?", work).Scan(&current, &heartbeat, &stopped)
	if err == sql.ErrNoRows {
		_, err = tx.Exec(`INSERT INTO mission_supervision(work_id,conductor_id,source,started_at,heartbeat_at,reconciled_at,last_check_at,last_error,next_check_at,stopped_at)
			VALUES(?,?,?,?,?,'','','',?,'')`, work, conductor, source, stamp, stamp, supervisionNext(at))
		if err != nil {
			return false, err
		}
		return true, tx.Commit()
	}
	if err != nil {
		return false, err
	}
	seen, parseErr := time.Parse(time.RFC3339Nano, heartbeat)
	live := stopped == "" && parseErr == nil && seen.Sub(at) <= missionClockFutureTolerance && at.Sub(seen) <= missionConductorStaleAfter
	if current != conductor && live {
		return false, nil
	}
	result, err := tx.Exec(`UPDATE mission_supervision SET conductor_id=?,source=?,started_at=?,heartbeat_at=?,
		reconciled_at='',last_check_at='',last_error='',next_check_at=?,stopped_at=''
		WHERE work_id=? AND conductor_id=? AND heartbeat_at=? AND stopped_at=?`,
		conductor, source, stamp, stamp, supervisionNext(at), work, current, heartbeat, stopped)
	if err != nil {
		return false, err
	}
	changed, err := result.RowsAffected()
	if err != nil || changed != 1 {
		return false, err
	}
	return true, tx.Commit()
}

func (s *Store) checkMissionSupervision(work, conductor string, at time.Time, cycleErr error) error {
	stamp := at.UTC().Format(time.RFC3339Nano)
	message := ""
	if cycleErr != nil {
		message = cycleErr.Error()
	}
	result, err := s.db.Exec(`UPDATE mission_supervision SET heartbeat_at=?,reconciled_at=CASE WHEN reconciled_at='' THEN ? ELSE reconciled_at END,
		last_check_at=?,last_error=?,next_check_at=?,stopped_at='' WHERE work_id=? AND conductor_id=?`,
		stamp, stamp, stamp, message, supervisionNext(at), work, conductor)
	if err != nil {
		return err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return fmt.Errorf("bail de supervision remplacé pour %s", work)
	}
	return nil
}

// renewMissionSupervision fences every reconciliation pass before it mutates
// durable agent, task or budget state. The in-memory registered set is only a
// cache: another process may have replaced an expired/stopped lease since the
// preceding pass.
func (s *Store) renewMissionSupervision(work, conductor string, at time.Time) error {
	stamp := at.UTC().Format(time.RFC3339Nano)
	result, err := s.db.Exec(`UPDATE mission_supervision SET heartbeat_at=?,next_check_at=?
		WHERE work_id=? AND conductor_id=? AND stopped_at=''`, stamp, supervisionNext(at), work, conductor)
	if err != nil {
		return err
	}
	if n, _ := result.RowsAffected(); n != 1 {
		return fmt.Errorf("bail de supervision remplacé pour %s", work)
	}
	return nil
}

func (s *Store) stopMissionSupervision(work, conductor string, at time.Time) {
	stamp := at.UTC().Format(time.RFC3339Nano)
	_, _ = s.db.Exec("UPDATE mission_supervision SET heartbeat_at=?,stopped_at=?,next_check_at='' WHERE work_id=? AND conductor_id=?", stamp, stamp, work, conductor)
}

func (s *Store) missionSupervision(work string, at time.Time) (MissionSupervision, error) {
	var d MissionSupervision
	var heartbeat, stopped string
	err := s.db.QueryRow(`SELECT source,heartbeat_at,reconciled_at,last_check_at,last_error,next_check_at,stopped_at
		FROM mission_supervision WHERE work_id=?`, work).Scan(&d.Source, &heartbeat, &d.ReconciledAt, &d.LastCheckAt, &d.LastError, &d.NextCheckAt, &stopped)
	if actionErr := s.lastMissionConductorAction(work, at, &d); actionErr != nil {
		return d, actionErr
	}
	if err == sql.ErrNoRows {
		d.State = "absent"
		return d, nil
	}
	if err != nil {
		return d, err
	}
	d.ObservedAt = heartbeat
	seen, parseErr := time.Parse(time.RFC3339Nano, heartbeat)
	checked, checkErr := time.Parse(time.RFC3339Nano, d.LastCheckAt)
	next, nextErr := time.Parse(time.RFC3339Nano, d.NextCheckAt)
	if checkErr == nil {
		d.LastCheckRelative = supervisionPastRelative(at.Sub(checked))
	}
	switch {
	case stopped != "":
		d.State = "absent"
		d.NextCheckAt = ""
	case parseErr != nil:
		d.State = "absent"
		d.NextCheckAt = ""
		d.ClockIssue = "Horodatage du conducteur illisible ; présence non confirmée."
	case seen.Sub(at) > missionClockFutureTolerance:
		d.State = "absent"
		d.NextCheckAt = ""
		d.ClockIssue = "Horloge modifiée : la présence du conducteur doit être confirmée par une nouvelle vérification."
	case at.Sub(seen) > missionConductorStaleAfter:
		d.State = "absent"
		d.NextCheckAt = ""
	case d.LastError != "":
		d.State = "error"
	default:
		d.State = "active"
	}
	if d.NextCheckAt != "" && nextErr == nil && d.State != "absent" {
		d.Late = at.After(next)
		d.NextCheckRelative = supervisionNextRelative(next.Sub(at))
	} else if d.NextCheckAt != "" && nextErr != nil {
		d.NextCheckAt = ""
		d.ClockIssue = "Prochaine vérification illisible ; aucune échéance n’est annoncée."
	}
	return d, nil
}

// Only durable engine mutations count as conductor actions. In particular,
// an authorization, a heartbeat, a waiting explanation or an external Codex
// supervision message is not promoted to an action of the Swarm conductor.
func (s *Store) lastMissionConductorAction(work string, at time.Time, d *MissionSupervision) error {
	var action MissionSupervisionAction
	err := s.db.QueryRow(`SELECT at,kind,message FROM cockpit_events
		WHERE work_id=? AND (kind='conductor' OR kind IN ('validation-accepted','validation-retained')
		 OR (kind='dispatch' AND message LIKE '% : départ automatique · %'))
		ORDER BY seq DESC LIMIT 1`, work).Scan(&action.At, &action.Kind, &action.Summary)
	if err == sql.ErrNoRows {
		return nil
	}
	if err != nil {
		return err
	}
	action.Actor = "Conducteur Swarm"
	stamp, err := time.Parse(time.RFC3339Nano, action.At)
	if err == nil {
		action.Relative = supervisionPastRelative(at.Sub(stamp))
	} else {
		action.Relative = "date illisible"
	}
	d.LastAction = &action
	return nil
}

func supervisionPastRelative(age time.Duration) string {
	if age < -missionClockFutureTolerance {
		return "horloge incohérente"
	}
	if age < time.Minute {
		return "à l’instant"
	}
	if age < time.Hour {
		return fmt.Sprintf("il y a environ %d min", int(age.Round(time.Minute)/time.Minute))
	}
	return fmt.Sprintf("il y a environ %d h", int(age.Round(time.Hour)/time.Hour))
}

func supervisionNextRelative(wait time.Duration) string {
	if wait > 0 {
		if wait <= missionPollInterval {
			return "dans moins de 2 s"
		}
		return fmt.Sprintf("dans environ %d s", int(wait.Round(time.Second)/time.Second))
	}
	late := -wait
	if late < time.Second {
		return "attendue maintenant"
	}
	return fmt.Sprintf("en retard d’environ %d s", int(late.Round(time.Second)/time.Second))
}
