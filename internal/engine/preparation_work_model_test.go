package engine

import (
	"encoding/json"
	"strings"
	"testing"
)

func TestPreparationWorkModelProjectionAndBoundaries(t *testing.T) {
	m, err := preparationTemplate("client-portal")
	if err != nil {
		t.Fatal(err)
	}
	if m.WorkModel == nil || len(m.WorkModel.Increments) != 5 {
		t.Fatal("missing portal work model")
	}
	for _, lang := range []string{"fr", "en"} {
		for _, inc := range m.WorkModel.Increments {
			for _, intervention := range []string{"create", "improve", "correct", "migrate"} {
				r := PreparationTemplateAnswers{TemplateID: m.ID, Language: lang, Increment: inc.ID, Intervention: intervention, Answers: map[string]string{}}
				for _, q := range m.Questions {
					r.Answers[q.ID] = "User-owned answer " + q.ID
				}
				v, err := checkPreparationTemplate(r)
				if err != nil {
					t.Fatal(lang, inc.ID, intervention, err)
				}
				if !v.NeedComplete || v.LaunchAuthorized || v.WorkPlan == nil || v.Increment != inc.ID {
					t.Fatal("projection or authority broken")
				}
				if _, err := validateActionPlan(*v.WorkPlan, true); err == nil {
					t.Fatal("unresolved authority decisions must block adoption")
				}
				if intervention != "create" && v.WorkPlan.Product.Mode != "existing" {
					t.Fatal("existing application rebuilt implicitly")
				}
				b, _ := json.Marshal(v.WorkPlan)
				if _, err := parseActionPlan(string(b)); err != nil {
					t.Fatal(err)
				}
				if inc.ID != "foundations" {
					by := map[string]PlanMission{}
					for _, task := range v.WorkPlan.Tasks {
						by[task.ID] = task
					}
					if strings.Join(by["JOIN"].Depends, ",") != "BACK,FRONT" || strings.Join(by["V10"].Depends, ",") != "JOIN" {
						t.Fatal("design/backend convergence lost")
					}
					if len(v.WorkPlan.Tasks) != 8 {
						t.Fatal("incomplete feature cycle")
					}
				}
			}
		}
	}
}

func TestPreparationWorkModelRejectsInvalidGraphAndRequest(t *testing.T) {
	m, err := preparationTemplate("client-portal")
	if err != nil {
		t.Fatal(err)
	}
	for _, r := range []PreparationTemplateAnswers{
		{TemplateID: "client-portal", Language: "fr", Increment: "unknown"},
		{TemplateID: "dashboard", Language: "en", Increment: "session"},
	} {
		if _, err := checkPreparationTemplate(r); err == nil {
			t.Fatal("invalid increment accepted")
		}
	}
	copyModel := func() *PreparationWorkModel {
		b, _ := json.Marshal(m.WorkModel)
		var copy PreparationWorkModel
		_ = json.Unmarshal(b, &copy)
		return &copy
	}
	for _, kind := range []string{"cycle", "reference", "translation"} {
		c := copyModel()
		p := c.Increments[0].Plans["fr"]
		switch kind {
		case "cycle":
			p.Tasks[0].Depends = []string{"F5"}
		case "reference":
			p.Tasks[0].Depends = []string{"absent"}
		case "translation":
			delete(c.Increments[0].Plans, "en")
		}
		c.Increments[0].Plans["fr"] = p
		if err := validatePreparationWorkModel(c); err == nil {
			t.Fatal("invalid model accepted", kind)
		}
	}
	v, err := checkPreparationTemplate(PreparationTemplateAnswers{TemplateID: m.ID, Language: "fr", Increment: "session"})
	if err != nil {
		t.Fatal(err)
	}
	if v.NeedComplete || len(v.Missing) != 8 || len(v.WorkPlan.Questions) != 13 {
		t.Fatal("missing input decisions lost")
	}
	if len(m.WorkModel.Increments[1].Plans["fr"].Questions) != 5 {
		t.Fatal("request mutated catalogue")
	}
}
