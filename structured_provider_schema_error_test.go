//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

func TestStructuredSchemaRejectionDiagnostic(t *testing.T) {
	dir := t.TempDir()
	cmd := filepath.Join(dir, "claude")
	script := `#!/bin/sh
printf '%s\n' '{"type":"error","message":"invalid_json_schema PRIVATE_PAYLOAD"}' '{"type":"turn.failed","error":{"message":"invalid_json_schema PRIVATE_PAYLOAD"}}'
exit 1
`
	if e := os.WriteFile(cmd, []byte(script), 0700); e != nil {
		t.Fatal(e)
	}
	_, e := runStructuredProvider(Provider{Command: cmd}, nil, "prompt", "{}", time.Second, func() bool { return true }, nil)
	if e == nil || !strings.Contains(e.Error(), "invalid_json_schema") || strings.Contains(e.Error(), "PRIVATE_PAYLOAD") {
		t.Fatal(e)
	}
}
