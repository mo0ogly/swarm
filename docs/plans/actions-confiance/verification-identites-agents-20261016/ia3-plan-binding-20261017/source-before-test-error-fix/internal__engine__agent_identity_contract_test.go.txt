//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"
)

func agentIdentityTestRequest() agentIdentityRequest {
	return agentIdentityRequest{Schema: 1, EventID: "create-1", Scope: agentIdentityScope{"work-1", 3, hash([]byte("adopted-plan"))},
		GrantID: "grant-child", Operation: "agent.create", Resource: "task/child", ContentDigest: hash([]byte("exact launch and context"))}
}

func TestAgentIdentityRequestStrictContract(t *testing.T) {
	r := agentIdentityTestRequest()
	raw, err := json.Marshal(r)
	if err != nil {
		t.Fatal(err)
	}
	got, err := decodeAgentIdentityRequest(raw)
	if err != nil || got != r {
		t.Fatalf("valid request: %#v, %v", got, err)
	}
	scopeRaw, err := json.Marshal(r.Scope)
	if err != nil {
		t.Fatal(err)
	}
	for name, body := range map[string]string{
		"null": "null", "array": "[]", "empty": "{}", "extra-document": string(raw) + " {}",
		"self-asserted-actor":     strings.TrimSuffix(string(raw), "}") + `,"actor":{"principal_kind":"human"}}`,
		"self-asserted-authority": strings.TrimSuffix(string(raw), "}") + `,"authority":{"roots":{"grant-child":true}}}`,
		"self-asserted-approval":  strings.TrimSuffix(string(raw), "}") + `,"human_approved":true}`,
		"duplicate":               strings.TrimSuffix(string(raw), "}") + `,"grant_id":"another"}`,
		"escaped-duplicate":       strings.TrimSuffix(string(raw), "}") + `,"grant_\u0069d":"another"}`,
		"nested-duplicate":        strings.Replace(string(raw), `"plan_version":3`, `"plan_version":3,"plan_version":4`, 1),
		"case-alias":              strings.Replace(string(raw), `"grant_id"`, `"Grant_ID"`, 1),
		"nested-case-alias":       strings.Replace(string(raw), `"work_id"`, `"Work_ID"`, 1),
		"unknown-nested":          strings.Replace(string(raw), `"plan_version":3`, `"plan_version":3,"approved":true`, 1),
		"null-scope":              strings.Replace(string(raw), `"scope":`+string(scopeRaw), `"scope":null`, 1),
		"wildcard-resource":       strings.Replace(string(raw), `"task/child"`, `"task/*"`, 1),
		"control-resource":        strings.Replace(string(raw), `"task/child"`, `"task/\u0000child"`, 1),
		"invalid-utf8":            string(append([]byte{'{', '"', 0xff, '"', ':'}, []byte("0}")...)),
	} {
		t.Run(name, func(t *testing.T) {
			if _, err := decodeAgentIdentityRequest([]byte(body)); err == nil {
				t.Fatal("malformed or self-authorizing request accepted")
			}
		})
	}
}

func TestAgentIdentityCallerUsesTrustWithoutRolePromotion(t *testing.T) {
	s, at := trustFixture(t)
	path, _, _ := s.trustPaths()
	before, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	for name, token := range map[string]string{"requester": trustRequesterToken, "approver": trustApproverToken, "admin": trustAdminToken} {
		t.Run(name, func(t *testing.T) {
			_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
				caller, err := resolveAgentIdentityCaller(s.root, st, token, at)
				if err != nil {
					return nil, err
				}
				if caller.actor.Principal.Kind != "local" || caller.actor.InstanceID != "" || caller.bindingID != "" {
					t.Fatal("local bearer promoted to human or attested instance")
				}
				_, err = evaluateAgentIdentity(agentIdentitySnapshot{}, agentIdentityAuthority{}, caller, agentIdentityTestRequest(), at)
				if err == nil {
					t.Fatal("Trust business role implicitly granted creation rights")
				}
				return nil, nil
			})
			if err != nil {
				t.Fatal(err)
			}
		})
	}
	for _, token := range []string{"", "invented-credential-012345"} {
		_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
			_, err := resolveAgentIdentityCaller(s.root, st, token, at)
			return nil, err
		})
		if err == nil {
			t.Fatal("missing/invalid caller authenticated")
		}
	}
	after, err := os.ReadFile(path)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatalf("read or refusal changed Trust storage: %v", err)
	}
}

