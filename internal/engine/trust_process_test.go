//go:build linux

package engine

import (
	"bytes"
	"encoding/json"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"
)

func TestActionsConfianceT5PublicNamespace(t *testing.T) {
	s, _ := trustFixture(t)
	t.Setenv("SWARM_TRUST_TOKEN", trustAdminToken)
	var out, errOut bytes.Buffer
	code := run([]string{"--root", s.root, "--json", "process", "show"}, &out, &errOut)
	if code != 0 {
		t.Fatalf("process namespace unavailable: code %d", code)
	}
	var view struct {
		Schema      int            `json:"schema_version"`
		Definitions map[string]any `json:"definitions"`
		Cases       []any          `json:"cases"`
	}
	if json.Unmarshal(out.Bytes(), &view) != nil || view.Schema != 1 || view.Definitions == nil || view.Cases == nil {
		t.Fatal("process view contract missing")
	}
}

func processCommand(t *testing.T, s *Store, op string, body map[string]any, token string) (any, error) {
	t.Helper()
	raw, err := json.Marshal(body)
	if err != nil {
		t.Fatal(err)
	}
	return s.trustProcessOperation(op, raw, token)
}
func processTestDocument(id string) map[string]any {
	label := func(fr, en string) map[string]string { return map[string]string{"fr": fr, "en": en} }
	return map[string]any{"schema_version": 1, "id": id, "label": label("Contrôle de facture", "Invoice review"), "start": "received", "nodes": []any{map[string]any{"id": "received", "label": label("Reçu", "Received"), "terminal": false}, map[string]any{"id": "review", "label": label("À valider", "To approve"), "terminal": false}, map[string]any{"id": "closed", "label": label("Validé", "Approved"), "terminal": true}}, "transitions": []any{map[string]any{"id": "submit", "from": "received", "to": "review", "label": label("Soumettre", "Submit"), "kind": "advance", "roles": []string{"requester"}, "minimum_integrity": 1}, map[string]any{"id": "approve", "from": "review", "to": "closed", "label": label("Valider", "Approve"), "kind": "approval", "roles": []string{"approver"}, "minimum_integrity": 2}, map[string]any{"id": "reopen", "from": "closed", "to": "review", "label": label("Rouvrir par exception", "Reopen by exception"), "kind": "exception", "roles": []string{"admin"}, "minimum_integrity": 3}}}
}
func processConfigured(t *testing.T) (*Store, map[string]any) {
	t.Helper()
	s, _ := trustFixture(t)
	change := lifecycleChange(t, "alice", false, 1)
	change.ProcessPolicy = &TrustProcessPolicy{Enabled: true}
	change.DataPolicy = &TrustDataPolicy{Enabled: true, Classifications: []string{"public", "internal"}, Destinations: []string{"local-process-state"}, Providers: []string{"swarm"}, Tools: []string{"process-workflow"}, Resources: []TrustDataResource{{Resource: "invoice/acme", Classification: "internal"}}}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	v, err := processCommand(t, s, "definition-publish", map[string]any{"schema_version": 1, "event_id": "process-definition-1", "definition": processTestDocument("invoice"), "expected_version": 0, "reason": "publish isolated invoice review"}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	return s, v.(map[string]any)
}
func createProcessCase(t *testing.T, s *Store, def map[string]any, event string) TrustProcessCase {
	t.Helper()
	v, err := processCommand(t, s, "case-create", map[string]any{"schema_version": 1, "event_id": event, "definition_id": "invoice", "definition_version": def["version"], "definition_digest": def["content_digest"], "participants": []string{"bob", "effect-service"}, "resource": "invoice/acme", "data_context": map[string]string{"classification": "internal", "destination": "local-process-state", "provider": "swarm", "tool": "process-workflow"}, "title": "Invoice ACME", "reference": "ACME-001"}, trustRequesterToken)
	if err != nil {
		t.Fatal(err)
	}
	return v.(TrustProcessCase)
}
func TestActionsConfianceT5VersionPinningAndApproval(t *testing.T) {
	s, def := processConfigured(t)
	c := createProcessCase(t, s, def, "case-first")
	if _, err := processCommand(t, s, "definition-publish", map[string]any{"schema_version": 1, "event_id": "process-definition-2", "definition": processTestDocument("invoice"), "expected_version": 1, "reason": "publish next isolated version"}, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	body := map[string]any{"schema_version": 1, "event_id": "case-submit", "case_id": c.ID, "expected_revision": 1, "definition_digest": c.DefinitionDigest, "input_digest": c.InputDigest, "transition_id": "submit"}
	if _, err := processCommand(t, s, "case-transition", body, trustRequesterToken); err != nil {
		t.Fatal(err)
	}
	body["event_id"] = "case-approve"
	body["expected_revision"] = 2
	body["transition_id"] = "approve"
	body["reason"] = "reviewed exact invoice content"
	if _, err := processCommand(t, s, "case-transition", body, trustRequesterToken); err == nil {
		t.Fatal("requester approved own case")
	}
	if _, err := processCommand(t, s, "case-transition", body, trustApproverToken); err != nil {
		t.Fatal(err)
	}
	v, err := processCommand(t, s, "case-show", map[string]any{"case_id": c.ID}, trustRequesterToken)
	if err != nil {
		t.Fatal(err)
	}
	after := v.(TrustProcessCase)
	if after.DefinitionVersion != 1 || after.Node != "closed" || after.Revision != 3 || len(after.History) != 2 {
		t.Fatal("case moved definition or failed to progress")
	}
}

func TestActionsConfianceT5ReplayRechecksDataAdmission(t *testing.T) {
	s, def := processConfigured(t)
	createProcessCase(t, s, def, "case-first")
	change := lifecycleChange(t, "alice", false, 2)
	change.DataPolicy = &TrustDataPolicy{Enabled: false, Classifications: []string{"internal"}, Destinations: []string{"local-process-state"}, Providers: []string{"swarm"}, Tools: []string{"process-workflow"}, Resources: []TrustDataResource{{Resource: "invoice/acme", Classification: "internal"}}}
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	raw := map[string]any{"schema_version": 1, "event_id": "case-first", "definition_id": "invoice", "definition_version": def["version"], "definition_digest": def["content_digest"], "participants": []string{"bob", "effect-service"}, "resource": "invoice/acme", "data_context": map[string]string{"classification": "internal", "destination": "local-process-state", "provider": "swarm", "tool": "process-workflow"}, "title": "Invoice ACME", "reference": "ACME-001"}
	if _, err := processCommand(t, s, "case-create", raw, trustRequesterToken); err == nil {
		t.Fatal("historical create replay returned case data after policy withdrawal")
	}
}

func TestActionsConfianceT5InvalidDefinitionsNoMutation(t *testing.T) {
	cases := map[string]func(map[string]any){"cycle": func(d map[string]any) { d["transitions"].([]any)[0].(map[string]any)["to"] = "received" }, "missing-node": func(d map[string]any) { d["transitions"].([]any)[0].(map[string]any)["to"] = "absent" }, "no-start": func(d map[string]any) { d["start"] = "absent" }, "no-terminal": func(d map[string]any) { d["nodes"].([]any)[2].(map[string]any)["terminal"] = false }, "empty-roles": func(d map[string]any) { d["transitions"].([]any)[0].(map[string]any)["roles"] = []string{} }, "weak-approval": func(d map[string]any) { d["transitions"].([]any)[1].(map[string]any)["minimum_integrity"] = 1 }, "weak-exception": func(d map[string]any) { d["transitions"].([]any)[2].(map[string]any)["roles"] = []string{"requester"} }, "null-terminal": func(d map[string]any) { d["nodes"].([]any)[2].(map[string]any)["terminal"] = nil }, "script": func(d map[string]any) { d["command"] = "outside-command" }}
	for name, mutate := range cases {
		t.Run(name, func(t *testing.T) {
			s, _ := processConfigured(t)
			path, _, _ := s.trustPaths()
			before, _ := os.ReadFile(path)
			d := processTestDocument("invalid")
			mutate(d)
			if _, err := processCommand(t, s, "definition-publish", map[string]any{"schema_version": 1, "event_id": "bad-definition", "definition": d, "expected_version": 0, "reason": "invalid definition must be denied"}, trustAdminToken); err == nil {
				t.Fatal("invalid definition published")
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) {
				t.Fatal("invalid definition changed state")
			}
		})
	}
}
func TestActionsConfianceT5PolicyPublicPreviewPersistence(t *testing.T) {
	s, _ := trustFixture(t)
	body := map[string]any{"schema_version": 1, "event_id": "process-policy", "expected_revision": 1, "policy": map[string]bool{"enabled": true}, "reason": "activate isolated process policy"}
	path, _, _ := s.trustPaths()
	before, _ := os.ReadFile(path)
	if _, err := processCommand(t, s, "policy-preview", body, trustRequesterToken); err == nil {
		t.Fatal("requester previewed administrative policy")
	}
	if _, err := processCommand(t, s, "policy-preview", body, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated state")
	}
	if _, err := processCommand(t, s, "policy-apply", body, trustAdminToken); err != nil {
		t.Fatal(err)
	}
	v, err := processCommand(t, s, "show", map[string]any{}, trustAdminToken)
	if err != nil {
		t.Fatal(err)
	}
	if !bool(v.(map[string]any)["policy"].(TrustProcessPolicy).Enabled) {
		t.Fatal("effective policy not saved")
	}
	st, err := s.loadTrustState()
	if err != nil || st.Revision != 2 || st.ProcessPolicy == nil || !bool(st.ProcessPolicy.Enabled) {
		t.Fatal("policy persistence missing", err)
	}
}

func TestActionsConfianceT5TransitionGuardsAndRestart(t *testing.T) {
	s, def := processConfigured(t)
	c := createProcessCase(t, s, def, "guard-case")
	path, _, _ := s.trustPaths()
	for _, variant := range []string{"digest", "revision", "input", "role", "exception", "actor"} {
		t.Run(variant, func(t *testing.T) {
			body := map[string]any{"schema_version": 1, "event_id": "guard-" + variant, "case_id": c.ID, "expected_revision": 1, "definition_digest": c.DefinitionDigest, "input_digest": c.InputDigest, "transition_id": "submit"}
			token := trustRequesterToken
			switch variant {
			case "digest":
				body["definition_digest"] = "wrong"
			case "revision":
				body["expected_revision"] = 2
			case "input":
				body["input_digest"] = "wrong"
			case "role":
				token = trustApproverToken
			case "exception":
				body["transition_id"] = "reopen"
			case "actor":
				body["actor"] = "bob"
			}
			before, _ := os.ReadFile(path)
			if _, err := processCommand(t, s, "case-transition", body, token); err == nil {
				t.Fatal("invalid transition accepted")
			}
			after, _ := os.ReadFile(path)
			if !bytes.Equal(before, after) {
				t.Fatal("rejection changed state or captured original")
			}
		})
	}
	body := map[string]any{"schema_version": 1, "event_id": "exact-submit", "case_id": c.ID, "expected_revision": 1, "definition_digest": c.DefinitionDigest, "input_digest": c.InputDigest, "transition_id": "submit"}
	if _, err := processCommand(t, s, "case-transition", body, trustRequesterToken); err != nil {
		t.Fatal(err)
	}
	before, _ := os.ReadFile(path)
	if _, err := processCommand(t, s, "case-transition", body, trustRequesterToken); err != nil {
		t.Fatal("exact replay failed", err)
	}
	after, _ := os.ReadFile(path)
	if !bytes.Equal(before, after) {
		t.Fatal("replay mutated state")
	}
	body["reason"] = "changed contents"
	if _, err := processCommand(t, s, "case-transition", body, trustRequesterToken); err == nil {
		t.Fatal("event reuse changed contents")
	}
	reloaded := &Store{root: s.root}
	v, err := processCommand(t, reloaded, "case-show", map[string]any{"case_id": c.ID}, trustApproverToken)
	if err != nil || v.(TrustProcessCase).Node != "review" {
		t.Fatal("restart lost progress", err)
	}
	body["event_id"] = "approve-guard-case"
	body["expected_revision"] = 2
	body["transition_id"] = "approve"
	body["reason"] = "independent exact content review"
	if _, err := processCommand(t, s, "case-transition", body, trustApproverToken); err != nil {
		t.Fatal(err)
	}
	body["event_id"] = "reopen-guard-case"
	body["expected_revision"] = 3
	body["transition_id"] = "reopen"
	body["reason"] = "authorized documented exception"
	if _, err := processCommand(t, s, "case-transition", body, trustApproverToken); err == nil {
		t.Fatal("non admin reopened case")
	}
	if _, err := processCommand(t, s, "case-transition", body, trustAdminToken); err != nil {
		t.Fatal(err)
	}
}
func TestActionsConfianceT5ConcurrencyAndPrivacy(t *testing.T) {
	s, def := processConfigured(t)
	c := createProcessCase(t, s, def, "race-case")
	ready := make(chan struct{})
	results := make(chan error, 2)
	for _, event := range []string{"race-a", "race-b"} {
		go func(event string) {
			<-ready
			_, err := s.trustProcessOperation("case-transition", processTestJSON(map[string]any{"schema_version": 1, "event_id": event, "case_id": c.ID, "expected_revision": 1, "definition_digest": c.DefinitionDigest, "input_digest": c.InputDigest, "transition_id": "submit"}), trustRequesterToken)
			results <- err
		}(event)
	}
	close(ready)
	wins := 0
	for i := 0; i < 2; i++ {
		if <-results == nil {
			wins++
		}
	}
	if wins != 1 {
		t.Fatalf("concurrent winners %d", wins)
	}
	change := lifecycleChange(t, "alice", false, 2)
	change.Identities = append(change.Identities, TrustIdentityInput{Subject: "outsider", Token: "outsider-token-0123456789", Integrity: 2, Roles: []string{"approver"}})
	if _, err := s.trustConfig(change, false); err != nil {
		t.Fatal(err)
	}
	if _, err := processCommand(t, s, "case-show", map[string]any{"case_id": c.ID}, "outsider-token-0123456789"); err == nil {
		t.Fatal("non participant read case")
	}
	v, err := processCommand(t, s, "show", map[string]any{}, "outsider-token-0123456789")
	if err != nil {
		t.Fatal(err)
	}
	if len(v.(map[string]any)["cases"].([]any)) != 0 {
		t.Fatal("non participant inbox leaked case")
	}
}
func TestActionsConfianceT5RemoteNamespace(t *testing.T) {
	s, _ := processConfigured(t)
	server := httptest.NewUnstartedServer(nil)
	server.Config.Handler = newWebHandler(s, server.Listener.Addr().String(), trustRemoteFixtureSession)
	server.Start()
	defer server.Close()
	t.Setenv("SWARM_WEB_SESSION", trustRemoteFixtureSession)
	t.Setenv("SWARM_TRUST_TOKEN", trustAdminToken)
	root := filepath.Join(t.TempDir(), "absent")
	var out, errs bytes.Buffer
	if code := run([]string{"--root", root, "--server", server.URL, "process", "show", "--json"}, &out, &errs); code != 0 {
		t.Fatalf("remote process view refused %d %s", code, errs.Bytes())
	}
	var view map[string]any
	if json.Unmarshal(out.Bytes(), &view) != nil || view["config_revision"] != float64(2) {
		t.Fatal("wrong server view")
	}
	if _, err := os.Stat(root); !os.IsNotExist(err) {
		t.Fatal("remote process touched local root")
	}
}

func processTestJSON(value any) []byte {
	raw, err := json.Marshal(value)
	if err != nil {
		panic(err)
	}
	return raw
}
