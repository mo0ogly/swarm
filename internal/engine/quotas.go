package engine

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

type QuotaValues struct {
	ReviewContextTokens *int `json:"review_context_tokens,omitempty"`
	ReviewContextFiles  *int `json:"review_context_files,omitempty"`
	ReviewSourceBytes   *int `json:"review_source_bytes,omitempty"`
	ReviewContextBytes  *int `json:"review_context_bytes,omitempty"`
	ReviewFiles         *int `json:"review_delivery_files,omitempty"`
	Scopes              *int `json:"planning_scopes,omitempty"`
	Depth               *int `json:"delegation_depth,omitempty"`
	Tasks               *int `json:"planning_tasks,omitempty"`
	JudgeCycles         *int `json:"judge_cycles,omitempty"`
	Planning            int  `json:"planning_activations"`
	Decisions           int  `json:"planning_decisions"`
	Reviews             *int `json:"review_calls"`
}
type QuotaAuthorization struct {
	BeforeScopes map[string]int `json:"before_scope_activations,omitempty"`
	AfterScopes  map[string]int `json:"after_scope_activations,omitempty"`
	Before       QuotaValues    `json:"before"`
	After        QuotaValues    `json:"after"`
	Actor        string         `json:"actor"`
	At           string         `json:"at"`
	Reason       string         `json:"reason"`
}
type QuotaChange struct {
	ScopeActivations map[string]int `json:"scope_activations,omitempty"`
	Schema           int            `json:"schema_version"`
	EventID          string         `json:"event_id"`
	Revision         int            `json:"expected_revision"`
	Limits           QuotaValues    `json:"limits"`
	Reason           string         `json:"reason"`
}
type QuotaView struct {
	ReviewManifestBytes int                 `json:"review_manifest_transport_bytes"`
	Bounds              QuotaValues         `json:"bounds"`
	Revision            int                 `json:"revision"`
	Limits              QuotaValues         `json:"limits"`
	Consumed            QuotaValues         `json:"consumed"`
	Remaining           QuotaValues         `json:"remaining"`
	Authorization       *QuotaAuthorization `json:"last_authorization,omitempty"`
	Scopes              []PlanningScope     `json:"scopes"`
}

