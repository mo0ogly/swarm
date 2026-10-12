//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func defectCitationNativeFixture(t *testing.T, name string) (managedReviewFragmentPacket, string) {
	t.Helper()
	var packet managedReviewFragmentPacket
	raw, err := os.ReadFile(filepath.Join("testdata", "defect-citation-native-packet.json"))
	if err != nil {
		t.Fatal(err)
	}
	if err = json.Unmarshal(raw, &packet); err != nil {
		t.Fatal(err)
	}
	reply, err := os.ReadFile(filepath.Join("testdata", "defect-citation-native-"+name+"-reply.json"))
	if err != nil {
		t.Fatal(err)
	}
	return packet, string(reply)
}

func TestDefectCitationNativeLegacyOriginalsRemainRejected(t *testing.T) {
	for _, name := range []string{"first", "second"} {
		t.Run(name, func(t *testing.T) {
			packet, reply := defectCitationNativeFixture(t, name)
			_, _, err := parseManagedFragmentInspection(reply, packet)
			if err == nil || commandFailure(err).Code != reviewFragmentProtocolError {
				t.Fatal("invalid native citation admitted", err)
			}
			var r managedFragmentInspection
			if err = decodeManagedFragmentInspection([]byte(reply), packet, &r); err != nil {
				t.Fatal(err)
			}
			d := r.Defects[0]
			text := fragmentDefectText(packet.Artifacts[d.Artifact])
			if name == "first" && (!fragmentDefectQuoteAtLine(text, d.Line, d.Quote) || len(strings.TrimSpace(d.Quote)) != 7) {
				t.Fatal("native exact short citation no longer discriminates")
			}
			if name == "second" {
				position := strings.Index(text, d.Quote)
				if position < 0 || strings.Count(text, d.Quote) != 1 || strings.Count(text[:position], "\n")+1 == d.Line {
					t.Fatal("native wrong-line counterexample no longer discriminates")
				}
			}
		})
	}
}

func quoteOnlyNativeReply(t *testing.T, packet managedReviewFragmentPacket, reply string) string {
	t.Helper()
	var wire map[string]any
	if err := json.Unmarshal([]byte(reply), &wire); err != nil {
		t.Fatal(err)
	}
	for _, value := range wire["defects"].([]any) {
		delete(value.(map[string]any), "line")
	}
	raw, _ := json.Marshal(packet)
	wire["packet_sha256"] = hash(raw)
	encoded, err := json.Marshal(wire)
	if err != nil {
		t.Fatal(err)
	}
	return string(encoded)
}

func TestDefectCitationNativeUniqueBytesComputePhysicalLine(t *testing.T) {
	p, original := defectCitationNativeFixture(t, "second")
	p.DefectCitationContractVersion = managedFragmentDefectCitationContractVersion
	reply := quoteOnlyNativeReply(t, p, original)
	state, result, err := parseManagedFragmentInspection(reply, p)
	if err != nil || state != "changes_requested" || len(result.Defects) != 1 {
		t.Fatal(state, err)
	}
	d := result.Defects[0]
	if d.Line == 4 || d.Quote != "panic: test timed out after 4m30s" || !fragmentDefectQuoteAtLine(fragmentDefectText(p.Artifacts[d.Artifact]), d.Line, d.Quote) {
		t.Fatal("literal bytes or physical location lost", d)
	}
	// This local diagnostic shadow is not the original native opinion and does
	// not establish that an historical failed run is a current business defect.
	p, original = defectCitationNativeFixture(t, "first")
	p.DefectCitationContractVersion = managedFragmentDefectCitationContractVersion
	var shadow map[string]any
	if err := json.Unmarshal([]byte(quoteOnlyNativeReply(t, p, original)), &shadow); err != nil {
		t.Fatal(err)
	}
	encoded, _ := json.Marshal(shadow)
	state, result, err = parseManagedFragmentInspection(string(encoded), p)
	if err != nil || state != "changes_requested" || result.Defects[0].Quote != "+exit=1\n" || result.Defects[0].Line != 7 || len(result.Defects) != 3 {
		t.Fatal("exact short native citation lost", state, err)
	}
	shadow["defects"] = append(shadow["defects"].([]any), shadow["defects"].([]any)[0])
	encoded, _ = json.Marshal(shadow)
	if _, _, err := parseManagedFragmentInspection(string(encoded), p); err == nil || !strings.Contains(err.Error(), "duplicate defect claim") {
		t.Fatal("duplicate claim admitted", err)
	}

}

