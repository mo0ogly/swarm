//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func setPreflightProvider(t *testing.T, s *Store, p Provider) {
	t.Helper()
	raw, err := json.Marshal(Providers{Schema: 1, Providers: map[string]Provider{"fixture": p}})
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(filepath.Join(s.root, ".swarm", "providers.json"), raw, 0600); err != nil {
		t.Fatal(err)
	}
}

func verifiedPreflightProvider(command string, args ...string) Provider {
	return Provider{Command: command, PreflightRequired: true, PreflightArgs: args, PreflightKind: "provider-context", PreflightCaps: append([]string{}, requiredProviderCapabilities...)}
}

const verifiedPreflightJSON = `{"schema_version":1,"capabilities":{"process":"verified","workspace_read":"verified","workspace_write":"verified"}}`

func successfulShellPreflight() Provider {
	return verifiedPreflightProvider("/bin/sh", "-c", "printf '%s\\n' '"+verifiedPreflightJSON+"'")
}

func TestPreflightFailureConsumesNoAttemptOrBudget(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	marker := filepath.Join(s.root, "model-called")
	p := verifiedPreflightProvider("/bin/sh", "-c", "exit 7")
	p.Args = []string{"-c", "touch " + marker}
	setPreflightProvider(t, s, p)
	r := Launch{Schema: 1, EventID: "preflight-failure", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	if _, created, err := s.prepare(w.ID, r); err == nil || created {
		t.Fatalf("prévol en échec accepté : created=%t err=%v", created, err)
	} else {
		var failure *PreflightError
		if !errors.As(err, &failure) || failure.Result.Verdict != "intervention" {
			t.Fatalf("verdict inattendu : %T %v", err, err)
		}
	}
	after, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	task, _ := after.task("t1")
	if after.Revision != w.Revision || task.Status != "todo" || len(task.Attempts) != 0 {
		t.Fatalf("mutation métier malgré le prévol : revision=%d task=%+v", after.Revision, task)
	}
	var agents, reservations int
	if err = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents); err != nil {
		t.Fatal(err)
	}
	if err = s.db.QueryRow("SELECT count(*) FROM reservations").Scan(&reservations); err != nil {
		t.Fatal(err)
	}
	if agents != 0 || reservations != 0 {
		t.Fatalf("intention ou budget réservé : agents=%d reservations=%d", agents, reservations)
	}
	if _, err = os.Stat(marker); !os.IsNotExist(err) {
		t.Fatalf("les arguments métier ont été exécutés : %v", err)
	}
}

func TestPreflightReadyLeavesNoTemporaryFileAndExpires(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	p := successfulShellPreflight()
	p.PreflightTTL = 1
	setPreflightProvider(t, s, p)
	result, err := s.preflightLaunch(w.ID, Launch{Provider: "fixture", Workspace: "."})
	if err != nil || result.Verdict != "ready" || len(result.Checks) != 5 {
		t.Fatalf("prévol inattendu : %+v err=%v", result, err)
	}
	matches, err := filepath.Glob(filepath.Join(s.root, ".swarm-preflight-*"))
	if err != nil || len(matches) != 0 {
		t.Fatalf("mutation temporaire restante : %v err=%v", matches, err)
	}
	until, err := time.Parse(time.RFC3339Nano, result.ValidUntil)
	if err != nil {
		t.Fatal(err)
	}
	if !result.Fresh(until.Add(-time.Nanosecond), result.Scope) || result.Fresh(until, result.Scope) || result.Fresh(until.Add(-time.Second), "autre") {
		t.Fatal("contrat de fraîcheur non respecté")
	}
	p.PreflightArgs = []string{"-c", "printf '%s\\n' '" + verifiedPreflightJSON + "' # changed"}
	setPreflightProvider(t, s, p)
	changed, err := s.preflightLaunch(w.ID, Launch{Provider: "fixture", Workspace: "."})
	if err != nil || changed.Scope == result.Scope {
		t.Fatalf("changement de configuration non invalidant : %+v err=%v", changed, err)
	}
}

func TestPreflightTimeoutIsBoundedWaiting(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	p := verifiedPreflightProvider("/bin/sh", "-c", "sleep 5")
	p.PreflightTimeout = 1
	setPreflightProvider(t, s, p)
	started := time.Now()
	result, err := s.preflightLaunch(w.ID, Launch{Provider: "fixture", Workspace: "."})
	if err == nil || result.Verdict != "waiting" {
		t.Fatalf("délai non classé en attente : %+v err=%v", result, err)
	}
	if elapsed := time.Since(started); elapsed < 900*time.Millisecond || elapsed > 2*time.Second {
		t.Fatalf("prévol non borné : %s", elapsed)
	}
}

