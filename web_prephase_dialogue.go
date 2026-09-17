//go:build linux

package main

import (
	"io"
	"net/http"
)

func (s *Store) registerPreparationDialogue(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/preparations/providers", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		send(w, map[string]any{"providers": s.preparationCapabilities(), "scope": "Documents enregistrés, extraits explicitement joints, méthode sélectionnée et conversation. Aucun outil ni exploration autonome du dépôt.", "max_turns": preparationTurnLimit, "max_context_bytes": preparationContextLimit})
	})
	mux.HandleFunc("/api/v1/preparations/dialogue", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			w.WriteHeader(405)
			return
		}
		ts, e := s.preparationDialogue(r.URL.Query().Get("id"))
		if e != nil {
			fail(w, e)
			return
		}
		send(w, ts)
	})
	mux.HandleFunc("/api/v1/preparations/send", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			w.WriteHeader(405)
			return
		}
		b, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 16384))
		if e != nil {
			fail(w, e)
			return
		}
		var req PreparationSend
		if e = strict(b, &req); e != nil {
			fail(w, e)
			return
		}
		t, e := s.sendPreparation(req)
		if e == nil {
			e = s.spawnPreparationTurn(t)
		}
		if e != nil {
			fail(w, e)
			return
		}
		t.Prompt = ""
		send(w, t)
	})
	mux.HandleFunc("/api/v1/preparations/stop", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			w.WriteHeader(405)
			return
		}
		b, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 1024))
		if e != nil {
			fail(w, e)
			return
		}
		var req struct {
			ID   string `json:"preparation_id"`
			Turn string `json:"turn_id"`
		}
		if e = strict(b, &req); e != nil {
			fail(w, e)
			return
		}
		t, e := s.cancelPreparation(req.ID, req.Turn)
		if e != nil {
			fail(w, e)
			return
		}
		t.Prompt = ""
		send(w, t)
	})
}
