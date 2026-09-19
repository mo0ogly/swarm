//go:build linux

package main

import (
	"os"
	"path/filepath"
	"sync"
	"testing"
	"time"
)

func TestWorkspaceCoordinationMigratesFromV15(t *testing.T) {
	s := storeTest(t)
	root := s.root
	if _, err := s.db.Exec(`DROP TABLE workspace_integrations; DROP TABLE workspace_turns; PRAGMA user_version=15`); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	var version int
	if err = reopened.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion {
		t.Fatal(version, err)
	}
	if _, err = reopened.workspaceTurns(); err != nil {
		t.Fatal(err)
	}
}

func TestWorkspaceTurnsAreFIFOAcrossMissionsAndReleaseIsVerified(t *testing.T) {
	s := storeTest(t)
	firstWork, _ := setupAgent(t, s)
	secondWork, _ := setupAgent(t, s)
	shared := filepath.Join(s.root, "shared")
	if err := os.Mkdir(shared, 0700); err != nil {
		t.Fatal(err)
	}
	if err := s.enqueueWorkspaceTurn(firstWork.ID, "t1", shared); err != nil {
		t.Fatal(err)
	}
	if err := s.enqueueWorkspaceTurn(secondWork.ID, "t1", shared); err != nil {
		t.Fatal(err)
	}
	turns, err := s.workspaceTurns()
	if err != nil || len(turns) != 2 {
		t.Fatal(turns, err)
	}
	blocked := workspaceTurnBlockers(turns, secondWork.ID)
	if blocked["t1"] == "" {
		t.Fatalf("le second tour commun a dépassé le premier : %+v", turns)
	}
	tx, err := s.db.Begin()
	if err != nil {
		t.Fatal(err)
	}
	if err = claimWorkspaceTurn(tx, firstWork.ID, "t1", shared, "agent-first"); err != nil {
		t.Fatal(err)
	}
	if err = tx.Commit(); err != nil {
		t.Fatal(err)
	}
	if err = s.releaseWorkspaceTurn(Agent{ID: "wrong", WorkID: firstWork.ID, TaskID: "t1", Origin: originConductor}); err == nil {
		t.Fatal("une libération par un autre agent a été annoncée")
	}
	if err = s.releaseWorkspaceTurn(Agent{ID: "agent-first", WorkID: firstWork.ID, TaskID: "t1", Origin: originConductor}); err != nil {
		t.Fatal(err)
	}
	turns, _ = s.workspaceTurns()
	if blocked = workspaceTurnBlockers(turns, secondWork.ID); blocked["t1"] != "" {
		t.Fatalf("le tour suivant n’a pas été réveillé après libération : %s", blocked["t1"])
	}
}

func TestTwoAgentsReallyRunTogetherInDisjointRecipeDirectories(t *testing.T) {
	s := storeTest(t)
	firstWork, first := setupAgent(t, s)
	secondWork, second := setupAgent(t, s)
	first.Workspace = filepath.Join(s.root, "recipe-a")
	second.Workspace = filepath.Join(s.root, "recipe-b")
	first.Instruction = "TEST_SLEEP"
	second.Instruction = "TEST_SLEEP"
	for _, path := range []string{first.Workspace, second.Workspace} {
		if err := os.Mkdir(path, 0700); err != nil {
			t.Fatal(err)
		}
	}
	firstAgent, _, err := s.prepare(firstWork.ID, first)
	if err != nil {
		t.Fatal(err)
	}
	secondAgent, _, err := s.prepare(secondWork.ID, second)
	if err != nil {
		t.Fatal(err)
	}
	var group sync.WaitGroup
	errors := make(chan error, 2)
	for _, id := range []string{firstAgent.ID, secondAgent.ID} {
		group.Add(1)
		go func(agentID string) {
			defer group.Done()
			errors <- s.supervise(agentID)
		}(id)
	}
	deadline := time.Now().Add(5 * time.Second)
	for {
		a, firstErr := s.agent(firstAgent.ID)
		b, secondErr := s.agent(secondAgent.ID)
		if firstErr != nil || secondErr != nil {
			t.Fatal(firstErr, secondErr)
		}
		if a.Status == "running" && b.Status == "running" {
			break
		}
		if time.Now().After(deadline) {
			t.Fatalf("chevauchement réel non observé : %s / %s", a.Status, b.Status)
		}
		time.Sleep(20 * time.Millisecond)
	}
	if err = s.stopAgent(firstAgent.ID); err != nil {
		t.Fatal(err)
	}
	if err = s.stopAgent(secondAgent.ID); err != nil {
		t.Fatal(err)
	}
	group.Wait()
	close(errors)
	for err = range errors {
		if err != nil {
			t.Fatal(err)
		}
	}
}

