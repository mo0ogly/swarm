package engine

import (
	"io"
	"net/http"
)

func (s *Store) registerStorageRetryAdmin(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/storage-retry", func(w http.ResponseWriter, r *http.Request) {
		switch r.Method {
		case "GET":
			status, err := s.storageRetryStatus()
			if err != nil {
				fail(w, err)
				return
			}
			send(w, status)
		case "POST":
			raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 4096))
			if err != nil {
				fail(w, err)
				return
			}
			var policy StorageRetryPolicy
			if err = strict(raw, &policy); err != nil {
				fail(w, err)
				return
			}
			status, err := s.saveStorageRetryPolicy(policy)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, status)
		default:
			http.Error(w, "GET ou POST requis", http.StatusMethodNotAllowed)
		}
	})
}
