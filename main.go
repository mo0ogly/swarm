package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
)

const help = `swarm — compagnon local de reprise (schema_version: 1)

Options globales : --root <projet> --json
swarm init
swarm aide [sujet]
swarm providers init|show
swarm console [travail] [--plain]
swarm prepare list|methods|show|history|create|save|method|adopt-brief|validate-plan|export
swarm dispatch <travail>
swarm autonomy <travail> [manuel|assiste|autonome] [créneaux]
swarm mission status|start|pause|resume|stop|watch <travail> [--input profil.json]
swarm profile <travail> [tâche] [--input profil.json]
swarm control <travail> --input commande.json
swarm agent start <travail> --input lancement.json
swarm agent list <travail>
swarm agent show|stop|reconcile <agent>
swarm agent logs <agent> [après_seq] [--output nouveau.jsonl]
swarm work create --input fichier.json
swarm work update <travail> --input fichier.json
swarm work list
swarm work show <travail>
swarm task add|update <travail> --input fichier.json
swarm checkpoint <travail> --input fichier.json
swarm ooda <travail> --input fichier.json
swarm gate <travail> --task <tâche> --input requête.json
swarm resume [travail]
swarm export <travail> --output archive.zip
swarm import --input archive.zip
swarm evaluate --input evidence.json [--phase delivery]

Les mutations exigent schema_version, event_id et expected_revision.
Une gate attend aussi task_id, phase, document (méthode d’évaluation 2).
Les requêtes sont décrites dans README.md. --input - lit stdin.
Codes généraux : 0 succès ; 1 gate bloquée ; 2 erreur.
Préparer : 2 contrat ; 3 conflit ; 4 autorisation ; 5 fournisseur ou méthode indisponible ; 6 arrêt non confirmé.
`

