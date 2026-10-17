//go:build linux

package engine

import (
	"context"
	"fmt"
	"io"
	"os"
	"time"
)

func (s *Store) trustActionsCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 2 {
		return fmt.Errorf("swarm trust kernel-observe|kernel-check|confinement-check|identity-context|show|workload-login|workload-logout|oidc-start|oidc-finish|oidc-logout|oidc-session|oidc-backchannel-logout|config-preview|config-apply|mandate-create|mandate-revoke|request|approve|commit|reconcile|direct-write|elevate-integrity|export|anchor|verify")
	}
	action := pos[1]
	if action == "kernel-observe" {
		if input == "" {
			return fmt.Errorf("observation noyau : --input requis")
		}
		raw, err := readInput(input)
		if err != nil {
			return err
		}
		requestStore, err := s.trustServiceScope(context.Background(), os.Getenv("SWARM_TRUST_TOKEN"))
		if err != nil {
			return err
		}
		value, err := requestStore.trustKernelObserve(context.Background(), raw, os.Getenv("SWARM_TRUST_TOKEN"))
		if err != nil {
			return err
		}
		return printKernelObserve(out, value)
	}
	if action == "kernel-check" {
		raw := []byte("{}")
		var err error
		if input != "" {
			raw, err = readInput(input)
			if err != nil {
				return err
			}
		}
		requestStore, err := s.trustServiceScope(context.Background(), os.Getenv("SWARM_TRUST_TOKEN"))
		if err != nil {
			return err
		}
		v, err := requestStore.trustKernelCheck(context.Background(), raw, os.Getenv("SWARM_TRUST_TOKEN"))
		if err != nil {
			return err
		}
		return printTrustKernelCheck(out, v)
	}
	if action == "workload-introspection-key" {
		return fmt.Errorf("workload-introspection-key : --server requis ; clé uniquement en mémoire du serveur persistant")
	}

	switch action {
	case "show":
		v, err := s.trustView()
		if err != nil {
			return err
		}
		return printJSON(out, v)
	case "export":
		v, err := s.trustExport()
		if err != nil {
			return err
		}
		return printJSON(out, v)
	case "anchor":
		v, err := s.trustAnchor()
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if action == "confinement-check" {
		raw := []byte("{}")
		if input != "" {
			var err error
			raw, err = readInput(input)
			if err != nil {
				return err
			}
		}
		v, err := s.trustSealedCheck(context.Background(), raw)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if action == "identity-context" {
		raw := []byte("{}")
		if input != "" {
			var err error
			raw, err = readInput(input)
			if err != nil {
				return err
			}
		}
		v, err := s.trustIdentityContext(raw, os.Getenv("SWARM_TRUST_TOKEN"), time.Time{})
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if input == "" {
		return fmt.Errorf("--input requis")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	if action == "oidc-backchannel-logout" {
		v, err := s.trustOIDCLogoutOperation(context.Background(), raw)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if action == "workload-login" || action == "workload-logout" {
		v, err := s.trustWorkloadOperation(context.Background(), action, raw)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if action == "oidc-start" || action == "oidc-finish" || action == "oidc-logout" || action == "oidc-session" {
		v, err := s.trustOIDCOperation(context.Background(), action, raw, os.Getenv("SWARM_TRUST_TOKEN"))
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if action == "config-preview" || action == "config-apply" {
		var change TrustConfigChange
		if err = strict(raw, &change); err != nil {
			return err
		}
		v, err := s.trustConfig(change, action == "config-preview")
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	var cmd TrustCommand
	if err = strict(raw, &cmd); err != nil {
		return err
	}
	if action == "verify" {
		v, err := verifyTrustCommand(cmd)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	cmd.original = append([]byte{}, raw...)
	v, err := s.trustOperate(action, cmd, os.Getenv("SWARM_TRUST_TOKEN"), time.Time{})
	if err != nil {
		return err
	}
	return printJSON(out, v)
}
