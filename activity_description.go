//go:build linux

package main

import "strings"

// Only operational metadata is displayed; tool results and model reasoning are
// never copied into this summary. Treat provider descriptions as declarations.
func operationText(value string, limit int) string {
	text := terminalText(value)
	lower := strings.ToLower(text)
	for _, sensitive := range []string{"password", "passwd", "secret", "token", "api_key", "api-key", "authorization", "private key"} {
		if strings.Contains(lower, sensitive) {
			return "[détail masqué : donnée potentiellement sensible]"
		}
	}
	return clip(text, limit)
}
func describeOperation(name string, input any) (string, string) {
	m, _ := input.(map[string]any)
	get := func(k string) string { s, _ := m[k].(string); return operationText(s, 600) }
	description := get("description")
	path := get("file_path")
	if path == "" {
		path = get("path")
	}
	action := "Exécution de l’outil " + operationText(name, 80)
	detail := ""
	switch name {
	case "Read":
		action = "Lecture d’un fichier"
		detail = path
	case "Write":
		action = "Écriture d’un fichier"
		detail = path
	case "Edit", "MultiEdit":
		action = "Modification d’un fichier"
		detail = path
	case "Glob", "Grep":
		action = "Recherche dans les fichiers"
		detail = path
	case "Bash", "command_execution":
		action = "Exécution d’une commande"
		detail = get("command")
		if command, ok := input.(string); ok {
			detail = operationText(command, 600)
		}
	}
	if description != "" {
		action = "Annonce de l’agent : " + description
	}
	return action, detail
}

func operationTarget(detail string) string {
	if detail == "" {
		return ""
	}
	return " · " + detail
}