func quotaValues(p *PlanningState) (QuotaValues, QuotaValues) {
	limits := QuotaValues{Planning: p.MaxActivations, Decisions: p.MaxDecisions}
	used := QuotaValues{Planning: p.Activations, Decisions: p.Decisions}
	tasks := p.MaxTasks
	limits.Tasks = &tasks
	scopes, depth := p.MaxScopes, p.MaxDepth
	limits.Scopes, limits.Depth = &scopes, &depth
	if p.Reviewer != nil {
		a, b := p.Reviewer.MaxCalls, p.Reviewer.Calls
		limits.Reviews = &a
		files := effectiveReviewFileLimit(p.Reviewer)
		limits.ReviewFiles = &files
		source := effectiveReviewSourcePolicy(p.Reviewer)
		limits.ReviewContextFiles, limits.ReviewSourceBytes, limits.ReviewContextBytes = &source.Files, &source.SourceBytes, &source.TotalBytes
		tokens := p.Reviewer.InputContextTokens
		limits.ReviewContextTokens = &tokens
		used.Reviews = &b
	}
	if p.Judge != nil {
		limit, consumed := p.Judge.MaxCycles, p.Judge.Cycle
		limits.JudgeCycles, used.JudgeCycles = &limit, &consumed
	}
	return limits, used
}
func quotaView(w Work) (QuotaView, error) {
	if w.Planning == nil {
		return QuotaView{}, fmt.Errorf("Cette mission ne dispose pas de planification hiérarchique.")
	}
	limits, used := quotaValues(w.Planning)
	count := len(w.Tasks)
	used.Tasks = &count
	scopeCount := len(w.Planning.Scopes)
	depth := 0
	for _, scope := range w.Planning.Scopes {
		ancestors := planningAncestors(w.Planning, scope.ID)
		if len(ancestors) == 0 {
			return QuotaView{}, fmt.Errorf("hiérarchie invalide ; aucun budget déduit")
		}
		depth = max(depth, len(ancestors))
	}
	used.Scopes, used.Depth = &scopeCount, &depth
	availableTasks := max(0, *limits.Tasks-count)
	remaining := QuotaValues{Planning: max(0, limits.Planning-used.Planning), Decisions: max(0, limits.Decisions-used.Decisions)}
	remaining.Tasks = &availableTasks
	if *limits.Scopes > 0 {
		n := max(0, *limits.Scopes-scopeCount)
		remaining.Scopes = &n
	}
	if *limits.Depth > 0 {
		n := max(0, *limits.Depth-depth)
		remaining.Depth = &n
	}
	if limits.Reviews != nil {
		n := max(0, *limits.Reviews-*used.Reviews)
		remaining.Reviews = &n
	}
	if limits.JudgeCycles != nil && *limits.JudgeCycles > 0 {
		n := max(0, *limits.JudgeCycles-*used.JudgeCycles)
		remaining.JudgeCycles = &n
	}
	// Zero/null bounds mean no additional policy ceiling in the executable.
	// Explicit persisted budgets and current consumption remain authoritative.
	return QuotaView{ReviewManifestBytes: reviewManifestTransportBytes, Bounds: QuotaValues{}, Revision: w.Revision, Limits: limits, Consumed: used, Remaining: remaining, Authorization: w.Planning.QuotaAuthorization, Scopes: append([]PlanningScope(nil), w.Planning.Scopes...)}, nil
}
func applyQuotaChange(w *Work, r QuotaChange) error {
	if r.Schema != 1 || r.Revision < 1 {
		return fmt.Errorf("Version de demande ou révision invalide.")
	}
	if len(strings.TrimSpace(r.Reason)) < 8 || len(r.Reason) > 2000 {
		return fmt.Errorf("Expliquez la modification des plafonds (8 à 2 000 caractères).")
	}
	p := w.Planning
	if p == nil {
		return fmt.Errorf("Cette mission ne dispose pas de planification hiérarchique.")
	}
	if r.Limits.Planning < max(1, p.Activations) || r.Limits.Decisions < max(1, p.Decisions) {
		return fmt.Errorf("Plafonds de planification incompatibles avec la consommation actuelle.")
	}
	if r.Limits.Tasks != nil && *r.Limits.Tasks < max(1, len(w.Tasks)) {
		return fmt.Errorf("Plafond de tâches inférieur au nombre de tâches existantes.")
	}
	view, err := quotaView(*w)
	if err != nil {
		return err
	}
	if r.Limits.Scopes != nil && (*r.Limits.Scopes < 0 || (*r.Limits.Scopes > 0 && *r.Limits.Scopes < *view.Consumed.Scopes)) {
		return fmt.Errorf("Plafond de périmètres inférieur à la hiérarchie existante.")
	}
	if r.Limits.Depth != nil && (*r.Limits.Depth < 0 || (*r.Limits.Depth > 0 && *r.Limits.Depth < *view.Consumed.Depth)) {
		return fmt.Errorf("Profondeur configurée inférieure à la hiérarchie existante.")
	}
	for _, limit := range []*int{r.Limits.ReviewContextFiles, r.Limits.ReviewSourceBytes, r.Limits.ReviewContextBytes} {
		if limit != nil && (p.Reviewer == nil || *limit < 0) {
			return fmt.Errorf("limite de contexte de revue invalide : vérificateur requis, 0 sans plafond ou entier positif")
		}
	}
	if r.Limits.ReviewContextTokens != nil && (p.Reviewer == nil || *r.Limits.ReviewContextTokens < 0) {
		return fmt.Errorf("fenêtre de contexte de revue invalide : 0 pour le défaut du client ou un nombre positif de tokens")
	}
	if r.Limits.ReviewFiles != nil && (p.Reviewer == nil || *r.Limits.ReviewFiles < 0) {
		return fmt.Errorf("limite de fichiers de revue invalide : vérificateur requis, 0 sans plafond ou entier positif")
	}
	if p.Reviewer == nil && r.Limits.Reviews != nil {
		return fmt.Errorf("Aucun vérificateur configuré ; configurez son identité avant son budget.")
	}
	if p.Reviewer != nil && (r.Limits.Reviews == nil || *r.Limits.Reviews < max(1, p.Reviewer.Calls)) {
		return fmt.Errorf("Plafond de vérification au moins égal à la consommation actuelle requis.")
	}
	if r.Limits.JudgeCycles != nil {
		limit := *r.Limits.JudgeCycles
		if p.Judge == nil || limit < 0 || (limit > 0 && limit < p.Judge.Cycle) {
			return fmt.Errorf("Plafond de cycles du juge invalide : juge configuré requis, 0 sans plafond ou valeur au moins égale à la consommation actuelle.")
		}
	}
	for _, scope := range p.Scopes {
		if scope.Holder != "" {
			return fmt.Errorf("Un planificateur détient encore une session ; attendez sa libération avant de modifier les plafonds.")
		}
	}
	for _, task := range w.Tasks {
		if task.IndependentReview != nil && task.IndependentReview.State == "running" {
			return fmt.Errorf("Une vérification est en cours ; attendez sa fin avant de modifier les plafonds.")
		}
	}
	beforeScopes, afterScopes := map[string]int{}, map[string]int{}
	for id, limit := range r.ScopeActivations {
		scope, e := p.scope(id)
		if e != nil {
			return e
		}
		if limit < max(1, scope.Activations) || limit > r.Limits.Planning {
			return fmt.Errorf("Plafond de périmètre incompatible avec sa consommation ou le plafond global : %s", id)
		}
		beforeScopes[id], afterScopes[id] = scope.ActivationLimit, limit
	}
	for _, scope := range p.Scopes {
		limit, changed := r.ScopeActivations[scope.ID]
		if !changed {
			limit = scope.ActivationLimit
		}
		if scope.Parent == "" {
			continue
		}
		parent, e := p.scope(scope.Parent)
		if e != nil {
			return e
		}
		parentLimit, parentChanged := r.ScopeActivations[parent.ID]
		if !parentChanged {
			parentLimit = parent.ActivationLimit
		}
		if parentLimit == 0 {
			parentLimit = r.Limits.Planning
		}
		if (changed || parentChanged) && limit > parentLimit {
			return fmt.Errorf("Le plafond enfant dépasse celui du parent : %s", scope.ID)
		}
	}
	before, _ := quotaValues(p)
	p.MaxActivations = r.Limits.Planning
	p.MaxDecisions = r.Limits.Decisions
	if r.Limits.Tasks != nil {
		p.MaxTasks = *r.Limits.Tasks
	}
	if r.Limits.Scopes != nil {
		p.MaxScopes = *r.Limits.Scopes
	}
	if r.Limits.Depth != nil {
		p.MaxDepth = *r.Limits.Depth
	}
	if p.Reviewer != nil {
		p.Reviewer.MaxCalls = *r.Limits.Reviews
		if r.Limits.ReviewContextTokens != nil {
			p.Reviewer.InputContextTokens = *r.Limits.ReviewContextTokens
		}
		for _, field := range []struct {
			value  *int
			target **int
		}{{r.Limits.ReviewContextFiles, &p.Reviewer.MaxContextFiles}, {r.Limits.ReviewSourceBytes, &p.Reviewer.MaxSourceBytes}, {r.Limits.ReviewContextBytes, &p.Reviewer.MaxContextBytes}} {
			if field.value != nil {
				n := *field.value
				*field.target = &n
			}
		}
		if r.Limits.ReviewFiles != nil {
			n := *r.Limits.ReviewFiles
			p.Reviewer.MaxFiles = &n
		}
	}
	if r.Limits.JudgeCycles != nil {
		p.Judge.MaxCycles = *r.Limits.JudgeCycles
	}
	after, _ := quotaValues(p)
	for id, limit := range r.ScopeActivations {
		scope, _ := p.scope(id)
		scope.ActivationLimit = limit
	}
	p.QuotaAuthorization = &QuotaAuthorization{BeforeScopes: beforeScopes, AfterScopes: afterScopes, Before: before, After: after, Actor: operatorIdentity(), At: now(), Reason: strings.TrimSpace(r.Reason)}
	// Failures, pauses, counters and unlisted sub-scope allocations are preserved.
	// Explicit per-scope changes are audited; increasing a cap is not acceptance.
	return nil
}

