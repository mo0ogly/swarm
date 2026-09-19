//go:build linux

package main

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

const (
	preflightTimeoutDefault = 3
	preflightTimeoutMax     = 10
	preflightTTLDefault     = 30
	preflightTTLMax         = 300
	preflightOutputLimit    = 4096
	preflightMinFreeBytes   = 1 << 20
)

type PreflightCheck struct {
	Name   string `json:"name"`
	Status string `json:"status"`
	Detail string `json:"detail"`
}

// PreflightResult is a short-lived observation, never an authorization.
// prepareLaunch rechecks its scope and freshness inside the launch transaction.
type PreflightResult struct {
	Schema       int               `json:"schema_version"`
	Verdict      string            `json:"verdict"`
	CheckedAt    string            `json:"checked_at"`
	ValidUntil   string            `json:"valid_until"`
	Scope        string            `json:"scope"`
	Provider     string            `json:"provider"`
	Workspace    string            `json:"workspace"`
	DurationMS   int64             `json:"duration_ms"`
	Verification string            `json:"verification"`
	Capabilities map[string]string `json:"capabilities"`
	Checks       []PreflightCheck  `json:"checks"`
}

type PreflightError struct{ Result PreflightResult }

func (e *PreflightError) Error() string {
	detail := "précontrôle non prêt"
	if len(e.Result.Checks) > 0 {
		detail = e.Result.Checks[len(e.Result.Checks)-1].Detail
	}
	return fmt.Sprintf("précontrôle %s : %s", e.Result.Verdict, detail)
}

func (r PreflightResult) Fresh(at time.Time, scope string) bool {
	until, err := time.Parse(time.RFC3339Nano, r.ValidUntil)
	return err == nil && r.Verdict == "ready" && r.Scope == scope && at.Before(until)
}

type cappedWriter struct {
	b strings.Builder
	n int
}

func (w *cappedWriter) Write(p []byte) (int, error) {
	n := len(p)
	if w.n < preflightOutputLimit {
		left := preflightOutputLimit - w.n
		if len(p) > left {
			p = p[:left]
		}
		_, _ = w.b.Write(p)
		w.n += len(p)
	}
	return n, nil
}

func preflightScope(provider string, p Provider, cwd string, launch Launch) string {
	envContext := map[string]string{}
	for _, entry := range providerEnvironment(p.Env) {
		name, value, _ := strings.Cut(entry, "=")
		envContext[name] = hash([]byte(value))
	}
	raw, _ := json.Marshal(struct {
		Provider      string            `json:"provider"`
		Command       string            `json:"command"`
		Args          []string          `json:"business_args"`
		ProbeArgs     []string          `json:"preflight_args"`
		ProbeKind     string            `json:"preflight_kind"`
		ProbeCaps     []string          `json:"preflight_capabilities"`
		ProbeTimeout  int               `json:"probe_timeout"`
		ProbeTTL      int               `json:"probe_ttl"`
		ProbeRequired bool              `json:"preflight_required"`
		EnvContext    map[string]string `json:"environment_context"`
		CWD           string            `json:"workspace"`
		Mode          string            `json:"mode"`
	}{provider, p.Command, p.Args, p.PreflightArgs, p.PreflightKind, p.PreflightCaps, p.PreflightTimeout, p.PreflightTTL, p.PreflightRequired, envContext, cwd, launch.Mode})
	return hash(raw)
}

var requiredProviderCapabilities = []string{"process", "workspace_read", "workspace_write"}

func preflightArgs(p Provider) []string { return append([]string{}, p.PreflightArgs...) }

func declaredProviderCapabilities(p Provider) map[string]bool {
	result := map[string]bool{}
	for _, capability := range p.PreflightCaps {
		result[capability] = true
	}
	return result
}

type providerPreflightReceipt struct {
	Schema       int               `json:"schema_version"`
	Capabilities map[string]string `json:"capabilities"`
}

