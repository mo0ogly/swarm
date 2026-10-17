//go:build linux && amd64

package engine

import (
	"fmt"
	"golang.org/x/sys/unix"
	"os"
	"path/filepath"
	"runtime"
	"strconv"
	"syscall"
	"unsafe"
)

func trustSealedLandlock(work string) (int, error) {
	abi, _, e := unix.RawSyscall(unix.SYS_LANDLOCK_CREATE_RULESET, 0, 0, unix.LANDLOCK_CREATE_RULESET_VERSION)
	if e != 0 || abi < 6 {
		return 0, trustSealedFailure()
	}
	attrs := unix.LandlockRulesetAttr{Access_fs: 0xffff, Access_net: 3, Scoped: 3}
	fd, _, e := unix.RawSyscall(unix.SYS_LANDLOCK_CREATE_RULESET, uintptr(unsafe.Pointer(&attrs)), unsafe.Sizeof(attrs), 0)
	if e != 0 {
		return 0, e
	}
	defer unix.Close(int(fd))
	dir, err := unix.Open(work, unix.O_PATH|unix.O_CLOEXEC, 0)
	if err != nil {
		return 0, err
	}
	defer unix.Close(dir)
	rights := uint64(unix.LANDLOCK_ACCESS_FS_READ_FILE | unix.LANDLOCK_ACCESS_FS_READ_DIR | unix.LANDLOCK_ACCESS_FS_WRITE_FILE | unix.LANDLOCK_ACCESS_FS_MAKE_REG | unix.LANDLOCK_ACCESS_FS_REMOVE_FILE | unix.LANDLOCK_ACCESS_FS_MAKE_DIR | unix.LANDLOCK_ACCESS_FS_REMOVE_DIR | unix.LANDLOCK_ACCESS_FS_MAKE_SYM | unix.LANDLOCK_ACCESS_FS_REFER | unix.LANDLOCK_ACCESS_FS_TRUNCATE)
	rule := unix.LandlockPathBeneathAttr{Allowed_access: rights, Parent_fd: int32(dir)}
	if _, _, e = unix.RawSyscall6(unix.SYS_LANDLOCK_ADD_RULE, fd, unix.LANDLOCK_RULE_PATH_BENEATH, uintptr(unsafe.Pointer(&rule)), 0, 0, 0); e != 0 {
		return 0, e
	}
	if err = unix.Prctl(unix.PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0); err != nil {
		return 0, err
	}
	if _, _, e = unix.RawSyscall(unix.SYS_LANDLOCK_RESTRICT_SELF, fd, 0, 0); e != 0 {
		return 0, e
	}
	return int(abi), nil
}
func trustSealedSeccomp() error {
	filters := []unix.SockFilter{{Code: unix.BPF_LD | unix.BPF_W | unix.BPF_ABS, K: 4}, {Code: unix.BPF_JMP | unix.BPF_JEQ | unix.BPF_K, K: unix.AUDIT_ARCH_X86_64, Jt: 1}, {Code: unix.BPF_RET | unix.BPF_K, K: unix.SECCOMP_RET_KILL_PROCESS}, {Code: unix.BPF_LD | unix.BPF_W | unix.BPF_ABS, K: 0}, {Code: unix.BPF_JMP | unix.BPF_JGE | unix.BPF_K, K: 0x40000000, Jf: 1}, {Code: unix.BPF_RET | unix.BPF_K, K: unix.SECCOMP_RET_KILL_PROCESS}}
	blocked := []uint32{unix.SYS_SOCKET, unix.SYS_CONNECT, unix.SYS_ACCEPT, unix.SYS_SENDTO, unix.SYS_RECVFROM, unix.SYS_SENDMSG, unix.SYS_RECVMSG, unix.SYS_BIND, unix.SYS_LISTEN, unix.SYS_GETSOCKNAME, unix.SYS_GETPEERNAME, unix.SYS_SOCKETPAIR, unix.SYS_SETSOCKOPT, unix.SYS_GETSOCKOPT, unix.SYS_ACCEPT4, unix.SYS_RECVMMSG, unix.SYS_SENDMMSG, unix.SYS_IO_URING_SETUP, unix.SYS_IO_URING_ENTER, unix.SYS_IO_URING_REGISTER, unix.SYS_EXECVE, unix.SYS_EXECVEAT, unix.SYS_PTRACE, unix.SYS_PROCESS_VM_READV, unix.SYS_PROCESS_VM_WRITEV, unix.SYS_PIDFD_GETFD, unix.SYS_SETPGID, unix.SYS_SETSID, unix.SYS_KILL, unix.SYS_TKILL, unix.SYS_BPF, unix.SYS_PERF_EVENT_OPEN}
	for _, nr := range blocked {
		filters = append(filters, unix.SockFilter{Code: unix.BPF_JMP | unix.BPF_JEQ | unix.BPF_K, K: nr, Jf: 1}, unix.SockFilter{Code: unix.BPF_RET | unix.BPF_K, K: unix.SECCOMP_RET_ERRNO | uint32(unix.EPERM)})
	}
	filters = append(filters, unix.SockFilter{Code: unix.BPF_RET | unix.BPF_K, K: unix.SECCOMP_RET_ALLOW})
	prog := unix.SockFprog{Len: uint16(len(filters)), Filter: &filters[0]}
	ret, _, e := unix.RawSyscall6(unix.SYS_SECCOMP, unix.SECCOMP_SET_MODE_FILTER, unix.SECCOMP_FILTER_FLAG_TSYNC, uintptr(unsafe.Pointer(&prog)), 0, 0, 0)
	runtime.KeepAlive(filters)
	if e != 0 {
		return e
	}
	if ret != 0 {
		return fmt.Errorf("seccomp synchronization refused")
	}
	return nil
}
func trustSealedChildProbe(input trustSealedInput) (trustSealedResult, error) {
	runtime.LockOSThread()
	result := trustSealedResult{Checks: map[string]bool{}}
	for _, fd := range []int{0, 1, 2} {
		var stat unix.Stat_t
		if unix.Fstat(fd, &stat) != nil || stat.Mode&unix.S_IFMT != unix.S_IFIFO {
			return result, trustSealedFailure()
		}
	}
	result.Checks["minimal_environment"] = len(os.Environ()) == 2 && os.Getenv("LANG") == "C" && os.Getenv("TZ") == "UTC"
	if unix.Prctl(unix.PR_SET_DUMPABLE, 0, 0, 0, 0) != nil {
		return result, trustSealedFailure()
	}
	dump, err := unix.PrctlRetInt(unix.PR_GET_DUMPABLE, 0, 0, 0, 0)
	if err != nil {
		return result, err
	}
	result.Checks["not_dumpable"] = dump == 0
	entries, err := os.ReadDir("/proc/self/fd")
	if err != nil {
		return result, err
	}
	found := false
	closed := true
	for _, entry := range entries {
		fd, err := strconv.Atoi(entry.Name())
		if err != nil || fd < 3 {
			continue
		}
		flags, err := unix.FcntlInt(uintptr(fd), unix.F_GETFD, 0)
		if err != nil || flags&unix.FD_CLOEXEC != 0 {
			continue
		}
		found = true
		if unix.Close(fd) != nil {
			closed = false
		}
		if _, err = unix.FcntlInt(uintptr(fd), unix.F_GETFD, 0); err != unix.EBADF {
			closed = false
		}
	}
	result.Checks["foreign_fd_closed"] = found && closed
	result.ABI, err = trustSealedLandlock(input.Work)
	if err != nil {
		return result, err
	}
	if err = trustSealedSeccomp(); err != nil {
		return result, err
	}
	nnp, err := unix.PrctlRetInt(unix.PR_GET_NO_NEW_PRIVS, 0, 0, 0, 0)
	if err != nil {
		return result, err
	}
	result.Checks["no_new_privs"] = nnp == 1
	raw, err := os.ReadFile(filepath.Join(input.Work, "allowed"))
	result.Checks["work_read"] = err == nil && string(raw) == "allowed"
	result.Checks["work_write"] = os.WriteFile(filepath.Join(input.Work, "result"), []byte("owned result"), 0600) == nil
	paths := map[string]string{"outside_denied": input.Private, "traversal_denied": input.Work + "/../private/token", "symlink_denied": filepath.Join(input.Work, "linked"), "proc_denied": "/proc/self/status"}
	for name, path := range paths {
		_, err := os.ReadFile(path)
		result.Checks[name] = os.IsPermission(err)
	}
	pid, err := syscall.ForkExec("/bin/true", []string{"true"}, &syscall.ProcAttr{Files: []uintptr{0, 1, 2}, Env: []string{}})
	result.Checks["binary_denied"] = err == unix.EPERM || err == unix.EACCES
	if err == nil {
		var status syscall.WaitStatus
		syscall.Wait4(pid, &status, 0, nil)
	}
	for name, domain := range map[string]int{"tcp_denied": unix.AF_INET, "udp_denied": unix.AF_INET, "unix_denied": unix.AF_UNIX} {
		kind := unix.SOCK_STREAM
		if name == "udp_denied" {
			kind = unix.SOCK_DGRAM
		}
		fd, err := unix.Socket(domain, kind, 0)
		result.Checks[name] = err == unix.EPERM || err == unix.EACCES
		if err == nil {
			unix.Close(fd)
		}
	}
	return result, nil
}
