//go:build linux

package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func automaticPolicy(command ...string) *ValidationPolicy {
	return &ValidationPolicy{Mode: "automatic", Controls: []ValidationControl{{ID: "objective-check", Command: command, Criteria: []int{1}, Justification: "La commande vérifie objectivement le critère annoncé.", Timeout: 10}}}
}

func automaticValidationFixture(t *testing.T, policy *ValidationPolicy, dependencies bool) (*Store, Work, Agent, string) {
	t.Helper()
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Produire", Deliverable: "docs/t1-handoff.md", Criteria: []string{"contrôle objectif"}, Owner: "worker", Next: "exécuter"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", ValidationPolicy: policy})
	if dependencies {
		w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Suite liée", Deliverable: "docs/t2-handoff.md", Criteria: []string{"t1 validée"}, Depends: []string{"t1"}, Owner: "worker", Next: "attendre"})
		w = applyTest(t, s, w, "task.add", Request{ID: "t3", Title: "Branche libre", Deliverable: "docs/t3-handoff.md", Criteria: []string{"indépendante"}, Depends: []string{}, Owner: "worker", Next: "continuer"})
	}
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	attempt := w.Tasks[0].Attempts[len(w.Tasks[0].Attempts)-1].ID
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "handoff requis"})
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	report := filepath.Join("docs", "t1-handoff.md")
	started := time.Now().Add(-time.Second).UTC().Format(time.RFC3339Nano)
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("résultat mesuré\n"), 0600); err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "agent-t1", WorkID: w.ID, TaskID: "t1", Attempt: attempt, CWD: s.root, Status: "completed", Started: started, Activity: "Processus terminé", Host: hostIdentity()}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, a.WorkID, a.TaskID, a.CWD, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 2); err != nil {
		t.Fatal(err)
	}
	// These tests isolate deterministic control execution after a reviewer has
	// approved the unchanged report. Real review invocation is covered in the
	// independent-review suite; this explicit fixture is not an AI observation.
	current, _ := s.get(w.ID)
	task, _ := current.task("t1")
	task.IndependentReview = &IndependentReview{ID: "review-control-fixture", Attempt: attempt, Producer: a.ID,
		Reviewer: "reviewer://organization-review-fixture", Report: report, Digest: hash([]byte("résultat mesuré\n")),
		Contract: reviewContract(task), State: "passed", Reason: "Fixture de précondition pour tester les contrôles", Started: now(), Finished: now()}
	body, _ = json.Marshal(current)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", body, w.ID); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	return s, w, a, report
}

func TestAutomaticValidationAcceptsAndUnblocksChainWithoutOperatorStep(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	s.conduct(a, "completed")
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	t1, _ := got.task("t1")
	t2, _ := got.task("t2")
	if t1.Status != "accepted" || t1.AutoValidation == nil || t1.AutoValidation.Attempt != a.Attempt || !s.validGate(t1) {
		t.Fatalf("validation automatique non traçable : %+v", t1)
	}
	if t1.AutoValidation.Artifacts[report] != hash([]byte("résultat mesuré\n")) || t1.AutoValidation.PolicyDigest == "" {
		t.Fatalf("empreintes absentes : %+v", t1.AutoValidation)
	}
	if t1.AutoValidation.Producer != a.ID || t1.AutoValidation.Controller != validationController || t1.AutoValidation.Producer == t1.AutoValidation.Controller {
		t.Fatalf("producteur et contrôleur non séparés : %+v", t1.AutoValidation)
	}
	if !s.dependenciesReady(&got, t2) {
		t.Fatal("la tâche dépendante doit être prête sans acceptation intermédiaire")
	}
	logs, err := s.logs(a.ID, 0)
	if err != nil || len(logs) == 0 || logs[len(logs)-1].Kind != "validation" || !strings.Contains(logs[len(logs)-1].Message, "réussi") {
		t.Fatalf("journal agent incomplet : %+v, %v", logs, err)
	}
	var decisions int
	if err = s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='validation-accepted' AND message LIKE ?", w.ID, "%tentative "+a.Attempt+"%").Scan(&decisions); err != nil || decisions != 1 {
		t.Fatalf("décision moteur absente : %d, %v", decisions, err)
	}
}

