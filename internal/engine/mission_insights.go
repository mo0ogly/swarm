//go:build linux

package engine

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sort"
	"time"
)

type MissionWait struct {
	Task   string `json:"task"`
	Title  string `json:"title"`
	State  string `json:"state"`
	Reason string `json:"reason"`
}
type RecoveryPreview struct {
	Work         string             `json:"work"`
	Task         string             `json:"task"`
	Revision     int                `json:"revision"`
	Agent        string             `json:"agent,omitempty"`
	Kept         []string           `json:"kept"`
	Redone       []string           `json:"redone"`
	Correction   string             `json:"correction"`
	Criteria     []string           `json:"criteria"`
	Limits       string             `json:"limits"`
	SinceRefusal *MissionChanges    `json:"since_refusal,omitempty"`
	Evidence     []RecoveryEvidence `json:"evidence,omitempty"`
	Remaining    []string           `json:"remaining_criteria"`
	RefusalNote  string             `json:"refusal_note"`
	EvidenceNote string             `json:"evidence_note"`
}

// RecoveryEvidence (REQ-QW4): one entry per report/artifact bound to the task's
// latest bound review: unchanged inputs, changed inputs, or unknown.
// This input comparison is not a favorable verdict or permission to reuse acceptance.
type RecoveryEvidence struct {
	Report string `json:"report"`
	State  string `json:"state"`
	Reason string `json:"reason"`
}
type MissionChange struct {
	Revision int    `json:"revision"`
	At       string `json:"at"`
	Category string `json:"category"`
	Label    string `json:"label"`
	Task     string `json:"task,omitempty"`
	Title    string `json:"title,omitempty"`
}
type MissionChanges struct {
	From  int             `json:"from_revision"`
	To    int             `json:"to_revision"`
	First bool            `json:"first_visit"`
	More  bool            `json:"more"`
	Items []MissionChange `json:"items"`
}
type SpendingRow struct {
	Kind         string    `json:"kind"`
	ID           string    `json:"id"`
	Label        string    `json:"label"`
	Calls        int       `json:"recorded_calls"`
	Tools        int       `json:"observed_tool_calls"`
	UnknownTools int       `json:"attempts_with_incomplete_tool_measurement"`
	Input        int64     `json:"input_tokens"`
	Output       int64     `json:"output_tokens"`
	MissingUsage int       `json:"calls_without_usage"`
	Cost         CostTotal `json:"cost"`
	// Reviews and TaskRetries (REQ-QW7) are scoped to this one task, unlike
	// MissionSpending.Retries which sums every task so a single task's
	// rework cannot be read off the engine-wide total.
	Reviews          int    `json:"reviews_recorded"`
	TaskRetries      int    `json:"task_retries"`
	DurationMS       *int64 `json:"recorded_process_ms"`
	MissingDurations int    `json:"attempts_without_duration"`
}
type MissionSpending struct {
	ObservedAt string          `json:"observed_at"`
	Source     string          `json:"measurement_source"`
	Rows       []SpendingRow   `json:"rows"`
	Controls   int             `json:"recorded_control_executions"`
	Retries    int             `json:"worker_retries"`
	Note       string          `json:"note"`
	Attempts   []AttemptLedger `json:"attempts"`
}

// AttemptLedger is the per-attempt bilan (REQ-QW3): one row per agent+attempt
// of a task, never aggregated with other attempts of the same task. Unlike
// SpendingRow, it keeps the process outcome (ProcessState, from the agent's
// own lifecycle) separate from task acceptance (Accepted, decided by the
// engine independently of any single attempt's process ending).
type AttemptLedger struct {
	Task           string    `json:"task"`
	Label          string    `json:"label"`
	Agent          string    `json:"agent"`
	Attempt        string    `json:"attempt"`
	Role           string    `json:"role"`
	RequestedModel string    `json:"requested_model,omitempty"`
	ObservedModel  string    `json:"observed_model,omitempty"`
	ProcessState   string    `json:"process_state"`
	Accepted       bool      `json:"task_acceptance_recorded"`
	Measurement    string    `json:"measurement"`
	Calls          int       `json:"recorded_calls"`
	Tools          int       `json:"observed_tool_calls"`
	Measured       bool      `json:"measured"`
	Reads          int       `json:"tool_reads"`
	Writes         int       `json:"tool_writes"`
	Unclassified   int       `json:"tool_unclassified"`
	Tests          string    `json:"tests"`
	Errors         int       `json:"tool_errors_total"`
	Repeats        int       `json:"tool_repeats_total"`
	Degraded       string    `json:"degraded,omitempty"`
	Input          int64     `json:"input_tokens"`
	Output         int64     `json:"output_tokens"`
	MissingUsage   bool      `json:"usage_missing"`
	Cost           CostTotal `json:"cost"`
	// Started/Ended (REQ-QW7) come straight from the agent's own lifecycle
	// timestamps, never computed: a still-running or never-started attempt
	// reports an empty value, not an invented duration.
	Started    string `json:"started"`
	Ended      string `json:"ended,omitempty"`
	DurationMS *int64 `json:"recorded_process_ms"`
}

