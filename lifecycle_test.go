//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"syscall"
	"testing"
	"time"
)

func lifecycleFixture(t *testing.T, s *Store) Work {
	t.Helper()
	w := taskTest(t, s, createTest(t, s))
	proof := filepath.Join(s.root, "mission-proof.txt")
	if err := os.WriteFile(proof, []byte("preuve à conserver"), 0600); err != nil {
		t.Fatal(err)
	}
	w = applyTest(t, s, w, "checkpoint", Request{Summary: "preuve disponible", Next: "archiver", Memory: []string{"mission-proof.txt"}})
	return w
}

func lifecycleRequest(p LifecyclePreview, event string) LifecycleRequest {
	return LifecycleRequest{Schema: 1, EventID: event, Revision: p.Revision, Action: p.Action, RetentionDays: p.RetentionDays, PreviewToken: p.Token}
}

func TestLifecycleArchiveRestoreAndRevisionConfirmation(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "archive"})
	if err != nil || p.Token == "" || len(p.Data) == 0 || len(p.Excluded) != 3 {
		t.Fatalf("aperçu incomplet: %+v %v", p, err)
	}
	req := lifecycleRequest(p, "archive-once")
	receipt, err := s.lifecycleApply(w.ID, req)
	if err != nil || receipt.ArchivePath == "" || !receipt.Restorable {
		t.Fatalf("archive: %+v %v", receipt, err)
	}
	if _, err = os.Stat(receipt.ArchivePath); err != nil {
		t.Fatal(err)
	}
	other := storeTest(t)
	imported, importErr := other.importBundle(receipt.ArchivePath)
	if importErr != nil || imported.ID != w.ID || imported.Title != w.Title {
		t.Fatalf("archive non exportable: %+v %v", imported, importErr)
	}
	if _, err = s.dispatch(w.ID); err == nil || commandFailure(err).Code != "mission_archived" {
		t.Fatalf("départ admis pendant archivage: %v", err)
	}
	replayed, err := s.lifecycleApply(w.ID, req)
	if err != nil || replayed.ArchivePath != receipt.ArchivePath {
		t.Fatalf("double clic non idempotent: %+v %v", replayed, err)
	}
	restore, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if err != nil || !restore.Archived {
		t.Fatalf("restauration absente: %+v %v", restore, err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(restore, "restore-once")); err != nil {
		t.Fatal(err)
	}
	stale := lifecycleRequest(p, "stale-archive")
	if _, err = s.lifecycleApply(w.ID, stale); err == nil || commandFailure(err).Code != "preview_conflict" {
		t.Fatalf("ancien aperçu accepté: %v", err)
	}
	if got, readErr := os.ReadFile(filepath.Join(s.root, "mission-proof.txt")); readErr != nil || string(got) != "preuve à conserver" {
		t.Fatalf("source projet modifiée: %q %v", got, readErr)
	}
}

func TestLifecyclePurgeRetentionPreservesReceiptsAndProjectFiles(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	old := time.Now().UTC().Add(-60 * 24 * time.Hour).Format(time.RFC3339Nano)
	recent := time.Now().UTC().Format(time.RFC3339Nano)
	a := Agent{ID: "agent-fini", WorkID: w.ID, TaskID: "t1", Attempt: "attempt-fini", Status: "completed", Started: old, Ended: old, CWD: s.root}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?),(?,?,?,?)", a.ID, old, "output", "ancien", a.ID, recent, "output", "récent"); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("INSERT INTO terminal_events(agent_id,data) VALUES(?,?)", a.ID, []byte("ancienne sortie")); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("INSERT INTO cockpit_events(work_id,at,kind,message) VALUES(?,?,?,?)", w.ID, old, "validation-accepted", "reçu à garder"); err != nil {
		t.Fatal(err)
	}
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "purge", RetentionDays: 30})
	if err != nil || p.Data[0].Count != 1 || p.Data[1].Count != 1 {
		t.Fatalf("aperçu de purge: %+v %v", p, err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(p, "purge-old")); err != nil {
		t.Fatal(err)
	}
	var logs, terminal, receipts int
	_ = s.db.QueryRow("SELECT count(*) FROM agent_logs WHERE agent_id=?", a.ID).Scan(&logs)
	_ = s.db.QueryRow("SELECT count(*) FROM terminal_events WHERE agent_id=?", a.ID).Scan(&terminal)
	_ = s.db.QueryRow("SELECT count(*) FROM cockpit_events WHERE work_id=? AND kind='validation-accepted'", w.ID).Scan(&receipts)
	if logs != 1 || terminal != 0 || receipts != 1 {
		t.Fatalf("rétention incorrecte logs=%d terminal=%d reçus=%d", logs, terminal, receipts)
	}
	if _, err = os.Stat(filepath.Join(s.root, "mission-proof.txt")); err != nil {
		t.Fatal("preuve projet supprimée", err)
	}
	if _, err = s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "purge"}); err == nil || commandFailure(err).Code != "invalid_retention" {
		t.Fatalf("purge globale implicite acceptée: %v", err)
	}
}

