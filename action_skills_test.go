//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func skillFixture(t *testing.T, s *Store, path, text string) ActionSkill {
	t.Helper()
	full := filepath.Join(s.root, path)
	if err := os.MkdirAll(filepath.Dir(full), 0700); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(full, []byte(text), 0600); err != nil {
		t.Fatal(err)
	}
	item, _, err := s.readActionSkill(path)
	if err != nil {
		t.Fatal(err)
	}
	return item
}
func TestActionSkillsCatalogSelectionAndFreshness(t *testing.T) {
	s := storeTest(t)
	item := skillFixture(t, s, ".claude/skills/review/SKILL.md", "---\nname: Review\ndescription: Inspect the assigned change\n---\nPROJECT_SKILL_MARKER\n")
	alias := filepath.Join(s.root, ".agents/skills")
	os.MkdirAll(filepath.Dir(alias), 0700)
	if err := os.Symlink(filepath.Join(s.root, ".claude/skills"), alias); err != nil {
		t.Fatal(err)
	}
	items, err := s.actionSkillCatalog()
	if err != nil || len(items) != 1 || items[0].Name != "Review" || items[0].Description != "Inspect the assigned change" {
		t.Fatal(items, err)
	}
	w, prompt, err := s.projectAgentWorkflow("worker")
	if err != nil {
		t.Fatal(err)
	}
	if strings.Contains(prompt, "PROJECT_SKILL_MARKER") || len(w.Skills) > 0 {
		t.Fatal("skill activated without selection")
	}
	selected, body, err := s.withActionSkills(w, prompt, []ActionSkillSelection{{item.Path, item.SHA256}})
	if err != nil {
		t.Fatal(err)
	}
	if len(selected.Skills) != 1 || !strings.Contains(body, "PROJECT_SKILL_MARKER") || selected.SHA256 == w.SHA256 {
		t.Fatal("selected skill not bound to workflow")
	}
	if err := s.actionSkillsGuard(selected.Skills); err != nil {
		t.Fatal(err)
	}
	os.WriteFile(filepath.Join(s.root, item.Path), []byte("changed"), 0600)
	if s.actionSkillsGuard(selected.Skills) == nil {
		t.Fatal("changed skill accepted before launch")
	}
	if _, _, err := s.withActionSkills(w, prompt, []ActionSkillSelection{{item.Path, item.SHA256}}); err == nil {
		t.Fatal("stale selection accepted")
	}
}
func TestActionSkillsRefuseUnsafeSourcesAndDuplicateAliases(t *testing.T) {
	s := storeTest(t)
	a := skillFixture(t, s, ".claude/skills/safe/SKILL.md", "safe")
	w, prompt, _ := s.projectAgentWorkflow("worker")
	for _, choices := range [][]ActionSkillSelection{{{a.Path, ""}}, {{a.Path, a.SHA256}, {a.Path, a.SHA256}}, {{"../outside/SKILL.md", a.SHA256}}, {{".claude/settings.json", a.SHA256}}} {
		if _, _, err := s.withActionSkills(w, prompt, choices); err == nil {
			t.Fatal("unsafe selection accepted", choices)
		}
	}
	alias := filepath.Join(s.root, ".agents/skills/safe")
	os.MkdirAll(filepath.Dir(alias), 0700)
	os.Symlink(filepath.Join(s.root, ".claude/skills/safe"), alias)
	if _, _, err := s.withActionSkills(w, prompt, []ActionSkillSelection{{a.Path, a.SHA256}, {".agents/skills/safe/SKILL.md", a.SHA256}}); err == nil {
		t.Fatal("duplicate canonical skill accepted")
	}
	for _, text := range []string{strings.Repeat("x", 16001), "sk-ant-" + strings.Repeat("a", 35), string([]byte{0, 255})} {
		os.WriteFile(filepath.Join(s.root, a.Path), []byte(text), 0600)
		if _, _, err := s.readActionSkill(a.Path); err == nil {
			t.Fatal("unsafe contents accepted")
		}
	}
	outside := t.TempDir()
	os.MkdirAll(filepath.Join(s.root, ".claude/skills/outside"), 0700)
	os.WriteFile(filepath.Join(outside, "SKILL.md"), []byte("external"), 0600)
	os.Symlink(filepath.Join(outside, "SKILL.md"), filepath.Join(s.root, ".claude/skills/outside/SKILL.md"))
	if _, _, err := s.readActionSkill(".claude/skills/outside/SKILL.md"); err == nil {
		t.Fatal("external symlink accepted")
	}
	w, prompt, _ = s.projectAgentWorkflow("reviewer")
	if _, _, err := s.withActionSkills(w, prompt, []ActionSkillSelection{{a.Path, a.SHA256}}); err == nil {
		t.Fatal("action selection expanded reviewer role")
	}
}

