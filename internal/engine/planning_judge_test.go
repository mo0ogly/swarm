//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

// The adopted plan names this checkpoint control. Exercise its real managed
// cycle and public CLI instead of allowing an empty regexp match to look green.
func TestManagedCheckpointRecovery(t *testing.T) {
	s, w := managedFixture(t)
	before := w.Planning
	owner, _ := before.scope("root")
	request := PlanningRequest{Schema: 1, EventID: "checkpoint-judge", Revision: w.Revision, Scope: "root", ScopeRevision: owner.Revision, JudgeCycle: 1, JudgeVerdict: "continue", Reason: "Conserver les preuves avant le renouvellement"}
	input := filepath.Join(t.TempDir(), "request.json")
	write := func(r PlanningRequest) {
		t.Helper()
		b, _ := json.Marshal(r)
		if err := os.WriteFile(input, b, 0600); err != nil {
			t.Fatal(err)
		}
	}
	write(request)
	var output bytes.Buffer
	if err := s.planningCLI([]string{"planning", "judge", w.ID}, input, &output); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	current, _ := reopened.get(w.ID)
	owner, _ = current.Planning.scope("root")
	request = PlanningRequest{Schema: 1, EventID: "checkpoint-renew", Revision: current.Revision, Scope: "root", ScopeRevision: owner.Revision, JudgeCycle: 1}
	write(request)
	if err = reopened.planningCLI([]string{"planning", "renew", w.ID}, input, &output); err != nil {
		t.Fatal(err)
	}
	after, _ := reopened.get(w.ID)
	if after.Planning.Judge.Cycle != 2 || after.Planning.Judge.Verdict != "" || len(after.Planning.Judge.History) != 1 || after.Planning.Repository.Candidate != before.Repository.Candidate || after.Planning.Activations != before.Activations || after.Planning.Decisions != before.Decisions || len(after.Tasks) != len(w.Tasks) || managedReviewCalls(t, reopened) != 0 {
		t.Fatal("checkpoint recovery changed candidate, history, tasks or accounting")
	}
	if err = reopened.planningCLI([]string{"planning", "renew", w.ID}, input, &output); err != nil {
		t.Fatal(err)
	}
	replay, _ := reopened.get(w.ID)
	if replay.Revision != after.Revision || replay.Planning.Judge.Cycle != 2 || managedReviewCalls(t, reopened) != 0 {
		t.Fatal("checkpoint replay caused another effect or review")
	}
}

func judgeDo(t *testing.T, s *Store, w Work, action string, cycle int, extra PlanningRequest) (Work, error) {
	t.Helper()
	owner, e := w.Planning.scope("root")
	if e != nil {
		t.Fatal(e)
	}
	extra.Schema = 1
	extra.EventID = newID("test-judge-")
	extra.Revision = w.Revision
	extra.Scope = "root"
	extra.ScopeRevision = owner.Revision
	extra.JudgeCycle = cycle
	return s.planningChange(w.ID, action, extra)
}

func TestPlanningJudgeEndRejectsMatchingCandidateWithoutProofs(t *testing.T) {
	s, w := planningFixture(t)
	w.Planning.Repository = &ManagedRepository{Source: "fixture", Candidate: "candidate-without-proofs"}
	raw, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	if _, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "end", Reason: "Le candidat seul ne prouve pas les exigences", ExpectedCandidate: "candidate-without-proofs"}); err == nil {
		t.Fatal("matching candidate allowed global closure without requirement evidence")
	}
	got, _ := s.get(w.ID)
	if got.Revision != w.Revision || got.Planning.Judge.Closed || len(got.Planning.Judge.History) != 0 {
		t.Fatal("rejected closure changed revision, verdict or history")
	}
}

// req-5: the global judge renders a verdict distinct from -- and never
// mutating -- the independent per-candidate review fields.
func TestPlanningJudgeVerdictDistinctFromCandidateReview(t *testing.T) {
	s, w := planningFixture(t)
	if w.Planning.Judge == nil || w.Planning.Judge.Cycle != 1 || w.Planning.Judge.MaxCycles != 0 {
		t.Fatalf("juge non initialisé à l'activation : %+v", w.Planning.Judge)
	}
	got, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "objectif en cours, aucun blocage"})
	if err != nil {
		t.Fatal(err)
	}
	if got.Planning.Judge.Verdict != "continue" || len(got.Planning.Judge.History) != 1 || got.Planning.Judge.History[0].Cycle != 1 {
		t.Fatalf("verdict non enregistré : %+v", got.Planning.Judge)
	}
	if got.Planning.Judge.Closed {
		t.Fatal("continue ne doit jamais clore la mission")
	}
	// Distinctness: the verdict never touches the candidate-review surface.
	if got.Planning.Repository != nil || got.Planning.ReviewerRequired || got.Planning.Reviewer != nil {
		t.Fatal("le juge global a modifié un champ de la revue indépendante de candidat")
	}
}

