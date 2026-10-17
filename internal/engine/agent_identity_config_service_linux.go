//go:build linux

package engine

import (
	"bytes"
	"context"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net"
	"sync"
	"sync/atomic"
	"time"

	"swarm.local/companion/internal/enrollmentconfig"
	"swarm.local/companion/internal/identitycontract"
)

// This private adapter has no CLI/HTTP route or listener. Trusted bootstrap must
// supply the paths/pins and an authority controller independent of project data.
// No field in a packet authenticates its sender or grants a configuration right.
type agentIdentityConfigRequest struct {
	Schema    int                      `json:"schema_version"`
	Challenge string                   `json:"challenge"`
	Pins      enrollmentconfig.Pins    `json:"pins"`
	Operation string                   `json:"operation"`
	Change    *enrollmentconfig.Change `json:"change"`
}

func encodeAgentIdentityConfigRequest(r agentIdentityConfigRequest) ([]byte, error) {
	if r.Schema != 2 {
		return nil, enrollmentconfig.ErrVersion
	}
	nonce, err := base64.RawURLEncoding.Strict().DecodeString(r.Challenge)
	if err != nil || len(nonce) != 32 || base64.RawURLEncoding.EncodeToString(nonce) != r.Challenge {
		return nil, enrollmentconfig.ErrFormat
	}
	if err := r.Pins.Validate(); err != nil {
		return nil, errors.Join(enrollmentconfig.ErrFormat, err)
	}
	switch r.Operation {
	case "read":
		if r.Change != nil {
			return nil, enrollmentconfig.ErrFormat
		}
	case "preview", "create", "apply":
		if r.Change == nil {
			return nil, enrollmentconfig.ErrFormat
		}
		if _, err := enrollmentconfig.EncodeChange(*r.Change); err != nil {
			return nil, err
		}
	default:
		return nil, enrollmentconfig.ErrFormat
	}
	return json.Marshal(r)
}

func decodeAgentIdentityConfigRequest(raw []byte) (agentIdentityConfigRequest, error) {
	var r agentIdentityConfigRequest
	if err := json.Unmarshal(raw, &r); err != nil {
		return agentIdentityConfigRequest{}, fmt.Errorf("%w: %w", enrollmentconfig.ErrFormat, err)
	}
	canonical, err := encodeAgentIdentityConfigRequest(r)
	if err != nil {
		return agentIdentityConfigRequest{}, err
	}
	if !bytes.Equal(raw, canonical) {
		return agentIdentityConfigRequest{}, enrollmentconfig.ErrFormat
	}
	return r, nil
}

type agentIdentityConfigReply struct {
	Schema        int                          `json:"schema_version"`
	RequestDigest string                       `json:"request_digest"`
	Operation     string                       `json:"operation"`
	View          *enrollmentconfig.View       `json:"view"`
	Projection    *enrollmentconfig.Projection `json:"projection"`
	Error         string                       `json:"error"`
}

func agentIdentityConfigRequestDigest(raw []byte) string {
	digest := sha256.Sum256(append([]byte("swarm-installation-control-v2\x00request\x00"), raw...))
	return hex.EncodeToString(digest[:])
}

// withAuthority authenticates the actual message sender and authorizes the exact
// request. It must keep its independent authority/revocation serialization held
// through run's return, execute synchronously and neither retain nor close the
// message. A callback fixture is not real authentication or OS confinement.
type agentIdentityConfigService struct {
	configPath, witnessPath string
	owner                   uint32
	pins                    enrollmentconfig.Pins
	now                     func() time.Time
	withAuthority           func(context.Context, *agentIdentityMessage, agentIdentityConfigRequest, func(identitycontract.Principal) error) error
}

func (r agentIdentityConfigRequest) owned() agentIdentityConfigRequest {
	if r.Change != nil {
		c := *r.Change
		r.Change = &c
	}
	return r
}

