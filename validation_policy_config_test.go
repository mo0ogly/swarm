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

func validationConfigFixture(t *testing.T) (*Store, Work) {
	t.Helper()
	s := storeTest(t)
	w := createTest(t, s)
	w = applyTest(t, s, w, "task.add", Request{ID: "t1", Title: "Vérifier", Deliverable: "docs/t1.md", Criteria: []string{"tests réussis", "rendu lisible"}, Owner: "worker"})
	return s, w
}

func automaticChange(w Work, command []string) ValidationPolicyChange {
	return ValidationPolicyChange{Schema: 1, Revision: w.Revision, TaskID: "t1", Intent: "replace", Policy: &ValidationPolicy{Mode: "automatic", Controls: []ValidationControl{{ID: "tests", Command: command, Criteria: []int{1, 2}, Justification: "La suite cible les deux critères et échoue sur toute régression.", Dir: ".", Timeout: 20}}}}
}

func validationInput(t *testing.T, root string, raw []byte) string {
	t.Helper()
	path := filepath.Join(root, ".swarm", newID("validation-input-")+".json")
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

func TestValidationPolicyGuidedPreviewApplyModifyAndRemove(t *testing.T) {
	s, w := validationConfigFixture(t)
	change := automaticChange(w, []string{"go", "test", "./..."})
	preview, err := s.previewValidationPolicy(w.ID, change)
	if err != nil {
		t.Fatal(err)
	}
	if preview.Mode != "automatic" || len(preview.Criteria) != 2 || preview.Criteria[0].Review != "contrôle structuré préautorisé" || preview.Token == "" {
		t.Fatalf("aperçu incomplet : %+v", preview)
	}
	if !strings.Contains(strings.Join(preview.Warnings, " "), "IA") || !strings.Contains(strings.Join(preview.Limits, " "), "300 secondes") {
		t.Fatalf("limites ou autorité absentes : %+v", preview)
	}
	change.EventID, change.PreviewToken = "policy-apply-1", preview.Token
	w, err = s.applyValidationPolicy(w.ID, change)
	if err != nil || w.Tasks[0].ValidationPolicy == nil || w.Tasks[0].ValidationPolicy.Actor == "" {
		t.Fatalf("application absente : %+v %v", w.Tasks[0], err)
	}

	// Une modification exige son propre aperçu et invalide les décisions liées.
	w.Tasks[0].Gate = &GateRecord{}
	w.Tasks[0].AutoValidation = &AutomaticValidation{State: "accepted"}
	if raw, marshalErr := json.Marshal(w); marshalErr != nil {
		t.Fatal(marshalErr)
	} else if _, execErr := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); execErr != nil {
		t.Fatal(execErr)
	}
	modified := automaticChange(w, []string{"pytest", "-q"})
	modified.Revision = w.Revision
	second, err := s.previewValidationPolicy(w.ID, modified)
	if err != nil {
		t.Fatal(err)
	}
	modified.EventID, modified.PreviewToken = "policy-apply-2", second.Token
	w, err = s.applyValidationPolicy(w.ID, modified)
	if err != nil || w.Tasks[0].Gate != nil || w.Tasks[0].AutoValidation != nil || w.Tasks[0].ValidationPolicy.Controls[0].Command[0] != "pytest" {
		t.Fatalf("modification non maîtrisée : %+v %v", w.Tasks[0], err)
	}

	remove := ValidationPolicyChange{Schema: 1, Revision: w.Revision, TaskID: "t1", Intent: "remove"}
	removal, err := s.previewValidationPolicy(w.ID, remove)
	if err != nil || !strings.Contains(removal.Confirmation, "Retirer") {
		t.Fatalf("aperçu de retrait : %+v %v", removal, err)
	}
	remove.EventID, remove.PreviewToken = "policy-remove", removal.Token
	w, err = s.applyValidationPolicy(w.ID, remove)
	if err != nil || w.Tasks[0].ValidationPolicy != nil {
		t.Fatalf("retrait non appliqué : %+v %v", w.Tasks[0], err)
	}
}

