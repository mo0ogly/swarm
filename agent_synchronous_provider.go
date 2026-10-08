package main

import "strings"

// A managed attempt ends with its provider process. A provider wakeup cannot
// resume that attempt and must not replace delivery of the current result.
func synchronousAttemptProvider(p Provider, mode string) Provider {
	if mode != "" || providerAdapter(p) != "claude" {
		return p
	}
	args := append([]string(nil), p.Args...)
	for i := len(args) - 1; i >= 0; i-- {
		for _, flag := range []string{"--disallowedTools", "--disallowed-tools"} {
			if strings.HasPrefix(args[i], flag+"=") {
				args[i] += ",ScheduleWakeup"
				p.Args = args
				return p
			}
			if args[i] == flag {
				if i+1 < len(args) && !strings.HasPrefix(args[i+1], "-") {
					args[i+1] += ",ScheduleWakeup"
				} else {
					args = append(args[:i+1], append([]string{"ScheduleWakeup"}, args[i+1:]...)...)
				}
				p.Args = args
				return p
			}
		}
	}
	p.Args = append(args, "--disallowedTools", "ScheduleWakeup")
	return p
}
