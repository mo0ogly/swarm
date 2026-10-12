//go:build linux

package engine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func preparedChangedSkill(t *testing.T) (*Store, Work, Launch, ActionSkill, []byte) {
	t.Helper()
	s, w, _, r := managedRejectedRetry(t)
	old := skillFixture(t, s, ".claude/skills/check/SKILL.md", "ORIGINAL_PREPARED_SKILL")
	r.Skills = []ActionSkillSelection{{old.Path, old.SHA256}}
	if _, err := s.ensureManagedAttempt(w, r); err != nil {
		t.Fatal(err)
	}
	manifest, err := os.ReadFile(filepath.Join(w.Planning.Repository.Storage, "copy-"+r.EventID+".json"))
	if err != nil {
		t.Fatal(err)
	}
	next := skillFixture(t, s, old.Path, "CURRENT_PREPARED_SKILL")
	return s, w, r, next, manifest
}

func TestPreparedSkillRefreshPreservesCopyAndFrozenSettings(t *testing.T) {
	s, w, r, next, original := preparedChangedSkill(t)
	p, err := s.preparedLaunchForTask(w, &w.Tasks[0])
	if err != nil || p == nil || p.Ready || p.Reason == "" {
		t.Fatal("stale skill advertised as ready", p, err)
	}
	if _, created, err := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision); err == nil || created {
		t.Fatal("ordinary resume silently adopted a changed skill")
	}
	choice := PreparedSkillRefresh{Confirm: true, Reason: "Reviewed the updated method before resuming the retained launch", Skills: []ActionSkillSelection{{next.Path, next.SHA256}}}
	a, created, err := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision, choice)
	if err != nil || !created {
		t.Fatal("explicit skill refresh failed", created, err)
	}
	if !strings.Contains(a.Prompt, "CURRENT_PREPARED_SKILL") || strings.Contains(a.Prompt, "ORIGINAL_PREPARED_SKILL") || a.Provider != r.Provider || a.Timeout != r.Timeout || a.Role != r.Role {
		t.Fatal("wrong actual prompt or changed frozen settings")
	}
	manifest, err := os.ReadFile(filepath.Join(w.Planning.Repository.Storage, "copy-"+r.EventID+".json"))
	if err != nil || hash(manifest) != hash(original) {
		t.Fatal("original preparation rewritten", err)
	}
	var saved []byte
	if err = s.db.QueryRow("SELECT request FROM agents WHERE id=?", a.ID).Scan(&saved); err != nil {
		t.Fatal(err)
	}
	var actual Launch
	if err = json.Unmarshal(saved, &actual); err != nil || actual.SkillRefresh == nil || actual.SkillRefresh.Prepared != r.EventID || actual.SkillRefresh.Original != preparedRequestDigest(r) || actual.SkillRefresh.Previous[0].SHA256 != r.Skills[0].SHA256 || actual.Skills[0].SHA256 != next.SHA256 || actual.SkillRefresh.Reason != choice.Reason {
		t.Fatal("lost recorded refresh attribution", err)
	}
	replay, duplicate, err := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision, choice)
	if err != nil || duplicate || replay.ID != a.ID {
		t.Fatal("duplicate worker reservation", err)
	}
	after, _ := s.get(w.ID)
	if len(after.Tasks[0].Attempts) != 2 || after.Tasks[0].PlanMaxAttempts != w.Tasks[0].PlanMaxAttempts || managedReviewCalls(t, s) != 1 || after.Planning.Repository.Candidate != w.Planning.Repository.Candidate {
		t.Fatal("refresh changed calls, canonical candidate or limits")
	}
}

func TestPreparedSkillRefreshRequiresExactExplicitSelection(t *testing.T) {
	for _, mode := range []string{"no-confirm", "no-reason", "wrong-hash", "different-path", "empty", "forged-receipt", "changed-provider", "raised-timeout"} {
		t.Run(mode, func(t *testing.T) {
			s, w, r, next, original := preparedChangedSkill(t)
			choice := PreparedSkillRefresh{Confirm: true, Reason: "Explicitly adopt the reviewed current skill", Skills: []ActionSkillSelection{{next.Path, next.SHA256}}}
			switch mode {
			case "no-confirm":
				choice.Confirm = false
			case "no-reason":
				choice.Reason = ""
			case "wrong-hash":
				choice.Skills[0].SHA256 = r.Skills[0].SHA256
			case "different-path":
				choice.Skills[0].Path = ".claude/skills/other/SKILL.md"
			case "empty":
				choice.Skills = nil
			case "forged-receipt":
				r.SkillRefresh = &PreparedSkillRefreshReceipt{Prepared: r.EventID, Reason: choice.Reason}
				if _, created, err := s.prepare(w.ID, r); err == nil || created {
					t.Fatal("client forged engine refresh authority")
				}
			case "changed-provider", "raised-timeout":
				requested, err := s.refreshPreparedSkills(r, choice)
				if err != nil {
					t.Fatal(err)
				}
				if mode == "changed-provider" {
					requested.Provider = "another-provider"
				} else {
					requested.Timeout++
				}
				if preparedRequestCompatible(r, requested) {
					t.Fatal("skill refresh expanded frozen launch settings")
				}
			}
			if mode != "forged-receipt" && mode != "changed-provider" && mode != "raised-timeout" {
				if _, created, err := s.resumePreparedLaunch(w.ID, r.EventID, w.Revision, choice); err == nil || created {
					t.Fatal("invalid selection launched a worker")
				}
			}
			after, _ := s.get(w.ID)
			manifest, err := os.ReadFile(filepath.Join(w.Planning.Repository.Storage, "copy-"+r.EventID+".json"))
			if err != nil || hash(manifest) != hash(original) || after.Revision != w.Revision || len(after.Tasks[0].Attempts) != 1 || managedReviewCalls(t, s) != 1 {
				t.Fatal("invalid refresh changed historical evidence or counters")
			}
		})
	}
}