func TestLifecycleDeleteTrashRestoreAndActiveGuards(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	a := Agent{ID: "agent-actif", WorkID: w.ID, TaskID: "t1", Attempt: "attempt", Status: "queued", Started: now(), CWD: s.root}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	blocked, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if err != nil || len(blocked.Blocked) == 0 {
		t.Fatalf("agent actif non signalé: %+v %v", blocked, err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(blocked, "delete-blocked")); err == nil || commandFailure(err).Code != "lifecycle_blocked" {
		t.Fatalf("suppression active acceptée: %v", err)
	}
	if _, err = s.db.Exec("UPDATE agents SET status='completed',body=json_set(body,'$.status','completed') WHERE id=?", a.ID); err != nil {
		t.Fatal(err)
	}
	if _, err = s.db.Exec("INSERT INTO agent_logs(agent_id,at,kind,message) VALUES(?,?,?,?)", a.ID, now(), "output", "journal restaurable"); err != nil {
		t.Fatal(err)
	}
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if err != nil || len(p.Blocked) != 0 {
		t.Fatalf("suppression encore bloquée: %+v %v", p, err)
	}
	receipt, err := s.lifecycleApply(w.ID, lifecycleRequest(p, "delete-trash"))
	if err != nil || !receipt.Restorable {
		t.Fatalf("corbeille: %+v %v", receipt, err)
	}
	if _, err = s.get(w.ID); err == nil {
		t.Fatal("mission encore active")
	}
	if _, err = os.Stat(filepath.Join(s.root, "mission-proof.txt")); err != nil {
		t.Fatal("fichier référencé supprimé", err)
	}
	restore, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if err != nil || restore.Title != w.Title {
		t.Fatalf("aperçu corbeille: %+v %v", restore, err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(restore, "restore-trash")); err != nil {
		t.Fatal(err)
	}
	if _, err = s.get(w.ID); err != nil {
		t.Fatal(err)
	}
	var logs int
	if err = s.db.QueryRow("SELECT count(*) FROM agent_logs WHERE agent_id=?", a.ID).Scan(&logs); err != nil || logs != 1 {
		t.Fatalf("journal non restauré: %d %v", logs, err)
	}
}

func TestLifecycleRestoreFailureIsAtomic(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	a := Agent{ID: "agent-conflit", WorkID: w.ID, TaskID: "t1", Attempt: "attempt", Status: "completed", Started: now(), CWD: s.root}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	p, _ := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if _, err := s.lifecycleApply(w.ID, lifecycleRequest(p, "delete-conflict")); err != nil {
		t.Fatal(err)
	}
	other := lifecycleFixture(t, s)
	otherAgent := a
	otherAgent.WorkID = other.ID
	otherAgent.TaskID = "t1"
	otherRaw, _ := json.Marshal(otherAgent)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", otherAgent.ID, other.ID, otherAgent.TaskID, other.ID, otherAgent.Status, otherRaw, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	restore, _ := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "restore"})
	if _, err := s.lifecycleApply(w.ID, lifecycleRequest(restore, "restore-conflict")); err == nil || !strings.Contains(err.Error(), "restauration atomique") {
		t.Fatalf("collision non détectée: %v", err)
	}
	if _, err := s.get(w.ID); err == nil {
		t.Fatal("restauration partielle publiée")
	}
	var trash int
	if err := s.db.QueryRow("SELECT count(*) FROM mission_trash WHERE work_id=?", w.ID).Scan(&trash); err != nil || trash != 1 {
		t.Fatalf("corbeille perdue après échec: %d %v", trash, err)
	}
}

func TestLifecycleArchiveRejectsSymlinkedInternalDirectory(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	target := t.TempDir()
	if err := os.Symlink(target, filepath.Join(s.root, ".swarm", "lifecycle")); err != nil {
		t.Skipf("symlink indisponible: %v", err)
	}
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "archive"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(p, "archive-symlink")); err == nil || !strings.Contains(err.Error(), "non local") {
		t.Fatalf("symlink accepté: %v", err)
	}
}

func TestLifecycleCLIAndWebSharePreviewContract(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	raw, _ := json.Marshal(LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "archive"})
	input := filepath.Join(t.TempDir(), "request.json")
	if err := os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	var out bytes.Buffer
	if err := lifecycleCLI(s, []string{"lifecycle", "preview", w.ID, "archive"}, input, true, &out); err != nil {
		t.Fatal(err)
	}
	var cli LifecyclePreview
	if err := json.Unmarshal(out.Bytes(), &cli); err != nil {
		t.Fatal(err)
	}
	webValue, err := s.webAction(webRequest{Kind: "lifecycle-preview", Work: w.ID, Revision: w.Revision, LifecycleAction: "archive"})
	if err != nil {
		t.Fatal(err)
	}
	web := webValue.(LifecyclePreview)
	if cli.Token != web.Token || cli.Confirmation != web.Confirmation {
		t.Fatalf("contrats divergents: cli=%+v web=%+v", cli, web)
	}
	confirmation := lifecycleRequest(cli, "cli-archive")
	raw, _ = json.Marshal(confirmation)
	if err = os.WriteFile(input, raw, 0600); err != nil {
		t.Fatal(err)
	}
	out.Reset()
	if err = lifecycleCLI(s, []string{"lifecycle", "apply", w.ID, "archive"}, input, true, &out); err != nil {
		t.Fatal(err)
	}
	var archived LifecycleReceipt
	if err = json.Unmarshal(out.Bytes(), &archived); err != nil || archived.Action != "archive" {
		t.Fatalf("reçu CLI: %+v %v", archived, err)
	}
	webValue, err = s.webAction(webRequest{Kind: "lifecycle-preview", Work: w.ID, Revision: w.Revision, LifecycleAction: "restore"})
	if err != nil {
		t.Fatal(err)
	}
	restore := webValue.(LifecyclePreview)
	webValue, err = s.webAction(webRequest{Kind: "lifecycle-apply", Work: w.ID, Revision: w.Revision, LifecycleAction: "restore", Event: "web-restore", PreviewToken: restore.Token})
	if err != nil || webValue.(LifecycleReceipt).Action != "restore" {
		t.Fatalf("reçu web: %+v %v", webValue, err)
	}
	if _, err := s.lifecycleApply(w.ID, LifecycleRequest{Schema: 1, EventID: "wrong-token", Revision: w.Revision, Action: "delete", PreviewToken: strings.Repeat("0", 64)}); err == nil || commandFailure(err).Code != "preview_conflict" {
		t.Fatalf("confirmation arbitraire acceptée: %v", err)
	}
}

