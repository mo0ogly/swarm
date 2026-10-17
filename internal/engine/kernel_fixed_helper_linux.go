//go:build linux

package engine

import (
	"errors"
	"golang.org/x/sys/unix"
	"os"
	"strconv"
)

func kernelFixedHelperSecurity(parent int, start string) error {
	if parent != os.Getppid() || len(os.Environ()) != 2 || os.Getenv("LANG") != "C" || os.Getenv("TZ") != "UTC" {
		return errors.New("fixed kernel helper environment or parent changed")
	}
	current, err := kernelObservationProcessStart(parent)
	if err != nil {
		return err
	}
	if current != start {
		return errors.New("kernel helper parent identity changed")
	}
	if err = unix.Prctl(unix.PR_SET_DUMPABLE, 0, 0, 0, 0); err != nil {
		return err
	}
	if err = unix.Prctl(unix.PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0); err != nil {
		return err
	}
	entries, err := os.ReadDir("/proc/self/fd")
	if err != nil {
		return err
	}
	for _, entry := range entries {
		fd, e := strconv.Atoi(entry.Name())
		if e != nil || fd < 3 {
			continue
		}
		flags, e := unix.FcntlInt(uintptr(fd), unix.F_GETFD, 0)
		if errors.Is(e, unix.EBADF) {
			continue
		}
		if e != nil {
			return e
		}
		if flags&unix.FD_CLOEXEC == 0 {
			if e = unix.Close(fd); e != nil {
				return e
			}
		}
	}
	return nil
}
