//go:build linux

package main

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

const (
	maxValidationControls = 8
	maxValidationArgs     = 32
	maxValidationOutput   = 64 << 10
	// Le préfixe contient des caractères interdits aux identifiants d'agents :
	// un producteur ne peut donc jamais se faire passer pour ce contrôleur.
	validationController = "controller://swarm-validation"
)

var validationPrograms = map[string]bool{
	"go": true, "git": true, "node": true, "npm": true,
	"python": true, "python3": true, "pytest": true,
}

func normalizeValidationPolicy(p ValidationPolicy) (ValidationPolicy, error) {
	p.Mode = strings.TrimSpace(p.Mode)
	if p.Mode != "human" && p.Mode != "automatic" {
		return p, fmt.Errorf("validation_policy.mode : human ou automatic requis")
	}
	if p.Mode == "human" && len(p.Controls) != 0 {
		return p, fmt.Errorf("une politique human ne peut pas préautoriser de contrôle")
	}
	if p.Mode == "automatic" && (len(p.Controls) == 0 || len(p.Controls) > maxValidationControls) {
		return p, fmt.Errorf("une politique automatic exige 1 à %d contrôles", maxValidationControls)
	}
	seen := map[string]bool{}
	totalTimeout := 0
	for i := range p.Controls {
		c := &p.Controls[i]
		c.ID = strings.TrimSpace(c.ID)
		if !safeName(c.ID) || seen[c.ID] {
			return p, fmt.Errorf("identifiant de contrôle absent, invalide ou dupliqué")
		}
		if c.ID == "deliverable" {
			return p, fmt.Errorf("deliverable est un identifiant de contrôle réservé")
		}
		seen[c.ID] = true
		if len(c.Command) == 0 || len(c.Command) > maxValidationArgs || !validationPrograms[c.Command[0]] {
			return p, fmt.Errorf("%s : programme non autorisé ou commande trop longue", c.ID)
		}
		if len(c.Criteria) == 0 {
			return p, fmt.Errorf("%s : au moins un critère couvert est requis", c.ID)
		}
		c.Justification = strings.TrimSpace(c.Justification)
		if len([]rune(c.Justification)) < 8 || len([]rune(c.Justification)) > 500 {
			return p, fmt.Errorf("%s : justification objective requise (8 à 500 caractères)", c.ID)
		}
		criterionSeen := map[int]bool{}
		for _, criterion := range c.Criteria {
			if criterion < 1 || criterionSeen[criterion] {
				return p, fmt.Errorf("%s : indices de critères invalides ou dupliqués", c.ID)
			}
			criterionSeen[criterion] = true
		}
		for _, arg := range c.Command {
			if strings.TrimSpace(arg) == "" || strings.ContainsRune(arg, 0) {
				return p, fmt.Errorf("%s : argument vide ou invalide", c.ID)
			}
		}
		if filepath.IsAbs(c.Dir) {
			return p, fmt.Errorf("%s : dir doit être relatif au projet", c.ID)
		}
		if c.Timeout == 0 {
			c.Timeout = 60
		}
		if c.Timeout < 1 || c.Timeout > 300 {
			return p, fmt.Errorf("%s : timeout_seconds doit être compris entre 1 et 300", c.ID)
		}
		totalTimeout += c.Timeout
	}
	if totalTimeout > 300 {
		return p, fmt.Errorf("budget cumulé des contrôles supérieur à 300 secondes")
	}
	p.Authorized, p.Actor = "", ""
	return p, nil
}

func validationPolicyCoversTask(p ValidationPolicy, t *Task) error {
	if p.Mode != "automatic" {
		return nil
	}
	criteria := map[int]bool{}
	controls := map[string]bool{}
	for _, control := range p.Controls {
		controls[control.ID] = true
		for _, criterion := range control.Criteria {
			if criterion > len(t.Criteria) {
				return fmt.Errorf("%s : critère %d inconnu", control.ID, criterion)
			}
			criteria[criterion] = true
		}
	}
	for i := range t.Criteria {
		if !criteria[i+1] {
			return fmt.Errorf("critère %d sans contrôle automatique explicite", i+1)
		}
	}
	for id := range t.PlanChecks {
		if !controls[id] {
			return fmt.Errorf("contrôle obligatoire du plan non préautorisé : %s", id)
		}
	}
	return nil
}

