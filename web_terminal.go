//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strconv"
	"strings"
)

func (s *Store) registerTerminals(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	// xterm uses dynamically generated, nonced style elements and positioning
	// attributes. Script policy stays self-only; this exception is terminal-frame-only.
	serve := func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" && r.Method != "HEAD" {
			w.WriteHeader(405)
			return
		}
		raw, e := cockpitWeb.ReadFile("web/terminal.html")
		if e != nil {
			fail(w, e)
			return
		}
		nonce := newID("style-")
		policy := strings.Replace(w.Header().Get("Content-Security-Policy"), "frame-ancestors 'none'", "frame-ancestors 'self'", 1)
		w.Header().Set("Content-Security-Policy", strings.Replace(policy, "style-src 'self'", "style-src 'self' 'nonce-"+nonce+"'", 1)+"; style-src-attr 'unsafe-inline'")
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		if r.Method == "GET" {
			_, _ = io.WriteString(w, strings.ReplaceAll(string(raw), "SWARM_STYLE_NONCE", nonce))
		}
	}
	mux.HandleFunc("/terminal.html", serve)
	mux.HandleFunc("/api/v1/terminal", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		a, e := s.agent(r.URL.Query().Get("agent"))
		if e != nil {
			fail(w, e)
			return
		}
		if a.WorkID != r.URL.Query().Get("work") {
			fail(w, fmt.Errorf("Session hors travail"))
			return
		}
		after, e := strconv.ParseInt(r.URL.Query().Get("after"), 10, 64)
		if e != nil || after < 0 {
			fail(w, fmt.Errorf("Curseur invalide"))
			return
		}
		events := []terminalEvent{}
		if interactiveMode(a.Mode) {
			events, e = s.terminalEvents(a.ID, after)
		} else {
			var logs []AgentLog
			logs, e = s.logs(a.ID, after)
			for _, l := range logs {
				events = append(events, terminalEvent{Seq: l.Seq, Data: []byte(l.At + " · " + l.Kind + " · " + terminalText(l.Message) + "\r\n")})
			}
		}
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"agent": a.ID, "mode": a.Mode, "status": a.Status, "desired": a.Desired, "events": events, "monitoring": agentMonitoring(a), "progress": a.Progress, "health": pilotAgentHealth(a, a.Desired, now()), "usage": a.Usage})
	})
	mux.HandleFunc("/api/v1/terminal/control", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			w.WriteHeader(405)
			return
		}
		var request terminalRequest
		d := json.NewDecoder(http.MaxBytesReader(w, r.Body, 10000))
		d.DisallowUnknownFields()
		if e := d.Decode(&request); e != nil {
			fail(w, e)
			return
		}
		if d.Decode(&struct{}{}) != io.EOF {
			fail(w, fmt.Errorf("Un seul objet attendu"))
			return
		}
		a, e := s.agent(request.Agent)
		if e != nil {
			fail(w, e)
			return
		}
		if a.WorkID != request.Work {
			fail(w, fmt.Errorf("Session hors travail"))
			return
		}
		v, e := s.terminalRPC(a, request)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
}
