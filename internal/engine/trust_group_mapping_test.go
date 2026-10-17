//go:build linux

package engine

import (
	"bytes"
	"context"
	"encoding/json"
	"os"
	"reflect"
	"strings"
	"testing"
	"time"
)

func groupMappingChange(t *testing.T, s *Store, policy any) TrustConfigChange {
	t.Helper()
	v, err := s.trustView()
	if err != nil {
		t.Fatal(err)
	}
	revision := v.(map[string]any)["revision"].(int)
	c := lifecycleChange(t, "alice", false, revision)
	raw, err := json.Marshal(c)
	if err != nil {
		t.Fatal(err)
	}
	var fields map[string]json.RawMessage
	json.Unmarshal(raw, &fields)
	fields["group_mapping"], err = json.Marshal(policy)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ = json.Marshal(fields)
	if err = strict(raw, &c); err != nil {
		t.Fatal("group mapping contract missing", err)
	}
	return c
}
func TestActionsConfianceT3GroupMappingPublicPolicy(t *testing.T) {
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	c := groupMappingChange(t, s, map[string]any{"enabled": false, "claim": "groups", "rules": []any{}})
	v, err := s.trustConfig(c, true)
	if err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(v)
	var d map[string]any
	json.Unmarshal(raw, &d)
	if _, ok := d["group_mapping"]; !ok {
		t.Fatal("preview omits effective mapping")
	}
	after, _ := os.ReadFile(p)
	if !bytes.Equal(before, after) {
		t.Fatal("preview mutated")
	}
}

func activeGroupMapping() any {
	return map[string]any{"enabled": true, "claim": "groups", "rules": []any{map[string]any{"group": "employees", "roles": []string{"requester"}}, map[string]any{"group": "reviewers", "roles": []string{"approver"}}, map[string]any{"group": "services", "roles": []string{"effect"}}, map[string]any{"group": "administrators", "roles": []string{"admin"}}}}
}
func enableGroupMapping(t *testing.T, s *Store) {
	t.Helper()
	c := groupMappingChange(t, s, activeGroupMapping())
	if _, err := s.trustConfig(c, false); err != nil {
		t.Fatal(err)
	}
}
func TestActionsConfianceT3GroupMappingHumanRolesAndPersistence(t *testing.T) {
	f := newOIDCFixture(t)
	f.modify = func(c map[string]any) { c["groups"] = []string{"employees", "administrators", "unknown", "employees"} }
	s := oidcConfiguredStore(t, f)
	enableGroupMapping(t, s)
	v, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s))
	if err != nil {
		t.Fatal(err)
	}
	token := v.(map[string]any)["session_token"].(string)
	idv, err := s.trustIdentityContext([]byte(`{}`), token, time.Now())
	if err != nil {
		t.Fatal(err)
	}
	c := idv.(*TrustIdentityContext)
	if !reflect.DeepEqual(c.Attributes.Roles, []string{"requester"}) || c.Attributes.Integrity != 1 || c.Attributes.RoleMappingDigest == "" {
		t.Fatal("groups amplified local rights or lost mapping digest")
	}
	p, _, _ := s.trustPaths()
	raw, _ := os.ReadFile(p)
	if strings.Contains(string(raw), token) {
		t.Fatal("raw token stored")
	}
	var st trustState
	if strict(raw, &st) != nil {
		t.Fatal("unreadable persisted state")
	}
	session := st.OIDCSessions[trustTokenHash(token)]
	if !reflect.DeepEqual(session.MappedRoles, []string{"requester"}) || session.GroupMappingDigest != c.Attributes.RoleMappingDigest {
		t.Fatal("mapping decision not persisted")
	}
	sessionRaw, _ := json.Marshal(session)
	if strings.Contains(string(sessionRaw), "employees") || strings.Contains(string(sessionRaw), "administrators") {
		t.Fatal("raw groups stored in session")
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	again, err := reopened.trustIdentityContext([]byte(`{}`), token, time.Now())
	if err != nil || !reflect.DeepEqual(c, again) {
		t.Fatal("mapping lost on reopen", err)
	}
	enableGroupMapping(t, s)
	if _, err = s.trustIdentityContext([]byte(`{}`), token, time.Now()); err == nil {
		t.Fatal("old mapping session survives reconfiguration")
	}
}
func TestActionsConfianceT3GroupMappingRejectsSignedClaims(t *testing.T) {
	for _, tc := range []struct {
		name   string
		claim  any
		absent bool
	}{{"absent", nil, true}, {"null", nil, false}, {"scalar", "employees", false}, {"object", map[string]any{}, false}, {"empty", []string{}, false}, {"unknown", []string{"unknown"}, false}, {"outside-ceiling", []string{"administrators"}, false}, {"mixed-types", []any{"employees", 1}, false}, {"empty-element", []string{"employees", ""}, false}, {"null-element", []any{"employees", nil}, false}} {
		t.Run(tc.name, func(t *testing.T) {
			f := newOIDCFixture(t)
			f.modify = func(c map[string]any) {
				if !tc.absent {
					c["groups"] = tc.claim
				}
			}
			s := oidcConfiguredStore(t, f)
			enableGroupMapping(t, s)
			before, _ := s.trustView()
			if _, err := s.trustOIDCFinish(context.Background(), oidcAuthorize(t, s)); err == nil {
				t.Fatal("invalid group claim authenticated")
			}
			_, err := s.withTrustLock(false, func(st *trustState) (any, error) {
				if len(st.OIDCSessions) != 0 || len(st.Actions) != 0 || len(st.Ledger) != 0 {
					t.Fatal("refusal minted a session or business object")
				}
				return nil, nil
			})
			if err != nil {
				t.Fatal(err)
			}
			after, _ := s.trustView()
			if !reflect.DeepEqual(before, after) {
				t.Fatal("invalid groups changed public business state")
			}
		})
	}
}

