package engine

import (
	"bytes"
	"crypto/sha256"
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"sync"
	"testing"
)

func TestGraphDraftB01PreservesAcceptedEvidence(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "running"})
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "submitted", Outcome: "completed"})
	w = gateTest(t, s, w, gateDocument(t, s, "t1"))
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Status: "accepted"})
	proofPath := filepath.Join(s.root, "proof-t1.txt")
	proofBefore, err := os.ReadFile(proofPath)
	if err != nil {
		t.Fatal(err)
	}
	acceptedBefore, err := json.Marshal(w.Tasks[0])
	if err != nil {
		t.Fatal(err)
	}
	assertPreserved := func(stage string) Work {
		t.Helper()
		current, err := s.get(w.ID)
		if err != nil {
			t.Fatal(err)
		}
		accepted, err := current.task("t1")
		if err != nil {
			t.Fatal(err)
		}
		encoded, err := json.Marshal(accepted)
		if err != nil || !bytes.Equal(encoded, acceptedBefore) {
			t.Fatalf("%s changed the accepted task, gate or attempt history: %v", stage, err)
		}
		proofAfter, err := os.ReadFile(proofPath)
		if err != nil || !bytes.Equal(proofBefore, proofAfter) || sha256.Sum256(proofBefore) != sha256.Sum256(proofAfter) {
			t.Fatalf("%s changed accepted proof content or digest: %v", stage, err)
		}
		if accepted.Status != "accepted" || !s.validGate(accepted) {
			t.Fatalf("%s invalidated accepted evidence", stage)
		}
		return current
	}
	d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	assertPreserved("preview")
	applyGraphDraftTest(t, s, w, d, p, "accepted-proof-apply")
	after := assertPreserved("apply")
	if !taskDependsOn(t, after, "t2", "t1") || after.Revision != w.Revision+1 {
		t.Fatal("draft was not actually applied")
	}
}

func graphDraftWork(t *testing.T, s *Store) Work {
	t.Helper()
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Amont", Deliverable: "preuve", Criteria: []string{"preuve"}})
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Aval", Deliverable: "preuve", Criteria: []string{"preuve"}})
	w = applyTest(t, s, w, "task.add", Request{ID: "t3", Title: "Autre", Deliverable: "preuve", Criteria: []string{"preuve"}})
	return w
}

func createGraphDraftTest(t *testing.T, s *Store, w Work, operations ...GraphDraftOperation) (GraphDraft, GraphDraftPreview) {
	t.Helper()
	d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: operations})
	if err != nil {
		t.Fatal(err)
	}
	p, err := s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
	if err != nil {
		t.Fatal(err)
	}
	return d, p
}

func applyGraphDraftTest(t *testing.T, s *Store, w Work, d GraphDraft, p GraphDraftPreview, event string) GraphDraftApplyResult {
	t.Helper()
	result, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: event, ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
	if err != nil {
		t.Fatal(err)
	}
	return result
}

func graphCode(err error) string {
	var command *CommandError
	if errors.As(err, &command) {
		return command.Code
	}
	return ""
}

func taskDependsOn(t *testing.T, w Work, task, dependency string) bool {
	t.Helper()
	value, err := w.task(task)
	if err != nil {
		t.Fatal(err)
	}
	for _, candidate := range value.Depends {
		if candidate == dependency {
			return true
		}
	}
	return false
}

