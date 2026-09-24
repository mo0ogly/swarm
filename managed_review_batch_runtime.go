//go:build linux

package main

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"time"
)

func batchPlanDigest(c managedReviewContext, prefix string, batches []managedReviewBatch, cfg *ReviewerConfig, workflow AgentWorkflow) string {
	raw, _ := json.Marshal([]any{c, prefix, batches, cfg.ProviderDigest, cfg.ModelRoute, workflow})
	return hash(raw)
}

func (s *Store) batchReply(v ManagedReviewBatchVerdict, b managedReviewBatch) (string, error) {
	read := func(path, digest string) ([]byte, error) {
		p, e := safeReport(s.root, path)
		if e != nil {
			return nil, e
		}
		raw, e := os.ReadFile(p)
		if e != nil || digest == "" || hash(raw) != digest {
			return nil, fmt.Errorf("preuve du lot modifiée ou absente")
		}
		return raw, nil
	}
	raw, e := read(v.Context, v.ContextDigest)
	if e != nil {
		return "", e
	}
	expected, _ := json.Marshal(b.Context)
	if string(raw) != string(expected) || !reflect.DeepEqual(v.Tasks, b.Tasks) {
		return "", fmt.Errorf("affectation ou contexte du lot modifié")
	}
	reply, e := read(v.ReplyPath, v.ReplyDigest)
	return string(reply), e
}

