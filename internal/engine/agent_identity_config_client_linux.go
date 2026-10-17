//go:build linux

package engine

import (
	"bytes"
	"context"
	"crypto/rand"
	"encoding/base64"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"os"
	"strconv"
	"sync/atomic"

	"golang.org/x/sys/unix"
	"swarm.local/companion/internal/enrollmentconfig"
)

var errAgentIdentityConfigReply = errors.New("identity configuration: invalid or unrelated reply")
var errAgentIdentityConfigPeer = errors.New("identity configuration: independently pinned service process required")
var errAgentIdentityConfigUsed = errors.New("identity configuration: channel already consumed")
var errAgentIdentityConfigUnavailable = errors.New("identity configuration: service unavailable")

// Linux v6.17 fs/pidfs.c assigns a unique 64-bit inode to struct pid. Older
// anonymous-inode pidfds and 32-bit inode truncation must not become PID fallbacks.
// The caller supplies an independently held process descriptor, not a packet PID.
func agentIdentityConfigProcessToken(file *os.File) ([2]uint64, error) {
	var token [2]uint64
	if file == nil || strconv.IntSize != 64 {
		return token, errAgentIdentityConfigPeer
	}
	raw, err := file.SyscallConn()
	if err != nil {
		return token, err
	}
	var stat unix.Stat_t
	var fs unix.Statfs_t
	var cause error
	err = raw.Control(func(fd uintptr) {
		cause = unix.Fstatfs(int(fd), &fs)
		if cause == nil {
			cause = unix.Fstat(int(fd), &stat)
		}
	})
	if err != nil || cause != nil {
		return token, errors.Join(err, cause)
	}
	if fs.Type != unix.PID_FS_MAGIC || stat.Ino < 2 {
		return token, errAgentIdentityConfigPeer
	}
	return [2]uint64{uint64(stat.Dev), stat.Ino}, nil
}

type agentIdentityConfigClient struct {
	conn    *net.UnixConn
	process *os.File
	uid     uint32
	pins    enrollmentconfig.Pins
	used    atomic.Bool
}

// Ownership transfers only on success. Keep the launcher's original pidfd alive
// independently; the client duplicates it and owns its connection and duplicate.
func newAgentIdentityConfigClient(conn *net.UnixConn, process *os.File, uid uint32, pins enrollmentconfig.Pins) (*agentIdentityConfigClient, error) {
	if err := pins.Validate(); err != nil {
		return nil, err
	}
	if _, err := agentIdentityConfigProcessToken(process); err != nil {
		return nil, err
	}
	raw, err := process.SyscallConn()
	if err != nil {
		return nil, err
	}
	fd := -1
	var duplicateErr error
	if err = raw.Control(func(f uintptr) { fd, duplicateErr = unix.FcntlInt(f, unix.F_DUPFD_CLOEXEC, 0) }); err != nil || duplicateErr != nil {
		return nil, errors.Join(err, duplicateErr)
	}
	copy := os.NewFile(uintptr(fd), "identity-config-pinned-service")
	if err = configureAgentIdentityChannel(conn); err != nil {
		return nil, errors.Join(err, copy.Close())
	}
	return &agentIdentityConfigClient{conn: conn, process: copy, uid: uid, pins: pins}, nil
}

func agentIdentityConfigCloseError(err error) error {
	if errors.Is(err, net.ErrClosed) || errors.Is(err, os.ErrClosed) {
		return nil
	}
	return err
}

func (c *agentIdentityConfigClient) close() error {
	if c == nil {
		return nil
	}
	return errors.Join(agentIdentityConfigCloseError(c.conn.Close()), agentIdentityConfigCloseError(c.process.Close()))
}

func agentIdentityConfigReplyError(code string) error {
	switch code {
	case "":
		return nil
	case "denied":
		return enrollmentconfig.ErrUnauthorized
	case "invalid_request":
		return enrollmentconfig.ErrFormat
	case "conflict":
		return enrollmentconfig.ErrConflict
	case "busy":
		return enrollmentconfig.ErrBusy
	case "pending":
		return enrollmentconfig.ErrPending
	case "not_configured":
		return enrollmentconfig.ErrNotConfigured
	case "indeterminate":
		return enrollmentconfig.ErrIndeterminate
	case "unavailable":
		return errAgentIdentityConfigUnavailable
	default:
		return errAgentIdentityConfigReply
	}
}

