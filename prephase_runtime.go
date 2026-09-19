//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

func (s *Store) spawnPreparationTurn(t PreparationTurn) error {
	// Replayed sends may spawn a supervisor, but only one SQL claim can invoke IA.
	if t.Status != "pending" {
		return nil
	}
	exe, e := os.Executable()
	if e != nil {
		return e
	}
	cmd := exec.Command(exe, "--root", s.root, "_prepare_turn", t.ID)
	cmd.Dir = s.root
	cmd.SysProcAttr = &syscall.SysProcAttr{Setsid: true}
	null, e := os.OpenFile(os.DevNull, os.O_RDWR, 0)
	if e != nil {
		return e
	}
	defer null.Close()
	cmd.Stdin = null
	cmd.Stdout = null
	cmd.Stderr = null
	if e = cmd.Start(); e != nil {
		_ = s.finishPreparationTurn(t, "", nil, "Supervision non démarrée.")
		return e
	}
	return cmd.Process.Release()
}
func stopPreparationProcess(t PreparationTurn) {
	if t.PID > 0 && t.Host == hostIdentity() && t.ProcessStamp != "" && processStamp(t.PID) == t.ProcessStamp {
		_ = syscall.Kill(-t.PID, syscall.SIGKILL)
	}
}
func (s *Store) runPreparationTurn(t PreparationTurn) {
	s.runPreparationTurnWithin(t, time.Duration(t.TimeoutSeconds)*time.Second)
}
func (s *Store) runPreparationTurnWithin(t PreparationTurn, deadline time.Duration) {
	if deadline <= 0 || deadline > 120*time.Second {
		deadline = 120 * time.Second
	}
	t.Status = "running"
	t.SupervisorPID = os.Getpid()
	t.SupervisorStamp = processStamp(os.Getpid())
	t.Host = hostIdentity()
	raw, _ := json.Marshal(t)
	result, e := s.db.Exec("UPDATE preparation_turns SET status='running',body=? WHERE id=? AND status='pending'", raw, t.ID)
	if e != nil {
		return
	}
	n, _ := result.RowsAffected()
	if n != 1 {
		return
	}
	fail := func(msg string) { _ = s.finishPreparationTurn(t, "", nil, msg) }
	ps, e := s.providers()
	if e != nil {
		fail("Configuration du fournisseur indisponible.")
		return
	}
	p, ok := ps.Providers[t.Provider]
	b, _ := json.Marshal(p)
	if !ok || hash(b) != t.ProviderDigest {
		fail("Fournisseur modifié depuis l’envoi ; aucun appel lancé.")
		return
	}
	p, e = assistantProvider(p)
	if e != nil {
		fail(e.Error())
		return
	}
	// Apply the persisted route after stripping tool/permission arguments.
	if t.ModelRoute != nil {
		p = applyModelRoute(p, t.ModelRoute)
	} else {
		p, _, e = resolveModel(p, "auto", "preparation")
		if e != nil {
			fail(e.Error())
			return
		}
	}
	dir, e := os.MkdirTemp("", "swarm-preparation-")
	if e != nil {
		fail("Répertoire isolé indisponible.")
		return
	}
	defer os.RemoveAll(dir)
	schema := `{"type":"object","additionalProperties":false,"required":["message","brief"],"properties":{"message":{"type":"string"},"brief":{"type":"string"}}}`
	if t.Target == "plan" {
		schema = strings.ReplaceAll(schema, `"brief"`, `"plan"`)
	}
	if filepath.Base(p.Command) == "codex" {
		schemaPath := filepath.Join(dir, "reply-schema.json")
		if e = os.WriteFile(schemaPath, []byte(schema), 0600); e != nil {
			fail("Contrat de réponse indisponible.")
			return
		}
		p.Args = append(p.Args[:len(p.Args)-1], "--output-schema", schemaPath, "-")
	} else {
		p.Args = append(p.Args, "--json-schema", schema)
	}
	cmd := exec.Command(p.Command, p.Args...)
	cmd.Dir = dir
	cmd.Env = providerEnvironment(p.Env)
	cmd.Stdin = strings.NewReader(t.Prompt)
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	cmd.WaitDelay = 2 * time.Second
	reader, writer := io.Pipe()
	cmd.Stdout = writer
	diagnostic := &assistDiagnostic{}
	cmd.Stderr = diagnostic
	output := make(chan assistOutput, 1)
	go func() { output <- readAssistOutput(reader) }()
	// Cancellation may win between the claim and process creation.
	current, e := s.preparationTurn(t.ID)
	if e != nil || current.Status != "running" {
		writer.Close()
		<-output
		reader.Close()
		fail("Envoi interrompu avant l’appel IA.")
		return
	}
	// Record possible expenditure before starting: a crash cannot release a possibly billed call.
	if e = s.markPreparationAttempt(t.ID); e != nil {
		writer.Close()
		<-output
		reader.Close()
		fail("Envoi interrompu avant l’appel IA.")
		return
	}
	if e = cmd.Start(); e != nil {
		writer.Close()
		<-output
		reader.Close()
		fail("Fournisseur non démarré : " + guardBlock(e.Error(), 300))
		return
	}
	killGroup := func() { _ = syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL) }
	t.PID = cmd.Process.Pid
	t.ProcessStamp = processStamp(t.PID)
	raw, _ = json.Marshal(t)
	result, e = s.db.Exec("UPDATE preparation_turns SET body=? WHERE id=? AND status='running'", raw, t.ID)
	if e != nil {
		killGroup()
	} else {
		n, _ = result.RowsAffected()
		if n != 1 {
			killGroup()
		}
	}
	done := make(chan error, 1)
	go func() { done <- cmd.Wait(); writer.Close() }()
	timer := time.NewTimer(deadline)
	defer timer.Stop()
	poll := time.NewTicker(200 * time.Millisecond)
	defer poll.Stop()
	failure := ""
	var waitErr error
	waiting := true
	for waiting {
		select {
		case waitErr = <-done:
			waiting = false
		case <-timer.C:
			failure = fmt.Sprintf("Délai de %.0f secondes dépassé ; aucune relance automatique.", deadline.Seconds())
			killGroup()
			waitErr = <-done
			waiting = false
		case <-poll.C:
			current, err := s.preparationTurn(t.ID)
			if err != nil || current.Status != "running" {
				failure = "Échange interrompu."
				killGroup()
				waitErr = <-done
				waiting = false
			}
		}
	}
	killGroup()
	out := <-output
	reader.Close()
	if failure == "" && out.err != nil {
		failure = "Réponse du fournisseur illisible ou trop volumineuse."
	}
	if failure == "" && waitErr != nil {
		failure = "Le fournisseur s’est arrêté en erreur : " + guardBlock(diagnostic.String(), 500)
	}
	_ = s.finishPreparationTurn(t, out.reply, out.usage, failure)
}