func (s *Store) reviewManagedBatches(w Work, a Agent, receiptPath string, receipt []byte, c managedReviewContext, prefix string, workflow AgentWorkflow, timeout int, batches []managedReviewBatch) error {
	cfg := w.Planning.Reviewer
	task, e := w.task(a.TaskID)
	if e != nil {
		return e
	}
	ps, e := s.providers()
	if e != nil {
		return e
	}
	provider, ok := ps.Providers[cfg.Provider]
	rawProvider, _ := json.Marshal(provider)
	if !ok || hash(rawProvider) != cfg.ProviderDigest {
		return fmt.Errorf("configuration du vérificateur absente ou modifiée")
	}
	level := "auto"
	if cfg.ModelRoute != nil {
		level = cfg.ModelRoute.Level
	}
	provider, route, e := resolveModel(provider, level, "planning")
	if e != nil {
		return e
	}
	if cfg.ModelRoute != nil && (route == nil || route.PolicyHash != cfg.ModelRoute.PolicyHash) {
		return fmt.Errorf("politique du modèle de revue modifiée")
	}
	data, _ := json.Marshal(c)
	digest := batchPlanDigest(c, prefix, batches, cfg, workflow)
	record := IndependentReview{ModelRoute: route, ID: newID("review-"), Attempt: a.Attempt, Producer: a.ID, Reviewer: "reviewer://" + cfg.Provider, Contract: reviewContract(task), CandidateSHA: c.Candidate, PreviousCandidate: c.Previous, Receipt: receiptPath, ReceiptDigest: hash(receipt), ContextDigest: hash(data), State: "running", Reason: "Examen indépendant cumulatif en plusieurs lots.", Started: now(), Workflow: &workflow, TimeoutSeconds: timeout, BatchPlanDigest: digest, BatchProviderDigest: cfg.ProviderDigest}
	for _, tc := range c.Tasks {
		if tc.Task == a.TaskID {
			record.Report = managedReviewReportPath(receiptPath, tc.Task)
			record.GitReport = tc.Binding.Report
			record.Digest = tc.Binding.ReportDigest
		}
	}
	record.Context = filepath.ToSlash(filepath.Join(filepath.Dir(receiptPath), "review-context.json"))
	record.Batches = make([]ManagedReviewBatchVerdict, len(batches))
	missing := len(batches)
	// Only a DB-anchored explicit retry can donate paid, completed batches.
	if old := task.BatchReviewResume; old != nil {
		if old.BatchPlanDigest != digest || old.CandidateSHA != record.CandidateSHA || old.PreviousCandidate != record.PreviousCandidate || old.Attempt != record.Attempt || old.Producer != record.Producer || old.Contract != record.Contract || old.ReceiptDigest != record.ReceiptDigest || old.ContextDigest != record.ContextDigest || old.BatchProviderDigest != cfg.ProviderDigest || len(old.Batches) != len(batches) {
			return fmt.Errorf("plan de lots de reprise périmé ; aucun avis réutilisé")
		}
		if e = s.managedReviewFilesIntact(*old); e != nil {
			return e
		}
		for i, v := range old.Batches {
			if v.State != "passed" {
				continue
			}
			reply, err := s.batchReply(v, batches[i])
			if err != nil {
				return err
			}
			subset := batches[i].Context
			subset.Tasks = nil
			for _, id := range batches[i].Tasks {
				for _, tc := range c.Tasks {
					if tc.Task == id {
						subset.Tasks = append(subset.Tasks, tc)
					}
				}
			}
			state, _, err := parseManagedReview(reply, subset)
			if err != nil || state != "passed" {
				return fmt.Errorf("avis acquis du lot %d invalide", i+1)
			}
			record.Batches[i] = v
			missing--
		}
	}
	if cfg.Calls+missing > cfg.MaxCalls {
		return fmt.Errorf("budget insuffisant pour les %d appels de lots restants", missing)
	}
	// Write shared canonical evidence only after validating any previous bundle.
	for _, tc := range c.Tasks {
		path := filepath.Join(s.root, managedReviewReportPath(receiptPath, tc.Task))
		if e = os.MkdirAll(filepath.Dir(path), 0700); e != nil {
			return e
		}
		if e = atomicWrite(path, []byte(tc.Report)); e != nil {
			return e
		}
	}
	if e = atomicWrite(filepath.Join(s.root, record.Context), data); e != nil {
		return e
	}
	raw, _ := json.Marshal(record)
	_, e = s.mutate(w.ID, "review.managed.batch.begin", record.ID+"-begin", w.Revision, raw, func(cw *Work) error {
		ct, err := cw.task(a.TaskID)
		if err != nil {
			return err
		}
		if ct.IndependentReview != nil || !currentTaskAttempt(ct, a.Attempt) || cw.Planning.Repository.Candidate != record.PreviousCandidate || managedReviewContract(*cw, a.TaskID) != managedReviewContract(w, a.TaskID) || cw.Planning.Reviewer.Calls+missing > cw.Planning.Reviewer.MaxCalls {
			return fmt.Errorf("revue par lots devenue indisponible")
		}
		ct.IndependentReview = &record
		ct.BatchReviewResume = nil
		return nil
	})
	if e != nil {
		return e
	}
	finish := func(state, reason string) error {
		record.State = state
		record.Reason = guardBlock(reason, 4000)
		record.Finished = now()
		if err := s.saveManagedReview(w.ID, a, record); err != nil {
			return err
		}
		if state != "passed" {
			return fmt.Errorf("vérificateur indépendant : %s", reason)
		}
		return nil
	}
	for i, b := range batches {
		if record.Batches[i].State == "passed" {
			continue
		}
		child := ManagedReviewBatchVerdict{ID: newID("review-batch-"), Tasks: b.Tasks, State: "running", Started: now()}
		contextData, _ := json.Marshal(b.Context)
		child.Context = filepath.ToSlash(filepath.Join(filepath.Dir(receiptPath), child.ID+"-context.json"))
		child.ContextDigest = hash(contextData)
		if e = atomicWrite(filepath.Join(s.root, child.Context), contextData); e != nil {
			return finish("error", e.Error())
		}
		current, err := s.get(w.ID)
		if err != nil {
			return err
		}
		raw, _ = json.Marshal(child)
		_, e = s.mutateWithHook(w.ID, "review.managed.batch.claim", child.ID, current.Revision, raw, func(cw *Work) error {
			ct, err := cw.task(a.TaskID)
			if err != nil {
				return err
			}
			if ct.IndependentReview == nil || ct.IndependentReview.ID != record.ID || !currentTaskAttempt(ct, a.Attempt) || managedReviewContract(*cw, a.TaskID) != managedReviewContract(w, a.TaskID) || cw.Planning.Repository.Candidate != record.PreviousCandidate || cw.Planning.Reviewer.Calls >= cw.Planning.Reviewer.MaxCalls {
				return fmt.Errorf("lot devenu indisponible")
			}
			if err = s.managedReviewFilesIntact(record); err != nil {
				return err
			}
			if err = s.managedBatchPlanIntact(*cw, record); err != nil {
				return err
			}
			ct.IndependentReview.Batches[i] = child
			cw.Planning.Reviewer.Calls++
			return nil
		}, func(tx *sql.Tx, _ *Work) error {
			if err := s.providerCooldownGuard(cfg.Provider); err != nil {
				return err
			}
			return reservePlanningCall(tx, w.ID, "reviewer", child.ID)
		})
		if e != nil {
			return finish("error", e.Error())
		}
		record.Batches[i] = child
		reply, callErr := runStructuredProvider(provider, route, b.Prompt, managedReviewSchema, time.Duration(timeout)*time.Second, func() bool {
			if s.providerCooldownGuard(cfg.Provider) != nil || s.paused(w.ID) || s.managedReviewFilesIntact(record) != nil {
				return false
			}
			cw, err := s.get(w.ID)
			if err != nil || cw.Planning == nil || cw.Planning.Repository == nil || cw.Planning.Repository.Candidate != record.PreviousCandidate || managedReviewContract(cw, a.TaskID) != managedReviewContract(w, a.TaskID) || s.managedBatchProviderIntact(cw.Planning.Reviewer, record) != nil {
				return false
			}
			ct, err := cw.task(a.TaskID)
			return err == nil && ct.IndependentReview != nil && ct.IndependentReview.ID == record.ID && len(ct.IndependentReview.Batches) > i && ct.IndependentReview.Batches[i].ID == child.ID
		}, func(u *Usage) { _ = s.savePlanningUsage(child.ID, u) }, s.providerCooldownObserver(cfg.Provider, child.ID))
		child.Finished = now()
		if reply != "" {
			child.ReplyPath = filepath.ToSlash(filepath.Join(filepath.Dir(receiptPath), child.ID+"-reply.json"))
			child.ReplyDigest = hash([]byte(reply))
			if err = atomicWrite(filepath.Join(s.root, child.ReplyPath), []byte(reply)); err != nil {
				callErr = err
			}
		}
		subset := b.Context
		subset.Tasks = nil
		for _, id := range b.Tasks {
			for _, tc := range c.Tasks {
				if tc.Task == id {
					subset.Tasks = append(subset.Tasks, tc)
				}
			}
		}
		if callErr == nil {
			child.State, _, callErr = parseManagedReview(reply, subset)
		}
		if callErr != nil {
			child.State = "error"
		}
		record.Batches[i] = child
		current, err = s.get(w.ID)
		if err != nil {
			return err
		}
		raw, _ = json.Marshal(child)
		_, e = s.mutate(w.ID, "review.managed.batch.result", child.ID+"-result", current.Revision, raw, func(cw *Work) error {
			ct, err := cw.task(a.TaskID)
			if err != nil {
				return err
			}
			if ct.IndependentReview == nil || ct.IndependentReview.ID != record.ID || ct.IndependentReview.Batches[i].ID != child.ID {
				return fmt.Errorf("lot remplacé")
			}
			ct.IndependentReview.Batches[i] = child
			return nil
		})
		if e != nil {
			return e
		}
		if callErr != nil {
			return finish("error", callErr.Error())
		}
		if child.State != "passed" {
			return finish("changes_requested", fmt.Sprintf("Lot %d : corrections ou preuves demandées", i+1))
		}
	}
	replies := []string{}
	for i, v := range record.Batches {
		reply, err := s.batchReply(v, batches[i])
		if err != nil {
			return finish("stale", err.Error())
		}
		replies = append(replies, reply)
	}
	state, entries, err := parseManagedReviewBatchReplies(c, batches, replies)
	if err != nil {
		return finish("error", err.Error())
	}
	record.ManagedTasks = entries
	for _, entry := range entries {
		if entry.Task == a.TaskID {
			record.Criteria = entry.Criteria
		}
	}
	return finish(state, "Tous les lots ont examiné les critères du même candidat.")
}