func readInput(path string) ([]byte, error) {
	var r io.Reader = os.Stdin
	if path != "-" {
		f, e := os.Open(path)
		if e != nil {
			return nil, e
		}
		defer f.Close()
		r = f
	}
	b, e := io.ReadAll(io.LimitReader(r, (16<<20)+1))
	if len(b) > 16<<20 {
		return nil, fmt.Errorf("entrée supérieure à 16 Mio")
	}
	return b, e
}
func strict(b []byte, v any) error {
	d := json.NewDecoder(bytes.NewReader(b))
	d.DisallowUnknownFields()
	if e := d.Decode(v); e != nil {
		return e
	}
	var extra any
	if e := d.Decode(&extra); e != io.EOF {
		return fmt.Errorf("un seul document JSON requis")
	}
	return nil
}
func printJSON(w io.Writer, v any) error {
	e := json.NewEncoder(w)
	e.SetIndent("", "  ")
	e.SetEscapeHTML(false)
	return e.Encode(v)
}
func run(args []string, out, errOut io.Writer) int {
	root := "."
	input := ""
	output := ""
	task := ""
	phase := "delivery"
	asJSON := false
	pos := []string{}
	for i := 0; i < len(args); i++ {
		a := args[i]
		switch a {
		case "--json":
			asJSON = true
		case "--help", "-h":
			fmt.Fprint(out, help)
			return 0
		case "--root", "--input", "--output", "--task", "--phase":
			if i+1 == len(args) {
				fmt.Fprintln(errOut, "valeur manquante :", a)
				return 2
			}
			i++
			switch a {
			case "--root":
				root = args[i]
			case "--input":
				input = args[i]
			case "--output":
				output = args[i]
			case "--task":
				task = args[i]
			case "--phase":
				phase = args[i]
			}
		default:
			pos = append(pos, a)
		}
	}
	fail := func(e error) int {
		if asJSON {
			_ = printJSON(errOut, map[string]any{"schema_version": 1, "error": e.Error(), "failure": commandFailure(e)})
		} else {
			fmt.Fprintln(errOut, "Erreur :", e)
		}
		switch commandFailure(e).Code {
		case "conflict":
			return 3
		case "source_refused", "preparation_disabled":
			return 4
		case "provider_unavailable", "method_unavailable":
			return 5
		case "interrupted":
			return 6
		default:
			return 2
		}
	}
	if len(pos) == 0 {
		fmt.Fprint(out, help)
		return 0
	}
	if pos[0] == "aide" || pos[0] == "help" {
		topic := ""
		if len(pos) > 2 {
			return fail(fmt.Errorf("swarm aide [sujet]"))
		}
		if len(pos) == 2 {
			topic = pos[1]
		}
		text, err := cliTopicHelp(topic)
		if err != nil {
			return fail(err)
		}
		if asJSON {
			_ = printJSON(out, map[string]string{"topic": topic, "help": text})
		} else {
			fmt.Fprint(out, text)
		}
		return 0
	}
	root, e := filepath.Abs(root)
	if e != nil {
		return fail(e)
	}
	root, e = filepath.EvalSymlinks(root)
	if e != nil {
		return fail(e)
	}
	if pos[0] == "evaluate" {
		raw, e := readInput(input)
		if e != nil {
			return fail(e)
		}
		ev, e := evaluate(raw, root, phase)
		if e != nil {
			return fail(e)
		}
		_ = printJSON(out, ev)
		if !ev.Allowed {
			return 1
		}
		return 0
	}
	s, e := openStore(root, pos[0] == "init")
	if e != nil {
		return fail(e)
	}
	defer s.db.Close()
	if pos[0] == "prepare" {
		if e := s.preparationEntry(pos[1:], input, output, asJSON, out); e != nil {
			return fail(e)
		}
		return 0
	}
	if pos[0] == "mission" {
		returnErr := missionCLI(s, pos, input, asJSON, out)
		if returnErr != nil {
			return fail(returnErr)
		}
		return 0
	}
	if pos[0] == "console" || pos[0] == "agent" || pos[0] == "providers" || (pos[0] == "_supervise" || pos[0] == "_assist" || pos[0] == "_prepare_turn" || pos[0] == "_dialogue_agent") || pos[0] == "control" || pos[0] == "web" || pos[0] == "dispatch" || pos[0] == "autonomy" || pos[0] == "profile" {
		if e := agentCLI(s, pos, input, output, asJSON, out); e != nil {
			return fail(e)
		}
		return 0
	}
	if pos[0] == "init" {
		_ = printJSON(out, map[string]any{"schema_version": 1, "root": s.root, "state": ".swarm/state.db"})
		return 0
	}
	kind := pos[0]
	id := ""
	if kind == "work" || kind == "task" {
		if len(pos) < 2 {
			return fail(fmt.Errorf("sous-commande manquante"))
		}
		kind += "." + pos[1]
		if len(pos) > 2 {
			id = pos[2]
		}
	} else if len(pos) > 1 {
		id = pos[1]
	}
	if kind == "work.list" || kind == "resume" && id == "" {
		ws, e := s.list()
		if e != nil {
			return fail(e)
		}
		if asJSON {
			_ = printJSON(out, ws)
		} else {
			fmt.Fprint(out, s.listView(ws))
		}
		return 0
	}
	emit := func(w Work) int {
		v, e := s.view(w)
		if kind != "work.show" {
			v, e = s.render(w)
		}
		if e != nil {
			return fail(fmt.Errorf("état conservé mais fiche non générée : %w", e))
		}
		if asJSON {
			ev, _ := s.events(w.ID)
			current := map[string]any{}
			for _, t := range w.Tasks {
				if t.Gate != nil {
					r, e := evaluate(t.Gate.Document, s.root, t.Gate.Evaluation.Phase)
					if e != nil {
						current[t.ID] = map[string]any{"valid": false, "error": e.Error()}
					} else {
						current[t.ID] = map[string]any{"valid": true, "evaluation": r}
					}
				}
			}
			_ = printJSON(out, map[string]any{"work": w, "events": ev, "current_gates": current, "current_git": gitState(s.root), "resume_markdown": v})
		} else {
			fmt.Fprintln(out, v)
		}
		return 0
	}
	if kind == "work.show" || kind == "resume" {
		w, e := s.get(id)
		if e != nil {
			return fail(e)
		}
		return emit(w)
	}
	if kind == "export" {
		if output == "" {
			return fail(fmt.Errorf("--output requis"))
		}
		if e = s.export(id, output); e != nil {
			return fail(e)
		}
		_ = printJSON(out, map[string]string{"archive": output})
		return 0
	}
	if kind == "import" {
		w, e := s.importBundle(input)
		if e != nil {
			return fail(e)
		}
		return emit(w)
	}
	if kind == "gate" {
		var r struct {
			Schema   int             `json:"schema_version"`
			Event    string          `json:"event_id"`
			Revision int             `json:"expected_revision"`
			TaskID   string          `json:"task_id"`
			Phase    string          `json:"phase"`
			Name     string          `json:"name"`
			Document json.RawMessage `json:"document"`
		}
		b, e := readInput(input)
		if e != nil {
			return fail(e)
		}
		if e = strict(b, &r); e != nil {
			return fail(e)
		}
		if r.Schema != 1 || r.TaskID == "" || task != "" && task != r.TaskID {
			return fail(fmt.Errorf("schema_version/task_id invalide"))
		}
		normalized, _ := json.Marshal(r)
		w, e := s.mutate(id, kind, r.Event, r.Revision, normalized, func(w *Work) error {
			return s.applyGateDocument(w, r.TaskID, r.Phase, r.Name, r.Document)
		})
		if e != nil {
			return fail(e)
		}
		code := emit(w)
		if code == 0 {
			t, _ := w.task(r.TaskID)
			if t.Gate != nil && !t.Gate.Evaluation.Allowed {
				return 1
			}
		}
		return code
	}
	switch kind {
	case "work.create", "work.update", "task.add", "task.update", "checkpoint", "ooda":
	default:
		return fail(fmt.Errorf("commande inconnue : %s", kind))
	}
	b, e := readInput(input)
	if e != nil {
		return fail(e)
	}
	var r Request
	if e = strict(b, &r); e != nil {
		return fail(e)
	}
	if r.Schema != 1 {
		return fail(fmt.Errorf("schema_version doit valoir 1"))
	}
	// Une entree fournie par un appelant reste un geste humain : le moteur
	// ecrit ses mutations en interne, jamais par cette porte.
	r.Origin = ""
	w, e := s.executeRequest(id, kind, r)
	if e != nil {
		return fail(e)
	}
	return emit(w)
}
func main() { os.Exit(run(os.Args[1:], os.Stdout, os.Stderr)) }
