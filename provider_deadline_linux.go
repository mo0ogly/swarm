//go:build linux

package main

import (
	"golang.org/x/sys/unix"
	"time"
)

// CLOCK_BOOTTIME includes suspend and cannot jump with wall-clock corrections.
// The fallback remains finite on a kernel without CLOCK_BOOTTIME support.
func suspendAwareNow() time.Duration {
	var ts unix.Timespec
	if err := unix.ClockGettime(unix.CLOCK_BOOTTIME, &ts); err == nil {
		return time.Duration(ts.Nano())
	}
	return time.Duration(time.Now().UnixNano())
}
