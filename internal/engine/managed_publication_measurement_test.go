//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

// An explicit isolated measurement recipe, not a real-provider benchmark.
// Compare admission levels on identical two-result publication workloads.
func TestManagedPublicationMeasuredConcurrency(t *testing.T) {
	output := os.Getenv("SWARM_PUBLICATION_MEASUREMENT_OUTPUT")
	if output == "" {
		t.Skip("explicit measurement output required")
	}
	type sample struct {
		Concurrency    int      `json:"concurrency"`
		Iteration      int      `json:"iteration"`
		Seconds        float64  `json:"publication_seconds"`
		StagingSeconds *float64 `json:"sibling_staging_seconds,omitempty"`
		Accepted       int      `json:"accepted"`
		Calls          int      `json:"review_fixture_calls"`
		Overlap        bool     `json:"retained_during_sibling_review"`
	}
	samples := []sample{}
	for _, level := range []int{1, 2} {
		for iteration := 1; iteration <= 3; iteration++ {
			s, w := managedFixture(t)
			first := managedCompleted(t, s, w, "first", "initial\n")
			w, err := s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			second := managedCompleted(t, s, w, "second", "initial\n")
			ps, err := s.providers()
			if err != nil {
				t.Fatal(err)
			}
			script := ps.Providers["managed-review-fixture"].Command
			code, err := os.ReadFile(script)
			if err != nil {
				t.Fatal(err)
			}
			// Fixed fixture inference delay, identical in both configurations.
			code = []byte(strings.Replace(string(code), "import json, os, sys", "import json, os, sys, time\nopen(__file__+'.entered','w').close()\ntime.sleep(.08)", 1))
			if err = os.WriteFile(script, code, 0700); err != nil {
				t.Fatal(err)
			}
			other, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			t.Cleanup(func() { _ = other.db.Close() })
			measured := sample{Concurrency: level, Iteration: iteration}
			started := time.Now()
			if level == 1 {
				if err = s.integrateManagedAttempt(first); err != nil {
					t.Fatal(err)
				}
			} else {
				done := make(chan error, 1)
				go func() { done <- s.integrateManagedAttempt(first) }()
				deadline := time.Now().Add(10 * time.Second)
				for {
					if _, err = os.Stat(script + ".entered"); err == nil {
						break
					}
					if time.Now().After(deadline) {
						t.Fatal("fixture review not reached")
					}
					time.Sleep(time.Millisecond)
				}
				stagingStarted := time.Now()
				if err = other.integrateManagedAttempt(second); err != nil {
					t.Fatal(err)
				}
				elapsed := time.Since(stagingStarted).Seconds()
				measured.StagingSeconds = &elapsed
				item, err := other.managedAttempt(second.ID)
				if err != nil || item.Result == "" {
					t.Fatal("sibling result not retained", err)
				}
				mid, err := other.get(w.ID)
				if err != nil {
					t.Fatal(err)
				}
				measured.Overlap = mid.Tasks[0].IndependentReview != nil && mid.Tasks[0].IndependentReview.State == "running" && mid.Tasks[1].IndependentReview == nil
				// Measure, do not require a scheduler timing: a short fixture
				// review can finish before Git retention does. The barrier-based
				// regression separately proves retention during a held review.
				if err = <-done; err != nil {
					t.Fatal(err)
				}
			}
			if err = other.integrateManagedAttempt(second); err != nil {
				t.Fatal(err)
			}
			measured.Seconds = time.Since(started).Seconds()
			final, err := other.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			for _, task := range final.Tasks {
				if task.Status == "accepted" && other.acceptedFresh(&final, &task, map[string]bool{}) {
					measured.Accepted++
				}
			}
			measured.Calls = managedReviewCalls(t, other)
			if measured.Accepted != 2 || measured.Calls != 2 {
				t.Fatal("comparison changed quality or review count", measured)
			}
			samples = append(samples, measured)
		}
	}
	raw, err := json.MarshalIndent(map[string]any{"status": "PASS", "provider": "deterministic local subprocess", "production": "results prepared before measurement; production time not measured", "fixture_inference_delay_seconds": 0.08, "iterations_per_level": 3, "samples": samples, "tokens": nil, "cache": nil, "provider_cost": nil, "limits": "Publication path only, warm host caches uncontrolled. No real model, independent assessment of this code, native acceptance or global speedup claim."}, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	if err = os.WriteFile(output, append(raw, '\n'), 0600); err != nil {
		t.Fatal(err)
	}
}
