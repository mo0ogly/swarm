//go:build linux

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"syscall"
)

func findRetex(w *Work, id string) (*Retex, error) {
	if !safeName(id) {
		return nil, fmt.Errorf("Identifiant RETEX invalide")
	}
	for i := range w.Retex {
		if w.Retex[i].ID == id {
			return &w.Retex[i], nil
		}
	}
	return nil, fmt.Errorf("RETEX inconnu")
}
func retexMarkdown(w Work, r Retex) string {
	return fmt.Sprintf("# RETEX — %s\n\nTravail : %s\nRETEX : %s\nDate : %s\nÉtat : %s\nNature : %s\nSource : tâche %s ; tentative %s ; SHA-256 %s\nTâche APEX : %s\n\n## Constat\n%s\n\n## Proposition\n%s\n\n## Effort\n%s\n\n## Risque\n%s\n\n## Critère de réussite\n%s\n\n## Preuves\n%s\n", w.Title, w.ID, r.ID, r.At, r.Status, r.Nature, r.Source.Task, r.Source.Attempt, r.Source.SHA256, r.Task, r.Problem, r.Proposal, r.Effort, r.Risk, r.Criteria, r.Proof)
}

// Reject symlinks component by component before creating generated directories.
func generatedDir(root, rel string) (string, error) {
	p := root
	for _, part := range strings.Split(rel, "/") {
		if !safeName(part) && part != ".claude" {
			return "", fmt.Errorf("répertoire non autorisé")
		}
		p = filepath.Join(p, part)
		st, e := os.Lstat(p)
		if os.IsNotExist(e) {
			if e = os.Mkdir(p, 0700); e != nil {
				return "", e
			}
		} else if e != nil {
			return "", e
		} else if st.Mode()&os.ModeSymlink != 0 || !st.IsDir() {
			return "", fmt.Errorf("répertoire généré incompatible : %s", part)
		}
	}
	return p, nil
}
func (s *Store) verifiedRetex(w Work, r Retex) error {
	task, err := w.task(r.Task)
	if err != nil || task.Status != "accepted" || r.Nature != "fait" || !nonempty(r.Proof) || !s.acceptedFresh(&w, task, map[string]bool{}) {
		return fmt.Errorf("Un fait sourcé et une tâche APEX acceptée avec des preuves fraîches sont requis.")
	}
	return nil
}
func (s *Store) retexAction(q webRequest) (any, error) {
	w, e := s.get(q.Work)
	if e != nil {
		return nil, e
	}
	if q.Kind == "retex-guide-preview" || q.Kind == "retex-guide" {
		r, e := findRetex(&w, q.Task)
		if e != nil {
			return nil, e
		}
		if r.Status != "verifie" {
			return nil, fmt.Errorf("Vérifier le RETEX avant promotion.")
		}
		if e = s.verifiedRetex(w, *r); e != nil {
			return nil, e
		}
		return s.retexGuide(w, *r, q)
	}
	raw, _ := json.Marshal(q)
	return s.mutate(q.Work, q.Kind, q.Event, q.Revision, raw, func(w *Work) error {
		if q.Kind == "retex-save" {
			r := q.Retex
			if !safeName(r.ID) {
				return fmt.Errorf("Identifiant RETEX invalide")
			}
			if _, e := findRetex(w, r.ID); e == nil {
				return fmt.Errorf("RETEX déjà présent")
			}
			if _, e := resolveDialogue(*w, r.Source); e != nil {
				return e
			}
			count := 0
			for _, prior := range w.Retex {
				if prior.Source == r.Source {
					count++
				}
			}
			if count >= 3 {
				return fmt.Errorf("Trois propositions maximum par réponse source.")
			}
			for _, v := range []string{r.Problem, r.Proposal, r.Effort, r.Risk, r.Criteria} {
				if !nonempty(v) || len(v) > 8000 {
					return fmt.Errorf("Renseigner constat, proposition, effort, risque et critère (8000 octets maximum chacun).")
				}
			}
			if len(r.Proof) > 8000 || r.Nature != "fait" && r.Nature != "hypothese" {
				return fmt.Errorf("Nature ou preuves invalides")
			}
			r.ExportPending = true
			r.Status = "propose"
			r.Task = ""
			r.At = now()
			r.ExportHash = ""
			r.ExportError = ""
			r.Lesson = ""
			w.Retex = append(w.Retex, r)
			return nil
		}
		r, e := findRetex(w, q.Task)
		if e != nil {
			return e
		}
		r.ExportPending = true
		switch q.Kind {
		case "retex-qualify":
			if r.Lesson != "" || r.Status == "verifie" {
				return fmt.Errorf("RETEX déjà vérifié : conserver sa qualification.")
			}
			if q.Retex.Nature != "fait" && q.Retex.Nature != "hypothese" || len(q.Retex.Proof) > 8000 {
				return fmt.Errorf("Qualification invalide")
			}
			r.Nature = q.Retex.Nature
			r.Proof = q.Retex.Proof
		case "retex-task":
			if r.Task != "" {
				return nil
			}
			if r.Status == "ecarte" || r.Status == "remplace" {
				return fmt.Errorf("RETEX écarté ou remplacé")
			}
			id := "apex-" + r.ID
			if e = s.apply(w, "task.add", Request{ID: id, Title: r.Problem, Deliverable: "docs/" + id + "-handoff.md", Criteria: []string{r.Criteria, "Preuves fraîches et parcours utilisateur vérifié"}, Next: r.Proposal + "\nSource RETEX : " + r.ID + "\nRisque : " + r.Risk + "\nArrêt : deux corrections maximum sur une même hypothèse ; sinon retour au plan."}); e != nil {
				return e
			}
			r.Task = id
			r.Status = "retenu"
		case "retex-status":
			switch q.Note {
			case "retenu", "en_cours", "ecarte", "remplace":
			case "verifie":
				if e = s.verifiedRetex(*w, *r); e != nil {
					return e
				}
			default:
				return fmt.Errorf("État RETEX invalide")
			}
			if r.Lesson != "" {
				return fmt.Errorf("Une leçon publiée doit être révisée dans le Guide avec sa preuve ; ne pas changer silencieusement son RETEX.")
			}
			r.Status = q.Note
		case "retex-export":
			dir, e := generatedDir(s.root, "docs/retex/swarm")
			if e != nil {
				r.ExportError = e.Error()
				return nil
			}
			path := filepath.Join(dir, r.ID+".md")
			body := []byte(retexMarkdown(*w, *r))
			if st, e := os.Lstat(path); e == nil && (!st.Mode().IsRegular() || st.Mode()&os.ModeSymlink != 0) {
				r.ExportError = "Fichier cible incompatible"
				return nil
			}
			old, readErr := os.ReadFile(path)
			if readErr != nil && !os.IsNotExist(readErr) {
				r.ExportError = "Fichier existant illisible : export refusé."
				return nil
			}
			if readErr == nil && hash(old) != r.ExportHash && string(old) != string(body) {
				r.ExportError = "Fichier modifié hors Swarm : conserver vos modifications avant de réexporter."
				return nil
			}

			if e = atomicWrite(path, body); e != nil {
				r.ExportError = e.Error()
				return nil
			}
			r.ExportPending = false
			r.ExportHash = hash(body)
			r.ExportError = ""
		default:
			return fmt.Errorf("Action RETEX inconnue")
		}
		return nil
	})
}
func (s *Store) retexGuide(w Work, r Retex, q webRequest) (any, error) {
	if w.Revision != q.Revision {
		return nil, fmt.Errorf("Révision modifiée : réexaminer la leçon")
	}
	if !nonempty(q.Note) || len(q.Note) > 6000 || strings.Count(q.Note, "\n") > 150 {
		return nil, fmt.Errorf("Leçon requise, limitée à 6000 octets et 150 lignes")
	}
	dir, e := generatedDir(s.root, ".claude/field-guide")
	if e != nil {
		return nil, e
	}
	// Process-wide coordination for cockpit publishers; externally edited files are checked by hash.
	lock, e := os.OpenFile(filepath.Join(s.root, ".swarm", "guide.lock"), os.O_CREATE|os.O_RDWR|syscall.O_NOFOLLOW, 0600)
	if e != nil {
		return nil, e
	}
	defer lock.Close()
	if e = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX); e != nil {
		return nil, e
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	index := filepath.Join(dir, "index.md")
	before, e := os.ReadFile(index)
	if os.IsNotExist(e) {
		before = []byte("# Guide de terrain\n")
	} else if e != nil {
		return nil, e
	}
	if st, e := os.Lstat(index); e == nil && st.Mode()&os.ModeSymlink != 0 {
		return nil, fmt.Errorf("Index symbolique refusé")
	}
	name := "retex-" + r.ID + ".md"
	target := filepath.Join(dir, name)
	note := fmt.Sprintf("# Leçon vérifiée — %s\n\n%s\n\nSource RETEX : %s ; travail %s ; tâche %s.\nPreuves : %s\nLimite : revalider après changement du périmètre ou des preuves.\n", r.Problem, q.Note, r.ID, w.ID, r.Task, r.Proof)
	old, e := os.ReadFile(target)
	if e != nil && !os.IsNotExist(e) {
		return nil, e
	}
	if st, e := os.Lstat(target); e == nil && st.Mode()&os.ModeSymlink != 0 {
		return nil, fmt.Errorf("Note symbolique refusée")
	}
	if len(old) > 0 && string(old) != note {
		return nil, fmt.Errorf("Une note différente existe déjà : ne pas l’écraser.")
	}
	after := string(before)
	if !strings.Contains(after, "("+name+")") {
		after += "\n- [RETEX " + r.ID + "](" + name + ") : leçon vérifiée, sources et limites.\n"
	}
	treeHash, e := validateGuide(s.root, dir, map[string]string{"index.md": after, name: note})
	if e != nil {
		return nil, e
	}
	digest := hash([]byte(string(before) + "\x00" + string(old) + "\x00" + note + "\x00" + treeHash))
	result := map[string]any{"digest": digest, "preview": "INDEX AVANT\n" + string(before) + "\nINDEX APRÈS\n" + after + "\nNOTE À AJOUTER\n" + note, "path": ".claude/field-guide/" + name}
	if q.Kind == "retex-guide-preview" {
		return result, nil
	}
	if q.ContextHash != digest {
		return nil, fmt.Errorf("Guide modifié : examiner un nouvel aperçu.")
	}
	raw, _ := json.Marshal(q)
	_, e = s.mutate(w.ID, q.Kind, q.Event, q.Revision, raw, func(current *Work) error {
		r, e := findRetex(current, r.ID)
		if e != nil {
			return e
		}
		if e = s.verifiedRetex(*current, *r); e != nil {
			return e
		}
		if e = atomicWrite(target, []byte(note)); e != nil {
			return e
		}
		if e = atomicWrite(index, []byte(after)); e != nil {
			return e
		}
		r.Lesson = ".claude/field-guide/" + name
		return nil
	})
	return result, e
}
