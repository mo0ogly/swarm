//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"testing"
	"time"
)

func TestKernelObservePublicOwnedSource(t *testing.T) {
	binary := os.Getenv("SWARM_OBSERVE_TEST_BINARY")
	if binary == "" {
		t.Skip("requires explicitly built public binary; one owned-source native availability check")
	}
	s, a := kernelAttemptFixture(t)
	c := kernelObserveChange(t, s, true)
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
	before := kernelObserveStateBytes(t, s)
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()
	server := exec.CommandContext(ctx, binary, "--root", s.root, "web", "127.0.0.1:0")
	reader, writer := io.Pipe()
	server.Stdout = writer
	server.Stderr = writer
	if err := server.Start(); err != nil {
		t.Fatal(err)
	}
	defer func() { _ = server.Process.Kill(); _ = server.Wait(); _ = writer.Close(); _ = reader.Close() }()
	receipt := map[string]any{"pid": server.Process.Pid, "start": processStamp(server.Process.Pid), "binary": binary, "root": s.root, "source_pid": os.Getpid(), "source_kind": "registered owned Go test fixture, no provider"}
	out := os.Getenv("SWARM_OBSERVE_TEST_EVIDENCE")
	if out != "" {
		b, _ := json.MarshalIndent(receipt, "", "  ")
		if err := os.WriteFile(filepath.Join(out, "observe-public-process.json"), b, 0600); err != nil {
			t.Fatal(err)
		}
	}
	ready := make(chan []string, 1)
	go func() {
		defer reader.Close()
		var log bytes.Buffer
		buf := make([]byte, 4096)
		pattern := regexp.MustCompile(`http://(127\.0\.0\.1:[0-9]+)/session/(session-[a-zA-Z0-9]+)`)
		for {
			n, e := reader.Read(buf)
			log.Write(buf[:n])
			if m := pattern.FindStringSubmatch(log.String()); m != nil {
				ready <- m
				_, _ = io.Copy(io.Discard, reader)
				return
			}
			if e != nil {
				return
			}
		}
	}()
	var m []string
	select {
	case m = <-ready:
	case <-ctx.Done():
		t.Fatal("owned public server startup failed")
	}
	raw, _ := json.Marshal(kernelObserveCommand{AgentID: a.ID})
	client := exec.CommandContext(ctx, binary, "--root", filepath.Join(t.TempDir(), "absent"), "--server", "http://"+m[1], "--json", "trust", "kernel-observe", "--input", "-")
	client.Env = append(os.Environ(), "SWARM_TRUST_TOKEN="+trustAdminToken, "SWARM_WEB_SESSION="+m[2])
	client.Stdin = bytes.NewReader(raw)
	var stdout, stderr bytes.Buffer
	client.Stdout = &stdout
	client.Stderr = &stderr
	err := client.Run()
	if client.ProcessState == nil || client.ProcessState.ExitCode() != 3 {
		if out != "" {
			_ = os.WriteFile(filepath.Join(out, "observe-public-private-failure.json"), append(stdout.Bytes(), stderr.Bytes()...), 0600)
		}
		t.Fatalf("expected public unavailable code 3, observed %v", err)
	}
	var result TrustKernelObserveResult
	if json.Unmarshal(stdout.Bytes(), &result) != nil || result.Available || !result.CleanupVerified || len(result.Events) != 0 || result.Attribution.AgentID != a.ID || result.Attribution.PID != os.Getpid() {
		t.Fatal("public attribution/unavailability contract failed")
	}
	if !bytes.Equal(before, kernelObserveStateBytes(t, s)) {
		t.Fatal("observer mutated business state")
	}
	if out != "" {
		_ = os.WriteFile(filepath.Join(out, "observe-public-unavailable.json"), stdout.Bytes(), 0600)
	}
	t.Log("actual remote CLI and HTTP: code3, registry-derived owned source, no synthetic events, unchanged state")
}
