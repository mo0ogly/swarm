package main

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"
	"time"
)

func missionCLI(s *Store, args []string, input string, asJSON bool, out io.Writer) error {
	if len(args) < 3 {
		return fmt.Errorf("usage : mission status|preview|start|pause|resume|stop|watch WORK [--input profil.json]")
	}
	action, work := args[1], args[2]
	switch action {
	case "preview", "start":
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
		if action == "preview" {
			preview, err := s.missionLaunchPreview(work, p, s.slots(work))
			if err != nil {
				return err
			}
			if asJSON {
				return printJSON(out, preview)
			}
			fmt.Fprintf(out, "%d départ(s) possible(s) maintenant · concurrence réelle %d/%d\n", preview.Immediate, preview.EffectiveConcurrency, preview.RequestedSlots)
			fmt.Fprintln(out, preview.ConcurrencyDetail)
			fmt.Fprintln(out, "Autorisation examinée une fois :")
			fmt.Fprintf(out, "- Portée : %s\n", preview.Contract.Scope)
			fmt.Fprintf(out, "- Budget : %s\n", preview.Contract.Budget)
			fmt.Fprintf(out, "- Reprises : %s\n", preview.Contract.Recovery)
			fmt.Fprintf(out, "- Validations : %s\n", preview.Contract.Validation)
			for _, item := range preview.Departures {
				fmt.Fprintf(out, "- Départ : %s\n", item.Title)
			}
			for _, item := range preview.Waiting {
				fmt.Fprintf(out, "- Attente : %s — %s\n", item.Title, item.Reason)
			}
			for _, limit := range preview.Limits {
				fmt.Fprintf(out, "- Limite : %s\n", limit)
			}
			return nil
		}
		if _, e = s.missionLaunchPreview(work, p, s.slots(work)); e != nil {
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
		fmt.Fprintln(out, "Démarrage du conducteur : la première vérification va réconcilier les tentatives existantes avant tout nouveau départ.")
		fmt.Fprintln(out, "Gardez cette commande ouverte. Ctrl-C arrête les prochains départs automatiques ; les agents déjà lancés conservent leur propre supervision.")
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
	fmt.Fprintln(out, "Mission")
	printMissionUnderstanding(out, d.Understanding, "  ")
	fmt.Fprintln(out, d.Organization.Label)
	for _, issue := range d.Organization.Issues {
		fmt.Fprintln(out, "- "+issue)
	}
	fmt.Fprintln(out, d.Organization.Verification)
	if current, e := s.get(work); e == nil {
		for _, t := range current.Tasks {
			if r := t.IndependentReview; r != nil {
				fmt.Fprintf(out, "Vérification — %s : %s · %s\n", t.Title, reviewStateLabel(r.State), r.Reason)
			}
		}
	}
	fmt.Fprintln(out, d.EvidenceStage)
	permission := "non autorisée"
	if d.Authorized {
		permission = "autorisée"
	}
	fmt.Fprintf(out, "Autorisation : %s · conducteur : %s", permission, d.Supervision.State)
	if d.Supervision.Source != "" {
		fmt.Fprintf(out, " (%s)", d.Supervision.Source)
	}
	fmt.Fprintln(out)
	if d.Supervision.LastCheckAt != "" {
		fmt.Fprintf(out, "Dernière vérification : %s", missionReadableTime(d.Supervision.LastCheckAt))
		if d.Supervision.LastCheckRelative != "" {
			fmt.Fprintf(out, " (%s)", d.Supervision.LastCheckRelative)
		}
		if d.Supervision.LastError != "" {
			fmt.Fprintf(out, " · erreur : %s", d.Supervision.LastError)
		}
		if d.Supervision.NextCheckAt != "" {
			fmt.Fprintf(out, " · prochaine vérification : %s", d.Supervision.NextCheckRelative)
		}
		fmt.Fprintln(out)
	}
	if d.Supervision.ClockIssue != "" {
		fmt.Fprintf(out, "Supervision non confirmée : %s\n", d.Supervision.ClockIssue)
	}
	if action := d.Supervision.LastAction; action != nil {
		fmt.Fprintf(out, "Dernière action — %s · %s (%s) : %s\n", action.Actor, missionReadableTime(action.At), action.Relative, action.Summary)
	} else {
		fmt.Fprintln(out, "Dernière action — Conducteur Swarm : aucune action enregistrée pour cette mission.")
	}
	for _, t := range d.Tasks {
		fmt.Fprintf(out, "\nTâche — %s\n", t.Title)
		fmt.Fprintf(out, "  Résultat : %s\n", t.Result.Label)
		fmt.Fprintf(out, "  Processus : %s · rapport : %s · validation : %s\n", t.Result.ProcessLabel, t.Result.ReportLabel, t.Result.ValidationLabel)
		printMissionUnderstanding(out, t.Understanding, "  ")
		fmt.Fprintf(out, "  Action disponible : %s · tâche %s · %d dépendants\n", t.Label, t.Target, t.Impact)
		if t.Diagnostic != nil {
			printAttemptDiagnostic(out, *t.Diagnostic, "  ")
		}
	}
	fmt.Fprintln(out, "\nCoordination")
	for _, phase := range d.Coordination {
		when := ""
		if phase.At != "" {
			when = " · " + missionReadableTime(phase.At)
			if phase.Relative != "" {
				when += " (" + phase.Relative + ")"
			}
		}
		fmt.Fprintf(out, "- %s : %s%s\n  Qui agit : %s · prochaine étape : %s\n", phase.Label, phase.Summary, when, phase.Actor, phase.NextStep)
	}
	if d.Authorized && !d.Enabled && !d.Paused {
		if d.Supervision.State == "error" {
			fmt.Fprintf(out, "Autorisation conservée, conducteur en erreur : le prochain essai est annoncé ci-dessus ; gardez `swarm mission watch %s` ouvert ou vérifiez `swarm web`.\n", work)
		} else {
			fmt.Fprintf(out, "Autorisation conservée, supervision absente : gardez `swarm mission watch %s` ouvert ou démarrez `swarm web`.\n", work)
		}
	}
	return nil
}

func missionReadableTime(value string) string {
	stamp, err := time.Parse(time.RFC3339Nano, value)
	if err != nil {
		return value
	}
	return stamp.Local().Format("02/01/2006 15:04:05 MST")
}

func printAttemptDiagnostic(out io.Writer, diagnostic AttemptDiagnostic, indent string) {
	fmt.Fprintf(out, "%sDiagnostic de la tentative %s : %s\n", indent, diagnostic.AgentID, diagnostic.Summary)
	for _, item := range diagnostic.Items {
		fmt.Fprintf(out, "%s- %s\n", indent, item.Label)
		fmt.Fprintf(out, "%s  Cause : %s\n", indent, item.Cause)
		fmt.Fprintf(out, "%s  Conséquence : %s\n", indent, item.Consequence)
		fmt.Fprintf(out, "%s  Action disponible : %s\n", indent, item.Action)
		if len(item.Traces) > 0 {
			fmt.Fprintf(out, "%s  Traces techniques :\n", indent)
			for _, trace := range item.Traces {
				fmt.Fprintf(out, "%s    %s\n", indent, trace)
			}
		}
	}
}

func printMissionUnderstanding(out io.Writer, facts MissionUnderstanding, indent string) {
	fmt.Fprintf(out, "%sCe qui se passe : %s\n", indent, facts.What)
	fmt.Fprintf(out, "%sProchaine étape : %s\n", indent, facts.NextStep)
	fmt.Fprintf(out, "%sQui agit : %s\n", indent, facts.Actor)
}