func validationPolicyDigest(p ValidationPolicy) string {
	copy := p
	copy.Authorized, copy.Actor = "", ""
	b, _ := json.Marshal(copy)
	return hash(b)
}

type limitedValidationOutput struct {
	b bytes.Buffer
	n int
}

func (w *limitedValidationOutput) Write(p []byte) (int, error) {
	w.n += len(p)
	remaining := maxValidationOutput - w.b.Len()
	if remaining > 0 {
		if len(p) > remaining {
			_, _ = w.b.Write(p[:remaining])
		} else {
			_, _ = w.b.Write(p)
		}
	}
	return len(p), nil
}

func validationDir(root, rel string) (string, error) {
	if rel == "" || rel == "." {
		return root, nil
	}
	joined := filepath.Join(root, rel)
	resolved, err := filepath.EvalSymlinks(joined)
	if err != nil {
		return "", err
	}
	inside, err := filepath.Rel(root, resolved)
	if err != nil || inside == ".." || strings.HasPrefix(inside, ".."+string(filepath.Separator)) {
		return "", fmt.Errorf("répertoire de contrôle hors projet : %s", rel)
	}
	info, err := os.Stat(resolved)
	if err != nil || !info.IsDir() {
		return "", fmt.Errorf("répertoire de contrôle invalide : %s", rel)
	}
	return resolved, nil
}

func runValidationControl(root string, c ValidationControl) ValidationControlResult {
	r := ValidationControlResult{ID: c.ID, ExitCode: -1}
	dir, err := validationDir(root, c.Dir)
	if err != nil {
		r.Summary = err.Error()
		return r
	}
	ctx, cancel := context.WithTimeout(context.Background(), time.Duration(c.Timeout)*time.Second)
	defer cancel()
	cmd := exec.CommandContext(ctx, c.Command[0], c.Command[1:]...)
	cmd.Dir = dir
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	cmd.Cancel = func() error { return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) }
	cmd.WaitDelay = time.Second
	defer func() {
		if cmd.Process != nil {
			_ = syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		}
	}()
	var output limitedValidationOutput
	cmd.Stdout, cmd.Stderr = &output, &output
	err = cmd.Run()
	r.OutputHash = hash(output.b.Bytes())
	if err == nil {
		r.Passed, r.ExitCode, r.Summary = true, 0, "contrôle réussi"
	} else if ctx.Err() == context.DeadlineExceeded {
		r.Summary = fmt.Sprintf("délai de %ds dépassé", c.Timeout)
	} else if exit, ok := err.(*exec.ExitError); ok {
		r.ExitCode = exit.ExitCode()
		r.Summary = fmt.Sprintf("contrôle en échec (code %d)", r.ExitCode)
	} else {
		r.Summary = "exécution impossible : " + err.Error()
	}
	if output.n > maxValidationOutput {
		r.Summary += fmt.Sprintf(" ; sortie tronquée à %d octets", maxValidationOutput)
	}
	return r
}

func (s *Store) automaticValidationAuthorized(work string) (bool, string) {
	p, err := s.missionPolicy(work)
	if err != nil {
		return false, "politique de mission illisible : " + err.Error()
	}
	if !p.Enabled || s.autonomy(work) != autonomyAuto {
		return false, "mission continue non autorisée"
	}
	if s.paused(work) {
		return false, "mission en pause"
	}
	return true, ""
}

func automaticValidationGuard(tx *sql.Tx, work string) error {
	var mode string
	var paused int
	if err := tx.QueryRow("SELECT autonomy,paused FROM cockpit_controls WHERE work_id=?", work).Scan(&mode, &paused); err != nil {
		return fmt.Errorf("autorisation de validation absente : %w", err)
	}
	if mode != autonomyAuto || paused != 0 {
		return fmt.Errorf("validation automatique suspendue ou autonomie désactivée")
	}
	var raw string
	if err := tx.QueryRow("SELECT message FROM cockpit_events WHERE work_id=? AND kind='mission-policy' ORDER BY rowid DESC LIMIT 1", work).Scan(&raw); err != nil {
		return fmt.Errorf("autorisation de mission absente : %w", err)
	}
	var policy MissionPolicy
	if err := json.Unmarshal([]byte(raw), &policy); err != nil || !policy.Enabled {
		return fmt.Errorf("autorisation de mission absente ou révoquée")
	}
	return nil
}

