//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"syscall"
)

type laboratoryChildInput struct {
	Probe trustSealedInput     `json:"probe"`
	Input TrustLaboratoryInput `json:"input"`
}

func runLaboratoryChild(in io.Reader, out io.Writer) int {
	raw, err := io.ReadAll(io.LimitReader(in, 1<<20))
	var c laboratoryChildInput
	if err != nil || strict(raw, &c) != nil || !laboratoryInputValid(&c.Input) || !laboratoryCleanWork(c.Probe.Work) {
		return 4
	}
	boundary, err := trustSealedChildProbe(c.Probe)
	if err != nil {
		return 4
	}
	for _, name := range trustSealedCheckNames {
		if !boundary.Checks[name] {
			return 4
		}
	}
	cases, err := laboratoryExecute(c.Probe.Work, c.Input)
	if err != nil {
		return 4
	}
	result := TrustLaboratoryResult{Input: c.Input, Cases: cases, Containment: boundary, Simulation: true}
	result.DeterministicDigest = laboratoryResultDigest(result)
	if json.NewEncoder(out).Encode(result) != nil {
		return 4
	}
	return 0
}

func runLaboratoryProcess(ctx context.Context, input TrustLaboratoryInput, argv []string) (TrustLaboratoryResult, error) {
	runtime.LockOSThread()
	defer runtime.UnlockOSThread()
	var result TrustLaboratoryResult
	root, err := os.MkdirTemp("", "swarm-laboratory-")
	if err != nil {
		return result, err
	}
	defer os.RemoveAll(root)
	work, privateDir := filepath.Join(root, "work"), filepath.Join(root, "private")
	private := filepath.Join(privateDir, "token")
	if os.Mkdir(work, 0700) != nil || os.Mkdir(privateDir, 0700) != nil || os.WriteFile(filepath.Join(work, "allowed"), []byte("allowed"), 0600) != nil || os.WriteFile(private, []byte("fixture-private-value"), 0600) != nil || os.Symlink(private, filepath.Join(work, "linked")) != nil {
		return result, laboratoryInvalid()
	}
	foreign, err := os.Open(private)
	if err != nil {
		return result, err
	}
	defer foreign.Close()
	if argv == nil {
		exe, e := os.Executable()
		if e != nil {
			return result, e
		}
		argv = []string{exe, "_trust_laboratory"}
	}
	cmd := exec.CommandContext(ctx, argv[0], argv[1:]...)
	cmd.Env = []string{"LANG=C", "TZ=UTC"}
	cmd.Dir = work
	cmd.ExtraFiles = []*os.File{foreign}
	image, err := pinTrustFixedHelperImage(cmd)
	if err != nil {
		return result, err
	}
	defer image.Close()
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true, Pdeathsig: syscall.SIGKILL}
	payload, _ := json.Marshal(laboratoryChildInput{Probe: trustSealedInput{Work: work, Private: private}, Input: input})
	cmd.Stdin = bytes.NewReader(payload)
	var stdout, stderr bytes.Buffer
	cmd.Stdout = &stdout
	cmd.Stderr = &stderr
	cmd.Cancel = func() error {
		if cmd.Process == nil {
			return os.ErrProcessDone
		}
		e := syscall.Kill(-cmd.Process.Pid, syscall.SIGKILL)
		if errors.Is(e, syscall.ESRCH) {
			return os.ErrProcessDone
		}
		return e
	}
	err = cmd.Run()
	if ctx.Err() != nil {
		return result, ctx.Err()
	}
	if err != nil || stderr.Len() != 0 || stdout.Len() > 1<<20 {
		return result, laboratoryInvalid()
	}
	return laboratoryPublicResult(stdout.Bytes(), input)
}