func (s *Store) managedBatchProofsIntact(r IndependentReview) error {
	if len(r.Batches) == 0 {
		return nil
	}
	path, e := safeReport(s.root, r.Context)
	if e != nil {
		return e
	}
	raw, e := os.ReadFile(path)
	if e != nil {
		return e
	}
	var canonical managedReviewContext
	if e = json.Unmarshal(raw, &canonical); e != nil {
		return e
	}
	batches := []managedReviewBatch{}
	replies := []string{}
	for _, v := range r.Batches {
		if v.ID == "" {
			if r.State == "passed" {
				return fmt.Errorf("lot non exécuté")
			}
			continue
		}
		path, err := safeReport(s.root, v.Context)
		if err != nil {
			return err
		}
		data, err := os.ReadFile(path)
		if err != nil || hash(data) != v.ContextDigest {
			return fmt.Errorf("contexte du lot modifié")
		}
		var ctx managedReviewContext
		if err = json.Unmarshal(data, &ctx); err != nil {
			return err
		}
		expectedTasks, _ := json.Marshal(canonical.Tasks)
		actualTasks, _ := json.Marshal(ctx.Tasks)
		if !sameManagedBaseline(ctx.Baseline, canonical.Baseline) || ctx.Candidate != canonical.Candidate || ctx.Previous != canonical.Previous || ctx.Diff != canonical.Diff || string(ctx.Receipt) != string(canonical.Receipt) || string(expectedTasks) != string(actualTasks) {
			return fmt.Errorf("preuves globales du lot modifiées")
		}
		b := managedReviewBatch{Tasks: v.Tasks, Context: ctx}
		batches = append(batches, b)
		if v.ReplyPath != "" {
			reply, err := s.batchReply(v, b)
			if err != nil {
				return err
			}
			replies = append(replies, reply)
		} else if r.State == "passed" {
			return fmt.Errorf("réponse du lot absente")
		}
		if r.State == "passed" && v.State != "passed" {
			return fmt.Errorf("lot non favorable")
		}
	}
	if r.State == "passed" {
		state, records, err := parseManagedReviewBatchReplies(canonical, batches, replies)
		if err != nil || state != "passed" || !reflect.DeepEqual(records, r.ManagedTasks) {
			return fmt.Errorf("avis cumulatif différent des réponses originales des lots : %v", err)
		}
	}
	return nil
}