func preflightLimits(p Provider) (time.Duration, time.Duration, error) {
	timeout := p.PreflightTimeout
	if timeout == 0 {
		timeout = preflightTimeoutDefault
	}
	ttl := p.PreflightTTL
	if ttl == 0 {
		ttl = preflightTTLDefault
	}
	if timeout < 1 || timeout > preflightTimeoutMax {
		return 0, 0, fmt.Errorf("preflight_timeout_seconds : 1 à %d", preflightTimeoutMax)
	}
	if ttl < 1 || ttl > preflightTTLMax {
		return 0, 0, fmt.Errorf("preflight_ttl_seconds : 1 à %d", preflightTTLMax)
	}
	return time.Duration(timeout) * time.Second, time.Duration(ttl) * time.Second, nil
}

func preflightFailure(result PreflightResult, verdict, name, detail string, started time.Time) (PreflightResult, error) {
	result.Verdict = verdict
	result.DurationMS = time.Since(started).Milliseconds()
	result.Checks = append(result.Checks, PreflightCheck{Name: name, Status: "failed", Detail: detail})
	return result, &PreflightError{Result: result}
}

// runProviderPreflight exécute seulement la sonde explicitement configurée.
// provider-context est le contrat d'un adaptateur déterministe opérant dans le
// même contexte que le fournisseur. operator-command reste observable mais ne
// démontre jamais, à lui seul, les capacités du sandbox et peut appeler un modèle.
func (s *Store) runProviderPreflight(provider string, p Provider, cwd string, launch Launch) (PreflightResult, error) {
	started := time.Now()
	timeout, ttl, err := preflightLimits(p)
	result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: started.UTC().Format(time.RFC3339Nano),
		Provider: provider, Workspace: cwd, Scope: preflightScope(provider, p, cwd, launch), Verification: "unverified", Capabilities: map[string]string{}}
	for _, capability := range requiredProviderCapabilities {
		result.Capabilities[capability] = "unverified"
	}
	if err != nil {
		return preflightFailure(result, "intervention", "configuration", err.Error(), started)
	}
	result.ValidUntil = started.Add(ttl).UTC().Format(time.RFC3339Nano)

	if !filepath.IsAbs(p.Command) {
		return preflightFailure(result, "intervention", "configuration", "chemin absolu requis pour l’exécutable", started)
	}
	info, err := os.Stat(p.Command)
	if err != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
		return preflightFailure(result, "intervention", "configuration", "exécutable fournisseur indisponible", started)
	}
	result.Checks = append(result.Checks, PreflightCheck{Name: "configuration", Status: "ready", Detail: "exécutable et limites valides"})
	if p.PreflightKind != "provider-context" && p.PreflightKind != "operator-command" && p.PreflightKind != "" {
		return preflightFailure(result, "intervention", "configuration", "preflight_kind doit valoir provider-context ou operator-command", started)
	}

	providerVerified := false
	if len(p.PreflightArgs) == 0 {
		if p.PreflightRequired {
			return preflightFailure(result, "intervention", "provider_context", "précontrôle requis : aucun adaptateur de contexte fournisseur configuré", started)
		}
		result.Checks = append(result.Checks, PreflightCheck{Name: "provider_context", Status: "unverified", Detail: "migration compatible : capacités du contexte fournisseur non vérifiées"})
	} else {
		ctx, cancel := context.WithTimeout(context.Background(), timeout)
		defer cancel()
		cmd := exec.CommandContext(ctx, p.Command, preflightArgs(p)...)
		cmd.Dir = cwd
		cmd.Env = providerEnvironment(p.Env)
		cmd.Stdin = strings.NewReader("")
		cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
		cmd.Cancel = func() error {
			if cmd.Process == nil {
				return nil
			}
			return syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		}
		cmd.WaitDelay = 250 * time.Millisecond
		output := &cappedWriter{}
		cmd.Stdout, cmd.Stderr = output, output
		err = cmd.Run()
		if ctx.Err() == context.DeadlineExceeded {
			return preflightFailure(result, "waiting", "command", fmt.Sprintf("commande minimale interrompue après %s", timeout), started)
		}
		if err != nil {
			var exitErr *exec.ExitError
			if errors.As(err, &exitErr) {
				return preflightFailure(result, "intervention", "command", fmt.Sprintf("commande minimale en échec (code %d)", exitErr.ExitCode()), started)
			}
			return preflightFailure(result, "intervention", "command", "commande minimale impossible : "+err.Error(), started)
		}
		result.Checks = append(result.Checks, PreflightCheck{Name: "command", Status: "ready", Detail: "sonde configurée terminée"})
		if p.PreflightKind != "provider-context" {
			if p.PreflightRequired {
				return preflightFailure(result, "intervention", "provider_context", "précontrôle requis : la commande configurée ne démontre pas le contexte fournisseur", started)
			}
			result.Checks = append(result.Checks, PreflightCheck{Name: "provider_context", Status: "unverified", Detail: "commande historique réussie ; capacités du contexte fournisseur non vérifiées"})
		} else {
			declared := declaredProviderCapabilities(p)
			var receipt providerPreflightReceipt
			if err = json.Unmarshal([]byte(strings.TrimSpace(output.b.String())), &receipt); err != nil || receipt.Schema != 1 {
				return preflightFailure(result, "intervention", "provider_context", "reçu structuré de l’adaptateur absent ou invalide", started)
			}
			for _, capability := range requiredProviderCapabilities {
				if !declared[capability] {
					return preflightFailure(result, "intervention", "provider_context", "adaptateur incomplet : capacité requise non démontrée : "+capability, started)
				}
				if receipt.Capabilities[capability] != "verified" {
					return preflightFailure(result, "intervention", "provider_context", "reçu adaptateur incomplet : capacité requise non vérifiée : "+capability, started)
				}
				result.Capabilities[capability] = "verified"
			}
			providerVerified = true
		}
	}

	dir, err := os.Open(cwd)
	if err != nil {
		return preflightFailure(result, "intervention", "read", "lecture du dossier impossible : "+err.Error(), started)
	}
	_, readErr := dir.Readdirnames(1)
	closeErr := dir.Close()
	if readErr != nil && readErr != io.EOF {
		return preflightFailure(result, "intervention", "read", "lecture du dossier impossible : "+readErr.Error(), started)
	}
	if closeErr != nil {
		return preflightFailure(result, "intervention", "read", "fermeture du dossier impossible : "+closeErr.Error(), started)
	}
	result.Checks = append(result.Checks, PreflightCheck{Name: "host_read", Status: "ready", Detail: "dossier lisible depuis l’hôte Swarm"})

	temp, err := os.CreateTemp(cwd, ".swarm-preflight-")
	if err != nil {
		return preflightFailure(result, "intervention", "write", "écriture isolée impossible : "+err.Error(), started)
	}
	tempName := temp.Name()
	cleanup := func() error { return os.Remove(tempName) }
	if _, err = temp.Write([]byte("swarm-preflight\n")); err == nil {
		err = temp.Sync()
	}
	if closeErr = temp.Close(); err == nil {
		err = closeErr
	}
	removeErr := cleanup()
	if err != nil {
		return preflightFailure(result, "intervention", "write", "écriture isolée impossible : "+err.Error(), started)
	}
	if removeErr != nil {
		return preflightFailure(result, "intervention", "write", "fichier temporaire non supprimé : "+removeErr.Error(), started)
	}
	result.Checks = append(result.Checks, PreflightCheck{Name: "host_write", Status: "ready", Detail: "écriture hôte temporaire créée, synchronisée et supprimée"})

	var stat syscall.Statfs_t
	if err = syscall.Statfs(cwd, &stat); err != nil {
		return preflightFailure(result, "intervention", "resources", "ressources du volume illisibles : "+err.Error(), started)
	}
	available := stat.Bavail * uint64(stat.Bsize)
	if available < preflightMinFreeBytes {
		return preflightFailure(result, "waiting", "resources", "moins de 1 Mio disponible sur le volume", started)
	}
	result.Checks = append(result.Checks, PreflightCheck{Name: "resources", Status: "ready", Detail: fmt.Sprintf("%d octets disponibles", available)})
	if providerVerified {
		result.Verdict = "ready"
		result.Verification = "verified"
	} else {
		result.Verdict = "compatible"
		result.Verification = "unverified"
	}
	result.DurationMS = time.Since(started).Milliseconds()
	return result, nil
}

