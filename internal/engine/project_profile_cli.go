package engine

import (
	"fmt"
	"io"
)

func (s *Store) projectProfileCLI(args []string, input string, asJSON bool, out io.Writer) error {
	if len(args) == 0 || (len(args) != 1 && !(len(args) == 2 && args[0] == "select")) {
		return fmt.Errorf("usage : project-profile show|check|list|select PROFILE|apply [--input profil.json]")
	}
	if args[0] == "list" {
		v, e := s.projectProfileCatalog()
		if e != nil {
			return e
		}
		if asJSON {
			return printJSON(out, v)
		}
		for _, c := range v.Choices {
			fmt.Fprintf(out, "%s · %s · %t · %s\n", c.ID, uiText(c.Title), c.Available, c.Reason)
		}
		return nil
	}
	if args[0] == "select" {
		if len(args) != 2 {
			return fmt.Errorf("usage : project-profile select PROFILE --input profil.json")
		}
		b, e := readInput(input)
		if e != nil {
			return e
		}
		var req struct {
			Expected string `json:"expected_sha256"`
		}
		if e = strict(b, &req); e != nil {
			return e
		}
		v, e := s.selectProjectProfile(args[1], req.Expected)
		if e != nil {
			return e
		}
		if asJSON {
			return printJSON(out, v)
		}
		fmt.Fprintln(out, uiText("Profil du projet :"), v.Name)
		return nil
	}
	if args[0] == "apply" {
		b, e := readInput(input)
		if e != nil {
			return e
		}
		var req struct {
			Profile  ProjectProfile `json:"profile"`
			Expected string         `json:"expected_sha256"`
		}
		if e = strict(b, &req); e != nil {
			return e
		}
		if e = s.applyProjectProfile(req.Profile, req.Expected); e != nil {
			return e
		}
	} else if args[0] != "show" && args[0] != "check" {
		return fmt.Errorf("usage : project-profile show|check|list|select PROFILE|apply [--input profil.json]")
	}
	v, e := s.projectProfileStatus()
	if e != nil {
		return e
	}
	if asJSON {
		return printJSON(out, v)
	}
	if !v.Enabled {
		fmt.Fprintln(out, uiText("Aucun profil de projet configuré."))
	} else {
		fmt.Fprintln(out, uiText("Profil du projet :"), v.Name)
		for _, role := range v.Roles {
			fmt.Fprintf(out, "%s · %s\n", role.Role, role.SHA256)
			for _, source := range role.Sources {
				fmt.Fprintf(out, "  %s · %d bytes · %s\n", source.Path, source.Bytes, source.SHA256)
			}
		}
	}
	fmt.Fprintln(out, uiText("Configuration Claude détectée, pas transmise aux planificateurs :"), v.ClaudeSettings)
	return nil
}
