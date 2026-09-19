//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"syscall"
)

type PolicyChange struct {
	Version  int                     `json:"version"`
	Expected string                  `json:"expected_digest"`
	Policies map[string]*ModelPolicy `json:"policies"`
	Preview  bool                    `json:"preview"`
}

func (s *Store) providerAdminState() (map[string]any, error) {
	path, e := localFile(s.root, ".swarm/providers.json")
	if e != nil {
		return nil, e
	}
	raw, e := os.ReadFile(path)
	if e != nil {
		return nil, e
	}
	var ps Providers
	if e = strict(raw, &ps); e != nil {
		return nil, e
	}
	if ps.Schema != 1 {
		return nil, fmt.Errorf("Version fournisseurs inconnue.")
	}
	rows := map[string]any{}
	policies := map[string]*ModelPolicy{}
	for id, p := range ps.Providers {
		policy := effectiveModelPolicy(p)
		if policy != nil {
			policies[id] = policy
		}
		health := "Exécutable disponible ; authentification et modèle à tester."
		info, err := os.Stat(p.Command)
		if err != nil || !info.Mode().IsRegular() || info.Mode()&0111 == 0 {
			health = "Exécutable indisponible."
		}
		support := ""
		if _, err = assistantProvider(p); err != nil {
			support = err.Error()
		}
		rows[id] = map[string]any{"adapter": providerAdapter(p), "command": p.Command, "policy": policy, "models": modelOptions(p), "saved": p.ModelPolicy != nil, "health": health, "assistant_unavailable": support}
	}
	return map[string]any{"version": 1, "digest": hash(raw), "providers": rows, "export": map[string]any{"version": 1, "policies": policies}, "scope": "Administration de l’opérateur local authentifié. Les secrets et permissions des exécutables restent dans leur configuration d’origine."}, nil
}
func (s *Store) changePolicies(change PolicyChange) (map[string]any, error) {
	if change.Version != 1 || len(change.Policies) == 0 || len(change.Policies) > 32 {
		return nil, fmt.Errorf("Document de politiques version 1 requis, avec 1 à 32 fournisseurs.")
	}
	lock, e := os.OpenFile(filepath.Join(s.root, ".swarm", "providers.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if e != nil {
		return nil, e
	}
	defer lock.Close()
	if e = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); e != nil {
		return nil, e
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	path, e := localFile(s.root, ".swarm/providers.json")
	if e != nil {
		return nil, e
	}
	before, e := os.ReadFile(path)
	if e != nil {
		return nil, e
	}
	if change.Expected == "" || change.Expected != hash(before) {
		return nil, &CommandError{Code: "revision_conflict", Message: "Les fournisseurs ont changé ; actualiser et examiner à nouveau les politiques.", Retryable: true}
	}
	var ps Providers
	if e = strict(before, &ps); e != nil {
		return nil, e
	}
	for id, policy := range change.Policies {
		p, ok := ps.Providers[id]
		if !ok {
			return nil, fmt.Errorf("Fournisseur non configuré : %s.", id)
		}
		if e = validateModelPolicy(p, policy); e != nil {
			return nil, fmt.Errorf("%s : %w", id, e)
		}
		p.ModelPolicy = policy
		ps.Providers[id] = p
	}
	after, e := json.MarshalIndent(ps, "", "  ")
	if e != nil {
		return nil, e
	}
	if len(after) > 65536 {
		return nil, fmt.Errorf("Configuration supérieure à 64 Kio.")
	}
	result := map[string]any{"preview": change.Preview, "policies": change.Policies, "before_digest": hash(before), "after_digest": hash(after), "applied": false}
	if change.Preview {
		return result, nil
	}
	// Durable prior configuration is retained before the atomic replacement.
	backup := filepath.Join(s.root, ".swarm", "providers-before-"+hash(before)+".json")
	if e = atomicWrite(backup, before); e != nil {
		return nil, e
	}
	if e = atomicWrite(path, after); e != nil {
		return nil, e
	}
	result["applied"] = true
	result["saved_at"] = now()
	// Audit is best-effort after the primary atomic commit; never misreport an
	// applied configuration as a failed mutation just because its audit failed.
	audit, _ := json.Marshal(map[string]any{"at": now(), "actor": operatorIdentity(), "before": hash(before), "after": hash(after), "policies": change.Policies})
	f, err := os.OpenFile(filepath.Join(s.root, ".swarm", "provider-policy-events.jsonl"), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0600)
	if err == nil {
		_, err = f.Write(append(audit, '\n'))
		if err == nil {
			err = f.Sync()
		}
		_ = f.Close()
	}
	if err != nil {
		result["audit_warning"] = "Politique enregistrée ; journal d’administration indisponible : " + err.Error()
	}
	return result, nil
}
func (s *Store) registerProviderAdmin(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	s.registerAIConnections(mux, send, fail)
	mux.HandleFunc("/api/v1/providers/admin", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		state, e := s.providerAdminState()
		if e != nil {
			fail(w, e)
			return
		}
		send(w, state)
	})
	mux.HandleFunc("/api/v1/providers/resolve", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "GET" {
			http.Error(w, "GET requis", 405)
			return
		}
		ps, e := s.providers()
		if e != nil {
			fail(w, e)
			return
		}
		p, ok := ps.Providers[r.URL.Query().Get("provider")]
		if !ok {
			fail(w, fmt.Errorf("Fournisseur inconnu."))
			return
		}
		purpose := r.URL.Query().Get("purpose")
		if purpose != "page" && purpose != "work" && purpose != "brainstorm" && purpose != "preparation" && purpose != "planning" {
			fail(w, fmt.Errorf("Usage inconnu."))
			return
		}
		_, route, e := resolveModel(p, r.URL.Query().Get("level"), purpose)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, map[string]any{"route": route})
	})
	mux.HandleFunc("/api/v1/providers/policies", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != "POST" {
			http.Error(w, "POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if e != nil {
			fail(w, e)
			return
		}
		var change PolicyChange
		if e = strict(raw, &change); e != nil {
			fail(w, e)
			return
		}
		result, e := s.changePolicies(change)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, result)
	})
}
