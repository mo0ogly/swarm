package main

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	"os"
	"path/filepath"
)

const preparationMigration = `BEGIN IMMEDIATE;
CREATE TABLE IF NOT EXISTS preparations(id TEXT PRIMARY KEY, work_id TEXT REFERENCES works(id), revision INTEGER NOT NULL, body BLOB NOT NULL);
CREATE TABLE IF NOT EXISTS preparation_documents(preparation_id TEXT NOT NULL REFERENCES preparations(id), kind TEXT NOT NULL, revision INTEGER NOT NULL, body BLOB NOT NULL, PRIMARY KEY(preparation_id,kind,revision));
CREATE TABLE IF NOT EXISTS preparation_commands(preparation_id TEXT NOT NULL REFERENCES preparations(id), event_id TEXT NOT NULL, request BLOB NOT NULL, response BLOB NOT NULL, PRIMARY KEY(preparation_id,event_id));
PRAGMA user_version=7;
COMMIT;`

func migratePreparations(db *sql.DB, root string, backup bool) error {
	if backup {
		path := filepath.Join(root, ".swarm", newID("state-pre-v7-")+".db")
		f, e := os.OpenFile(path, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0600)
		if e != nil {
			return e
		}
		if e = f.Close(); e != nil {
			return e
		}
		if _, e = db.Exec("VACUUM INTO ?", path); e != nil {
			os.Remove(path)
			return fmt.Errorf("sauvegarde avant préparation : %w", e)
		}
	}
	_, e := db.Exec(preparationMigration)
	return e
}

func (s *Store) preparation(id string) (Preparation, error) {
	var p Preparation
	var b []byte
	e := s.db.QueryRow("SELECT body FROM preparations WHERE id=?", id).Scan(&b)
	if errors.Is(e, sql.ErrNoRows) {
		return p, preparationError("not_found", "Préparation introuvable dans ce projet.")
	}
	if e != nil {
		return p, e
	}
	e = json.Unmarshal(b, &p)
	s.preparationFreshness(&p)
	return p, e
}

func (s *Store) preparations() ([]Preparation, error) {
	rows, e := s.db.Query("SELECT body FROM preparations ORDER BY rowid DESC LIMIT 100")
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []Preparation{}
	for rows.Next() {
		var b []byte
		var p Preparation
		if e = rows.Scan(&b); e != nil {
			return nil, e
		}
		if e = json.Unmarshal(b, &p); e != nil {
			return nil, e
		}
		s.preparationFreshness(&p)
		out = append(out, p)
	}
	return out, rows.Err()
}

func (s *Store) preparationHistory(id, kind string, before int) ([]PreparationDocument, error) {
	if !preparationDocumentKind(kind) {
		return nil, preparationError("invalid_document", "Document inconnu.")
	}
	if _, e := s.preparation(id); e != nil {
		return nil, e
	}
	if before <= 0 {
		before = int(^uint(0) >> 1)
	}
	rows, e := s.db.Query("SELECT body FROM preparation_documents WHERE preparation_id=? AND kind=? AND revision<? ORDER BY revision DESC LIMIT 50", id, kind, before)
	if e != nil {
		return nil, e
	}
	defer rows.Close()
	out := []PreparationDocument{}
	for rows.Next() {
		var b []byte
		var d PreparationDocument
		if e = rows.Scan(&b); e != nil {
			return nil, e
		}
		if e = json.Unmarshal(b, &d); e != nil {
			return nil, e
		}
		out = append(out, d)
	}
	return out, rows.Err()
}

