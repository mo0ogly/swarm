package engine

import (
	"bytes"
	"encoding/json"
	"os"
	"strings"
	"testing"
)

func TestPreparationTemplatesCLIIsReadOnlyAndRejectsUnknown(t *testing.T) {
	s := &Store{}
	var out bytes.Buffer
	if err := s.preparationCLI([]string{"template", "missing"}, "", "", &out); err == nil {
		t.Fatal("unknown template accepted")
	}
	if err := s.preparationCLI([]string{"templates", "unexpected"}, "", "", &out); err == nil {
		t.Fatal("unexpected arguments accepted")
	}
	if err := s.preparationCLI([]string{"templates"}, "", "", &out); err != nil {
		t.Fatal(err)
	}
	var ts []PreparationTemplate
	if err := json.Unmarshal(out.Bytes(), &ts); err != nil {
		t.Fatal(err)
	}
	ids := map[string]bool{}
	for _, v := range ts {
		if ids[v.ID] || v.Version != 1 {
			t.Fatal("invalid catalogue identity", v.ID)
		}
		ids[v.ID] = true
		for _, lang := range []string{"fr", "en"} {
			if len(v.Need[lang]) < 300 || len(v.Need[lang]) > 16000 || v.Title[lang] == "" {
				t.Fatal("invalid localized draft", v.ID, lang)
			}
		}
		if (v.ID == "correction" && v.RecommendedMethod != "debug") || (v.ID != "correction" && v.RecommendedMethod != "ks-product") {
			t.Fatal("unsupported method")
		}
		out.Reset()
		if err := s.preparationCLI([]string{"template", v.ID}, "", "", &out); err != nil {
			t.Fatal(err)
		}
		if !strings.Contains(out.String(), v.ID) {
			t.Fatal("missing CLI template")
		}
	}
	if len(ts) != 4 {
		t.Fatal("missing starter workflows")
	}
}

func TestPreparationTemplateGuidanceCompletenessAndBoundaries(t *testing.T) {
	templates, err := preparationTemplates()
	if err != nil {
		t.Fatal(err)
	}
	for _, template := range templates {
		for _, lang := range []string{"fr", "en"} {
			r := PreparationTemplateAnswers{TemplateID: template.ID, Language: lang, Answers: map[string]string{}}
			empty, err := checkPreparationTemplate(r)
			if err != nil {
				t.Fatal(err)
			}
			if empty.NeedComplete || empty.LaunchAuthorized || len(empty.Missing) != 8 || len(empty.Team) != 3 {
				t.Fatalf("invalid empty projection: %+v", empty)
			}
			roles := map[string]bool{}
			for _, role := range empty.Team {
				roles[role.Role] = true
			}
			if !roles["planner"] || !roles["worker"] || !roles["reviewer"] {
				t.Fatal("missing role")
			}
			for _, q := range template.Questions {
				r.Answers[q.ID] = "Answer for " + q.ID
			}
			complete, err := checkPreparationTemplate(r)
			if err != nil {
				t.Fatal(err)
			}
			if !complete.NeedComplete || complete.LaunchAuthorized || complete.Answered != 8 || len(complete.Missing) != 0 {
				t.Fatalf("invalid completed projection: %+v", complete)
			}
			for _, q := range template.Questions {
				if !strings.Contains(complete.Need, r.Answers[q.ID]) {
					t.Fatal("answer not projected", template.ID, lang, q.ID)
				}
			}
			if strings.Contains(complete.Need, "[à préciser") || strings.Contains(complete.Need, "[define") || strings.Contains(complete.Need, "[describe") {
				t.Fatal("unfilled placeholder", template.ID, lang)
			}
			r.Answers["scope"] = "   "
			r.Answers["acceptance"] = "[à préciser]"
			partial, err := checkPreparationTemplate(r)
			if err != nil {
				t.Fatal(err)
			}
			if partial.NeedComplete || len(partial.Missing) != 2 {
				t.Fatal("blank/placeholders considered complete")
			}
			r.Answers["surprise"] = "unknown"
			if _, err = checkPreparationTemplate(r); err == nil {
				t.Fatal("unknown key accepted")
			}
			delete(r.Answers, "surprise")
			r.Answers["scope"] = strings.Repeat("é", 501)
			if _, err = checkPreparationTemplate(r); err == nil {
				t.Fatal("byte limit ignored")
			}
		}
	}
	if _, err = checkPreparationTemplate(PreparationTemplateAnswers{TemplateID: "ks-feature", Language: "xx"}); err == nil {
		t.Fatal("invalid language")
	}
	s := &Store{}
	var out bytes.Buffer
	file := t.TempDir() + "/answers.json"
	if err = os.WriteFile(file, []byte(`{"language":"en","answers":{"objective":"User outcome"}}`), 0600); err != nil {
		t.Fatal(err)
	}
	if err = s.preparationCLI([]string{"template-check", "ks-feature"}, file, "", &out); err != nil {
		t.Fatal(err)
	}
	var checked PreparationTemplateCheck
	if err = json.Unmarshal(out.Bytes(), &checked); err != nil {
		t.Fatal(err)
	}
	if checked.Answered != 1 || checked.NeedComplete || checked.LaunchAuthorized {
		t.Fatal("CLI differs from engine")
	}
}

func TestPreparationTemplateMethodsMatchActualAgentFraming(t *testing.T) {
	templates, err := preparationTemplates()
	if err != nil {
		t.Fatal(err)
	}
	for _, template := range templates {
		for _, role := range template.Team {
			want, _, err := agentWorkflow(role.Role)
			if err != nil {
				t.Fatal(err)
			}
			if role.Workflow == nil || role.Workflow.SHA256 != want.SHA256 || role.Workflow.Role != want.Role || strings.Join(role.Workflow.Methods, ",") != strings.Join(want.Methods, ",") {
				t.Fatal("template diverges from launch framing", template.ID, role.Role)
			}
			for _, lang := range []string{"fr", "en"} {
				if role.MethodGuidance[lang] == "" || !strings.Contains(template.Need[lang], role.MethodGuidance[lang]) {
					t.Fatal("role methods not carried into draft", template.ID, role.Role, lang)
				}
			}
			if role.Role == "reviewer" && containsString(role.Workflow.Methods, "apex") {
				t.Fatal("reviewer received implementation methods")
			}
		}
	}
}

func TestPreparationTemplateReadableIDsAndLegacyLookup(t *testing.T) {
	for old, current := range map[string]string{"ks-feature": "application", "ks-bugfix": "correction", "ks-interface": "interface", "ks-product": "product"} {
		template, err := preparationTemplate(old)
		if err != nil {
			t.Fatal(err)
		}
		if template.ID != current || (current == "correction" && template.RecommendedMethod != "debug") || (current != "correction" && template.RecommendedMethod != "ks-product") {
			t.Fatal("legacy lookup does not return current template", old)
		}
		for _, lang := range []string{"fr", "en"} {
			if strings.Contains(template.Need[lang], "ks-") || strings.Contains(template.Need[lang], "KS") || strings.Contains(template.Need[lang], "APEX") || strings.Contains(template.Need[lang], "PDCA") {
				t.Fatal("technical names remain in draft", current, lang)
			}
		}
	}
}