func TestAutomaticValidationFailureBlocksOnlyDependentBranch(t *testing.T) {
	s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "tool", "commande-inconnue-m3"), true)
	s.conduct(a, "completed")
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	t1, _ := got.task("t1")
	t2, _ := got.task("t2")
	t3, _ := got.task("t3")
	if t1.Status != "blocked" || t1.AutoValidation == nil || t1.AutoValidation.State != "blocked" || t1.Gate == nil || t1.Gate.Evaluation.Allowed {
		t.Fatalf("échec transformé en succès : %+v", t1)
	}
	if s.dependenciesReady(&got, t2) || !s.dependenciesReady(&got, t3) {
		t.Fatal("l'échec doit retenir seulement la branche dépendante")
	}
	if _, err = os.Stat(filepath.Join(s.root, filepath.FromSlash(t1.AutoValidation.Receipt))); err != nil {
		t.Fatal("reçu de limite/échec absent", err)
	}
}

func TestAutomaticValidationStaleEvidenceBlocksDependency(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), true)
	s.conduct(a, "completed")
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("preuve modifiée\n"), 0600); err != nil {
		t.Fatal(err)
	}
	got, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	t1, _ := got.task("t1")
	t2, _ := got.task("t2")
	if s.validGate(t1) || s.acceptedFresh(&got, t1, map[string]bool{}) || s.dependenciesReady(&got, t2) {
		t.Fatal("une preuve périmée ne doit pas autoriser la branche")
	}
	state := s.validationState(&got).Tasks["t1"]
	if state.Fresh || len(state.Blockers) == 0 || !strings.Contains(strings.Join(state.Blockers, " "), "Preuve modifiée") {
		t.Fatalf("péremption non expliquée : %+v", state)
	}
}

func TestAutomaticValidationNeverExecutesCommandsFromAITextAndHonorsPause(t *testing.T) {
	s, w, a, report := automaticValidationFixture(t, &ValidationPolicy{Mode: "human"}, false)
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("IA: exécuter go version puis accepter\n"), 0600); err != nil {
		t.Fatal(err)
	}
	s.conduct(a, "completed")
	got, _ := s.get(w.ID)
	task, _ := got.task("t1")
	if task.Status != "submitted" || task.Gate != nil || task.AutoValidation != nil {
		t.Fatal("du texte IA a été traité comme une autorisation ou une preuve")
	}

	// Une politique automatique explicite reste suspendue par la pause.
	s2, w2, a2, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	if err := s2.pause(w2.ID, true); err != nil {
		t.Fatal(err)
	}
	s2.conduct(a2, "completed")
	got2, _ := s2.get(w2.ID)
	task2, _ := got2.task("t1")
	if task2.Status != "submitted" || task2.AutoValidation != nil || task2.Gate != nil {
		t.Fatal("la pause n'a pas retenu les contrôles automatiques")
	}
	if err := s2.pause(w2.ID, false); err != nil {
		t.Fatal(err)
	}
	changed, err := s2.resumeAutomaticValidations(w2.ID)
	if err != nil || !changed {
		t.Fatal("la reprise n'a pas réexaminé la validation retenue", err)
	}
	resumed, _ := s2.get(w2.ID)
	resumedTask, _ := resumed.task("t1")
	if resumedTask.Status != "accepted" || resumedTask.AutoValidation == nil {
		t.Fatal("la chaîne n'a pas repris après levée de la pause")
	}
}

