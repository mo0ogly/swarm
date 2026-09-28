//go:build linux

package main

import (
	"context"
	"strings"
	"testing"
	"time"
)

func TestManagedPublicationReservesWriterBeforeEvidenceChecks(t *testing.T) {
	for _, kind := range []string{"managed.integrated", "task.attempt-extension", "task.corrective-recovery", "review.managed.claim", "review.managed.result", "review.managed.batch.claim"} {
		t.Run(kind, func(t *testing.T) {
			s := storeTest(t)
			w := createTest(t, s)
			other, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer other.db.Close()
			ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
			defer cancel()
			conn, err := other.db.Conn(ctx)
			if err != nil {
				t.Fatal(err)
			}
			defer conn.Close()
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
			// Fail immediately on a real competing write while the first callback
			// remains blocked. No scheduler delay can stand in for contention.
			if _, err = conn.ExecContext(ctx, "PRAGMA busy_timeout=0"); err != nil {
				close(release)
				<-published
				t.Fatal(err)
			}
			_, writerErr := conn.ExecContext(ctx, "BEGIN IMMEDIATE")
			if writerErr == nil {
				_, _ = conn.ExecContext(ctx, "ROLLBACK")
			}
			close(release)
			err = <-published
			if writerErr == nil {
				t.Fatal("competing writer entered during evidence checks")
			}
			if !strings.Contains(writerErr.Error(), "SQLITE_BUSY") && !strings.Contains(writerErr.Error(), "database is locked") {
				t.Fatal("unexpected competing write error", writerErr)
			}
			if _, e := conn.ExecContext(ctx, "UPDATE works SET revision=revision WHERE id=?", w.ID); e != nil {
				t.Fatal("writer still blocked after publication", e)
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
