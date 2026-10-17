//go:build linux

package enrollmentconfig

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// Coordinator keeps the config and its witness in distinct owned directories.
// It is not a separate OS authority or protection against restoring both stores.
// No application-facing adapter may bypass it to learn a head from config alone.
type Coordinator struct {
	config  *Store
	witness *Store
}

func within(parent, child string) bool {
	rel, err := filepath.Rel(parent, child)
	return err == nil && rel != ".." && !strings.HasPrefix(rel, ".."+string(os.PathSeparator))
}

func OpenCoordinated(configPath, witnessPath string, owner uint32, pins Pins, authorize Authorizer) (*Coordinator, error) {
	if !filepath.IsAbs(configPath) || !filepath.IsAbs(witnessPath) ||
		filepath.Clean(configPath) != configPath || filepath.Clean(witnessPath) != witnessPath ||
		within(configPath, witnessPath) || within(witnessPath, configPath) {
		return nil, ErrPath
	}
	config, err := Open(configPath, owner, pins, authorize)
	if err != nil {
		return nil, err
	}
	witness, err := Open(witnessPath, owner, pins, authorize)
	if err != nil {
		return nil, errors.Join(err, config.Close())
	}
	a, ae := config.dir.Stat()
	b, be := witness.dir.Stat()
	if ae != nil || be != nil || os.SameFile(a, b) {
		return nil, errors.Join(ErrPath, ae, be, config.Close(), witness.Close())
	}
	return &Coordinator{config: config, witness: witness}, nil
}

func (c *Coordinator) Close() error {
	if c == nil {
		return nil
	}
	return errors.Join(c.config.Close(), c.witness.Close())
}

func (c *Coordinator) reserve(ctx context.Context) (func() error, error) {
	if c == nil {
		return nil, os.ErrClosed
	}
	a, err := c.config.reserve(ctx)
	if err != nil {
		return nil, err
	}
	b, err := c.witness.reserve(ctx)
	if err != nil {
		return nil, errors.Join(err, a())
	}
	return func() error { return errors.Join(b(), a()) }, nil
}

// pair never initializes a witness from a configuration or silently resets one.
func (c *Coordinator) pair() (*Snapshot, *witnessRecord, error) {
	s, err := c.config.load()
	var current *Snapshot
	if err == nil {
		current = &s
	} else if !errors.Is(err, ErrNotConfigured) {
		return nil, nil, err
	}
	w, err := c.loadWitness()
	if errors.Is(err, ErrWitnessMissing) && current == nil {
		return nil, nil, nil
	}
	if err != nil {
		return nil, nil, err
	}
	return current, w, nil
}

func committedView(current *Snapshot, w *witnessRecord) (View, error) {
	if w == nil {
		return View{}, ErrNotConfigured
	}
	if w.Pending != nil {
		return View{}, ErrPending
	}
	if current == nil {
		return View{}, ErrNotConfigured
	}
	v, err := view(*current)
	if err != nil {
		return View{}, err
	}
	if v.Head != w.Committed {
		return View{}, ErrHead
	}
	return v, nil
}

func (c *Coordinator) barriers() error {
	for _, s := range []*Store{c.config, c.witness} {
		role := "configuration"
		if s == c.witness {
			role = "witness"
		}
		if err := s.safeDirectory(); err != nil {
			return fmt.Errorf("check %s directory: %w", role, err)
		}
		if err := s.syncDir(); err != nil {
			return fmt.Errorf("sync %s directory: %w: %w", role, ErrIndeterminate, err)
		}
	}
	return nil
}

func (c *Coordinator) Read(ctx context.Context) (result View, err error) {
	release, err := c.reserve(ctx)
	if err != nil {
		return View{}, err
	}
	defer finish(release, &result, &err)
	current, w, err := c.pair()
	if err != nil {
		return View{}, err
	}
	result, err = committedView(current, w)
	if err != nil {
		return View{}, err
	}
	if err = c.barriers(); err != nil {
		return View{}, err
	}
	if err = ctx.Err(); err != nil {
		return View{}, err
	}
	return result, nil
}

func (c *Coordinator) Preview(ctx context.Context, change Change, at time.Time) (result Projection, err error) {
	if err = change.validate(); err != nil {
		return Projection{}, err
	}
	if at.IsZero() {
		return Projection{}, ErrFormat
	}
	release, err := c.reserve(ctx)
	if err != nil {
		return Projection{}, err
	}
	defer func() {
		if e := release(); e != nil {
			result, err = Projection{}, errors.Join(err, storageError(e))
		}
	}()
	current, w, err := c.pair()
	if err != nil {
		return Projection{}, err
	}
	if w != nil {
		v, e := committedView(current, w)
		if e != nil {
			return Projection{}, e
		}
		result.Base = v.Head
	}
	next, replay, err := c.config.prepare(ctx, current, change, at)
	if err != nil {
		return Projection{}, err
	}
	result.Next, result.Replay = next, replay
	return result, nil
}