const attemptTestsUnknown = "inconnu : aucun signal fiable ne distingue un test dans les commandes observées"

func attemptLedgers(w Work, agents []Agent) []AttemptLedger {
	out := []AttemptLedger{}
	for _, a := range agents {
		label := a.TaskID
		accepted := false
		if t, _ := w.task(a.TaskID); t != nil {
			label = t.Title
			accepted = t.Status == "accepted"
		}
		row := AttemptLedger{Task: a.TaskID, Label: label, Agent: a.ID, Attempt: a.Attempt, Role: a.Role,
			ProcessState: a.Status, Accepted: accepted, Calls: 1, Tools: a.Progress.ToolCalls,
			Tests: attemptTestsUnknown, Degraded: a.Progress.Degraded, Measurement: "unknown",
			Started: a.Started, Ended: a.Ended, DurationMS: recordedProcessDuration(a.Started, a.Ended)}
		if a.ModelRoute != nil {
			row.RequestedModel = a.ModelRoute.Model
		}
		if a.ReportedModel != nil {
			row.ObservedModel = a.ReportedModel.Model
		}
		if a.Progress.MetricsVersion == attemptMetricsVersion && a.Mode != "terminal" {
			row.Measured = true
			row.Measurement = "observed"
			if a.Progress.Degraded != "" || a.Progress.PendingTools > 0 {
				row.Measurement = "partial"
			}
			row.Reads = a.Progress.Reads
			row.Writes = a.Progress.Writes
			row.Unclassified = a.Progress.Unclassified
			row.Errors = a.Progress.Errors
			row.Repeats = a.Progress.Repeats
		}
		if a.Usage == nil {
			row.MissingUsage = true
			row.Cost.Silent++
		} else {
			row.Input = a.Usage.Input
			row.Output = a.Usage.Output
			if a.Usage.ReportedCost == nil {
				row.Cost.Silent++
			} else {
				row.Cost.WithCost++
				row.Cost.Reported += *a.Usage.ReportedCost
			}
		}
		out = append(out, row)
	}
	sort.Slice(out, func(i, j int) bool {
		if out[i].Task != out[j].Task {
			return out[i].Task < out[j].Task
		}
		if out[i].Agent != out[j].Agent {
			return out[i].Agent < out[j].Agent
		}
		return out[i].Attempt < out[j].Attempt
	})
	return out
}

func (s *Store) taskWaits(w *Work, t *Task) []MissionWait {
	out := []MissionWait{}
	for _, id := range t.Depends {
		parent, _ := w.task(id)
		if s.acceptedFresh(w, parent, map[string]bool{}) {
			continue
		}
		item := MissionWait{Task: id, Title: id, State: "missing", Reason: "Prérequis absent du plan."}
		if parent != nil {
			item.Title = parent.Title
			item.State = parent.Status
			item.Reason = "Ce résultat n’est pas encore validé."
			if parent.Status == "accepted" {
				item.State = "stale"
				item.Reason = "Validation ancienne : les preuves actuelles doivent être vérifiées à nouveau."
			}
		}
		out = append(out, item)
	}
	return out
}

