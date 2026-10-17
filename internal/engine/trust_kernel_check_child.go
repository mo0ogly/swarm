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

func init() {
	if len(os.Args) == 2 && os.Args[1] == "_kernel_check" {
		runtime.LockOSThread()
	}
}

func runTrustKernelCheckChild(in io.Reader) int {
	runtime.LockOSThread()
	raw, err := io.ReadAll(io.LimitReader(in, kernelObservationFrameBytes+1))
	if err != nil || len(raw) > kernelObservationFrameBytes {
		return 2
	}
	var b kernelCheckBootstrap
	if strict(raw, &b) != nil || b.Protocol != kernelCheckProtocol || !kernelObservationHex(b.Nonce, 16) || b.Parent != os.Getppid() || validateTrustKernelPolicy(&b.Policy) != nil || !bool(b.Policy.Enabled) {
		return 2
	}
	if kernelFixedHelperSecurity(b.Parent, b.ParentStart) != nil {
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
	if json.NewEncoder(c).Encode(map[string]string{"protocol": b.Protocol, "nonce": b.Nonce}) != nil {
		return 2
	}
	reader := bufio.NewReaderSize(c, kernelObservationFrameBytes+1)
	var ack struct {
		Run bool `json:"run"`
	}
	if kernelCheckRead(reader, &ack) != nil || !ack.Run {
		return 2
	}
	result := runKernelOwnedFixture(b.Policy, b.Nonce, b.Socket)
	if json.NewEncoder(c).Encode(result) != nil {
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
