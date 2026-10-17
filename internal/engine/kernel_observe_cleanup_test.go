//go:build linux

package engine

import (
	"errors"
	"io"
	"testing"
)

type kernelObserveBadCloser struct{}

func (kernelObserveBadCloser) Close() error { return errors.New("owned fixture close failed") }
func TestKernelObserveFailedAcquisitionCleanup(t *testing.T) {
	_, err := acquireKernelBackend([]kernelBackendStage{{"owned", func() (io.Closer, error) { return kernelObserveBadCloser{}, nil }}, {"failure", func() (io.Closer, error) { return nil, errors.New("fixture acquisition failed") }}})
	if err == nil || kernelBackendCleanAfterFailure(err) {
		t.Fatal("failed cleanup reported clean after acquisition failure")
	}
}
