//go:build linux

package main

import (
	"net/http"
	"strconv"
)

func (s *Store) registerPreparationResources(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	for _, kind := range []string{"files", "source", "budget"} {
		kind := kind
		mux.HandleFunc("/api/v1/preparations/"+kind, func(w http.ResponseWriter, r *http.Request) {
			if r.Method != "GET" {
				w.WriteHeader(405)
				return
			}
			q := r.URL.Query()
			if _, e := s.preparation(q.Get("id")); e != nil {
				fail(w, e)
				return
			}
			switch kind {
			case "files":
				v, e := s.preparationSourceFiles(q.Get("scope"), q.Get("q"))
				if e != nil {
					fail(w, e)
					return
				}
				send(w, v)
			case "source":
				start, e := strconv.Atoi(q.Get("start"))
				if e != nil {
					fail(w, preparationError("source_range", "Ligne de début requise."))
					return
				}
				end, e := strconv.Atoi(q.Get("end"))
				if e != nil {
					fail(w, preparationError("source_range", "Ligne de fin requise."))
					return
				}
				v, e := s.preparationReadSource(q.Get("path"), start, end)
				if e != nil {
					fail(w, e)
					return
				}
				send(w, v)
			case "budget":
				v, e := s.preparationBudget(q.Get("id"))
				if e != nil {
					fail(w, e)
					return
				}
				send(w, v)
			}
		})
	}
}
