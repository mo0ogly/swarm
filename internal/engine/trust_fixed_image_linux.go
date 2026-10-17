//go:build linux

package engine

import (
	"fmt"
	"os"
	"os/exec"
)

func pinTrustFixedHelperImage(cmd *exec.Cmd) (*os.File, error) {
	image, err := os.Open("/proc/self/exe")
	if err != nil {
		return nil, err
	}
	fd := 3 + len(cmd.ExtraFiles)
	cmd.ExtraFiles = append(cmd.ExtraFiles, image)
	cmd.Path = fmt.Sprintf("/proc/self/fd/%d", fd)
	return image, nil
}
