//go:build linux && amd64

package engine

import (
	"errors"
	"os"
	"strings"
	"testing"
	"time"
)

type observeBackendFixture struct {
	records                              []kernelBackendRecord
	stats                                kernelBackendStats
	readErr, stopErr, statsErr, closeErr error
	closed, stopped                      bool
	deadline                             time.Time
}

func (b *observeBackendFixture) Read() (kernelBackendRecord, error) {
	if len(b.records) == 0 {
		if b.readErr != nil {
			return kernelBackendRecord{}, b.readErr
		}
		return kernelBackendRecord{}, os.ErrDeadlineExceeded
	}
	r := b.records[0]
	b.records = b.records[1:]
	return r, nil
}
func (b *observeBackendFixture) SetDeadline(t time.Time)            { b.deadline = t }
func (b *observeBackendFixture) Stop() error                        { b.stopped = true; return b.stopErr }
func (b *observeBackendFixture) Stats() (kernelBackendStats, error) { return b.stats, b.statsErr }
func (b *observeBackendFixture) Close() error                       { b.closed = true; return b.closeErr }
func TestKernelObserveCounterAndCleanup(t *testing.T) {
	for _, variant := range []string{"limit", "window", "quiet", "read", "stop", "stats", "close", "emit", "sequence", "counters"} {
		t.Run(variant, func(t *testing.T) {
			p := kernelObservePolicy(nil)
			p.Enabled = true
			p.MaxEvents = 1
			l := kernelObservationLease{AgentID: "agent", Mission: "mission", Task: "task", Attempt: "attempt", PID: 123, Start: "42", ResourceDigest: strings.Repeat("a", 64)}
			b := &observeBackendFixture{records: []kernelBackendRecord{{Sequence: 2, KtimeNS: 1, Kind: "file"}}, stats: kernelBackendStats{Attempts: 4, Emitted: 3, Lost: 1}}
			fault := errors.New("fixture failure")
			switch variant {
			case "window":
				p.MaxEvents = 2
			case "quiet":
				b.records = nil
				b.stats = kernelBackendStats{}
			case "read":
				b.records = nil
				b.readErr = fault
			case "stop":
				b.stopErr = fault
			case "stats":
				b.statsErr = fault
			case "close":
				b.closeErr = fault
			case "sequence":
				b.records[0].Sequence = 0
			case "counters":
				b.stats.Emitted = 0
			}
			events := []kernelObservation{}
			r, err := collectKernelObservedEvents(b, p, l, strings.Repeat("b", 32), func(e kernelObservation) error {
				if variant == "emit" {
					return fault
				}
				events = append(events, e)
				return nil
			})
			good := variant == "limit" || variant == "window" || variant == "quiet"
			if (err == nil) != good || !b.closed || b.deadline.IsZero() {
				t.Fatalf("result %v closed %v", err, b.closed)
			}
			if good {
				r.Events = events
				if !b.stopped || validateKernelObserveResult(r, p, l) != nil {
					t.Fatalf("invalid bounded result: %+v", r)
				}
				if variant == "limit" && (r.Omitted != 2 || r.KernelLost != 1 || events[0].LostBefore != 1 || r.StopReason != "event_limit") {
					t.Fatal("loss or truncation hidden")
				}
				if r.Complete || r.GlobalQualification {
					t.Fatal("partial result overqualified")
				}
			}
			if variant == "close" && r.CleanupVerified {
				t.Fatal("cleanup failure hidden")
			}
		})
	}
}
