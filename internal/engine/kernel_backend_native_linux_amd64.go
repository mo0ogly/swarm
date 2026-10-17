//go:build linux && amd64

package engine

import (
	"errors"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"sync"
	"unsafe"

	"github.com/cilium/ebpf"
	"github.com/cilium/ebpf/ringbuf"
	"golang.org/x/sys/unix"
)

type kernelBackendNative struct {
	mu                   sync.Mutex
	resources            *kernelBackendResources
	ring, gate, counters *ebpf.Map
	reader               *ringbuf.Reader
	perf                 []*os.File
	pid                  int
	stopped              bool
	stopErr              error
}

type kernelBackendStats struct {
	Attempts uint64 `json:"attempts"`
	Emitted  uint64 `json:"emitted"`
	Lost     uint64 `json:"lost"`
}

func kernelBackendTarget(l kernelObservationLease) error {
	if l.Host != hostIdentity() || l.AgentID == "" || l.Mission == "" || l.Task == "" || l.Attempt == "" || !kernelObservationHex(l.ResourceDigest, 32) {
		return errors.New("protected kernel observation source required")
	}
	start, err := kernelObservationProcessStart(l.PID)
	if err != nil {
		return err
	}
	if start != l.Start {
		return errors.New("kernel observation target changed")
	}
	return nil
}

func kernelBackendPIDAlive(fd int) error {
	p := []unix.PollFd{{Fd: int32(fd), Events: unix.POLLIN}}
	n, err := unix.Poll(p, 0)
	if err != nil {
		return err
	}
	if n != 0 || p[0].Revents != 0 {
		return errors.New("kernel observation target exited")
	}
	return nil
}

func kernelBackendTraceID(probe kernelBackendProbe) (uint64, error) {
	for _, root := range []string{"/sys/kernel/tracing/events", "/sys/kernel/debug/tracing/events"} {
		b, err := os.ReadFile(filepath.Join(root, probe.Group, probe.Name, "id"))
		if os.IsNotExist(err) {
			continue
		}
		if err != nil {
			return 0, err
		}
		return strconv.ParseUint(strings.TrimSpace(string(b)), 10, 64)
	}
	return 0, errors.New("fixed tracepoint unavailable")
}

func kernelBackendPerf(pid int, traceID uint64, program *ebpf.Program) (*os.File, error) {
	attr := unix.PerfEventAttr{Type: unix.PERF_TYPE_TRACEPOINT, Size: uint32(unsafe.Sizeof(unix.PerfEventAttr{})), Config: traceID, Sample: 1, Bits: unix.PerfBitDisabled}
	fd, err := unix.PerfEventOpen(&attr, pid, -1, -1, unix.PERF_FLAG_FD_CLOEXEC)
	if err != nil {
		return nil, err
	}
	f := os.NewFile(uintptr(fd), "swarm-owned-tracepoint")
	if err = unix.IoctlSetInt(fd, unix.PERF_EVENT_IOC_SET_BPF, program.FD()); err != nil {
		_ = f.Close()
		return nil, err
	}
	return f, nil
}

