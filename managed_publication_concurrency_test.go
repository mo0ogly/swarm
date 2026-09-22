//go:build linux

package main

import (
	"testing"
	"time"
)

func TestManagedPublicationReservesWriterBeforeEvidenceChecks(t *testing.T) {
	for _, kind := range []string{"managed.integrated", "task.attempt-extension", "task.corrective-recovery"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			other, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer other.db.Close()
			checking := make(chan struct{})
			release := make(chan struct{})
			published := make(chan error, 1)
			go func() {
				_, err := s.mutate(w.ID, kind, "publication-concurrency", w.Revision, []byte(`{}`), func(current *Work) error {
					close(checking)
					<-release
					current.Summary = "publication checked"
					return nil
				})
				published <- err
			}()
			select {
			case <-checking:
			case err := <-published:
				t.Fatal("publication failed before checks", err)
			case <-time.After(5 * time.Second):
				t.Fatal("publication did not enter checks")
			}
			competing := make(chan error, 1)
			started := make(chan struct{})
			go func() {
				close(started)
				_, err := other.db.Exec("UPDATE works SET revision=revision WHERE id=?", w.ID)
				competing <- err
			}()
			<-started
			premature := false
			select {
			case <-competing:
				premature = true
			case <-time.After(100 * time.Millisecond):
			}
			close(release)
			err = <-published
			if !premature {
				if writerErr := <-competing; writerErr != nil {
					t.Fatal(writerErr)
				}
			}
			if premature {
				t.Fatal("competing writer entered during evidence checks; publication can lose its snapshot", err)
			}
			if err != nil {
				t.Fatal(err)
			}
			after, err := s.get(w.ID)
			if err != nil || after.Revision != w.Revision+1 || after.Summary != "publication checked" {
				t.Fatal("publication not committed once", err)
			}
		})
	}
}