func (s *Store) registerQuotas(mux *http.ServeMux, send func(http.ResponseWriter, any), fail func(http.ResponseWriter, error)) {
	mux.HandleFunc("/api/v1/quotas", func(out http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			http.Error(out, "GET requis", http.StatusMethodNotAllowed)
			return
		}
		w, err := s.get(r.URL.Query().Get("work"))
		if err != nil {
			fail(out, err)
			return
		}
		view, err := quotaView(w)
		if err != nil {
			fail(out, err)
			return
		}
		send(out, view)
	})
}
func (s *Store) previewQuotas(work string, r QuotaChange) (any, error) {
	w, err := s.get(work)
	if err != nil {
		return nil, err
	}
	if w.Revision != r.Revision {
		return nil, &CommandError{Code: "revision_conflict", Message: "Le travail a changé ; actualiser puis confirmer.", Retryable: true}
	}
	current, err := quotaView(w)
	if err != nil {
		return nil, err
	}
	if err = s.validateReviewContextQuota(w, r); err != nil {
		return nil, err
	}
	// get returns a detached Work; never persist this preview.
	if err = applyQuotaChange(&w, r); err != nil {
		return nil, err
	}
	proposed, err := quotaView(w)
	if err != nil {
		return nil, err
	}
	return map[string]any{"current": current, "proposed": proposed}, nil
}
func (s *Store) configureQuotas(work string, r QuotaChange) (Work, error) {
	raw, err := json.Marshal(r)
	if err != nil {
		return Work{}, err
	}
	return s.mutate(work, "quotas.configure", r.EventID, r.Revision, raw, func(w *Work) error {
		if err := s.validateReviewContextQuota(*w, r); err != nil {
			return err
		}
		return applyQuotaChange(w, r)
	})
}
func (s *Store) quotasCLI(pos []string, input string, out io.Writer) error {
	if len(pos) != 3 {
		return fmt.Errorf("swarm quotas show|preview|apply <work> [--input quotas.json]")
	}
	if pos[1] == "show" {
		w, err := s.get(pos[2])
		if err != nil {
			return err
		}
		v, err := quotaView(w)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	if pos[1] != "preview" && pos[1] != "apply" {
		return fmt.Errorf("Action de plafonds inconnue.")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r QuotaChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	if pos[1] == "preview" {
		v, err := s.previewQuotas(pos[2], r)
		if err != nil {
			return err
		}
		return printJSON(out, v)
	}
	w, err := s.configureQuotas(pos[2], r)
	if err != nil {
		return err
	}
	return printJSON(out, w)
}

func (s *Store) validateReviewContextQuota(w Work, r QuotaChange) error {
	if r.Limits.ReviewContextTokens == nil || *r.Limits.ReviewContextTokens == 0 {
		return nil
	}
	if *r.Limits.ReviewContextTokens < 0 || w.Planning == nil || w.Planning.Reviewer == nil {
		return fmt.Errorf("fenêtre de contexte de revue invalide")
	}
	cfg := *w.Planning.Reviewer
	cfg.InputContextTokens = *r.Limits.ReviewContextTokens
	_, err := s.reviewInputBudget(&cfg)
	return err
}