func (s *agentIdentityConfigService) handle(ctx context.Context, message *agentIdentityMessage) (agentIdentityConfigReply, error) {
	reply := agentIdentityConfigReply{Schema: 2}
	if message == nil || message.process == nil {
		return reply, enrollmentconfig.ErrUnauthorized
	}
	reply.RequestDigest = agentIdentityConfigRequestDigest(message.raw)
	request, err := decodeAgentIdentityConfigRequest(message.raw)
	if err != nil {
		return reply, err
	}
	reply.Operation = request.Operation
	if s == nil || s.withAuthority == nil {
		return reply, enrollmentconfig.ErrUnauthorized
	}
	if request.Pins != s.pins {
		return reply, enrollmentconfig.ErrPins
	}
	if err = ctx.Err(); err != nil {
		return reply, err
	}
	// A trusted controller must not accidentally manufacture a success by omitting
	// run, swallowing its error, invoking it twice or retaining it after returning.
	var mu sync.Mutex
	active, called, invalid := true, false, false
	var operationErr error
	var result agentIdentityConfigReply
	run := func(actor identitycontract.Principal) error {
		mu.Lock()
		defer mu.Unlock()
		if !active || called {
			invalid = true
			return enrollmentconfig.ErrUnauthorized
		}
		called = true
		operationErr = func() error {
			if err := ctx.Err(); err != nil {
				return err
			}
			if actor != s.pins.Principal || !actor.Valid() || actor.Kind != "human" {
				return enrollmentconfig.ErrUnauthorized
			}
			authorize := func(ctx context.Context, _ *enrollmentconfig.Snapshot, change enrollmentconfig.Change) (identitycontract.Principal, error) {
				if err := ctx.Err(); err != nil {
					return identitycontract.Principal{}, err
				}
				if request.Change == nil || change != *request.Change {
					return identitycontract.Principal{}, enrollmentconfig.ErrUnauthorized
				}
				return actor, nil
			}
			coordinator, err := enrollmentconfig.OpenCoordinated(s.configPath, s.witnessPath, s.owner, s.pins, authorize)
			if err != nil {
				return err
			}
			at := time.Now().UTC()
			if s.now != nil {
				at = s.now().UTC()
			}
			result = reply
			switch request.Operation {
			case "read":
				v, e := coordinator.Read(ctx)
				err = e
				if e == nil {
					result.View = &v
				}
			case "preview":
				p, e := coordinator.Preview(ctx, *request.Change, at)
				err = e
				if e == nil {
					result.Projection = &p
				}
			case "create":
				v, e := coordinator.Create(ctx, *request.Change, at)
				err = e
				if e == nil {
					result.View = &v
				}
			case "apply":
				v, e := coordinator.Apply(ctx, *request.Change, at)
				err = e
				if e == nil {
					result.View = &v
				}
			}
			closeErr := coordinator.Close()
			if closeErr != nil && err == nil && (request.Operation == "create" || request.Operation == "apply") {
				closeErr = errors.Join(enrollmentconfig.ErrIndeterminate, closeErr)
			}
			return errors.Join(err, closeErr)
		}()
		return operationErr
	}
	authorityErr := s.withAuthority(ctx, message, request.owned(), run)
	mu.Lock()
	defer mu.Unlock()
	active = false
	if !called || invalid {
		authorityErr = errors.Join(authorityErr, enrollmentconfig.ErrUnauthorized)
	}
	err = errors.Join(authorityErr, operationErr)
	if err != nil {
		if called && operationErr == nil && (request.Operation == "create" || request.Operation == "apply") {
			err = errors.Join(enrollmentconfig.ErrIndeterminate, err)
		}
		return reply, err
	}
	return result, nil
}

// Only safe codes cross this boundary. The returned Go error preserves original
// causes for protected service diagnostics; never send it verbatim to a client.
func agentIdentityConfigErrorCode(err error) string {
	switch {
	case err == nil:
		return ""
	case errors.Is(err, enrollmentconfig.ErrIndeterminate):
		return "indeterminate"
	case errors.Is(err, enrollmentconfig.ErrUnauthorized):
		return "denied"
	case errors.Is(err, enrollmentconfig.ErrFormat), errors.Is(err, enrollmentconfig.ErrVersion), errors.Is(err, enrollmentconfig.ErrMode):
		return "invalid_request"
	case errors.Is(err, enrollmentconfig.ErrConflict), errors.Is(err, enrollmentconfig.ErrEvent), errors.Is(err, enrollmentconfig.ErrExists):
		return "conflict"
	case errors.Is(err, enrollmentconfig.ErrBusy):
		return "busy"
	case errors.Is(err, enrollmentconfig.ErrPending):
		return "pending"
	case errors.Is(err, enrollmentconfig.ErrNotConfigured):
		return "not_configured"
	default:
		return "unavailable"
	}
}

type agentIdentityConfigChannel struct {
	conn *net.UnixConn
	used atomic.Bool
}

// Configure before releasing any sender. Neither this constructor nor serve
// installs a listener, chooses a buffer policy, deadline, goroutine or retry.
// On success the channel owns the connection for one serve call and closes it
// on every terminal outcome, so failed replies cannot leave the peer waiting.
// The owner can interrupt a blocked receive by closing this connection. A context
// alone does not interrupt that server-side socket read.
func newAgentIdentityConfigChannel(conn *net.UnixConn) (*agentIdentityConfigChannel, error) {
	if err := configureAgentIdentityChannel(conn); err != nil {
		return nil, err
	}
	return &agentIdentityConfigChannel{conn: conn}, nil
}

func (c *agentIdentityConfigChannel) serve(ctx context.Context, buffer []byte, service *agentIdentityConfigService) (resultErr error) {
	if c == nil || c.conn == nil {
		return enrollmentconfig.ErrUnauthorized
	}
	if !c.used.CompareAndSwap(false, true) {
		return errAgentIdentityConfigUsed
	}
	defer func() { resultErr = errors.Join(resultErr, agentIdentityConfigCloseError(c.conn.Close())) }()
	if err := ctx.Err(); err != nil {
		return err
	}
	message, err := receiveAgentIdentityMessage(c.conn, buffer)
	if err != nil {
		return fmt.Errorf("receive configuration request: %w", err)
	}
	reply, operationErr := service.handle(ctx, message)
	closeErr := message.Close()
	if closeErr != nil {
		if operationErr == nil && (reply.Operation == "create" || reply.Operation == "apply") {
			closeErr = errors.Join(enrollmentconfig.ErrIndeterminate, closeErr)
		}
		operationErr = errors.Join(operationErr, closeErr)
		reply.View, reply.Projection = nil, nil
	}
	reply.Error = agentIdentityConfigErrorCode(operationErr)
	raw, encodeErr := json.Marshal(reply)
	if encodeErr != nil {
		return errors.Join(operationErr, enrollmentconfig.ErrIndeterminate, fmt.Errorf("encode configuration reply: %w", encodeErr))
	}
	n, sendErr := c.conn.Write(raw)
	if sendErr == nil && n != len(raw) {
		sendErr = io.ErrShortWrite
	}
	if sendErr != nil {
		if operationErr == nil && (reply.Operation == "create" || reply.Operation == "apply") {
			sendErr = errors.Join(enrollmentconfig.ErrIndeterminate, sendErr)
		}
		return errors.Join(operationErr, fmt.Errorf("send configuration reply: %w", sendErr))
	}
	return operationErr
}
