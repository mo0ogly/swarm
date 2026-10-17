//go:build linux

package enrollmentconfig

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"golang.org/x/sys/unix"
)

type Store struct {
	mu        sync.Mutex
	dir       *os.File
	lock      *os.File
	owner     uint32
	pins      Pins
	authorize Authorizer
	syncDir   func() error
}

func storageError(err error) error { return fmt.Errorf("%w: %w", ErrStorage, err) }
func pathError(err error) error    { return fmt.Errorf("%w: %w", ErrPath, err) }

// Open requires an existing absolute private directory under the actual service
// UID. Every component is opened without following symlinks. Ancestors must be
// owned by root/service and not group/other writable, except a root-owned sticky
// directory such as /tmp. The final directory must be exactly 0700. Open may
// initialize config.lock; no config or default values are created.
//
// These checks are not confinement of same-UID agents. Bootstrap must protect
// the handle, pins, authorizer and independent checkpoint outside agent control.
func Open(path string, owner uint32, pins Pins, authorize Authorizer) (*Store, error) {
	if !pins.valid() {
		return nil, ErrPins
	}
	if authorize == nil {
		return nil, ErrUnauthorized
	}
	if uint32(os.Geteuid()) != owner || !filepath.IsAbs(path) || filepath.Clean(path) != path || path == "/" || strings.IndexByte(path, 0) >= 0 {
		return nil, ErrPath
	}
	fd, err := unix.Open("/", unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC, 0)
	if err != nil {
		return nil, pathError(err)
	}
	parts := strings.Split(strings.TrimPrefix(path, "/"), "/")
	for i, component := range parts {
		next, openErr := unix.Openat(fd, component, unix.O_RDONLY|unix.O_DIRECTORY|unix.O_CLOEXEC|unix.O_NOFOLLOW, 0)
		closeErr := unix.Close(fd)
		if openErr != nil {
			return nil, pathError(errors.Join(openErr, closeErr))
		}
		fd = next
		var st unix.Stat_t
		if err = unix.Fstat(fd, &st); err == nil {
			if i == len(parts)-1 {
				if st.Uid != owner || st.Mode&07777 != 0700 {
					err = ErrPath
				}
			} else if st.Uid != 0 && st.Uid != owner || st.Mode&0022 != 0 && !(st.Uid == 0 && st.Mode&unix.S_ISVTX != 0) {
				err = ErrPath
			}
		}
		if err != nil || closeErr != nil {
			return nil, pathError(errors.Join(err, closeErr, unix.Close(fd)))
		}
	}
	s := &Store{dir: os.NewFile(uintptr(fd), "installation-config-directory"), owner: owner, pins: pins, authorize: authorize}
	lockFD, err := unix.Openat(fd, "config.lock", unix.O_RDWR|unix.O_CREAT|unix.O_NOFOLLOW|unix.O_CLOEXEC|unix.O_NONBLOCK, 0600)
	if err != nil {
		return nil, pathError(errors.Join(err, s.dir.Close()))
	}
	s.lock = os.NewFile(uintptr(lockFD), "installation-config-lock")
	if err = s.safeFile(lockFD); err != nil {
		return nil, errors.Join(err, s.lock.Close(), s.dir.Close())
	}
	s.syncDir = s.dir.Sync
	return s, nil
}

func (s *Store) safeFile(fd int) error {
	var st unix.Stat_t
	if err := unix.Fstat(fd, &st); err != nil {
		return pathError(err)
	}
	if st.Mode&unix.S_IFMT != unix.S_IFREG || st.Mode&07777 != 0600 || st.Uid != s.owner || st.Nlink != 1 {
		return ErrPath
	}
	return nil
}

func (s *Store) safeDirectory() error {
	var st unix.Stat_t
	if err := unix.Fstat(int(s.dir.Fd()), &st); err != nil {
		return pathError(err)
	}
	if st.Mode&unix.S_IFMT != unix.S_IFDIR || st.Mode&07777 != 0700 || st.Uid != s.owner {
		return ErrPath
	}
	return nil
}

