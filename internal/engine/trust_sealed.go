//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"time"
)

type TrustSealedPolicy struct {
	Enabled        trustOIDCEnabled `json:"enabled"`
	TimeoutSeconds int              `json:"timeout_seconds"`
}

func (p *TrustSealedPolicy) UnmarshalJSON(raw []byte) error {
	var fields map[string]json.RawMessage
	if err := json.Unmarshal(raw, &fields); err != nil {
		return err
	}
	for _, key := range []string{"enabled", "timeout_seconds"} {
		v, ok := fields[key]
		if !ok || bytes.Equal(bytes.TrimSpace(v), []byte("null")) {
			return fmt.Errorf("exécution scellée : champs explicites non null requis")
		}
	}
	type plain TrustSealedPolicy
	var v plain
	if err := strict(raw, &v); err != nil {
		return err
	}
	*p = TrustSealedPolicy(v)
	return nil
}
func defaultTrustSealedPolicy() TrustSealedPolicy { return TrustSealedPolicy{TimeoutSeconds: 15} }
func trustSealedPolicyValue(p *TrustSealedPolicy) TrustSealedPolicy {
	if p == nil {
		return defaultTrustSealedPolicy()
	}
	return *p
}
func validateTrustSealedPolicy(p *TrustSealedPolicy) error {
	if p != nil && (p.TimeoutSeconds <= 0 || int64(p.TimeoutSeconds) > int64((1<<63-1)/int64(time.Second))) {
		return fmt.Errorf("exécution scellée : durée en secondes strictement positive et représentable requise ; 0 refusé")
	}
	return nil
}
func trustSealedDenied() error {
	return &CommandError{Code: "authorization_required", Message: "exécution scellée désactivée ; activez sa politique dans Admin"}
}
func trustSealedTimeout() error {
	return &CommandError{Code: "containment_timeout", Message: "vérification scellée interrompue ou délai configuré dépassé"}
}
func trustSealedFailure() error {
	return &CommandError{Code: "containment_unavailable", Message: "vérification du processus scellé refusée ; frontière non démontrée"}
}

var trustSealedCheckNames = []string{"work_read", "work_write", "outside_denied", "traversal_denied", "symlink_denied", "proc_denied", "binary_denied", "tcp_denied", "udp_denied", "unix_denied", "foreign_fd_closed", "minimal_environment", "no_new_privs", "not_dumpable"}

type trustSealedResult struct {
	ABI    int             `json:"landlock_abi"`
	Checks map[string]bool `json:"checks"`
}
type trustSealedInput struct {
	Work    string `json:"work"`
	Private string `json:"private"`
}

