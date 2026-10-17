//go:build linux

package engine

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

// Pure fixture: no Store, enrolled human, attested process or adoption is claimed.
func memoryUsageFixture(t *testing.T) (string, Work, Launch, memoryUsageTarget) {
	t.Helper()
	root := t.TempDir()
	w := Work{Schema: 1, ID: "work-memory", Revision: 7, Title: "Memory scope", Objective: "Exact context", Scope: "fixture", Criteria: []string{"Current evidence"}, Tasks: []Task{
		{ID: "target", Title: "Receiver task", Status: "todo", Criteria: []string{"Observe context"}},
		{ID: "other", Title: "Another task", Status: "todo"},
		{ID: "dialogue-0", Brainstorm: true, Question: "Question zéro", Answers: []BrainstormAnswer{{Attempt: "agent-0", Text: "Réponse zéro"}}},
		{ID: "dialogue-1", Brainstorm: true, Question: "Question un", Answers: []BrainstormAnswer{{Attempt: "agent-1", Text: "Réponse un"}}},
	}}
	r := Launch{TaskID: "target", Role: "worker", Summary: &MemorySummarySelection{Schema: 1, Cutoff: 4, Reason: "Explicit memory choice"}}
	for _, hit := range dialogueHits(w) {
		r.Summary.Dialogues = append(r.Summary.Dialogues, hit.Ref)
	}
	refreshMemoryUsageSelection(t, root, w, &r)
	target := memoryUsageTarget{Scope: agentIdentityScope{WorkID: w.ID, PlanVersion: 2, PlanDigest: hash([]byte("fixture protected scope"))}, Task: r.TaskID,
		Recipient: agentIdentityActor{Principal: agentIdentityPrincipal{ID: "receiver", Kind: "service", Issuer: "urn:fixture:issuer", Subject: "receiver-subject"}, InstanceID: "receiver-instance"}}
	return root, w, r, target
}

func refreshMemoryUsageSelection(t *testing.T, root string, w Work, r *Launch) {
	t.Helper()
	packet, sources, err := summarySources(root, w, r.TaskID, *r.Summary)
	if err != nil {
		t.Fatal(err)
	}
	r.Summary.SourceSHA = packet.SHA256
	r.Summary.Protected = []MemorySummaryProtection{
		{ID: "current", Message: sources[0].ID, Kind: "evidence", State: "historical"},
		{ID: "intent", Message: sources[1].ID, Kind: "intention", State: "future"},
		{ID: "evidence", Message: sources[2].ID, Kind: "evidence", State: "historical"},
	}
}

func memoryUsageClone(t *testing.T, w Work, r Launch) (Work, Launch) {
	t.Helper()
	raw, err := json.Marshal(struct {
		W Work
		R Launch
	}{w, r})
	if err != nil {
		t.Fatal(err)
	}
	var copy struct {
		W Work
		R Launch
	}
	if err = json.Unmarshal(raw, &copy); err != nil {
		t.Fatal(err)
	}
	return copy.W, copy.R
}

func TestMemoryUsageBindingExactSourcesAndNoOperatorTransmission(t *testing.T) {
	root, w, r, target := memoryUsageFixture(t)
	before, err := json.Marshal(struct {
		W Work
		R Launch
	}{w, r})
	if err != nil {
		t.Fatal(err)
	}
	b, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	requests, err := b.requests("usage-event", "reader-grant")
	if err != nil {
		t.Fatal(err)
	}
	if len(requests) != 13 {
		t.Fatalf("want 5 reads + 5 summaries + 3 transmissions, got %d", len(requests))
	}
	if len(b.projection.Sources) != 5 || b.projection.Sources[0].Transmit {
		t.Fatal("operator snapshot became transmissible")
	}
	// q1 is omitted, a1 remains recent. Neither omission nor task snapshot is transmitted.
	if b.projection.Sources[3].Transmit || !b.projection.Sources[4].Transmit {
		t.Fatal("omission/received mapping lost")
	}
	ids := map[string]bool{}
	rights := map[agentIdentityPermission]bool{}
	for _, q := range requests {
		if !q.valid() || q.ContentDigest != b.digest || ids[q.EventID] {
			t.Fatal("invalid, detached or duplicate request")
		}
		ids[q.EventID] = true
		p := agentIdentityPermission{q.Operation, q.Resource}
		if rights[p] {
			t.Fatal("duplicated permission")
		}
		rights[p] = true
	}
	for i, source := range b.projection.Sources {
		if !strings.HasPrefix(source.Resource, "memory:v2:") || strings.ContainsAny(source.Resource, "*?[]") {
			t.Fatal("non-exact source")
		}
		if !rights[agentIdentityPermission{"memory.read", source.Resource}] || !rights[agentIdentityPermission{"memory.summarize", source.Resource}] {
			t.Fatalf("source %d lacks independent read/summary requirements", i)
		}
		if rights[agentIdentityPermission{"memory.transmit", source.Resource}] != source.Transmit {
			t.Fatal("transmission is not exact")
		}
	}
	body, receipt, err := memorySummaryAttachment(root, w, r)
	if err != nil {
		t.Fatal(err)
	}
	if b.projection.AttachmentSHA != hash([]byte(body)) || b.projection.SelectionSHA != receipt.SelectionSHA || b.projection.AttachmentBytes != len(body) {
		t.Fatal("binding is detached from real M19 bytes")
	}
	again, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	repeated, err := again.requests("usage-event", "reader-grant")
	if err != nil || !reflect.DeepEqual(requests, repeated) {
		t.Fatal("same resolved inputs changed requests", err)
	}
	after, _ := json.Marshal(struct {
		W Work
		R Launch
	}{w, r})
	if string(before) != string(after) {
		t.Fatal("binding mutated Work or selection")
	}
}

