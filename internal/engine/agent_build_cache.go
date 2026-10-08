//go:build linux

package engine

import (
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// A worker's global Go build cache can be readable but not writable inside a
// workspace-write sandbox. Use a reusable cache within its authorized workspace;
// never chmod, erase, or mount the user's global cache into that sandbox.
func workerBuildCache(cwd string) (string, error) {
	root, err := filepath.EvalSymlinks(cwd)
	if err != nil {
		return "", err
	}
	dir := root
	for _, part := range []string{".swarm", "cache", "go-build"} {
		dir = filepath.Join(dir, part)
		if err := os.Mkdir(dir, 0700); err != nil && !os.IsExist(err) {
			return "", err
		}
		info, err := os.Lstat(dir)
		if err != nil {
			return "", err
		}
		if !info.IsDir() || info.Mode()&os.ModeSymlink != 0 {
			return "", fmt.Errorf(uiText("cache de compilation : répertoire ordinaire requis : %s"), dir)
		}
	}
	probe, err := os.CreateTemp(dir, ".write-check-")
	if err != nil {
		return "", fmt.Errorf("cache de compilation inaccessible en écriture : %w", err)
	}
	name := probe.Name()
	closeErr := probe.Close()
	removeErr := os.Remove(name)
	if closeErr != nil {
		return "", closeErr
	}
	if removeErr != nil {
		return "", removeErr
	}
	return dir, nil
}

func workerCacheEnvironment(env []string, cache string) []string {
	result := make([]string, 0, len(env)+1)
	for _, value := range env {
		if !strings.HasPrefix(value, "GOCACHE=") {
			result = append(result, value)
		}
	}
	return append(result, "GOCACHE="+cache)
}

func workerCacheArgs(command string, args []string, cache string) []string {
	result := append([]string(nil), args...)
	if filepath.Base(command) != "codex" {
		return result
	}
	// Codex may filter inherited variables. Set only this non-secret tool variable,
	// preserving its sandbox and all other environment/security policies.
	config := []string{"-c", "shell_environment_policy.set.GOCACHE=" + strconv.Quote(cache)}
	at := len(result)
	if at > 0 && result[at-1] == "-" {
		at--
	}
	result = append(result[:at], append(config, result[at:]...)...)
	return result
}

func (s *Store) workerLaunchEnvironment(a Agent) ([]string, []string, error) {
	env, args := providerEnvironment(a.Env), append([]string(nil), a.Args...)
	if a.Role != "worker" {
		return env, args, nil
	}
	cache, err := workerBuildCache(a.CWD)
	if err != nil {
		return nil, nil, fmt.Errorf("cache de compilation non prêt : %w", err)
	}
	if err := s.log(a.ID, "environment", fmt.Sprintf(uiText("Cache de compilation Go prêt : %s ; cache global conservé."), cache)); err != nil {
		return nil, nil, err
	}
	return workerCacheEnvironment(env, cache), workerCacheArgs(a.Command, args, cache), nil
}