func TestAgentIdentityCallerFederationAndReopen(t *testing.T) {
	for _, kind := range []string{"human", "service"} {
		t.Run(kind, func(t *testing.T) {
			var s *Store
			var token, issuer, subject string
			if kind == "human" {
				f := newOIDCFixture(t)
				s = oidcConfiguredStore(t, f)
				v, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
				if err != nil {
					t.Fatal(err)
				}
				token = v.(map[string]any)["session_token"].(string)
				issuer, subject = f.server.URL, "employee-1"
			} else {
				f := newWorkloadFixture(t)
				s = workloadConfiguredStore(t, f)
				v, err := s.trustWorkloadLogin(context.Background(), workloadAssertion(t, f, nil))
				if err != nil {
					t.Fatal(err)
				}
				token = v.(map[string]any)["session_token"].(string)
				issuer, subject = f.server.URL, "service-principal"
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			var caller agentIdentityCaller
			at := time.Now().UTC()
			_, err = reopened.withTrustLock(false, func(st *trustState) (any, error) {
				var err error
				caller, err = resolveAgentIdentityCaller(reopened.root, st, token, at)
				return nil, err
			})
			if err != nil {
				t.Fatal(err)
			}
			if caller.actor.Principal.Kind != kind || caller.actor.Principal.Issuer != issuer || caller.actor.Principal.Subject != subject || caller.expiresAt == nil {
				t.Fatalf("verified federation context lost: %#v", caller)
			}
			encoded, err := json.Marshal(caller.actor)
			if err != nil || strings.Contains(string(encoded), token) || strings.Contains(string(encoded), trustTokenHash(token)) {
				t.Fatal("raw credential or digest leaked into attribution")
			}
			_, err = reopened.withTrustLock(false, func(st *trustState) (any, error) {
				_, err := resolveAgentIdentityCaller(reopened.root, st, token, *caller.expiresAt)
				return nil, err
			})
			if err == nil {
				t.Fatal("expired federated identity accepted")
			}
		})
	}
}

func TestAgentIdentityCallerSuspensionAndLegacyContext(t *testing.T) {
	s, at := trustFixture(t)
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
		copy := *st
		copy.Identities = append([]trustIdentity{}, st.Identities...)
		for i := range copy.Identities {
			if copy.Identities[i].Subject == "alice" {
				copy.Identities[i].Disabled = true
			}
		}
		if _, err := resolveAgentIdentityCaller(s.root, &copy, trustRequesterToken, at); err == nil {
			t.Fatal("suspended caller accepted")
		}
		if _, err := resolveAgentIdentityCaller(s.root, st, trustRequesterToken, time.Time{}); err == nil {
			t.Fatal("missing clock accepted")
		}
		if reflect.DeepEqual(st.Identities, copy.Identities) {
			t.Fatal("suspension fixture did not change")
		}
		return nil, nil
	})
	if err != nil {
		t.Fatal(err)
	}
	f := newOIDCFixture(t)
	legacy := oidcConfiguredStore(t, f)
	_, err = legacy.withTrustLock(false, func(st *trustState) (any, error) {
		st.OIDCSessions = map[string]trustOIDCSession{trustTokenHash("legacy-identity-token-012345"): {Subject: "alice", Revision: st.Revision, ExpiresAt: at.Add(time.Hour)}}
		if _, err := resolveAgentIdentityCaller(legacy.root, st, "legacy-identity-token-012345", at); err == nil {
			t.Fatal("legacy session acquired invented issuer/subject/authentication")
		}
		return nil, nil
	})
	if err != nil {
		t.Fatal(err)
	}
}

func TestAgentIdentityPrincipalKeepsOpaqueIssuerAndSubject(t *testing.T) {
	p := agentIdentityPrincipal{ID: "service-1", Kind: "service", Issuer: "https://[::1]/issuer", Subject: "service[eu]"}
	if !p.valid() {
		t.Fatal("valid opaque issuer/subject confused with permission wildcard syntax")
	}
	if (agentIdentityPermission{Operation: "memory.read", Resource: "memory/*"}).valid() {
		t.Fatal("wildcard permission accepted")
	}
}

func agentIdentityRootTestState(t *testing.T, caller agentIdentityCaller, at time.Time) (agentIdentitySnapshot, agentIdentityAuthority, []byte) {
	t.Helper()
	r := agentIdentityTestRequest()
	r.GrantID = "grant-root"
	permission := agentIdentityPermission{r.Operation, r.Resource}
	root := agentIdentityGrant{ID: r.GrantID, Issuer: caller.actor, Holder: caller.actor, Scope: r.Scope, TrustRevision: caller.trustRevision,
		Permissions: []agentIdentityPermission{permission}, NotBefore: at.Add(-time.Minute), ExpiresAt: at.Add(time.Hour)}
	snapshot := agentIdentitySnapshot{Schema: 1, Revision: 1, Grants: map[string]agentIdentityGrant{root.ID: root}, Instances: map[string]agentIdentityInstance{}}
	digest, err := agentIdentityDigest(snapshot)
	if err != nil {
		t.Fatal(err)
	}
	authority := agentIdentityAuthority{projectRoot: caller.projectRoot, snapshotDigest: digest, snapshotRevision: snapshot.Revision, trustRevision: caller.trustRevision, scope: r.Scope, roots: map[string]bool{root.ID: true}}
	raw, err := json.Marshal(r)
	if err != nil {
		t.Fatal(err)
	}
	return snapshot, authority, raw
}