func (s *Store) trustSealedCheck(ctx context.Context, raw []byte) (any, error) {
	return s.trustSealedCheckUsing(ctx, raw, func(ctx context.Context, p TrustSealedPolicy) (trustSealedResult, error) {
		exe, err := os.Executable()
		if err != nil {
			return trustSealedResult{}, trustSealedFailure()
		}
		return runTrustSealedProbe(ctx, p, []string{exe, "_trust_sealed"})
	})
}
func (s *Store) trustSealedCheckUsing(ctx context.Context, raw []byte, runner func(context.Context, TrustSealedPolicy) (trustSealedResult, error)) (any, error) {
	if ctx.Err() != nil {
		return nil, trustSealedTimeout()
	}
	var request map[string]json.RawMessage
	if err := strict(raw, &request); err != nil {
		return nil, err
	}
	if request == nil || len(request) != 0 {
		return nil, fmt.Errorf("requête de vérification vide requise")
	}
	var policy TrustSealedPolicy
	var revision int
	_, err := s.withTrustReadAvailable(func(st *trustState) (any, error) {
		policy = trustSealedPolicyValue(st.SealedExecution)
		revision = st.Revision
		if !bool(policy.Enabled) {
			return nil, trustSealedDenied()
		}
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	result, err := runner(ctx, policy)
	if err != nil {
		return nil, err
	}
	if ctx.Err() != nil {
		return nil, trustSealedTimeout()
	}
	_, err = s.withTrustReadAvailable(func(st *trustState) (any, error) {
		if st.Revision != revision || trustSealedPolicyValue(st.SealedExecution) != policy {
			return nil, &CommandError{Code: "revision_conflict", Message: "configuration modifiée ; recommencez la vérification"}
		}
		return nil, nil
	})
	if err != nil {
		return nil, err
	}
	return map[string]any{"timeout_seconds": policy.TimeoutSeconds, "schema_version": 1, "scope": "sealed_helper_only", "revision": revision, "landlock_abi": result.ABI, "checks": result.Checks, "passed": true, "provider_containment": false, "secret_grants_supported": false}, nil
}
func runTrustSealedProbe(ctx context.Context, p TrustSealedPolicy, argv []string) (trustSealedResult, error) {
	var result trustSealedResult
	if !bool(p.Enabled) {
		return result, trustSealedDenied()
	}
	if err := validateTrustSealedPolicy(&p); err != nil {
		return result, err
	}
	root, err := os.MkdirTemp("", "swarm-sealed-check-")
	if err != nil {
		return result, trustSealedFailure()
	}
	defer os.RemoveAll(root)
	work := filepath.Join(root, "work")
	privateDir := filepath.Join(root, "private")
	private := filepath.Join(privateDir, "token")
	for _, dir := range []string{work, privateDir} {
		if err = os.Mkdir(dir, 0700); err != nil {
			return result, trustSealedFailure()
		}
	}
	if os.WriteFile(filepath.Join(work, "allowed"), []byte("allowed"), 0600) != nil || os.WriteFile(private, []byte("fixture-private-value"), 0600) != nil || os.Symlink(private, filepath.Join(work, "linked")) != nil {
		return result, trustSealedFailure()
	}
	foreign, err := os.Open(private)
	if err != nil {
		return result, trustSealedFailure()
	}
	defer foreign.Close()
	raw, _ := json.Marshal(trustSealedInput{Work: work, Private: private})
	childCtx, cancel := context.WithTimeout(ctx, time.Duration(p.TimeoutSeconds)*time.Second)
	defer cancel()
	cmd := exec.CommandContext(childCtx, argv[0], argv[1:]...)
	cmd.Env = []string{"LANG=C", "TZ=UTC"}
	cmd.Dir = work
	cmd.Stdin = bytes.NewReader(raw)
	cmd.ExtraFiles = []*os.File{foreign}
	image, err := pinTrustFixedHelperImage(cmd)
	if err != nil {
		return result, trustSealedFailure()
	}
	defer image.Close()
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return os.ErrProcessDone
		}
		err := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		if errors.Is(err, syscall.ESRCH) {
			return os.ErrProcessDone
		}
		return err
	}
	if err = cmd.Run(); err != nil {
		if childCtx.Err() != nil {
			return result, trustSealedTimeout()
		}
		return result, trustSealedFailure()
	}
	if stderr.Len() != 0 || strict(stdout.Bytes(), &result) != nil || result.ABI < 6 || len(result.Checks) != len(trustSealedCheckNames) {
		return trustSealedResult{}, trustSealedFailure()
	}
	for _, name := range trustSealedCheckNames {
		if !result.Checks[name] {
			return trustSealedResult{}, trustSealedFailure()
		}
	}
	return result, nil
}
func runTrustSealedChild(in io.Reader, out io.Writer) int {
	raw, err := io.ReadAll(io.LimitReader(in, (1<<20)+1))
	if err != nil || len(raw) > 1<<20 {
		return 4
	}
	var input trustSealedInput
	if strict(raw, &input) != nil || input.Work == "" || input.Private == "" {
		return 4
	}
	result, err := trustSealedChildProbe(input)
	if err != nil {
		return 4
	}
	if printJSON(out, result) != nil {
		return 4
	}
	return 0
}
