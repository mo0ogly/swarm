//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"strings"
	"testing"
	"time"
)

// Real scripted wrapper and normal memory admission/selection, never a model.
func memoryDialogueFixture(t *testing.T) (*Store, Agent, MemoryReference, string) {
	t.Helper()
	s, configured, script := dialogueRestoreFixture(t, "codex")
	_, _, ref := admittedMemoryFixture(t, s)
	w := taskTest(t, s, createTest(t, s))
	selection := ProjectMemoryRequest{Schema: 1, EventID: newID("memory-"), Revision: w.Revision, Work: w.ID, Task: "t1", Selections: []MemoryReference{ref}, Reason: "Use the attributed memory in this conversation"}
	selection = memoryPreparedRequest(t, s, "select", selection)
	v, err := s.projectMemoryOperation("select", selection)
	if err != nil {
		t.Fatal(err)
	}
	w = v.(Work)
	a, _, err := s.prepare(w.ID, Launch{Schema: 1, EventID: newID("agent-"), Revision: w.Revision, TaskID: "t1", Provider: configured.Provider, Role: "worker", Mode: "dialogue", Workspace: configured.CWD, Capture: true, Limits: &configured.Limits, Timeout: configured.Timeout, Instruction: "Use the selected version and confirm the retained provider boundary"})
	if err != nil {
		t.Fatal(err)
	}
	m, err := runDialogueReceiptWrapper(t, s, a, "/fin\n")
	if err != nil {
		t.Fatal(err)
	}
	a.Progress, a.Usage, _, _ = m.executionSnapshot()
	if err = s.finishAgent(a, "completed", "Owned conversation with selected memory", nil); err != nil {
		t.Fatal(err)
	}
	a, err = s.agent(a.ID)
	if err != nil {
		t.Fatal(err)
	}
	return s, a, ref, script
}

func selectDialogueMemoryTest(t *testing.T, s *Store, a Agent, refs []MemoryReference) Work {
	t.Helper()
	w, err := s.get(a.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	if w.Tasks[0].Status != "todo" {
		w, err = s.executeRequest(w.ID, "task.update", Request{Schema: 1, EventID: newID("task-"), Revision: w.Revision, ID: a.TaskID, Status: "todo", Next: "Review memory selection before the next execution"})
		if err != nil {
			t.Fatal(err)
		}
	}
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("select-"), Revision: w.Revision, Work: w.ID, Task: a.TaskID, Selections: refs, Reason: "Explicitly review the next execution selection"}
	q = memoryPreparedRequest(t, s, "select", q)
	v, err := s.projectMemoryOperation("select", q)
	if err != nil {
		t.Fatal(err)
	}
	return v.(Work)
}

func withdrawDialogueMemoryTest(t *testing.T, s *Store, ref MemoryReference) {
	t.Helper()
	w, err := s.get(ref.Work)
	if err != nil {
		t.Fatal(err)
	}
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("withdraw-"), Revision: w.Revision, Work: w.ID, ID: ref.ID, ExpectedSHA256: ref.SHA256, Reason: "Withdraw this version without erasing previous received contexts"}
	q = memoryPreparedRequest(t, s, "withdraw", q)
	if _, err = s.projectMemoryOperation("withdraw", q); err != nil {
		t.Fatal(err)
	}
}

func TestDialogueRestoreMemoryDeselectionCannotForgetHistory(t *testing.T) {
	s, source, ref, script := memoryDialogueFixture(t)
	original := dialogueSourceSHA(source)
	checkpoint := loadDialogueCheckpointTest(t, s, source).SHA256
	selectDialogueMemoryTest(t, s, source, []MemoryReference{})
	r := dialogueRestoreRequestTest(t, s, source)
	before := recoverySnapshotTest(t, s)
	calls, _ := os.ReadFile(script + ".calls")
	if _, _, err := s.resumeDialogue(source.ID, r, false); err == nil {
		t.Fatal("deselected memory silently reintroduced by the retained provider session")
	}
	if recoverySnapshotTest(t, s) != before {
		t.Fatal("refusal changed work, agents or reservations")
	}
	afterCalls, _ := os.ReadFile(script + ".calls")
	if !bytes.Equal(calls, afterCalls) {
		t.Fatal("refusal invoked provider")
	}
	selectDialogueMemoryTest(t, s, source, []MemoryReference{ref})
	r = dialogueRestoreRequestTest(t, s, source)
	if _, _, err := s.resumeDialogue(source.ID, r, true); err != nil {
		t.Fatal("explicitly restored valid selection still refused", err)
	}
	retained, _ := s.agent(source.ID)
	if dialogueSourceSHA(retained) != original || loadDialogueCheckpointTest(t, s, source).SHA256 != checkpoint {
		t.Fatal("selection rewrote historical source or boundary")
	}
}

