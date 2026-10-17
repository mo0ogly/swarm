//go:build linux

package engine

import (
	"os"
	"os/exec"
	"strconv"
	"testing"
)

func TestTrustFixedHelperPinnedImage(t *testing.T) {
	for _, extra := range []bool{false, true} {
		t.Run(strconv.FormatBool(extra), func(t *testing.T) {
			cmd := exec.Command("/untrusted/replaced/server", "_trust_secret")
			var inherited *os.File
			if extra {
				var err error
				inherited, err = os.Open("/dev/null")
				if err != nil {
					t.Fatal(err)
				}
				defer inherited.Close()
				cmd.ExtraFiles = []*os.File{inherited}
			}
			image, err := pinTrustFixedHelperImage(cmd)
			if err != nil {
				t.Fatal(err)
			}
			current, err := os.Stat("/proc/self/exe")
			if err != nil {
				t.Fatal(err)
			}
			actual, err := image.Stat()
			if err != nil || !os.SameFile(current, actual) {
				t.Fatal("running image not pinned")
			}
			index := 3 + len(cmd.ExtraFiles) - 1
			if cmd.Path != "/proc/self/fd/"+strconv.Itoa(index) || cmd.ExtraFiles[len(cmd.ExtraFiles)-1] != image {
				t.Fatal("execution not bound to owned image descriptor")
			}
			if extra && cmd.ExtraFiles[0] != inherited {
				t.Fatal("fixture FD index changed")
			}
			if err = image.Close(); err != nil {
				t.Fatal(err)
			}
			if _, err = image.Stat(); err == nil {
				t.Fatal("parent image descriptor leaked")
			}
		})
	}
}
