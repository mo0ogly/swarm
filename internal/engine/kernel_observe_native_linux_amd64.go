//go:build linux && amd64

package engine

import (
	"errors"
	"os"
	"time"
)

type kernelObserveBackend interface {
	Read() (kernelBackendRecord, error)
	SetDeadline(time.Time)
	Stop() error
	Stats() (kernelBackendStats, error)
	Close() error
}
type kernelObserveNativeBackend struct{ *kernelBackendNative }

func (b kernelObserveNativeBackend) SetDeadline(t time.Time) { b.reader.SetDeadline(t) }

func collectKernelObservedEvents(b kernelObserveBackend, p TrustKernelObservePolicy, l kernelObservationLease, boot string, emit func(kernelObservation) error) (result TrustKernelObserveResult, err error) {
	result = kernelObserveFixed(p, l)
	result.Available = true
	result.StopReason = "window"
	defer func() {
		closeErr := b.Close()
		if closeErr != nil {
			result.CleanupVerified = false
			err = errors.Join(err, closeErr)
		}
	}()
	end := time.Now().Add(time.Duration(p.WindowSeconds) * time.Second)
	b.SetDeadline(end)
	var previous uint64
	sent := 0
	for sent < p.MaxEvents && time.Now().Before(end) {
		record, e := b.Read()
		if errors.Is(e, os.ErrDeadlineExceeded) {
			break
		}
		if e != nil {
			return result, e
		}
		if record.Sequence <= previous {
			return result, errors.New("kernel collector sequence regressed")
		}
		event := kernelObservation{CollectorBootID: boot, Sequence: record.Sequence, KtimeNS: record.KtimeNS, EventType: record.Kind, PID: l.PID, PIDStartTime: l.Start, ParentPID: 0, CgroupID: record.CgroupID, MissionID: l.Mission, TaskID: l.Task, AttemptID: l.Attempt, ResourceDigest: l.ResourceDigest, LostBefore: record.Sequence - previous - 1, ProbeSet: "swarm-minimal-v1"}
		if e = emit(event); e != nil {
			return result, e
		}
		previous = record.Sequence
		sent++
	}
	if sent == p.MaxEvents {
		result.StopReason = "event_limit"
	}
	if err = b.Stop(); err != nil {
		return result, err
	}
	stats, e := b.Stats()
	if e != nil {
		return result, e
	}
	if stats.Emitted < uint64(sent) {
		return result, errors.New("kernel collector counters precede received events")
	}
	result.Attempts = stats.Attempts
	result.Emitted = stats.Emitted
	result.KernelLost = stats.Lost
	result.Omitted = stats.Emitted - uint64(sent)
	result.CleanupVerified = true
	return result, nil
}

func runKernelObserveNative(p TrustKernelObservePolicy, l kernelObservationLease, boot string, fresh func() (kernelObservationLease, error), emit func(kernelObservation) error) (TrustKernelObserveResult, error) {
	b, err := newKernelBackendNative(l, uint32(p.RingBytes), fresh)
	if err != nil {
		r := kernelObserveFixed(p, l)
		r.StopReason = "unavailable"
		r.Diagnostic = err.Error()
		r.CleanupVerified = kernelBackendCleanAfterFailure(err)
		return r, nil
	}
	return collectKernelObservedEvents(kernelObserveNativeBackend{b}, p, l, boot, emit)
}
