//go:build linux

package engine

import (
	"bytes"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"testing"
	"time"
)

func TestActionsConfianceT6CorruptChainStopsEffects(t *testing.T) {
	for _, access := range []bool{false, true} {
		for _, variant := range []string{"reason", "sequence", "previous", "approval", "direct", "replay", "fresh-event"} {
			t.Run(fmt.Sprint(access)+"/"+variant, func(t *testing.T) {
				s, at := trustFixture(t)
				a := connectorApprove(t, s, at, connectorFixtureAction(t, s, at, access))
				c := trustCmd("guarded-effect")
				c.ActionID = a.ID
				if variant == "replay" || variant == "fresh-event" {
					if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
						t.Fatal(err)
					}
				}
				_, err := s.withTrustLock(true, func(st *trustState) (any, error) {
					switch variant {
					case "sequence":
						st.Evidence[0].Sequence++
					case "previous":
						st.Evidence[1].PreviousHash = "changed"
					case "approval":
						st.Evidence[len(st.Evidence)-1].Hash = "changed"
					default:
						st.Evidence[0].Reason += "tampered"
					}
					return nil, nil
				})
				if err != nil {
					t.Fatal(err)
				}
				path := filepath.Join(s.root, ".swarm", "trust-actions.json")
				before, err := os.ReadFile(path)
				if err != nil {
					t.Fatal(err)
				}
				if variant == "direct" {
					_, err = s.withTrustLock(false, func(st *trustState) (any, error) {
						n := len(st.Ledger) + len(st.AccessLedger)
						e := trustConnectorCommit(st, a, trustIdentity{Subject: "effect-service", Integrity: 2, Roles: []string{"effect"}}, at, false)
						if e == nil || len(st.Ledger)+len(st.AccessLedger) != n {
							t.Error("direct SPI accepted corrupt chain or mutated native state")
						}
						return nil, nil
					})
				} else {
					if variant == "fresh-event" {
						c.EventID = "fresh-after-corruption"
					}
					_, err = s.trustOperate("commit", c, trustEffectToken, at)
					if variant == "replay" {
						if err != nil {
							t.Error("historical read failed", err)
						}
					} else if err == nil {
						t.Error("effect or replay accepted an already corrupt chain")
					}
				}
				after, e := os.ReadFile(path)
				if e != nil || !bytes.Equal(before, after) {
					t.Error("rejected operation changed persisted state")
				}
			})
		}
	}
}

func TestActionsConfianceT6EffectProofReconciliationSnapshot(t *testing.T) {
	for _, access := range []bool{false, true} {
		t.Run(fmt.Sprint(access), func(t *testing.T) {
			s, at := trustFixture(t)
			a := connectorApprove(t, s, at, connectorFixtureAction(t, s, at, access))
			c := trustCmd("snapshot-commit")
			c.ActionID = a.ID
			c.LoseResponse = true
			if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
				t.Fatal(err)
			}
			before, err := s.trustExport()
			if err != nil {
				t.Fatal(err)
			}
			original, _ := json.Marshal(before.Evidence[len(before.Evidence)-1])
			c.EventID = "snapshot-reconcile"
			c.LoseResponse = false
			if _, err = s.trustOperate("reconcile", c, trustEffectToken, at.Add(time.Second)); err != nil {
				t.Fatal(err)
			}
			after, err := s.trustExport()
			if err != nil {
				t.Fatal(err)
			}
			retained, _ := json.Marshal(after.Evidence[len(before.Evidence)-1])
			if !bytes.Equal(original, retained) || before.Evidence[len(before.Evidence)-1].EffectReceipt.Result.State != "unknown" {
				t.Fatal("reconciliation rewrote the earlier observation")
			}
			if err := verifyTrustEvidenceChain(after); err != nil {
				t.Fatal(err)
			}
		})
	}
}

