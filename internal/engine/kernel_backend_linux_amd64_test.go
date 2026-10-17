//go:build linux && amd64

package engine

import (
	"bytes"
	"encoding/binary"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"reflect"
	"strings"
	"sync"
	"testing"

	"github.com/cilium/ebpf/asm"
	"golang.org/x/sys/unix"
)

func TestKernelBackendRecord(t *testing.T) {
	raw := make([]byte, 32)
	binary.LittleEndian.PutUint64(raw[0:8], 1)
	binary.LittleEndian.PutUint64(raw[8:16], 42)
	binary.LittleEndian.PutUint64(raw[16:24], 99)
	binary.LittleEndian.PutUint32(raw[24:28], 123)
	binary.LittleEndian.PutUint32(raw[28:32], 2)
	r, err := decodeKernelBackendRecord(raw, 123)
	if err != nil || r.Sequence != 1 || r.KtimeNS != 42 || r.Kind != "file" {
		t.Fatalf("record: %v", err)
	}
	for _, kind := range []string{"short", "foreign", "unknown", "zero_sequence", "zero_clock"} {
		t.Run(kind, func(t *testing.T) {
			b := append([]byte(nil), raw...)
			pid := 123
			switch kind {
			case "short":
				b = b[:31]
			case "foreign":
				pid = 124
			case "unknown":
				binary.LittleEndian.PutUint32(b[28:32], 77)
			case "zero_sequence":
				binary.LittleEndian.PutUint64(b[:8], 0)
			case "zero_clock":
				binary.LittleEndian.PutUint64(b[8:16], 0)
			}
			if _, err := decodeKernelBackendRecord(b, pid); err == nil {
				t.Fatal("invalid kernel sample accepted")
			}
		})
	}
}

type kernelBackendTestCloser struct {
	name   string
	closed *[]string
	fail   bool
}

func (c *kernelBackendTestCloser) Close() error {
	if c == nil {
		panic("nil resource close")
	}
	*c.closed = append(*c.closed, c.name)
	if c.fail {
		return os.ErrPermission
	}
	return nil
}

func TestKernelBackendAcquisitionCleanup(t *testing.T) {
	for failed := 0; failed < 5; failed++ {
		t.Run(fmt.Sprint(failed), func(t *testing.T) {
			var closed []string
			var stages []kernelBackendStage
			for i := 0; i < 5; i++ {
				i := i
				stages = append(stages, kernelBackendStage{fmt.Sprint(i), func() (io.Closer, error) {
					if i == failed {
						var nilResource *kernelBackendTestCloser
						return nilResource, os.ErrPermission
					}
					return &kernelBackendTestCloser{fmt.Sprint(i), &closed, false}, nil
				}})
			}
			if _, err := acquireKernelBackend(stages); !errors.Is(err, os.ErrPermission) {
				t.Fatal("acquisition failure lost")
			}
			var expected []string
			for i := failed - 1; i >= 0; i-- {
				expected = append(expected, fmt.Sprint(i))
			}
			if !reflect.DeepEqual(closed, expected) {
				t.Fatalf("leaked or wrong cleanup: %v", closed)
			}
		})
	}
	var closed []string
	r, err := acquireKernelBackend([]kernelBackendStage{{"first", func() (io.Closer, error) { return &kernelBackendTestCloser{"first", &closed, true}, nil }}, {"last", func() (io.Closer, error) { return &kernelBackendTestCloser{"last", &closed, false}, nil }}})
	if err != nil {
		t.Fatal(err)
	}
	var wg sync.WaitGroup
	for i := 0; i < 8; i++ {
		wg.Add(1)
		go func() {
			defer wg.Done()
			if !errors.Is(r.Close(), os.ErrPermission) {
				t.Error("cleanup error lost")
			}
		}()
	}
	wg.Wait()
	if !reflect.DeepEqual(closed, []string{"last", "first"}) {
		t.Fatalf("cleanup repeated: %v", closed)
	}
}

func TestKernelBackendLossCounters(t *testing.T) {
	s, err := validateKernelBackendStats([3]uint64{10, 7, 3})
	if err != nil || s.Lost != 3 {
		t.Fatal("last loss hidden")
	}
	for _, bad := range [][3]uint64{{1, 2, 0}, {1, 0, 2}, {10, 7, 2}, {0, ^uint64(0), 1}} {
		if _, err := validateKernelBackendStats(bad); err == nil {
			t.Fatal("inconsistent counters accepted")
		}
	}
}

func TestKernelBackendRefusesBeforeAcquisition(t *testing.T) {
	l, _ := kernelObservationFixture(t)
	for _, kind := range []string{"ring_zero", "ring_unaligned", "missing_source", "stale_source", "source_error", "stale_pid"} {
		t.Run(kind, func(t *testing.T) {
			bytes := uint32(os.Getpagesize())
			want := l
			fresh := func() (kernelObservationLease, error) { return l, nil }
			switch kind {
			case "ring_zero":
				bytes = 0
			case "ring_unaligned":
				bytes++
			case "missing_source":
				fresh = nil
			case "stale_source":
				fresh = func() (kernelObservationLease, error) { v := l; v.Attempt = "new"; return v, nil }
			case "source_error":
				fresh = func() (kernelObservationLease, error) { return kernelObservationLease{}, os.ErrPermission }
			case "stale_pid":
				want.Start = "1"
				fresh = func() (kernelObservationLease, error) { return want, nil }
			}
			if _, err := newKernelBackendNative(want, bytes, fresh); err == nil || strings.HasPrefix(err.Error(), "kernel backend ") {
				t.Fatal("invalid target admitted or native acquisition started")
			}
		})
	}
}

