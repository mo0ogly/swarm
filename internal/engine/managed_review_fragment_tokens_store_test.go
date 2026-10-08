//go:build linux

package engine

import (
	"encoding/json"
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
	for _, mode := range []string{"pass", "fail", "capacity-drift", "coordinated"} {
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
				if state == "passed" || managedReviewCalls(t, s) != 1 {
					t.Fatal("defect not stopped", state, err)
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
