//go:build linux

package main

import (
	"encoding/json"
	"errors"
	"io"
	"net/http"
	"os"
	"strconv"
	"strings"
)

// Registered behind the same loopback host, session cookie, Origin and CSRF
// middleware as the cockpit. GETs are read-only; no provider is started here.
func (s *Store) registerPreparations(mux *http.ServeMux) {
	mux.HandleFunc("/prepare.html", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" && r.Method != "HEAD" {
			w.WriteHeader(405)
			return
		}
		if os.Getenv("SWARM_PREPARATION_DISABLED") == "1" {
			w.Header().Set("Content-Type", "text/html; charset=utf-8")
			w.WriteHeader(http.StatusServiceUnavailable)
			io.WriteString(w, `<!doctype html><html lang="fr" data-theme="etat"><head><meta name="viewport" content="width=device-width,initial-scale=1"><title>Préparation désactivée</title><link rel="stylesheet" href="/wattson_themes.css"><link rel="stylesheet" href="/prephase.css"></head><body><main><h1>Préparation désactivée</h1><p>Le pilotage reste disponible. Les documents sont conservés et restent consultables ou exportables depuis le CLI.</p><a href="/">Revenir au pilotage des agents</a></main></body></html>`)
			return
		}
		raw, err := cockpitWeb.ReadFile("web/prepare.html")
		if err != nil {
			http.Error(w, "Écran indisponible", 500)
			return
		}
		nonce := newID("style-")
		policy := w.Header().Get("Content-Security-Policy")
		w.Header().Set("Content-Security-Policy", strings.Replace(policy, "style-src 'self'", "style-src 'self' 'nonce-"+nonce+"'", 1)+"; style-src-attr 'unsafe-inline'")
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if r.Method == "GET" {
			_, _ = io.WriteString(w, strings.ReplaceAll(string(raw), "SWARM_STYLE_NONCE", nonce))
		}
	})

	send := func(w http.ResponseWriter, v any) {
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		_ = json.NewEncoder(w).Encode(v)
	}
	fail := func(w http.ResponseWriter, e error) {
		status := http.StatusBadRequest
		code := "invalid_request"
		var p *PreparationError
		if errors.As(e, &p) {
			code = p.Code
			if code == "not_found" {
				status = 404
			}
			if code == "conflict" || code == "stale_document" || code == "stale_method" || code == "stale_brief" || code == "stale_plan" {
				status = 409
			}
		}
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		w.WriteHeader(status)
		send(w, map[string]any{"error": e.Error(), "code": code})
	}
	mux.HandleFunc("/api/v1/preparations/conversion", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		review, e := s.preparationConversionReview(r.URL.Query().Get("id"))
		if e != nil {
			fail(w, e)
			return
		}
		send(w, review)
	})
	s.registerPreparationDialogue(mux, send, fail)
	s.registerPreparationResources(mux, send, fail)
	mux.HandleFunc("/api/v1/preparations", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		ps, e := s.preparations()
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"preparations": ps, "limit": 100})
	})
	mux.HandleFunc("/api/v1/preparations/methods", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		send(w, s.preparationMethods())
	})
	mux.HandleFunc("/api/v1/preparations/show", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		p, e := s.preparation(r.URL.Query().Get("id"))
		if e != nil {
			fail(w, e)
			return
		}
		send(w, p)
	})
	mux.HandleFunc("/api/v1/preparations/history", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		q := r.URL.Query()
		before := 0
		if q.Get("before") != "" {
			n, e := strconv.Atoi(q.Get("before"))
			if e != nil || n < 1 {
				fail(w, preparationError("invalid_request", "Révision positive requise."))
				return
			}
			before = n
		}
		d, e := s.preparationHistory(q.Get("id"), q.Get("document"), before)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, d)
	})
	mux.HandleFunc("/api/v1/preparations/command", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			w.WriteHeader(405)
			return
		}
		b, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if e != nil {
			fail(w, e)
			return
		}
		var req PreparationRequest
		if e = strict(b, &req); e != nil {
			fail(w, e)
			return
		}
		p, e := s.preparationCommand(req)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, p)
	})
}