func recoveryPreviewFor(w Work, t *Task, a *Agent) RecoveryPreview {
	p := RecoveryPreview{Work: w.ID, Task: t.ID, Revision: w.Revision,
		Kept:       []string{"Historique des tentatives, rapports et avis précédents.", "Critères et dépendances du plan."},
		Redone:     []string{"Vérification des conditions de lancement et du candidat courant.", "Contrôles et revue requis pour la nouvelle tentative."},
		Correction: t.Next, Criteria: append([]string{}, t.Criteria...),
		Limits: "Aucun budget ni plafond n’est augmenté. Ce résumé n’autorise aucun départ et ne valide aucun résultat."}
	if a != nil {
		p.Agent = a.ID
		p.Kept = append(p.Kept, "Fournisseur et espace de la tentative sélectionnée, sous réserve des règles du lancement.")
		p.Redone = append(p.Redone, "Une nouvelle tentative d’agent ; l’ancienne reste dans l’historique.")
	}
	if t.CorrectiveRecovery != nil && t.CorrectiveRecovery.Instruction != "" {
		p.Correction = t.CorrectiveRecovery.Instruction
	}
	if w.Planning != nil && w.Planning.Repository != nil {
		p.Kept = append(p.Kept, "Résultat précédent réutilisable seulement après contrôle du dossier de reprise ; ancienne copie conservée.")
	}
	return p
}

func (s *Store) recoveryPreview(work, task, agent string) (RecoveryPreview, error) {
	w, e := s.get(work)
	if e != nil {
		return RecoveryPreview{}, e
	}
	t, e := w.task(task)
	if e != nil {
		return RecoveryPreview{}, e
	}
	agents, e := s.agents(work)
	if e != nil {
		return RecoveryPreview{}, e
	}
	var p RecoveryPreview
	found := false
	for i := range agents {
		a := &agents[i]
		if a.TaskID == task && (agent == "" || a.ID == agent) {
			p, found = recoveryPreviewFor(w, t, a), true
			break
		}
	}
	if !found {
		if agent != "" {
			return RecoveryPreview{}, fmt.Errorf("tentative hors tâche ou introuvable")
		}
		p = recoveryPreviewFor(w, t, nil)
	}
	if v, at, ok, err := s.lastRefusalRevision(w, task); err != nil {
		p.RefusalNote = "Historique du refus indisponible ; aucun changement n’est supposé."
	} else if ok {
		changes, err := s.missionChanges(w, Visit{Revision: v, At: at})
		if err != nil {
			return RecoveryPreview{}, err
		}
		// Other tasks' activity is not a change to this refused result.
		filtered := []MissionChange{}
		for _, item := range changes.Items {
			if item.Task == task {
				filtered = append(filtered, item)
			}
		}
		changes.Items = filtered
		p.SinceRefusal = &changes
		p.RefusalNote = "Changements enregistrés depuis le dernier refus de cette tâche."
	} else {
		p.RefusalNote = "Aucun refus identifié dans les 500 derniers événements ; historique plus ancien non inspecté."
	}
	p.Evidence = s.recoveryEvidence(t)
	p.EvidenceNote = "Seules les preuves liées au dernier avis sont comparées, au plus 64 fichiers et 32 Mio ; aucun inventaire du dépôt. Une preuve inchangée ne vaut ni avis favorable ni validation."
	p.Remaining = append([]string{}, t.Criteria...)
	r := recoveryReview(t)
	unchanged := len(p.Evidence) > 0
	for _, e := range p.Evidence {
		if e.State != "unchanged" {
			unchanged = false
		}
	}
	if r != nil && unchanged && r.Contract == reviewContract(t) {
		p.Remaining = []string{}
		for i, criterion := range t.Criteria {
			covered := false
			for _, verdict := range r.Criteria {
				if verdict.Index == i+1 && verdict.Verdict == "pass" {
					covered = true
				}
			}
			if !covered {
				p.Remaining = append(p.Remaining, criterion)
			}
		}
	}

	return p, nil
}

