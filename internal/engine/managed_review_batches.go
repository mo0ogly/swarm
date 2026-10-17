//go:build linux

package engine

import (
	"encoding/json"
	"errors"
	"fmt"
)

const managedReviewPromptLimit = 192 * 1024

var errManagedReviewBatchSize = errors.New(managedReviewContextTooLarge)

// This is a preflight plan, never a review verdict. All global evidence remains
// in every batch; only supplemental sources are assigned by task. Runtime must
// persist and validate the whole plan before counting/calling any batch.
type managedReviewBatch struct {
	Tasks   []string             `json:"tasks"`
	Context managedReviewContext `json:"context"`
	Prompt  string               `json:"prompt"`
}

func boundedManagedReviewPrompt(prefix string, c managedReviewContext) (string, error) {
	if c.CitationContractVersion == 2 {
		prompt, _, err := managedReferencedReviewPrompt(prefix, c)
		return prompt, err
	}
	prefix = incrementalReviewPrefix(prefix, c)
	data, _ := json.Marshal(c)
	prompt := prefix + "\nSWARM_MANAGED_REVIEW_CONTEXT\n" + string(data)
	if len(prompt) > managedReviewPromptLimit {
		prompt = prefix + managedReviewTextPacket(c)
	}
	return prompt, nil
}

// sourceOwners comes from the immutable per-task context manifests, not a model
// response. Missing or foreign assignments are rejected instead of guessing.
func planManagedReviewBatches(prefix string, c managedReviewContext, sourceOwners map[string][]string) ([]managedReviewBatch, error) {
	// Preserve every previously valid plan byte for byte, including paid lots.
	// The lossless hunk transport is a fallback only after the whole legacy
	// preflight fails. It cannot reshuffle an existing valid review plan.
	batches, err := planManagedReviewBatchesTransport(prefix, c, sourceOwners, false)
	if !errors.Is(err, errManagedReviewBatchSize) {
		return batches, err
	}
	return planManagedReviewBatchesTransport(prefix, c, sourceOwners, true)
}