func decodeAgentIdentityConfigReply(raw []byte, request agentIdentityConfigRequest, pins enrollmentconfig.Pins) (agentIdentityConfigReply, error) {
	var reply agentIdentityConfigReply
	if err := json.Unmarshal(raw, &reply); err != nil {
		return agentIdentityConfigReply{}, errors.Join(errAgentIdentityConfigReply, err)
	}
	canonical, err := json.Marshal(reply)
	if err != nil || !bytes.Equal(canonical, raw) {
		return agentIdentityConfigReply{}, errors.Join(errAgentIdentityConfigReply, err)
	}
	requestRaw, err := encodeAgentIdentityConfigRequest(request)
	if err != nil {
		return agentIdentityConfigReply{}, err
	}
	if reply.Schema != 2 || reply.Operation != request.Operation || request.Pins != pins || reply.RequestDigest != agentIdentityConfigRequestDigest(requestRaw) {
		return agentIdentityConfigReply{}, errAgentIdentityConfigReply
	}
	if reply.Error != "" {
		if reply.View != nil || reply.Projection != nil || errors.Is(agentIdentityConfigReplyError(reply.Error), errAgentIdentityConfigReply) {
			return agentIdentityConfigReply{}, errAgentIdentityConfigReply
		}
		return reply, nil
	}
	var saved enrollmentconfig.View
	if request.Operation == "preview" {
		if reply.Projection == nil || reply.View != nil {
			return agentIdentityConfigReply{}, errAgentIdentityConfigReply
		}
		if err = enrollmentconfig.ValidateProjection(*reply.Projection); err != nil {
			return agentIdentityConfigReply{}, errors.Join(errAgentIdentityConfigReply, err)
		}
		saved = reply.Projection.Next
	} else {
		if reply.View == nil || reply.Projection != nil {
			return agentIdentityConfigReply{}, errAgentIdentityConfigReply
		}
		if err = enrollmentconfig.ValidateView(*reply.View); err != nil {
			return agentIdentityConfigReply{}, errors.Join(errAgentIdentityConfigReply, err)
		}
		saved = *reply.View
	}
	if saved.Snapshot.Pins != pins {
		return agentIdentityConfigReply{}, enrollmentconfig.ErrPins
	}
	if request.Change != nil && saved.Snapshot.History[len(saved.Snapshot.History)-1].Change != *request.Change {
		return agentIdentityConfigReply{}, errAgentIdentityConfigReply
	}
	return reply, nil
}

func (c *agentIdentityConfigClient) exchange(ctx context.Context, operation string, change *enrollmentconfig.Change, buffer []byte) (result agentIdentityConfigReply, err error) {
	if c == nil {
		return result, errAgentIdentityConfigPeer
	}
	if !c.used.CompareAndSwap(false, true) {
		return result, errAgentIdentityConfigUsed
	}
	mutating, attempted, definitive := operation == "create" || operation == "apply", false, false
	defer func() {
		if closeErr := c.close(); closeErr != nil {
			err = errors.Join(err, closeErr)
			definitive = false
		}
		if err != nil {
			result = agentIdentityConfigReply{}
			if attempted && mutating && !definitive {
				err = errors.Join(enrollmentconfig.ErrIndeterminate, err)
			}
		}
	}()
	if err = ctx.Err(); err != nil {
		return result, err
	}
	if len(buffer) == 0 {
		return result, enrollmentconfig.ErrFormat
	}
	if _, err = agentIdentityConfigProcessToken(c.process); err != nil {
		return result, err
	}
	nonce := make([]byte, 32)
	if _, err = rand.Read(nonce); err != nil {
		return result, err
	}
	request := agentIdentityConfigRequest{Schema: 2, Challenge: base64.RawURLEncoding.EncodeToString(nonce), Pins: c.pins, Operation: operation, Change: change}.owned()
	raw, err := encodeAgentIdentityConfigRequest(request)
	if err != nil {
		return result, err
	}
	closed := make(chan error, 1)
	stop := context.AfterFunc(ctx, func() { closed <- agentIdentityConfigCloseError(c.conn.Close()) })
	defer func() {
		if !stop() {
			err = errors.Join(err, <-closed, ctx.Err())
		}
	}()
	if err = ctx.Err(); err != nil {
		return result, err
	}
	attempted = true
	n, err := c.conn.Write(raw)
	if err != nil {
		return result, fmt.Errorf("send configuration request: %w", err)
	}
	if n != len(raw) {
		return result, io.ErrShortWrite
	}
	message, err := receiveAgentIdentityMessage(c.conn, buffer)
	if err != nil {
		return result, fmt.Errorf("receive configuration reply: %w", err)
	}
	defer func() { err = errors.Join(err, message.Close()) }()
	expected, err := agentIdentityConfigProcessToken(c.process)
	if err != nil {
		return result, err
	}
	actual, err := agentIdentityConfigProcessToken(message.process)
	if err != nil {
		return result, err
	}
	if actual != expected || message.uid != c.uid {
		return result, errAgentIdentityConfigPeer
	}
	result, err = decodeAgentIdentityConfigReply(message.raw, request, c.pins)
	if err != nil {
		return agentIdentityConfigReply{}, err
	}
	definitive = true
	if err = agentIdentityConfigReplyError(result.Error); err != nil {
		return agentIdentityConfigReply{}, err
	}
	return result, nil
}
