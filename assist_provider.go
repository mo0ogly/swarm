//go:build linux

package main

import (
	"fmt"
	"os"
	"path/filepath"
)

// Provider configuration is operator-trusted, but coding arguments must not be
// reused for page explanations. Unknown adapters fail closed. Only model choice
// is carried over, never permission, workspace, resume, plugin or hook flags.
func assistantProvider(p Provider) (Provider, error) {
	if !filepath.IsAbs(p.Command) {
		return p, fmt.Errorf("Chemin absolu du fournisseur requis.")
	}
	info, e := os.Stat(p.Command)
	if e != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
		return p, fmt.Errorf("Exécutable du fournisseur indisponible.")
	}
	model := ""
	for i := 0; i < len(p.Args)-1; i++ {
		if p.Args[i] == "--model" || p.Args[i] == "-m" {
			model = p.Args[i+1]
			i++
		}
	}
	switch filepath.Base(p.Command) {
	case "claude", "skynet_harness":
		p.Args = []string{"-p", "--output-format", "stream-json", "--verbose", "--tools", "", "--safe-mode", "--strict-mcp-config", "--mcp-config", `{"mcpServers":{}}`, "--no-session-persistence"}
	case "codex":
		p.Args = []string{"exec", "--json", "--sandbox", "read-only", "--ignore-user-config", "--ignore-rules", "--ephemeral", "--skip-git-repo-check", "-c", `web_search="disabled"`}
		for _, f := range []string{"shell_tool", "unified_exec", "multi_agent", "apps", "hooks", "browser_use", "computer_use", "image_generation", "view_image", "code_mode", "code_mode_host", "tool_suggest"} {
			p.Args = append(p.Args, "--disable", f)
		}
	default:
		return p, fmt.Errorf("Assistant de page indisponible pour ce fournisseur : adaptateur sans outils non vérifié. Utiliser Claude ou Codex ; les agents de travail restent disponibles.")
	}
	if model != "" {
		p.Args = append(p.Args, "--model", model)
	}
	if filepath.Base(p.Command) == "codex" {
		p.Args = append(p.Args, "-")
	}
	return p, nil
}
