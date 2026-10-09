//go:build linux

package engine

import (
	"encoding/json"
	"reflect"
	"strings"
	"testing"
)

func fixtureProductPlan() ActionPlan {
	p := fixturePlan()
	p.Tasks[0].Phase = "research"
	p.Tasks[1].Phase = "implement"
	p.Product = &ProductPlan{Mode: "existing", Journeys: []UserJourney{
		{ID: "order", Title: "Commander", Goal: "Recevoir sa commande", StoryIDs: []string{"s01-order", "s02-history"}},
		{ID: "account", Title: "Compte", Goal: "Retrouver ses achats", StoryIDs: []string{"s02-history"}},
	}, Stories: []UserStory{
		{ID: "s01-order", Title: "Passer commande", User: "Client", Value: "Acheter une pizza", Criteria: []string{"Commande persistée et confirmée"}, Complexity: 3, Depends: []string{}, TaskIDs: []string{"T1", "T2"}},
		{ID: "s02-history", Title: "Historique", User: "Client", Value: "Retrouver ses achats", Criteria: []string{"Achats du client seulement"}, Complexity: 2, Depends: []string{"s01-order"}, TaskIDs: []string{"T1"}},
	}}
	return p
}

func TestProductPlanReferencesAndCompatibility(t *testing.T) {
	for _, p := range []ActionPlan{fixturePlan(), fixtureProductPlan()} {
		raw, _ := json.Marshal(p)
		if _, err := parseActionPlan(string(raw)); err != nil {
			t.Fatal(err)
		}
	}
	cases := map[string]func(*ActionPlan){
		"mode":               func(p *ActionPlan) { p.Product.Mode = "imaginary" },
		"story-id":           func(p *ActionPlan) { p.Product.Stories[0].ID = "backend" },
		"duplicate-story":    func(p *ActionPlan) { p.Product.Stories[1].ID = p.Product.Stories[0].ID },
		"unknown-task":       func(p *ActionPlan) { p.Product.Stories[0].TaskIDs = []string{"missing"} },
		"duplicate-task":     func(p *ActionPlan) { p.Product.Stories[0].TaskIDs = []string{"T1", "T1"} },
		"unknown-story":      func(p *ActionPlan) { p.Product.Journeys[0].StoryIDs = []string{"missing"} },
		"duplicate-journey":  func(p *ActionPlan) { p.Product.Journeys[1].ID = "order" },
		"uncovered-story":    func(p *ActionPlan) { p.Product.Journeys = p.Product.Journeys[1:] },
		"unknown-dependency": func(p *ActionPlan) { p.Product.Stories[0].Depends = []string{"missing"} },
		"cycle":              func(p *ActionPlan) { p.Product.Stories[0].Depends = []string{"s02-history"} },
		"self-cycle":         func(p *ActionPlan) { p.Product.Stories[0].Depends = []string{"s01-order"} },
		"complexity-five":    func(p *ActionPlan) { p.Product.Stories[0].Complexity = 5 },
		"missing-risk":       func(p *ActionPlan) { p.Product.Stories[0].Complexity = 4 },
		"missing-criteria":   func(p *ActionPlan) { p.Product.Stories[0].Criteria = []string{} },
		"empty-criterion":    func(p *ActionPlan) { p.Product.Stories[0].Criteria = []string{" "} },
		"phase":              func(p *ActionPlan) { p.Tasks[0].Phase = "auto-accept" },
	}
	for name, edit := range cases {
		t.Run(name, func(t *testing.T) {
			p := fixtureProductPlan()
			edit(&p)
			if _, err := validateActionPlan(p, true); err == nil {
				t.Fatal("invalid product accepted")
			}
		})
	}
	p := fixtureProductPlan()
	p.Product.Stories[1].TaskIDs = []string{}
	if _, err := validateActionPlan(p, true); err != nil {
		t.Fatal("unplanned story should be preserved", err)
	}
}

func TestProductAdoptedPlanPersistsMappingsAndWorkerContext(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	w = seedPlan(t, s, w, fixtureProductPlan())
	review, err := s.readPlan(w.ID, "plan-source")
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.commitPlan(w.ID, newID("e-"), w.Revision, review)
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	w, err = reopened.get(w.ID)
	if err != nil || !reflect.DeepEqual(w.Plans[0].Spec.Product, review.Spec.Product) || len(w.Plans[0].TaskMap) != 2 {
		t.Fatal("product/map lost on reopen", err)
	}
	id := w.Plans[0].TaskMap["T1"]
	r.TaskID, r.Revision, r.EventID = id, w.Revision, newID("a-")
	a, _, err := reopened.prepare(w.ID, r)
	if err != nil {
		t.Fatal(err)
	}
	for _, fragment := range []string{"s01-order", "s02-history", "Commande persistée et confirmée", "Étape produit : research", "KS product delivery"} {
		if !strings.Contains(a.Prompt, fragment) {
			t.Fatalf("worker did not receive %s", fragment)
		}
	}
	if a.Limits.MaxToolCalls != 5 || a.Role != "worker" {
		t.Fatal("product raised limits or changed role")
	}
}

func TestProductStoryRevisionInvalidatesSharedTasksAndDependents(t *testing.T) {
	old := fixtureProductPlan()
	next := fixtureProductPlan()
	next.Product.Stories[1].Criteria = []string{"Historique après redémarrage"}
	changes := preparationPlanChanges(old, next)
	if len(changes) != 2 || changes[0].Kind != "modifiée" || changes[1].Kind != "à revérifier (dépendance modifiée)" {
		t.Fatalf("shared contract was silently presented as unchanged: %+v", changes)
	}
	next = fixtureProductPlan()
	next.Product.Journeys[0].Title = "Nouveau libellé"
	for _, change := range preparationPlanChanges(old, next) {
		if change.Kind != "conservée" {
			t.Fatal("presentation rename changed execution evidence")
		}
	}
}

