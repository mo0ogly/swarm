//go:build linux

package engine

import (
	"bufio"
	"encoding/json"
	"errors"
	"io"
	"net"
)

const kernelObserveProtocol = "swarm-kernel-observe-v1"

type kernelObserveBootstrap struct {
	Protocol    string                   `json:"protocol"`
	Nonce       string                   `json:"nonce"`
	Socket      string                   `json:"socket"`
	Parent      int                      `json:"parent"`
	ParentStart string                   `json:"parent_start"`
	Policy      TrustKernelObservePolicy `json:"policy"`
	Source      kernelObservationLease   `json:"source"`
}
type kernelObservePacket struct {
	Type     string          `json:"type"`
	Protocol string          `json:"protocol,omitempty"`
	Nonce    string          `json:"nonce,omitempty"`
	Event    json.RawMessage `json:"event,omitempty"`
	Result   json.RawMessage `json:"result,omitempty"`
}

func kernelObserveSend(w io.Writer, p kernelObservePacket) error { return json.NewEncoder(w).Encode(p) }
func kernelObserveAuthorize(c *net.UnixConn, r *bufio.Reader, l kernelObservationLease) (kernelObservationLease, error) {
	if err := kernelObserveSend(c, kernelObservePacket{Type: "authorize"}); err != nil {
		return kernelObservationLease{}, err
	}
	var ack struct {
		Authorized bool `json:"authorized"`
	}
	if err := kernelCheckRead(r, &ack); err != nil {
		return kernelObservationLease{}, err
	}
	if !ack.Authorized {
		return kernelObservationLease{}, errors.New("kernel observer authorization withdrawn")
	}
	return l, nil
}