func TestActionSkillsPersistPerTaskAndPreventChangedProviderLaunch(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	item := skillFixture(t, s, ".claude/skills/check/SKILL.md", "TASK_ONLY_SKILL_MARKER")
	r.Skills = []ActionSkillSelection{{item.Path, item.SHA256}}
	a, created, err := s.prepare(w.ID, r)
	if err != nil || !created {
		t.Fatal(err)
	}
	if !strings.Contains(a.Prompt, "TASK_ONLY_SKILL_MARKER") || len(a.Workflow.Skills) != 1 {
		t.Fatal("selected skill absent from actual agent prompt")
	}
	current, err := s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	if current.Profile == nil || len(current.Profile.Skills) != 0 {
		t.Fatal("task skills leaked to mission defaults")
	}
	found := false
	for _, task := range current.Tasks {
		if task.ID == r.TaskID {
			found = task.Profile != nil && len(task.Profile.Skills) == 1
		}
	}
	if !found {
		t.Fatal("task profile did not preserve selected skills")
	}
	if err := os.WriteFile(filepath.Join(s.root, item.Path), []byte("changed after reservation"), 0600); err != nil {
		t.Fatal(err)
	}
	if err := s.supervise(a.ID); err != nil {
		t.Fatal(err)
	}
	stopped, err := s.agent(a.ID)
	if err != nil || stopped.Status != "failed" {
		t.Fatal(stopped, err)
	}
}

func TestActionSkillsRetryPreservesOrExplicitlyClearsSelection(t *testing.T) {
	for _, clear := range []bool{false, true} {
		t.Run(map[bool]string{false: "preserve", true: "clear"}[clear], func(t *testing.T) {
			s := storeTest(t)
			w, r := setupAgent(t, s)
			item := skillFixture(t, s, ".claude/skills/check/SKILL.md", "RETRY_SKILL_MARKER")
			r.Skills = []ActionSkillSelection{{item.Path, item.SHA256}}
			a, _, err := s.prepare(w.ID, r)
			if err != nil {
				t.Fatal(err)
			}
			if err = s.reconcile(a.ID); err != nil {
				t.Fatal(err)
			}
			w, err = s.get(w.ID)
			if err != nil {
				t.Fatal(err)
			}
			r.Previous = a.ID
			r.Revision = w.Revision
			r.EventID = newID("agent-")
			r.Skills = nil
			if clear {
				r.Skills = []ActionSkillSelection{}
			}
			next, _, err := s.prepare(w.ID, r)
			if err != nil {
				t.Fatal(err)
			}
			if clear && (len(next.Workflow.Skills) != 0 || strings.Contains(next.Prompt, "RETRY_SKILL_MARKER")) {
				t.Fatal("cleared skill retained")
			}
			if !clear && (len(next.Workflow.Skills) != 1 || !strings.Contains(next.Prompt, "RETRY_SKILL_MARKER")) {
				t.Fatal("omitted selection did not preserve skill")
			}
		})
	}
}