// req-8: no global closure ("end") without an attested candidate matching
// the engine's own current managed-repository candidate.
func judgeAcceptedFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s, w := managedFixture(t)
	for _, id := range []string{"first", "second"} {
		a := managedCompleted(t, s, w, id, "Accepted "+id+"\n")
		if err := s.finishAgent(a, "completed", "", nil); err != nil {
			t.Fatal(err)
		}
		a, _ = s.agent(a.ID)
		if err := s.integrateManagedAttempt(a); err != nil {
			t.Fatal(err)
		}
		w, _ = s.get(w.ID)
	}
	for i := range w.Tasks {
		if !s.acceptedFresh(&w, &w.Tasks[i], map[string]bool{}) {
			t.Fatal("fixture lacks fresh acceptance")
		}
	}
	return s, w
}

func TestPlanningJudgeEndRequiresFreshMatchingCandidate(t *testing.T) {
	s, w := judgeAcceptedFixture(t)
	for _, candidate := range []string{"", "candidate-different"} {
		if _, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "end", Reason: "Toutes les preuves sont examinées", ExpectedCandidate: candidate}); err == nil {
			t.Fatal("absent or different candidate accepted")
		}
	}
	pending := []string{}
	for _, event := range w.Planning.Inbox {
		if event.Scope == "root" && event.Decision == "" {
			pending = append(pending, event.ID)
		}
	}
	// A leased decision acknowledges the events before a separate direct verdict.
	w, request := planningClaim(t, s, w, "root")
	request.Operations = nil
	var err error
	w, err = s.planningChange(w.ID, "decide", request)
	if err != nil {
		t.Fatal(err)
	}
	for _, event := range w.Planning.Inbox {
		if containsString(pending, event.ID) && event.Decision == "" {
			t.Fatal("inbox event not acknowledged")
		}
	}
	got, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "end", Reason: "Toutes les tâches sont acceptées avec preuves fraîches", ExpectedCandidate: w.Planning.Repository.Candidate})
	if err != nil {
		t.Fatal(err)
	}
	if !got.Planning.Judge.Closed || got.Planning.Judge.EvidenceRef != w.Planning.Repository.Candidate || got.Planning.Scopes[0].State != w.Planning.Scopes[0].State {
		t.Fatal("judge verdict replaced scope conduct or omitted evidence")
	}
	if _, err = judgeDo(t, s, got, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "Reprise"}); err == nil {
		t.Fatal("closed judge reopened")
	}
	if _, err = judgeDo(t, s, got, "renew", 1, PlanningRequest{}); err == nil {
		t.Fatal("closed judge renewed")
	}
}

func TestPlanningJudgeEndRejectsStaleReviewAndActiveProcess(t *testing.T) {
	for _, kind := range []string{"review", "control", "active"} {
		t.Run(kind, func(t *testing.T) {
			s, w := judgeAcceptedFixture(t)
			switch kind {
			case "review":
				w.Tasks[0].IndependentReview.State = "changes_requested"
			case "control":
				if err := os.WriteFile(filepath.Join(s.root, w.Tasks[0].AutoValidation.Receipt), []byte("altered control receipt"), 0600); err != nil {
					t.Fatal(err)
				}
			case "active":
				a := Agent{ID: "active-end-fixture", WorkID: w.ID, TaskID: "first", Role: "worker", Status: "running"}
				body, _ := json.Marshal(a)
				if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,'','running','',?,?)", a.ID, w.ID, a.TaskID, body, []byte(`{}`)); err != nil {
					t.Fatal(err)
				}
			}
			body, _ := json.Marshal(w)
			if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
				t.Fatal(err)
			}
			if _, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "end", Reason: "Même identité ne remplace pas les preuves", ExpectedCandidate: w.Planning.Repository.Candidate}); err == nil {
				t.Fatal("invalid end accepted", kind)
			}
			got, _ := s.get(w.ID)
			if got.Revision != w.Revision || got.Planning.Judge.Closed || len(got.Planning.Judge.History) != 0 {
				t.Fatal("rejected end mutated judge")
			}
		})
	}
}

