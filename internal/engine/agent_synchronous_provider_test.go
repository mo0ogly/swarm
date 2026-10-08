package engine

import (
	"reflect"
	"testing"
)

func TestSynchronousAttemptPreservesProviderDenials(t *testing.T) {
	for _, tc := range []struct{ args, want []string }{
		{[]string{"-p"}, []string{"-p", "--disallowedTools", "ScheduleWakeup"}},
		{[]string{"--disallowedTools", "Bash,Edit", "--model", "sonnet"}, []string{"--disallowedTools", "Bash,Edit,ScheduleWakeup", "--model", "sonnet"}},
		{[]string{"--disallowed-tools=Bash"}, []string{"--disallowed-tools=Bash,ScheduleWakeup"}},
	} {
		original := append([]string(nil), tc.args...)
		got := synchronousAttemptProvider(Provider{Command: "/usr/bin/claude", Args: tc.args}, "")
		if !reflect.DeepEqual(got.Args, tc.want) || !reflect.DeepEqual(tc.args, original) {
			t.Fatalf("got %v; input %v", got.Args, tc.args)
		}
	}
}
func TestSynchronousAttemptLeavesOtherModesAndProviders(t *testing.T) {
	for _, tc := range []struct{ command, mode string }{{"/usr/bin/codex", ""}, {"/usr/bin/claude", "terminal"}, {"/usr/bin/claude", "dialogue"}, {"/custom/wrapper", ""}} {
		p := Provider{Command: tc.command, Args: []string{"original"}}
		if got := synchronousAttemptProvider(p, tc.mode); !reflect.DeepEqual(got, p) {
			t.Fatalf("changed %s/%s", tc.command, tc.mode)
		}
	}
}