func TestGraphDraftB01Persistence(t *testing.T) {
	root := t.TempDir()
	s, err := openStore(root, true)
	if err != nil {
		t.Fatal(err)
	}
	w := graphDraftWork(t, s)
	d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}})
	if err != nil {
		t.Fatal(err)
	}
	before, _ := s.get(w.ID)
	if taskDependsOn(t, before, "t2", "t1") {
		t.Fatal("la sauvegarde du brouillon a modifié le plan")
	}
	s.db.Close()
	s, err = openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { s.db.Close() })
	loaded, err := s.getGraphDraft(operatorIdentity(), w.ID, d.ID)
	if err != nil || loaded.ContentDigest != d.ContentDigest || loaded.Status != "editing" {
		t.Fatalf("brouillon non durable: %#v %v", loaded, err)
	}
	p, err := s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
	if err != nil {
		t.Fatal(err)
	}
	result := applyGraphDraftTest(t, s, w, d, p, "persist-apply")
	if result.Revision != w.Revision+1 {
		t.Fatalf("révision=%d", result.Revision)
	}
	after, _ := s.get(w.ID)
	if !taskDependsOn(t, after, "t2", "t1") {
		t.Fatal("ajout non appliqué")
	}
	var agents int
	if err = s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&agents); err != nil || agents != 0 {
		t.Fatalf("départ implicite: %d %v", agents, err)
	}
	d2, p2 := createGraphDraftTest(t, s, after, GraphDraftOperation{Kind: "remove_dependency", Prerequisite: "t1", Dependent: "t2"})
	applyGraphDraftTest(t, s, after, d2, p2, "persist-remove")
	removed, _ := s.get(w.ID)
	if taskDependsOn(t, removed, "t2", "t1") {
		t.Fatal("retrait non appliqué")
	}
	t.Run("migration-from-v23", func(t *testing.T) {
		legacyRoot := t.TempDir()
		legacy, openErr := openStore(legacyRoot, true)
		if openErr != nil {
			t.Fatal(openErr)
		}
		if _, openErr = legacy.db.Exec("DROP TABLE graph_draft_authorizations; DROP TABLE graph_drafts; PRAGMA user_version=23"); openErr != nil {
			t.Fatal(openErr)
		}
		if openErr = legacy.db.Close(); openErr != nil {
			t.Fatal(openErr)
		}
		migrated, openErr := openStore(legacyRoot, false)
		if openErr != nil {
			t.Fatal(openErr)
		}
		defer migrated.db.Close()
		var version, tables int
		if openErr = migrated.db.QueryRow("PRAGMA user_version").Scan(&version); openErr != nil {
			t.Fatal(openErr)
		}
		if openErr = migrated.db.QueryRow("SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN ('graph_drafts','graph_draft_authorizations')").Scan(&tables); openErr != nil {
			t.Fatal(openErr)
		}
		if version != schemaVersion || tables != 2 {
			t.Fatalf("migration incomplète: version=%d tables=%d", version, tables)
		}
	})
}

func TestGraphDraftB01InvalidDependencies(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	w = applyTest(t, s, w, "task.update", Request{ID: "t2", Depends: []string{"t1"}})
	cases := []struct {
		name, code string
		op         GraphDraftOperation
	}{
		{"unknown", "unknown_task", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "absent", Dependent: "t3"}},
		{"duplicate", "duplicate_dependency", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}},
		{"cycle", "dependency_cycle", GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"}},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{tc.op}})
			if err != nil {
				t.Fatal(err)
			}
			_, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
			if graphCode(err) != tc.code {
				t.Fatalf("code=%q err=%v", graphCode(err), err)
			}
			current, getErr := s.get(w.ID)
			if getErr != nil || current.Revision != w.Revision || !taskDependsOn(t, current, "t2", "t1") {
				t.Fatalf("état altéré après refus: rev=%d err=%v", current.Revision, getErr)
			}
		})
	}
}

func TestGraphDraftB01ConcurrentApply(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	d1, p1 := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	d2, p2 := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t3"})
	requests := []GraphDraftApplyRequest{
		{Schema: 1, WorkID: w.ID, DraftID: d1.ID, EventID: "concurrent-one", ExpectedRevision: w.Revision, PreviewToken: p1.PreviewToken, ContentDigest: p1.ContentDigest},
		{Schema: 1, WorkID: w.ID, DraftID: d2.ID, EventID: "concurrent-two", ExpectedRevision: w.Revision, PreviewToken: p2.PreviewToken, ContentDigest: p2.ContentDigest},
	}
	var wg sync.WaitGroup
	errs := make([]error, 2)
	for i := range requests {
		wg.Add(1)
		go func(i int) { defer wg.Done(); _, errs[i] = s.applyGraphDraft(operatorIdentity(), requests[i]) }(i)
	}
	wg.Wait()
	passes, conflicts := 0, 0
	for _, err := range errs {
		if err == nil {
			passes++
		} else if graphCode(err) == "revision_conflict" {
			conflicts++
		} else {
			t.Fatalf("erreur inattendue: %v", err)
		}
	}
	if passes != 1 || conflicts != 1 {
		t.Fatalf("passes=%d conflits=%d erreurs=%v", passes, conflicts, errs)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision+1 {
		t.Fatalf("deuxième effet ou effet perdu, révision=%d", current.Revision)
	}
}

func TestGraphDraftB01Idempotency(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	request := GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "stable-event", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest}
	first, err := s.applyGraphDraft(operatorIdentity(), request)
	if err != nil {
		t.Fatal(err)
	}
	second, err := s.applyGraphDraft(operatorIdentity(), request)
	if err != nil || second.Revision != first.Revision || second.AppliedAt != first.AppliedAt {
		t.Fatalf("rejeu non identique: %#v %v", second, err)
	}
	request.ContentDigest = "different-content"
	if _, err = s.applyGraphDraft(operatorIdentity(), request); graphCode(err) != "event_conflict" {
		t.Fatalf("contenu différent accepté: %v", err)
	}
	current, _ := s.get(w.ID)
	if current.Revision != first.Revision {
		t.Fatal("le rejeu a produit un second effet")
	}
}

