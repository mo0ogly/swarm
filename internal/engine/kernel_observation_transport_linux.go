//go:build linux

package engine

import (
	"bufio"
	"errors"
	"net"

	"golang.org/x/sys/unix"
)

type kernelObservationPeer struct {
	PID   int
	UID   uint32
	Start string
}

func authenticateKernelObservationPeer(c *net.UnixConn, want kernelObservationPeer) error {
	if c == nil || want.PID <= 0 || want.Start == "" {
		return errors.New("collector process identity required")
	}
	raw, err := c.SyscallConn()
	if err != nil {
		return err
	}
	var cred *unix.Ucred
	var peerErr error
	if err = raw.Control(func(fd uintptr) { cred, peerErr = unix.GetsockoptUcred(int(fd), unix.SOL_SOCKET, unix.SO_PEERCRED) }); err != nil {
		return err
	}
	if peerErr != nil {
		return peerErr
	}
	if cred == nil || int(cred.Pid) != want.PID || cred.Uid != want.UID {
		return errors.New("unexpected observation collector peer")
	}
	start, err := kernelObservationProcessStart(want.PID)
	if err != nil {
		return err
	}
	if start != want.Start {
		return errors.New("observation collector process identity changed")
	}
	return nil
}

type kernelObservationReceiver struct {
	conn   *net.UnixConn
	reader *bufio.Reader
	peer   kernelObservationPeer
	stream *kernelObservationStream
	fresh  func() (kernelObservationLease, error)
}

func newKernelObservationReceiver(c *net.UnixConn, p kernelObservationPeer, s *kernelObservationStream, fresh func() (kernelObservationLease, error)) (*kernelObservationReceiver, error) {
	if s == nil || fresh == nil {
		return nil, errors.New("protected observation source required")
	}
	if err := authenticateKernelObservationPeer(c, p); err != nil {
		return nil, err
	}
	return &kernelObservationReceiver{c, bufio.NewReaderSize(c, kernelObservationFrameBytes+1), p, s, fresh}, nil
}

// The single reader belongs to the launcher. Cancellation closes its connection.
// Peer credentials attest a process, never that its bytes originated in eBPF.
func (r *kernelObservationReceiver) read() (e kernelObservation, err error) {
	defer func() {
		if err != nil {
			r.stream.disconnect()
			_ = r.conn.Close()
		}
	}()
	if err = authenticateKernelObservationPeer(r.conn, r.peer); err != nil {
		return e, err
	}
	b, err := r.reader.ReadSlice('\n')
	if err != nil {
		return e, err
	}
	e, err = decodeKernelObservation(b[:len(b)-1])
	if err != nil {
		return kernelObservation{}, err
	}
	// Recheck after I/O: the peer or source may have changed while waiting.
	if err = authenticateKernelObservationPeer(r.conn, r.peer); err != nil {
		return kernelObservation{}, err
	}
	fresh, err := r.fresh()
	if err != nil {
		return kernelObservation{}, err
	}
	if err = r.stream.accept(e, fresh); err != nil {
		return kernelObservation{}, err
	}
	return e, nil
}
