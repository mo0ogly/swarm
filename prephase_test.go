package main

import (
	"bytes"
	"database/sql"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
)

func prepRequest(p Preparation, action string) PreparationRequest {
	rev := p.Revision
	return PreparationRequest{Version: 1, ID: p.ID, Event: newID("test-"), Action: action, Revision: &rev}
}

func prepCreate(t *testing.T, s *Store) Preparation {
	t.Helper()
	p, e := s.preparationCommand(prepRequest(Preparation{}, "create"))
	if e != nil {
		t.Fatal(e)
	}
	return p
}

func prepSave(t *testing.T, s *Store, p Preparation, kind, text string) Preparation {
	t.Helper()
	r := prepRequest(p, "save")
	r.Document = kind
	r.Text = text
	v, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	return v
}

func prepMethods(t *testing.T, s *Store) {
	t.Helper()
	for _, path := range []string{".claude/skills/apex/SKILL.md", ".claude/commands/ks-feature.md", ".claude/commands/ks-plan.md", ".claude/skills/audit-pdca/SKILL.md", "tools/agent-workflows/CONTRACT.md"} {
		full := filepath.Join(s.root, path)
		if e := os.MkdirAll(filepath.Dir(full), 0700); e != nil {
			t.Fatal(e)
		}
		if e := os.WriteFile(full, []byte("Préparation seulement"), 0600); e != nil {
			t.Fatal(e)
		}
	}
}

func TestPreparationDraftReopenHistoryAndNoAgents(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	if p.Method != "apex" || p.MethodHash != "" {
		t.Fatal("draft must work without methods")
	}
	p = prepSave(t, s, p, "besoin", "Évolution web et CLI")
	p = prepSave(t, s, p, "besoin", "Version courante")
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	got, e := other.preparation(p.ID)
	if e != nil || got.Documents["besoin"].Text != "Version courante" {
		t.Fatal(got, e)
	}
	h, e := other.preparationHistory(p.ID, "besoin", 0)
	if e != nil || len(h) != 2 || h[1].Text != "Évolution web et CLI" {
		t.Fatal(h, e)
	}
	h, e = other.preparationHistory(p.ID, "besoin", h[0].Revision)
	if e != nil || len(h) != 1 {
		t.Fatal(h, e)
	}
	for _, table := range []string{"works", "agents", "assist_turns", "reservations"} {
		var n int
		if e = s.db.QueryRow("SELECT count(*) FROM " + table).Scan(&n); e != nil {
			t.Fatal(e)
		}
		if n != 0 {
			t.Fatal("preparation created side effects", table, n)
		}
	}
}

func TestPreparationReceiptsAndConcurrentClients(t *testing.T) {
	s := storeTest(t)
	r := prepRequest(Preparation{}, "create")
	p, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	dup, e := s.preparationCommand(r)
	if e != nil || dup.ID != p.ID {
		t.Fatal(dup, e)
	}
	r.Title = "different"
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("event payload changed")
	}
	r = prepRequest(p, "save")
	r.Document = "brief"
	r.Text = "A"
	a, e := s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	b := prepSave(t, s, a, "brief", "B")
	receipt, e := s.preparationCommand(r)
	if e != nil || receipt.Revision != a.Revision || !receipt.ReceiptHistorical {
		t.Fatal("lost response replay failed", e)
	}
	current, _ := s.preparation(p.ID)
	if current.Revision != b.Revision || current.Documents["brief"].Text != "B" {
		t.Fatal("replay clobbered current")
	}
	r.Event = newID("new-")
	r.Text = "C"
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("stale write accepted")
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	start := make(chan struct{})
	out := make(chan error, 2)
	var wg sync.WaitGroup
	for _, client := range []*Store{s, other} {
		wg.Add(1)
		go func(client *Store) {
			defer wg.Done()
			<-start
			r := prepRequest(b, "save")
			r.Document = "brief"
			r.Text = "concurrent"
			_, err := client.preparationCommand(r)
			out <- err
		}(client)
	}
	close(start)
	wg.Wait()
	close(out)
	success := 0
	for err := range out {
		if err == nil {
			success++
		}
	}
	if success != 1 {
		t.Fatal("expected exactly one concurrent writer", success)
	}
	h, _ := s.preparationHistory(p.ID, "brief", 0)
	if len(h) != 3 {
		t.Fatal("orphan or lost history", h)
	}
}

