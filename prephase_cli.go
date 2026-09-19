package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
)

func (s *Store) preparationCLI(args []string, input, output string, out io.Writer) error {
	if len(args) == 0 {
		_, e := fmt.Fprintln(out, "Préparations — documents et dialogue IA\nprepare list | methods | show ID | conversion ID | history ID DOCUMENT [avant_revision]\nprepare create --input requete.json\nprepare save|method|adopt-brief|validate-plan|answer-questions|authorize-plan|create-missions|revise-missions|release-plan ID --input requete.json\nprepare export ID --output NOUVEAU_DOSSIER\nLes commandes renvoient du JSON. prepare providers | dialogue ID | send ID --input requete.json | stop ID TURN\nprepare files ID [recherche] | source ID CHEMIN DEBUT FIN | budget ID\nprepare source-add|source-remove|budget ID --input requete.json\nprepare use-proposal ID --input requete.json\nprepare chat [ID] : dialogue interactif ; prepare resume ID reprend en terminal.\nCodes : 0 succès, 2 contrat, 3 conflit, 4 autorisation, 5 fournisseur/méthode indisponible, 6 arrêt non confirmé.")
		return e
	}
	kind := args[0]
	if kind == "files" || kind == "source" || kind == "budget" && input == "" {
		if len(args) < 2 {
			return fmt.Errorf("prepare %s ID [chemin/recherche]", kind)
		}
		if _, e := s.preparation(args[1]); e != nil {
			return e
		}
		switch kind {
		case "files":
			if len(args) > 4 {
				return fmt.Errorf("prepare files ID [recherche] [dossier]")
			}
			q := ""
			if len(args) >= 3 {
				q = args[2]
			}
			scope := ""
			if len(args) == 4 {
				scope = args[3]
			}
			v, e := s.preparationSourceFiles(scope, q)
			if e != nil {
				return e
			}
			return printJSON(out, v)
		case "source":
			if len(args) != 5 {
				return fmt.Errorf("prepare source ID CHEMIN DEBUT FIN")
			}
			a, e := strconv.Atoi(args[3])
			if e != nil {
				return e
			}
			b, e := strconv.Atoi(args[4])
			if e != nil {
				return e
			}
			v, e := s.preparationReadSource(args[2], a, b)
			if e != nil {
				return e
			}
			return printJSON(out, v)
		case "budget":
			if len(args) != 2 {
				return fmt.Errorf("prepare budget ID [--input requete.json]")
			}
			v, e := s.preparationBudget(args[1])
			if e != nil {
				return e
			}
			return printJSON(out, v)
		}
	}

	if kind == "context" {
		if len(args) != 2 {
			return fmt.Errorf("prepare context ID")
		}
		p, e := s.preparation(args[1])
		if e != nil {
			return e
		}
		m, e := s.preparationMethod(p.Method)
		if e != nil {
			return e
		}
		ts, e := s.preparationTurns(p.ID)
		if e != nil {
			return e
		}
		result := map[string]any{"history_turns": len(ts), "limit_bytes": preparationContextLimit, "message": uiText("Diagnostic sans appel IA ; le prochain message s’ajoute à ce contexte.")}
		for _, mode := range []string{"full", "recent"} {
			prompt, err := s.preparationPromptForMode(p, m, ts, "Continuer.", "brief", mode)
			entry := map[string]any{"fits": err == nil, "bytes": len(prompt)}
			if err != nil {
				entry["reason"] = err.Error()
				delete(entry, "bytes")
			}
			result[mode] = entry
		}
		return printJSON(out, result)
	}
	if kind == "conversion" {
		if len(args) != 2 {
			return fmt.Errorf("prepare conversion ID")
		}
		review, e := s.preparationConversionReview(args[1])
		if e != nil {
			return e
		}
		return printJSON(out, review)
	}
	if kind == "providers" {
		return printJSON(out, s.preparationCapabilities())
	}
	if kind == "dialogue" {
		if len(args) != 2 {
			return fmt.Errorf("prepare dialogue ID")
		}
		ts, e := s.preparationDialogue(args[1])
		if e != nil {
			return e
		}
		return printJSON(out, ts)
	}
	if kind == "stop" {
		if len(args) != 3 {
			return fmt.Errorf("prepare stop ID TURN")
		}
		t, e := s.cancelPreparation(args[1], args[2])
		if e != nil {
			return e
		}
		t.Prompt = ""
		return printJSON(out, t)
	}
	if kind == "send" {
		if len(args) != 2 || input == "" {
			return fmt.Errorf("prepare send ID --input requete.json")
		}
		b, e := readInput(input)
		if e != nil {
			return e
		}
		if len(b) > 16384 {
			return fmt.Errorf("Requête trop volumineuse")
		}
		var r PreparationSend
		if e = strict(b, &r); e != nil {
			return e
		}
		if r.ID != args[1] {
			return fmt.Errorf("Préparation différente de la cible")
		}
		t, e := s.sendPreparation(r)
		if e != nil {
			return e
		}
		if e = s.spawnPreparationTurn(t); e != nil {
			return e
		}
		t.Prompt = ""
		return printJSON(out, t)
	}
	if kind == "list" || kind == "methods" {
		if len(args) != 1 {
			return preparationError("invalid_request", uiText("Cette commande ne prend pas d’identifiant."))
		}
		if kind == "methods" {
			return printJSON(out, s.preparationMethods())
		}
		p, e := s.preparations()
		if e != nil {
			return e
		}
		return printJSON(out, map[string]any{"preparations": p, "limit": 100})
	}
	if kind == "show" || kind == "resume" || kind == "export" || kind == "history" {
		if len(args) < 2 {
			return preparationError("invalid_request", uiText("Identifiant de préparation requis."))
		}
		if kind == "history" {
			if len(args) < 3 || len(args) > 4 {
				return preparationError("invalid_request", "history ID besoin|brief|plan [avant_revision]")
			}
			before := 0
			if len(args) == 4 {
				n, e := strconv.Atoi(args[3])
				if e != nil || n < 1 {
					return preparationError("invalid_request", uiText("Révision positive requise."))
				}
				before = n
			}
			d, e := s.preparationHistory(args[1], args[2], before)
			if e != nil {
				return e
			}
			return printJSON(out, d)
		}
		if len(args) != 2 {
			return preparationError("invalid_request", uiText("Un seul identifiant attendu."))
		}
		p, e := s.preparation(args[1])
		if e != nil {
			return e
		}
		if kind == "export" {
			if e = exportPreparation(p, output); e != nil {
				return e
			}
			return printJSON(out, map[string]string{"directory": output, "preparation_id": p.ID})
		}
		return printJSON(out, p)
	}
	if input == "" {
		return preparationError("invalid_request", uiText("--input requis ; chaque mutation exige version, action, event_id et expected_revision."))
	}
	b, e := readInput(input)
	if e != nil {
		return e
	}
	if len(b) > 65536 {
		return preparationError("invalid_request", uiText("Requête limitée à 64 Kio."))
	}
	var r PreparationRequest
	if e = strict(b, &r); e != nil {
		return e
	}
	if r.Action != kind {
		return preparationError("invalid_request", uiText("L’action du document doit correspondre à la sous-commande."))
	}
	if kind == "create" {
		if len(args) != 1 {
			return preparationError("invalid_request", uiText("Création sans identifiant."))
		}
	} else {
		if len(args) != 2 || r.ID != args[1] {
			return preparationError("invalid_request", uiText("L’identifiant du document doit correspondre à la cible."))
		}
	}
	p, e := s.preparationCommand(r)
	if e != nil {
		return e
	}
	return printJSON(out, p)
}

