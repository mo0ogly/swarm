//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func fragmentPlanFixture() managedReviewContext {
	c, _ := hunkPacketFixture()
	c.Diff = ""
	for i := 0; i < 20; i++ {
		c.Diff += fmt.Sprintf("diff --git a/f%d b/f%d\n", i, i) + strings.Repeat("+évidence\n", 3000)
	}
	return c
}
func TestManagedFragmentCompleteEvidence(t *testing.T) {
	c := fragmentPlanFixture()
	before, _ := json.Marshal(c)
	p, e := planManagedReviewFragments(c, 12, 2)
	if e != nil {
		t.Fatal(e)
	}
	if p.Executable || len(p.Packets) < 2 {
		t.Fatal("not an execution or trivial fixture")
	}
	again, e := planManagedReviewFragments(c, 12, 2)
	if e != nil || !reflect.DeepEqual(p, again) {
		t.Fatal("nondeterministic", e)
	}
	after, _ := json.Marshal(c)
	if string(before) != string(after) {
		t.Fatal("canonical evidence changed")
	}
	var rebuilt strings.Builder
	kinds := map[string]int{}
	for _, packet := range p.Packets {
		for _, artifact := range packet.Artifacts {
			kinds[artifact.Kind]++
			if artifact.Kind == "diff" {
				rebuilt.WriteString(artifact.Content)
			}
		}
	}
	if rebuilt.String() != c.Diff || kinds["context"] != 1 || kinds["source"] != len(c.Sources) {
		t.Fatal("evidence omitted")
	}
	if e = validateManagedReviewFragments(c, p); e != nil {
		t.Fatal(e)
	}
}
func TestManagedFragmentRejectsTampering(t *testing.T) {
	for _, kind := range []string{"missing", "duplicate", "content", "candidate", "report", "source", "receipt", "order", "executable", "budget"} {
		t.Run(kind, func(t *testing.T) {
			c := fragmentPlanFixture()
			p, e := planManagedReviewFragments(c, 12, 2)
			if e != nil {
				t.Fatal(e)
			}
			switch kind {
			case "missing":
				p.Packets = p.Packets[:len(p.Packets)-1]
			case "duplicate":
				p.Packets = append(p.Packets, p.Packets[0])
			case "content":
				p.Packets[0].Artifacts[0].Content += "changed"
			case "candidate":
				c.Candidate = "different"
			case "report":
				c.Tasks[0].Report += "changed"
			case "source":
				c.Sources[0].Content += "changed"
			case "receipt":
				c.Receipt = json.RawMessage(`{"changed":true}`)
			case "order":
				p.Packets[0], p.Packets[1] = p.Packets[1], p.Packets[0]
			case "executable":
				p.Executable = true
			case "budget":
				p.AvailableCalls = 1
			}
			if validateManagedReviewFragments(c, p) == nil {
				t.Fatal("tamper accepted")
			}
		})
	}
}
func TestManagedFragmentRejectsUnboundedEvidence(t *testing.T) {
	c := fragmentPlanFixture()
	if _, e := planManagedReviewFragments(c, 3, 2); e == nil {
		t.Fatal("budget")
	}
	c.Tasks[0].Report = strings.Repeat("x", managedReviewPromptLimit)
	if _, e := planManagedReviewFragments(c, 12, 2); e == nil {
		t.Fatal("oversized report")
	}
	c = fragmentPlanFixture()
	c.Diff += "diff --git a/b b/b\nGIT binary patch\n"
	if _, e := planManagedReviewFragments(c, 12, 2); e == nil {
		t.Fatal("binary")
	}
}

func TestManagedFragmentPreviewReadOnly(t *testing.T) {
	s, w, a := unpaidReviewFixture(t)
	before, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	itemBefore, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	p, e := s.previewManagedFragments(w.ID, a.TaskID)
	if e != nil {
		t.Fatal(e)
	}
	if p.Executable || p.ContextDigest == "" || len(p.Packets) == 0 {
		t.Fatal("invalid preview")
	}
	request := filepath.Join(t.TempDir(), "request.json")
	raw, _ := json.Marshal(map[string]string{"task_id": a.TaskID})
	if e = os.WriteFile(request, raw, 0600); e != nil {
		t.Fatal(e)
	}
	var out bytes.Buffer
	if e = s.planningCLI([]string{"planning", "fragment-preview", w.ID}, request, &out); e != nil {
		t.Fatal(e)
	}
	var viaCLI managedReviewFragmentPlan
	if e = json.Unmarshal(out.Bytes(), &viaCLI); e != nil || !reflect.DeepEqual(p, viaCLI) {
		t.Fatal("CLI changed plan", e)
	}
	mux := http.NewServeMux()
	s.registerPlanning(mux, func(w http.ResponseWriter, v any) { _ = json.NewEncoder(w).Encode(v) }, func(w http.ResponseWriter, e error) { http.Error(w, e.Error(), 400) })
	rec := httptest.NewRecorder()
	mux.ServeHTTP(rec, httptest.NewRequest("GET", "/api/v1/planning?work="+w.ID+"&task="+a.TaskID+"&action=fragment-preview", nil))
	var viaAPI managedReviewFragmentPlan
	if rec.Code != 200 {
		t.Fatal("preview HTTP", rec.Code, rec.Body.String())
	}
	if e = json.Unmarshal(rec.Body.Bytes(), &viaAPI); e != nil || !reflect.DeepEqual(p, viaAPI) {
		t.Fatal("API changed plan", e)
	}
	after, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	itemAfter, e := s.managedAttempt(a.ID)
	if e != nil {
		t.Fatal(e)
	}
	if !reflect.DeepEqual(before, after) || !reflect.DeepEqual(itemBefore, itemAfter) {
		t.Fatal("preview mutated runtime")
	}
	if _, e = s.previewManagedFragments(w.ID, "unknown"); e == nil {
		t.Fatal("unknown task")
	}
}
