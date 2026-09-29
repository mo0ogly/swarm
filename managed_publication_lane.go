//go:build linux

package main

import (
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"syscall"
)

// Workers keep running in parallel. Candidate checks, review and publication
// form one lane per work, so a sibling publication cannot invalidate a paid review.
// This separate lock does not hold the Git operation lock during inference.
func managedPublicationLane(root, work string) (func(), bool, error) {
	if !safeName(work) {
		return nil, false, fmt.Errorf("identifiant de mission invalide")
	}
	dir := filepath.Join(root, ".swarm", "managed", work)
	if err := os.MkdirAll(dir, 0700); err != nil {
		return nil, false, err
	}
	f, err := os.OpenFile(filepath.Join(dir, "publication.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		return nil, false, err
	}
	if err = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		f.Close()
		if errors.Is(err, syscall.EWOULDBLOCK) || errors.Is(err, syscall.EAGAIN) {
			return nil, false, nil
		}
		return nil, false, err
	}
	return func() { _ = syscall.Flock(int(f.Fd()), syscall.LOCK_UN); _ = f.Close() }, true, nil
}