// lastRefusalRevision (REQ-QW4): the mission revision at which this task was
// last refused or blocked, read from the durable event log — never from the
// operator's own last visit, which tracks a different, human-anchored boundary.
// Bounded to the same 500 most recent mission events scanned by missionChanges;
// an older refusal outside that window is reported absent, not assumed at r0.
func (s *Store) lastRefusalRevision(w Work, task string) (int, string, bool, error) {
	rows, e := s.db.Query("SELECT revision,kind,at,payload FROM events WHERE work_id=? AND revision<=? ORDER BY revision DESC LIMIT 500", w.ID, w.Revision)
	if e != nil {
		return 0, "", false, e
	}
	defer rows.Close()
	for rows.Next() {
		var rev int
		var kind, at string
		var raw []byte
		if e = rows.Scan(&rev, &kind, &at, &raw); e != nil {
			return 0, "", false, e
		}
		category, _ := missionChangeCategory(kind, raw)
		if category != "block" {
			continue
		}
		var p activityPayload
		_ = json.Unmarshal(raw, &p)
		if kind == "review.result" {
			var review IndependentReview
			_ = json.Unmarshal(raw, &review)
			for _, t2 := range w.Tasks {
				if t2.ID != task {
					continue
				}
				if (t2.IndependentReview != nil && t2.IndependentReview.ID == review.ID) || taskReviewHistoryHas(&t2, review.ID) {
					return rev, at, true, nil
				}
			}
			continue
		}
		if p.task() == task {
			return rev, at, true, nil
		}
	}
	return 0, "", false, rows.Err()
}

func taskReviewHistoryHas(t *Task, id string) bool {
	for _, r := range t.PreviousReviews {
		if r.ID == id {
			return true
		}
	}
	return false
}

// recoveryEvidence (REQ-QW4): classifies the report/artifacts bound to the
// task's current or last refusal verdict. Mirrors the hash comparison already
// used by boundReviewEvidenceChanged, but exposes each input individually
// instead of collapsing them into a single "changed" bool, so the operator
// sees which specific proof is still reusable and which is stale.
func recoveryReview(t *Task) *IndependentReview {
	if t.IndependentReview != nil {
		return t.IndependentReview
	}
	if len(t.PreviousReviews) > 0 {
		return &t.PreviousReviews[len(t.PreviousReviews)-1]
	}
	return nil
}

// taskReviewCount (REQ-QW7) counts only reviews actually recorded for this
// task: its current independent review plus its retained history. A task
// never reviewed reports zero, a real count, not an absence to hide.
func taskReviewCount(t *Task) int {
	n := len(t.PreviousReviews)
	if t.IndependentReview != nil {
		n++
	}
	return n
}

func (s *Store) recoveryEvidence(t *Task) []RecoveryEvidence {
	r := recoveryReview(t)
	if r == nil {
		return nil
	}
	inputs := map[string]string{}
	for name, digest := range r.ReportArtifacts {
		inputs[name] = digest
	}
	if r.Report != "" {
		inputs[r.Report] = r.Digest
	}
	names := make([]string, 0, len(inputs))
	for name := range inputs {
		names = append(names, name)
	}
	sort.Strings(names)
	out := make([]RecoveryEvidence, 0, min(64, len(names)))
	remaining := int64(32 << 20)
	for i, name := range names {
		if i >= 64 {
			out = append(out, RecoveryEvidence{State: "unknown", Reason: "Autres preuves non inspectées : limite de 64 fichiers atteinte."})
			break
		}
		item := RecoveryEvidence{Report: name, State: "unknown", Reason: "Empreinte ou contenu indisponible : réutilisation non démontrée."}
		path, err := safeReport(s.root, name)
		if err == nil && inputs[name] != "" && remaining > 0 {
			f, e := os.Open(path)
			if e == nil {
				info, e := f.Stat()
				if e == nil && info.Mode().IsRegular() && info.Size() <= 8<<20 && info.Size() <= remaining {
					h := sha256.New()
					n, e := io.Copy(h, io.LimitReader(f, min(int64(8<<20), remaining)+1))
					remaining -= n
					if e == nil && n <= 8<<20 && remaining >= 0 {
						if hex.EncodeToString(h.Sum(nil)) == inputs[name] {
							item.State, item.Reason = "unchanged", "Contenu inchangé ; réutilisable comme entrée seulement, pas comme validation."
						} else {
							item.State, item.Reason = "changed", "Contenu modifié depuis l’avis : ancienne preuve périmée, vérification requise."
						}
					}
				}
				f.Close()
			}
		}
		out = append(out, item)
	}
	return out
}