func newKernelBackendNative(l kernelObservationLease, ringBytes uint32, fresh func() (kernelObservationLease, error)) (*kernelBackendNative, error) {
	if ringBytes < uint32(os.Getpagesize()) || ringBytes&(ringBytes-1) != 0 || l.PID <= 0 || uint64(l.PID) > 1<<31-1 || fresh == nil {
		return nil, errors.New("explicit page-aligned power-of-two ring size and protected target required")
	}
	check := func() error {
		current, err := fresh()
		if err != nil {
			return err
		}
		if current != l {
			return errors.New("kernel observation source lease changed")
		}
		return kernelBackendTarget(l)
	}
	if err := check(); err != nil {
		return nil, err
	}
	b := &kernelBackendNative{pid: l.PID}
	var pidfd *os.File
	stages := []kernelBackendStage{
		{"pidfd", func() (io.Closer, error) {
			fd, err := unix.PidfdOpen(l.PID, 0)
			if err != nil {
				return nil, err
			}
			pidfd = os.NewFile(uintptr(fd), "swarm-owned-pid")
			return pidfd, kernelBackendPIDAlive(fd)
		}},
		{"ring", func() (io.Closer, error) {
			var err error
			b.ring, err = ebpf.NewMap(&ebpf.MapSpec{Name: "swarm_events", Type: ebpf.RingBuf, MaxEntries: ringBytes})
			return b.ring, err
		}},
		{"gate", func() (io.Closer, error) {
			var err error
			b.gate, err = ebpf.NewMap(&ebpf.MapSpec{Name: "swarm_gate", Type: ebpf.Array, KeySize: 4, ValueSize: 8, MaxEntries: 1})
			return b.gate, err
		}},
		{"counters", func() (io.Closer, error) {
			var err error
			b.counters, err = ebpf.NewMap(&ebpf.MapSpec{Name: "swarm_counts", Type: ebpf.Array, KeySize: 4, ValueSize: 24, MaxEntries: 1})
			return b.counters, err
		}},
		{"reader", func() (io.Closer, error) {
			var err error
			b.reader, err = ringbuf.NewReader(b.ring)
			return b.reader, err
		}},
	}
	for _, probe := range kernelBackendProbes {
		probe := probe
		var program *ebpf.Program
		stages = append(stages, kernelBackendStage{probe.Name + "-program", func() (io.Closer, error) {
			ins, err := kernelBackendInstructions(l.PID, probe.Kind, b.ring.FD(), b.gate.FD(), b.counters.FD())
			if err != nil {
				return nil, err
			}
			program, err = ebpf.NewProgram(&ebpf.ProgramSpec{Name: "swarm_owned", Type: ebpf.TracePoint, Instructions: ins, License: "Dual MIT/GPL"})
			return program, err
		}}, kernelBackendStage{probe.Name + "-perf", func() (io.Closer, error) {
			id, err := kernelBackendTraceID(probe)
			if err != nil {
				return nil, err
			}
			f, err := kernelBackendPerf(l.PID, id, program)
			if err == nil {
				b.perf = append(b.perf, f)
			}
			return f, err
		}})
	}
	stages = append(stages, kernelBackendStage{"arm", func() (io.Closer, error) {
		for _, f := range b.perf {
			if err := unix.IoctlSetInt(int(f.Fd()), unix.PERF_EVENT_IOC_ENABLE, 0); err != nil {
				return nil, err
			}
		}
		if err := check(); err != nil {
			return nil, err
		}
		if err := kernelBackendPIDAlive(int(pidfd.Fd())); err != nil {
			return nil, err
		}
		return nil, b.gate.Update(uint32(0), uint64(1), ebpf.UpdateAny)
	}})
	var err error
	b.resources, err = acquireKernelBackend(stages)
	if err != nil {
		return nil, err
	}
	return b, nil
}

func (b *kernelBackendNative) Read() (kernelBackendRecord, error) {
	r, err := b.reader.Read()
	if err != nil {
		return kernelBackendRecord{}, err
	}
	return decodeKernelBackendRecord(r.RawSample, b.pid)
}

// Stop disables emission and perf events before a final counter read.
// Closing the reader separately permits draining committed records after Stop.
func (b *kernelBackendNative) Stop() error {
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.stopped {
		return b.stopErr
	}
	b.stopped = true
	err := b.gate.Update(uint32(0), uint64(0), ebpf.UpdateAny)
	for _, f := range b.perf {
		err = errors.Join(err, unix.IoctlSetInt(int(f.Fd()), unix.PERF_EVENT_IOC_DISABLE, 0))
	}
	b.stopErr = err
	return b.stopErr
}

func (b *kernelBackendNative) Stats() (kernelBackendStats, error) {
	b.mu.Lock()
	defer b.mu.Unlock()
	if !b.stopped {
		return kernelBackendStats{}, errors.New("stop kernel backend before final statistics")
	}
	if b.stopErr != nil {
		return kernelBackendStats{}, b.stopErr
	}
	var counts [3]uint64
	if err := b.counters.Lookup(uint32(0), &counts); err != nil {
		return kernelBackendStats{}, err
	}
	return validateKernelBackendStats(counts)
}

func validateKernelBackendStats(c [3]uint64) (kernelBackendStats, error) {
	if c[1] > c[0] || c[2] > c[0] || c[1] != c[0]-c[2] {
		return kernelBackendStats{}, errors.New("kernel event counters inconsistent or overflowed")
	}
	return kernelBackendStats{c[0], c[1], c[2]}, nil
}

func (b *kernelBackendNative) Flush() error { return b.reader.Flush() }

func (b *kernelBackendNative) Close() error {
	err := b.Stop()
	return errors.Join(err, b.resources.Close())
}