func TestGraphDraftB01Authorization(t *testing.T) {
	s := storeTest(t)
	w := graphDraftWork(t, s)
	if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, true, nil); err != nil {
		t.Fatal(err)
	}
	d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"})
	if err := s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, false, nil); err != nil {
		t.Fatal(err)
	}
	_, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "revoked-event", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
	if graphCode(err) != "authorization_required" {
		t.Fatalf("droit retiré non refusé: %v", err)
	}
	current, _ := s.get(w.ID)
	if current.Revision != w.Revision || taskDependsOn(t, current, "t2", "t1") {
		t.Fatal("effet malgré révocation")
	}
	if err = s.setGraphDraftAuthorization(w.ID, operatorIdentity(), true, true, []string{"t2"}); err != nil {
		t.Fatal(err)
	}
	d2, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}, {Kind: "add_dependency", Prerequisite: "t2", Dependent: "t3"}}})
	if err != nil {
		t.Fatal(err)
	}
	_, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d2.ID, ExpectedRevision: w.Revision})
	if graphCode(err) != "authorization_required" {
		t.Fatalf("portée excessive non refusée: %v", err)
	}
}

func TestGraphDraftB01ActiveScope(t *testing.T) {
	t.Run("active-before-preview", func(t *testing.T) {
		s := storeTest(t)
		w := graphDraftWork(t, s)
		w = applyTest(t, s, w, "task.update", Request{ID: "t2", Status: "running"})
		d, err := s.saveGraphDraft(operatorIdentity(), GraphDraftSaveRequest{Schema: 1, WorkID: w.ID, ExpectedRevision: w.Revision, Operations: []GraphDraftOperation{{Kind: "add_dependency", Prerequisite: "t1", Dependent: "t2"}}})
		if err != nil {
			t.Fatal(err)
		}
		_, err = s.previewGraphDraft(operatorIdentity(), GraphDraftPreviewRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, ExpectedRevision: w.Revision})
		if graphCode(err) != "active_scope_conflict" {
			t.Fatalf("périmètre actif non refusé: %v", err)
		}
		current, _ := s.get(w.ID)
		if current.Revision != w.Revision || taskDependsOn(t, current, "t2", "t1") {
			t.Fatal("état modifié malgré tâche active")
		}
	})
	t.Run("active-after-preview", func(t *testing.T) {
		s := storeTest(t)
		w, launch := setupAgent(t, s)
		w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Autre", Deliverable: "preuve", Criteria: []string{"preuve"}})
		launch.Revision = w.Revision
		d, p := createGraphDraftTest(t, s, w, GraphDraftOperation{Kind: "add_dependency", Prerequisite: "t2", Dependent: "t1"})
		if _, created, err := s.prepare(w.ID, launch); err != nil || !created {
			t.Fatalf("agent actif non préparé: created=%v err=%v", created, err)
		}
		_, err := s.applyGraphDraft(operatorIdentity(), GraphDraftApplyRequest{Schema: 1, WorkID: w.ID, DraftID: d.ID, EventID: "active-after-preview", ExpectedRevision: w.Revision, PreviewToken: p.PreviewToken, ContentDigest: p.ContentDigest})
		if graphCode(err) != "active_scope_conflict" {
			t.Fatalf("activité apparue après preview non refusée: %v", err)
		}
		current, _ := s.get(w.ID)
		if taskDependsOn(t, current, "t1", "t2") {
			t.Fatal("état modifié malgré activité concurrente")
		}
	})
}