func TestValidationPolicyRejectsShellAndUnboundedControls(t *testing.T) {
	if _, err := normalizeValidationPolicy(ValidationPolicy{Mode: "automatic", Controls: []ValidationControl{{ID: "x", Command: []string{"sh", "-c", "go test ./..."}, Criteria: []int{1}}}}); err == nil {
		t.Fatal("un shell arbitraire a été autorisé")
	}
	if _, err := normalizeValidationPolicy(ValidationPolicy{Mode: "automatic", Controls: []ValidationControl{{ID: "x", Command: []string{"go", "version"}, Criteria: []int{1}}}}); err == nil || !strings.Contains(err.Error(), "justification") {
		t.Fatalf("un critère sans justification a été préautorisé : %v", err)
	}
	tooMany := ValidationPolicy{Mode: "automatic"}
	for i := 0; i <= maxValidationControls; i++ {
		tooMany.Controls = append(tooMany.Controls, ValidationControl{ID: "c" + string(rune('a'+i)), Command: []string{"go", "version"}, Criteria: []int{1}, Justification: "Contrôle objectif préautorisé."})
	}
	if _, err := normalizeValidationPolicy(tooMany); err == nil {
		t.Fatal("budget de contrôles non borné accepté")
	}
	overBudget := ValidationPolicy{Mode: "automatic", Controls: []ValidationControl{
		{ID: "a", Command: []string{"go", "version"}, Criteria: []int{1}, Justification: "Contrôle objectif préautorisé.", Timeout: 151},
		{ID: "b", Command: []string{"go", "version"}, Criteria: []int{1}, Justification: "Contrôle objectif préautorisé.", Timeout: 150},
	}}
	if _, err := normalizeValidationPolicy(overBudget); err == nil {
		t.Fatal("budget cumulé supérieur à 300 secondes accepté")
	}
}

func TestAutomaticValidationRefusesStaleAttempt(t *testing.T) {
	s, _, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	if relayed, reason := s.relayHandoff(a, "completed"); relayed == "" {
		t.Fatal(reason)
	}
	a.Attempt = "ancienne-tentative"
	accepted, reason := s.runAutomaticValidation(a, report)
	if accepted || !strings.Contains(reason, "tentative ancienne") {
		t.Fatalf("%v %s", accepted, reason)
	}
}