func TestLifecycleMissionInventoryAndCLIExposeRecoverableStates(t *testing.T) {
	s := storeTest(t)
	active := lifecycleFixture(t, s)
	archived := lifecycleFixture(t, s)
	archivePreview, err := s.lifecyclePreview(archived.ID, LifecycleRequest{Schema: 1, Revision: archived.Revision, Action: "archive"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(archived.ID, lifecycleRequest(archivePreview, "inventory-archive")); err != nil {
		t.Fatal(err)
	}
	deleted := lifecycleFixture(t, s)
	deletePreview, err := s.lifecyclePreview(deleted.ID, LifecycleRequest{Schema: 1, Revision: deleted.Revision, Action: "delete"})
	if err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(deleted.ID, lifecycleRequest(deletePreview, "inventory-delete")); err != nil {
		t.Fatal(err)
	}

	missions, err := s.lifecycleMissions()
	if err != nil {
		t.Fatal(err)
	}
	states := map[string]string{}
	for _, mission := range missions {
		states[mission.WorkID] = mission.State
	}
	if states[active.ID] != "active" || states[archived.ID] != "archived" || states[deleted.ID] != "trash" {
		t.Fatalf("inventaire incomplet: %#v", states)
	}
	var out bytes.Buffer
	if err = lifecycleCLI(s, []string{"lifecycle", "list"}, "", false, &out); err != nil {
		t.Fatal(err)
	}
	for _, expected := range []string{"Active", "Archivée", "Dans la corbeille", "l’aperçu seul ne modifie rien"} {
		if !strings.Contains(out.String(), expected) {
			t.Fatalf("sortie CLI sans %q: %s", expected, out.String())
		}
	}
}

func TestLifecycleApplyRefusesActiveValidationControl(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "purge", RetentionDays: 30})
	if err != nil {
		t.Fatal(err)
	}
	lock, err := os.OpenFile(filepath.Join(s.root, ".swarm", "automatic-validation.lock"), os.O_CREATE|os.O_RDWR, 0600)
	if err != nil {
		t.Fatal(err)
	}
	defer lock.Close()
	if err = syscall.Flock(int(lock.Fd()), syscall.LOCK_EX|syscall.LOCK_NB); err != nil {
		t.Fatal(err)
	}
	defer syscall.Flock(int(lock.Fd()), syscall.LOCK_UN)
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(p, "purge-control-active")); err == nil || commandFailure(err).Code != "active_control" {
		t.Fatalf("contrôle actif non protégé: %v", err)
	}
}

