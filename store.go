package main

import (
	"database/sql"
	"encoding/json"
	"errors"
	"fmt"
	_ "modernc.org/sqlite"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

type Store struct {
	db          *sql.DB
	root        string
	readDigests map[string]string // request-local clone only; never retained by the live store
}

func openStore(root string, init bool) (*Store, error) {
	root, e := filepath.Abs(root)
	if e != nil {
		return nil, e
	}
	root, e = filepath.EvalSymlinks(root)
	if e != nil {
		return nil, e
	}
	dir := filepath.Join(root, ".swarm")
	if st, e := os.Lstat(dir); e == nil && (!st.IsDir() || st.Mode()&os.ModeSymlink != 0) {
		return nil, fmt.Errorf(".swarm doit être un répertoire local")
	}
	if init {
		if e = os.MkdirAll(dir, 0700); e != nil {
			return nil, e
		}
		if e = os.MkdirAll(filepath.Join(dir, "views"), 0700); e != nil {
			return nil, e
		}
		if e = atomicWrite(filepath.Join(dir, ".gitignore"), []byte("*\n")); e != nil {
			return nil, e
		}
	}

	for _, name := range []string{"views", "imports"} {
		if st, err := os.Lstat(filepath.Join(dir, name)); err == nil && (!st.IsDir() || st.Mode()&os.ModeSymlink != 0) {
			return nil, fmt.Errorf("répertoire interne non local : %s", name)
		}
	}
	path := filepath.Join(dir, "state.db")
	if st, e := os.Lstat(path); e == nil && st.Mode()&os.ModeSymlink != 0 {
		return nil, fmt.Errorf("base symbolique refusée")
	}
	if !init {
		if _, e = os.Stat(path); e != nil {
			return nil, fmt.Errorf("espace absent ; lancer swarm init")
		}
	}
	db, e := sql.Open("sqlite", path)
	if e != nil {
		return nil, e
	}
	db.SetMaxOpenConns(1)
	s := &Store{db: db, root: root}
	fail := func(e error) (*Store, error) { db.Close(); return nil, e }
	if _, e = db.Exec(`PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON; PRAGMA synchronous=FULL;`); e != nil {
		return fail(e)
	}
	var version int
	if e = db.QueryRow("PRAGMA user_version").Scan(&version); e != nil {
		return fail(e)
	}
	if version < 0 || version > schemaVersion {
		return fail(fmt.Errorf("version de stockage non supportée : %d", version))
	}
	if version == 0 {
		if !init {
			return fail(fmt.Errorf("base non initialisée"))
		}
		_, e = db.Exec(`BEGIN; CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body BLOB NOT NULL); CREATE TABLE events(id TEXT PRIMARY KEY, work_id TEXT NOT NULL REFERENCES works(id), revision INTEGER NOT NULL, kind TEXT NOT NULL, at TEXT NOT NULL, payload BLOB NOT NULL, request BLOB NOT NULL, UNIQUE(work_id,revision)); PRAGMA user_version=1; COMMIT;`)
		if e != nil {
			return fail(e)
		}
	}
	if version == 1 {
		backup := filepath.Join(dir, newID("state-pre-v2-")+".db")
		f, err := os.OpenFile(backup, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return fail(err)
		}
		if err = f.Close(); err != nil {
			return fail(err)
		}
		if _, err = db.Exec("VACUUM INTO ?", backup); err != nil {
			_ = os.Remove(backup)
			return fail(fmt.Errorf("sauvegarde avant migration : %w", err))
		}
	}
	if version < 2 {
		if _, e = db.Exec(agentMigration); e != nil {
			return fail(e)
		}
	}
	if version == 2 {
		backup := filepath.Join(dir, newID("state-pre-v3-")+".db")
		f, err := os.OpenFile(backup, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return fail(err)
		}
		f.Close()
		if _, err = db.Exec("VACUUM INTO ?", backup); err != nil {
			_ = os.Remove(backup)
			return fail(err)
		}
	}
	if version < 3 {
		if _, e = db.Exec(trackingMigration); e != nil {
			return fail(e)
		}
	}
	if version == 3 {
		backup := filepath.Join(dir, newID("state-pre-v4-")+".db")
		f, err := os.OpenFile(backup, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return fail(err)
		}
		f.Close()
		if _, err = db.Exec("VACUUM INTO ?", backup); err != nil {
			_ = os.Remove(backup)
			return fail(err)
		}
	}
	if version < 4 {
		if _, e = db.Exec(assistMigration); e != nil {
			return fail(e)
		}
	}
	if version == 4 {
		backup := filepath.Join(dir, newID("state-pre-v5-")+".db")
		f, err := os.OpenFile(backup, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
		if err != nil {
			return fail(err)
		}
		f.Close()
		if _, err = db.Exec("VACUUM INTO ?", backup); err != nil {
			_ = os.Remove(backup)
			return fail(err)
		}
	}
	if version < 5 {
		if e = migrateAutonomy(db); e != nil {
			return fail(e)
		}
	}
	if version < 6 {
		if e = migrateDecisionAuthorKind(db); e != nil {
			return fail(e)
		}
	}
	if version < 7 {
		if e = migratePreparations(db, root, version != 0); e != nil {
			return fail(e)
		}
	}
	if version < 8 {
		if version != 0 {
			backup := filepath.Join(dir, newID("state-pre-v8-")+".db")
			f, err := os.OpenFile(backup, os.O_CREATE|os.O_EXCL|os.O_WRONLY, 0600)
			if err != nil {
				return fail(err)
			}
			f.Close()
			if _, err = db.Exec("VACUUM INTO ?", backup); err != nil {
				return fail(err)
			}
		}
		if _, e = db.Exec(preparationTurnMigration); e != nil {
			return fail(e)
		}
	}
	if version < 9 {
		if e = migratePreparationLocks(db, root, version != 0); e != nil {
			return fail(e)
		}
	}
	if version < 10 {
		if e = migrateTerminals(db, root, version != 0); e != nil {
			return fail(e)
		}
	}
	if version < 11 {
		if e = migratePreparationBudget(db, root, version != 0); e != nil {
			return fail(e)
		}
	}
	if e = os.Chmod(path, 0600); e != nil {
		return fail(e)
	}
	return s, nil
}

// Les réglages d'autonomie s'ajoutent à une table existante : selon la version
// d'origine, les colonnes peuvent déjà être présentes. Vérifier avant d'ajouter,
// plutôt que de supposer un chemin de migration unique.
func migrateAutonomy(db *sql.DB) error {
	existing := map[string]bool{}
	rows, e := db.Query("PRAGMA table_info(cockpit_controls)")
	if e != nil {
		return e
	}
	for rows.Next() {
		var cid int
		var name, kind string
		var notnull, primary int
		var def any
		if e = rows.Scan(&cid, &name, &kind, &notnull, &def, &primary); e != nil {
			rows.Close()
			return e
		}
		existing[name] = true
	}
	rows.Close()
	if e = rows.Err(); e != nil {
		return e
	}
	statements := []string{}
	if !existing["autonomy"] {
		statements = append(statements, "ALTER TABLE cockpit_controls ADD COLUMN autonomy TEXT NOT NULL DEFAULT 'autonome'")
	}
	if !existing["slots"] {
		statements = append(statements, "ALTER TABLE cockpit_controls ADD COLUMN slots INTEGER NOT NULL DEFAULT 2")
	}
	for _, statement := range statements {
		if _, e = db.Exec(statement); e != nil {
			return e
		}
	}
	_, e = db.Exec("PRAGMA user_version=5")
	return e
}

// Les fermetures écrites par le moteur partageaient le type « decision » avec
// les acquittements humains, donc le fil d'activité les attribuait à
// l'opérateur. Le type est désormais distinct à l'écriture ; les lignes déjà
// enregistrées sont reclassées à partir de leur propre message, qui contient
// l'auteur — ce n'est pas une supposition, c'est relire ce qui a été écrit.
func migrateDecisionAuthorKind(db *sql.DB) error {
	if _, e := db.Exec(`UPDATE cockpit_events SET kind='decision.moteur'
		WHERE kind='decision' AND instr(message, ' : ') > 0
		AND substr(message, instr(message, ' : ') + 3, length('moteur : ')) = 'moteur : '`); e != nil {
		return e
	}
	_, e := db.Exec("PRAGMA user_version=6")
	return e
}

func (s *Store) get(id string) (Work, error) {
	var b []byte
	var w Work
	e := s.db.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&b)
	if e != nil {
		return w, fmt.Errorf("travail %s : %w", id, e)
	}
	e = json.Unmarshal(b, &w)
	return w, e
}
func (s *Store) list() ([]Work, error) {
	rs, e := s.db.Query("SELECT body FROM works ORDER BY rowid DESC")
	if e != nil {
		return nil, e
	}
	defer rs.Close()
	out := []Work{}
	for rs.Next() {
		var b []byte
		if e = rs.Scan(&b); e != nil {
			return nil, e
		}
		var w Work
		if e = json.Unmarshal(b, &w); e != nil {
			return nil, e
		}
		out = append(out, w)
	}
	return out, rs.Err()
}
func (s *Store) events(id string) ([]Event, error) {
	rs, e := s.db.Query("SELECT id,work_id,revision,kind,at,payload FROM events WHERE work_id=? ORDER BY revision", id)
	if e != nil {
		return nil, e
	}
	defer rs.Close()
	out := []Event{}
	for rs.Next() {
		var v Event
		if e = rs.Scan(&v.ID, &v.WorkID, &v.Revision, &v.Kind, &v.At, &v.Payload); e != nil {
			return nil, e
		}
		out = append(out, v)
	}
	return out, rs.Err()
}
func gitState(root string) GitState {
	run := func(args ...string) string {
		c := exec.Command("git", append([]string{"-C", root}, args...)...)
		b, e := c.Output()
		if e != nil {
			return ""
		}
		return strings.TrimSpace(string(b))
	}
	return GitState{run("rev-parse", "HEAD"), run("branch", "--show-current"), run("status", "--porcelain", "--untracked-files=normal")}
}

// The event key is an idempotency key, checked before optimistic concurrency.
func (s *Store) mutate(id, kind, event string, expected int, request []byte, fn func(*Work) error) (Work, error) {
	return s.mutateWithHook(id, kind, event, expected, request, fn, nil)
}

func (s *Store) mutateWithHook(id, kind, event string, expected int, request []byte, fn func(*Work) error, hook func(*sql.Tx, *Work) error) (Work, error) {
	var w Work
	if !safeName(event) {
		return w, fmt.Errorf("event_id obligatoire (lettres, chiffres, tirets)")
	}
	tx, e := s.db.Begin()
	if e != nil {
		return w, e
	}
	defer tx.Rollback()
	var oldID, oldKind string
	var oldRequest []byte
	e = tx.QueryRow("SELECT work_id,kind,request FROM events WHERE id=?", event).Scan(&oldID, &oldKind, &oldRequest)
	if e == nil {
		if (id != "" && oldID != id) || oldKind != kind || string(oldRequest) != string(request) {
			return w, &CommandError{Code: "event_conflict", Message: "event_id déjà utilisé pour un contenu différent"}
		}
		var b []byte
		if e = tx.QueryRow("SELECT body FROM works WHERE id=?", oldID).Scan(&b); e != nil {
			return w, e
		}
		e = json.Unmarshal(b, &w)
		return w, e
	}
	if !errors.Is(e, sql.ErrNoRows) {
		return w, e
	}
	create := kind == "work.create"
	if create {
		if expected != 0 {
			return w, fmt.Errorf("création : expected_revision doit valoir 0")
		}
		w = Work{Schema: 1, ID: newID("w-"), Created: now(), Tasks: []Task{}}
	} else {
		var b []byte
		if e = tx.QueryRow("SELECT body FROM works WHERE id=?", id).Scan(&b); e != nil {
			return w, e
		}
		if e = json.Unmarshal(b, &w); e != nil {
			return w, e
		}
		if w.Revision != expected {
			return w, &CommandError{Code: "revision_conflict", Message: fmt.Sprintf("révision périmée : attendue %d, courante %d ; relire le travail", expected, w.Revision), Retryable: true}
		}
	}
	if kind == "work.update" {
		var count int
		if e = tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", id).Scan(&count); e != nil {
			return w, e
		}
		if count > 0 {
			return w, fmt.Errorf("arrêter et réconcilier les agents avant de modifier le contrat du travail")
		}
	}
	// The ownership guard uses this transaction, after replay and revision checks.
	if kind == "task.update" || kind == "task.submit" {
		var r Request
		if e = json.Unmarshal(request, &r); e != nil {
			return w, e
		}
		if r.Status == "running" {
			if e = preparationLaunchGuard(tx, id, r.ID); e != nil {
				return w, e
			}
		}
		if r.Status != "" || r.editsDefinition() {
			var count int
			if e = tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND (task_id=? OR ?) AND status IN ('queued','starting','running','stopping')", id, r.ID, r.editsDefinition()).Scan(&count); e != nil {
				return w, e
			}
			if count > 0 {
				return w, &CommandError{Code: "active_agent", Message: "arrêter et réconcilier les agents concernés avant modification du statut ou du contrat"}
			}
		}
	}
	if e = fn(&w); e != nil {
		return w, e
	}
	w.Revision++
	w.Updated = now()
	b, e := json.Marshal(w)
	if e != nil {
		return w, e
	}
	if create {
		_, e = tx.Exec("INSERT INTO works VALUES(?,?,?)", w.ID, w.Revision, b)
	} else {
		var r sql.Result
		r, e = tx.Exec("UPDATE works SET revision=?,body=? WHERE id=? AND revision=?", w.Revision, b, w.ID, expected)
		if e == nil {
			n, _ := r.RowsAffected()
			if n != 1 {
				e = fmt.Errorf("conflit de révision")
			}
		}
	}
	if e != nil {
		return w, e
	}
	if hook != nil {
		if e = hook(tx, &w); e != nil {
			return w, e
		}
	}
	_, e = tx.Exec("INSERT INTO events VALUES(?,?,?,?,?,?,?)", event, w.ID, w.Revision, kind, w.Updated, request, request)
	if e != nil {
		return w, e
	}
	e = tx.Commit()
	return w, e
}
func (s *Store) apply(w *Work, kind string, r Request) error {
	switch kind {
	case "work.update":
		return updateWorkDefinition(w, r)
	case "work.create":
		if !nonempty(r.Title) || !nonempty(r.Objective) || !nonempty(r.Scope) || len(r.Criteria) == 0 {
			return fmt.Errorf("title, objective, scope et criteria requis")
		}
		w.Title = r.Title
		w.Objective = r.Objective
		w.Scope = r.Scope
		w.Criteria = r.Criteria
		w.Next = r.Next
		w.Git = gitState(s.root)
	case "task.add":
		if !safeName(r.ID) || !nonempty(r.Title) || !nonempty(r.Deliverable) || len(r.Criteria) == 0 {
			return fmt.Errorf("id, title, deliverable et criteria requis")
		}
		if _, e := w.task(r.ID); e == nil {
			return fmt.Errorf("tâche déjà présente")
		}
		for _, dep := range r.Depends {
			if _, e := w.task(dep); e != nil {
				return e
			}
		}
		w.Tasks = append(w.Tasks, Task{ID: r.ID, Title: r.Title, Deliverable: r.Deliverable, Criteria: r.Criteria, Owner: r.Owner, Depends: r.Depends, Status: "todo", Next: r.Next, Attempts: []Attempt{}})
	case "task.update":
		t, e := w.task(r.ID)
		if e != nil {
			return e
		}
		if e = updateTaskDefinition(w, t, r); e != nil {
			return e
		}
		if r.Status == "" {
			// Metadata edits do not re-accept or relaunch a task, and must not
			// erase a blocker or demand fresh gates merely to change its owner.
			if r.Owner != "" {
				t.Owner = r.Owner
			}
			if r.Next != "" {
				t.Next = r.Next
			}
			if r.Blocker != "" {
				t.Blocker = r.Blocker
			}
			return nil
		}
		if status(r.Status) == "" {
			return fmt.Errorf("statut inconnu")
		}
		transitions := map[string]string{"todo": "running blocked abandoned", "running": "submitted blocked todo abandoned", "blocked": "todo running abandoned", "submitted": "accepted running blocked abandoned", "accepted": "todo", "waived": "todo", "abandoned": "todo"}
		if r.Status != t.Status && !strings.Contains(" "+transitions[t.Status]+" ", " "+r.Status+" ") {
			return &CommandError{Code: "invalid_transition", Message: fmt.Sprintf("transition refusée : %s → %s", t.Status, r.Status)}
		}
		if r.Status == "running" || r.Status == "accepted" {
			for _, dep := range t.Depends {
				d, _ := w.task(dep)
				if !s.acceptedFresh(w, d, map[string]bool{}) {
					return fmt.Errorf("dépendance non validée ou périmée : %s", dep)
				}
			}
		}
		if r.Status == "accepted" && !s.validGate(t) {
			return fmt.Errorf("acceptation refusée : gate delivery courante requise")
		}
		if r.Status == "blocked" && !nonempty(r.Blocker) {
			return fmt.Errorf("motif du blocage requis")
		}
		if t.Status == "running" && r.Status != "running" {
			if r.Outcome != "completed" && r.Outcome != "interrupted" && r.Outcome != "failed" {
				return fmt.Errorf("outcome requis : completed, interrupted, failed")
			}
			if r.Status == "submitted" && r.Outcome != "completed" {
				return fmt.Errorf("soumission : outcome completed requis")
			}
			a := &t.Attempts[len(t.Attempts)-1]
			a.Status = r.Outcome
			a.Ended = now()
		}
		if r.Status == "running" && t.Status != "running" {
			t.Attempts = append(t.Attempts, Attempt{ID: newID("a-"), Status: "recorded", Started: now()})
			t.Gate = nil
		}
		if r.Status == "todo" && t.Status != r.Status {
			if t.Status == "accepted" && !s.acceptedFresh(w, t, map[string]bool{}) && t.Gate != nil {
				t.Revalidation = &Revalidation{PreviousArtifacts: t.Gate.Evaluation.Artifacts, Config: t.Gate.Evaluation.ConfigDigest}
			}
			t.Gate = nil
			t.Override = nil
		}
		t.Status = r.Status
		t.Blocker = r.Blocker
		if r.Next != "" {
			t.Next = r.Next
		}
		if r.Owner != "" {
			t.Owner = r.Owner
		}
	case "checkpoint":
		if !nonempty(r.Summary) {
			return fmt.Errorf("summary requis")
		}
		w.Summary = r.Summary
		w.Next = r.Next
		for _, p := range r.Memory {
			if _, e := localFile(s.root, p); e != nil {
				return e
			}
		}
		w.Memory = r.Memory
		w.Git = gitState(s.root)
	case "ooda":
		if !nonempty(r.Observation) || !nonempty(r.Orientation) || !nonempty(r.Decision) || !nonempty(r.Owner) || !nonempty(r.Next) {
			return fmt.Errorf("OODA : observation, orientation, decision, owner et next requis")
		}
		w.Next = r.Next
		if r.Result != "" {
			w.Summary = r.Result
		}
	default:
		return fmt.Errorf("opération inconnue")
	}
	return nil
}

// Every top-level read gets new physical fingerprints. Nested reads share them,
// so common proofs are read once even when many gates reference the same binary.
func (s *Store) readScope() *Store {
	if s.readDigests != nil {
		return s
	}
	clone := *s
	clone.readDigests = map[string]string{}
	return &clone
}
