//go:build linux

package engine

import (
	"io"
	"net/http"
	"strings"
	"time"
)

func registerTrustActionsHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/trust-actions", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet {
			v, err := s.trustView()
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if r.Method != http.MethodPost {
			http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
			return
		}
		r.Body = http.MaxBytesReader(w, r.Body, 1<<20)
		raw, err := io.ReadAll(r.Body)
		if err != nil {
			fail(w, err)
			return
		}
		action := r.URL.Query().Get("action")
		requestStore := s
		switch action {
		case "kernel-observe", "kernel-check", "identity-context", "oidc-start", "mandate-create", "mandate-revoke", "request", "approve", "commit", "reconcile", "direct-write", "elevate-integrity":
			requestStore, err = s.trustServiceScope(r.Context(), trustHTTPBearer(r.Header.Get("Authorization")))
			if err != nil {
				fail(w, err)
				return
			}
		}
		if action == "workload-introspection-key" {
			var empty struct{}
			if err := strict(raw, &empty); err != nil {
				fail(w, err)
				return
			}
			v, err := s.trustIntrospectionKey(trustHTTPBearer(r.Header.Get("Authorization")))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "confinement-check" {
			v, err := s.trustSealedCheck(r.Context(), raw)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "kernel-observe" {
			w.Header().Set("Cache-Control", "no-store")
			value, err := requestStore.trustKernelObserve(r.Context(), raw, trustHTTPBearer(r.Header.Get("Authorization")))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, value)
			return
		}
		if action == "kernel-check" {
			w.Header().Set("Cache-Control", "no-store")
			v, err := requestStore.trustKernelCheck(r.Context(), raw, trustHTTPBearer(r.Header.Get("Authorization")))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "identity-context" {
			w.Header().Set("Cache-Control", "no-store")
			v, err := requestStore.trustIdentityContext(raw, trustHTTPBearer(r.Header.Get("Authorization")), time.Time{})
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "oidc-backchannel-logout" {
			v, err := s.trustOIDCLogoutOperation(r.Context(), raw)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "workload-login" || action == "workload-logout" {
			w.Header().Set("Cache-Control", "no-store")
			v, err := s.trustWorkloadOperation(r.Context(), action, raw)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "oidc-start" || action == "oidc-finish" || action == "oidc-logout" || action == "oidc-session" {
			w.Header().Set("Cache-Control", "no-store")
			v, err := requestStore.trustOIDCOperation(r.Context(), action, raw, trustHTTPBearer(r.Header.Get("Authorization")))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "config-preview" || action == "config-apply" {
			var change TrustConfigChange
			if err := strict(raw, &change); err != nil {
				fail(w, err)
				return
			}
			v, err := s.trustConfig(change, action == "config-preview")
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "export" {
			v, err := s.trustExport()
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		if action == "anchor" {
			v, err := s.trustAnchor()
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		var cmd TrustCommand
		if err := strict(raw, &cmd); err != nil {
			fail(w, err)
			return
		}
		if action == "verify" {
			v, err := verifyTrustCommand(cmd)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, v)
			return
		}
		authorization := r.Header.Get("Authorization")
		if !strings.HasPrefix(authorization, "Bearer ") || strings.TrimSpace(authorization[7:]) == "" || strings.ContainsAny(authorization[7:], " \t\r\n") {
			fail(w, &CommandError{Code: "authorization_required", Message: "Authorization: Bearer <jeton> requis"})
			return
		}
		token := authorization[7:]
		cmd.original = append([]byte{}, raw...)
		v, err := requestStore.trustOperate(action, cmd, token, time.Time{})
		if err != nil {
			fail(w, err)
			return
		}
		send(w, v)
	})
}

func trustHTTPBearer(authorization string) string {
	if !strings.HasPrefix(authorization, "Bearer ") || strings.TrimSpace(authorization[7:]) == "" || strings.ContainsAny(authorization[7:], " \t\r\n") {
		return ""
	}
	return authorization[7:]
}