func TestLifecyclePurgeKeepsRecentlyFinishedLongAttempt(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	old := time.Now().UTC().Add(-60 * 24 * time.Hour).Format(time.RFC3339Nano)
	a := Agent{ID: "long-agent", WorkID: w.ID, TaskID: "t1", Status: "completed", Started: old, Ended: now(), CWD: s.root}
	body, _ := json.Marshal(a)
	if _, err := s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("INSERT INTO terminal_events(agent_id,data) VALUES(?,?)", a.ID, []byte("recent output")); err != nil {
		t.Fatal(err)
	}
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "purge", RetentionDays: 30})
	if err != nil {
		t.Fatal(err)
	}
	if p.Data[1].Count != 0 {
		t.Fatalf("recent terminal output selected: %+v", p.Data)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(p, "keep-recent")); err != nil {
		t.Fatal(err)
	}
	var n int
	s.db.QueryRow("SELECT count(*) FROM terminal_events WHERE agent_id=?", a.ID).Scan(&n)
	if n != 1 {
		t.Fatal("recent output removed")
	}
}

// A web heartbeat also observes idle missions; observation alone must not
// prevent cleanup. Enabled automatic dispatch still prevents it.
func TestLifecycleObservedIdleMissionCanBeManaged(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	if err := s.beginMissionSupervision(w.ID, "web", "serveur web", time.Now()); err != nil {
		t.Fatal(err)
	}
	preview := func() LifecyclePreview {
		p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
		if err != nil {
			t.Fatal(err)
		}
		return p
	}
	if p := preview(); len(p.Blocked) != 0 {
		t.Fatalf("idle observation blocks cleanup: %v", p.Blocked)
	}
	if _, err := s.db.Exec("INSERT INTO cockpit_controls(work_id,autonomy,slots,paused) VALUES(?, 'autonome', 1, 0) ON CONFLICT(work_id) DO UPDATE SET autonomy='autonome',paused=0", w.ID); err != nil {
		t.Fatal(err)
	}
	if err := s.setMission(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if p := preview(); len(p.Blocked) == 0 {
		t.Fatal("enabled dispatch must block cleanup")
	}
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	if p := preview(); len(p.Blocked) != 0 {
		t.Fatalf("paused observation blocks cleanup: %v", p.Blocked)
	}
}

func TestLifecycleConcurrentConfirmationHasOneReceipt(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	other, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer other.db.Close()
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "archive"})
	if err != nil {
		t.Fatal(err)
	}
	req := lifecycleRequest(p, "concurrent-archive")
	start := make(chan struct{})
	results := make(chan error, 2)
	for _, store := range []*Store{s, other} {
		go func(st *Store) { <-start; _, err := st.lifecycleApply(w.ID, req); results <- err }(store)
	}
	close(start)
	success := 0
	for i := 0; i < 2; i++ {
		if err := <-results; err == nil {
			success++
		}
	}
	if success == 0 {
		t.Fatal("aucune confirmation réussie")
	}
	// A concurrent loser can retry exactly the same operation safely.
	if _, err := other.lifecycleApply(w.ID, req); err != nil {
		t.Fatal(err)
	}
	var count int
	if err := s.db.QueryRow("SELECT count(*) FROM lifecycle_receipts WHERE work_id=?", w.ID).Scan(&count); err != nil || count != 1 {
		t.Fatalf("reçus=%d err=%v", count, err)
	}
}

func TestLifecycleApplyRechecksAgentQueuedAfterPreview(t *testing.T) {
	s := storeTest(t)
	w := lifecycleFixture(t, s)
	p, err := s.lifecyclePreview(w.ID, LifecycleRequest{Schema: 1, Revision: w.Revision, Action: "delete"})
	if err != nil {
		t.Fatal(err)
	}
	a := Agent{ID: "queued-after-preview", WorkID: w.ID, TaskID: "t1", Status: "queued", CWD: s.root}
	body, _ := json.Marshal(a)
	if _, err = s.db.Exec("INSERT INTO agents(id,work_id,task_id,cwd,status,desired,body,request) VALUES(?,?,?,?,?,'',?,?)", a.ID, w.ID, a.TaskID, s.root, a.Status, body, []byte(`{}`)); err != nil {
		t.Fatal(err)
	}
	if _, err = s.lifecycleApply(w.ID, lifecycleRequest(p, "stale-queued")); err == nil {
		t.Fatal("départ réservé effacé après aperçu")
	}
	if _, err = s.get(w.ID); err != nil {
		t.Fatal("mission perdue", err)
	}
	var count int
	if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE id=?", a.ID).Scan(&count); err != nil || count != 1 {
		t.Fatal("agent perdu", err)
	}
}