func planManagedReviewBatchesTransport(prefix string, c managedReviewContext, sourceOwners map[string][]string, reuseHunks bool) ([]managedReviewBatch, error) {
	bounded := func(prefix string, c managedReviewContext) (string, error) {
		prompt, err := boundedManagedReviewPrompt(prefix, c)
		if err != nil {
			return "", err
		}
		if c.CitationContractVersion != 2 && reuseHunks && len(prompt) > managedReviewPromptLimit {
			alternative := incrementalReviewPrefix(prefix, c) + managedReviewPacket(c, true)
			if len(alternative) < len(prompt) {
				return alternative, nil
			}
		}
		return prompt, nil
	}
	if len(c.Tasks) == 0 {
		return nil, fmt.Errorf("revue sans tâche")
	}
	tasks := map[string]bool{}
	for _, t := range c.Tasks {
		if t.Task == "" || tasks[t.Task] || len(t.Criteria) == 0 {
			return nil, fmt.Errorf("contrat de tâches incomplet ou dupliqué")
		}
		tasks[t.Task] = true
	}
	sources := map[string]ReviewSource{}
	for _, s := range c.Sources {
		if s.Path == "" || s.Bytes != len(s.Content) || s.Digest != hash([]byte(s.Content)) {
			return nil, fmt.Errorf("source de revue incohérente")
		}
		if _, ok := sources[s.Path]; ok {
			return nil, fmt.Errorf("source de revue dupliquée")
		}
		sources[s.Path] = s
	}
	assigned := map[string]bool{}
	for id, paths := range sourceOwners {
		if !tasks[id] {
			return nil, fmt.Errorf("affectation de tâche inconnue")
		}
		seen := map[string]bool{}
		for _, p := range paths {
			if _, ok := sources[p]; !ok || seen[p] {
				return nil, fmt.Errorf("affectation de source inconnue ou dupliquée")
			}
			seen[p] = true
			assigned[p] = true
		}
	}
	for p := range sources {
		if !assigned[p] {
			return nil, fmt.Errorf("source non affectée : %s", p)
		}
	}
	// Preserve the ordinary single-context transport and instructions exactly.
	prompt, err := bounded(prefix, c)
	if err != nil {
		return nil, err
	}
	all := managedReviewBatch{Context: c, Prompt: prompt}
	for _, task := range c.Tasks {
		all.Tasks = append(all.Tasks, task.Task)
	}
	inputBytes, err := managedReviewBatchInputBytes(all)
	if err != nil {
		return nil, err
	}
	if inputBytes <= managedReviewPromptLimit {
		return []managedReviewBatch{all}, nil
	}

	makeBatch := func(ids []string) (managedReviewBatch, error) {
		selected := map[string]bool{}
		for _, id := range ids {
			for _, p := range sourceOwners[id] {
				selected[p] = true
			}
		}
		subset := c
		subset.Sources = nil
		for _, s := range c.Sources {
			if selected[s.Path] {
				subset.Sources = append(subset.Sources, s)
			}
		}
		names, _ := json.Marshal(ids)
		instruction := prefix + "\nLOT DE REVUE : examine uniquement les tâches de cette liste JSON et retourne uniquement leurs verdicts : " + string(names) + ". Tous les rapports, critères, contrôles et le diff cumulatif restent présents pour le contexte global. Les autres tâches seront examinées séparément ; ne présume pas leur validation. Les sources annexes jointes sont celles déclarées pour les tâches affectées. Si une source absente est nécessaire, retourne unknown, jamais pass par défaut.\n"
		prompt, err := bounded(instruction, subset)
		return managedReviewBatch{Tasks: append([]string(nil), ids...), Context: subset, Prompt: prompt}, err
	}
	batches := []managedReviewBatch{}
	ids := []string{}
	for _, t := range c.Tasks {
		next := append(append([]string(nil), ids...), t.Task)
		b, err := makeBatch(next)
		if err != nil {
			return nil, err
		}
		inputBytes, err := managedReviewBatchInputBytes(b)
		if err != nil {
			return nil, err
		}
		if inputBytes > managedReviewPromptLimit {
			if len(ids) == 0 {
				return nil, fmt.Errorf("%w : tâche %s", errManagedReviewBatchSize, t.Task)
			}
			previous, err := makeBatch(ids)
			if err != nil {
				return nil, err
			}
			batches = append(batches, previous)
			ids = []string{t.Task}
			single, err := makeBatch(ids)
			if err != nil {
				return nil, err
			}
			singleBytes, err := managedReviewBatchInputBytes(single)
			if err != nil {
				return nil, err
			}
			if singleBytes > managedReviewPromptLimit {
				return nil, fmt.Errorf("%w : tâche %s", errManagedReviewBatchSize, t.Task)
			}
		} else {
			ids = next
		}
	}
	if len(ids) > 0 {
		last, err := makeBatch(ids)
		if err != nil {
			return nil, err
		}
		batches = append(batches, last)
	}
	return batches, nil
}

// Resolve ownership from immutable Git manifests. Sources omitted from the
// canonical list must be exact added-file duplicates already present in the diff.
func managedReviewSourceOwners(w Work, c managedReviewContext) (map[string][]string, error) {
	// Collection enforces current operator policy before a new review is frozen.
	// Rechecking a paid context verifies immutable attribution, never a later
	// policy. No context bytes or assignments are trusted without Git comparison.
	planning := *w.Planning
	reviewer := ReviewerConfig{}
	if planning.Reviewer != nil {
		reviewer = *planning.Reviewer
	}
	zero := 0
	reviewer.MaxContextFiles, reviewer.MaxSourceBytes, reviewer.MaxContextBytes = &zero, &zero, &zero
	planning.Reviewer = &reviewer
	w.Planning = &planning
	owners := map[string][]string{}
	canonical := map[string]ReviewSource{}
	for _, source := range c.Sources {
		canonical[source.Path] = source
	}
	for _, task := range c.Tasks {
		sources, err := managedReviewSources(w, []managedReviewTaskContext{task}, c.Candidate)
		if err != nil {
			return nil, err
		}
		owners[task.Task] = nil
		for _, source := range sources {
			expected, ok := canonical[source.Path]
			if !ok {
				if !addedSourceInDiff(c.Diff, source) {
					return nil, fmt.Errorf("source du manifeste absente du contexte : %s", source.Path)
				}
				continue
			}
			if source.Digest != expected.Digest || source.Blob != expected.Blob || source.Bytes != expected.Bytes || source.Content != expected.Content {
				return nil, fmt.Errorf("source du manifeste différente du contexte : %s", source.Path)
			}
			owners[task.Task] = append(owners[task.Task], source.Path)
		}
	}
	return owners, nil
}