func TestPreparationPlanFreshnessAndMethodVersions(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	p = prepSave(t, s, p, "brief", "Brief A")
	r := prepRequest(p, "adopt-brief")
	r.Hash = "wrong"
	if _, e := s.preparationCommand(r); e == nil {
		t.Fatal("unreviewed adoption")
	}
	r.Hash = p.Documents["brief"].Hash
	var e error
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	spec := fixturePlan()
	spec.Questions = []PlanQuestion{{Question: "Périmètre ?", Answer: "Web et CLI"}}
	b, _ := json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(b))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, e = s.preparationCommand(r)
	if e != nil || !p.PlanReady {
		t.Fatal("human-answered plan not valid", p, e)
	}
	if e = os.WriteFile(filepath.Join(s.root, ".claude/skills/apex/SKILL.md"), []byte("Nouvelle méthode"), 0600); e != nil {
		t.Fatal(e)
	}
	current, e := s.preparation(p.ID)
	if e != nil || current.PlanReady {
		t.Fatal("stale method remains ready")
	}
	r = prepRequest(p, "method")
	r.Method = "apex"
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("old plan accepted for new method")
	}
	p = prepSave(t, s, p, "plan", string(b))
	p = prepSave(t, s, p, "brief", "Brief B")
	r = prepRequest(p, "adopt-brief")
	r.Hash = p.Documents["brief"].Hash
	p, e = s.preparationCommand(r)
	if e != nil {
		t.Fatal(e)
	}
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("old plan accepted for new brief")
	}
	spec.Questions[0].Answer = ""
	b, _ = json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(b))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	if _, e = s.preparationCommand(r); e == nil {
		t.Fatal("open decision accepted")
	}
}

func TestPreparationValidationExportAndCLI(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	for _, edit := range []func(*PreparationRequest){
		func(r *PreparationRequest) { r.Revision = nil }, func(r *PreparationRequest) { r.Document = "../../outside" },
		func(r *PreparationRequest) { r.Text = strings.Repeat("a", 16001) }, func(r *PreparationRequest) { r.Text = string([]byte{0xff}) },
		func(r *PreparationRequest) { r.WorkID = "another-work" },
	} {
		r := prepRequest(p, "save")
		r.Document = "brief"
		edit(&r)
		if _, e := s.preparationCommand(r); e == nil {
			t.Fatal("invalid request accepted", r)
		}
	}
	p = prepSave(t, s, p, "brief", "Texte <script> non exécuté")
	dir := filepath.Join(t.TempDir(), "export")
	if e := exportPreparation(p, dir); e != nil {
		t.Fatal(e)
	}
	if e := exportPreparation(p, dir); e == nil {
		t.Fatal("export overwrote directory")
	}
	b, e := os.ReadFile(filepath.Join(dir, "brief.md"))
	if e != nil || string(b) != p.Documents["brief"].Text {
		t.Fatal(e)
	}
	var out, errout bytes.Buffer
	if code := run([]string{"--root", s.root, "prepare", "show", p.ID, "--json"}, &out, &errout); code != 0 {
		t.Fatal(code, errout.String())
	}
	var got Preparation
	if e = json.Unmarshal(out.Bytes(), &got); e != nil || got.ID != p.ID {
		t.Fatal(e, out.String())
	}
}

