//go:build linux

package main

import (
	"os"
	"os/exec"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestWorkerBuildCacheAvoidsGlobalReadOnlyCache(t *testing.T) {
	workspace := t.TempDir()
	global := filepath.Join(t.TempDir(), "global-cache")
	if err := os.Mkdir(global, 0700); err != nil {
		t.Fatal(err)
	}
	sentinel := filepath.Join(global, "preserve")
	if err := os.WriteFile(sentinel, []byte("unchanged"), 0400); err != nil {
		t.Fatal(err)
	}
	if err := os.Chmod(global, 0500); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.Chmod(global, 0700) })
	cache, err := workerBuildCache(workspace)
	if err != nil {
		t.Fatal(err)
	}
	again, err := workerBuildCache(workspace)
	if err != nil || again != cache {
		t.Fatalf("cache not reusable: %q %v", again, err)
	}
	env := workerCacheEnvironment([]string{"PATH=" + os.Getenv("PATH"), "GOCACHE=" + global, "OTHER=retained"}, cache)
	cmd := exec.Command("sh", "-c", `printf '%s' "$GOCACHE"; printf usable > "$GOCACHE/proof"`)
	cmd.Env = env
	output, err := cmd.CombinedOutput()
	if err != nil || string(output) != cache {
		t.Fatalf("worker cache unusable: %s %v", output, err)
	}
	contents, err := os.ReadFile(sentinel)
	if err != nil || string(contents) != "unchanged" {
		t.Fatal("global cache modified")
	}
	if contents, err := os.ReadFile(filepath.Join(cache, "proof")); err != nil || string(contents) != "usable" {
		t.Fatal("cache write not demonstrated")
	}
	for _, entry := range env {
		if entry == "GOCACHE="+global {
			t.Fatal("inherited global cache retained")
		}
	}
	if !strings.Contains(strings.Join(env, "\n"), "OTHER=retained") {
		t.Fatal("unrelated environment changed")
	}
}

func TestWorkerBuildCacheRejectsRedirectAndFile(t *testing.T) {
	for _, part := range []string{".swarm", "cache", "go-build"} {
		root, outside := t.TempDir(), t.TempDir()
		parent := root
		for _, name := range []string{".swarm", "cache", "go-build"} {
			path := filepath.Join(parent, name)
			if name == part {
				if err := os.Symlink(outside, path); err != nil {
					t.Fatal(err)
				}
				break
			}
			if err := os.Mkdir(path, 0700); err != nil {
				t.Fatal(err)
			}
			parent = path
		}
		if _, err := workerBuildCache(root); err == nil {
			t.Fatalf("redirect %s accepted", part)
		}
		files, err := os.ReadDir(outside)
		if err != nil || len(files) != 0 {
			t.Fatal("redirect destination modified")
		}
	}
	root := t.TempDir()
	if err := os.WriteFile(filepath.Join(root, ".swarm"), []byte("user data"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err := workerBuildCache(root); err == nil {
		t.Fatal("ordinary file overwritten")
	}
}

func TestWorkerCacheArgsRetainSandboxAndInput(t *testing.T) {
	original := []string{"exec", "--json", "--sandbox", "workspace-write", "-"}
	before := append([]string(nil), original...)
	cache := filepath.Join(t.TempDir(), "cache with spaces")
	got := workerCacheArgs("/usr/bin/codex", original, cache)
	if !reflect.DeepEqual(original, before) {
		t.Fatal("stored args changed")
	}
	joined := strings.Join(got, "\n")
	if got[len(got)-1] != "-" || !strings.Contains(joined, "workspace-write") || !strings.Contains(joined, "shell_environment_policy.set.GOCACHE=") || strings.Contains(joined, "danger-full-access") {
		t.Fatalf("sandbox/input changed: %v", got)
	}
	if !reflect.DeepEqual(workerCacheArgs("claude", original, cache), original) {
		t.Fatal("Claude args changed")
	}
}

func TestWorkerLaunchCacheEnvironmentAndJournal(t *testing.T) {
	s := storeTest(t)
	w, request := setupAgent(t, s)
	request.Role = "worker"
	a, _, err := s.prepare(w.ID, request)
	if err != nil {
		t.Fatal(err)
	}
	env, _, err := s.workerLaunchEnvironment(a)
	if err != nil {
		t.Fatal(err)
	}
	cache := ""
	for _, value := range env {
		if strings.HasPrefix(value, "GOCACHE=") {
			cache = strings.TrimPrefix(value, "GOCACHE=")
		}
	}
	if cache == "" || !strings.HasPrefix(cache, a.CWD+string(filepath.Separator)) {
		t.Fatalf("cache outside workspace: %s", cache)
	}
	cmd := exec.Command("sh", "-c", `printf engine-ready > "$GOCACHE/proof"`)
	cmd.Env = env
	if out, err := cmd.CombinedOutput(); err != nil {
		t.Fatalf("provider cache failed: %s %v", out, err)
	}
	logs, err := s.logs(a.ID, 0)
	if err != nil {
		t.Fatal(err)
	}
	found := false
	for _, log := range logs {
		if log.Kind == "environment" && strings.Contains(log.Message, cache) {
			found = true
		}
	}
	if !found {
		t.Fatal("cache not visible in agent journal")
	}
	a.Role = "reviewer"
	env, args, err := s.workerLaunchEnvironment(a)
	if err != nil || !reflect.DeepEqual(args, a.Args) {
		t.Fatal("reviewer command changed")
	}
	for _, value := range env {
		if strings.HasPrefix(value, "GOCACHE=") {
			t.Fatal("worker cache imposed on reviewer")
		}
	}
}