// Parse original independent replies, never summaries or caller-supplied pass
// flags. Return no partial aggregate: every task and criterion must be covered
// exactly once on the same candidate before publication can even be considered.
func parseManagedReviewBatchReplies(c managedReviewContext, batches []managedReviewBatch, replies []string) (string, []ManagedTaskReview, error) {
	if len(batches) == 0 || len(batches) != len(replies) {
		return "", nil, fmt.Errorf("réponses de lots incomplètes")
	}
	canonical, _ := json.Marshal(c.Tasks)
	sources := map[string]ReviewSource{}
	for _, s := range c.Sources {
		sources[s.Path] = s
	}
	expected := map[string]managedReviewTaskContext{}
	for _, t := range c.Tasks {
		if _, dup := expected[t.Task]; dup {
			return "", nil, fmt.Errorf("tâche canonique dupliquée")
		}
		expected[t.Task] = t
	}
	seen := map[string]bool{}
	seenSources := map[string]bool{}
	records := map[string]ManagedTaskReview{}
	state := "passed"
	for i, b := range batches {
		actual, _ := json.Marshal(b.Context.Tasks)
		if b.Context.CitationContractVersion != c.CitationContractVersion || b.Context.InspectionScopeVersion != c.InspectionScopeVersion || !sameManagedBaseline(b.Context.Baseline, c.Baseline) || b.Context.Candidate != c.Candidate || b.Context.Previous != c.Previous || b.Context.Diff != c.Diff || string(b.Context.Receipt) != string(c.Receipt) || string(actual) != string(canonical) {
			return "", nil, fmt.Errorf("preuves globales différentes dans un lot")
		}
		subset := b.Context
		subset.Tasks = nil
		for _, id := range b.Tasks {
			t, ok := expected[id]
			if !ok || seen[id] {
				return "", nil, fmt.Errorf("affectation de lot inconnue ou dupliquée")
			}
			seen[id] = true
			subset.Tasks = append(subset.Tasks, t)
		}
		if len(subset.Tasks) == 0 {
			return "", nil, fmt.Errorf("lot sans tâche")
		}
		supplied := map[string]bool{}
		for _, s := range b.Context.Sources {
			original, ok := sources[s.Path]
			if !ok || supplied[s.Path] || s != original {
				return "", nil, fmt.Errorf("source de lot différente du contexte canonique")
			}
			supplied[s.Path] = true
			seenSources[s.Path] = true
		}
		verdict, entries, err := parseManagedBatchReply(replies[i], b)
		if err != nil {
			return "", nil, fmt.Errorf("lot %d : %w", i+1, err)
		}
		if verdict != "passed" {
			state = "changes_requested"
		}
		for _, entry := range entries {
			records[entry.Task] = entry
		}
	}
	if len(seen) != len(expected) {
		return "", nil, fmt.Errorf("couverture cumulative incomplète")
	}
	if len(seenSources) != len(sources) {
		return "", nil, fmt.Errorf("source canonique absente des lots")
	}
	result := []ManagedTaskReview{}
	for _, t := range c.Tasks {
		result = append(result, records[t.Task])
	}
	return state, result, nil
}