func TestMemoryUsageBindingChangesWhenPlanProjectionDoesNot(t *testing.T) {
	root, w, r, target := memoryUsageFixture(t)
	before, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(w)
	planBefore, err := agentIdentityPlanProjectionDigest(raw)
	if err != nil {
		t.Fatal(err)
	}
	w.Tasks[2].Answers[0].Text = "Changed answer under the same plan"
	raw, _ = json.Marshal(w)
	planAfter, err := agentIdentityPlanProjectionDigest(raw)
	if err != nil {
		t.Fatal(err)
	}
	if planBefore != planAfter {
		t.Fatal("fixture did not reproduce the content/plan separation")
	}
	if _, err = memorySummaryUsageBinding(root, w, r, target); err == nil {
		t.Fatal("stale selected source accepted")
	}
	r.Summary.Dialogues[0] = dialogueHits(w)[0].Ref
	refreshMemoryUsageSelection(t, root, w, &r)
	after, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	if before.digest == after.digest {
		t.Fatal("changed source reused the same memory proposal")
	}
	requests, err := before.requests("usage-event", "reader-grant")
	if err != nil {
		t.Fatal(err)
	}
	snapshot, authority, caller, at := memoryUsageAuthority(t, root, before, requests)
	decision, err := evaluateAgentIdentity(snapshot, authority, caller, requests[0], at)
	if err != nil {
		t.Fatal(err)
	}
	changedRequests, err := after.requests("usage-event", "reader-grant")
	if err != nil {
		t.Fatal(err)
	}
	_, err = reuseAgentIdentityDecision(decision, snapshot, authority, caller, changedRequests[0], at)
	var code *CommandError
	if !errors.As(err, &code) || code.Code != "agent_identity_decision_stale" {
		t.Fatalf("wrong changed-content refusal: %v", err)
	}
}

func TestMemoryUsageBindingTracksDestinationAndSelection(t *testing.T) {
	root, base, launch, target := memoryUsageFixture(t)
	old, err := memorySummaryUsageBinding(root, base, launch, target)
	if err != nil {
		t.Fatal(err)
	}
	for name, change := range map[string]func(*Work, *Launch, *memoryUsageTarget, *string){
		"recipient-principal": func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) {
			d.Recipient.Principal.Subject = "another-subject"
		},
		"recipient-instance": func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) {
			d.Recipient.InstanceID = "replacement-instance"
		},
		"plan-version": func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Scope.PlanVersion++ },
		"plan-digest": func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) {
			d.Scope.PlanDigest = hash([]byte("other scope"))
		},
		"target-task":     func(_ *Work, r *Launch, d *memoryUsageTarget, _ *string) { r.TaskID = "other"; d.Task = "other" },
		"task-definition": func(w *Work, _ *Launch, _ *memoryUsageTarget, _ *string) { w.Tasks[0].Title = "New task contract" },
		"project-root": func(_ *Work, _ *Launch, _ *memoryUsageTarget, root *string) {
			*root = filepath.Join(*root, "another-project")
		},
		"annotation": func(_ *Work, r *Launch, _ *memoryUsageTarget, _ *string) {
			r.Summary.Reason = "Different exact selection"
		},
		"revision": func(w *Work, _ *Launch, _ *memoryUsageTarget, _ *string) { w.Revision++ },
	} {
		t.Run(name, func(t *testing.T) {
			w, r := memoryUsageClone(t, base, launch)
			d, project := target, root
			change(&w, &r, &d, &project)
			// Re-export only the packet hash: preserve the explicit changed annotations.
			packet, _, err := summarySources(project, w, r.TaskID, *r.Summary)
			if err != nil {
				t.Fatal(err)
			}
			r.Summary.SourceSHA = packet.SHA256
			got, err := memorySummaryUsageBinding(project, w, r, d)
			if err != nil {
				t.Fatal(err)
			}
			if got.digest == old.digest {
				t.Fatal("changed contract retained old binding")
			}
		})
	}
}

