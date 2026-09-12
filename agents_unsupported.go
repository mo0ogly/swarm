//go:build !linux

package main

import (
	"fmt"
	"io"
	"os"
	"unicode"
)

// Core persistence/scoring remains buildable; process supervision requires Linux.
func hostIdentity() string        { host, _ := os.Hostname(); return host + ":unsupported-runtime" }
func processStamp(pid int) string { return "" }
func terminalText(s string) string {
	return stringMapTerminal(s)
}
func stringMapTerminal(s string) string {
	out := []rune{}
	for _, r := range s {
		if unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {
			if r == '\n' || r == '\t' {
				out = append(out, ' ')
			}
			continue
		}
		out = append(out, r)
	}
	return string(out)
}
func agentCLI(s *Store, pos []string, input, output string, asJSON bool, out io.Writer) error {
	return fmt.Errorf("cockpit et superviseur disponibles sous Linux ; work/resume/export restent utilisables")
}
