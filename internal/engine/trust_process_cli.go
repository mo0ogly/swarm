//go:build linux

package engine

import (
	"fmt"
	"io"
	"os"
)

func (s *Store) trustProcessCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("swarm process show|policy-preview|policy-apply|definition-preview|definition-publish|case-create|case-show|case-transition")
	}
	raw := []byte("{}")
	if input != "" {
		var err error
		raw, err = readInput(input)
		if err != nil {
			return err
		}
	} else if pos[1] != "show" {
		return fmt.Errorf("--input requis")
	}
	value, err := s.trustProcessOperation(pos[1], raw, os.Getenv("SWARM_TRUST_TOKEN"))
	if err != nil {
		return err
	}
	return printJSON(out, value)
}
