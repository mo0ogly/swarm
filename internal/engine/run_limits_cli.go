package engine

import (
	"fmt"
	"io"
	"strconv"
)

// runLimitsCLI is a 1:1 CLI mirror of the engine's run-limits administration
// API (REQ-ADM-02): it only parses positional arguments and a JSON payload,
// then delegates every validation and mutation to the exact Store methods T2
// implements and tests in run_limits_admin.go / run_limits_admin_test.go —
// configureRunLimits, currentRunLimitsConfig, runLimitsHistory,
// rollbackRunLimits, effectiveRunLimits. No scope, bound or concurrency rule
// is re-implemented here: an out-of-bounds value, an unknown scope, a stale
// expected_revision or a reused event_id with different content are refused
// by the same code path TestRunLimitsConfigRejectsInvalidValue and
// TestRunLimitsConfigConcurrencyAndReplay already cover.
func (s *Store) runLimitsCLI(pos []string, input string, out io.Writer) error {
	usage := "swarm run-limits performance show|history|preview|apply [--input configuration.json] | swarm run-limits show|history <portée> <mission> <clé> | apply <portée> <mission> <clé> --input changement.json | rollback <portée> <mission> <clé> <révision_cible> --input requête.json | effective <mission> <rôle> <tâche>"
	if len(pos) < 2 {
		return fmt.Errorf("%s", usage)
	}
	action := pos[1]
	if action == "performance" {
		return s.graphPerformanceCLI(pos, input, out)
	}
	if action == "effective" {
		if len(pos) != 5 {
			return fmt.Errorf("swarm run-limits effective <mission> <rôle> <tâche>")
		}
		l, err := s.effectiveRunLimits(runLimitsArg(pos[2]), runLimitsArg(pos[3]), runLimitsArg(pos[4]))
		if err != nil {
			return err
		}
		return printJSON(out, l)
	}
	if len(pos) < 5 {
		return fmt.Errorf("%s", usage)
	}
	scope, mission, key := pos[2], runLimitsArg(pos[3]), runLimitsArg(pos[4])
	switch action {
	case "show":
		if len(pos) != 5 {
			return fmt.Errorf("swarm run-limits show <portée> <mission> <clé>")
		}
		entry, err := s.currentRunLimitsConfig(scope, mission, key)
		if err != nil {
			return err
		}
		return printJSON(out, entry)
	case "history":
		if len(pos) != 5 {
			return fmt.Errorf("swarm run-limits history <portée> <mission> <clé>")
		}
		hist, err := s.runLimitsHistory(scope, mission, key)
		if err != nil {
			return err
		}
		return printJSON(out, hist)
	case "apply":
		if len(pos) != 5 {
			return fmt.Errorf("swarm run-limits apply <portée> <mission> <clé> --input changement.json")
		}
		raw, err := readInput(input)
		if err != nil {
			return err
		}
		var r RunLimitsConfigChange
		if err = strict(raw, &r); err != nil {
			return err
		}
		if r.Scope != "" && r.Scope != scope {
			return fmt.Errorf("scope du JSON ne correspond pas à la portée en argument")
		}
		if r.Mission != "" && r.Mission != mission {
			return fmt.Errorf("mission_id du JSON ne correspond pas à l'argument")
		}
		if r.Key != "" && r.Key != key {
			return fmt.Errorf("scope_key du JSON ne correspond pas à l'argument")
		}
		r.Scope, r.Mission, r.Key = scope, mission, key
		entry, err := s.configureRunLimits(r)
		if err != nil {
			return err
		}
		return printJSON(out, entry)
	case "rollback":
		if len(pos) != 6 {
			return fmt.Errorf("swarm run-limits rollback <portée> <mission> <clé> <révision_cible> --input requête.json")
		}
		toRevision, err := strconv.Atoi(pos[5])
		if err != nil {
			return fmt.Errorf("révision cible invalide : %s", pos[5])
		}
		raw, err := readInput(input)
		if err != nil {
			return err
		}
		var r struct {
			EventID  string `json:"event_id"`
			Reason   string `json:"reason"`
			Revision int    `json:"expected_revision"`
		}
		if err = strict(raw, &r); err != nil {
			return err
		}
		entry, err := s.rollbackRunLimits(scope, mission, key, toRevision, r.EventID, r.Reason, r.Revision)
		if err != nil {
			return err
		}
		return printJSON(out, entry)
	default:
		return fmt.Errorf("action run-limits inconnue : %s", action)
	}
}

// runLimitsArg turns the CLI's empty-scope placeholder "-" into "", the
// value validRunLimitsScope (run_limits_admin.go) expects wherever a scope
// leaves mission or key unused (e.g. project scope). No scope rule is
// decided here; an invalid combination still gets refused by the engine.
func runLimitsArg(v string) string {
	if v == "-" {
		return ""
	}
	return v
}