// The receipt is checked before revision or external method availability. A lost
// response can be recovered even after later edits or a method file change.
func (s *Store) preparationCommand(r PreparationRequest) (p Preparation, err error) {
	if os.Getenv("SWARM_PREPARATION_DISABLED") == "1" {
		return p, preparationError("preparation_disabled", "Préparation désactivée par configuration ; lecture et export conservés.")
	}
	defer func() {
		var coded interface{ Code() int }
		if errors.As(err, &coded) && (coded.Code()&255 == 5 || coded.Code()&255 == 6) {
			err = preparationError("conflict", "Une autre écriture est en cours. Relire la préparation puis réessayer avec le même événement si la commande est inchangée.")
		}
	}()
	if e := r.validate(); e != nil {
		return p, e
	}
	if r.Action == "create" {
		if r.ID != "" || *r.Revision != 0 {
			return p, preparationError("invalid_request", "Création : identifiant absent et révision 0 requis.")
		}
	}
	raw, _ := json.Marshal(r)
	id := r.ID
	if r.Action == "create" {
		id = "prep-" + hash([]byte(r.Event))[:24]
	}
	tx, e := s.db.Begin()
	if e != nil {
		return p, e
	}
	defer tx.Rollback()
	var savedRequest, savedResponse []byte
	e = tx.QueryRow("SELECT request,response FROM preparation_commands WHERE preparation_id=? AND event_id=?", id, r.Event).Scan(&savedRequest, &savedResponse)
	if e == nil {
		if string(savedRequest) != string(raw) {
			return p, preparationError("conflict", "Cet événement désigne déjà une autre commande.")
		}
		e = json.Unmarshal(savedResponse, &p)
		if e != nil {
			return p, e
		}
		var currentRevision int
		if e = tx.QueryRow("SELECT revision FROM preparations WHERE id=?", id).Scan(&currentRevision); e != nil {
			return p, e
		}
		p.ReceiptHistorical = currentRevision != p.Revision
		s.preparationFreshness(&p)
		return p, e
	}
	if !errors.Is(e, sql.ErrNoRows) {
		return p, e
	}
	if r.Action == "create" {
		p = Preparation{ID: id, WorkID: r.WorkID, Title: preparationTitle(r.Title), Method: r.Method, CreatedAt: now(), Documents: map[string]PreparationDocument{}}
		if p.Method == "" {
			p.Method = "apex"
		}
		// A draft remains usable without installed methods or a provider.
		if p.Method == "audit_pdca" {
			p.Method = "audit-pdca"
		}
		if p.Method != "apex" && p.Method != "ks-feature" && p.Method != "audit-pdca" {
			return p, preparationError("method_unavailable", "Méthode inconnue.")
		}
		if m, err := s.preparationMethod(p.Method); err == nil {
			p.MethodHash = m.Hash
		}
		if p.WorkID != "" {
			var exists string
			if e = tx.QueryRow("SELECT id FROM works WHERE id=?", p.WorkID).Scan(&exists); e != nil {
				return p, preparationError("not_found", "Travail cible introuvable dans ce projet.")
			}
		}
	} else {
		var b []byte
		if e = tx.QueryRow("SELECT body FROM preparations WHERE id=?", id).Scan(&b); e != nil {
			if errors.Is(e, sql.ErrNoRows) {
				return p, preparationError("not_found", "Préparation introuvable.")
			}
			return p, e
		}
		if e = json.Unmarshal(b, &p); e != nil {
			return p, e
		}
		if p.Revision != *r.Revision {
			return p, preparationError("conflict", "La préparation a changé. Relire la version courante ; votre texte n’a pas été enregistré.")
		}
	}
	edit := r
	if r.Action == "use-proposal" {
		var rawTurn []byte
		var status string
		var t PreparationTurn
		if e = tx.QueryRow("SELECT body,status FROM preparation_turns WHERE id=? AND preparation_id=?", r.Turn, id).Scan(&rawTurn, &status); e != nil {
			return p, preparationError("not_found", "Proposition introuvable.")
		}
		if e = json.Unmarshal(rawTurn, &t); e != nil {
			return p, e
		}
		m, err := s.preparationMethod(p.Method)
		if status != "answered" || t.Answer == nil || (t.Target != "plan" && t.Answer.Brief == "") || t.Revision != p.Revision || err != nil || m.Hash != t.MethodHash {
			return p, preparationError("stale_document", "Proposition ancienne ou indisponible ; demandez une mise à jour avant de l’utiliser.")
		}
		edit.Action = "save"
		edit.Document = "brief"
		edit.Text = t.Answer.Brief
		if t.Target == "plan" {
			if r.AdoptBrief {
				return p, preparationError("invalid_request", "Un plan ne peut pas être adopté comme brief.")
			}
			if !preparationBriefCurrent(p) {
				return p, preparationError("stale_brief", "Brief à réadopter avant utilisation du plan.")
			}
			proposal, err := parseActionPlan(t.Answer.Plan)
			if err != nil {
				return p, preparationError("invalid_plan", err.Error())
			}
			var prior ActionPlan
			if strict([]byte(p.Documents["plan"].Text), &prior) == nil {
				for i := range proposal.Questions {
					for _, known := range prior.Questions {
						if known.Question == proposal.Questions[i].Question && nonempty(known.Answer) {
							proposal.Questions[i].Answer = known.Answer
						}
					}
				}
			}
			saved, err := json.Marshal(proposal)
			if err != nil {
				return p, err
			}
			edit.Document = "plan"
			edit.Text = string(saved)
		}
	}
	if r.Action == "create-missions" || r.Action == "authorize-plan" || r.Action == "revise-missions" || r.Action == "release-plan" {
		if e = s.convertPreparation(tx, &p, r); e != nil {
			return p, e
		}
	}
	var document *PreparationDocument
	if e = s.editPreparation(&p, edit, &document); e != nil {
		return p, e
	}
	if r.Action == "use-proposal" && document != nil {
		document.SourceTurn = r.Turn
		p.Documents[document.Kind] = *document
		if r.AdoptBrief {
			adopted := *document
			p.Brief = &adopted
		}
	}
	p.Revision++
	p.UpdatedAt = now()
	s.preparationFreshness(&p)
	b, e := json.Marshal(p)
	if e != nil {
		return p, e
	}
	if r.Action == "create" {
		var work any
		if p.WorkID != "" {
			work = p.WorkID
		}
		_, e = tx.Exec("INSERT INTO preparations(id,work_id,revision,body) VALUES(?,?,?,?)", id, work, p.Revision, b)
	} else {
		var result sql.Result
		result, e = tx.Exec("UPDATE preparations SET revision=?,body=?,work_id=NULLIF(?,'') WHERE id=? AND revision=?", p.Revision, b, p.WorkID, id, *r.Revision)
		if e == nil {
			n, err := result.RowsAffected()
			e = err
			if n != 1 && e == nil {
				e = preparationError("conflict", "La préparation a changé ; réessayer après lecture.")
			}
		}
	}
	if e != nil {
		return p, e
	}
	if document != nil {
		d, _ := json.Marshal(document)
		if _, e = tx.Exec("INSERT INTO preparation_documents(preparation_id,kind,revision,body) VALUES(?,?,?,?)", id, document.Kind, document.Revision, d); e != nil {
			return p, e
		}
	}
	if _, e = tx.Exec("INSERT INTO preparation_commands(preparation_id,event_id,request,response) VALUES(?,?,?,?)", id, r.Event, raw, b); e != nil {
		return p, e
	}
	if e = tx.Commit(); e != nil {
		return p, e
	}
	return p, nil
}

