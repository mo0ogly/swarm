//go:build linux

package main

import (
	"bytes"
	"encoding/base64"
	"encoding/json"
	"image"
	"image/png"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func screenshotFixture(t *testing.T, root, name string) []byte {
	t.Helper()
	var b bytes.Buffer
	if err := png.Encode(&b, image.NewRGBA(image.Rect(0, 0, 2, 3))); err != nil {
		t.Fatal(err)
	}
	path := filepath.Join(root, name)
	if err := os.MkdirAll(filepath.Dir(path), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, b.Bytes(), 0600); err != nil {
		t.Fatal(err)
	}
	return b.Bytes()
}

func TestIndependentReviewImagesOnlyDeclaredFreshScreenshots(t *testing.T) {
	s := storeTest(t)
	name := "docs/screenshots/declared.png"
	raw := screenshotFixture(t, s.root, name)
	screenshotFixture(t, s.root, "docs/screenshots/not-declared.png")
	task := &Task{ValidationPolicy: &ValidationPolicy{Mode: "human", Controls: []ValidationControl{{Inputs: []string{name, name, "docs/result.md"}}}}}
	images, err := s.independentReviewImages(task, map[string]string{name: hash(raw)})
	if err != nil || len(images) != 1 || !bytes.Equal(images[0].Data, raw) {
		t.Fatalf("images=%v err=%v", len(images), err)
	}
	if _, err = s.independentReviewImages(task, map[string]string{name: hash([]byte("old"))}); err == nil {
		t.Fatal("stale screenshot transmitted")
	}
	task.ValidationPolicy.Controls[0].Inputs = []string{"docs/other.png"}
	if _, err = s.independentReviewImages(task, nil); err == nil {
		t.Fatal("undeclared screenshot scope admitted")
	}
	task.ValidationPolicy.Controls[0].Inputs = []string{name}
	if err = os.WriteFile(filepath.Join(s.root, name), []byte("not an image"), 0600); err != nil {
		t.Fatal(err)
	}
	if _, err = s.independentReviewImages(task, map[string]string{name: hash([]byte("not an image"))}); err == nil {
		t.Fatal("invalid image admitted")
	}
}

func TestStructuredReviewImagesContainsActualBytesAndNoTools(t *testing.T) {
	p, err := assistantProvider(Provider{Command: "/usr/bin/claude", Args: []string{"--dangerously-skip-permissions", "--resume", "producer"}})
	// assistantProvider checks executability; use a local executable named claude.
	if err != nil {
		command := filepath.Join(t.TempDir(), "claude")
		if err = os.WriteFile(command, []byte("#!/bin/sh\nexit 0\n"), 0700); err != nil {
			t.Fatal(err)
		}
		p, err = assistantProvider(Provider{Command: command, Args: []string{"--dangerously-skip-permissions", "--resume", "producer"}})
	}
	if err != nil {
		t.Fatal(err)
	}
	input, err := structuredReviewInput(&p, "Examine the pixels", []reviewImage{{Path: "docs/screenshots/a.png", SHA256: "digest", MediaType: "image/png", Data: []byte("actual bytes")}})
	if err != nil {
		t.Fatal(err)
	}
	var message struct {
		Type    string `json:"type"`
		Message struct {
			Role    string                       `json:"role"`
			Content []map[string]json.RawMessage `json:"content"`
		} `json:"message"`
	}
	if err = json.Unmarshal([]byte(input), &message); err != nil {
		t.Fatal(err)
	}
	if message.Type != "user" || message.Message.Role != "user" || len(message.Message.Content) != 3 {
		t.Fatalf("unexpected message %s", input)
	}
	var source map[string]string
	if err = json.Unmarshal(message.Message.Content[2]["source"], &source); err != nil {
		t.Fatal(err)
	}
	decoded, err := base64.StdEncoding.DecodeString(source["data"])
	if err != nil || string(decoded) != "actual bytes" || source["media_type"] != "image/png" {
		t.Fatal("image bytes missing")
	}
	args := strings.Join(p.Args, "|")
	if !strings.Contains(args, "--tools||--safe-mode") || !strings.Contains(args, "--input-format|stream-json") || strings.Contains(args, "--resume") || strings.Contains(args, "skip-permissions") {
		t.Fatal("unsafe review arguments", args)
	}
	if _, err = structuredReviewInput(&Provider{Command: "/usr/bin/codex"}, "p", []reviewImage{{}}); err == nil {
		t.Fatal("unsupported provider silently loses images")
	}
	prompt, err := structuredReviewInput(&p, "unchanged text", nil)
	if err != nil || prompt != "unchanged text" {
		t.Fatal("text-only review changed")
	}
}
