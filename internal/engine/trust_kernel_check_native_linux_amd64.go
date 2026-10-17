//go:build linux && amd64

package engine

import (
	"errors"
	"os"
	"os/exec"
	"syscall"

	"github.com/cilium/ebpf/ringbuf"
	"golang.org/x/sys/unix"
)

func runKernelOwnedFixture(p TrustKernelCheckPolicy, nonce, socket string) (r TrustKernelCheckResult) {
	r = kernelCheckFixedResult()
	r.CleanupVerified = true
	r.EffectivePolicy = p
	r.OwnedPIDs = []int{os.Getpid()}
	if unix.Gettid() != os.Getpid() {
		r.Reason = "main_thread_unavailable"
		return r
	}
	start, err := kernelObservationProcessStart(os.Getpid())
	if err != nil {
		r.Reason = "source_unavailable"
		return r
	}
	lease := kernelCheckLease(os.Getpid(), start, nonce)
	root, err := os.MkdirTemp("", "swarm-kernel-fixture-")
	if err != nil {
		r.Reason = "fixture_setup_failed"
		return r
	}
	defer func() {
		if e := os.RemoveAll(root); e != nil {
			r.CleanupVerified = false
			r.FixtureVerified = false
			r.Reason = "cleanup_failed"
			r.Diagnostic = e.Error()
		}
	}()
	if err = os.Chdir(root); err != nil {
		r.Reason = "fixture_setup_failed"
		return r
	}
	exe, err := os.Open("/proc/self/exe")
	if err != nil {
		r.Reason = "fixture_setup_failed"
		return r
	}
	defer func() {
		if e := exe.Close(); e != nil {
			r.CleanupVerified = false
			r.FixtureVerified = false
			r.Reason = "cleanup_failed"
			r.Diagnostic = e.Error()
		}
	}()
	b, err := newKernelBackendNative(lease, uint32(p.RingBytes), func() (kernelObservationLease, error) { return lease, kernelBackendTarget(lease) })
	if err != nil {
		r.CleanupVerified = kernelBackendCleanAfterFailure(err)
		r.Reason = "backend_unavailable"
		if errors.Is(err, unix.EPERM) || errors.Is(err, unix.EACCES) {
			r.Reason = "permission_denied"
		}
		r.Diagnostic = err.Error()
		return r
	}
	r.Available = true
	defer func() {
		if e := b.Close(); e != nil {
			r.CleanupVerified = false
			r.FixtureVerified = false
			r.Reason = "cleanup_failed"
			r.Diagnostic = e.Error()
		}
	}()
	fixtureOK := true
	f, e := os.OpenFile("public-fixture", os.O_CREATE|os.O_EXCL|os.O_RDWR, 0600)
	if e != nil {
		fixtureOK = false
	} else {
		_, e = f.WriteString("public diagnostic fixture")
		if e != nil {
			fixtureOK = false
		}
		if e = f.Close(); e != nil {
			fixtureOK = false
		}
	}
	fd, e := unix.Socket(unix.AF_UNIX, unix.SOCK_STREAM|unix.SOCK_CLOEXEC, 0)
	if e != nil {
		fixtureOK = false
	} else {
		if e = unix.Connect(fd, &unix.SockaddrUnix{Name: socket}); e != nil {
			fixtureOK = false
		}
		if e = unix.Close(fd); e != nil {
			fixtureOK = false
		}
	}
	child := exec.Command("/proc/self/fd/3", kernelCheckExitArgs()...)
	child.ExtraFiles = []*os.File{exe}
	child.Env = []string{"LANG=C", "TZ=UTC"}
	child.SysProcAttr = &syscall.SysProcAttr{Pdeathsig: syscall.SIGKILL}
	if e = child.Run(); e != nil {
		fixtureOK = false
	}
	if err = b.Stop(); err != nil {
		r.Reason = "stop_failed"
		r.Diagnostic = err.Error()
		return r
	}
	stats, err := b.Stats()
	if err != nil {
		r.Reason = "statistics_failed"
		r.Diagnostic = err.Error()
		return r
	}
	r.Lost = stats.Lost
	if err = b.Flush(); err != nil {
		r.Reason = "drain_failed"
		r.Diagnostic = err.Error()
		return r
	}
	var previous uint64
	kinds := map[string]bool{}
	for {
		record, e := b.Read()
		if errors.Is(e, ringbuf.ErrFlushed) {
			break
		}
		if e != nil {
			r.Reason = "invalid_record"
			r.Diagnostic = e.Error()
			return r
		}
		if record.Sequence <= previous {
			r.Reason = "invalid_sequence"
			return r
		}
		event := kernelObservation{CollectorBootID: nonce, Sequence: record.Sequence, KtimeNS: record.KtimeNS, EventType: record.Kind, PID: lease.PID, PIDStartTime: lease.Start, ParentPID: os.Getppid(), CgroupID: record.CgroupID, MissionID: lease.Mission, TaskID: lease.Task, AttemptID: lease.Attempt, ResourceDigest: lease.ResourceDigest, LostBefore: record.Sequence - previous - 1, ProbeSet: "swarm-minimal-v1"}
		r.Events = append(r.Events, event)
		previous = record.Sequence
		kinds[record.Kind] = true
	}
	if err = b.Close(); err != nil {
		r.Reason = "cleanup_failed"
		r.Diagnostic = err.Error()
		return r
	}
	r.FixtureVerified = fixtureOK && uint64(len(r.Events)) == stats.Emitted && stats.Lost == 0 && previous == stats.Attempts && kinds["process"] && kinds["file"] && kinds["network"]
	if !r.FixtureVerified {
		r.Reason = "fixture_unqualified"
	}
	return r
}