// Isolated retained pre-M15 intent: M14 could create a restored execution with
// an empty latest selection even though an ancestor received a version.
func legacyMemoryDialogueIntentTest(t *testing.T, s *Store, source Agent) Agent {
	t.Helper()
	w := selectDialogueMemoryTest(t, s, source, []MemoryReference{})
	a, _, err := s.prepare(w.ID, Launch{Schema: 1, EventID: newID("legacy-"), Revision: w.Revision, TaskID: source.TaskID, Provider: source.Provider, Role: source.Role, Mode: "dialogue", Workspace: source.CWD, Capture: source.Capture, Timeout: source.Timeout, Limits: &source.Limits, Previous: source.ID, Instruction: "Retained historical intent created before inherited memory was checked"})
	if err != nil {
		t.Fatal(err)
	}
	c := loadDialogueCheckpointTest(t, s, source)
	lineage := dialogueRestoreSource{agent: source, checkpoint: &c, turns: 1}
	a.DialogueRestore = lineage.receipt(&DialogueRestoreChoice{Checkpoint: c.SHA256, Facts: hash([]byte("retained pre-M15 reviewed facts")), Confirm: true, Reason: "Historical explicit restoration"}, false)
	if err = s.saveAgent(a); err != nil {
		t.Fatal(err)
	}
	return a
}

func TestProjectMemoryDialogueInheritedUseAndWithdrawal(t *testing.T) {
	s, source, ref, _ := memoryDialogueFixture(t)
	a := legacyMemoryDialogueIntentTest(t, s, source)
	before := recoverySnapshotTest(t, s)
	preview, err := s.projectMemoryCatalog("")
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(preview)
	var view struct {
		Entries []struct {
			Ref  MemoryReference `json:"ref"`
			Uses []MemoryUse     `json:"used_by"`
		} `json:"entries"`
	}
	if err = json.Unmarshal(raw, &view); err != nil {
		t.Fatal(err)
	}
	found := false
	for _, entry := range view.Entries {
		for _, use := range entry.Uses {
			if entry.Ref == ref && use.Agent == a.ID && use.Reference == ref {
				found = true
			}
		}
	}
	if !found {
		t.Fatal("catalogue hides inherited memory used by an active restored execution")
	}
	if recoverySnapshotTest(t, s) != before {
		t.Fatal("catalogue mutated state")
	}
	withdrawDialogueMemoryTest(t, s, ref)
	if err = s.projectMemoryGuard(a); err == nil {
		t.Fatal("withdrawn ancestor memory bypasses startup with an empty current manifest")
	}
	retained, _ := s.agent(source.ID)
	if dialogueSourceSHA(retained) != dialogueSourceSHA(source) {
		t.Fatal("withdrawal rewrote source context")
	}
}

func TestProjectMemoryDialogueValidChainAndStartupWithdrawal(t *testing.T) {
	s, source, ref, script := memoryDialogueFixture(t)
	r := dialogueRestoreRequestTest(t, s, source)
	a, _, err := s.resumeDialogue(source.ID, r, false)
	if err != nil {
		t.Fatal(err)
	}
	uses, err := memoryUses(s.db)
	if err != nil || len(uses) != 1 || uses[0].Agent != a.ID || uses[0].Reference != ref {
		t.Fatal("shared ancestor reference duplicated or lost", uses, err)
	}
	if err = s.projectMemoryGuard(a); err != nil {
		t.Fatal("unchanged selected lineage refused", err)
	}
	w, _ := s.get(ref.Work)
	q := ProjectMemoryRequest{Schema: 1, EventID: newID("withdraw-"), Revision: w.Revision, Work: w.ID, ID: ref.ID, ExpectedSHA256: ref.SHA256, Reason: "Inspect the active use before withdrawal"}
	v, err := s.projectMemoryOperation("withdraw-preview", q)
	if err != nil {
		t.Fatal(err)
	}
	affected := v.(map[string]any)["affected_attempts"].([]MemoryUse)
	if len(affected) != 1 || affected[0].Agent != a.ID {
		t.Fatal("withdrawal preview omitted active restored use", affected)
	}
	calls, _ := os.ReadFile(script + ".calls")
	withdrawDialogueMemoryTest(t, s, ref)
	before := recoverySnapshotTest(t, s)
	if err = s.dialogueRestoreStartGuard(a); err == nil {
		t.Fatal("withdrawal after reservation bypassed startup")
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("startup guard mutated the retained state")
	}
	afterCalls, _ := os.ReadFile(script + ".calls")
	if !bytes.Equal(calls, afterCalls) {
		t.Fatal("startup refusal invoked provider")
	}
}

