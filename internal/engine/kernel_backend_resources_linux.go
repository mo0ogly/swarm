//go:build linux

package engine

import (
	"errors"
	"fmt"
	"io"
	"reflect"
	"sync"
)

type kernelBackendStage struct {
	name string
	open func() (io.Closer, error)
}

type kernelBackendResources struct {
	once    sync.Once
	closers []io.Closer
	err     error
}

func acquireKernelBackend(stages []kernelBackendStage) (*kernelBackendResources, error) {
	r := &kernelBackendResources{}
	for _, stage := range stages {
		c, err := stage.open()
		if c != nil && !(reflect.ValueOf(c).Kind() == reflect.Ptr && reflect.ValueOf(c).IsNil()) {
			r.closers = append(r.closers, c)
		}
		if err != nil {
			closeErr := r.Close()
			if closeErr != nil {
				closeErr = &kernelBackendCleanupError{Err: closeErr}
			}
			return nil, errors.Join(fmt.Errorf("kernel backend %s: %w", stage.name, err), closeErr)
		}
	}
	return r, nil
}

func (r *kernelBackendResources) Close() error {
	r.once.Do(func() {
		for i := len(r.closers) - 1; i >= 0; i-- {
			r.err = errors.Join(r.err, r.closers[i].Close())
		}
	})
	return r.err
}

type kernelBackendCleanupError struct{ Err error }

func (e *kernelBackendCleanupError) Error() string { return "kernel backend cleanup: " + e.Err.Error() }
func (e *kernelBackendCleanupError) Unwrap() error { return e.Err }
func kernelBackendCleanAfterFailure(err error) bool {
	var failure *kernelBackendCleanupError
	return !errors.As(err, &failure)
}
