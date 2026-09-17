//go:build linux

package main

import (
	"fmt"
	"os"
	"os/exec"
	"os/signal"
	"path/filepath"
	"strings"
	"unicode/utf8"
)

// Quoting for argv only. No shell, variable expansion, substitutions or pipelines.
func preparationEditorArgs(value string) ([]string, error) {
	if info, e := os.Stat(value); e == nil && !info.IsDir() {
		return []string{value}, nil
	}
	args := []string{}
	var word strings.Builder
	quote := rune(0)
	escape := false
	for _, c := range value {
		if escape {
			word.WriteRune(c)
			escape = false
			continue
		}
		if c == '\\' && quote != '\'' {
			escape = true
			continue
		}
		if quote != 0 {
			if c == quote {
				quote = 0
			} else {
				word.WriteRune(c)
			}
			continue
		}
		if c == '\'' || c == '"' {
			quote = c
			continue
		}
		if c == ' ' || c == '\t' {
			if word.Len() > 0 {
				args = append(args, word.String())
				word.Reset()
			}
			continue
		}
		word.WriteRune(c)
	}
	if escape || quote != 0 {
		return nil, fmt.Errorf("VISUAL/EDITOR : guillemets non terminés.")
	}
	if word.Len() > 0 {
		args = append(args, word.String())
	}
	if len(args) == 0 {
		return nil, fmt.Errorf("Définissez VISUAL ou EDITOR, par exemple nano ou 'code --wait'.")
	}
	return args, nil
}
func (t *preparationTerminal) editExternal(kind string, in *os.File) error {
	if t.p.ID == "" {
		return fmt.Errorf("Enregistrez d’abord le besoin.")
	}
	editor := os.Getenv("VISUAL")
	if editor == "" {
		editor = os.Getenv("EDITOR")
	}
	args, e := preparationEditorArgs(editor)
	if e != nil {
		return e
	}
	dir, e := os.MkdirTemp("", "swarm-document-")
	if e != nil {
		return e
	}
	file := filepath.Join(dir, kind+".txt")
	if e = os.WriteFile(file, []byte(t.p.Documents[kind].Text), 0600); e != nil {
		return e
	}
	request := t.request("save")
	request.Document = kind
	cmd := exec.Command(args[0], append(args[1:], file)...)
	cmd.Stdin = in
	cmd.Stdout = t.out
	cmd.Stderr = t.out
	interrupt := make(chan os.Signal, 1)
	signal.Notify(interrupt, os.Interrupt)
	defer signal.Stop(interrupt)
	if e = cmd.Run(); e != nil {
		return fmt.Errorf("Éditeur interrompu, document serveur inchangé. Fichier conservé : %s", file)
	}
	info, e := os.Lstat(file)
	if e != nil || !info.Mode().IsRegular() || info.Size() > 16000 {
		return fmt.Errorf("Fichier non régulier ou trop long ; non importé, conservé : %s", file)
	}
	b, e := os.ReadFile(file)
	if e != nil || !utf8.Valid(b) {
		return fmt.Errorf("Document UTF-8 illisible ; fichier conservé : %s", file)
	}
	if string(b) == t.p.Documents[kind].Text {
		os.RemoveAll(dir)
		t.say("Document inchangé.")
		return nil
	}
	request.Text = string(b)
	if e = t.mutate(request); e != nil {
		return fmt.Errorf("%v Fichier local conservé : %s", e, file)
	}
	os.RemoveAll(dir)
	if kind == "plan" {
		var p ActionPlan
		if e = strict(b, &p); e != nil {
			t.say("Brouillon enregistré, JSON invalide : " + e.Error())
		} else if _, e = validateActionPlan(p, true); e != nil {
			t.say("Brouillon enregistré, plan à compléter : " + e.Error())
		} else {
			t.say("Brouillon enregistré ; /verifier contrôle cette version.")
		}
	}
	return nil
}
