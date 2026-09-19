//go:build linux

package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"time"
)

type AIConnection struct {
	ID       string `json:"id"`
	Label    string `json:"label"`
	BaseURL  string `json:"base_url"`
	Model    string `json:"model"`
	Key      string `json:"key,omitempty"`
	Disabled bool   `json:"disabled"`
}
type AIConnectionChange struct {
	Version    int          `json:"version"`
	Expected   string       `json:"expected_digest"`
	Connection AIConnection `json:"connection"`
	ReplaceKey bool         `json:"replace_key"`
}

func (s *Store) readAIConnections() (map[string]AIConnection, string, error) {
	path, e := localFile(s.root, ".swarm/ai-connections.json")
	if os.IsNotExist(e) {
		return map[string]AIConnection{}, hash(nil), nil
	}
	if e != nil {
		return nil, "", e
	}
	b, e := os.ReadFile(path)
	if os.IsNotExist(e) {
		return map[string]AIConnection{}, hash(nil), nil
	}
	if e != nil {
		return nil, "", e
	}
	if len(b) > 65536 {
		return nil, "", fmt.Errorf("Configuration IA trop grande.")
	}
	var all map[string]AIConnection
	e = strict(b, &all)
	if all == nil {
		all = map[string]AIConnection{}
	}
	return all, hash(b), e
}
func validateAIConnection(c AIConnection) error {
	if !safeName(c.ID) || len(c.ID) > 64 || strings.HasPrefix(c.ID, "api-") {
		return fmt.Errorf("Identifiant requis : lettres, chiffres, tirets ; sans préfixe api-.")
	}
	if strings.TrimSpace(c.Label) == "" || len(c.Label) > 100 || !modelName.MatchString(c.Model) {
		return fmt.Errorf("Nom et identifiant du modèle requis.")
	}
	u, e := url.Parse(c.BaseURL)
	if e != nil || u.Host == "" || u.User != nil || u.RawQuery != "" || u.Fragment != "" || (u.Scheme != "http" && u.Scheme != "https") {
		return fmt.Errorf("Adresse HTTP(S) requise, sans identifiants, paramètres ni fragment.")
	}
	if len(c.Key) > 8192 || strings.ContainsAny(c.Key, "\r\n") {
		return fmt.Errorf("Clé invalide.")
	}
	return nil
}
func (s *Store) saveAIConnection(change AIConnectionChange) error {
	if change.Version != 1 {
		return fmt.Errorf("Version 1 requise.")
	}
	c := change.Connection
	c.BaseURL = strings.TrimRight(strings.TrimSpace(c.BaseURL), "/")
	c.Model = strings.TrimSpace(c.Model)
	if e := validateAIConnection(c); e != nil {
		return e
	}
	lock, e := os.OpenFile(filepath.Join(s.root, ".swarm/providers.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if e != nil {
		return e
	}
	defer lock.Close()
	if e = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); e != nil {
		return e
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	all, digest, e := s.readAIConnections()
	if e != nil {
		return e
	}
	if change.Expected != digest {
		return fmt.Errorf("Connexions modifiées : actualisez avant d’enregistrer.")
	}
	if !change.ReplaceKey {
		old := all[c.ID]
		if old.Key != "" && old.BaseURL != c.BaseURL {
			return fmt.Errorf("Adresse modifiée : renseignez explicitement la clé pour cette destination.")
		}
		c.Key = old.Key
	}
	if _, ok := all[c.ID]; !ok && len(all) >= 32 {
		return fmt.Errorf("Maximum de 32 connexions.")
	}
	// API providers have a reserved prefix; never replace an existing executable.
	ps, e := s.providersFile()
	if e != nil {
		return e
	}
	if _, ok := ps.Providers["api-"+c.ID]; ok {
		return fmt.Errorf("Identifiant déjà utilisé par un exécutable.")
	}
	all[c.ID] = c
	b, e := json.MarshalIndent(all, "", "  ")
	if e != nil {
		return e
	}
	if len(b) > 65536 {
		return fmt.Errorf("Configuration IA trop grande.")
	}
	path := filepath.Join(s.root, ".swarm", "ai-connections.json")
	return atomicWrite(path, b)
}
func (s *Store) aiConnectionsPublic() (any, error) {
	all, digest, e := s.readAIConnections()
	if e != nil {
		return nil, e
	}
	rows := []map[string]any{}
	for _, c := range all {
		rows = append(rows, map[string]any{"id": c.ID, "label": c.Label, "base_url": c.BaseURL, "model": c.Model, "disabled": c.Disabled, "has_key": c.Key != "", "provider": "api-" + c.ID})
	}
	return map[string]any{"version": 1, "digest": digest, "connections": rows}, nil
}
func callAIConnection(ctx context.Context, c AIConnection, prompt, schema string) (string, error) {
	if e := validateAIConnection(c); e != nil {
		return "", e
	}
	if c.Disabled {
		return "", fmt.Errorf("Connexion désactivée.")
	}
	messages := []map[string]string{}
	if schema != "" {
		messages = append(messages, map[string]string{"role": "system", "content": "Répondre uniquement avec un objet JSON conforme au contrat suivant. Aucun outil disponible.\n" + schema})
	}
	messages = append(messages, map[string]string{"role": "user", "content": prompt})
	body, _ := json.Marshal(map[string]any{"model": c.Model, "messages": messages, "stream": false})
	req, e := http.NewRequestWithContext(ctx, "POST", strings.TrimRight(c.BaseURL, "/")+"/chat/completions", bytes.NewReader(body))
	if e != nil {
		return "", fmt.Errorf("Adresse invalide.")
	}
	req.Header.Set("Content-Type", "application/json")
	if c.Key != "" {
		req.Header.Set("Authorization", "Bearer "+c.Key)
	}
	client := &http.Client{Timeout: 60 * time.Second, CheckRedirect: func(*http.Request, []*http.Request) error { return fmt.Errorf("Redirection refusée") }}
	resp, e := client.Do(req)
	if e != nil {
		return "", fmt.Errorf("Connexion impossible ou délai dépassé ; vérifiez l’adresse et l’accès réseau.")
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return "", fmt.Errorf("Le serveur a répondu HTTP %d. Vérifiez la clé, le modèle et l’adresse.", resp.StatusCode)
	}
	raw, e := io.ReadAll(io.LimitReader(resp.Body, (1<<20)+1))
	if e != nil || len(raw) > 1<<20 {
		return "", fmt.Errorf("Réponse illisible ou supérieure à 1 Mio.")
	}
	var answer struct {
		Choices []struct {
			Message struct {
				Content   string            `json:"content"`
				ToolCalls []json.RawMessage `json:"tool_calls"`
			} `json:"message"`
		} `json:"choices"`
	}
	if json.Unmarshal(raw, &answer) != nil || len(answer.Choices) == 0 || strings.TrimSpace(answer.Choices[0].Message.Content) == "" || len(answer.Choices[0].Message.ToolCalls) > 0 {
		return "", fmt.Errorf("Réponse texte attendue ; les appels d’outils ne sont pas pris en charge par cette connexion.")
	}
	return answer.Choices[0].Message.Content, nil
}
func (s *Store) registerAIConnections(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/providers/connections", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == "GET" {
			v, e := s.aiConnectionsPublic()
			if e != nil {
				fail(w, e)
			} else {
				send(w, v)
			}
			return
		}
		if r.Method != "POST" {
			http.Error(w, "GET ou POST requis", 405)
			return
		}
		var c AIConnectionChange
		b, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 20000))
		if e == nil {
			e = strict(b, &c)
		}
		if e == nil {
			e = s.saveAIConnection(c)
		}
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]bool{"saved": true})
	})
	mux.HandleFunc("/api/v1/providers/connections/test", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		var change AIConnectionChange
		b, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 20000))
		if e == nil {
			e = strict(b, &change)
		}
		if e != nil {
			fail(w, e)
			return
		}
		c := change.Connection
		if !change.ReplaceKey {
			all, _, e := s.readAIConnections()
			if e != nil {
				fail(w, e)
				return
			}
			old := all[c.ID]
			if old.BaseURL == c.BaseURL {
				c.Key = old.Key
			} else if old.Key != "" {
				fail(w, fmt.Errorf("Adresse modifiée : fournissez explicitement la clé pour cette destination."))
				return
			}
		}
		ctx, cancel := context.WithTimeout(r.Context(), 15*time.Second)
		defer cancel()
		start := time.Now()
		reply, e := callAIConnection(ctx, c, "Réponds seulement : Connexion réussie.", "")
		if e != nil {
			send(w, map[string]any{"ok": false, "error": e.Error(), "latency_ms": time.Since(start).Milliseconds()})
			return
		}
		if c.Key != "" {
			reply = strings.ReplaceAll(reply, c.Key, "[secret masqué]")
		}
		send(w, map[string]any{"ok": true, "text": guardBlock(reply, 500), "latency_ms": time.Since(start).Milliseconds()})
	})
}

