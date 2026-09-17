package main

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"
)

func missionCLI(s *Store, args []string, input string, asJSON bool, out io.Writer) error {
	if len(args) < 3 {
		return fmt.Errorf("usage : mission status|start|pause|resume|stop|watch WORK [--input profil.json]")
	}
	action, work := args[1], args[2]
	switch action {
	case "start":
		var p LaunchProfile
		if input != "" {
			b, e := readInput(input)
			if e != nil {
				return e
			}
			if e = strict(b, &p); e != nil {
				return e
			}
		} else {
			w, e := s.get(work)
			if e != nil {
				return e
			}
			if w.Profile == nil {
				return fmt.Errorf("Profil requis : --input avec provider, workspace et role")
			}
			p = *w.Profile
		}
		w, e := s.get(work)
		if e != nil {
			return e
		}
		if e = s.configureMission(work, p, s.slots(work), w.Revision); e != nil {
			return e
		}
	case "pause":
		if e := s.pause(work, true); e != nil {
			return e
		}
	case "stop":
		if e := s.stopMission(work); e != nil {
			return e
		}
	case "resume":
		p, e := s.missionPolicy(work)
		if e != nil {
			return e
		}
		if !p.Enabled {
			return fmt.Errorf("Autorisation absente : mission start WORK")
		}
		if e = s.setAutonomy(work, autonomyAuto, s.slots(work)); e != nil {
			return e
		}
		if e = s.pause(work, false); e != nil {
			return e
		}
	case "watch":
		if _, e := s.get(work); e != nil {
			return e
		}
		fmt.Fprintln(out, "Conducteur actif. Ctrl-C arrête cette veille ; les agents déjà lancés restent supervisés.")
		ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
		defer cancel()
		s.missionLoop(ctx, work)
		return nil
	case "status":
	default:
		return fmt.Errorf("action mission inconnue : %s", action)
	}
	d, e := s.missionStatus(work)
	if e != nil {
		return e
	}
	if asJSON {
		return printJSON(out, d)
	}
	fmt.Fprintln(out, d.Summary)
	fmt.Fprintln(out, d.Next)
	for _, t := range d.Tasks {
		fmt.Fprintf(out, "- %s : %s\n  → %s · tâche %s · %d dépendants\n", t.Title, t.Reason, t.Label, t.Target, t.Impact)
	}
	if d.Enabled {
		fmt.Fprintf(out, "La mission continue nécessite le serveur web actif ou : swarm mission watch %s\n", work)
	}
	return nil
}
