//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// tests/engine_acceptance.cjs --case launch requires a passing
// TestEngineContractLaunch to consider E1 ("no launch or success without an
// independent reviewer") demonstrated. Without this test the runner reports
// "No complete passing behavioral test run" even though the underlying guard
// (see TestReviewerLaunchContractRejectsLegacyOmission) already holds; this
// wires the acceptance harness to that guard via the same public entry points.
func TestEngineContractLaunch(t *testing.T) {
	s := storeTest(t)
	w, launch := setupAgent(t, s)
	organizedFixtureStore(t, s)
	w, _ = s.get(w.ID)
	w.Planning.Reviewer = nil
	w.Planning.ReviewerRequired = false // historical bug must not be an exemption
	raw, _ := json.Marshal(w)
	if _, e := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); e != nil {
		t.Fatal(e)
	}
	launch.Revision = w.Revision
	if _, _, e := s.prepare(w.ID, launch); e == nil || !strings.Contains(e.Error(), "Vérificateur") {
		t.Fatalf("manual launch bypassed reviewer: %v", e)
	}
	profile := LaunchProfile{Provider: "fixture", Workspace: s.root, Role: "worker"}
	if e := s.configureMission(w.ID, profile, 1, w.Revision); e == nil {
		t.Fatal("mission start bypassed reviewer")
	}
	preview, e := s.missionLaunchPreview(w.ID, profile, 1)
	if e != nil || preview.Organization.Ready || preview.Immediate != 0 || preview.Token != "" {
		t.Fatalf("preview promised an unreviewed launch: %+v %v", preview, e)
	}
	if _, e := s.webAction(webRequest{Kind: "mission-start", Work: w.ID, Event: "engine-contract-launch", Revision: w.Revision, Provider: "fixture", Workspace: s.root, Slots: 1, PreviewToken: preview.Token}); e == nil {
		t.Fatal("web action bypassed reviewer")
	}
	input := filepath.Join(t.TempDir(), "profile.json")
	raw, _ = json.Marshal(profile)
	if e := os.WriteFile(input, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out bytes.Buffer
	if e := missionCLI(s, []string{"mission", "start", w.ID}, input, true, &out); e == nil {
		t.Fatal("CLI start bypassed reviewer")
	}
	var count int
	if e := s.db.QueryRow("SELECT count(*) FROM agents WHERE work_id=?", w.ID).Scan(&count); e != nil || count != 0 {
		t.Fatalf("refusal reserved an agent: %d %v", count, e)
	}
	after, _ := s.get(w.ID)
	if after.Revision != w.Revision {
		t.Fatal("refusal changed work")
	}
	policy, _ := s.missionPolicy(w.ID)
	if policy.Enabled {
		t.Fatal("refusal authorized mission")
	}
}

// All obligations below execute under the engine's authorized launch command;
// a report citing an unrelated green test is not sufficient evidence.
func TestEngineContractLaunchCompleteEvidence(t *testing.T) {
	t.Run("legacy_missing_reviewer", TestReviewerLaunchContractRejectsLegacyOmission)
	t.Run("unavailable_reviewer", TestReviewerLaunchContractUnavailableReviewer)
	t.Run("spent_budget", TestReviewerLaunchContractSpentBudgetDoesNotInvalidateOrganization)
	t.Run("acceptance_and_proof_freshness", TestAcceptanceFreshnessAndAttempts)
	t.Run("reopened_dependency", TestReopenedDependencyInvalidatesAcceptance)
	t.Run("import_missing_no_overwrite", TestExportImportMissingAndNoOverwrite)
	t.Run("import_atomic_version", TestImportMissingPieceAndUnknownVersion)
	t.Run("hostile_archive", TestRejectHostileArchive)
	t.Run("managed_acceptance_no_reviewer", TestManagedIndependentReviewLegacyFlagCannotBypassAcceptance)
	t.Run("managed_missing_or_exhausted", TestManagedIndependentReviewMissingOrBudgetExhaustedCannotPublish)
}