func TestActionsConfianceT6EffectProofAnchorsAndTampering(t *testing.T) {
	s := evidenceFixture(t)
	at := time.Now().UTC()
	a := connectorApprove(t, s, at, connectorFixtureAction(t, s, at, false))
	c := trustCmd("anchored-effect")
	c.ActionID = a.ID
	if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
		t.Fatal(err)
	}
	k := evidencePrepare(t, s, "effect-proof-key", 0)
	evidenceActivate(t, s, k, "activate-effect-proof-key", 1)
	exp, public, pin := evidenceAnchor(t, s)
	anchor, err := s.trustAnchor()
	if err != nil {
		t.Fatal(err)
	}
	key, err := s.trustProofKey()
	if err != nil {
		t.Fatal(err)
	}
	verify := func(e TrustProofExport) bool {
		a := verifyTrustProof(e, anchor, hex.EncodeToString(key))
		_, b := verifyPublicEvidence(TrustEvidenceCommand{Export: &e, Anchor: &public, TrustedKey: &pin})
		return a == nil && b == nil
	}
	if !verify(exp) {
		t.Fatal("intact new proof failed native anchors")
	}
	for _, variant := range []string{"policy", "policy-missing", "effect-missing", "intent", "result", "native"} {
		t.Run(variant, func(t *testing.T) {
			raw, _ := json.Marshal(exp)
			var changed TrustProofExport
			_ = json.Unmarshal(raw, &changed)
			index := -1
			for i, event := range changed.Evidence {
				if event.Kind == "commit" && event.Result == "committed" {
					index = i
				}
			}
			if index < 0 {
				t.Fatal("no committed proof")
			}
			e := &changed.Evidence[index]
			switch variant {
			case "policy":
				e.PolicyContext.Policy.RequireSeparation = false
			case "policy-missing":
				e.PolicyContext = nil
			case "effect-missing":
				e.EffectReceipt = nil
			case "intent":
				e.EffectReceipt.Intent.MandateHash = "changed"
			case "result":
				e.EffectReceipt.Result.Digest = "changed"
			case "native":
				e.EffectReceipt.Native = json.RawMessage(`{}`)
			}
			hmacErr := verifyTrustProof(changed, anchor, hex.EncodeToString(key))
			_, publicErr := verifyPublicEvidence(TrustEvidenceCommand{Export: &changed, Anchor: &public, TrustedKey: &pin})
			if hmacErr == nil || publicErr == nil {
				t.Fatal("an anchor accepted changed or missing proof context")
			}
		})
	}
}

func TestActionsConfianceT6ExportBindsEffectAndPolicy(t *testing.T) {
	for _, access := range []bool{false, true} {
		t.Run(fmt.Sprint(access), func(t *testing.T) {
			s, at := trustFixture(t)
			a := connectorApprove(t, s, at, connectorFixtureAction(t, s, at, access))
			c := trustCmd("export-effect")
			c.ActionID = a.ID
			if _, err := s.trustOperate("commit", c, trustEffectToken, at); err != nil {
				t.Fatal(err)
			}
			export, err := s.trustExport()
			if err != nil {
				t.Fatal(err)
			}
			var receipt trustReceipt
			_, err = s.withTrustLock(false, func(st *trustState) (any, error) { receipt = st.Receipts[a.IdempotencyKey]; return nil, nil })
			if err != nil {
				t.Fatal(err)
			}
			if err := verifyTrustEvidenceChain(export); err != nil {
				t.Fatal(err)
			}
			raw, _ := json.Marshal(export.Evidence[len(export.Evidence)-1])
			var fields map[string]json.RawMessage
			if err := json.Unmarshal(raw, &fields); err != nil {
				t.Fatal(err)
			}
			var policy struct {
				Schema   int         `json:"schema_version"`
				Revision int         `json:"revision"`
				Policy   TrustPolicy `json:"policy"`
			}
			if json.Unmarshal(fields["policy_context"], &policy) != nil || policy.Schema != 1 || policy.Revision != 1 || !policy.Policy.RequireSeparation {
				t.Error("export omits the effective versioned policy")
			}
			var effect struct {
				Schema int                `json:"schema_version"`
				Intent *TrustEffectIntent `json:"intent"`
				Result *TrustEffectResult `json:"result"`
				Native json.RawMessage    `json:"native_result"`
			}
			if json.Unmarshal(fields["effect_receipt"], &effect) != nil || effect.Schema != 1 || hashMust(effect.Intent) != hashMust(receipt.Intent) || hashMust(effect.Result) != hashMust(receipt.Result) || len(effect.Native) == 0 || hash(effect.Native) != receipt.Result.Digest {
				t.Error("export omits the exact connector intent/result/native observation")
			}
		})
	}
}