// The subprocess exposes only text completion, never filesystem or shell tools.
func runAPIChat(args []string, out, errOut io.Writer) int {
	if len(args) < 3 {
		return 2
	}
	s := &Store{root: args[1]}
	all, _, e := s.readAIConnections()
	c, ok := all[args[2]]
	if e != nil || !ok || c.Disabled {
		fmt.Fprintln(errOut, "Connexion absente ou désactivée.")
		return 2
	}
	raw, _ := json.Marshal(c)
	schema := ""
	matched := false
	for i := 3; i+1 < len(args); i += 2 {
		switch args[i] {
		case "--connection-digest":
			if args[i+1] != hash(raw) {
				fmt.Fprintln(errOut, "Connexion modifiée depuis la préparation de l’appel.")
				return 2
			}
			matched = true
		case "--model":
			if args[i+1] != c.Model {
				fmt.Fprintln(errOut, "Modèle modifié depuis la sélection.")
				return 2
			}
		case "--json-schema":
			schema = args[i+1]
		default:
			return 2
		}
	}
	if !matched {
		return 2
	}
	prompt, e := io.ReadAll(io.LimitReader(os.Stdin, 1000001))
	if e != nil || len(prompt) > 1000000 {
		return 2
	}
	reply, e := callAIConnection(context.Background(), c, string(prompt), schema)
	if e != nil {
		fmt.Fprintln(errOut, e.Error())
		return 2
	}
	result := map[string]any{"type": "result", "result": reply}
	if schema != "" {
		var obj map[string]any
		if json.Unmarshal([]byte(reply), &obj) != nil {
			fmt.Fprintln(errOut, "Le modèle n’a pas respecté la réponse JSON demandée.")
			return 2
		}
		result["structured_output"] = obj
	}
	_ = json.NewEncoder(out).Encode(result)
	return 0
}