// Execute the admission prefix only. This is not a kernel verifier substitute.
func kernelBackendFilterReachedMemory(ins asm.Instructions, tgid, tid uint32) (bool, error) {
	var regs [11]uint64
	labels := map[string]int{}
	for i, v := range ins {
		if v.Symbol() != "" {
			labels[v.Symbol()] = i
		}
	}
	for pc := 0; pc < len(ins); pc++ {
		v := ins[pc]
		if v.OpCode.Class() == asm.LdClass || v.OpCode.Class() == asm.LdXClass || v.OpCode.Class() == asm.StXClass || v.OpCode.Class() == asm.StClass {
			return true, nil
		}
		if v.IsBuiltinCall() {
			if v.Constant != int64(asm.FnGetCurrentPidTgid) {
				return false, fmt.Errorf("unexpected helper before filter")
			}
			regs[0] = uint64(tgid)<<32 | uint64(tid)
			continue
		}
		switch v.OpCode.Class() {
		case asm.ALU64Class, asm.ALUClass:
			src := uint64(v.Constant)
			if v.OpCode.Source() == asm.RegSource {
				src = regs[v.Src]
			}
			switch v.OpCode.ALUOp() {
			case asm.Mov:
				regs[v.Dst] = src
			case asm.RSh:
				regs[v.Dst] >>= src
			default:
				return false, fmt.Errorf("unknown admission ALU")
			}
			if v.OpCode.Class() == asm.ALUClass {
				regs[v.Dst] = uint64(uint32(regs[v.Dst]))
			}
		case asm.JumpClass:
			switch v.OpCode.JumpOp() {
			case asm.Exit:
				return false, nil
			case asm.JNE:
				if regs[v.Dst] != uint64(v.Constant) {
					target, ok := labels[v.Reference()]
					if !ok {
						return false, fmt.Errorf("unresolved filter target")
					}
					pc = target - 1
				}
			default:
				return false, fmt.Errorf("unknown admission jump")
			}
		default:
			return false, fmt.Errorf("unknown admission instruction")
		}
	}
	return false, fmt.Errorf("unterminated admission filter")
}

func TestKernelBackendProgramScope(t *testing.T) {
	for _, probe := range kernelBackendProbes {
		ins, err := kernelBackendInstructions(123, probe.Kind, 10, 11, 12)
		if err != nil {
			t.Fatal(err)
		}
		for _, ids := range [][2]uint32{{123, 123}, {124, 124}, {123, 124}, {124, 123}} {
			entered, err := kernelBackendFilterReachedMemory(ins, ids[0], ids[1])
			if err != nil {
				t.Fatal(err)
			}
			if entered != (ids[0] == 123 && ids[1] == 123) {
				t.Fatalf("foreign target passed kernel filter: %v", ids)
			}
		}
		var encoded bytes.Buffer
		if err = ins.Marshal(&encoded, binary.LittleEndian); err != nil || encoded.Len() == 0 {
			t.Fatalf("unresolved native program: %v", err)
		}
	}
	if _, err := kernelBackendInstructions(0, 1, 10, 11, 12); err == nil {
		t.Fatal("invalid target accepted")
	}
	if _, err := kernelBackendInstructions(123, 99, 10, 11, 12); err == nil {
		t.Fatal("unknown probe accepted")
	}
}

func TestKernelBackendNativeAvailability(t *testing.T) {
	path := os.Getenv("SWARM_TEST_EBPF_NATIVE_EVIDENCE")
	if path == "" {
		t.Skip("explicit owned native diagnostic only")
	}
	l, _ := kernelObservationFixture(t)
	b, err := newKernelBackendNative(l, uint32(os.Getpagesize()), func() (kernelObservationLease, error) { return l, nil })
	v := map[string]any{"source": "ebpf", "available": err == nil, "scope": "owned_test_process_main_thread_only", "target_pid": l.PID, "target_start": l.Start, "native_event_coverage_verified": false, "global_qualification": false, "privilege_change": false, "public_cli_exit_code_observed": false}
	if err != nil {
		v["error"] = err.Error()
		v["permission_denied"] = errors.Is(err, unix.EPERM) || errors.Is(err, unix.EACCES)
	} else {
		if err = b.Close(); err != nil {
			t.Fatal(err)
		}
	}
	raw, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	f, err := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
	if err != nil {
		t.Fatal(err)
	}
	if _, err = f.Write(append(raw, '\n')); err != nil {
		_ = f.Close()
		t.Fatal(err)
	}
	if err = f.Close(); err != nil {
		t.Fatal(err)
	}
	t.Logf("owned native diagnostic saved; available=%v; event coverage unverified", v["available"])
}