func (s *Store) Close() error {
	if s == nil {
		return nil
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	if s.dir == nil {
		return nil
	}
	err := errors.Join(s.lock.Close(), s.dir.Close())
	s.dir, s.lock = nil, nil
	return err
}

// reserve never waits or retries a contested reservation. This exposes actual
// contention to the supervisor instead of inventing a timeout or retry ceiling.
func (s *Store) reserve(ctx context.Context) (func() error, error) {
	if ctx == nil {
		return nil, ErrFormat
	}
	if err := ctx.Err(); err != nil {
		return nil, err
	}
	if s == nil || !s.mu.TryLock() {
		return nil, ErrBusy
	}
	if s.dir == nil {
		s.mu.Unlock()
		return nil, os.ErrClosed
	}
	if err := s.safeDirectory(); err != nil {
		s.mu.Unlock()
		return nil, err
	}
	if err := s.safeFile(int(s.lock.Fd())); err != nil {
		s.mu.Unlock()
		return nil, err
	}
	if err := unix.Flock(int(s.lock.Fd()), unix.LOCK_EX|unix.LOCK_NB); err != nil {
		s.mu.Unlock()
		if errors.Is(err, unix.EWOULDBLOCK) {
			return nil, ErrBusy
		}
		return nil, storageError(err)
	}
	return func() error {
		err := unix.Flock(int(s.lock.Fd()), unix.LOCK_UN)
		s.mu.Unlock()
		return err
	}, nil
}

func (s *Store) readDocument(name string) ([]byte, error) {
	fd, err := unix.Openat(int(s.dir.Fd()), name, unix.O_RDONLY|unix.O_NOFOLLOW|unix.O_CLOEXEC|unix.O_NONBLOCK, 0)
	if errors.Is(err, unix.ENOENT) {
		return nil, ErrNotConfigured
	}
	if err != nil {
		return nil, pathError(err)
	}
	f := os.NewFile(uintptr(fd), "installation-config")
	if err = s.safeFile(fd); err != nil {
		return nil, errors.Join(err, f.Close())
	}
	raw, readErr := io.ReadAll(f)
	if err = errors.Join(readErr, f.Close()); err != nil {
		return nil, storageError(err)
	}
	return raw, nil
}

func (s *Store) load() (Snapshot, error) {
	raw, err := s.readDocument("config.json")
	if err != nil {
		return Snapshot{}, err
	}
	snapshot, err := Decode(raw)
	if err != nil {
		return Snapshot{}, err
	}
	if snapshot.Pins != s.pins {
		return Snapshot{}, ErrPins
	}
	return snapshot, nil
}

func finish(release func() error, result *View, err *error) {
	if e := release(); e != nil {
		*err = errors.Join(*err, storageError(e))
		*result = View{}
	}
}

func (s *Store) Read(ctx context.Context, expected Head) (result View, err error) {
	if expected.Revision == 0 || !expected.valid() {
		return View{}, ErrHead
	}
	release, err := s.reserve(ctx)
	if err != nil {
		return View{}, err
	}
	defer finish(release, &result, &err)
	snapshot, err := s.load()
	if err != nil {
		return View{}, err
	}
	result, err = view(snapshot)
	if err != nil {
		return View{}, err
	}
	if result.Head != expected {
		return View{}, ErrHead
	}
	if err = ctx.Err(); err != nil {
		return View{}, err
	}
	return result, nil
}

func (s *Store) prepare(ctx context.Context, current *Snapshot, c Change, at time.Time) (View, bool, error) {
	var copy *Snapshot
	if current != nil {
		v := owned(*current)
		copy = &v
	}
	actor, err := s.authorize(ctx, copy, c)
	if err != nil {
		return View{}, false, fmt.Errorf("%w: %w", ErrUnauthorized, err)
	}
	if actor != s.pins.Principal {
		return View{}, false, ErrUnauthorized
	}
	if err = ctx.Err(); err != nil {
		return View{}, false, err
	}
	snapshot := Snapshot{Schema: 1, Pins: s.pins}
	base := Head{}
	if current != nil {
		snapshot = owned(*current)
		currentView, err := view(snapshot)
		if err != nil {
			return View{}, false, err
		}
		base = currentView.Head
		for i, entry := range snapshot.History {
			if entry.Change.EventID == c.EventID {
				if i == len(snapshot.History)-1 && entry.Change == c {
					return currentView, true, nil
				}
				return View{}, false, ErrEvent
			}
		}
	}
	if c.Expected != base {
		return View{}, false, ErrConflict
	}
	if base.Revision == ^uint64(0) {
		return View{}, false, ErrFormat
	}
	snapshot.History = append(snapshot.History, Entry{Change: c, Actor: actor, At: at.UTC()})
	// Preview and application validate the same full representation, including
	// fallible timestamp encoding, before deriving or returning a proposed head.
	if _, err := Encode(snapshot); err != nil {
		return View{}, false, err
	}
	result, err := view(snapshot)
	return result, false, err
}

func (s *Store) Preview(ctx context.Context, c Change, at time.Time) (result Projection, err error) {
	if err = c.validate(); err != nil {
		return Projection{}, err
	}
	if at.IsZero() {
		return Projection{}, ErrFormat
	}
	release, err := s.reserve(ctx)
	if err != nil {
		return Projection{}, err
	}
	defer func() {
		if e := release(); e != nil {
			result, err = Projection{}, errors.Join(err, storageError(e))
		}
	}()
	snapshot, err := s.load()
	var current *Snapshot
	if err == nil {
		current = &snapshot
		currentView, viewErr := view(snapshot)
		if viewErr != nil {
			return Projection{}, viewErr
		}
		result.Base = currentView.Head
	} else if !errors.Is(err, ErrNotConfigured) {
		return Projection{}, err
	}
	next, replay, err := s.prepare(ctx, current, c, at)
	if err != nil {
		return Projection{}, err
	}
	result.Next, result.Replay = next, replay
	return result, nil
}

func (s *Store) Create(ctx context.Context, c Change, at time.Time) (View, error) {
	if c.Expected != (Head{}) {
		return View{}, ErrConflict
	}
	return s.change(ctx, c, at, true)
}

func (s *Store) Apply(ctx context.Context, c Change, at time.Time) (View, error) {
	return s.change(ctx, c, at, false)
}

func (s *Store) change(ctx context.Context, c Change, at time.Time, create bool) (result View, err error) {
	if err = c.validate(); err != nil {
		return View{}, err
	}
	if at.IsZero() {
		return View{}, ErrFormat
	}
	release, err := s.reserve(ctx)
	if err != nil {
		return View{}, err
	}
	defer finish(release, &result, &err)
	snapshot, err := s.load()
	var current *Snapshot
	if err == nil {
		current = &snapshot
	} else if !create || !errors.Is(err, ErrNotConfigured) {
		return View{}, err
	}
	result, replay, err := s.prepare(ctx, current, c, at)
	if err != nil {
		return View{}, err
	}
	if create && current != nil && !replay {
		return View{}, ErrExists
	}
	if replay {
		// Retry a current event after an uncertain directory sync without adding
		// history. Revalidate authorization and complete the durability barrier.
		if err = s.syncDir(); err != nil {
			return View{}, fmt.Errorf("%w: %w", ErrIndeterminate, err)
		}
		return result, nil
	}
	raw, err := Encode(result.Snapshot)
	if err != nil {
		return View{}, err
	}
	if err = ctx.Err(); err != nil {
		return View{}, err
	}
	if err = s.persist(raw, create); err != nil {
		return View{}, err
	}
	return result, nil
}

func (s *Store) persist(raw []byte, create bool) (err error) {
	return s.persistDocument("config.json", raw, create)
}

// Names are private fixed basenames, never paths received from callers.
func (s *Store) persistDocument(destination string, raw []byte, create bool) (err error) {
	if err = s.safeDirectory(); err != nil {
		return err
	}
	var nonce [16]byte
	if _, err = rand.Read(nonce[:]); err != nil {
		return storageError(err)
	}
	name := ".config-" + hex.EncodeToString(nonce[:])
	dirFD := int(s.dir.Fd())
	fd, err := unix.Openat(dirFD, name, unix.O_WRONLY|unix.O_CREAT|unix.O_EXCL|unix.O_NOFOLLOW|unix.O_CLOEXEC, 0600)
	if err != nil {
		return storageError(err)
	}
	f := os.NewFile(uintptr(fd), "installation-config-temporary")
	closed := false
	defer func() {
		if !closed {
			err = errors.Join(err, f.Close())
		}
		if e := unix.Unlinkat(dirFD, name, 0); e != nil && !errors.Is(e, unix.ENOENT) {
			err = errors.Join(err, storageError(e))
		}
	}()
	if err = s.safeFile(fd); err != nil {
		return err
	}
	if n, e := f.Write(raw); e != nil || n != len(raw) {
		return storageError(errors.Join(e, io.ErrShortWrite))
	}
	if err = f.Sync(); err != nil {
		return storageError(err)
	}
	if err = f.Close(); err != nil {
		closed = true
		return storageError(err)
	}
	closed = true
	if create {
		err = unix.Renameat2(dirFD, name, dirFD, destination, unix.RENAME_NOREPLACE)
	} else {
		err = unix.Renameat(dirFD, name, dirFD, destination)
	}
	if errors.Is(err, unix.EEXIST) {
		return ErrExists
	}
	if err != nil {
		return storageError(err)
	}
	if err = s.syncDir(); err != nil {
		return fmt.Errorf("%w: %w", ErrIndeterminate, err)
	}
	return nil
}