// Typed events only: never classify prose or treat an historical event as
// current acceptance. Unknown kinds stay in the full activity feed.
func missionChangeCategory(kind string, raw []byte) (string, string) {
	var p struct {
		Status string `json:"status"`
		Next   string `json:"next"`
		State  string `json:"state"`
	}
	_ = json.Unmarshal(raw, &p)
	switch kind {
	case "task.update":
		switch p.Status {
		case "submitted":
			return "result", "Résultat soumis pour examen"
		case "accepted":
			return "result", "Acceptation enregistrée"
		case "blocked":
			return "block", "Blocage enregistré"
		}
		if p.Next != "" {
			return "decision", "Consigne de reprise modifiée"
		}
	case "review.result":
		if p.State == "passed" {
			return "result", "Avis indépendant enregistré"
		}
		return "block", "Vérification à examiner"
	case "review.failure", "planning.proof-stale":
		return "block", "Preuve ou vérification à examiner"
	case "task.auto-validation":
		if p.State == "blocked" {
			return "block", "Contrôle en échec"
		}
		return "result", "Contrôles enregistrés"
	case "task.auto-validation-reviewed":
		return "result", "Acceptation après contrôles et revue"
	case "decision", "decision.moteur", "planning.decide", "planning.authorize-recovery", "planning.extend-attempt":
		return "decision", "Décision enregistrée"
	}
	return "", ""
}

func (s *Store) missionChanges(w Work, v Visit) (MissionChanges, error) {
	out := MissionChanges{From: v.Revision, To: w.Revision, First: v.At == "", Items: []MissionChange{}}
	if out.First {
		return out, nil
	}
	rows, e := s.db.Query("SELECT revision,kind,at,payload FROM events WHERE work_id=? AND revision>? AND revision<=? ORDER BY revision DESC LIMIT 201", w.ID, v.Revision, w.Revision)
	if e != nil {
		return out, e
	}
	defer rows.Close()
	count := 0
	for rows.Next() {
		var rev int
		var kind, at string
		var raw []byte
		if e = rows.Scan(&rev, &kind, &at, &raw); e != nil {
			return out, e
		}
		count++
		if count > 200 {
			out.More = true
			break
		}
		category, label := missionChangeCategory(kind, raw)
		if category == "" {
			continue
		}
		var p activityPayload
		_ = json.Unmarshal(raw, &p)
		id := p.task()
		if kind == "task.auto-validation" {
			id = ""
			var record AutomaticValidation
			_ = json.Unmarshal(raw, &record)
			for _, t := range w.Tasks {
				for _, a := range t.Attempts {
					if a.ID == record.Attempt {
						id = t.ID
					}
				}
			}
		}
		// Review IDs and receipt IDs do not name tasks. Resolve only identities
		// explicitly persisted on the current task or its review history.
		if kind == "review.result" {
			id = ""
			var review IndependentReview
			_ = json.Unmarshal(raw, &review)
			for _, t := range w.Tasks {
				if t.IndependentReview != nil && t.IndependentReview.ID == review.ID {
					id = t.ID
				}
				for _, r := range t.PreviousReviews {
					if r.ID == review.ID {
						id = t.ID
					}
				}
			}
		}
		title := ""
		if t, _ := w.task(id); t != nil {
			title = t.Title
		} else {
			id = ""
		}
		out.Items = append(out.Items, MissionChange{rev, at, category, label, id, title})
	}
	return out, rows.Err()
}