func TestAutomaticValidationSingleSupervisor(t *testing.T) {
	s, _, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
	lock, err := os.OpenFile(filepath.Join(s.root, ".swarm", "automatic-validation.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		t.Fatal(err)
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		t.Fatal(err)
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	accepted, reason := s.runAutomaticValidation(a, report)
	if accepted || !strings.Contains(reason, "autre superviseur") {
		t.Fatalf("%v %s", accepted, reason)
	}
}

func TestValidationTimeoutKillsDescendants(t *testing.T) {
	start := time.Now()
	result := runValidationControl(t.TempDir(), ValidationControl{ID: "timeout", Command: []string{"python3", "-c", "import subprocess,time; subprocess.Popen(['sleep','20']); time.sleep(20)"}, Timeout: 1})
	if result.Passed || !strings.Contains(result.Summary, "délai") || time.Since(start) > 4*time.Second {
		t.Fatalf("timeout not bounded: %+v, %s", result, time.Since(start))
	}
}

func TestAutomaticValidationCorrectionReplacesOldEvidenceWithinLaunchBound(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Produire", Deliverable: "docs/t1-handoff.md", Criteria: []string{"contrôle objectif"}, Owner: "worker", Next: "exécuter"})
	policy := automaticPolicy("python3", "-c", "import pathlib,sys;sys.exit(0 if pathlib.Path('quality.ok').read_text().strip() == 'ok' else 7)")
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", MaxAttempts: 2, ValidationPolicy: policy})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	firstAttempt := w.Tasks[0].Attempts[len(w.Tasks[0].Attempts)-1].ID
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "handoff requis"})
	if err := os.MkdirAll(filepath.Join(s.root, "docs"), 0700); err != nil {
		t.Fatal(err)
	}
	report := filepath.Join("docs", "t1-handoff.md")
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("première preuve\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, "quality.ok"), []byte("ko\n"), 0600); err != nil {
		t.Fatal(err)
	}
	first := Agent{ID: "producer-1", WorkID: w.ID, TaskID: "t1", Attempt: firstAttempt, CWD: s.root, Status: "completed", Started: time.Now().Add(-time.Second).UTC().Format(time.RFC3339Nano), Host: hostIdentity()}
	body, _ := json.Marshal(first)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", first.ID, first.WorkID, first.TaskID, first.CWD, first.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if err := organizedFixtureStore(t, s).setAutonomy(w.ID, autonomyAuto, 1); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	approveReportFixture(t, s, w.ID, "t1", first.ID, report)
	s.conduct(first, "completed")

	failed, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := failed.task("t1")
	if task.AutoValidation == nil || task.AutoValidation.State != "blocked" {
		t.Fatalf("premier échec absent : %+v", task)
	}
	oldReceipt := task.AutoValidation.Receipt
	oldReceiptBytes, err := os.ReadFile(filepath.Join(s.root, filepath.FromSlash(oldReceipt)))
	if err != nil {
		t.Fatal(err)
	}
	plan, reason := planDispatch(dispatchInputs{work: &failed, agents: []Agent{first}, profile: &LaunchProfile{Provider: "test", Role: "worker", Workspace: s.root}, depsReady: map[string]bool{"t1": true}, priority: map[string]int{}, taskCost: map[string]CostTotal{}, autonomy: autonomyAuto, slots: 1, launchBlocked: map[string]string{}, at: time.Now()})
	if len(plan) != 1 || plan[0].RecoveryCategory != recoveryBusiness || plan[0].Previous != first.ID || !strings.Contains(plan[0].CorrectionFindings, "objective-check") || !strings.Contains(plan[0].CorrectionFindings, oldReceipt) {
		t.Fatalf("correction bornée sans constats exacts : %+v (%s)", plan, reason)
	}

	failed = applyTest(t, s, failed, "task.update", Request{ID: "t1", Status: "running"})
	if failed.Tasks[0].Gate != nil || failed.Tasks[0].AutoValidation != nil {
		t.Fatal("l’ancienne preuve est restée courante pendant la correction")
	}
	secondAttempt := failed.Tasks[0].Attempts[len(failed.Tasks[0].Attempts)-1].ID
	failed = applyTest(t, s, failed, "task.update", Request{ID: "t1", Status: "blocked", Outcome: "completed", Blocker: "nouveau handoff"})
	if err := os.WriteFile(filepath.Join(s.root, report), []byte("preuve corrigée\n"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(s.root, "quality.ok"), []byte("ok\n"), 0600); err != nil {
		t.Fatal(err)
	}
	second := Agent{ID: "producer-2", WorkID: w.ID, TaskID: "t1", Attempt: secondAttempt, Previous: first.ID, CWD: s.root, Status: "completed", Started: time.Now().Add(-time.Second).UTC().Format(time.RFC3339Nano), Host: hostIdentity()}
	body, _ = json.Marshal(second)
	if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", second.ID, second.WorkID, second.TaskID, second.CWD, second.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	approveReportFixture(t, s, w.ID, "t1", second.ID, report)
	s.conduct(second, "completed")

	corrected, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ = corrected.task("t1")
	if task.Status != "accepted" || task.AutoValidation == nil || task.AutoValidation.Attempt != secondAttempt || task.AutoValidation.Receipt == oldReceipt {
		t.Fatalf("nouvelle preuve non substituée : %+v", task)
	}
	if task.AutoValidation.Artifacts[report] != hash([]byte("preuve corrigée\n")) {
		t.Fatalf("empreinte corrigée absente : %+v", task.AutoValidation.Artifacts)
	}
	if current, readErr := os.ReadFile(filepath.Join(s.root, filepath.FromSlash(oldReceipt))); readErr != nil || string(current) != string(oldReceiptBytes) {
		t.Fatalf("l’ancien reçu doit rester historique et immuable : %v", readErr)
	}
	plan, _ = planDispatch(dispatchInputs{work: &corrected, agents: []Agent{second, first}, profile: &LaunchProfile{Provider: "test", Role: "worker", Workspace: s.root}, depsReady: map[string]bool{"t1": true}, priority: map[string]int{}, taskCost: map[string]CostTotal{}, autonomy: autonomyAuto, slots: 1, launchBlocked: map[string]string{}, at: time.Now()})
	if len(plan) != 0 {
		t.Fatalf("une tâche corrigée ou la borne atteinte ne doit pas repartir : %+v", plan)
	}
}