func TestMemoryUsageBindingRejectsInvalidWithoutPartialResult(t *testing.T) {
	root, base, launch, target := memoryUsageFixture(t)
	cases := map[string]func(*Work, *Launch, *memoryUsageTarget, *string){
		"no-summary":    func(_ *Work, r *Launch, _ *memoryUsageTarget, _ *string) { r.Summary = nil },
		"no-recipient":  func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Recipient = agentIdentityActor{} },
		"no-instance":   func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Recipient.InstanceID = "" },
		"no-scope":      func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Scope = agentIdentityScope{} },
		"foreign-work":  func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Scope.WorkID = "another-work" },
		"wrong-task":    func(_ *Work, _ *Launch, d *memoryUsageTarget, _ *string) { d.Task = "other" },
		"relative-root": func(_ *Work, _ *Launch, _ *memoryUsageTarget, root *string) { *root = "relative" },
		"invalid-work":  func(w *Work, _ *Launch, _ *memoryUsageTarget, _ *string) { w.Schema = 2 },
		"zero-revision": func(w *Work, _ *Launch, _ *memoryUsageTarget, _ *string) { w.Revision = 0 },
		"invalid-utf8": func(w *Work, _ *Launch, _ *memoryUsageTarget, _ *string) {
			w.Tasks[2].Answers[0].Text = string([]byte{0xff})
		},
		"stale-packet": func(_ *Work, r *Launch, _ *memoryUsageTarget, _ *string) {
			r.Summary.SourceSHA = hash([]byte("old packet"))
		},
		"duplicate-source": func(_ *Work, r *Launch, _ *memoryUsageTarget, _ *string) {
			r.Summary.Dialogues = append(r.Summary.Dialogues, r.Summary.Dialogues[0])
		},
		"missing-anchor": func(_ *Work, r *Launch, _ *memoryUsageTarget, _ *string) { r.Summary.Protected = nil },
	}
	for name, change := range cases {
		t.Run(name, func(t *testing.T) {
			w, r := memoryUsageClone(t, base, launch)
			d, project := target, root
			change(&w, &r, &d, &project)
			b, err := memorySummaryUsageBinding(project, w, r, d)
			if err == nil || !reflect.DeepEqual(b, memoryUsageBinding{}) {
				t.Fatal("invalid input produced a complete/partial binding", err)
			}
		})
	}
	b, err := memorySummaryUsageBinding(root, base, launch, target)
	if err != nil {
		t.Fatal(err)
	}
	for _, ids := range [][2]string{{"", "grant"}, {"event", ""}, {"event/*", "grant"}} {
		if requests, err := b.requests(ids[0], ids[1]); err == nil || len(requests) != 0 {
			t.Fatal("invalid IDs produced requests")
		}
	}
	b.projection.AttachmentSHA = hash([]byte("substituted bytes"))
	if requests, err := b.requests("event", "grant"); err == nil || len(requests) != 0 {
		t.Fatal("edited proposal produced requests")
	}
	var empty memoryUsageBinding
	if requests, err := empty.requests("event", "grant"); err == nil || len(requests) != 0 {
		t.Fatal("zero binding became success")
	}
}