func TestPreparationJSONExitCodesAndFailureContract(t *testing.T) {
	s, p, _ := prepDialogueFixture(t, prepReplyScript)
	stale := prepRequest(p, "save")
	stale.Document = "besoin"
	stale.Text = "Version périmée"
	old := 0
	stale.Revision = &old
	b, _ := json.Marshal(stale)
	input := filepath.Join(t.TempDir(), "stale.json")
	if e := os.WriteFile(input, b, 0600); e != nil {
		t.Fatal(e)
	}
	var out, errout bytes.Buffer
	code := run([]string{"--root", s.root, "prepare", "save", p.ID, "--input", input, "--json"}, &out, &errout)
	if code != 3 || out.Len() != 0 {
		t.Fatal(code, out.String(), errout.String())
	}
	var body struct {
		Failure CommandError `json:"failure"`
	}
	if e := json.Unmarshal(errout.Bytes(), &body); e != nil || body.Failure.Code != "conflict" || !body.Failure.Retryable {
		t.Fatal(e, errout.String())
	}

	request := PreparationSend{Version: 1, ID: p.ID, Event: "provider-missing", Revision: p.Revision, Provider: "absent", Capability: "absent", Message: "Question"}
	b, _ = json.Marshal(request)
	input = filepath.Join(t.TempDir(), "provider.json")
	if e := os.WriteFile(input, b, 0600); e != nil {
		t.Fatal(e)
	}
	out.Reset()
	errout.Reset()
	code = run([]string{"--root", s.root, "prepare", "send", p.ID, "--input", input, "--json"}, &out, &errout)
	if code != 5 || out.Len() != 0 || !strings.Contains(errout.String(), `"code": "provider_unavailable"`) {
		t.Fatal(code, out.String(), errout.String())
	}
}

func TestPreparationV6MigrationBackupAndCompatibility(t *testing.T) {
	s := storeTest(t)
	if _, e := s.db.Exec(`INSERT INTO works(id,revision,body) VALUES('w-legacy',1,'{"id":"w-legacy","title":"Ancien travail","revision":1}')`); e != nil {
		t.Fatal(e)
	}
	if _, e := s.db.Exec("DROP TRIGGER preparation_launch_insert; DROP TRIGGER preparation_launch_update; DROP TABLE preparation_launch_locks; DROP TABLE preparation_commands; DROP TABLE preparation_documents; DROP TABLE preparations; PRAGMA user_version=6"); e != nil {
		t.Fatal(e)
	}
	other, e := openStore(s.root, false)
	if e != nil {
		t.Fatal(e)
	}
	defer other.db.Close()
	backups, e := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v7-*.db"))
	if e != nil || len(backups) != 1 {
		t.Fatal(backups, e)
	}
	backup, e := sql.Open("sqlite", backups[0])
	if e != nil {
		t.Fatal(e)
	}
	defer backup.Close()
	var oldVersion, count int
	if e = backup.QueryRow("PRAGMA user_version").Scan(&oldVersion); e != nil || oldVersion != 6 {
		t.Fatal("backup is not pre-migration", oldVersion, e)
	}
	if e = backup.QueryRow("SELECT count(*) FROM works WHERE id='w-legacy'").Scan(&count); e != nil || count != 1 {
		t.Fatal("legacy work missing in backup", e)
	}
	if old, e := other.get("w-legacy"); e != nil || old.Title != "Ancien travail" {
		t.Fatal("migration changed existing work", e)
	}
	p := prepCreate(t, other)
	if p.ID == "" {
		t.Fatal("migration incomplete")
	}
	if _, e = other.db.Exec("PRAGMA user_version=999"); e != nil {
		t.Fatal(e)
	}
	if future, e := openStore(s.root, false); e == nil {
		future.db.Close()
		t.Fatal("unknown schema accepted")
	}
}

func TestPreparationChangedNeedInvalidatesBrief(t *testing.T) {
	s := storeTest(t)
	prepMethods(t, s)
	p := prepCreate(t, s)
	p = prepSave(t, s, p, "besoin", "A")
	p = prepSave(t, s, p, "brief", "Brief A")
	p = prepSave(t, s, p, "besoin", "B")
	r := prepRequest(p, "adopt-brief")
	r.Hash = p.Documents["brief"].Hash
	if _, e := s.preparationCommand(r); e == nil {
		t.Fatal("brief from previous need adopted")
	}
	p = prepSave(t, s, p, "brief", "Brief B")
	r = prepRequest(p, "adopt-brief")
	r.Hash = p.Documents["brief"].Hash
	if _, e := s.preparationCommand(r); e != nil {
		t.Fatal(e)
	}
}
