//go:build linux

package engine

import (
	"context"
	"crypto/tls"
	"crypto/x509"
	"errors"
	"fmt"
	"net"
	"net/http/httptrace"
	"strings"
	"sync"
	"syscall"
	"time"
)

type aiConnectionDebugEvent struct {
	ElapsedMS int64  `json:"elapsed_ms"`
	Stage     string `json:"stage"`
	Message   string `json:"message"`
}
type aiConnectionDebug struct {
	mu     sync.Mutex
	start  time.Time
	events []aiConnectionDebugEvent
}

func (d *aiConnectionDebug) add(stage, message string) {
	if d == nil {
		return
	}
	d.mu.Lock()
	defer d.mu.Unlock()
	if len(d.events) < 64 {
		d.events = append(d.events, aiConnectionDebugEvent{time.Since(d.start).Milliseconds(), stage, message})
	}
}
func (d *aiConnectionDebug) snapshot(key string) []aiConnectionDebugEvent {
	d.mu.Lock()
	defer d.mu.Unlock()
	out := append([]aiConnectionDebugEvent{}, d.events...)
	for i := range out {
		if key != "" {
			out[i].Message = strings.ReplaceAll(out[i].Message, key, "[secret masqué]")
		}
	}
	return out
}
func (d *aiConnectionDebug) trace(ctx context.Context) context.Context {
	if d == nil {
		return ctx
	}
	return httptrace.WithClientTrace(ctx, &httptrace.ClientTrace{
		DNSStart: func(i httptrace.DNSStartInfo) { d.add("dns", fmt.Sprintf("Résolution DNS : %s", i.Host)) },
		DNSDone: func(i httptrace.DNSDoneInfo) {
			if i.Err != nil {
				d.add("dns", aiConnectionNetworkError(i.Err).Error())
			} else {
				d.add("dns", fmt.Sprintf("DNS résolu : %d adresse(s)", len(i.Addrs)))
			}
		},
		ConnectStart: func(network, addr string) { d.add("tcp", fmt.Sprintf("Connexion TCP : %s", addr)) },
		ConnectDone: func(network, addr string, err error) {
			if err != nil {
				d.add("tcp", aiConnectionNetworkError(err).Error())
			} else {
				d.add("tcp", "Connexion TCP établie")
			}
		},
		TLSHandshakeStart: func() { d.add("tls", "Vérification du certificat TLS") },
		TLSHandshakeDone: func(_ tls.ConnectionState, err error) {
			if err != nil {
				d.add("tls", aiConnectionNetworkError(err).Error())
			} else {
				d.add("tls", "Certificat TLS accepté")
			}
		},
		GotConn: func(i httptrace.GotConnInfo) {
			d.add("connection", fmt.Sprintf("Connexion disponible ; réutilisée : %v", i.Reused))
		},
		WroteRequest: func(i httptrace.WroteRequestInfo) {
			if i.Err != nil {
				d.add("send", aiConnectionNetworkError(i.Err).Error())
			} else {
				d.add("send", "Requête envoyée ; attente de la réponse")
			}
		},
		GotFirstResponseByte: func() { d.add("response", "Premier octet reçu") },
	})
}

// Never expose raw transport errors: they can contain proxy credentials or URLs.
func aiConnectionNetworkError(err error) error {
	var dns *net.DNSError
	var unknown x509.UnknownAuthorityError
	var hostname x509.HostnameError
	var invalid x509.CertificateInvalidError
	var verify *tls.CertificateVerificationError
	var network net.Error
	switch {
	case errors.Is(err, context.Canceled):
		return fmt.Errorf("Appel annulé.")
	case errors.Is(err, context.DeadlineExceeded):
		return fmt.Errorf("Délai dépassé ; le serveur n’a pas terminé la réponse dans le délai autorisé.")
	case errors.As(err, &unknown):
		return fmt.Errorf("Certificat TLS non reconnu ; installez l’autorité interne sur la machine ou dans le conteneur qui exécute Swarm.")
	case errors.As(err, &hostname):
		return fmt.Errorf("Certificat TLS incompatible avec le nom du serveur.")
	case errors.As(err, &invalid):
		return fmt.Errorf("Certificat TLS invalide ou expiré ; vérifiez le certificat et l’horloge du serveur Swarm.")
	case errors.As(err, &verify):
		return fmt.Errorf("Échec de vérification du certificat TLS sur le serveur Swarm.")
	case errors.As(err, &dns):
		return fmt.Errorf("Résolution DNS impossible depuis le serveur Swarm ; vérifiez son DNS et son accès au réseau interne.")
	case errors.Is(err, syscall.ECONNREFUSED):
		return fmt.Errorf("Connexion TCP refusée ; vérifiez le port et le service depuis le serveur Swarm.")
	case errors.As(err, &network) && network.Timeout():
		return fmt.Errorf("Délai réseau dépassé depuis le serveur Swarm ; vérifiez le routage, le proxy et le pare-feu.")
	case strings.Contains(err.Error(), "Redirection refusée"):
		return fmt.Errorf("Redirection HTTP refusée ; renseignez directement l’adresse finale de l’API.")
	default:
		return fmt.Errorf("Échec du transport HTTP depuis le serveur Swarm ; vérifiez son réseau, son proxy et la compatibilité TLS. Consultez la dernière étape du diagnostic.")
	}
}