// This is only a pure-kernel fixture. It does not enroll or attest the recipient.
func memoryUsageAuthority(t *testing.T, root string, b memoryUsageBinding, requests []agentIdentityRequest) (agentIdentitySnapshot, agentIdentityAuthority, agentIdentityCaller, time.Time) {
	t.Helper()
	at := time.Date(2026, 10, 17, 0, 0, 0, 0, time.UTC)
	actor := agentIdentityActor{Principal: agentIdentityPrincipal{ID: "requester", Kind: "service", Issuer: "urn:fixture:issuer", Subject: "requester"}}
	rights := make([]agentIdentityPermission, 0, len(requests))
	for _, q := range requests {
		rights = append(rights, agentIdentityPermission{q.Operation, q.Resource})
	}
	grant := agentIdentityGrant{ID: "reader-grant", Issuer: actor, Holder: actor, Scope: b.projection.Target.Scope, TrustRevision: 3, Permissions: rights, NotBefore: at.Add(-time.Minute), ExpiresAt: at.Add(time.Minute)}
	snapshot := agentIdentitySnapshot{Schema: 1, Revision: 1, Grants: map[string]agentIdentityGrant{grant.ID: grant}, Instances: map[string]agentIdentityInstance{}}
	digest, err := agentIdentityDigest(snapshot)
	if err != nil {
		t.Fatal(err)
	}
	authority := agentIdentityAuthority{projectRoot: root, snapshotDigest: digest, snapshotRevision: 1, trustRevision: 3, scope: grant.Scope, roots: map[string]bool{grant.ID: true}}
	return snapshot, authority, agentIdentityCaller{projectRoot: root, actor: actor, trustRevision: 3, verifiedAt: at}, at
}

func TestMemoryUsageRequestsUseIndependentKernelRightsAndFreshness(t *testing.T) {
	root, w, r, target := memoryUsageFixture(t)
	b, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	requests, err := b.requests("usage-event", "reader-grant")
	if err != nil {
		t.Fatal(err)
	}
	snapshot, authority, caller, at := memoryUsageAuthority(t, root, b, requests)
	for _, q := range requests {
		if _, err := evaluateAgentIdentity(snapshot, authority, caller, q, at); err != nil {
			t.Fatal(err)
		}
	}
	for _, missing := range []string{"memory.read", "memory.summarize", "memory.transmit"} {
		t.Run(missing, func(t *testing.T) {
			local := snapshot
			local.Grants = map[string]agentIdentityGrant{}
			g := snapshot.Grants["reader-grant"]
			g.Permissions = nil
			for _, p := range snapshot.Grants[g.ID].Permissions {
				if p.Operation != missing {
					g.Permissions = append(g.Permissions, p)
				}
			}
			local.Grants[g.ID] = g
			frame := authority
			frame.snapshotDigest, _ = agentIdentityDigest(local)
			denied := 0
			for _, q := range requests {
				_, err := evaluateAgentIdentity(local, frame, caller, q, at)
				if q.Operation == missing {
					var ce *CommandError
					if !errors.As(err, &ce) || ce.Code != "agent_identity_permission_denied" {
						t.Fatalf("wrong missing-right refusal: %v", err)
					}
					denied++
				} else if err != nil {
					t.Fatal(err)
				}
			}
			if denied == 0 {
				t.Fatal("no independent refusal tested")
			}
		})
	}
	original, err := evaluateAgentIdentity(snapshot, authority, caller, requests[0], at)
	if err != nil {
		t.Fatal(err)
	}
	target.Recipient.InstanceID = "replacement-instance"
	changed, err := memorySummaryUsageBinding(root, w, r, target)
	if err != nil {
		t.Fatal(err)
	}
	newer, err := changed.requests("usage-event", "reader-grant")
	if err != nil {
		t.Fatal(err)
	}
	if _, err = reuseAgentIdentityDecision(original, snapshot, authority, caller, newer[0], at); err != nil {
		var code *CommandError
		if !errors.As(err, &code) || code.Code != "agent_identity_decision_stale" {
			t.Fatalf("wrong changed-receiver refusal: %v", err)
		}
	} else {
		t.Fatal("decision reused for a different receiver")
	}
	for _, state := range []string{"expired", "revoked"} {
		t.Run(state, func(t *testing.T) {
			local := snapshot
			local.Grants = map[string]agentIdentityGrant{}
			g := snapshot.Grants["reader-grant"]
			if state == "expired" {
				g.ExpiresAt = at
			} else {
				g.RevokedAt = &at
			}
			local.Grants[g.ID] = g
			frame := authority
			frame.snapshotDigest, _ = agentIdentityDigest(local)
			if _, err := reuseAgentIdentityDecision(original, local, frame, caller, requests[0], at); err != nil {
				var code *CommandError
				if !errors.As(err, &code) || code.Code != "agent_identity_grant_inactive" {
					t.Fatalf("wrong %s refusal: %v", state, err)
				}
			} else {
				t.Fatal(fmt.Sprint(state, " grant reused"))
			}
		})
	}
}
