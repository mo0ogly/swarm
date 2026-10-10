//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

// TestSQLiteQualificationMeasurement is copied unchanged into the Git baseline
// by tests/sqlite_contention_final.py. It applies the same bounded contention to
// the real independent-review service and records effects instead of assuming
// that process completion implies recovery.
func TestSQLiteQualificationMeasurement(t *testing.T) {
	const samples = 9
	type observation struct {
		DurationMS         float64 `json:"duration_ms"`
		InitialStorageBusy bool    `json:"initial_storage_busy"`
		FinalState         string  `json:"final_state"`
		ReviewCalls        int     `json:"review_calls"`
		Attempts           int     `json:"attempts"`
	}
	result := struct {
		Schema   int           `json:"schema_version"`
		Workload string        `json:"workload"`
		Samples  []observation `json:"samples"`
	}{Schema: 1, Workload: "independent-review/result-persistence; retries=0; busy_timeout_ms=1; one held writer; one causal resume"}

	for i := 0; i < samples; i++ {
		s, w, command := sqliteAuditReviewFixture(t, true)
		sqliteAuditRetryPolicy(t, s, 0, 0, 1)
		data, err := os.ReadFile(command)
		if err != nil {
			t.Fatal(err)
		}
		data = []byte(strings.Replace(string(data), "cat >\"$0.prompt\"", "cat >\"$0.prompt\"\nprintf x >>\"$0.calls\"", 1))
		if err = os.WriteFile(command, data, 0700); err != nil {
			t.Fatal(err)
		}

		done := make(chan error, 1)
		started := time.Now()
		go func() { done <- s.independentReviewStep(w.ID) }()
		sqliteAuditWaitFile(t, command+".entered")
		_, tx := sqliteAuditWriter(t, s, w.ID)
		if err = os.WriteFile(command+".release", []byte("go"), 0600); err != nil {
			t.Fatal(err)
		}
		firstErr := <-done
		initialBusy := firstErr != nil && strings.Contains(firstErr.Error(), "SQLITE_BUSY")
		if !initialBusy {
			t.Fatalf("sample %d: expected bounded SQLITE_BUSY, got %v", i, firstErr)
		}
		if err = tx.Rollback(); err != nil {
			t.Fatal(err)
		}
		if err = s.independentReviewStep(w.ID); err != nil {
			t.Fatal(err)
		}
		got, err := s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		task, err := got.task(w.Tasks[0].ID)
		if err != nil || task.IndependentReview == nil {
			t.Fatalf("sample %d: missing review after causal resume: %v", i, err)
		}
		calls, err := os.ReadFile(command + ".calls")
		if err != nil {
			t.Fatal(err)
		}
		if got.Planning.Reviewer.Calls != 1 || len(calls) != 1 || len(task.Attempts) != 1 {
			t.Fatalf("sample %d: duplicate business effect: reviewer=%d provider=%d attempts=%d", i, got.Planning.Reviewer.Calls, len(calls), len(task.Attempts))
		}
		result.Samples = append(result.Samples, observation{
			DurationMS:         float64(time.Since(started).Microseconds()) / 1000,
			InitialStorageBusy: initialBusy,
			FinalState:         task.IndependentReview.State,
			ReviewCalls:        got.Planning.Reviewer.Calls,
			Attempts:           len(task.Attempts),
		})
	}

	path := os.Getenv("SQLITE_QUALIFICATION_OUTPUT")
	if path == "" {
		for i, sample := range result.Samples {
			if sample.FinalState != "passed" {
				t.Fatalf("sample %d: candidate did not persist the review: state=%s", i, sample.FinalState)
			}
		}
		return
	}
	raw, err := json.MarshalIndent(result, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(path, append(raw, '\n'), 0600); err != nil {
		t.Fatal(err)
	}
}
