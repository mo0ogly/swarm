//go:build linux

package engine

import (
	"context"
	"fmt"
	"io"
	"net/http"
	"os"
)

func registerTrustEvidenceHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/trust-evidence", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		op := r.URL.Query().Get("action")
		raw := []byte("{}")
		if r.Method == http.MethodGet && op == "" {
			op = "show"
		} else if r.Method == http.MethodPost {
			limit := int64(1 << 20)
			if op == "verify" {
				limit = 2 * trustStateMaxBytes
			}
			r.Body = http.MaxBytesReader(w, r.Body, limit)
			var err error
			raw, err = io.ReadAll(r.Body)
			if err != nil {
				fail(w, trustEvidenceInvalid())
				return
			}
		} else {
			fail(w, trustEvidenceInvalid())
			return
		}
		token := trustHTTPBearer(r.Header.Get("Authorization"))
		scope := s
		var err error
		if op != "verify" {
			scope, err = s.trustServiceScope(r.Context(), token)
			if err != nil {
				fail(w, err)
				return
			}
		}
		v, err := scope.trustEvidenceOperation(r.Context(), op, raw, token, true)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, v)
	})
}
func runTrustEvidenceCLI(s *Store, pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("evidence show|policy-preview|policy-apply|key-prepare|key-activate|key-revoke|anchor|verify|archive-preview|archive-create|archive-show|archive-release")
	}
	op := pos[1]
	if op == "key-prepare" || op == "key-activate" || op == "key-revoke" || op == "anchor" {
		return fmt.Errorf("preuve publique : --server requis ; matière privée en mémoire du serveur persistant")
	}
	raw := []byte("{}")
	if input != "" {
		var err error
		raw, err = readInput(input)
		if err != nil {
			return err
		}
	} else if op != "show" {
		return fmt.Errorf("--input requis")
	}
	var v any
	var err error
	if op == "verify" {
		cmd, e := parseTrustEvidence(op, raw)
		if e != nil {
			return e
		}
		v, err = verifyPublicEvidence(cmd)
	} else {
		v, err = s.trustEvidenceOperation(context.Background(), op, raw, os.Getenv("SWARM_TRUST_TOKEN"), false)
	}
	if err != nil {
		return err
	}
	return printJSON(out, v)
}
