package engine

import (
	"encoding/json"
	"fmt"
	"path"
	"path/filepath"
	"strings"
	"time"
)

// Exploration reads an immutable accepted Git tree. It executes no user command,
// changes no source and grants no production, integration or acceptance right.
type PlanningExplorationRequest struct {
	Mode      string   `json:"mode"`
	Candidate string   `json:"candidate_commit"`
	Directory string   `json:"directory,omitempty"`
	Files     []string `json:"files,omitempty"`
}
type PlanningExploration struct {
	Scope         string                     `json:"scope"`
	ScopeRevision int                        `json:"scope_revision"`
	Generation    int                        `json:"generation"`
	Revision      int                        `json:"observed_revision"`
	Decision      string                     `json:"request_decision"`
	Candidate     string                     `json:"candidate_commit"`
	Request       PlanningExplorationRequest `json:"request"`
	Entries       []string                   `json:"entries,omitempty"`
	Sources       []ReviewSource             `json:"sources,omitempty"`
	Digest        string                     `json:"sha256"`
}

func validExplorationDirectory(directory string) bool {
	return directory == "" || (path.Clean(directory) == directory && directory != "." && !path.IsAbs(directory) && !strings.HasPrefix(directory, "../") && !strings.ContainsAny(directory, "\x00\r\n\\:") && directory != ".git" && !strings.HasPrefix(directory, ".git/") && directory != ".swarm" && !strings.HasPrefix(directory, ".swarm/"))
}
func explorationDigest(e PlanningExploration) string {
	e.Digest = ""
	raw, _ := json.Marshal(e)
	return hash(raw)
}
func explorationCurrent(w Work, e *PlanningExploration) bool {
	return e == nil || (w.Planning != nil && w.Planning.Repository != nil && e.Candidate == w.Planning.Repository.Candidate && e.Digest == explorationDigest(*e))
}
func (s *Store) preparePlanningExploration(w Work, r PlanningRequest, op PlanningOperation) (*PlanningExploration, error) {
	if w.Planning == nil || w.Planning.Repository == nil {
		return nil, fmt.Errorf("exploration en lecture exige un dépôt géré accepté")
	}
	scope, err := w.Planning.scope(r.Scope)
	if err != nil {
		return nil, err
	}
	until, parseErr := time.Parse(time.RFC3339Nano, scope.Until)
	if w.Planning.Paused || s.paused(w.ID) || scope.Holder == "" || scope.Holder != r.Holder || scope.Generation != r.Generation || scope.Revision != r.ScopeRevision || parseErr != nil || !time.Now().Before(until) {
		return nil, planningError("stale_lease", "autorisation d’exploration absente ou périmée")
	}
	if op.ID == "" || !safeName(op.ID) || op.Title != "" || len(op.Requirements) != 0 || len(op.Criteria) != 0 || len(op.Depends) != 0 || op.Next != "" || op.MaxTasks != 0 || op.MaxActivations != 0 {
		return nil, fmt.Errorf("explore accepte uniquement id et deliverable JSON")
	}
	var query PlanningExplorationRequest
	if err = strict([]byte(op.Deliverable), &query); err != nil {
		return nil, err
	}
	if query.Candidate == "" || query.Candidate != w.Planning.Repository.Candidate {
		return nil, fmt.Errorf("candidat d’exploration remplacé ; relire le contexte")
	}
	_, method, err := s.projectAgentWorkflow(planningWorkflowRole(scope))
	if err != nil {
		return nil, err
	}
	// Reuse the existing technical wire envelope, after the actual role method
	// and instruction overhead. No independent hard-coded exploration policy.
	capacity := planningPromptContextLimit(method)
	e := &PlanningExploration{Scope: r.Scope, ScopeRevision: r.ScopeRevision, Generation: r.Generation, Revision: w.Revision, Decision: r.EventID, Candidate: query.Candidate, Request: query}
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	switch query.Mode {
	case "read":
		if query.Directory != "" || len(query.Files) == 0 {
			return nil, fmt.Errorf("read exige files non vide, sans directory")
		}
		seen := map[string]bool{}
		for _, name := range query.Files {
			if seen[name] || name == ".swarm" || strings.HasPrefix(name, ".swarm/") {
				return nil, fmt.Errorf("source répétée ou privée")
			}
			seen[name] = true
			source, readErr := candidateReviewSource(bare, query.Candidate, name, capacity)
			if readErr != nil {
				return nil, readErr
			}
			e.Sources = append(e.Sources, source)
			raw, _ := json.Marshal(e)
			if len(raw) > capacity {
				return nil, fmt.Errorf("exploration supérieure au transport du rôle (%d octets) ; réduire la requête, aucun contenu tronqué", capacity)
			}
		}
	case "list":
		if len(query.Files) != 0 || !validExplorationDirectory(query.Directory) {
			return nil, fmt.Errorf("list exige un directory relatif, sans files")
		}
		tree := query.Candidate
		if query.Directory != "" {
			tree, err = managedGit(bare, "rev-parse", query.Candidate+":"+query.Directory)
			if err != nil {
				return nil, err
			}
			kind, kindErr := managedGit(bare, "cat-file", "-t", tree)
			if kindErr != nil || kind != "tree" {
				return nil, fmt.Errorf("répertoire Git régulier requis")
			}
		}
		names, listErr := scopeGit(bare, "ls-tree", "--name-only", "-z", tree)
		if listErr != nil {
			return nil, listErr
		}
		if names != "" {
			for _, name := range strings.Split(strings.TrimSuffix(names, "\x00"), "\x00") {
				if name == ".git" || name == ".swarm" {
					continue
				}
				e.Entries = append(e.Entries, path.Join(query.Directory, name))
			}
		}
	default:
		return nil, fmt.Errorf("mode d’exploration requis : list ou read")
	}
	e.Digest = explorationDigest(*e)
	raw, _ := json.Marshal(e)
	if len(raw) > capacity {
		return nil, fmt.Errorf("inventaire supérieur au transport du rôle ; préciser le répertoire, aucun contenu tronqué")
	}
	return e, nil
}
func (s *Store) checkPlanningExplorationDelivery(w Work, r PlanningRequest) error {
	if len(r.explorationInputs) == 0 {
		return nil
	}
	raw, _ := json.Marshal(w)
	var projected Work
	if err := json.Unmarshal(raw, &projected); err != nil {
		return err
	}
	for i := range projected.Planning.Inbox {
		if containsString(r.Inputs, projected.Planning.Inbox[i].ID) {
			projected.Planning.Inbox[i].Decision = r.EventID
		}
	}
	for _, op := range r.Operations {
		if e := r.explorationInputs[op.ID]; op.Kind == "explore" && e != nil {
			projected.Planning.Inbox = append(projected.Planning.Inbox, PlanningEvent{ID: planningEventID(r.EventID, op.ID), Scope: r.Scope, Kind: "exploration", Message: "Exploration en lecture du candidat accepté ; originaux et attribution joints.", Exploration: e, At: now()})
		}
	}
	scope, err := projected.Planning.scope(r.Scope)
	if err != nil {
		return err
	}
	scope.Revision++
	scope.Generation++
	scope.Holder = ""
	scope.Until = ""
	scope.State = "ready"
	projected.Revision++
	_, method, err := s.projectAgentWorkflow(planningWorkflowRole(scope))
	if err != nil {
		return err
	}
	for _, e := range r.explorationInputs {
		// Every original must fit a whole single-event activation; admission may
		// batch fewer complete events, but must never truncate source bytes.
		oneRaw, _ := json.Marshal(projected)
		var one Work
		if err = json.Unmarshal(oneRaw, &one); err != nil {
			return err
		}
		filtered := []PlanningEvent{}
		for _, event := range one.Planning.Inbox {
			if event.Scope != r.Scope || event.Decision != "" {
				filtered = append(filtered, event)
			} else if event.Exploration != nil && event.Exploration.Digest == e.Digest {
				filtered = append(filtered, event)
			}
		}
		one.Planning.Inbox = filtered
		if _, _, err = s.planningDeliveryContext(one, r.Scope, planningPromptContextLimit(method)); err != nil {
			return fmt.Errorf("original d’exploration non transmissible au propriétaire : %w", err)
		}
	}
	return nil
}
func applyPlanningExploration(w *Work, scope string, op PlanningOperation, r PlanningRequest) error {
	e := r.explorationInputs[op.ID]
	if e == nil || e.Scope != scope || e.ScopeRevision != r.ScopeRevision || e.Generation != r.Generation || e.Revision != w.Revision || e.Decision != r.EventID || !explorationCurrent(*w, e) {
		return fmt.Errorf("preuve d’exploration absente ou périmée")
	}
	w.Planning.Inbox = append(w.Planning.Inbox, PlanningEvent{ID: planningEventID(r.EventID, op.ID), Scope: scope, Kind: "exploration", Message: "Exploration en lecture du candidat accepté ; originaux et attribution joints.", Exploration: e, At: now()})
	return nil
}

func planningExplorationRepository(w Work) any {
	if w.Planning == nil || w.Planning.Repository == nil {
		return nil
	}
	return map[string]any{"candidate_commit": w.Planning.Repository.Candidate, "modes": []string{"list", "read"}, "read_only": true, "source": "immutable accepted Git tree", "capacity": "actual role transport envelope; complete sources or explicit refusal"}
}
