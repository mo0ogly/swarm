//go:build linux

package engine

import (
	"io"
	"net/http"
)

func registerGraphDraftHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	decode := func(w http.ResponseWriter, r *http.Request, value any) error {
		raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if err != nil {
			return err
		}
		return strict(raw, value)
	}
	mux.HandleFunc("/api/v1/graph-drafts", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet {
			value, err := s.getGraphDraft(operatorIdentity(), r.URL.Query().Get("work"), r.URL.Query().Get("draft"))
			if err != nil {
				fail(w, err)
				return
			}
			send(w, value)
			return
		}
		if r.Method != http.MethodPost {
			http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
			return
		}
		var request GraphDraftSaveRequest
		if err := decode(w, r, &request); err != nil {
			fail(w, graphDraftError("invalid_input", err.Error()))
			return
		}
		value, err := s.saveGraphDraft(operatorIdentity(), request)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/graph-drafts/preview", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var request GraphDraftPreviewRequest
		if err := decode(w, r, &request); err != nil {
			fail(w, graphDraftError("invalid_input", err.Error()))
			return
		}
		value, err := s.previewGraphDraft(operatorIdentity(), request)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/graph-drafts/apply", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var request GraphDraftApplyRequest
		if err := decode(w, r, &request); err != nil {
			fail(w, graphDraftError("invalid_input", err.Error()))
			return
		}
		value, err := s.applyGraphDraft(operatorIdentity(), request)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
}