func TestPreflightCLIAndWebShareVerdict(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	setPreflightProvider(t, s, successfulShellPreflight())
	requestPath := filepath.Join(t.TempDir(), "preflight.json")
	request, _ := json.Marshal(Launch{Provider: "fixture", Workspace: "."})
	if err := os.WriteFile(requestPath, request, 0600); err != nil {
		t.Fatal(err)
	}
	var cli bytes.Buffer
	if err := agentCLI(s, []string{"agent", "preflight", w.ID}, requestPath, "", true, &cli); err != nil {
		t.Fatal(err)
	}
	var cliResult PreflightResult
	if err := json.Unmarshal(cli.Bytes(), &cliResult); err != nil {
		t.Fatal(err)
	}
	value, err := s.webAction(webRequest{Kind: "preflight", Work: w.ID, Provider: "fixture", Workspace: "."})
	if err != nil {
		t.Fatal(err)
	}
	webResult := value.(PreflightResult)
	if cliResult.Verdict != webResult.Verdict || cliResult.Scope != webResult.Scope || len(cliResult.Checks) != len(webResult.Checks) {
		t.Fatalf("divergence CLI/web : cli=%+v web=%+v", cliResult, webResult)
	}
}

func TestPreflightCLIAndWebShareUnverifiedVerdict(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	setPreflightProvider(t, s, Provider{Command: "/bin/sh", PreflightArgs: []string{"-c", "exit 0"}})
	requestPath := filepath.Join(t.TempDir(), "preflight-unverified.json")
	request, _ := json.Marshal(Launch{Provider: "fixture", Workspace: "."})
	if err := os.WriteFile(requestPath, request, 0600); err != nil {
		t.Fatal(err)
	}
	var cli bytes.Buffer
	if err := agentCLI(s, []string{"agent", "preflight", w.ID}, requestPath, "", true, &cli); err != nil {
		t.Fatal(err)
	}
	var cliResult PreflightResult
	if err := json.Unmarshal(cli.Bytes(), &cliResult); err != nil {
		t.Fatal(err)
	}
	value, err := s.webAction(webRequest{Kind: "preflight", Work: w.ID, Provider: "fixture", Workspace: "."})
	if err != nil {
		t.Fatal(err)
	}
	webResult := value.(PreflightResult)
	if cliResult.Verdict != "compatible" || cliResult.Verification != "unverified" || cliResult.Verdict != webResult.Verdict || cliResult.Verification != webResult.Verification {
		t.Fatalf("divergence CLI/web sur limite fournisseur : cli=%+v web=%+v", cliResult, webResult)
	}
}

func TestPreflightCLIAndWebShareRequiredRefusal(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)
	setPreflightProvider(t, s, Provider{Command: "/bin/sh", PreflightRequired: true})
	requestPath := filepath.Join(t.TempDir(), "preflight-required.json")
	request, _ := json.Marshal(Launch{Provider: "fixture", Workspace: "."})
	if err := os.WriteFile(requestPath, request, 0600); err != nil {
		t.Fatal(err)
	}
	var cli bytes.Buffer
	if err := agentCLI(s, []string{"agent", "preflight", w.ID}, requestPath, "", true, &cli); err != nil {
		t.Fatal(err)
	}
	var cliResult PreflightResult
	if err := json.Unmarshal(cli.Bytes(), &cliResult); err != nil {
		t.Fatal(err)
	}
	value, err := s.webAction(webRequest{Kind: "preflight", Work: w.ID, Provider: "fixture", Workspace: "."})
	if err != nil {
		t.Fatal(err)
	}
	webResult := value.(PreflightResult)
	if cliResult.Verdict != "intervention" || cliResult.Verification != "unverified" || cliResult.Verdict != webResult.Verdict || cliResult.Verification != webResult.Verification {
		t.Fatalf("divergence CLI/web sur politique requise : cli=%+v web=%+v", cliResult, webResult)
	}
}

