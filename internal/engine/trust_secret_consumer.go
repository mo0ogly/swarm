//go:build linux

package engine

import (
	"bufio"
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"syscall"
	"time"
)

type trustSecretChildInput struct {
	Probe trustSealedInput `json:"probe"`
	Grant TrustSecretGrant `json:"grant"`
	Size  int              `json:"size"`
}
type trustSecretReady struct {
	Ready  bool              `json:"ready"`
	Result trustSealedResult `json:"result"`
}

func trustSecretMessage(g TrustSecretGrant) []byte {
	legacy := struct {
		ID, Action, Digest, Principal, Connector, Destination string
		Revision                                              int
	}{g.ID, g.ActionID, g.Digest, g.Principal, g.Connector, g.Destination, g.Revision}
	if g.DataContext == nil {
		raw, _ := json.Marshal(legacy)
		return raw
	}
	raw, _ := json.Marshal(struct {
		Domain      string            `json:"domain"`
		Grant       any               `json:"grant"`
		DataContext *TrustDataContext `json:"data_context"`
	}{"swarm-secret-delivery-v1", legacy, g.DataContext})
	return raw
}

func runTrustSecretConsumer(ctx context.Context, p TrustSealedPolicy, value []byte, g TrustSecretGrant) (string, error) {
	if !bool(p.Enabled) || validateTrustSealedPolicy(&p) != nil || len(value) == 0 {
		return "", trustSecretDenied()
	}
	root, err := os.MkdirTemp("", "swarm-secret-consumer-")
	if err != nil {
		return "", trustSealedFailure()
	}
	defer os.RemoveAll(root)
	work, privateDir := filepath.Join(root, "work"), filepath.Join(root, "private")
	private := filepath.Join(privateDir, "token")
	if os.Mkdir(work, 0700) != nil || os.Mkdir(privateDir, 0700) != nil || os.WriteFile(filepath.Join(work, "allowed"), []byte("allowed"), 0600) != nil || os.WriteFile(private, []byte("fixture-private-value"), 0600) != nil || os.Symlink(private, filepath.Join(work, "linked")) != nil {
		return "", trustSealedFailure()
	}
	foreign, err := os.Open(private)
	if err != nil {
		return "", trustSealedFailure()
	}
	defer foreign.Close()
	exe, err := os.Executable()
	if err != nil {
		return "", trustSealedFailure()
	}
	childCtx, cancel := context.WithTimeout(ctx, time.Duration(p.TimeoutSeconds)*time.Second)
	defer cancel()
	cmd := exec.CommandContext(childCtx, exe, "_trust_secret")
	cmd.Env = []string{"LANG=C", "TZ=UTC"}
	cmd.Dir = work
	cmd.ExtraFiles = []*os.File{foreign}
	image, err := pinTrustFixedHelperImage(cmd)
	if err != nil {
		return "", trustSealedFailure()
	}
	defer image.Close()
	cmd.SysProcAttr = &syscall.SysProcAttr{Setpgid: true}
	in, err := cmd.StdinPipe()
	if err != nil {
		return "", trustSealedFailure()
	}
	defer in.Close()
	out, err := cmd.StdoutPipe()
	if err != nil {
		return "", trustSealedFailure()
	}
	defer out.Close()
	var stderr bytes.Buffer
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
	if cmd.Start() != nil {
		return "", trustSealedFailure()
	}
	finished := false
	defer func() {
		if !finished {
			_ = cmd.Cancel()
			_ = cmd.Wait()
		}
	}()
	fail := func() (string, error) {
		if childCtx.Err() != nil {
			return "", trustSealedTimeout()
		}
		return "", trustSealedFailure()
	}
	initial := trustSecretChildInput{Probe: trustSealedInput{Work: work, Private: private}, Grant: g, Size: len(value)}
	if json.NewEncoder(in).Encode(initial) != nil {
		return fail()
	}
	decoder := json.NewDecoder(out)
	var ready trustSecretReady
	if decoder.Decode(&ready) != nil || !ready.Ready || ready.Result.ABI < 6 || len(ready.Result.Checks) != len(trustSealedCheckNames) {
		return fail()
	}
	for _, name := range trustSealedCheckNames {
		if !ready.Result.Checks[name] {
			return fail()
		}
	}
	if _, err = in.Write(value); err != nil {
		return fail()
	}
	_ = in.Close()
	var reply struct {
		Attestation string `json:"attestation"`
	}
	if decoder.Decode(&reply) != nil {
		return fail()
	}
	var extra any
	if decoder.Decode(&extra) != io.EOF {
		return fail()
	}
	err = cmd.Wait()
	finished = true
	if err != nil || stderr.Len() != 0 {
		return fail()
	}
	decoded, e := hex.DecodeString(reply.Attestation)
	if e != nil || len(decoded) != sha256.Size {
		return fail()
	}
	return reply.Attestation, nil
}
func runTrustSecretChild(in io.Reader, out io.Writer) int {
	reader := bufio.NewReader(io.LimitReader(in, (2<<20)+1))
	line, err := reader.ReadBytes('\n')
	if err != nil || len(line) > 1<<20 {
		return 4
	}
	var initial trustSecretChildInput
	if strict(line, &initial) != nil || initial.Size <= 0 || initial.Size > 1<<20 || initial.Grant.Connector != "sealed-attestation" || initial.Grant.Destination != "local-attestation" {
		return 4
	}
	if initial.Grant.DataContext != nil && *initial.Grant.DataContext != (TrustDataContext{Classification: "restricted", Destination: "local-attestation", Provider: "swarm", Tool: "sealed-attestation"}) {
		return 4
	}
	result, err := trustSealedChildProbe(initial.Probe)
	if err != nil {
		return 4
	}
	for _, name := range trustSealedCheckNames {
		if !result.Checks[name] {
			return 4
		}
	}
	if printJSON(out, trustSecretReady{Ready: true, Result: result}) != nil {
		return 4
	}
	value := make([]byte, initial.Size)
	defer clear(value)
	if _, err = io.ReadFull(reader, value); err != nil {
		return 4
	}
	mac := hmac.New(sha256.New, value)
	_, _ = mac.Write(trustSecretMessage(initial.Grant))
	if printJSON(out, map[string]string{"attestation": hex.EncodeToString(mac.Sum(nil))}) != nil {
		return 4
	}
	return 0
}