func addMeasuredUsage(row *SpendingRow, u *Usage) {
	if u == nil {
		row.MissingUsage++
		row.Cost.Silent++
		return
	}
	row.Input += u.Input
	row.Output += u.Output
	if u.ReportedCost == nil {
		row.Cost.Silent++
	} else {
		row.Cost.WithCost++
		row.Cost.Reported += *u.ReportedCost
	}
}
func (s *Store) missionSpending(w Work, agents []Agent) (MissionSpending, error) {
	out := MissionSpending{ObservedAt: now(), Source: "agent.lifecycle + agent.progress + provider.result + task.review-history + planning_calls + events", Rows: []SpendingRow{}, Note: "Les appels d’outils, les appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n’est pas un zéro ; les relances internes du fournisseur ne sont pas toutes observables. Une réservation d’appel ne prouve pas son envoi au fournisseur."}
	by := map[string]*SpendingRow{}
	rowFor := func(kind, id, label string) *SpendingRow {
		key := kind + ":" + id
		if by[key] == nil {
			by[key] = &SpendingRow{Kind: kind, ID: id, Label: label}
		}
		return by[key]
	}
	for _, a := range agents {
		label := a.TaskID
		if t, _ := w.task(a.TaskID); t != nil {
			label = t.Title
		}
		row := rowFor("worker", a.TaskID, label)
		row.Calls++
		if duration := recordedProcessDuration(a.Started, a.Ended); duration != nil {
			if row.DurationMS == nil {
				row.DurationMS = new(int64)
			}
			*row.DurationMS += *duration
		} else {
			row.MissingDurations++
		}
		row.Tools += a.Progress.ToolCalls
		if a.Mode == "terminal" || a.Progress.Degraded != "" || a.Progress.MetricsVersion != attemptMetricsVersion {
			row.UnknownTools++
		}
		if t, _ := w.task(a.TaskID); t != nil {
			row.Reviews = taskReviewCount(t)
		}
		if a.Previous != "" {
			out.Retries++
			row.TaskRetries++
		}
		addMeasuredUsage(row, a.Usage)
	}
	rows, e := s.db.Query("SELECT scope_id,usage FROM planning_calls WHERE work_id=? AND state!='released' ORDER BY created_at,id", w.ID)
	if e != nil {
		return out, e
	}
	for rows.Next() {
		var scope string
		var raw []byte
		if e = rows.Scan(&scope, &raw); e != nil {
			rows.Close()
			return out, e
		}
		kind, label := "planner", "Responsable : "+scope
		if scope == "reviewer" {
			kind, label = "reviewer", "Vérificateur indépendant"
		}
		row := rowFor(kind, scope, label)
		row.Calls++
		var u Usage
		if len(raw) > 0 && string(raw) != "null" {
			if e = json.Unmarshal(raw, &u); e != nil {
				rows.Close()
				return out, e
			}
			addMeasuredUsage(row, &u)
		} else {
			addMeasuredUsage(row, nil)
		}
	}
	e = rows.Err()
	rows.Close()
	if e != nil {
		return out, e
	}
	controls, e := s.db.Query("SELECT payload FROM events WHERE work_id=? AND kind='task.auto-validation'", w.ID)
	if e != nil {
		return out, e
	}
	for controls.Next() {
		var raw []byte
		if e = controls.Scan(&raw); e != nil {
			controls.Close()
			return out, e
		}
		var record AutomaticValidation
		if e = json.Unmarshal(raw, &record); e != nil {
			controls.Close()
			return out, e
		}
		for _, c := range record.Controls {
			if c.Executed {
				out.Controls++
			}
		}
	}
	e = controls.Err()
	controls.Close()
	if e != nil {
		return out, e
	}
	keys := []string{}
	for k := range by {
		keys = append(keys, k)
	}
	sort.Strings(keys)
	for _, k := range keys {
		out.Rows = append(out.Rows, *by[k])
	}
	out.Attempts = attemptLedgers(w, agents)
	return out, nil
}

