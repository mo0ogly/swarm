//go:build linux

package engine

import (
	"io"
	"net/http"
	"strconv"
)

func registerAutomationHTTP(s *Store, mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	service := s.automationService()
	decode := func(w http.ResponseWriter, r *http.Request, value any) error {
		raw, err := io.ReadAll(http.MaxBytesReader(w, r.Body, 65536))
		if err != nil {
			return err
		}
		return strict(raw, value)
	}
	mux.HandleFunc("/api/v1/automation/programs", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(w, "GET requis", http.StatusMethodNotAllowed)
			return
		}
		id := r.URL.Query().Get("id")
		if id != "" {
			value, err := service.Show(id)
			if err != nil {
				fail(w, err)
				return
			}
			send(w, value)
			return
		}
		value, err := service.List()
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/automation/preview", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var request AutomationScheduleCreate
		if err := decode(w, r, &request); err != nil {
			fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
			return
		}
		count := 5
		if raw := r.URL.Query().Get("count"); raw != "" {
			if value, err := strconv.Atoi(raw); err == nil {
				count = value
			}
		}
		value, err := service.Preview(request, count, nil)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/automation/create", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var command AutomationCreateCommand
		if err := decode(w, r, &command); err != nil {
			fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
			return
		}
		value, err := service.Create(command, nil)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/automation/state", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var command AutomationStateCommand
		if err := decode(w, r, &command); err != nil {
			fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
			return
		}
		value, err := service.SetState(command, nil)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/automation/cancel", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodPost {
			http.Error(w, "POST requis", http.StatusMethodNotAllowed)
			return
		}
		var command AutomationCancelCommand
		if err := decode(w, r, &command); err != nil {
			fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
			return
		}
		value, err := service.Cancel(command)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
	mux.HandleFunc("/api/v1/automation/config", func(w http.ResponseWriter, r *http.Request) {
		if r.Method == http.MethodGet {
			value, err := s.automationConfig()
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
		var command AutomationConfigCommand
		if err := decode(w, r, &command); err != nil {
			fail(w, &CommandError{Code: "invalid_input", Message: err.Error()})
			return
		}
		value, err := service.ApplyConfig(command)
		if err != nil {
			fail(w, err)
			return
		}
		send(w, value)
	})
}
