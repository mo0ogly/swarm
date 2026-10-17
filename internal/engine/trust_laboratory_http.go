//go:build linux

package engine

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
)

func registerLaboratoryHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/trust-laboratory", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		op := r.URL.Query().Get("action")
		raw := []byte("{}")
		if r.Method == http.MethodGet && op == "" {
			op = "show"
		} else if r.Method == http.MethodPost {
			r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
			var err error
			raw, err = io.ReadAll(r.Body)
			if err != nil {
				fail(w, laboratoryInvalid())
				return
			}
		} else {
			fail(w, laboratoryInvalid())
			return
		}
		token := trustHTTPBearer(r.Header.Get("Authorization"))
		scope, err := s.trustServiceScope(r.Context(), token)
		if err != nil {
			fail(w, err)
			return
		}
		v, err := scope.laboratoryOperation(r.Context(), op, raw, token)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, v)
	})
}
func (s *Store) laboratoryCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("laboratory show|policy-preview|policy-apply|run|result|replay")
	}
	raw := []byte("{}")
	if input != "" {
		var err error
		raw, err = readInput(input)
		if err != nil {
			return err
		}
	} else if pos[1] != "show" {
		return fmt.Errorf("--input requis")
	}
	v, err := s.laboratoryOperation(context.Background(), pos[1], raw, os.Getenv("SWARM_TRUST_TOKEN"))
	if err != nil {
		return err
	}
	return printJSON(out, v)
}