func TestSerializedIntegrationKeepsConcurrentConflictUntouched(t *testing.T) {
	s, work, producer, consumer, artifact := exchangeFixture(t)
	exchange, _, err := s.sendExchange(work.ID, ExchangeSend{Schema: 1, EventID: "handoff-integration", Kind: "handoff",
		AgentID: producer.ID, TaskID: producer.TaskID, AttemptID: producer.Attempt, RecipientTask: consumer.TaskID,
		RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{artifact}})
	if err != nil {
		t.Fatal(err)
	}
	canonical := filepath.Join(s.root, "canonical")
	if err = os.Mkdir(canonical, 0700); err != nil {
		t.Fatal(err)
	}
	target := filepath.Join(canonical, "result.txt")
	if err = os.WriteFile(target, []byte("base"), 0600); err != nil {
		t.Fatal(err)
	}
	base := hash([]byte("base"))
	makeRequest := func(id string) IntegrationRequest {
		return IntegrationRequest{Schema: 1, EventID: id, ExchangeID: exchange.ID, CanonicalWorkspace: canonical,
			Artifacts: []IntegrationArtifact{{Source: artifact.Path, Target: "result.txt", BaseSHA256: base, ResultSHA256: artifact.SHA256}}}
	}
	start := make(chan struct{})
	receipts := make(chan IntegrationReceipt, 2)
	errors := make(chan error, 2)
	var group sync.WaitGroup
	for _, id := range []string{"integration-one", "integration-two"} {
		group.Add(1)
		go func(eventID string) {
			defer group.Done()
			<-start
			receipt, _, integrationErr := s.integrateWorkspace(work.ID, makeRequest(eventID))
			receipts <- receipt
			errors <- integrationErr
		}(id)
	}
	close(start)
	group.Wait()
	close(receipts)
	close(errors)
	conflictErrors := 0
	for err = range errors {
		if err != nil {
			if commandFailure(err).Code != "conflict" {
				t.Fatal(err)
			}
			conflictErrors++
		}
	}
	states := map[string]int{}
	for receipt := range receipts {
		states[receipt.State]++
		if receipt.State == "conflict" && len(receipt.Conflicts) == 0 {
			t.Fatal("conflit sans preuve conservée", receipt)
		}
	}
	if states["integrated"] != 1 || states["conflict"] != 1 {
		t.Fatalf("une intégration et un conflit attendus : %+v", states)
	}
	if conflictErrors != 1 {
		t.Fatalf("le conflit n’a pas produit un échec explicite : %d", conflictErrors)
	}
	content, err := os.ReadFile(target)
	if err != nil || string(content) != "proof" {
		t.Fatalf("le résultat intégré a été écrasé : %q, %v", content, err)
	}
}

func TestIntegrationDetectsExternalEditBeforeAnyWrite(t *testing.T) {
	s, work, producer, consumer, artifact := exchangeFixture(t)
	exchange, _, err := s.sendExchange(work.ID, ExchangeSend{Schema: 1, EventID: "handoff-external", Kind: "handoff",
		AgentID: producer.ID, TaskID: producer.TaskID, AttemptID: producer.Attempt, RecipientTask: consumer.TaskID,
		RecipientRole: "worker", ResultState: "completed", Artifacts: []ExchangeArtifact{artifact}})
	if err != nil {
		t.Fatal(err)
	}
	canonical := filepath.Join(s.root, "canonical-external")
	if err = os.Mkdir(canonical, 0700); err != nil {
		t.Fatal(err)
	}
	target := filepath.Join(canonical, "result.txt")
	if err = os.WriteFile(target, []byte("modification externe"), 0600); err != nil {
		t.Fatal(err)
	}
	receipt, created, err := s.integrateWorkspace(work.ID, IntegrationRequest{Schema: 1, EventID: "integration-external",
		ExchangeID: exchange.ID, CanonicalWorkspace: canonical, Artifacts: []IntegrationArtifact{{Source: artifact.Path,
			Target: "result.txt", BaseSHA256: hash([]byte("base attendue")), ResultSHA256: artifact.SHA256}}})
	if commandFailure(err).Code != "conflict" || !created || receipt.State != "conflict" || len(receipt.Conflicts) != 1 {
		t.Fatal(receipt, created, err)
	}
	content, _ := os.ReadFile(target)
	if string(content) != "modification externe" {
		t.Fatalf("le conflit externe a été écrasé : %q", content)
	}
}