func (s *Store) recheckPreflight(result PreflightResult, provider string, p Provider, cwd string, launch Launch) error {
	launchable := (result.Verdict == "ready" && result.Verification == "verified") ||
		(result.Verdict == "compatible" && result.Verification == "unverified" && !p.PreflightRequired)
	until, parseErr := time.Parse(time.RFC3339Nano, result.ValidUntil)
	if !launchable || parseErr != nil || result.Scope != preflightScope(provider, p, cwd, launch) || !time.Now().Before(until) {
		result.Verdict = "intervention"
		result.Verification = "expired"
		result.Checks = append(result.Checks, PreflightCheck{Name: "receipt", Status: "failed", Detail: "reçu de prévol périmé ou hors portée"})
		return &PreflightError{Result: result}
	}
	providers, err := s.providers()
	if err != nil {
		return err
	}
	configured, ok := providers.Providers[provider]
	if !ok {
		return fmt.Errorf("fournisseur non configuré")
	}
	if configured.APIConnectionID != "" {
		return fmt.Errorf("Connexion API sans outils : choisissez un exécutant installé pour cette tâche.")
	}
	if launch.Mode == "terminal" {
		configured, err = terminalProvider(configured)
		if err != nil {
			return err
		}
	}
	if launch.Mode == "dialogue" {
		if err = dialogueProvider(configured); err != nil {
			return err
		}
	}
	purpose := "work"
	if launch.Brainstorm {
		purpose = "brainstorm"
	}
	configured, _, err = resolveModel(configured, launch.Level, purpose)
	if err != nil {
		return err
	}
	if preflightScope(provider, configured, cwd, launch) != result.Scope {
		return fmt.Errorf("précontrôle périmé : configuration ou contexte modifié")
	}
	info, err := os.Stat(p.Command)
	if err != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
		return fmt.Errorf("précontrôle périmé : exécutable fournisseur indisponible")
	}
	var stat syscall.Statfs_t
	if err = syscall.Statfs(cwd, &stat); err != nil || stat.Bavail*uint64(stat.Bsize) < preflightMinFreeBytes {
		return fmt.Errorf("précontrôle périmé : ressource du workspace indisponible")
	}
	return nil
}