// A new directory, never an overwrite. No transcript or credentials are exported.
func exportPreparation(p Preparation, path string) error {
	if path == "" {
		return preparationError("invalid_request", uiText("--output nouveau_dossier requis."))
	}
	parent, e := filepath.Abs(filepath.Dir(path))
	if e != nil {
		return e
	}
	real, e := filepath.EvalSymlinks(parent)
	if e != nil {
		return e
	}
	if parent != real {
		return preparationError("invalid_request", uiText("Parent symbolique refusé pour l’export."))
	}
	if e = os.Mkdir(path, 0700); e != nil {
		return e
	}
	// Retain any written partial export on error; never remove user data.
	for _, kind := range []string{"besoin", "brief", "plan"} {
		d, ok := p.Documents[kind]
		if !ok {
			continue
		}
		ext := ".md"
		if kind == "plan" {
			ext = ".json"
		}
		f, err := os.OpenFile(filepath.Join(path, kind+ext), os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return err
		}
		_, err = io.WriteString(f, d.Text)
		closeErr := f.Close()
		if err != nil {
			return err
		}
		if closeErr != nil {
			return closeErr
		}
	}
	b, e := json.MarshalIndent(p, "", "  ")
	if e != nil {
		return e
	}
	f, e := os.OpenFile(filepath.Join(path, "manifest.json"), os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
	if e != nil {
		return e
	}
	_, e = f.Write(append(b, '\n'))
	closeErr := f.Close()
	if e != nil {
		return e
	}
	return closeErr
}
