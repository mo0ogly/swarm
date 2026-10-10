package engine

import (
	"strings"
	"testing"
)

func TestPreparationSubjectInterventionsAreBoundedAndPreserveAnswers(t *testing.T) {
	for _, id := range []string{"internal-tool", "client-portal", "service-api", "inventory", "data-pipeline", "dashboard"} {
		model, err := preparationTemplate(id)
		if err != nil {
			t.Fatal(id, err)
		}
		for _, lang := range []string{"fr", "en"} {
			for _, intervention := range []string{"create", "improve", "correct", "migrate"} {
				req := PreparationTemplateAnswers{TemplateID: id, Language: lang, Answers: map[string]string{}}
				for _, q := range model.Questions {
					req.Answers[q.ID] = "User answer " + q.ID
				}
				// Exercise the public input contract rather than frontend-only composition.
				requestJSON := `{"template_id":"` + id + `","language":"` + lang + `","intervention":"` + intervention + `","answers":{}}`
				if err := strict([]byte(requestJSON), &req); err != nil {
					t.Fatal(err)
				}
				for _, q := range model.Questions {
					req.Answers[q.ID] = "User answer " + q.ID
				}
				result, err := checkPreparationTemplate(req)
				if err != nil {
					t.Fatal(id, lang, intervention, err)
				}
				if !result.NeedComplete || result.LaunchAuthorized {
					t.Fatal("completeness became launch authorization")
				}
				for _, q := range model.Questions {
					if !strings.Contains(result.Need, req.Answers[q.ID]) {
						t.Fatal("answer lost", q.ID)
					}
				}
				if !strings.Contains(result.Need, "5") || !strings.Contains(result.Need, "6") {
					t.Fatal("missing shared and repeated cycle")
				}
			}
		}
	}
}

func TestPreparationSubjectsRejectUnknownInterventionAndRetainLegacyContract(t *testing.T) {
	for _, req := range []PreparationTemplateAnswers{
		{TemplateID: "service-api", Language: "en", Intervention: "launch"},
		{TemplateID: "service-api", Language: "en", Intervention: "CREATE"},
		{TemplateID: "application", Language: "en", Intervention: "create"},
	} {
		if _, err := checkPreparationTemplate(req); err == nil {
			t.Fatal("invalid intervention accepted", req)
		}
	}
	for _, id := range []string{"internal-tool", "client-portal", "service-api", "inventory", "data-pipeline", "dashboard"} {
		model, err := preparationTemplate(id)
		if err != nil {
			t.Fatal(err)
		}
		if model.Subject == nil || len(model.Subject.Interventions) != 4 {
			t.Fatal("missing subject contract")
		}
		for _, lang := range []string{"fr", "en"} {
			if model.Subject.Outcome[lang] == "" || model.Subject.Example[lang] == "" || len(model.Subject.Evidence[lang]) == 0 || len(model.Subject.Deliverables[lang]) == 0 || len(model.Subject.Risks[lang]) == 0 {
				t.Fatal("missing localized subject", id, lang)
			}
			req := PreparationTemplateAnswers{TemplateID: id, Language: lang, Intervention: "correct"}
			v, err := checkPreparationTemplate(req)
			if err != nil {
				t.Fatal(err)
			}
			if v.RecommendedMethod != "debug" || v.NeedComplete || v.LaunchAuthorized || len(v.Missing) != 8 {
				t.Fatal("correction boundaries lost", v)
			}
			req.Intervention = "migrate"
			v, err = checkPreparationTemplate(req)
			if err != nil {
				t.Fatal(err)
			}
			if v.RecommendedMethod != "ks-product" || v.LaunchAuthorized {
				t.Fatal("migration boundaries lost")
			}
			req.Intervention = ""
			v, err = checkPreparationTemplate(req)
			if err != nil || v.Intervention != "create" {
				t.Fatal("default intervention missing", err)
			}
		}
	}
}