// req-6: renewal opens a fresh cycle (new context for the next verdict)
// while preserving objective, requirements, counters and judge history.
func TestPlanningJudgeRenewalPreservesContextAndHistory(t *testing.T) {
	s, w := planningFixture(t)
	w, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "en cours"})
	if err != nil {
		t.Fatal(err)
	}
	objectiveBefore, requirementsBefore := w.Planning.Scopes[0].Objective, w.Planning.Scopes[0].Requirements
	maxActivationsBefore, maxTasksBefore := w.Planning.MaxActivations, w.Planning.MaxTasks
	got, err := judgeDo(t, s, w, "renew", 1, PlanningRequest{})
	if err != nil {
		t.Fatal(err)
	}
	j := got.Planning.Judge
	if j.Cycle != 2 || j.Verdict != "" || j.Reason != "" || j.EvidenceRef != "" {
		t.Fatalf("cycle non renouvelé proprement : %+v", j)
	}
	if len(j.History) != 1 || j.History[0].Cycle != 1 || j.History[0].Verdict != "continue" || j.History[0].RenewedAt == "" {
		t.Fatalf("historique du cycle précédent perdu ou non marqué renouvelé : %+v", j.History)
	}
	if got.Planning.Scopes[0].Objective != objectiveBefore || len(got.Planning.Scopes[0].Requirements) != len(requirementsBefore) {
		t.Fatal("objectif ou exigences perdus par le renouvellement")
	}
	if got.Planning.MaxActivations != maxActivationsBefore || got.Planning.MaxTasks != maxTasksBefore {
		t.Fatal("limites de la mission modifiées par un simple renouvellement de cycle")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	persisted, err := reopened.get(got.ID)
	if err != nil {
		t.Fatal(err)
	}
	context, err := planningContext(persisted, "root")
	if err != nil {
		t.Fatal(err)
	}
	var projection struct {
		Cycle struct {
			Cycle   int    `json:"cycle"`
			Verdict string `json:"verdict"`
			History int    `json:"history_records"`
		} `json:"global_cycle"`
	}
	if err = json.Unmarshal(context, &projection); err != nil || projection.Cycle.Cycle != 2 || projection.Cycle.Verdict != "" || projection.Cycle.History != 1 {
		t.Fatal("renewed context missing after reopening", err, string(context))
	}
	// A second verdict can now be rendered on the fresh cycle.
	got, err = judgeDo(t, s, got, "judge", 2, PlanningRequest{JudgeVerdict: "continue", Reason: "toujours en cours"})
	if err != nil {
		t.Fatal(err)
	}
	if len(got.Planning.Judge.History) != 2 {
		t.Fatalf("deuxième cycle non consigné : %+v", got.Planning.Judge.History)
	}
}

// req-7: replay (same event_id), a stale cycle number and a double verdict
// on the same cycle must all refuse the effect -- no double application.
func TestPlanningJudgeReplayStaleAndDoubleVerdictRefused(t *testing.T) {
	s, w := planningFixture(t)
	owner, _ := w.Planning.scope("root")
	request := PlanningRequest{Schema: 1, EventID: "fixed-judge-event", Revision: w.Revision, Scope: "root", ScopeRevision: owner.Revision, JudgeCycle: 1, JudgeVerdict: "continue", Reason: "premier verdict"}
	first, err := s.planningChange(w.ID, "judge", request)
	if err != nil {
		t.Fatal(err)
	}
	// Exact replay (same event_id, same content): idempotent, no new effect.
	replay, err := s.planningChange(w.ID, "judge", request)
	if err != nil {
		t.Fatal(err)
	}
	if replay.Revision != first.Revision || len(replay.Planning.Judge.History) != 1 {
		t.Fatalf("rejeu a produit un second effet : %+v", replay.Planning.Judge)
	}
	// Double verdict on the same cycle, different event: refused.
	if _, err = judgeDo(t, s, first, "judge", 1, PlanningRequest{JudgeVerdict: "end", Reason: "second verdict", ExpectedCandidate: "x"}); err == nil {
		t.Fatal("second verdict sur le même cycle accepté")
	}
	renewed, err := judgeDo(t, s, first, "renew", 1, PlanningRequest{})
	if err != nil {
		t.Fatal(err)
	}
	// Stale cycle: still claiming cycle 1 after renewal moved to cycle 2.
	if _, err = judgeDo(t, s, renewed, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "verdict périmé"}); err == nil {
		t.Fatal("verdict basé sur un cycle périmé accepté")
	}
	// Renewal cannot be requested twice for the same (now stale) cycle either.
	if _, err = judgeDo(t, s, renewed, "renew", 1, PlanningRequest{}); err == nil {
		t.Fatal("renouvellement basé sur un cycle périmé accepté")
	}
}