func TestProductRoleMethodsAreCompleteAndEmbedded(t *testing.T) {
	s := storeTest(t)
	r := prepRequest(prepCreate(t, s), "method")
	r.Method = "ks-product"
	if _, err := s.preparationCommand(r); err != nil {
		t.Fatal("product method cannot be selected", err)
	}
	zero := 0
	if _, err := s.preparationCommand(PreparationRequest{Version: 1, Event: newID("e-"), Action: "create", Revision: &zero, Method: "ks-product", Title: "Product", Text: "Scoped fixture"}); err != nil {
		t.Fatal("product template cannot create preparation", err)
	}
	for role, method := range map[string]string{"planner": "product-planning", "subplanner": "product-planning", "worker": "product-delivery", "reviewer": "product-review"} {
		w, prompt, err := agentWorkflow(role)
		if err != nil || !containsString(w.Methods, method) {
			t.Fatal(role, err)
		}
		path := ".claude/skills/" + method + "/SKILL.md"
		source, err := agentWorkflowFiles.ReadFile(path)
		if err != nil || !strings.Contains(prompt, string(source)) {
			t.Fatal("incomplete role method", path, err)
		}
	}
	for _, file := range []string{"prd", "stories", "stories-review-checklist", "architecture", "design-system", "design-screen", "design-brief", "research", "plan", "review-checklist", "adr"} {
		if b, err := agentWorkflowFiles.ReadFile("tools/agent-workflows/templates/ks/" + file + ".md"); err != nil || len(b) == 0 {
			t.Fatal("missing shipped template", file, err)
		}
	}
}

func TestProductPreparationRejectsMissingStructureWithoutLosingDraft(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	r := prepRequest(p, "method")
	r.Method = "ks-product"
	p, err := s.preparationCommand(r)
	if err != nil {
		t.Fatal(err)
	}
	p = prepAdopt(t, s, p)
	raw, _ := json.Marshal(fixturePlan())
	p = prepSave(t, s, p, "plan", string(raw))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	if _, err = s.preparationCommand(r); err == nil || !strings.Contains(err.Error(), "structure product") {
		t.Fatal("product method accepted flat plan", err)
	}
	kept, _ := s.preparation(p.ID)
	if kept.PlanReady || kept.Documents["plan"].Text != string(raw) || kept.Revision != p.Revision {
		t.Fatal("rejected check lost draft or created readiness")
	}
	raw, _ = json.Marshal(fixtureProductPlan())
	p = prepSave(t, s, p, "plan", string(raw))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	if _, err = s.preparationCommand(r); err != nil {
		t.Fatal(err)
	}
}

func TestProductRevisionPersistsSharedContextAndInvalidatesHistoricalState(t *testing.T) {
	s := storeTest(t)
	p := prepCreate(t, s)
	r := prepRequest(p, "method")
	r.Method = "ks-product"
	var err error
	p, err = s.preparationCommand(r)
	if err != nil {
		t.Fatal(err)
	}
	p = prepAdopt(t, s, p)
	spec := fixtureProductPlan()
	raw, _ := json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(raw))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, err = s.preparationCommand(r)
	if err != nil {
		t.Fatal(err)
	}
	p, err = s.preparationCommand(conversionRequest(t, s, p, "create-missions"))
	if err != nil {
		t.Fatal(err)
	}
	w, _ := s.get(p.WorkID)
	mapping := w.Plans[0].TaskMap
	// Isolated legacy fixture: raw accepted flags are historical, not fresh proof.
	for i := range w.Tasks {
		w.Tasks[i].Status = "accepted"
		w.Tasks[i].Response = "historical response"
	}
	raw, _ = json.Marshal(w)
	if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, w.ID); err != nil {
		t.Fatal(err)
	}
	spec.Product.Stories[1].Criteria = []string{"Historique après redémarrage"}
	raw, _ = json.Marshal(spec)
	p = prepSave(t, s, p, "plan", string(raw))
	r = prepRequest(p, "validate-plan")
	r.Hash = p.Documents["plan"].Hash
	p, err = s.preparationCommand(r)
	if err != nil {
		t.Fatal(err)
	}
	r = conversionRequest(t, s, p, "revise-missions")
	p, err = s.preparationCommand(r)
	if err != nil {
		t.Fatal(err)
	}
	w, err = s.get(p.WorkID)
	if err != nil {
		t.Fatal(err)
	}
	if len(w.Plans) != 2 || !reflect.DeepEqual(w.Plans[1].TaskMap, mapping) {
		t.Fatal("revision lost canonical IDs", w.Plans)
	}
	for _, task := range w.Tasks {
		if task.Status != "todo" || !task.LaunchHeld || task.Response != "historical response" || task.PlanMaxAttempts != map[string]int{mapping["T1"]: spec.Tasks[0].MaxAttempts, mapping["T2"]: spec.Tasks[1].MaxAttempts}[task.ID] {
			t.Fatal("revision reused acceptance, lost history or changed limit", task)
		}
	}
	shared, _ := w.task(mapping["T1"])
	if !strings.Contains(shared.Next, "Historique après redémarrage") {
		t.Fatal("shared worker context was stale")
	}
	revision := w.Revision
	if _, err = s.preparationCommand(r); err != nil {
		t.Fatal(err)
	}
	w, _ = s.get(w.ID)
	if w.Revision != revision {
		t.Fatal("revision replay mutated work")
	}
	agents, err := s.agents(w.ID)
	if err != nil || len(agents) != 0 {
		t.Fatal("revision launched an agent", err)
	}
}