func (s *Store) preflightLaunch(work string, r Launch) (PreflightResult, error) {
	if _, err := s.get(work); err != nil {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: r.Workspace}
		return preflightFailure(result, "intervention", "work", err.Error(), time.Now())
	}
	if !safeName(r.Provider) {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: r.Workspace}
		return preflightFailure(result, "intervention", "configuration", "fournisseur invalide", time.Now())
	}
	if r.Workspace == "" {
		r.Workspace = "."
	}
	cwd, err := resolveWorkspace(s.root, r.Workspace)
	if err != nil {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: r.Workspace}
		return preflightFailure(result, "intervention", "workspace", err.Error(), time.Now())
	}
	providers, err := s.providers()
	if err != nil {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: cwd}
		return preflightFailure(result, "intervention", "configuration", err.Error(), time.Now())
	}
	p, ok := providers.Providers[r.Provider]
	if !ok {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: cwd}
		return preflightFailure(result, "intervention", "configuration", "fournisseur non configuré", time.Now())
	}
	if r.Mode == "terminal" {
		p, err = terminalProvider(p)
		if err != nil {
			result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: cwd}
			return preflightFailure(result, "intervention", "configuration", err.Error(), time.Now())
		}
	}
	if r.Mode == "dialogue" {
		if err = dialogueProvider(p); err != nil {
			result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: cwd}
			return preflightFailure(result, "intervention", "configuration", err.Error(), time.Now())
		}
	}
	purpose := "work"
	if r.Brainstorm {
		purpose = "brainstorm"
	}
	p, _, err = resolveModel(p, r.Level, purpose)
	if err != nil {
		result := PreflightResult{Schema: 1, Verdict: "intervention", CheckedAt: time.Now().UTC().Format(time.RFC3339Nano), Provider: r.Provider, Workspace: cwd}
		return preflightFailure(result, "intervention", "configuration", err.Error(), time.Now())
	}
	return s.runProviderPreflight(r.Provider, p, cwd, r)
}