// req-7 (quota/limite): a mission paused refuses judge/renew effects, and
// exhausting the max_judge_cycles quota refuses further renewal.
func TestPlanningJudgePausedAndQuotaRefuseEffects(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, MaxJudgeCycles: 1})
	if w.Planning.Judge.MaxCycles != 1 {
		t.Fatalf("quota configuré non respecté : %+v", w.Planning.Judge)
	}
	w, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "en cours"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = judgeDo(t, s, w, "renew", 1, PlanningRequest{}); err == nil {
		t.Fatal("renouvellement accepté au-delà du quota de cycles")
	}
	// Pause the mission, then confirm a fresh judge call on the still-live
	// cycle is refused while paused.
	w = planningDo(t, s, w, "pause", PlanningRequest{})
	if !w.Planning.Paused {
		t.Fatal("mission non marquée en pause")
	}
	owner, _ := w.Planning.scope("root")
	_, err = s.planningChange(w.ID, "renew", PlanningRequest{Schema: 1, EventID: newID("test-judge-"), Revision: w.Revision, Scope: "root", ScopeRevision: owner.Revision, JudgeCycle: w.Planning.Judge.Cycle})
	if err == nil {
		t.Fatal("renouvellement accepté sur une mission en pause")
	}
}

func TestPlanningJudgeDirectEndCannotAcknowledgeInputs(t *testing.T) {
	s, w := judgeAcceptedFixture(t)
	var pending []string
	for _, event := range w.Planning.Inbox {
		if event.Scope == "root" && event.Decision == "" {
			pending = append(pending, event.ID)
		}
	}
	if len(pending) == 0 {
		t.Fatal("fixture lacks pending events")
	}
	for _, inputs := range [][]string{pending, append(append([]string(nil), pending...), "foreign-event")} {
		_, err := judgeDo(t, s, w, "judge", 1, PlanningRequest{Inputs: inputs, JudgeVerdict: "end", Reason: "A direct verdict cannot acknowledge inbox events", ExpectedCandidate: w.Planning.Repository.Candidate})
		if err == nil {
			t.Fatal("direct end silently exempted pending events")
		}
		got, err := s.get(w.ID)
		if err != nil || !reflect.DeepEqual(got, w) {
			t.Fatal("refused verdict mutated work", err)
		}
	}
}

func TestPlanningJudgePauseIsIndependentOfQuota(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = planningDo(t, s, w, "enable", PlanningRequest{MaxTasks: 10, MaxDecisions: 10, MaxActivations: 15, MaxJudgeCycles: 2})
	var err error
	w, err = judgeDo(t, s, w, "judge", 1, PlanningRequest{JudgeVerdict: "continue", Reason: "Continue before isolated pause check"})
	if err != nil {
		t.Fatal(err)
	}
	w = planningDo(t, s, w, "pause", PlanningRequest{})
	_, err = judgeDo(t, s, w, "renew", 1, PlanningRequest{})
	if err == nil || !strings.Contains(err.Error(), "planification en pause") {
		t.Fatal("pause did not supply the refusal", err)
	}
	got, e := s.get(w.ID)
	if e != nil || !reflect.DeepEqual(got, w) {
		t.Fatal("paused renewal changed work", e)
	}
	w = planningDo(t, s, w, "resume", PlanningRequest{})
	got, err = judgeDo(t, s, w, "renew", 1, PlanningRequest{})
	if err != nil || got.Planning.Judge.Cycle != 2 || len(got.Planning.Judge.History) != 1 {
		t.Fatal("same renewal did not succeed after resume", err)
	}
}