func TestActionsConfianceT3GroupMappingServiceUsesAccessToken(t *testing.T) {
	for _, valid := range []bool{true, false} {
		name := "assertion-ignored"
		if valid {
			name = "signed-access-token"
		}
		t.Run(name, func(t *testing.T) {
			f := newWorkloadFixture(t)
			if valid {
				f.modify = func(c map[string]any) { c["groups"] = []string{"services", "administrators"} }
			}
			s := workloadConfiguredStore(t, f)
			enableGroupMapping(t, s)
			cmd := workloadAssertion(t, f, func(c map[string]any) { c["groups"] = []string{"services"} })
			v, err := s.trustWorkloadLogin(context.Background(), cmd)
			if !valid {
				if err == nil {
					t.Fatal("assertion groups used as access-token groups")
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			token := v.(map[string]any)["session_token"].(string)
			c, err := s.trustIdentityContext([]byte(`{}`), token, time.Now())
			if err != nil {
				t.Fatal(err)
			}
			attrs := c.(*TrustIdentityContext).Attributes
			if !reflect.DeepEqual(attrs.Roles, []string{"effect"}) || attrs.Integrity != 2 || attrs.RoleMappingDigest == "" {
				t.Fatal("service role mapping incorrect")
			}
		})
	}
}
func TestActionsConfianceT3GroupMappingStepUpUsesDerivedRoles(t *testing.T) {
	for _, allowed := range []bool{false, true} {
		name := "requester-only"
		if allowed {
			name = "approver"
		}
		t.Run(name, func(t *testing.T) {
			s, f, _ := stepUpFixture(t)
			change := groupMappingChange(t, s, activeGroupMapping())
			for i := range change.Identities {
				if change.Identities[i].Subject == "bob" {
					change.Identities[i].Roles = []string{"requester", "approver"}
				}
			}
			if _, err := s.trustConfig(change, false); err != nil {
				t.Fatal(err)
			}
			now := time.Now().UTC()
			m := createMandate(t, s, now, "mapped-step-up", now.Add(time.Hour))
			a := createAction(t, s, now, "mapped-step-up-request", m.ID, "mapped-step-up-key", 100)
			f.modify = func(c map[string]any) {
				c["auth_time"] = time.Now().Unix()
				c["acr"] = "urn:example:strong"
				c["groups"] = []string{"employees"}
				if allowed {
					c["groups"] = []string{"reviewers"}
				}
			}
			v, err := s.trustOIDCFinish(context.Background(), stepUpAuthorize(t, s, a))
			if !allowed {
				if err == nil {
					t.Fatal("step-up used wider local roles instead of derived roles")
				}
				_, err = s.withTrustLock(false, func(st *trustState) (any, error) {
					if len(st.OIDCSessions) != 0 || st.Actions[a.ID].State != "pending_approval" {
						t.Fatal("denied mapping minted step-up or approved action")
					}
					return nil, nil
				})
				if err != nil {
					t.Fatal(err)
				}
				return
			}
			if err != nil {
				t.Fatal(err)
			}
			token := v.(map[string]any)["session_token"].(string)
			out, err := s.trustOperate("approve", stepUpApproveCommand(a, "mapped-approval"), token, time.Now())
			if err != nil || out.(TrustAction).State != "approved" {
				t.Fatal("derived approver could not approve", err)
			}
			exp, err := s.trustExport()
			if err != nil {
				t.Fatal(err)
			}
			ev := exp.Evidence[len(exp.Evidence)-1]
			if ev.IdentityContext == nil || !reflect.DeepEqual(ev.IdentityContext.Attributes.Roles, []string{"approver"}) || ev.IdentityContext.Attributes.RoleMappingDigest == "" {
				t.Fatal("evidence does not bind derived roles")
			}
		})
	}
}
func TestActionsConfianceT3GroupMappingInvalidConfiguration(t *testing.T) {
	cases := map[string]string{
		"null-enabled":    `{"enabled":null,"claim":"groups","rules":[]}`,
		"null-rules":      `{"enabled":false,"claim":"groups","rules":null}`,
		"missing-claim":   `{"enabled":false,"rules":[]}`,
		"blank-claim":     `{"enabled":false,"claim":"","rules":[]}`,
		"null-claim":      `{"enabled":false,"claim":null,"rules":[]}`,
		"unknown-key":     `{"enabled":false,"claim":"groups","rules":[],"other":true}`,
		"null-role":       `{"enabled":false,"claim":"groups","rules":[{"group":"x","roles":null}]}`,
		"empty-role":      `{"enabled":false,"claim":"groups","rules":[{"group":"x","roles":[]}]}`,
		"unknown-role":    `{"enabled":false,"claim":"groups","rules":[{"group":"x","roles":["root"]}]}`,
		"duplicate-role":  `{"enabled":false,"claim":"groups","rules":[{"group":"x","roles":["admin","admin"]}]}`,
		"duplicate-group": `{"enabled":false,"claim":"groups","rules":[{"group":"x","roles":["admin"]},{"group":"x","roles":["effect"]}]}`,
		"active-empty":    `{"enabled":true,"claim":"groups","rules":[]}`,
	}
	s, _ := trustFixture(t)
	p, _, _ := s.trustPaths()
	before, _ := os.ReadFile(p)
	for name, raw := range cases {
		t.Run(name, func(t *testing.T) {
			var policy TrustGroupMapping
			err := strict([]byte(raw), &policy)
			if err == nil {
				err = validateTrustGroupMapping(&policy, nil, nil)
			}
			if err == nil {
				t.Fatal("invalid mapping accepted")
			}
			after, _ := os.ReadFile(p)
			if !bytes.Equal(before, after) {
				t.Fatal("invalid configuration mutated")
			}
		})
	}
	if validateTrustGroupMapping(&TrustGroupMapping{Claim: "groups"}, nil, nil) == nil {
		t.Fatal("typed nil rules accepted")
	}
}
