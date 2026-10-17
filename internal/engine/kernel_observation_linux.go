//go:build linux

package engine

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"os"
	"strconv"
	"strings"
	"sync"
	"unicode/utf8"
)

// This is an internal transport contract, not evidence of kernel collection.
const kernelObservationFrameBytes = 16 * 1024

type kernelObservation struct {
	CollectorBootID string `json:"collector_boot_id"`
	Sequence        uint64 `json:"sequence,string"`
	KtimeNS         uint64 `json:"ktime_ns,string"`
	EventType       string `json:"event_type"`
	PID             int    `json:"pid"`
	PIDStartTime    string `json:"pid_start_time"`
	ParentPID       int    `json:"parent_pid"`
	CgroupID        uint64 `json:"cgroup_id,string"`
	MissionID       string `json:"mission_id"`
	TaskID          string `json:"task_id"`
	AttemptID       string `json:"attempt_id"`
	ResourceDigest  string `json:"resource_digest"`
	LostBefore      uint64 `json:"lost_before,string"`
	ProbeSet        string `json:"probe_set"`
}

type kernelObservationLease struct {
	AgentID, Mission, Task, Attempt, Start, Host, ResourceDigest string
	PID                                                          int
}

func kernelObservationHex(s string, size int) bool {
	b, err := hex.DecodeString(s)
	return err == nil && len(b) == size && s == strings.ToLower(s)
}

func newKernelObservationLease(a Agent, digest string) (kernelObservationLease, error) {
	l := kernelObservationLease{a.ID, a.WorkID, a.TaskID, a.Attempt, a.ChildStamp, a.Host, digest, a.Child}
	if a.Status != "running" || a.Desired != "" || l.Host != hostIdentity() || l.PID <= 0 || l.AgentID == "" || l.Mission == "" || l.Task == "" || l.Attempt == "" || !kernelObservationHex(digest, 32) {
		return kernelObservationLease{}, errors.New("observation source is not an active owned attempt")
	}
	start, err := kernelObservationProcessStart(l.PID)
	if err != nil {
		return kernelObservationLease{}, err
	}
	if start == "" || start != l.Start {
		return kernelObservationLease{}, errors.New("observation process identity changed")
	}
	return l, nil
}

// The active status and desired stop state come from authoritative SQL columns.
// Callers must use this reader again after I/O, rather than recycling a lease.
func (s *Store) kernelObservationSource(agentID, digest string) (kernelObservationLease, error) {
	a, err := s.agent(agentID)
	if err != nil {
		return kernelObservationLease{}, err
	}
	return newKernelObservationLease(a, digest)
}

func kernelObservationProcessStart(pid int) (string, error) {
	if pid <= 0 {
		return "", errors.New("invalid process identity")
	}
	b, err := os.ReadFile(fmt.Sprintf("/proc/%d/stat", pid))
	if err != nil {
		return "", fmt.Errorf("read observation process identity: %w", err)
	}
	return parseKernelObservationProcessStart(b, pid)
}

func parseKernelObservationProcessStart(b []byte, pid int) (string, error) {
	s := string(b)
	at := strings.LastIndex(s, ")")
	if !strings.HasPrefix(s, strconv.Itoa(pid)+" (") || at < 0 {
		return "", errors.New("invalid process stat")
	}
	f := strings.Fields(s[at+1:])
	if len(f) < 20 || f[0] == "Z" || f[0] == "X" {
		return "", errors.New("process is absent or terminal")
	}
	v, err := strconv.ParseUint(f[19], 10, 64)
	if err != nil || v == 0 || strconv.FormatUint(v, 10) != f[19] {
		return "", errors.New("invalid process start")
	}
	return f[19], nil
}

