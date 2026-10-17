//go:build linux

package engine

import (
	"fmt"
	"io"
	"net/http"
	"strings"
)

// JSON transport capacity, shared by HTTP, CLI and Go; no retention policy.
const projectMemoryTransportBytes = 1 << 20

func (s *Store) projectMemoryCatalog(query string) (any, error) {
	works, err := memoryWorks(s.db)
	if err != nil {
		return nil, err
	}
	uses, err := memoryUses(s.db)
	if err != nil {
		return nil, err
	}
	entries := []map[string]any{}
	query = foldDialogue(strings.TrimSpace(query))
	for _, w := range works {
		for _, entry := range w.ProjectMemory {
			for i, v := range entry.Versions {
				status, reason := "historical", "Version historique, non sélectionnable"
				if i == len(entry.Versions)-1 {
					status, reason = "active", "Version courante ; conditions à revérifier"
					if e := s.validateMemoryVersion(w, entry, v); e != nil {
						status, reason = "unavailable", e.Error()
					}
				}
				if query != "" && !strings.Contains(foldDialogue(v.Body+"\n"+v.Conditions), query) {
					continue
				}
				ref := MemoryReference{w.ID, entry.ID, v.Version, v.SHA256}
				usedBy := []MemoryUse{}
				for _, use := range uses {
					if use.Reference == ref {
						usedBy = append(usedBy, use)
					}
				}
				entries = append(entries, map[string]any{"ref": ref, "version": v, "status": status, "reason": reason, "work_revision": w.Revision, "source_title": w.Title, "used_by": usedBy})
			}
		}
	}
	return map[string]any{"entries": entries, "method": "literal", "query": query}, nil
}

func registerProjectMemoryHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/project-memory", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Cache-Control", "no-store")
		if r.URL.Query().Get("action") == "summary-source" && r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		if r.Method == http.MethodGet {
			v, e := s.projectMemoryCatalog(r.URL.Query().Get("q"))
			if e != nil {
				fail(w, e)
			} else {
				send(w, v)
			}
			return
		}
		if r.Method != http.MethodPost {
			http.Error(w, "GET ou POST requis", 405)
			return
		}
		raw, e := io.ReadAll(http.MaxBytesReader(w, r.Body, projectMemoryTransportBytes))
		if e != nil {
			fail(w, e)
			return
		}
		if r.URL.Query().Get("action") == "summary-source" {
			v, err := s.memorySummarySource(raw)
			if err != nil {
				fail(w, err)
			} else {
				send(w, v)
			}
			return
		}
		var q ProjectMemoryRequest
		if e = strict(raw, &q); e != nil {
			fail(w, e)
			return
		}
		v, e := s.projectMemoryOperation(r.URL.Query().Get("action"), q)
		if e != nil {
			fail(w, e)
			return
		}
		send(w, v)
	})
}

func (s *Store) projectMemoryCLI(pos []string, input string, out io.Writer) error {
	if len(pos) == 2 && pos[1] == "list" {
		v, e := s.projectMemoryCatalog("")
		if e != nil {
			return e
		}
		return printJSON(out, v)
	}
	if len(pos) != 2 {
		return fmt.Errorf("swarm memory list|summary-source|admit-preview|admit|replace-preview|replace|withdraw-preview|withdraw|select-preview|select --input demande.json")
	}
	raw, e := readInput(input)
	if e != nil {
		return e
	}
	if pos[1] == "summary-source" {
		v, err := s.memorySummarySource(raw)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	var q ProjectMemoryRequest
	if e = strict(raw, &q); e != nil {
		return e
	}
	v, e := s.projectMemoryOperation(pos[1], q)
	if e != nil {
		return e
	}
	return printJSON(out, v)
}