func (s *Store) editPreparation(p *Preparation, r PreparationRequest, d **PreparationDocument) error {
	switch r.Action {
	case "source-add", "source-remove":
		return s.editPreparationSource(p, r)
	case "budget":
		if err := validateBudget(*r.Budget); err != nil {
			return err
		}
		b := *r.Budget
		b.Updated = now()
		b.Actor = operatorIdentity()
		p.Budget = &b
	case "create":
		if r.Text != "" {
			doc := PreparationDocument{Kind: "besoin", Text: r.Text, Revision: 1, Hash: hash([]byte(r.Text)), At: now()}
			p.Documents["besoin"] = doc
			*d = &doc
		}
	case "save":
		if !preparationDocumentKind(r.Document) {
			return preparationError("invalid_document", "Choisir besoin, brief ou plan.")
		}
		doc := PreparationDocument{Kind: r.Document, Text: r.Text, Revision: p.Documents[r.Document].Revision + 1, Hash: hash([]byte(r.Text)), At: now()}
		if r.Document == "brief" {
			doc.NeedHash = p.Documents["besoin"].Hash
		}
		p.Documents[r.Document] = doc
		if r.Document == "plan" {
			if p.Brief != nil {
				doc.BriefHash = p.Brief.Hash
			}
			doc.MethodHash = p.MethodHash
			p.Documents[r.Document] = doc
		}
		*d = &doc
		p.Verdict = nil
	case "answer-questions":
		doc := p.Documents["plan"]
		if r.Hash != doc.Hash {
			return preparationError("stale_document", "Le plan a changé. Relire ses questions avant de répondre.")
		}
		var plan ActionPlan
		if e := strict([]byte(doc.Text), &plan); e != nil {
			return preparationError("invalid_plan", e.Error())
		}
		if _, e := validateActionPlan(plan, false); e != nil {
			return preparationError("invalid_plan", e.Error())
		}
		if len(r.Decisions) != len(plan.Questions) {
			return preparationError("invalid_request", "Toutes les questions du plan courant doivent être conservées.")
		}
		for i, q := range r.Decisions {
			if q.Question != plan.Questions[i].Question {
				return preparationError("stale_document", "Les questions ont changé ; aucune réponse enregistrée.")
			}
			plan.Questions[i].Answer = q.Answer
		}
		raw, e := json.MarshalIndent(plan, "", "  ")
		if e != nil {
			return e
		}
		if len(raw) > preparationDocumentLimit {
			return preparationError("invalid_document", "Plan et réponses dépassent 16 000 octets. Raccourcir les réponses.")
		}
		// Preserve source hashes: answering must never freshen an obsolete plan.
		doc.Text, doc.Hash, doc.At = string(raw), hash(raw), now()
		doc.Revision++
		p.Documents["plan"] = doc
		*d = &doc
		p.Verdict = nil
	case "method":
		m, e := s.preparationMethod(r.Method)
		if e != nil {
			return e
		}
		p.Method = m.ID
		p.MethodHash = m.Hash
		p.Verdict = nil
	case "adopt-brief":
		doc, ok := p.Documents["brief"]
		if !ok || !nonempty(doc.Text) || r.Hash == "" || r.Hash != doc.Hash {
			return preparationError("stale_document", "Relire le brief courant avant de l’adopter.")
		}
		if doc.NeedHash != p.Documents["besoin"].Hash {
			return preparationError("stale_brief", "Le besoin a changé. Relire et enregistrer un brief adapté avant adoption.")
		}
		p.Brief = &doc
		p.Verdict = nil
	case "validate-plan":
		m, e := s.preparationMethod(p.Method)
		if e != nil {
			return e
		}
		if m.Hash != p.MethodHash {
			return preparationError("stale_method", "La méthode a changé. Sélectionner sa version courante avant vérification.")
		}
		if p.Brief == nil || p.Brief.Hash != p.Documents["brief"].Hash || p.Brief.NeedHash != p.Documents["besoin"].Hash {
			return preparationError("stale_brief", "Adopter le brief courant avant de vérifier le plan.")
		}
		doc := p.Documents["plan"]
		if doc.BriefHash != p.Brief.Hash || doc.MethodHash != m.Hash {
			return preparationError("stale_plan", "Ce plan provient d’un ancien brief ou d’une ancienne méthode. Relire puis enregistrer une version adaptée.")
		}
		if r.Hash == "" || r.Hash != doc.Hash {
			return preparationError("stale_document", "Relire le plan courant avant vérification.")
		}
		var plan ActionPlan
		if e = strict([]byte(doc.Text), &plan); e != nil {
			return preparationError("invalid_plan", e.Error())
		}
		if _, e = validateActionPlan(plan, true); e != nil {
			return preparationError("invalid_plan", e.Error())
		}
		p.Verdict = &PreparationVerdict{PlanHash: doc.Hash, BriefHash: p.Brief.Hash, MethodHash: m.Hash, At: now()}
	}
	return nil
}
