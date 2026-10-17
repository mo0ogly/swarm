//go:build linux && amd64

package engine

import (
	"encoding/binary"
	"errors"
	"math"

	"github.com/cilium/ebpf/asm"
)

type kernelBackendProbe struct {
	Group, Name string
	Kind        uint32
}

var kernelBackendProbes = [...]kernelBackendProbe{
	{"sched", "sched_process_fork", 1},
	{"syscalls", "sys_enter_openat", 2},
	{"syscalls", "sys_enter_connect", 3},
}

type kernelBackendRecord struct {
	Sequence, KtimeNS, CgroupID uint64
	PID                         uint32
	Kind                        string
}

func decodeKernelBackendRecord(b []byte, pid int) (kernelBackendRecord, error) {
	var r kernelBackendRecord
	if len(b) != 32 || pid <= 0 || uint64(pid) > math.MaxInt32 {
		return r, errors.New("invalid native observation size or target")
	}
	r.Sequence = binary.LittleEndian.Uint64(b[:8])
	r.KtimeNS = binary.LittleEndian.Uint64(b[8:16])
	r.CgroupID = binary.LittleEndian.Uint64(b[16:24])
	r.PID = binary.LittleEndian.Uint32(b[24:28])
	if r.Sequence == 0 || r.KtimeNS == 0 || r.PID != uint32(pid) {
		return kernelBackendRecord{}, errors.New("native observation identity or clock invalid")
	}
	switch binary.LittleEndian.Uint32(b[28:32]) {
	case 1:
		r.Kind = "process"
	case 2:
		r.Kind = "file"
	case 3:
		r.Kind = "network"
	default:
		return kernelBackendRecord{}, errors.New("unknown native observation kind")
	}
	return r, nil
}

func kernelBackendInstructions(pid int, kind uint32, ringFD, gateFD, statsFD int) (asm.Instructions, error) {
	if pid <= 0 || uint64(pid) > math.MaxInt32 || kind < 1 || kind > 3 || ringFD < 0 || gateFD < 0 || statsFD < 0 {
		return nil, errors.New("invalid fixed kernel probe")
	}
	return asm.Instructions{
		asm.FnGetCurrentPidTgid.Call(),
		asm.Mov.Reg(asm.R6, asm.R0),
		asm.RSh.Imm(asm.R0, 32),
		asm.JNE.Imm(asm.R0, int32(pid), "exit"),
		asm.Mov.Reg32(asm.R0, asm.R6),
		asm.JNE.Imm(asm.R0, int32(pid), "exit"),
		asm.LoadMapValue(asm.R8, gateFD, 0).WithSymbol("owned"),
		asm.LoadMem(asm.R0, asm.R8, 0, asm.DWord),
		asm.JEq.Imm(asm.R0, 0, "exit"),
		asm.LoadMapValue(asm.R7, statsFD, 0),
		asm.Mov.Imm(asm.R1, 1),
		asm.FetchAdd.Mem(asm.R7, asm.R1, asm.DWord, 0),
		asm.Add.Imm(asm.R1, 1),
		asm.StoreMem(asm.RFP, -32, asm.R1, asm.DWord),
		asm.FnKtimeGetNs.Call(),
		asm.StoreMem(asm.RFP, -24, asm.R0, asm.DWord),
		asm.FnGetCurrentCgroupId.Call(),
		asm.StoreMem(asm.RFP, -16, asm.R0, asm.DWord),
		asm.StoreMem(asm.RFP, -8, asm.R6, asm.Word),
		asm.StoreImm(asm.RFP, -4, int64(kind), asm.Word),
		asm.LoadMapPtr(asm.R1, ringFD),
		asm.Mov.Reg(asm.R2, asm.RFP), asm.Add.Imm(asm.R2, -32),
		asm.Mov.Imm(asm.R3, 32), asm.Mov.Imm(asm.R4, 0),
		asm.FnRingbufOutput.Call(),
		asm.Mov.Imm(asm.R1, 1),
		asm.JNE.Imm(asm.R0, 0, "lost"),
		asm.AddAtomic.Mem(asm.R7, asm.R1, asm.DWord, 8),
		asm.Ja.Label("exit"),
		asm.AddAtomic.Mem(asm.R7, asm.R1, asm.DWord, 16).WithSymbol("lost"),
		asm.Mov.Imm(asm.R0, 0).WithSymbol("exit"), asm.Return(),
	}, nil
}
