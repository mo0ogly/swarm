//go:build linux

package engine

import (
	"context"
	"crypto/x509"
	"encoding/json"
	"errors"
	"net"
	"net/http"
	"net/http/httptest"
	"strings"
	"syscall"
	"testing"
	"time"
)

func TestAIConnectionNetworkErrors(t *testing.T) {
	for _, tc := range []struct {
		err  error
		want string
	}{
		{&net.DNSError{Err: "secret", Name: "secret"}, "DNS"},
		{x509.UnknownAuthorityError{}, "TLS non reconnu"},
		{x509.HostnameError{}, "nom du serveur"},
		{syscall.ECONNREFUSED, "TCP refusée"},
		{context.DeadlineExceeded, "Délai dépassé"},
		{errors.New("Redirection refusée secret"), "Redirection HTTP"},
		{errors.New("proxy password secret"), "transport HTTP"},
	} {
		e := aiConnectionNetworkError(tc.err)
		if !strings.Contains(e.Error(), tc.want) || strings.Contains(e.Error(), "secret") {
			t.Fatal(e)
		}
	}
}
func TestAIConnectionDebugWire(t *testing.T) {
	for _, status := range []int{200, 401, 404, 307} {
		target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			if status == 307 {
				w.Header().Set("Location", "http://secret.invalid")
			}
			w.WriteHeader(status)
			if status == 200 {
				_, _ = w.Write([]byte(`{"choices":[{"message":{"content":"OK"}}]}`))
			} else {
				_, _ = w.Write([]byte("private upstream secret"))
			}
		}))
		c := AIConnection{ID: "test", Label: "Test", Model: "local", BaseURL: target.URL, Key: "secret"}
		d := &aiConnectionDebug{start: time.Now()}
		reply, err := callAIConnectionDebug(context.Background(), c, "Test", "", d)
		if status == 200 && (err != nil || reply != "OK") {
			t.Fatal(reply, err)
		}
		if status != 200 && err == nil {
			t.Fatal("failure accepted")
		}
		b, _ := json.Marshal(d.snapshot(c.Key))
		if strings.Contains(string(b), "secret") || strings.Contains(string(b), "private upstream") {
			t.Fatal(string(b))
		}
		if !strings.Contains(string(b), "tcp") || !strings.Contains(string(b), "response") {
			t.Fatal("missing trace", string(b))
		}
		target.Close()
	}
	target := httptest.NewTLSServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {}))
	defer target.Close()
	_, err := callAIConnection(context.Background(), AIConnection{ID: "test", Label: "Test", Model: "local", BaseURL: target.URL}, "Test", "")
	if err == nil || !strings.Contains(err.Error(), "TLS non reconnu") {
		t.Fatal(err)
	}
}