func TestValidationPolicyPauseHumanReviewAndStalePreview(t *testing.T) {
	s, w := validationConfigFixture(t)
	if err := s.pause(w.ID, true); err != nil {
		t.Fatal(err)
	}
	change := automaticChange(w, []string{"go", "test", "./..."})
	preview, err := s.previewValidationPolicy(w.ID, change)
	if err != nil || !strings.Contains(strings.Join(preview.Warnings, " "), "pause") {
		t.Fatalf("pause absente de l’aperçu : %+v %v", preview, err)
	}
	change.PreviewToken, change.EventID = preview.Token, "stale-preview"
	w = applyTest(t, s, w, "task.update", Request{ID: "t1", Next: "révision concurrente"})
	if _, err = s.applyValidationPolicy(w.ID, change); err == nil || !strings.Contains(err.Error(), "révision") {
		t.Fatalf("aperçu périmé accepté : %v", err)
	}

	human := ValidationPolicyChange{Schema: 1, Revision: w.Revision, TaskID: "t1", Intent: "replace", Policy: &ValidationPolicy{Mode: "human"}}
	humanPreview, err := s.previewValidationPolicy(w.ID, human)
	if err != nil || humanPreview.Criteria[0].Review != "revue humaine" || !strings.Contains(strings.Join(humanPreview.Warnings, " "), "qualitatif") {
		t.Fatalf("revue qualitative non préservée : %+v %v", humanPreview, err)
	}
}

func TestValidationPolicyCLIUsesSamePreviewToken(t *testing.T) {
	s, w := validationConfigFixture(t)
	change := automaticChange(w, []string{"go", "version"})
	raw, _ := json.Marshal(change)
	input := validationInput(t, s.root, raw)
	var out, errs bytes.Buffer
	if code := run([]string{"--root", s.root, "--json", "validation", "preview", w.ID, "--task", "t1", "--input", input}, &out, &errs); code != 0 {
		t.Fatalf("preview CLI : %d %s", code, errs.String())
	}
	var preview ValidationPolicyPreview
	if err := json.Unmarshal(out.Bytes(), &preview); err != nil || preview.Token == "" {
		t.Fatalf("sortie CLI invalide : %s %v", out.String(), err)
	}
	change.PreviewToken, change.EventID = preview.Token, "cli-policy"
	raw, _ = json.Marshal(change)
	input = validationInput(t, s.root, raw)
	out.Reset()
	if code := run([]string{"--root", s.root, "--json", "validation", "apply", w.ID, "--task", "t1", "--input", input}, &out, &errs); code != 0 {
		t.Fatalf("apply CLI : %d %s", code, errs.String())
	}
}

func TestValidationPolicyWebActionUsesSharedContractAndAssets(t *testing.T) {
	s, w := validationConfigFixture(t)
	change := automaticChange(w, []string{"go", "version"})
	value, err := s.webAction(webRequest{Kind: "validation-policy-preview", Work: w.ID, Task: "t1", Revision: w.Revision, ValidationPolicy: change})
	preview, ok := value.(ValidationPolicyPreview)
	if err != nil || !ok || preview.Token == "" {
		t.Fatalf("aperçu web : %#v %v", value, err)
	}
	change.PreviewToken = preview.Token
	value, err = s.webAction(webRequest{Kind: "validation-policy-apply", Work: w.ID, Task: "t1", Revision: w.Revision, Event: "web-policy", ValidationPolicy: change})
	updated, ok := value.(Work)
	if err != nil || !ok || updated.Tasks[0].ValidationPolicy == nil {
		t.Fatalf("application web : %#v %v", value, err)
	}
	js, readErr := cockpitWeb.ReadFile("web/mission.js")
	if readErr != nil || !bytes.Contains(js, []byte("Configurer les validations")) {
		t.Fatalf("parcours web absent : %v", readErr)
	}
	js, readErr = cockpitWeb.ReadFile("web/cockpit.js")
	if readErr != nil || !bytes.Contains(js, []byte("validation-policy-preview")) || !bytes.Contains(js, []byte("validation-policy-apply")) {
		t.Fatalf("raccord web absent : %v", readErr)
	}
	css, readErr := cockpitWeb.ReadFile("web/mission.css")
	if readErr != nil || !bytes.Contains(css, []byte("--wattson-champ")) || bytes.Contains(css, []byte("#fff")) {
		t.Fatalf("surface thématique absente ou couleur littérale : %v", readErr)
	}
}