func decodeKernelObservation(b []byte) (kernelObservation, error) {
	var e kernelObservation
	if len(b) > kernelObservationFrameBytes {
		return e, errors.New("observation frame exceeds 16384-byte transport bound")
	}
	if !utf8.Valid(b) {
		return e, errors.New("invalid observation UTF-8")
	}
	d := json.NewDecoder(bytes.NewReader(b))
	t, err := d.Token()
	if err != nil || t != json.Delim('{') {
		return e, errors.New("observation object required")
	}
	fields := map[string]json.RawMessage{}
	for d.More() {
		t, err = d.Token()
		if err != nil {
			return e, err
		}
		k, ok := t.(string)
		if !ok {
			return e, errors.New("invalid observation key")
		}
		if _, ok = fields[k]; ok {
			return e, errors.New("duplicate observation key")
		}
		var raw json.RawMessage
		if err = d.Decode(&raw); err != nil {
			return e, err
		}
		if bytes.Equal(bytes.TrimSpace(raw), []byte("null")) {
			return e, errors.New("null observation field")
		}
		fields[k] = raw
	}
	if _, err = d.Token(); err != nil {
		return e, err
	}
	if _, err = d.Token(); err != io.EOF {
		return e, errors.New("trailing observation data")
	}
	keys := []string{"collector_boot_id", "sequence", "ktime_ns", "event_type", "pid", "pid_start_time", "parent_pid", "cgroup_id", "mission_id", "task_id", "attempt_id", "resource_digest", "lost_before", "probe_set"}
	if len(fields) != len(keys) {
		return e, errors.New("observation fields missing or unknown")
	}
	for _, k := range keys {
		if _, ok := fields[k]; !ok {
			return e, errors.New("observation field missing")
		}
	}
	if err = json.Unmarshal(b, &e); err != nil {
		return e, err
	}
	for _, k := range []string{"sequence", "ktime_ns", "cgroup_id", "lost_before"} {
		var s string
		if err = json.Unmarshal(fields[k], &s); err != nil {
			return e, err
		}
		v, err := strconv.ParseUint(s, 10, 64)
		if err != nil || strconv.FormatUint(v, 10) != s {
			return e, errors.New("noncanonical observation integer")
		}
	}
	if !kernelObservationHex(e.CollectorBootID, 16) || !kernelObservationHex(e.ResourceDigest, 32) || e.Sequence == 0 || e.KtimeNS == 0 || e.PID <= 0 || e.ParentPID < 0 || e.MissionID == "" || e.TaskID == "" || e.AttemptID == "" || e.ProbeSet != "swarm-minimal-v1" {
		return e, errors.New("invalid observation metadata")
	}
	start, err := strconv.ParseUint(e.PIDStartTime, 10, 64)
	if err != nil || start == 0 || strconv.FormatUint(start, 10) != e.PIDStartTime {
		return e, errors.New("invalid observation process start")
	}
	switch e.EventType {
	case "process", "file", "network":
	default:
		return e, errors.New("unknown observation event type")
	}
	return e, nil
}

type kernelObservationStream struct {
	mu                              sync.Mutex
	lease                           kernelObservationLease
	boot, probes                    string
	sequence, ktime, lost, accepted uint64
	disconnected                    bool
}

type kernelObservationStatus struct {
	Accepted     uint64 `json:"accepted"`
	Lost         uint64 `json:"lost"`
	Disconnected bool   `json:"disconnected"`
	Complete     bool   `json:"complete"`
}

func newKernelObservationStream(l kernelObservationLease, boot, probes string) *kernelObservationStream {
	return &kernelObservationStream{lease: l, boot: boot, probes: probes}
}

func (s *kernelObservationStream) accept(e kernelObservation, fresh kernelObservationLease) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.disconnected {
		return errors.New("observation stream disconnected")
	}
	if fresh != s.lease {
		return errors.New("observation source lease changed")
	}
	start, err := kernelObservationProcessStart(fresh.PID)
	if err != nil {
		return err
	}
	if start != fresh.Start {
		return errors.New("observation source process identity changed")
	}
	if e.CollectorBootID != s.boot || e.ProbeSet != s.probes || e.PID != fresh.PID || e.PIDStartTime != fresh.Start || e.MissionID != fresh.Mission || e.TaskID != fresh.Task || e.AttemptID != fresh.Attempt || e.ResourceDigest != fresh.ResourceDigest {
		return errors.New("observation attribution mismatch")
	}
	b, err := json.Marshal(e)
	if err != nil {
		return err
	}
	if _, err = decodeKernelObservation(b); err != nil {
		return err
	}
	if s.sequence == math.MaxUint64 || e.LostBefore > math.MaxUint64-s.sequence-1 || e.Sequence != s.sequence+e.LostBefore+1 || e.KtimeNS < s.ktime || e.LostBefore > math.MaxUint64-s.lost || s.accepted == math.MaxUint64 {
		return errors.New("observation order or loss counter invalid")
	}
	s.sequence = e.Sequence
	s.ktime = e.KtimeNS
	s.lost += e.LostBefore
	s.accepted++
	return nil
}

func (s *kernelObservationStream) snapshot() kernelObservationStatus {
	s.mu.Lock()
	defer s.mu.Unlock()
	// EOF, an empty stream, or loss-free transport cannot certify kernel coverage.
	return kernelObservationStatus{Accepted: s.accepted, Lost: s.lost, Disconnected: s.disconnected, Complete: false}
}

func (s *kernelObservationStream) disconnect() {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.disconnected = true
}
