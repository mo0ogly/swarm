//go:build linux

package engine

import (
	"bufio"
	"encoding/json"
	"io"
	"net"
	"os"
	"runtime"
)

func runKernelObserveChild(in io.Reader) int {
	runtime.LockOSThread()
	raw, err := io.ReadAll(io.LimitReader(in, kernelObservationFrameBytes+1))
	if err != nil || len(raw) > kernelObservationFrameBytes {
		return 2
	}
	var b kernelObserveBootstrap
	if strict(raw, &b) != nil || b.Protocol != kernelObserveProtocol || !kernelObservationHex(b.Nonce, 16) || validateKernelObservePolicy(&b.Policy) != nil || !bool(b.Policy.Enabled) || kernelFixedHelperSecurity(b.Parent, b.ParentStart) != nil {
		return 2
	}
	if b.Source.Host != hostIdentity() || b.Source.PID <= 0 {
		return 2
	}
	c, err := net.DialUnix("unix", nil, &net.UnixAddr{Name: b.Socket, Net: "unix"})
	if err != nil {
		return 2
	}
	defer c.Close()
	if authenticateKernelObservationPeer(c, kernelObservationPeer{b.Parent, uint32(os.Getuid()), b.ParentStart}) != nil {
		return 2
	}
	if kernelObserveSend(c, kernelObservePacket{Type: "ready", Protocol: b.Protocol, Nonce: b.Nonce}) != nil {
		return 2
	}
	reader := bufio.NewReaderSize(c, kernelObservationFrameBytes+1)
	fresh := func() (kernelObservationLease, error) { return kernelObserveAuthorize(c, reader, b.Source) }
	result, err := runKernelObserveNative(b.Policy, b.Source, b.Nonce, fresh, func(e kernelObservation) error {
		raw, err := json.Marshal(e)
		if err != nil {
			return err
		}
		return kernelObserveSend(c, kernelObservePacket{Type: "event", Event: raw})
	})
	if err != nil {
		result = kernelObserveFixed(b.Policy, b.Source)
		result.StopReason = "collector_failed"
		result.Diagnostic = err.Error()
	}
	body, e := json.Marshal(result)
	if e != nil {
		return 2
	}
	if kernelObserveSend(c, kernelObservePacket{Type: "result", Result: body}) != nil {
		return 2
	}
	var done struct {
		Done bool `json:"done"`
	}
	if kernelCheckRead(reader, &done) != nil || !done.Done {
		return 2
	}
	return 0
}