func (c *Coordinator) Create(ctx context.Context, change Change, at time.Time) (View, error) {
	if change.Expected != (Head{}) {
		return View{}, ErrConflict
	}
	return c.change(ctx, change, at, true)
}
func (c *Coordinator) Apply(ctx context.Context, change Change, at time.Time) (View, error) {
	return c.change(ctx, change, at, false)
}

func (c *Coordinator) change(ctx context.Context, change Change, at time.Time, create bool) (result View, err error) {
	if err = change.validate(); err != nil {
		return View{}, err
	}
	if at.IsZero() {
		return View{}, ErrFormat
	}
	release, err := c.reserve(ctx)
	if err != nil {
		return View{}, err
	}
	defer finish(release, &result, &err)
	current, w, err := c.pair()
	if err != nil {
		return View{}, err
	}
	if w != nil && w.Pending != nil {
		if change != w.Pending.Entry.Change {
			return View{}, ErrPending
		}
		return c.resume(ctx, current, w)
	}
	if w == nil {
		if !create {
			return View{}, ErrNotConfigured
		}
	} else if _, err = committedView(current, w); err != nil {
		return View{}, err
	}
	result, replay, err := c.config.prepare(ctx, current, change, at)
	if err != nil {
		return View{}, err
	}
	if replay {
		if err = c.barriers(); err != nil {
			return View{}, err
		}
		return result, nil
	}
	base := Head{}
	if w != nil {
		base = w.Committed
	}
	intent := witnessRecord{Schema: 1, Pins: c.config.pins, Committed: base, Pending: &witnessIntent{Entry: result.Snapshot.History[len(result.Snapshot.History)-1], Target: result.Head}}
	if err = c.saveWitness(intent, w == nil); err != nil {
		return View{}, fmt.Errorf("prepare witness: %w", err)
	}
	return c.commitIntent(ctx, current, result, intent, false)
}

func (c *Coordinator) Recover(ctx context.Context) (result View, err error) {
	release, err := c.reserve(ctx)
	if err != nil {
		return View{}, err
	}
	defer finish(release, &result, &err)
	current, w, err := c.pair()
	if err != nil {
		return View{}, err
	}
	if w == nil {
		return View{}, ErrNotConfigured
	}
	if w.Pending == nil {
		result, err = committedView(current, w)
		if err != nil {
			return View{}, err
		}
		if err = c.barriers(); err != nil {
			return View{}, err
		}
		return result, nil
	}
	return c.resume(ctx, current, w)
}

func (c *Coordinator) resume(ctx context.Context, current *Snapshot, w *witnessRecord) (View, error) {
	intent := w.Pending
	if current == nil {
		if w.Committed != (Head{}) {
			return View{}, ErrNotConfigured
		}
	} else {
		v, err := view(*current)
		if err != nil {
			return View{}, err
		}
		if v.Head != w.Committed && v.Head != intent.Target {
			return View{}, ErrHead
		}
	}
	result, _, err := c.config.prepare(ctx, current, intent.Entry.Change, intent.Entry.At)
	if err != nil {
		return View{}, err
	}
	last := result.Snapshot.History[len(result.Snapshot.History)-1]
	if result.Head != intent.Target || last.Change != intent.Entry.Change || last.Actor != intent.Entry.Actor || !last.At.Equal(intent.Entry.At) {
		return View{}, ErrHead
	}
	return c.commitIntent(ctx, current, result, *w, true)
}

func (c *Coordinator) commitIntent(ctx context.Context, current *Snapshot, result View, w witnessRecord, resuming bool) (View, error) {
	if err := c.config.safeDirectory(); err != nil {
		return View{}, fmt.Errorf("check configuration directory: %w", err)
	}
	if resuming {
		if err := c.witness.safeDirectory(); err != nil {
			return View{}, fmt.Errorf("check witness directory: %w", err)
		}
		if err := c.witness.syncDir(); err != nil {
			return View{}, fmt.Errorf("sync prepared witness: %w: %w", ErrIndeterminate, err)
		}
	}
	if err := ctx.Err(); err != nil {
		return View{}, errors.Join(ErrPending, err)
	}
	alreadyTarget := false
	if current != nil {
		v, err := view(*current)
		if err != nil {
			return View{}, err
		}
		alreadyTarget = v.Head == w.Pending.Target
	}
	if !alreadyTarget {
		raw, err := Encode(result.Snapshot)
		if err != nil {
			return View{}, err
		}
		if err = c.config.persist(raw, current == nil); err != nil {
			return View{}, fmt.Errorf("write configuration: %w", err)
		}
	} else if err := c.config.syncDir(); err != nil {
		return View{}, fmt.Errorf("sync recovered configuration: %w: %w", ErrIndeterminate, err)
	}
	w.Committed, w.Pending = result.Head, nil
	if err := c.saveWitness(w, false); err != nil {
		return View{}, fmt.Errorf("confirm witness: %w", err)
	}
	return result, nil
}
