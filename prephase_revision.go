package main

import (
	"database/sql"
	"fmt"
	"reflect"
)

type PreparationPlanChange struct {
	ID    string `json:"id"`
	Kind  string `json:"kind"`
	Title string `json:"title"`
}

func preparationPlanChanges(old, next ActionPlan) []PreparationPlanChange {
	before := map[string]PlanMission{}
	after := map[string]PlanMission{}
	out := []PreparationPlanChange{}
	for _, m := range old.Tasks {
		before[m.ID] = m
	}
	for _, m := range next.Tasks {
		after[m.ID] = m
		prior, ok := before[m.ID]
		kind := "conservée"
		if !ok {
			kind = "ajoutée"
		} else if !reflect.DeepEqual(prior, m) || old.Objective != next.Objective || !reflect.DeepEqual(old.Questions, next.Questions) || !reflect.DeepEqual(old.Assumptions, next.Assumptions) {
			kind = "modifiée"
		}
		out = append(out, PreparationPlanChange{m.ID, kind, m.Title})
	}
	for _, m := range old.Tasks {
		if _, ok := after[m.ID]; !ok {
			out = append(out, PreparationPlanChange{m.ID, "retirée (historique conservé)", m.Title})
		}
	}
	impacted := map[string]bool{}
	for _, c := range out {
		if c.Kind != "conservée" {
			impacted[c.ID] = true
		}
	}
	for again := true; again; {
		again = false
		for _, task := range old.Tasks {
			for _, dep := range task.Depends {
				if impacted[dep] && !impacted[task.ID] {
					impacted[task.ID] = true
					again = true
				}
			}
		}
	}
	for i := range out {
		if out[i].Kind == "conservée" && impacted[out[i].ID] {
			out[i].Kind = "à revérifier (dépendance modifiée)"
		}
	}
	return out
}
func (s *Store) revisePreparedMissions(tx *sql.Tx, w *Work, p *Preparation, r PreparationRequest) error {
	if p.Conversion == nil {
		return fmt.Errorf("aucun plan créé à réviser")
	}
	s.preparationFreshness(p)
	if !p.PlanReady || r.Hash != p.Documents["plan"].Hash {
		return preparationError("stale_plan", "Vérifier le plan courant avant d’appliquer sa révision.")
	}
	if r.Hash == p.Conversion.PlanHash {
		return nil
	}
	var active int
	if e := tx.QueryRow("SELECT count(*) FROM agents WHERE work_id=? AND status IN ('queued','starting','running','stopping')", w.ID).Scan(&active); e != nil {
		return e
	}
	if active > 0 {
		return fmt.Errorf("Une tentative est active. Mettez la mission en pause puis attendez sa fin ou arrêtez-la explicitement avant d’appliquer la révision. Le brouillon reste conservé.")
	}
	if w.Planning != nil {
		for _, sc := range w.Planning.Scopes {
			if sc.Holder != "" {
				return fmt.Errorf("Une décision du responsable est en cours ; suspendre la planification avant révision")
			}
		}
		if len(w.Planning.Scopes) != 1 {
			return fmt.Errorf("Révision avec branches déléguées : suspendre et réconcilier leurs responsabilités avant application")
		}
	}
	var spec ActionPlan
	if e := strict([]byte(p.Documents["plan"].Text), &spec); e != nil {
		return e
	}
	if _, e := validateActionPlan(spec, true); e != nil {
		return e
	}
	temp := Work{Tasks: []Task{}}
	if e := s.materializePlan(&temp, PlanReview{Source: p.ID, BriefHash: p.Brief.Hash, ResponseHash: r.Hash, Spec: spec}); e != nil {
		return e
	}
	oldIDs := map[string]bool{}
	for _, id := range p.Conversion.TaskIDs {
		oldIDs[id] = true
	}
	nextBy := map[string]Task{}
	for _, t := range temp.Tasks {
		nextBy[t.ID] = t
	}
	prefix := "plan-" + hash([]byte(p.ID))[:10] + "-"
	changed := map[string]bool{}
	for _, c := range preparationPlanChanges(p.Conversion.Spec, spec) {
		if c.Kind != "conservée" {
			changed[prefix+c.ID] = true
		}
	}
	// Revalidation propagates through all dependents, including tasks outside this plan.
	for again := true; again; {
		again = false
		for _, t := range w.Tasks {
			for _, dep := range t.Depends {
				if changed[dep] && !changed[t.ID] {
					changed[t.ID] = true
					again = true
				}
			}
		}
	}
	for i := range w.Tasks {
		t := &w.Tasks[i]
		if !changed[t.ID] {
			continue
		}
		if t.Status == "running" {
			return fmt.Errorf("tâche en cours : %s", t.ID)
		}
		if w.Planning != nil && oldIDs[t.ID] {
			if _, exists := nextBy[t.ID]; !exists {
				return fmt.Errorf("%s : retirer une mission changerait les exigences du responsable ; conserver la mission et réviser son contenu", t.Title)
			}
		}
		if nt, ok := nextBy[t.ID]; ok {
			if t.ValidationPolicy != nil && t.ValidationPolicy.Mode == "automatic" {
				for _, prior := range p.Conversion.Spec.Tasks {
					if prefix+prior.ID == t.ID {
						for _, next := range spec.Tasks {
							if next.ID == prior.ID && (prior.Entry != next.Entry || prior.Validation != next.Validation || prior.Delivery != next.Delivery || prior.Proof != next.Proof || prior.Scope != next.Scope || prior.Deliverable != next.Deliverable) {
								return fmt.Errorf("%s : le sens des contrôles change ; réautoriser la validation avant cette révision", t.Title)
							}
						}
					}
				}
			}
			if t.ValidationPolicy != nil && t.ValidationPolicy.Mode == "automatic" && !reflect.DeepEqual(t.Criteria, nt.Criteria) {
				return fmt.Errorf("%s : critères modifiés ; réautoriser une politique de revue humaine avant cette révision ou conserver les critères couverts par les contrôles", t.Title)
			}
			if w.Planning != nil && len(nt.Criteria) < len(t.Criteria) {
				return fmt.Errorf("%s : conserver les critères existants pour ne pas perdre une exigence du responsable", t.Title)
			}
			t.Title = nt.Title
			t.Deliverable = nt.Deliverable
			t.Criteria = nt.Criteria
			t.Depends = nt.Depends
			t.Next = nt.Next
			t.PlanRole = nt.PlanRole
			t.PlanChecks = nt.PlanChecks
			t.PlanMaxAttempts = nt.PlanMaxAttempts
			t.PlanToolLimit = nt.PlanToolLimit
			if w.Planning != nil && w.Planning.HumanReviewAuthorized != "" {
				root, _ := w.Planning.scope("root")
				for n, criterion := range t.Criteria {
					if n < len(t.Requirements) {
						var idx int
						fmt.Sscanf(t.Requirements[n], "req-%d", &idx)
						if idx > 0 && idx <= len(w.Criteria) {
							w.Criteria[idx-1] = criterion
						}
					} else {
						w.Criteria = append(w.Criteria, criterion)
						req := fmt.Sprintf("req-%d", len(w.Criteria))
						t.Requirements = append(t.Requirements, req)
						root.Requirements = append(root.Requirements, req)
					}
				}
			}
		}
		t.Status = "todo"
		t.Gate = nil
		t.AutoValidation = nil
		t.Override = nil
		t.Revalidation = nil
		t.Blocker = ""
		t.PlanningRetry = false
		if oldIDs[t.ID] {
			if _, ok := nextBy[t.ID]; !ok {
				t.Status = "abandoned"
				t.Blocker = "Retirée par la révision du plan ; historique conservé."
			}
		}
	}
	for _, nt := range temp.Tasks {
		if _, e := w.task(nt.ID); e == nil {
			if !oldIDs[nt.ID] {
				return fmt.Errorf("identifiant déjà occupé hors du plan : %s", nt.ID)
			}
			continue
		}
		if w.Planning != nil {
			if w.Planning.HumanReviewAuthorized == "" {
				return fmt.Errorf("Nouvelle tâche : une politique de contrôle doit être explicitement autorisée avant ajout au plan automatique")
			}
			root, _ := w.Planning.scope("root")
			nt.ScopeID = "root"
			for _, criterion := range nt.Criteria {
				w.Criteria = append(w.Criteria, criterion)
				req := fmt.Sprintf("req-%d", len(w.Criteria))
				nt.Requirements = append(nt.Requirements, req)
				root.Requirements = append(root.Requirements, req)
			}
			nt.ValidationPolicy = &ValidationPolicy{Mode: "human", Authorized: now(), Actor: operatorIdentity()}
		}
		w.Tasks = append(w.Tasks, nt)
	}
	// External dependents of retired tasks cannot be silently made runnable.
	for _, t := range w.Tasks {
		if t.Status == "abandoned" {
			continue
		}
		for _, dep := range t.Depends {
			if oldIDs[dep] {
				if _, ok := nextBy[dep]; !ok {
					return fmt.Errorf("%s dépend encore d’une tâche retirée ; réviser cette dépendance", t.Title)
				}
			}
		}
	}
	w.Objective = spec.Objective
	w.Scope = p.Brief.Text
	approved := temp.Plans[0]
	w.Plans = append(w.Plans, approved)
	p.Conversion = &PreparationConversion{WorkID: w.ID, PlanHash: r.Hash, BriefHash: p.Brief.Hash, MethodHash: p.MethodHash, TaskIDs: approved.TaskIDs, Spec: spec, CreatedAt: now()}
	for _, id := range approved.TaskIDs {
		t, _ := w.task(id)
		t.LaunchHeld = true
	}
	if w.Planning != nil {
		root, _ := w.Planning.scope("root")
		root.Objective = spec.Objective
		root.Revision++
		root.Generation++
		root.State = "waiting"
		w.Planning.Paused = true
		for i := range w.Planning.Inbox {
			if w.Planning.Inbox[i].Decision == "" {
				w.Planning.Inbox[i].Decision = "revision-" + r.Event
			}
		}
		w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: "revision-" + r.Event, Scope: "root", Kind: "plan_revision", Message: "Plan révisé par l’opérateur. Conserver les missions et leurs critères ; examiner les retours avant toute nouvelle tâche.", At: now()})
		if e := organizationGuard(*w); e != nil {
			return e
		}
	}
	// Pause is explicit in the review and transactional with the revision.
	_, e := tx.Exec("INSERT INTO cockpit_controls(work_id,paused) VALUES(?,1) ON CONFLICT(work_id) DO UPDATE SET paused=1", w.ID)
	// New works already exist here; no execution is launched by this mutation.
	return e
}