func printMissionChanges(out io.Writer, c MissionChanges) {
	fmt.Fprintln(out, uiText("Depuis votre dernière visite"))
	if c.First {
		fmt.Fprintln(out, uiText("Première visite : aucun repère précédent. L’historique complet reste disponible."))
		return
	}
	if c.More {
		fmt.Fprintln(out, uiText("Extrait des 200 derniers événements ; consultez le fil complet pour les plus anciens."))
	}
	if len(c.Items) == 0 {
		fmt.Fprintln(out, uiText("Aucun nouveau résultat, blocage ou décision dans cet extrait."))
	}
	for _, x := range c.Items {
		fmt.Fprintf(out, "r%d · %s · %s · %s\n", x.Revision, x.At, uiText(x.Label), x.Title)
	}
	fmt.Fprintln(out, uiText("Ces événements sont historiques ; l’état actuel de la tâche fait foi."))
}
func printMissionSpending(out io.Writer, c MissionSpending) {
	fmt.Fprintln(out, uiText("Où vont les appels et les coûts ?"))
	sourceLabel := uiText("non rapporté")
	if c.Source != "" {
		sourceLabel = uiText("Horaires des processus, activité reçue, usages fournisseurs et historique des revues.")
	}
	fmt.Fprintf(out, uiText("Mesures observées : %s · Source : %s\n"), attemptStampOrUnknown(c.ObservedAt), sourceLabel)
	for _, r := range c.Rows {
		label := r.Label
		if r.Kind != "worker" {
			label = uiEngineText(label)
		}
		if r.Kind == "worker" && r.Tools == 0 && r.UnknownTools > 0 {
			fmt.Fprintf(out, uiText("%s · %d tentatives enregistrées · appels d’outils non rapportés · %s\n"), label, r.Calls, uiEngineText(r.Cost.Text()))
		} else if r.Calls > 0 && r.MissingUsage >= r.Calls {
			fmt.Fprintf(out, uiText("%s · %d tentatives ou appels enregistrés · %d appels d’outils observés · jetons non rapportés · %s\n"), label, r.Calls, r.Tools, uiEngineText(r.Cost.Text()))
		} else {
			fmt.Fprintf(out, uiText("%s · %d tentatives ou appels enregistrés · %d appels d’outils observés · %d mesures d’outils incomplètes · %d/%d jetons entrée/sortie rapportés · %d usages absents · %s\n"), label, r.Calls, r.Tools, r.UnknownTools, r.Input, r.Output, r.MissingUsage, uiEngineText(r.Cost.Text()))
		}
		if r.Kind == "worker" {
			fmt.Fprintf(out, uiText("  Durée cumulée des processus : %s · %d durée(s) non rapportée(s)\n"), durationLabel(r.DurationMS), r.MissingDurations)
			fmt.Fprintf(out, uiText("  %d revue(s) enregistrée(s) pour cette tâche · %d reprise(s) pour cette tâche\n"), r.Reviews, r.TaskRetries)
		}
	}
	fmt.Fprintf(out, uiText("Moteur : %d contrôles enregistrés · %d reprises d’agents\n"), c.Controls, c.Retries)
	fmt.Fprintln(out, uiText(c.Note))
	printAttemptLedgers(out, c.Attempts)
}

// A known zero remains zero; absent, invalid or reversed timestamps remain unknown.
func recordedProcessDuration(started, ended string) *int64 {
	start, err := time.Parse(time.RFC3339Nano, started)
	if err != nil {
		return nil
	}
	end, err := time.Parse(time.RFC3339Nano, ended)
	if err != nil || end.Before(start) {
		return nil
	}
	value := end.Sub(start).Milliseconds()
	return &value
}
func durationLabel(value *int64) string {
	if value == nil {
		return uiText("non rapporté")
	}
	return fmt.Sprintf("%.3f s", float64(*value)/1000)
}

// attemptStampOrUnknown / attemptEndStampLabel (REQ-QW7): report an absent
// timestamp as such instead of an empty or invented value; a running/queued
// attempt without an end timestamp is "ongoing", not "unreported".
func attemptStampOrUnknown(stamp string) string {
	if stamp == "" {
		return uiText("non rapporté")
	}
	return stamp
}
func attemptEndStampLabel(r AttemptLedger) string {
	if r.Ended != "" {
		return r.Ended
	}
	if r.ProcessState == "running" || r.ProcessState == "queued" {
		return uiText("en cours")
	}
	return uiText("non rapporté")
}
func attemptProcessLabel(state string) string {
	switch state {
	case "completed":
		return "Processus terminé"
	case "interrupted":
		return "Processus interrompu"
	case "failed":
		return "Processus en échec"
	case "running":
		return "Processus en cours"
	case "queued":
		return "Démarrage en attente"
	default:
		return "État du processus inconnu"
	}
}

