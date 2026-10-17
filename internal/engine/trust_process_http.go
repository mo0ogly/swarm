//go:build linux

package engine

import (
	"io"
	"net/http"
)

func registerTrustProcessHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/processes", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		op := r.URL.Query().Get("action")
		raw := []byte("{}")
		if r.Method == http.MethodGet {
			if op != "" {
				fail(w, trustProcessInvalid())
				return
			}
			op = "show"
		} else if r.Method == http.MethodPost {
			r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
			var err error
			raw, err = io.ReadAll(r.Body)
			if err != nil {
				fail(w, trustProcessInvalid())
				return
			}
		} else {
			http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
			return
		}
		token := trustHTTPBearer(r.Header.Get("Authorization"))
		scope, err := s.trustServiceScope(r.Context(), token)
		if err != nil {
			fail(w, err)
			return
		}
		value, err := scope.trustProcessOperation(op, raw, token)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
}
