//go:build linux

package engine

import (
	"bytes"
	"crypto/subtle"
	"encoding/json"
	"fmt"
	"html/template"
	"net/http"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	resources "swarm.local/companion"
)

var webSessionDefaults = resources.WebSession

func (s *Store) webSessionLifetime() (int, error) {
	raw := webSessionDefaults
	if s != nil {
		path := filepath.Join(s.root, ".swarm", "web-session.json")
		if info, err := os.Lstat(path); err == nil {
			if !info.Mode().IsRegular() || info.Size() > 4096 {
				return 0, fmt.Errorf("web-session.json : fichier local borné requis")
			}
			raw, err = os.ReadFile(path)
			if err != nil {
				return 0, err
			}
		} else if !os.IsNotExist(err) {
			return 0, err
		}
	}
	var policy struct {
		Schema int `json:"schema_version"`
		MaxAge int `json:"cookie_max_age_seconds"`
	}
	if err := strict(raw, &policy); err != nil {
		return 0, err
	}
	if policy.Schema != 1 || policy.MaxAge < 1 || policy.MaxAge > 31536000 {
		return 0, fmt.Errorf("web-session.json : schema_version=1, cookie_max_age_seconds=1..31536000 requis")
	}
	return policy.MaxAge, nil
}

func webSessionCookie(host, token string, maxAge int) *http.Cookie {
	return &http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token, Path: "/", HttpOnly: true, SameSite: http.SameSiteStrictMode, MaxAge: maxAge}
}

func webAccessPage(w http.ResponseWriter, r *http.Request, code int, message string) {
	english := r.URL.Query().Get("lang") == "en"
	theme := "sombre"
	if r.URL.Query().Get("theme") == "etat" {
		theme = "etat"
	}
	text := struct{ Lang, Title, Description, Label, Button, Help, Message, Next, Theme, Dark, Light, Offline, Loading string }{
		"fr", "Connexion à Swarm", "Votre cockpit conserve ses missions et son adresse après un redémarrage.", "Clé d’accès locale", "Se connecter", "Sur cette machine, lancez ./swarm.sh open pour ouvrir automatiquement votre session. La clé reste privée dans le dossier du projet.", message, r.URL.RequestURI(), theme, "Sombre", "État", "Connexion interrompue. Vérifiez le serveur puis réessayez.", "Connexion…",
	}
	if english {
		text.Lang = "en"
		text.Title = "Sign in to Swarm"
		text.Description = "Your cockpit keeps its missions and address after a restart."
		text.Label = "Local access key"
		text.Button = "Sign in"
		text.Help = "On this computer, run ./swarm.sh open to open your session automatically. The key stays private in the project directory."
		text.Dark = "Dark"
		text.Light = "Light"
		text.Offline = "Connection interrupted. Check the server and try again."
		text.Loading = "Signing in…"
		if message != "" {
			text.Message = "Access key rejected. Try again."
		}
	}
	if r.URL.Path == "/login" {
		text.Next = "/"
	}
	content, _ := cockpitWeb.ReadFile("web/access.html")
	page, err := template.New("access").Parse(string(content))
	if err != nil {
		http.Error(w, "Page indisponible", 500)
		return
	}
	var rendered bytes.Buffer
	if err = page.Execute(&rendered, text); err != nil {
		http.Error(w, "Page indisponible", 500)
		return
	}
	w.Header().Set("Content-Type", "text/html; charset=utf-8")
	w.WriteHeader(code)
	_, _ = w.Write(rendered.Bytes())
}

func webLogin(w http.ResponseWriter, r *http.Request, host, token string, maxAge int) {
	if r.Method == "GET" {
		webAccessPage(w, r, 200, "")
		return
	}
	if r.Method != "POST" || r.Header.Get("Origin") != "http://"+host {
		http.Error(w, "Origine refusée", 403)
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, 4096)
	if r.ParseForm() != nil || subtle.ConstantTimeCompare([]byte(r.Form.Get("token")), []byte(token)) != 1 {
		if r.Header.Get("Accept") == "application/json" {
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(401)
			message := "Clé d’accès refusée. Réessayez."
			if r.URL.Query().Get("lang") == "en" {
				message = "Access key rejected. Try again."
			}
			_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
			return
		}
		webAccessPage(w, r, 401, "Clé d’accès refusée. Réessayez.")
		return
	}
	target := r.Form.Get("next")
	parsed, err := url.Parse(target)
	if err != nil || parsed.IsAbs() || parsed.Host != "" || strings.Contains(target, "\\") || len(target) == 0 || target[0] != '/' || (len(target) > 1 && target[1] == '/') {
		target = "/"
	}
	http.SetCookie(w, webSessionCookie(host, token, maxAge))
	if r.Header.Get("Accept") == "application/json" {
		w.Header().Set("Content-Type", "application/json")
		_ = json.NewEncoder(w).Encode(map[string]string{"redirect": target})
		return
	}
	http.Redirect(w, r, target, http.StatusSeeOther)
}
