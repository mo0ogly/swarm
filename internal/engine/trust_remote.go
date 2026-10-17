//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/url"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
)

func trustRemoteOrigin(raw string) (*url.URL, error) {
	u, err := url.Parse(raw)
	if err != nil || u.Scheme != "http" || u.User != nil || u.RawQuery != "" || u.ForceQuery || u.Fragment != "" || u.RawPath != "" || (u.Path != "" && u.Path != "/") {
		return nil, fmt.Errorf("--server trust : origine HTTP locale avec port, sans identifiant, chemin, requête ni fragment requise")
	}
	ip := net.ParseIP(u.Hostname())
	port, err := strconv.Atoi(u.Port())
	if ip == nil || !ip.IsLoopback() || err != nil || port < 1 || port > 65535 {
		return nil, fmt.Errorf("--server trust : adresse IP de boucle locale et port explicite requis")
	}
	u.Path, u.RawPath = "", ""
	return u, nil
}

func runTrustRemoteCLI(pos []string, input, server string, out io.Writer) error {
	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()
	return trustRemoteCLI(ctx, pos, input, server, os.Getenv("SWARM_WEB_SESSION"), os.Getenv("SWARM_TRUST_TOKEN"), out)
}

func trustRemoteCLI(ctx context.Context, pos []string, input, server, session, token string, out io.Writer) error {
	u, err := trustRemoteOrigin(server)
	if err != nil {
		return err
	}
	if len(pos) != 2 {
		return fmt.Errorf("trust : une opération publique est requise")
	}
	action := pos[1]
	if pos[0] == "laboratory" {
		switch action {
		case "show", "policy-preview", "policy-apply", "run", "result", "replay":
		default:
			return laboratoryInvalid()
		}
	} else if pos[0] == "evidence" {
		switch action {
		case "archive-preview", "archive-create", "archive-show", "archive-release", "show", "policy-preview", "policy-apply", "key-prepare", "key-activate", "key-revoke", "anchor", "verify":
		default:
			return fmt.Errorf("opération evidence distante inconnue")
		}
	} else if pos[0] == "process" {
		switch action {
		case "show", "policy-preview", "policy-apply", "definition-preview", "definition-publish", "case-create", "case-show", "case-transition":
		default:
			return fmt.Errorf("opération process distante inconnue")
		}
	} else {
		switch action {
		case "secret-ready", "secret-load", "secret-grant", "secret-consume", "secret-revoke", "secret-status":
		case "kernel-observe", "kernel-check", "workload-introspection-key", "show", "confinement-check", "identity-context", "workload-login", "workload-logout", "oidc-start", "oidc-finish", "oidc-logout", "oidc-session", "oidc-backchannel-logout", "config-preview", "config-apply", "mandate-create", "mandate-revoke", "request", "approve", "commit", "reconcile", "direct-write", "elevate-integrity", "export", "anchor", "verify":
		default:
			return fmt.Errorf("opération trust distante inconnue")
		}
	}
	if !strings.HasPrefix(session, "session-") || len(session) != len("session-")+24 || !safeName(session) {
		return &CommandError{Code: "authorization_required", Message: "SWARM_WEB_SESSION : session opérateur locale requise"}
	}
	if strings.ContainsAny(token, " \t\r\n") {
		return &CommandError{Code: "authorization_required", Message: "SWARM_TRUST_TOKEN invalide"}
	}
	if action == "secret-load" {
		if input != "-" {
			return fmt.Errorf("secret-load : --input - requis ; octets sur stdin uniquement")
		}
		var ready bytes.Buffer
		if err := trustRemoteCLI(ctx, []string{"trust", "secret-ready"}, "", server, session, token, &ready); err != nil {
			return err
		}
		if !trustSecretPrepared(ready.Bytes()) {
			return trustSecretDenied()
		}
		if err := trustSecretHarden(); err != nil {
			return err
		}
	}
	method := http.MethodPost
	raw := []byte("{}")
	if action == "show" {
		method = http.MethodGet
		if input != "" {
			return fmt.Errorf("trust show : --input refusé")
		}
	} else if input != "" {
		raw, err = readInput(input)
		if err != nil {
			return err
		}
	} else if action != "workload-introspection-key" && action != "identity-context" && action != "confinement-check" && action != "kernel-check" && action != "export" && action != "anchor" && action != "secret-ready" && action != "secret-status" {
		return fmt.Errorf("--input requis")
	}
	origin := u.String()
	u.Path = "/api/v1/trust-actions"
	if pos[0] == "laboratory" {
		u.Path = "/api/v1/trust-laboratory"
	}
	if pos[0] == "evidence" {
		u.Path = "/api/v1/trust-evidence"
	}
	if pos[0] == "process" {
		u.Path = "/api/v1/processes"
	}
	if strings.HasPrefix(action, "secret-") {
		u.Path = "/api/v1/trust-secrets"
	}
	if method == http.MethodPost {
		u.RawQuery = url.Values{"action": {action}}.Encode()
	}
	var body io.Reader
	if method == http.MethodPost {
		body = bytes.NewReader(raw)
	}
	req, err := http.NewRequestWithContext(ctx, method, u.String(), body)
	if err != nil {
		return fmt.Errorf("requête trust distante invalide")
	}
	req.Header.Set("Content-Type", "application/json")
	if action == "secret-load" {
		req.Header.Set("Content-Type", "application/octet-stream")
		defer clear(raw)
	}
	req.Header.Set("Accept", "application/json")
	req.Header.Set("Origin", origin)
	req.Header.Set("X-Swarm-CSRF", session)
	req.AddCookie(webSessionCookie(u.Host, session, 0))
	if token != "" {
		req.Header.Set("Authorization", "Bearer "+token)
	}
	transport := &http.Transport{DisableKeepAlives: true}
	defer transport.CloseIdleConnections()
	client := &http.Client{Transport: transport, CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse }}
	response, err := client.Do(req)
	if err != nil {
		return trustRemoteTransportFailure(ctx)
	}
	defer response.Body.Close()
	var object map[string]json.RawMessage
	decoder := json.NewDecoder(response.Body)
	decodeErr := decoder.Decode(&object)
	var extra any
	if decodeErr == nil && decoder.Decode(&extra) != io.EOF {
		decodeErr = fmt.Errorf("réponse supplémentaire")
	}
	if ctx.Err() != nil {
		return trustRemoteTransportFailure(ctx)
	}
	if response.StatusCode < 200 || response.StatusCode >= 300 {
		var failure CommandError
		if decodeErr == nil && json.Unmarshal(object["failure"], &failure) == nil && failure.Code != "" && failure.Message != "" {
			failure.Message = strings.ReplaceAll(failure.Message, session, "[session]")
			if token != "" {
				failure.Message = strings.ReplaceAll(failure.Message, token, "[jeton]")
			}
			return &failure
		}
		code := "command_failed"
		if response.StatusCode == http.StatusForbidden || response.StatusCode == http.StatusUnauthorized {
			code = "authorization_required"
		}
		return &CommandError{Code: code, Message: fmt.Sprintf("opération trust distante refusée : HTTP %d", response.StatusCode)}
	}
	if decodeErr != nil || len(object) == 0 {
		return fmt.Errorf("réponse trust distante invalide ou vide")
	}
	if action == "kernel-observe" {
		return printKernelObserve(out, object)
	}
	if action == "kernel-check" {
		return printTrustKernelCheck(out, object)
	}
	return printJSON(out, object)
}

func trustRemoteTransportFailure(ctx context.Context) error {
	if ctx.Err() != nil {
		return &CommandError{Code: "interrupted", Message: "transport trust interrompu ; vérifier l'état serveur avant toute reprise"}
	}
	return &CommandError{Code: "provider_unavailable", Message: "serveur trust local inaccessible ; aucun repli local, vérifier l'état serveur avant toute reprise"}
}