func TestSuccessfulLaunchPersistsAtomicPreflight(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	p := successfulShellPreflight()
	p.Args = []string{"-c", "exit 0"}
	setPreflightProvider(t, s, p)
	r := Launch{Schema: 1, EventID: "preflight-ready", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	agent, created, err := s.prepare(w.ID, r)
	if err != nil || !created || agent.Preflight == nil || agent.Preflight.Verdict != "ready" {
		t.Fatalf("départ prêt non enregistré : created=%t agent=%+v err=%v", created, agent, err)
	}
	stored, err := s.agent(agent.ID)
	if err != nil || stored.Preflight == nil || stored.Preflight.Scope != agent.Preflight.Scope {
		t.Fatalf("reçu de prévol absent : %+v err=%v", stored.Preflight, err)
	}
}

func TestReceiptExpiringDuringProbeIsRejectedBeforeReservation(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	p := verifiedPreflightProvider("/bin/sh", "-c", "sleep 1.1; printf '%s\\n' '"+verifiedPreflightJSON+"'")
	p.PreflightTimeout = 2
	p.PreflightTTL = 1
	p.Args = []string{"-c", "exit 0"}
	setPreflightProvider(t, s, p)
	r := Launch{Schema: 1, EventID: "expired-receipt", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	_, created, err := s.prepare(w.ID, r)
	if err == nil || created || !strings.Contains(err.Error(), "périmé") {
		t.Fatalf("reçu expiré accepté : created=%t err=%v", created, err)
	}
	var agents, reservations int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents)
	_ = s.db.QueryRow("SELECT count(*) FROM reservations").Scan(&reservations)
	if agents != 0 || reservations != 0 {
		t.Fatalf("réservation avec reçu expiré : agents=%d reservations=%d", agents, reservations)
	}
}

func TestLegacyProfileLaunchesWithoutInventedReceipt(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	// Profil historique : --version réussit, mais aucun adaptateur n'atteste
	// l'exécution et les accès dans le sandbox réel du fournisseur.
	setPreflightProvider(t, s, Provider{Command: "/bin/sh", Args: []string{"-c", "exit 0"}, PreflightArgs: []string{"-c", "exit 0"}})
	r := Launch{Schema: 1, EventID: "legacy-version", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	agent, created, err := s.prepare(w.ID, r)
	if err != nil || !created {
		t.Fatalf("profil historique bloqué : created=%t err=%v", created, err)
	}
	if agent.Preflight == nil || agent.Preflight.Verdict != "compatible" || agent.Preflight.Verification != "unverified" || agent.Preflight.Capabilities["workspace_write"] != "unverified" {
		t.Fatalf("limite fournisseur masquée : %+v", agent.Preflight)
	}
	stored, err := s.agent(agent.ID)
	if err != nil || stored.Preflight == nil || stored.Preflight.Verdict != "compatible" {
		t.Fatalf("prévol compatible non conservé : %+v err=%v", stored.Preflight, err)
	}
	if agent.Preflight.Verdict == "ready" || agent.Preflight.Verification == "verified" {
		t.Fatalf("compatibilité annoncée comme vérifiée : %+v", agent.Preflight)
	}
}

func TestRequiredPreflightRejectsUnknownCapabilitiesBeforeReservation(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	setPreflightProvider(t, s, Provider{Command: "/bin/sh", Args: []string{"-c", "exit 0"}, PreflightRequired: true})
	r := Launch{Schema: 1, EventID: "required-unknown", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	_, created, err := s.prepare(w.ID, r)
	if err == nil || created || !strings.Contains(err.Error(), "précontrôle requis") {
		t.Fatalf("capacité inconnue acceptée sous politique requise : created=%t err=%v", created, err)
	}
	var agents, reservations int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents)
	_ = s.db.QueryRow("SELECT count(*) FROM reservations").Scan(&reservations)
	if agents != 0 || reservations != 0 {
		t.Fatalf("réservation avant verdict requis : agents=%d reservations=%d", agents, reservations)
	}
}

func TestConfiguredProbeFailureIsNotBypassedInCompatibleMode(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	setPreflightProvider(t, s, Provider{Command: "/bin/sh", Args: []string{"-c", "exit 0"}, PreflightArgs: []string{"-c", "exit 9"}})
	r := Launch{Schema: 1, EventID: "legacy-probe-failed", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	_, created, err := s.prepare(w.ID, r)
	if err == nil || created || !strings.Contains(err.Error(), "code 9") {
		t.Fatalf("sonde configurée contournée : created=%t err=%v", created, err)
	}
	var agents int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents)
	if agents != 0 {
		t.Fatalf("tentative réservée après sonde en échec : %d", agents)
	}
}

func TestPreflightScopeIncludesEffectiveArgumentsModeAndEnvironment(t *testing.T) {
	t.Setenv("SWARM_PREFLIGHT_SCOPE", "one")
	p := successfulShellPreflight()
	p.Args = []string{"exec", "--sandbox", "workspace-write"}
	p.Env = []string{"SWARM_PREFLIGHT_SCOPE"}
	base := Launch{Mode: ""}
	scope := preflightScope("fixture", p, "/tmp/work", base)
	p.Args[len(p.Args)-1] = "read-only"
	if scope == preflightScope("fixture", p, "/tmp/work", base) {
		t.Fatal("arguments effectifs absents de la portée")
	}
	p.Args[len(p.Args)-1] = "workspace-write"
	if scope == preflightScope("fixture", p, "/tmp/work", Launch{Mode: "dialogue"}) {
		t.Fatal("mode absent de la portée")
	}
	t.Setenv("SWARM_PREFLIGHT_SCOPE", "two")
	if scope == preflightScope("fixture", p, "/tmp/work", base) {
		t.Fatal("contexte d’environnement absent de la portée")
	}
}

func TestSlowPreflightDoesNotHoldLaunchTransaction(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	marker := filepath.Join(s.root, "probe-started")
	p := verifiedPreflightProvider("/bin/sh", "-c", "touch "+marker+"; sleep 1; printf '%s\\n' '"+verifiedPreflightJSON+"'")
	p.Args = []string{"-c", "exit 0"}
	setPreflightProvider(t, s, p)
	r := Launch{Schema: 1, EventID: "slow-probe", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	done := make(chan error, 1)
	go func() { _, _, err := s.prepare(w.ID, r); done <- err }()
	deadline := time.Now().Add(500 * time.Millisecond)
	for {
		if _, err := os.Stat(marker); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("la sonde lente n’a pas démarré")
		}
		time.Sleep(10 * time.Millisecond)
	}
	started := time.Now()
	w = applyTest(t, s, w, "checkpoint", Request{Summary: "mutation concurrente", Next: "relire"})
	if elapsed := time.Since(started); elapsed > 500*time.Millisecond {
		t.Fatalf("transaction bloquée pendant la sonde : %s", elapsed)
	}
	if err := <-done; err == nil || !strings.Contains(err.Error(), "révision périmée") {
		t.Fatalf("départ non invalidé par la mutation concurrente : %v", err)
	}
}

func TestProviderContextChangedAfterProbeIsRejected(t *testing.T) {
	s := storeTest(t)
	w := taskTest(t, s, createTest(t, s))
	marker := filepath.Join(s.root, "probe-context-started")
	p := verifiedPreflightProvider("/bin/sh", "-c", "touch "+marker+"; sleep 1; printf '%s\\n' '"+verifiedPreflightJSON+"'")
	p.Args = []string{"-c", "exit 0"}
	setPreflightProvider(t, s, p)
	r := Launch{Schema: 1, EventID: "changed-context", Revision: w.Revision, TaskID: "t1", Provider: "fixture", Workspace: "."}
	done := make(chan error, 1)
	go func() { _, _, err := s.prepare(w.ID, r); done <- err }()
	deadline := time.Now().Add(500 * time.Millisecond)
	for {
		if _, err := os.Stat(marker); err == nil {
			break
		}
		if time.Now().After(deadline) {
			t.Fatal("la sonde n’a pas démarré")
		}
		time.Sleep(10 * time.Millisecond)
	}
	p.Args = []string{"-c", "exit 9"}
	setPreflightProvider(t, s, p)
	if err := <-done; err == nil || !strings.Contains(err.Error(), "précontrôle périmé") {
		t.Fatalf("contexte modifié accepté : %v", err)
	}
	var agents int
	_ = s.db.QueryRow("SELECT count(*) FROM agents").Scan(&agents)
	if agents != 0 {
		t.Fatalf("tentative réservée après changement de contexte : %d", agents)
	}
}

func TestPreflightScopeIncludesInheritedEnvironmentAndProbeLimits(t *testing.T) {
	p := Provider{Command: "/bin/sh"}
	cwd := t.TempDir()
	r := Launch{}
	t.Setenv("XDG_CACHE_HOME", cwd+"/first")
	before := preflightScope("fixture", p, cwd, r)
	t.Setenv("XDG_CACHE_HOME", cwd+"/second")
	if before == preflightScope("fixture", p, cwd, r) {
		t.Fatal("environnement implicite ignoré")
	}
	before = preflightScope("fixture", p, cwd, r)
	p.PreflightTTL = 1
	if before == preflightScope("fixture", p, cwd, r) {
		t.Fatal("TTL modifié ignoré")
	}
	before = preflightScope("fixture", p, cwd, r)
	p.PreflightTimeout = 1
	if before == preflightScope("fixture", p, cwd, r) {
		t.Fatal("délai modifié ignoré")
	}
}