func TestAgentIdentityInternalConsumerResolvesEveryRequest(t *testing.T) {
	s, at := trustFixture(t)
	var caller agentIdentityCaller
	_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
		var err error
		caller, err = resolveAgentIdentityCaller(s.root, st, trustRequesterToken, at)
		return nil, err
	})
	if err != nil {
		t.Fatal(err)
	}
	snapshot, authority, raw := agentIdentityRootTestState(t, caller, at)
	path, _, _ := s.trustPaths()
	before, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	decision, err := s.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, nil, at)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, &decision, at.Add(time.Second)); err != nil {
		t.Fatal(err)
	}
	for _, token := range []string{"", "invalid-token-0123456789", trustAdminToken} {
		if _, err := s.evaluateAgentIdentityRequest(raw, token, snapshot, authority, &decision, at.Add(time.Second)); err == nil {
			t.Fatal("replay returned previous decision to another/missing caller")
		}
	}
	after, err := os.ReadFile(path)
	if err != nil || !bytes.Equal(before, after) {
		t.Fatal("evaluation/replay/refusal changed Trust storage")
	}
}

func TestAgentIdentityInternalConsumerLogoutInvalidatesReplay(t *testing.T) {
	f := newOIDCFixture(t)
	s := oidcConfiguredStore(t, f)
	login, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
	if err != nil {
		t.Fatal(err)
	}
	token := login.(map[string]any)["session_token"].(string)
	var caller agentIdentityCaller
	at := time.Now().UTC()
	_, err = s.withTrustLock(false, func(st *trustState) (any, error) {
		var err error
		caller, err = resolveAgentIdentityCaller(s.root, st, token, at)
		return nil, err
	})
	if err != nil {
		t.Fatal(err)
	}
	snapshot, authority, raw := agentIdentityRootTestState(t, caller, at)
	decision, err := s.evaluateAgentIdentityRequest(raw, token, snapshot, authority, nil, time.Time{})
	if err != nil {
		t.Fatal(err)
	}
	if _, err := s.evaluateAgentIdentityRequest(raw, token, snapshot, authority, &decision, time.Time{}); err != nil {
		t.Fatal(err)
	}
	logout, _ := json.Marshal(map[string]string{"session_token": token})
	if _, err := s.trustOIDCOperation(context.Background(), "oidc-logout", logout); err != nil {
		t.Fatal(err)
	}
	var revision int
	_, err = s.withTrustLock(false, func(st *trustState) (any, error) { revision = st.Revision; return nil, nil })
	if err != nil {
		t.Fatal(err)
	}
	if revision != authority.trustRevision {
		t.Fatal("logout fixture unexpectedly changed configuration revision")
	}
	_, err = s.evaluateAgentIdentityRequest(raw, token, snapshot, authority, &decision, time.Time{})
	var denied *CommandError
	if !errors.As(err, &denied) || denied.Code != "authorization_required" {
		t.Fatalf("logged out session reused an old decision: %v", err)
	}
}

func TestAgentIdentityInternalConsumerRejectsForeignProject(t *testing.T) {
	first, at := trustFixture(t)
	second, _ := trustFixture(t)
	var caller agentIdentityCaller
	_, err := first.withTrustLock(false, func(st *trustState) (any, error) {
		var err error
		caller, err = resolveAgentIdentityCaller(first.root, st, trustRequesterToken, at)
		return nil, err
	})
	if err != nil {
		t.Fatal(err)
	}
	snapshot, authority, raw := agentIdentityRootTestState(t, caller, at)
	decision, err := first.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, nil, at)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := second.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, &decision, at); err == nil {
		t.Fatal("an authority and decision from another project were accepted despite a different Store root")
	}
	// A fixture's explicit independent authorization in project two can allow a
	// new decision, but must not turn project one's receipt into its own.
	authority.projectRoot = second.root
	secondDecision, err := second.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, nil, at)
	if err != nil {
		t.Fatal(err)
	}
	if secondDecision.ProjectDigest == decision.ProjectDigest {
		t.Fatal("project contexts collapsed")
	}
	if _, err := second.evaluateAgentIdentityRequest(raw, trustRequesterToken, snapshot, authority, &decision, at); err == nil {
		t.Fatal("a foreign project receipt was reused after separate authorization")
	}
}