func TestDefectCitationUniqueBytesAreLiteralAndUnambiguous(t *testing.T) {
	for _, tc := range []struct {
		name, text, quote string
		line              int
		valid             bool
	}{
		{"short entire patch line", "header\n+exit=1", "+exit=1", 2, true},
		{"invented trailing newline", "header\n+exit=1", "+exit=1\n", 0, false},
		{"inline Unicode", "head\n+  échec : résultat\n", "échec : résultat", 2, true},
		{"multiline original", "head\n+ exact\n+ suivant", "exact\n+ suivant", 2, true},
		{"space normalization", "+ x ;", "x;", 0, false},
		{"line separator normalization", "x\r\ny", "x\ny", 0, false},
		{"ambiguous", "same\nsame", "same", 0, false},
		{"overlapping ambiguous", "aaa", "aa", 0, false},
		{"blank", "x", " ", 0, false},
		{"separator start", "head\n+ next", "\n+ next", 0, false},
	} {
		t.Run(tc.name, func(t *testing.T) {
			line, err := fragmentDefectUniqueQuoteLine(tc.text, tc.quote)
			if tc.valid && (err != nil || line != tc.line) {
				t.Fatal(line, err)
			}
			if !tc.valid && err == nil {
				t.Fatal("unproved location admitted")
			}
		})
	}
}

func TestDefectCitationQuoteOnlyRejectsUnrequestedLocationAndUnknownVersion(t *testing.T) {
	p, original := defectCitationNativeFixture(t, "second")
	p.DefectCitationContractVersion = managedFragmentDefectCitationContractVersion
	var wire map[string]any
	if err := json.Unmarshal([]byte(original), &wire); err != nil {
		t.Fatal(err)
	}
	raw, _ := json.Marshal(p)
	wire["packet_sha256"] = hash(raw)
	encoded, _ := json.Marshal(wire)
	if _, _, err := parseManagedFragmentInspection(string(encoded), p); err == nil {
		t.Fatal("model-supplied line admitted under quote-only contract")
	}
	p.DefectCitationContractVersion = 99
	if _, _, err := parseManagedFragmentInspection(original, p); err == nil {
		t.Fatal("unknown contract accepted")
	}
	if _, err := managedFragmentInspectionPrompt("method", p); err == nil {
		t.Fatal("unknown contract sent")
	}
}

func TestDefectCitationMissingArtifactCannotDefaultToZero(t *testing.T) {
	p, r := inspectionReplyFixture()
	p.Version = 2
	p.DefectCitationContractVersion = managedFragmentDefectCitationContractVersion
	raw, _ := json.Marshal(p)
	r.PacketDigest = hash(raw)
	r.Findings[0].Verdict = "fail"
	r.Defects = []managedFragmentDefect{{Artifact: 0, Quote: p.Artifacts[0].Content, Explanation: "The original fixture demonstrates a defect requiring an attributed correction.", Reproduction: "Run the isolated fixture with its failing input.", Expected: "Reject invalid input."}}
	encoded, _ := json.Marshal(r)
	var wire map[string]any
	if err := json.Unmarshal(encoded, &wire); err != nil {
		t.Fatal(err)
	}
	d := wire["defects"].([]any)[0].(map[string]any)
	delete(d, "line")
	delete(d, "artifact")
	encoded, _ = json.Marshal(wire)
	if _, _, err := parseManagedFragmentInspection(string(encoded), p); err == nil {
		t.Fatal("missing artifact silently interpreted as zero")
	}
}

