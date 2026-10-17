//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

const tokenMetadataFixture = `
if '--version' in sys.argv:
 print('codex-cli isolated-test-client');sys.exit(0)
if 'debug' in sys.argv and 'models' in sys.argv:
 print(open(os.path.join(os.environ['CODEX_HOME'],'models_cache.json')).read());sys.exit(0)
`

func TestManagedFragmentTokenStoreRuntimeAndCapabilityDrift(t *testing.T) {
	for _, mode := range []string{"pass", "fail", "capacity-drift", "coordinated", "fresh-capacity"} {
		t.Run(mode, func(t *testing.T) {
			s, w, a, c, path, receipt := fragmentBeginFixture(t)
			catalog := filepath.Join(t.TempDir(), "models_cache.json")
			t.Setenv("CODEX_HOME", filepath.Dir(catalog))
			cache := `{"client_version":"isolated-test-client","models":[{"slug":"gpt-5.6-sol","visibility":"list","supported_reasoning_levels":[{"effort":"medium"}],"context_window":272000,"effective_context_window_percent":95}]}`
			if err := os.WriteFile(catalog, []byte(cache), 0600); err != nil {
				t.Fatal(err)
			}
			// Same deterministic reviewer, through the actual Codex CLI adapter.
			fixture := strings.Replace(fragmentRuntimeProviderFixture, "json.loads(sys.argv[sys.argv.index('--json-schema')+1])", "json.load(open(sys.argv[sys.argv.index('--output-schema')+1]))", 1)
			fixture = strings.Replace(fixture, "text=sys.stdin.read()", tokenMetadataFixture+"\ntext=sys.stdin.read()", 1)
			command := filepath.Join(s.root, "review-fixture/codex")
			if err := os.WriteFile(command, []byte(fixture), 0700); err != nil {
				t.Fatal(err)
			}
			ps, err := s.providers()
			if err != nil {
				t.Fatal(err)
			}
			ps.Providers["managed-review-fixture"] = Provider{Command: command}
			raw, _ := json.Marshal(ps)
			if err = os.WriteFile(filepath.Join(s.root, ".swarm/providers.json"), raw, 0600); err != nil {
				t.Fatal(err)
			}
			wantCalls := 3
			wantPackets := 1
			if mode == "coordinated" {
				wantCalls = 4
				wantPackets = 2
			}
			cfg, err := s.reviewerConfig("managed-review-fixture", "standard", wantCalls)
			if err != nil {
				t.Fatal(err)
			}
			w, err = s.mutate(w.ID, "test.review-capacity", "fixture-token-capacity", w.Revision, []byte(`{}`), func(w *Work) error { w.Planning.Reviewer = cfg; return nil })
			if err != nil {
				t.Fatal(err)
			}
			c, err = s.managedReviewContext(w, a, c.Candidate, receipt)
			if err != nil {
				t.Fatal(err)
			}
			if mode == "coordinated" {
				preview, e := s.managedScopePreview(w.ID, PlanningRequest{Task: a.TaskID}, false)
				if e != nil {
					t.Fatal(e)
				}
				proposal := ReviewCoordinationProposal{Candidate: c.Candidate, Evidence: coordinationEvidence(c), FinalReview: "Verify all interactions and original task criteria.", Lots: []ReviewCoordinationLot{{ID: "bounded", Kind: "component", Objective: "Inspect bounded candidate evidence", Files: preview.Files, Criteria: preview.Criteria}}}
				raw, _ := json.Marshal(proposal)
				w, err = s.mutate(w.ID, "test.coordination", "fixture-coordination", w.Revision, raw, func(current *Work) error {
					task, _ := current.task(a.TaskID)
					task.ReviewCoordination = &ReviewCoordinationRecord{Proposal: proposal, Digest: hash(raw), Order: []string{"bounded"}, State: "validated_not_executed"}
					return nil
				})
				if err != nil {
					t.Fatal(err)
				}
			}
			if mode == "fresh-capacity" {
				workflow, method, e := s.projectAgentWorkflow("reviewer")
				if e != nil {
					t.Fatal(e)
				}
				_ = workflow
				budget, e := s.reviewInputBudget(cfg)
				if e != nil {
					t.Fatal(e)
				}
				pieces, e := managedFragmentArtifacts(c)
				if e != nil {
					t.Fatal(e)
				}
				p, e := planFreshTokenFragments(c, [][]managedReviewFragmentArtifact{pieces}, nil, nil, nil, *budget, managedReviewPrefix(method), wantCalls)
				if e != nil {
					t.Fatal(e)
				}
				if p.InputBudget.FinalReserveContractVersion != 1 {
					t.Fatal("new contract not anchored")
				}
				// Actual complete prompts and deterministic independent replies
				// are validated together, including original final evidence.
				replies := make([]string, len(p.Packets))
				for i, packet := range p.Packets {
					raw, _ := json.Marshal(packet)
					findings := map[string]any{}
					for j := range packet.Artifacts {
						findings[fmt.Sprint(j)] = map[string]any{"v": "inspected", "r": "Content examined", "e": 0}
					}
					reply, _ := json.Marshal(map[string]any{"candidate_commit": packet.Candidate, "context_sha256": packet.ContextDigest, "packet_sha256": hash(raw), "findings": findings})
					replies[i] = string(reply)
				}
				if _, e = managedFragmentFinalBundle(c, p, replies); e != nil {
					t.Fatal(e)
				}
				if _, e = managedFragmentRequestPrompt("", c, p, replies); e != nil {
					t.Fatal(e)
				}
				if _, _, e = managedFragmentDecisionPrompt("", c, p, replies, nil); e != nil {
					t.Fatal(e)
				}
				return
			}
			cost, err := s.managedReviewCostPreview(w.ID, a.TaskID)
			if err != nil || cost.Protocol != 4 || cost.CallsRequired != wantCalls || !cost.FitsBudget || !cost.TransportReady {
				t.Fatal("preflight differs from execution", cost, err)
			}
			managedReviewMode(t, s, mode)
			r, err := s.beginManagedFragmentReview(w, a, c, path, receipt)
			if err != nil {
				t.Fatal(err)
			}
			p, _, err := s.readFragmentJournalAnchor(r)
			if err != nil || p.Version != 4 || fragmentPaidInspections(p) != wantPackets {
				t.Fatal("not a bounded token review", p.Version, err)
			}
			if managedReviewCalls(t, s) != 0 {
				t.Fatal("begin spent a call")
			}
			reopened, err := openStore(s.root, false)
			if err != nil {
				t.Fatal(err)
			}
			defer reopened.db.Close()
			if _, _, err = reopened.readFragmentJournalAnchor(r); err != nil {
				t.Fatal("reopen", err)
			}
			if mode == "capacity-drift" {
				os.WriteFile(command, []byte(fixture+"\n# client changed\n"), 0700)
				if _, _, err = reopened.readFragmentJournalAnchor(r); err == nil {
					t.Fatal("client drift accepted")
				}
				if managedReviewCalls(t, s) != 0 {
					t.Fatal("drift spent a call")
				}
				// Reading a completed historical record does not require the old
				// executable to remain installed. This is not task acceptance.
				historical := r
				historical.State = "changes_requested"
				if _, _, err = reopened.readFragmentJournalAnchor(historical); err != nil {
					t.Fatal("history invalidated by upgrade", err)
				}

				return
			}
			state, records, err := s.runManagedFragmentReview(w, a, r)
			if mode == "fail" {
				if err != nil || state != "changes_requested" || len(records) != len(c.Tasks) || managedReviewCalls(t, s) != wantCalls {
					t.Fatal("final independent refusal missing", state, err)
				}
			} else if err != nil || state != "passed" || len(records) != len(c.Tasks) || managedReviewCalls(t, s) != wantCalls {
				t.Fatal("protocol runtime", state, len(records), managedReviewCalls(t, s), err)
			}
			current, _ := s.get(w.ID)
			if current.Planning.Reviewer.Calls > wantCalls {
				t.Fatal("quota exceeded")
			}
		})
	}
}