func (s *Store) runAutomaticValidation(a Agent, report string) (bool, string) {
	// Serialize controls and receipt publication across web/watch processes.
	// A crashed holder releases the OS lock automatically.
	lock, err := os.OpenFile(filepath.Join(s.root, ".swarm", "automatic-validation.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return false, "verrou des contrôles inaccessible : " + err.Error()
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		return false, "contrôles déjà pris en charge par un autre superviseur"
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	w, err := s.get(a.WorkID)
	if err != nil {
		return false, err.Error()
	}
	t, err := w.task(a.TaskID)
	if err != nil || t.Status != "submitted" {
		return false, "tâche non soumise ou introuvable"
	}
	candidates, readErr := s.agents(a.WorkID)
	if readErr != nil {
		return false, "tentatives illisibles : " + readErr.Error()
	}
	currentAttempt := false
	for _, candidate := range candidates {
		if candidate.TaskID == a.TaskID {
			currentAttempt = candidate.ID == a.ID && candidate.Attempt == a.Attempt && candidate.Status == "completed"
			break
		}
	}
	if !currentAttempt {
		return false, "tentative ancienne ou non terminée ; validation refusée"
	}

	if t.ValidationPolicy == nil || t.ValidationPolicy.Mode != "automatic" {
		return false, "revue humaine conservée : aucun contrôle automatique préautorisé"
	}
	if ok, reason := s.automaticValidationAuthorized(w.ID); !ok {
		return false, "validation automatique retenue : " + reason
	}
	if e := s.independentReviewGuard(&w, t); e != nil {
		return false, e.Error()
	}
	policy := *t.ValidationPolicy
	if err := validationPolicyCoversTask(policy, t); err != nil {
		return false, "revue humaine conservée : " + err.Error()
	}
	digest := validationPolicyDigest(policy)
	if t.AutoValidation != nil && t.AutoValidation.Attempt == a.Attempt && t.AutoValidation.PolicyDigest == digest {
		return t.AutoValidation.State == "accepted", "validation déjà décidée pour cette tentative"
	}
	reportPath, err := safeReport(s.root, report)
	if err != nil {
		return false, "livrable inaccessible : " + err.Error()
	}
	reportBytes, err := os.ReadFile(reportPath)
	if err != nil || len(reportBytes) == 0 {
		return false, "livrable vide ou illisible"
	}
	results := make([]ValidationControlResult, 0, len(policy.Controls))
	passed := true
	for _, control := range policy.Controls {
		if ok, reason := s.automaticValidationAuthorized(w.ID); !ok {
			return false, "validation automatique retenue : " + reason
		}
		result := runValidationControl(s.root, control)
		results = append(results, result)
		passed = passed && result.Passed
	}
	relReceipt := filepath.ToSlash(filepath.Join(".swarm", "validation", w.ID, t.ID, a.Attempt+".json"))
	absReceipt := filepath.Join(s.root, filepath.FromSlash(relReceipt))
	if err = os.MkdirAll(filepath.Dir(absReceipt), 0700); err != nil {
		return false, "création du reçu impossible : " + err.Error()
	}
	record := AutomaticValidation{Attempt: a.Attempt, Producer: a.ID, Controller: validationController, PolicyDigest: digest, Policy: policy,
		Artifacts: map[string]string{report: hash(reportBytes)}, Controls: results,
		Receipt: relReceipt, State: "blocked", At: now()}
	if passed {
		record.State, record.Reason = "accepted", "tous les contrôles préautorisés ont réussi"
	} else {
		record.Reason = "au moins un contrôle préautorisé a échoué"
	}
	receiptBytes, _ := json.MarshalIndent(record, "", "  ")
	receiptBytes = append(receiptBytes, '\n')
	if err = atomicWrite(absReceipt, receiptBytes); err != nil {
		return false, "écriture du reçu impossible : " + err.Error()
	}
	record.Artifacts[relReceipt] = hash(receiptBytes)

	checks := []map[string]any{{"id": "deliverable", "domain": "automation", "mandatory": true, "gate": "delivery", "penalty": 100, "max_penalty": 100, "severity": "major"}}
	observed := []map[string]any{{"id": "deliverable", "status": "PASS", "count": 0, "evidence": []string{report}}}
	for _, result := range results {
		phase := "delivery"
		if planned := t.PlanChecks[result.ID]; planned != "" {
			phase = planned
		}
		checks = append(checks, map[string]any{"id": result.ID, "domain": "automation", "mandatory": true, "gate": phase, "penalty": 100, "max_penalty": 100, "severity": "major"})
		status, count := "PASS", 0
		if !result.Passed {
			status, count = "FAIL", 1
		}
		observed = append(observed, map[string]any{"id": result.ID, "status": status, "count": count, "evidence": []string{relReceipt}})
	}
	document, _ := json.Marshal(map[string]any{"method_version": "2", "scope_id": t.ID,
		"artifacts": record.Artifacts, "domains": map[string]int{"automation": 1},
		"checks": checks, "results": observed})
	evaluation, err := evaluate(document, s.root, "delivery")
	if err != nil {
		return false, "preuve devenue périmée avant décision : " + err.Error()
	}

	payload, _ := json.Marshal(record)
	_, err = s.mutateWithHook(w.ID, "task.auto-validation", newID("auto-validation-"), w.Revision, payload, func(current *Work) error {
		task, findErr := current.task(a.TaskID)
		if findErr != nil || task.Status != "submitted" || task.ValidationPolicy == nil || validationPolicyDigest(*task.ValidationPolicy) != digest {
			return fmt.Errorf("politique ou tâche modifiée pendant les contrôles ; revue humaine requise")
		}
		if e := s.independentReviewGuard(current, task); e != nil {
			return e
		}
		fresh, evalErr := evaluate(document, s.root, "delivery")
		if evalErr != nil {
			return fmt.Errorf("preuve périmée pendant les contrôles : %w", evalErr)
		}
		task.AutoValidation = &record
		task.Gate = &GateRecord{Name: "Validation automatique préautorisée", Document: document, Evaluation: fresh, At: now()}
		if !fresh.Allowed {
			task.Status = "blocked"
			task.Blocker = record.Reason
			task.Next = "Examiner le reçu " + relReceipt + " ; corriger ou modifier explicitement la politique avant une nouvelle tentative."
			return nil
		}
		for _, dep := range task.Depends {
			parent, _ := current.task(dep)
			if !s.acceptedFresh(current, parent, map[string]bool{}) {
				return fmt.Errorf("dépendance %s non validée ou preuve périmée", dep)
			}
		}
		task.Status = "accepted"
		task.Blocker = ""
		task.Next = "Acceptée automatiquement sur contrôles préautorisés ; reçu : " + relReceipt
		return nil
	}, func(tx *sql.Tx, _ *Work) error { return automaticValidationGuard(tx, w.ID) })
	if err != nil {
		return false, err.Error()
	}
	return evaluation.Allowed, record.Reason + " ; reçu " + relReceipt
}

// resumeAutomaticValidations makes pause/restart honest: a submitted result is
// reconsidered only from its persisted completed attempt and a still-provable
// handoff. It never reads a command from agent output or report text.
func (s *Store) resumeAutomaticValidations(work string) (bool, error) {
	w, err := s.get(work)
	if err != nil {
		return false, err
	}
	agents, err := s.agents(work)
	if err != nil {
		return false, err
	}
	for i := range w.Tasks {
		task := &w.Tasks[i]
		if task.Status != "submitted" || task.ValidationPolicy == nil || task.ValidationPolicy.Mode != "automatic" {
			continue
		}
		for _, agent := range agents {
			if agent.TaskID != task.ID || agent.Status != "completed" || agent.Attempt == "" {
				continue
			}
			report, _ := s.provenReport(task.ID, agent.Started)
			if report == "" {
				continue
			}
			accepted, reason := s.runAutomaticValidation(agent, report)
			kind := "validation-retained"
			if accepted {
				kind = "validation-accepted"
			}
			_ = s.log(agent.ID, "validation", reason)
			_ = s.controlEvent(work, kind, task.ID+" · tentative "+agent.Attempt+" : "+reason)
			return true, nil
		}
	}
	return false, nil
}
