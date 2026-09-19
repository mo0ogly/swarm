//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strconv"
	"time"
)

func agentCLI(s *Store, pos []string, input, output string, asJSON bool, out io.Writer) error {
	plain := false
	filtered := []string{}
	for _, part := range pos {
		if part == "--plain" {
			plain = true
		} else {
			filtered = append(filtered, part)
		}
	}
	pos = filtered
	arg := func(n int) string {
		if len(pos) > n {
			return pos[n]
		}
		return ""
	}
	switch pos[0] {
	case "workspace":
		switch arg(1) {
		case "status":
			turns, e := s.workspaceTurns()
			if e != nil {
				return e
			}
			integrations, e := s.workspaceIntegrationReceipts(arg(2))
			if e != nil {
				return e
			}
			return printJSON(out, map[string]any{"turns": turns, "integrations": integrations,
				"managed_worktrees": false, "integration_mode": "serialized_explicit"})
		case "integrate":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var request IntegrationRequest
			if e = strict(b, &request); e != nil {
				return e
			}
			receipt, created, e := s.integrateWorkspace(arg(2), request)
			if e != nil {
				return e
			}
			return printJSON(out, map[string]any{"receipt": receipt, "created": created})
		default:
			return fmt.Errorf("usage : workspace status WORK | workspace integrate WORK --input manifeste.json")
		}
	case "exchange":
		switch arg(1) {
		case "list":
			exchanges, e := s.agentExchanges(arg(2))
			if e != nil {
				return e
			}
			return printJSON(out, exchanges)
		case "send":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var request ExchangeSend
			if e = strict(b, &request); e != nil {
				return e
			}
			exchange, created, e := s.sendExchange(arg(2), request)
			if e != nil {
				return e
			}
			return printJSON(out, map[string]any{"exchange": exchange, "created": created})
		case "consume":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var request ExchangeConsume
			if e = strict(b, &request); e != nil {
				return e
			}
			exchange, consumed, e := s.consumeExchange(arg(2), request)
			if e != nil {
				return e
			}
			return printJSON(out, map[string]any{"exchange": exchange, "consumed": consumed})
		default:
			return fmt.Errorf("usage : exchange list|send|consume WORK [--input requête.json]")
		}
	case "control":
		b, e := readInput(input)
		if e != nil {
			return e
		}
		var request struct {
			Command string `json:"command"`
			Capture bool   `json:"capture_output"`
		}
		if e = strict(b, &request); e != nil {
			return e
		}
		state := &consoleState{capture: request.Capture}
		_, e = s.consoleCommand(arg(1), request.Command, state)
		if e != nil {
			return e
		}
		return printJSON(out, map[string]any{"message": state.message, "selected_agent": state.selected})
	case "profile":
		if input == "" {
			w, e := s.get(arg(1))
			if e != nil {
				return e
			}
			profils := map[string]*LaunchProfile{"travail": w.Profile}
			for _, x := range w.Tasks {
				if x.Profile != nil {
					profils[x.ID] = x.Profile
				}
			}
			return printJSON(out, profils)
		}
		b, e := readInput(input)
		if e != nil {
			return e
		}
		var profil LaunchProfile
		if e = strict(b, &profil); e != nil {
			return e
		}
		if e = s.setProfile(arg(1), arg(2), profil, -1); e != nil {
			return e
		}
		w, e := s.get(arg(1))
		if e != nil {
			return e
		}
		return printJSON(out, map[string]any{"work": arg(1), "task": arg(2), "profile": profil, "revision": w.Revision})
	case "dispatch":
		launched, e := s.dispatch(arg(1))
		if e != nil {
			return e
		}
		return printJSON(out, map[string]any{"launched": dispatchedIDs(launched),
			"autonomy": s.autonomy(arg(1)), "slots": s.slots(arg(1)), "paused": s.paused(arg(1)),
			"message": uiText("Départs automatiques effectués ; les refus sont journalisés dans le travail.")})
	case "autonomy":
		if arg(2) == "" {
			return printJSON(out, map[string]any{"autonomy": s.autonomy(arg(1)), "label": autonomyLabel(s.autonomy(arg(1))), "slots": s.slots(arg(1))})
		}
		slots := s.slots(arg(1))
		if arg(3) != "" {
			n, e := strconv.Atoi(arg(3))
			if e != nil {
				return fmt.Errorf("créneaux : nombre entier attendu")
			}
			slots = n
		}
		if e := s.setAutonomy(arg(1), arg(2), slots); e != nil {
			return e
		}
		return printJSON(out, map[string]any{"autonomy": s.autonomy(arg(1)), "label": autonomyLabel(s.autonomy(arg(1))), "slots": s.slots(arg(1))})
	case "web":
		return s.serveWeb(arg(1), out)
	case "_prepare_turn":
		if len(pos) != 2 {
			return fmt.Errorf("identifiant d’échange requis")
		}
		t, e := s.preparationTurn(pos[1])
		if e != nil {
			return e
		}
		s.runPreparationTurn(t)
		return nil
	case "_assist":
		if len(pos) != 2 {
			return fmt.Errorf("identifiant de question requis")
		}
		turn, e := s.assistTurn(pos[1])
		if e != nil {
			return e
		}
		s.runAssistTurn(turn)
		return nil
	case "_dialogue_agent":
		return s.runAgentDialogue(arg(1))
	case "_supervise":
		return s.supervise(arg(1))
	case "console":
		if plain {
			return s.plainConsole(arg(1), os.Stdin, out, asJSON)
		}
		return s.console(arg(1), os.Stdin, out, asJSON)
	case "providers":
		if arg(1) == "cooldown" {
			if len(pos) != 4 {
				return fmt.Errorf("providers cooldown show|clear <fournisseur> [--input demande.json]")
			}
			switch arg(2) {
			case "show":
				c, d, e := s.providerCooldown(arg(3))
				if e != nil {
					return e
				}
				active := false
				if c != nil {
					active = c.active(time.Now())
				}
				return printJSON(out, map[string]any{"provider": arg(3), "cooldown": c, "digest": d, "active": active})
			case "clear":
				b, e := readInput(input)
				if e != nil {
					return e
				}
				var r ProviderCooldownClear
				if e = strict(b, &r); e != nil {
					return e
				}
				c, e := s.clearProviderCooldown(arg(3), r)
				if e != nil {
					return e
				}
				return printJSON(out, map[string]any{"cooldown": c, "message": "Attente levée explicitement. Disponibilité du fournisseur non démontrée ; aucune exécution lancée et aucun budget remboursé."})
			default:
				return fmt.Errorf("providers cooldown show|clear <fournisseur>")
			}
		}
		if arg(1) == "init" {
			if e := s.initProviders(); e != nil {
				return e
			}
			fmt.Fprintln(out, uiText("Configuration créée : .swarm/providers.json (aucun fournisseur lancé)"))
			return nil
		}
		p, e := s.providers()
		if e != nil {
			return e
		}
		return printJSON(out, p)
	case "agent":
		switch arg(1) {
		case "preflight":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var r Launch
			if e = strict(b, &r); e != nil {
				return e
			}
			result, _ := s.preflightLaunch(arg(2), r)
			return printJSON(out, result)
		case "attach":
			if asJSON {
				return fmt.Errorf("agent attach est interactif ; retirer --json")
			}
			return s.attachTerminal(arg(2), os.Stdin, out)
		case "list":
			a, e := s.cockpitSnapshot(arg(2))
			if e != nil {
				return e
			}
			return printJSON(out, a)
		case "history":
			path, e := localFile(s.root, ".swarm/imports/"+arg(2)+"/manifest.json")
			if e != nil {
				return e
			}
			b, e := os.ReadFile(path)
			if e != nil {
				return e
			}
			var bundle Bundle
			if e = json.Unmarshal(b, &bundle); e != nil {
				return e
			}
			return printJSON(out, map[string]any{"historical_only": true, "history": bundle.Cockpit})
		case "show":
			a, e := s.agent(arg(2))
			if e != nil {
				return e
			}
			d, _ := s.desired(a.ID)
			return printJSON(out, map[string]any{"agent": a, "observed_status": observedAgent(a), "desired": d})
		case "start":
			b, e := readInput(input)
			if e != nil {
				return e
			}
			var r Launch
			if e = strict(b, &r); e != nil {
				return e
			}
			a, created, e := s.prepare(arg(2), r)
			if e != nil {
				return e
			}
			if created {
				if e = s.spawnAgent(a); e != nil {
					return e
				}
			}
			return printJSON(out, map[string]any{"agent": a, "created": created})
		case "stop":
			if e := s.stopAgent(arg(2)); e != nil {
				return e
			}
			fmt.Fprintln(out, uiText("Arrêt demandé ; vérifier agent show pour la confirmation."))
			return nil
		case "reconcile":
			return s.reconcile(arg(2))
		case "logs":
			after := int64(0)
			if arg(3) != "" {
				var e error
				after, e = strconv.ParseInt(arg(3), 10, 64)
				if e != nil {
					return e
				}
			}
			if _, e := s.agent(arg(2)); e != nil {
				return e
			}
			logs, e := s.logs(arg(2), after)
			if e != nil {
				return e
			}
			if output != "" {
				f, e := os.OpenFile(output, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
				if e != nil {
					return e
				}
				defer f.Close()
				for _, l := range logs {
					if e = json.NewEncoder(f).Encode(l); e != nil {
						return e
					}
				}
				return nil
			}
			return printJSON(out, logs)
		}
	}
	return fmt.Errorf("commande cockpit inconnue")
}