func printAttemptLedgers(out io.Writer, rows []AttemptLedger) {
	fmt.Fprintln(out, uiText("Bilan par tentative"))
	if len(rows) == 0 {
		fmt.Fprintln(out, uiText("Aucune tentative enregistrée."))
		return
	}
	for _, r := range rows {
		accepted := uiText("non")
		if r.Accepted {
			accepted = uiText("oui")
		}
		fmt.Fprintf(out, uiText("%s · agent %s · tentative %s · état du processus : %s · validation enregistrée de la tâche (toutes tentatives) : %s\n"), r.Label, r.Agent, r.Attempt, uiText(attemptProcessLabel(r.ProcessState)), accepted)
		fmt.Fprintf(out, uiText("  Départ : %s · Fin : %s\n"), attemptStampOrUnknown(r.Started), attemptEndStampLabel(r))
		fmt.Fprintf(out, uiText("  Durée du processus : %s\n"), durationLabel(r.DurationMS))
		if r.Tools == 0 && !r.Measured {
			fmt.Fprintln(out, uiText("  Appels d’outils non rapportés."))
		} else {
			fmt.Fprintf(out, uiText("  %d départ enregistré · %d appels d’outils observés ; les appels internes du fournisseur ne sont pas mesurés.\n"), r.Calls, r.Tools)
		}
		if r.Measurement == "partial" {
			fmt.Fprintln(out, uiText("  Mesure partielle : ces compteurs couvrent uniquement les événements reçus."))
		}
		if r.Measured {
			fmt.Fprintf(out, uiText("  %d appels d’outils observés · lectures %d · écritures %d · non classés %d · tests %s · erreurs %d · répétitions %d\n"), r.Tools, r.Reads, r.Writes, r.Unclassified, uiText(r.Tests), r.Errors, r.Repeats)
		} else {
			fmt.Fprintln(out, uiText("  Mesure détaillée indisponible pour cette tentative : lectures, écritures, erreurs et répétitions restent inconnues, pas zéro."))
		}
		if r.Degraded != "" {
			fmt.Fprintf(out, uiText("  Mesure incomplète : %s\n"), uiEngineText(r.Degraded))
		}
		if r.MissingUsage {
			fmt.Fprintln(out, uiText("  Coût et usage non rapportés par le fournisseur pour cette tentative."))
		} else {
			fmt.Fprintf(out, uiText("  %d/%d jetons entrée/sortie rapportés · %s\n"), r.Input, r.Output, uiEngineText(r.Cost.Text()))
		}
	}
}
func printRecoveryPreview(out io.Writer, p RecoveryPreview) {
	fmt.Fprintln(out, uiText("Avant une relance"))
	for _, section := range []struct {
		label string
		items []string
	}{{"Ce qui sera conservé", p.Kept}, {"Ce qui sera refait", p.Redone}, {"Critères inchangés", p.Criteria}} {
		fmt.Fprintln(out, uiText(section.label))
		for _, item := range section.items {
			fmt.Fprintln(out, "- "+uiEngineText(item))
		}
	}
	fmt.Fprintln(out, uiText("Depuis le refus"))
	fmt.Fprintln(out, uiText(p.RefusalNote))
	if p.SinceRefusal != nil {
		for _, c := range p.SinceRefusal.Items {
			fmt.Fprintf(out, "- r%d · %s · %s\n", c.Revision, c.At, uiText(c.Label))
		}
		if p.SinceRefusal.More {
			fmt.Fprintln(out, uiText("Historique partiel : d’autres événements restent à examiner."))
		}
	}
	fmt.Fprintln(out, uiText("Preuves à reprendre"))
	fmt.Fprintln(out, uiText(p.EvidenceNote))
	if len(p.Evidence) == 0 {
		fmt.Fprintln(out, uiText("Aucune preuve liée à un avis disponible ; réutilisation non démontrée."))
	}
	for _, e := range p.Evidence {
		fmt.Fprintf(out, "- %s · %s\n", e.Report, uiText(e.Reason))
	}
	fmt.Fprintln(out, uiText("Critères restant à vérifier"))
	for _, c := range p.Remaining {
		fmt.Fprintln(out, "- "+c)
	}
	if len(p.Remaining) == 0 {
		fmt.Fprintln(out, uiText("Tous les critères ont un avis favorable sur ces entrées inchangées ; les contrôles et la décision restent requis."))
	}
	fmt.Fprintln(out, uiText("Correction attendue"))
	fmt.Fprintln(out, p.Correction)
	fmt.Fprintln(out, uiText(p.Limits))
}