func TestProjectMemoryDialogueRejectsBrokenLineage(t *testing.T) {
	for _, kind := range []string{"missing_source", "foreign_source", "changed_source", "cycle", "missing_context", "duplicate_reference"} {
		t.Run(kind, func(t *testing.T) {
			s, source, ref, script := memoryDialogueFixture(t)
			a := legacyMemoryDialogueIntentTest(t, s, source)
			switch kind {
			case "missing_source":
				a.Previous, a.DialogueRestore.SourceAgent = "absent", "absent"
			case "foreign_source":
				a.WorkID = ref.Work
			case "changed_source":
				a.DialogueRestore.SourceState = strings.Repeat("0", 64)
			case "cycle":
				a.Previous, a.DialogueRestore.SourceAgent = a.ID, a.ID
			case "missing_context":
				source.Context = nil
				if err := s.saveAgent(source); err != nil {
					t.Fatal(err)
				}
				a.DialogueRestore.SourceState = dialogueSourceSHA(source)
			case "duplicate_reference":
				a.Context.Memory = []MemoryReference{ref, ref}
			}
			if err := s.saveAgent(a); err != nil {
				t.Fatal(err)
			}
			before := recoverySnapshotTest(t, s)
			calls, _ := os.ReadFile(script + ".calls")
			if err := s.projectMemoryGuard(a); err == nil {
				t.Fatal("invalid lineage became a successful empty memory guard")
			}
			if _, err := s.projectMemoryCatalog(""); err == nil {
				t.Fatal("invalid lineage became a successful catalogue")
			}
			if before != recoverySnapshotTest(t, s) {
				t.Fatal("invalid lineage read mutated state")
			}
			afterCalls, _ := os.ReadFile(script + ".calls")
			if !bytes.Equal(calls, afterCalls) {
				t.Fatal("invalid lineage invoked provider")
			}
		})
	}
}

func TestDialogueRestoreMemoryRechecksAncestorAfterPreflight(t *testing.T) {
	s, source, _, script := memoryDialogueFixture(t)
	middle, _, err := s.resumeDialogue(source.ID, dialogueRestoreRequestTest(t, s, source), false)
	if err != nil {
		t.Fatal(err)
	}
	m, err := runDialogueReceiptWrapper(t, s, middle, "/fin\n")
	if err != nil {
		t.Fatal(err)
	}
	middle.Progress, middle.Usage, _, _ = m.executionSnapshot()
	if err = s.finishAgent(middle, "completed", "Owned intermediate restored conversation", nil); err != nil {
		t.Fatal(err)
	}
	middle, err = s.agent(middle.ID)
	if err != nil {
		t.Fatal(err)
	}
	r := dialogueRestoreRequestTest(t, s, middle)
	if err = os.WriteFile(script+".block", []byte("wait"), 0600); err != nil {
		t.Fatal(err)
	}
	t.Cleanup(func() { _ = os.WriteFile(script+".release", []byte("release"), 0600) })
	done := make(chan error, 1)
	go func() { _, _, err := s.resumeDialogue(middle.ID, r, false); done <- err }()
	until := time.Now().Add(3 * time.Second)
	for {
		if _, err = os.Stat(script + ".ready"); err == nil {
			break
		}
		if time.Now().After(until) {
			t.Fatal("owned preflight did not reach ancestor barrier")
		}
		time.Sleep(10 * time.Millisecond)
	}
	source.Activity = "Original ancestor changed during the owned probe"
	if err = s.saveAgent(source); err != nil {
		t.Fatal(err)
	}
	before := recoverySnapshotTest(t, s)
	calls, _ := os.ReadFile(script + ".calls")
	if err = os.WriteFile(script+".release", []byte("release"), 0600); err != nil {
		t.Fatal(err)
	}
	select {
	case err = <-done:
		if err == nil {
			t.Fatal("changed ancestor accepted after preflight")
		}
	case <-time.After(6 * time.Second):
		t.Fatal("owned probe did not release launch")
	}
	if before != recoverySnapshotTest(t, s) {
		t.Fatal("ancestor freshness refusal wrote a launch")
	}
	afterCalls, _ := os.ReadFile(script + ".calls")
	if !bytes.Equal(calls, afterCalls) {
		t.Fatal("changed ancestor invoked provider")
	}
}