func (s *Store) managedBatchPlanIntact(w Work, r IndependentReview) error {
	if r.FragmentJournal != nil {
		if w.Planning == nil || w.Planning.Reviewer == nil {
			return fmt.Errorf("configuration de revue absente")
		}
		if err := s.managedBatchProviderIntact(w.Planning.Reviewer, r); err != nil {
			return err
		}
		raw, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
		if hash(raw) != r.FragmentJournal.ModelConfigDigest {
			return fmt.Errorf("modèle des fragments modifié")
		}
		_, _, err := s.readFragmentJournalAnchor(r)
		return err
	}
	if len(r.Batches) == 0 {
		return nil
	}
	if w.Planning == nil {
		return fmt.Errorf("planification des lots absente")
	}
	if err := s.managedBatchProviderIntact(w.Planning.Reviewer, r); err != nil {
		return err
	}
	path, e := safeReport(s.root, r.Context)
	if e != nil {
		return e
	}
	raw, e := os.ReadFile(path)
	if e != nil {
		return e
	}
	var c managedReviewContext
	if e = json.Unmarshal(raw, &c); e != nil {
		return e
	}
	owners, e := managedReviewSourceOwners(w, c)
	if e != nil {
		return e
	}
	workflow, prompt, e := agentWorkflow("reviewer")
	if e != nil {
		return e
	}
	prefix := managedReviewPrefix(prompt)
	batches, e := planManagedReviewBatches(prefix, c, owners)
	if e != nil {
		return e
	}
	if len(batches) != len(r.Batches) || batchPlanDigest(c, prefix, batches, w.Planning.Reviewer, workflow) != r.BatchPlanDigest {
		return fmt.Errorf("plan ou méthode des lots modifié")
	}
	for i, v := range r.Batches {
		expected, _ := json.Marshal(batches[i].Context)
		if v.ID != "" && (!reflect.DeepEqual(v.Tasks, batches[i].Tasks) || v.ContextDigest != hash(expected)) {
			return fmt.Errorf("affectation des lots modifiée")
		}
	}
	return nil
}

// Re-read the provider configuration: a stored digest alone cannot detect a
// configuration file edited while an independent review is in flight.
func (s *Store) managedBatchProviderIntact(cfg *ReviewerConfig, r IndependentReview) error {
	if cfg == nil || r.BatchProviderDigest != cfg.ProviderDigest {
		return fmt.Errorf("fournisseur des lots modifié")
	}
	ps, err := s.providers()
	if err != nil {
		return err
	}
	p, ok := ps.Providers[cfg.Provider]
	raw, _ := json.Marshal(p)
	if !ok || hash(raw) != r.BatchProviderDigest {
		return fmt.Errorf("configuration du fournisseur des lots modifiée")
	}
	level := "auto"
	if cfg.ModelRoute != nil {
		level = cfg.ModelRoute.Level
	}
	_, route, err := resolveModel(p, level, "planning")
	if err != nil {
		return err
	}
	if cfg.ModelRoute != nil && (route == nil || route.PolicyHash != cfg.ModelRoute.PolicyHash) {
		return fmt.Errorf("politique du modèle des lots modifiée")
	}
	return nil
}
