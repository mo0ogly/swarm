//go:build linux

package engine

import (
	"io"
	"mime"
	"net/http"
	"time"
)

func registerTrustSecretHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	b := newTrustSecretBroker(s)
	mux.HandleFunc("/api/v1/trust-secrets", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		op := r.URL.Query().Get("action")
		token := trustHTTPBearer(r.Header.Get("Authorization"))
		scoped, scopeErr := s.trustServiceScope(r.Context(), token)
		if scopeErr != nil {
			fail(w, scopeErr)
			return
		}
		requestBroker := *b
		requestBroker.s = scoped

		if op == "secret-load" {
			media, _, mediaErr := mime.ParseMediaType(r.Header.Get("Content-Type"))
			if mediaErr != nil || media != "application/octet-stream" {
				fail(w, trustSecretDenied())
				return
			}
			if err := requestBroker.lock(); err != nil {
				fail(w, err)
				return
			}
			_, err := requestBroker.withState(false, func(st *trustState, p TrustSecretPolicy, now time.Time) (any, error) {
				id, e := findTrustIdentityAt(st, token, now)
				if e != nil || !trustSecretAdmin(id) || !bool(p.Enabled) || requestBroker.readyRevision != st.Revision {
					return nil, trustSecretDenied()
				}
				_, err := trustSecretDeliveryContext(st)
				return nil, err
			})
			requestBroker.mu.Unlock()
			if err != nil {
				fail(w, err)
				return
			}
			r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
			raw, err := io.ReadAll(r.Body)
			if err != nil {
				clear(raw)
				fail(w, trustSecretDenied())
				return
			}
			v, err := requestBroker.load(r.Context(), token, raw)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
		raw, err := io.ReadAll(r.Body)
		if err != nil {
			fail(w, trustSecretDenied())
			return
		}
		cmd, err := trustSecretObject(raw)
		if err != nil {
			fail(w, err)
			return
		}
		if op == "secret-ready" {
			if cmd != (trustSecretRequest{}) {
				fail(w, trustSecretDenied())
				return
			}
			v, err := requestBroker.prepare(r.Context(), token)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		v, err := requestBroker.operation(r.Context(), op, cmd, token)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, v)
	})
}