func TestDefectCitationNewSchemasKeepHistoricalShape(t *testing.T) {
	p, original := defectCitationNativeFixture(t, "second")
	legacy := managedFragmentPacketSchema(p)
	var schema map[string]any
	if err := json.Unmarshal([]byte(legacy), &schema); err != nil {
		t.Fatal(err)
	}
	item := schema["properties"].(map[string]any)["defects"].(map[string]any)["items"].(map[string]any)
	if item["properties"].(map[string]any)["line"] == nil {
		t.Fatal("historical schema changed")
	}
	p.DefectCitationContractVersion = managedFragmentDefectCitationContractVersion
	current := managedFragmentPacketSchema(p)
	if err := json.Unmarshal([]byte(current), &schema); err != nil {
		t.Fatal(err)
	}
	item = schema["properties"].(map[string]any)["defects"].(map[string]any)["items"].(map[string]any)
	if item["properties"].(map[string]any)["line"] != nil || item["additionalProperties"] != false {
		t.Fatal("new model can still invent a line")
	}
	if folder := os.Getenv("SWARM_CITATION_SCHEMA_ORACLE_DIR"); folder != "" {
		if err := os.MkdirAll(folder, 0700); err != nil {
			t.Fatal(err)
		}
		for name, body := range map[string]string{"legacy-schema.json": legacy, "current-schema.json": current, "legacy-native-reply.json": original, "quote-only-native-shadow.json": quoteOnlyNativeReply(t, p, original)} {
			if err := os.WriteFile(filepath.Join(folder, name), []byte(body), 0600); err != nil {
				t.Fatal(err)
			}
		}
	}
}

func TestDefectCitationProtocolRestartKeepsMethodAndOldEvidence(t *testing.T) {
	s, w, a := planningProtocolReviewFixture(t)
	task, _ := w.task(a.TaskID)
	old := *task.IndependentReview
	plan, journal, err := s.readFragmentJournalAnchor(old)
	if err != nil {
		t.Fatal(err)
	}
	for i := range plan.Packets {
		plan.Packets[i].DefectCitationContractVersion = 0
	}
	raw, _ := json.Marshal(plan)
	planName := filepath.Join(filepath.Dir(old.FragmentJournal.Plan), "citation-legacy-plan.json")
	if err := atomicWrite(filepath.Join(s.root, planName), raw); err != nil {
		t.Fatal(err)
	}
	journal.PlanDigest = hash(raw)
	jr, _ := json.Marshal(journal)
	journalName := filepath.Join(filepath.Dir(old.FragmentJournal.Journal), "citation-legacy-journal.json")
	if err := atomicWrite(filepath.Join(s.root, journalName), jr); err != nil {
		t.Fatal(err)
	}
	old.FragmentJournal.Plan, old.FragmentJournal.PlanDigest = planName, hash(raw)
	old.FragmentJournal.Journal, old.FragmentJournal.JournalDigest = journalName, hash(jr)
	task.IndependentReview = &old
	wire, _ := json.Marshal(w)
	if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", wire, w.ID); err != nil {
		t.Fatal(err)
	}
	changed, method, err := s.fragmentReviewMethodChanged(&old)
	if err != nil || !changed || method != old.FragmentJournal.WorkflowDigest {
		t.Fatal("protocol drift lost original method", changed, err)
	}
	passed := old
	passed.State = "passed"
	if changed, _, err := s.fragmentReviewMethodChanged(&passed); err != nil || changed {
		t.Fatal("completed historical opinion invalidated", err)
	}
	preview, err := s.reviewRecoveryPreview(w.ID, a.TaskID)
	if err != nil || preview.State != "method_restart_ready" || preview.Reusable != 0 {
		t.Fatal(preview, err)
	}
	request := PlanningRequest{Schema: 1, Revision: w.Revision, EventID: "citation-wire-restart", Task: a.TaskID, ConfirmReviewErrorRepair: true, Reason: "Corrected quote-only output contract; fresh full review, historical opinions and consumed calls retained."}
	after, err := s.planningChange(w.ID, "retry-review", request)
	if err != nil {
		t.Fatal(err)
	}
	actual, _ := after.task(a.TaskID)
	if actual.IndependentReview != nil || len(actual.PreviousReviews) == 0 || actual.PreviousReviews[len(actual.PreviousReviews)-1].FragmentJournal.PlanDigest != old.FragmentJournal.PlanDigest || after.Planning.Reviewer.Calls != w.Planning.Reviewer.Calls || len(actual.Attempts) != len(task.Attempts) {
		t.Fatal("restart lost history or spent an inspection")
	}
	for name, digest := range map[string]string{planName: old.FragmentJournal.PlanDigest, journalName: old.FragmentJournal.JournalDigest} {
		data, err := os.ReadFile(filepath.Join(s.root, name))
		if err != nil || hash(data) != digest {
			t.Fatal("original changed", name, err)
		}
	}
}
