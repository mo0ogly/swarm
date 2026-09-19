package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
)

type PreparationConversion struct {
	WorkID     string     `json:"work_id"`
	PlanHash   string     `json:"plan_hash"`
	BriefHash  string     `json:"brief_hash"`
	MethodHash string     `json:"method_hash"`
	TaskIDs    []string   `json:"task_ids"`
	Spec       ActionPlan `json:"spec"`
	CreatedAt  string     `json:"created_at"`
	ReleasedAt string     `json:"released_at,omitempty"`
}
type PreparationConversionReview struct {
	Action       string                  `json:"action"`
	Changes      []PreparationPlanChange `json:"changes,omitempty"`
	Warning      string                  `json:"warning,omitempty"`
	Organization *Organization           `json:"organization,omitempty"`
	Preparation  Preparation             `json:"preparation"`
	WorkTitle    string                  `json:"work_title"`
	WorkRevision int                     `json:"work_revision"`
	Spec         ActionPlan              `json:"spec"`
}

func migratePreparationLocks(db *sql.DB, root string, backup bool) error {
	if backup {
		path := filepath.Join(root, ".swarm", newID("state-pre-v9-")+".db")
		f, e := os.OpenFile(path, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if e != nil {
			return e
		}
		if e = f.Close(); e != nil {
			return e
		}
		if _, e = db.Exec("VACUUM INTO ?", path); e != nil {
			return e
		}
	}
	_, e := db.Exec(`BEGIN IMMEDIATE;
 CREATE TABLE IF NOT EXISTS preparation_launch_locks(work_id TEXT NOT NULL REFERENCES works(id), task_id TEXT NOT NULL, preparation_id TEXT NOT NULL REFERENCES preparations(id), released INTEGER NOT NULL DEFAULT 0 CHECK(released IN (0,1)), PRIMARY KEY(work_id,task_id));
 CREATE TRIGGER IF NOT EXISTS preparation_launch_insert BEFORE INSERT ON agents WHEN EXISTS(SELECT 1 FROM preparation_launch_locks WHERE work_id=NEW.work_id AND task_id=NEW.task_id AND released=0) BEGIN SELECT RAISE(ABORT,'Plan verrouille : autoriser le demarrage depuis la preparation'); END;
 CREATE TRIGGER IF NOT EXISTS preparation_launch_update BEFORE UPDATE OF status ON agents WHEN NEW.status IN ('queued','starting','running') AND EXISTS(SELECT 1 FROM preparation_launch_locks WHERE work_id=NEW.work_id AND task_id=NEW.task_id AND released=0) BEGIN SELECT RAISE(ABORT,'Plan verrouille : autoriser le demarrage depuis la preparation'); END;
 PRAGMA user_version=9;
 COMMIT;`)
	return e
}
func preparationLaunchGuard(tx *sql.Tx, work, task string) error {
	var n int
	if e := tx.QueryRow("SELECT count(*) FROM preparation_launch_locks WHERE work_id=? AND task_id=? AND released=0", work, task).Scan(&n); e != nil {
		return e
	}
	if n > 0 {
		return fmt.Errorf("Missions créées, démarrage non autorisé. Ouvrir la préparation puis « Autoriser le démarrage des missions ».")
	}
	return nil
}
func (s *Store) preparationConversionReview(id string) (PreparationConversionReview, error) {
	p, e := s.preparation(id)
	if e != nil {
		return PreparationConversionReview{}, e
	}
	r := PreparationConversionReview{Preparation: p, WorkTitle: p.Title, Action: "create-missions"}
	if p.WorkID != "" {
		w, err := s.get(p.WorkID)
		if err != nil {
			return r, err
		}
		r.WorkTitle = w.Title
		r.WorkRevision = w.Revision
		o := organization(w)
		r.Organization = &o
	}
	if p.Conversion != nil {
		r.Spec = p.Conversion.Spec
		r.Action = "release-plan"
		if p.PlanReady && p.Documents["plan"].Hash != p.Conversion.PlanHash {
			var revised ActionPlan
			if e := strict([]byte(p.Documents["plan"].Text), &revised); e != nil {
				return r, e
			}
			r.Spec = revised
			r.Action = "revise-missions"
			r.Changes = preparationPlanChanges(p.Conversion.Spec, r.Spec)
			r.Warning = "Les résultats modifiés et leurs dépendants seront à revérifier. Les tentatives et versions restent conservées. La mission sera mise en pause, avec une nouvelle autorisation de départ nécessaire. Une tentative ou décision active bloque l’application."
		}
		return r, nil
	}
	if !p.PlanReady {
		return r, preparationError("stale_plan", "Vérifier le plan courant et résoudre ses décisions avant de créer les missions.")
	}
	e = strict([]byte(p.Documents["plan"].Text), &r.Spec)
	return r, e
}

// Called inside the preparation command transaction: work, edges, lock, receipt
// and provenance commit together. Never calls dispatch or changes autonomy.
func (s *Store) convertPreparation(tx *sql.Tx, p *Preparation, r PreparationRequest) error {
	if r.Action == "create-missions" && p.Conversion != nil {
		if r.Hash != p.Conversion.PlanHash {
			return preparationError("already_created", "Cette préparation a déjà créé ses missions. Ouvrir leur pilotage ; un autre plan exige une nouvelle préparation.")
		}
		return nil // Semantic replay with a new event also keeps the same missions.
	}
	if r.Action == "revise-missions" && p.Conversion != nil && r.Hash == p.Conversion.PlanHash {
		return nil
	}
	if r.Action == "release-plan" {
		if p.Conversion == nil || r.Hash != p.Conversion.PlanHash {
			return preparationError("stale_plan", "Relire le plan matérialisé avant d’autoriser son démarrage.")
		}
		if p.Conversion.ReleasedAt != "" {
			return nil
		}
	}
	var w Work
	create := p.WorkID == ""
	if create {
		if r.Action != "create-missions" || *r.WorkRevision != 0 {
			return preparationError("conflict", "Nouveau travail : révision cible 0 requise.")
		}
		w = Work{Schema: 1, ID: "w-" + hash([]byte(p.ID))[:24], Created: now(), Tasks: []Task{}}
	} else {
		var b []byte
		if e := tx.QueryRow("SELECT body FROM works WHERE id=?", p.WorkID).Scan(&b); e != nil {
			return e
		}
		if e := json.Unmarshal(b, &w); e != nil {
			return e
		}
		if w.Revision != *r.WorkRevision {
			return preparationError("conflict", "Le travail cible a changé. Rouvrir la revue avant de confirmer.")
		}
	}
	if r.Action == "revise-missions" {
		if e := s.revisePreparedMissions(tx, &w, p, r); e != nil {
			return e
		}
	} else if r.Action == "create-missions" {
		s.preparationFreshness(p)
		if !p.PlanReady || r.Hash != p.Documents["plan"].Hash {
			return preparationError("stale_plan", "Le verdict du plan n’est plus courant. Vérifier le plan avant de créer les missions.")
		}
		var spec ActionPlan
		if e := strict([]byte(p.Documents["plan"].Text), &spec); e != nil {
			return e
		}
		if _, e := validateActionPlan(spec, true); e != nil {
			return e
		}
		if create {
			if e := s.apply(&w, "work.create", Request{Title: p.Title, Objective: spec.Objective, Scope: p.Brief.Text, Criteria: []string{"Chaque mission satisfait ses critères et sa gate delivery avec des preuves fraîches."}}); e != nil {
				return e
			}
		}
		if e := s.materializePlan(&w, PlanReview{Source: p.ID, BriefHash: p.Brief.Hash, ResponseHash: r.Hash, Spec: spec}); e != nil {
			return e
		}
		if r.Organization != nil {
			if e := s.configurePreparedOrganization(&w, *p, *r.Organization); e != nil {
				return e
			}
		}
		approved := w.Plans[len(w.Plans)-1]
		p.Conversion = &PreparationConversion{WorkID: w.ID, PlanHash: r.Hash, BriefHash: p.Brief.Hash, MethodHash: p.MethodHash, TaskIDs: approved.TaskIDs, Spec: spec, CreatedAt: now()}
		p.WorkID = w.ID
		for _, id := range approved.TaskIDs {
			t, _ := w.task(id)
			t.LaunchHeld = true
		}
	} else {
		// Release is scoped to the immutable materialized snapshot, not a later draft.
		for _, id := range p.Conversion.TaskIDs {
			t, e := w.task(id)
			if e != nil {
				return e
			}
			result, e := tx.Exec("UPDATE preparation_launch_locks SET released=1 WHERE work_id=? AND task_id=? AND preparation_id=? AND released=0", w.ID, id, p.ID)
			if e != nil {
				return e
			}
			n, e := result.RowsAffected()
			if e != nil {
				return e
			}
			if n != 1 {
				return preparationError("conflict", "Verrou du plan incohérent ; aucune autorisation enregistrée.")
			}
			t.LaunchHeld = false
		}
		p.Conversion.ReleasedAt = now()
		if w.Planning != nil {
			w.Planning.Paused = false
		}
	}
	before := w.Revision
	w.Revision++
	w.Updated = now()
	body, e := json.Marshal(w)
	if e != nil {
		return e
	}
	if create {
		_, e = tx.Exec("INSERT INTO works(id,revision,body) VALUES(?,?,?)", w.ID, w.Revision, body)
	} else {
		var result sql.Result
		result, e = tx.Exec("UPDATE works SET revision=?,body=? WHERE id=? AND revision=?", w.Revision, body, w.ID, before)
		if e == nil {
			n, err := result.RowsAffected()
			e = err
			if n != 1 && e == nil {
				e = preparationError("conflict", "Travail modifié ; aucune mission enregistrée.")
			}
		}
	}
	if e != nil {
		return e
	}
	if r.Action == "create-missions" || r.Action == "revise-missions" {
		for _, id := range p.Conversion.TaskIDs {
			if _, e = tx.Exec("INSERT INTO preparation_launch_locks(work_id,task_id,preparation_id) VALUES(?,?,?) ON CONFLICT(work_id,task_id) DO UPDATE SET released=0", w.ID, id, p.ID); e != nil {
				return e
			}
		}
	}
	raw, _ := json.Marshal(r)
	_, e = tx.Exec("INSERT INTO events(id,work_id,revision,kind,at,payload,request) VALUES(?,?,?,?,?,?,?)", "prep-"+hash([]byte(p.ID + "/" + r.Event))[:32], w.ID, w.Revision, "preparation."+r.Action, w.Updated, raw, raw)
	return e
}
