//go:build linux

package engine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"reflect"
	"strings"
)

// Original reply and packet are retained byte-for-byte under their OLD identity.
// This is a local observation, never a fresh inspection or task verdict. Every
// original input in the packet is a dependency; any change invalidates the group.
// New cross-file effects must also be resolved explicitly in the final verdict.
type managedFragmentReuse struct {
	Packet   int                         `json:"packet"`
	Review   IndependentReview           `json:"origin_review"`
	Original managedReviewFragmentPacket `json:"original_packet"`
	Reply    string                      `json:"original_reply"`
}

func fragmentPaidInspections(p managedReviewFragmentPlan) int { return len(p.Packets) - len(p.Reused) }
func fragmentArtifactKey(a managedReviewFragmentArtifact) string {
	return a.Kind + "\x00" + a.Name + "\x00" + a.Digest
}
func fragmentScope(c managedReviewContext) string {
	var scopes []any
	for _, t := range c.Tasks {
		scopes = append(scopes, []any{t.Task, t.Title, t.Deliverable, t.Criteria, t.Binding.Contract, t.Binding.Policy})
	}
	raw, _ := json.Marshal(scopes)
	return hash(raw)
}
func fragmentReuseAt(p managedReviewFragmentPlan, index int) *managedFragmentReuse {
	for i := range p.Reused {
		if p.Reused[i].Packet == index {
			return &p.Reused[i]
		}
	}
	return nil
}
func parsePlannedFragment(reply string, p managedReviewFragmentPlan, index int) (string, managedFragmentInspection, error) {
	if r := fragmentReuseAt(p, index); r != nil {
		if reply != r.Reply {
			return "", managedFragmentInspection{}, fmt.Errorf("historical reply substituted")
		}
		return parseManagedFragmentInspection(reply, r.Original)
	}
	return parseManagedFragmentInspection(reply, p.Packets[index])
}

func validateFragmentReuseCoverage(c managedReviewContext, p managedReviewFragmentPlan, actual, expected []managedReviewFragmentArtifact) error {
	if len(p.Reused) == 0 || !strings.HasPrefix(p.ChangeDiff, "diff --git ") {
		return fmt.Errorf("historical observations without current change set")
	}
	inventory := map[string]string{}
	for _, a := range expected {
		key := fragmentArtifactKey(a)
		if _, ok := inventory[key]; ok {
			return fmt.Errorf("duplicate canonical artifact")
		}
		inventory[key] = a.Content
	}
	for _, a := range actual {
		key := fragmentArtifactKey(a)
		content, ok := inventory[key]
		if !ok || content != a.Content {
			return fmt.Errorf("incomplete or duplicated evidence coverage")
		}
		delete(inventory, key)
	}
	if len(inventory) > 0 {
		return fmt.Errorf("missing original evidence")
	}
	seen := map[int]bool{}
	for _, r := range p.Reused {
		if r.Packet < 0 || r.Packet >= len(p.Packets) || seen[r.Packet] || r.Review.FragmentJournal == nil || r.Review.CandidateSHA == c.Candidate || r.Original.Candidate != r.Review.CandidateSHA || !reflect.DeepEqual(r.Original.Artifacts, p.Packets[r.Packet].Artifacts) {
			return fmt.Errorf("historical observation identity or inputs changed")
		}
		seen[r.Packet] = true
		state, _, err := parseManagedFragmentInspection(r.Reply, r.Original)
		if err != nil || (state != "inspected" && state != "unknown") {
			return fmt.Errorf("historical refusal or incomplete response cannot be reused")
		}
	}
	return nil
}

func differentialFragmentPlan(c managedReviewContext, old managedReviewContext, origin IndependentReview, oldPlan managedReviewFragmentPlan, j managedFragmentJournal, delta string, available int) (managedReviewFragmentPlan, error) {
	if fragmentScope(c) != fragmentScope(old) || oldPlan.Version >= 3 || delta == "" {
		return planManagedReviewFragments(c, available, 2)
	}
	artifacts, err := managedFragmentArtifacts(c)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	all := map[string]managedReviewFragmentArtifact{}
	for _, a := range artifacts {
		all[fragmentArtifactKey(a)] = a
	}
	eligible, err := validateManagedFragmentJournal(j, oldPlan, j.Attempt, j.ProviderDigest)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	reused := []managedFragmentReuse{}
	for i, packet := range oldPlan.Packets {
		reply, ok := eligible[i]
		if !ok {
			continue
		}
		intact := true
		for _, a := range packet.Artifacts {
			current, ok := all[fragmentArtifactKey(a)]
			if !ok || current.Content != a.Content {
				intact = false
				break
			}
		}
		if !intact {
			continue
		}
		for _, a := range packet.Artifacts {
			delete(all, fragmentArtifactKey(a))
		}
		reused = append(reused, managedFragmentReuse{Review: origin, Original: packet, Reply: reply})
	}
	if len(reused) == 0 {
		return planManagedReviewFragments(c, available, 2)
	}
	fresh := []managedReviewFragmentArtifact{}
	for _, a := range artifacts {
		if _, ok := all[fragmentArtifactKey(a)]; ok {
			fresh = append(fresh, a)
		}
	}
	p, err := packManagedFragmentArtifacts(c, fresh, available, 2)
	if err != nil {
		return p, err
	}
	p.Version = 3
	p.ChangeDiff = delta
	for i := range p.Packets {
		p.Packets[i].Version = 3
	}
	for _, r := range reused {
		r.Packet = len(p.Packets)
		p.Reused = append(p.Reused, r)
		p.Packets = append(p.Packets, managedReviewFragmentPacket{Version: 3, Candidate: c.Candidate, ContextDigest: p.ContextDigest, Index: r.Packet, Artifacts: r.Original.Artifacts})
	}
	if err = validateManagedReviewFragments(c, p); err != nil {
		return managedReviewFragmentPlan{}, err
	}
	return p, nil
}

func (s *Store) storedFragmentOrigin(work, id string) (IndependentReview, error) {
	var r IndependentReview
	var raw []byte
	err := s.db.QueryRow("SELECT request FROM events WHERE work_id=? AND kind='review.managed.result' AND json_extract(request,'$.id')=? ORDER BY revision DESC LIMIT 1", work, id).Scan(&raw)
	if err != nil {
		return r, err
	}
	err = strict(raw, &r)
	return r, err
}
func (s *Store) planLegacyCandidateFragments(w Work, a Agent, c managedReviewContext, available int) (managedReviewFragmentPlan, error) {
	t, err := w.task(a.TaskID)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	if t.RecoveredResult == nil || t.RecoveredResult.PriorReview == "" {
		return planManagedReviewFragments(c, available, 2)
	}
	r, err := s.storedFragmentOrigin(w.ID, t.RecoveredResult.PriorReview)
	if err != nil || r.FragmentJournal == nil || (r.State != "changes_requested" && r.State != "error") || r.Contract != reviewContract(t) || r.Producer != a.ID || r.Attempt != a.Attempt || s.managedBatchProviderIntact(w.Planning.Reviewer, r) != nil {
		return planManagedReviewFragments(c, available, 2)
	}
	oldPlan, j, err := s.readFragmentJournalAnchor(r)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	// A terminal error can carry references to earlier proved observations.
	// None of its own replies or verdict are eligible for reuse.
	if r.State == "error" {
		if r.Finished == "" || oldPlan.Version < 3 || len(oldPlan.Reused) == 0 {
			return planManagedReviewFragments(c, available, 2)
		}
		for _, entry := range j.Entries {
			if entry.State == "reserved" {
				return managedReviewFragmentPlan{}, fmt.Errorf("historical review still has a reserved call")
			}
		}
	}
	// A differential plan is not a new origin. Resolve its original durable
	// record and recheck that proof directly. Never recursively promote a chain
	// of observations, nor reuse the intermediate candidate's verdict.
	if oldPlan.Version >= 3 {
		found := false
		for _, ref := range oldPlan.Reused {
			original, e := s.storedFragmentOrigin(w.ID, ref.Review.ID)
			if e != nil || !reflect.DeepEqual(original, ref.Review) {
				return managedReviewFragmentPlan{}, fmt.Errorf("historical origin differs from durable review")
			}
			op, oj, e := s.readFragmentJournalAnchor(original)
			if e != nil {
				return managedReviewFragmentPlan{}, e
			}
			if op.Version >= 3 {
				continue
			}
			// One original baseline supplies the complete delta used by the final
			// impact questions. Other histories are not silently combined.
			r, oldPlan, j = original, op, oj
			found = true
			break
		}
		if !found {
			return planManagedReviewFragments(c, available, 2)
		}
	}
	model, _ := json.Marshal(w.Planning.Reviewer.ModelRoute)
	if r.FragmentJournal.ModelConfigDigest != hash(model) {
		return planManagedReviewFragments(c, available, 2)
	}
	path, err := safeReport(s.root, r.Context)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	raw, err := os.ReadFile(path)
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	var old managedReviewContext
	if err = strict(raw, &old); err != nil {
		return managedReviewFragmentPlan{}, err
	}
	bare := filepath.Join(w.Planning.Repository.Storage, "repository.git")
	// Recovered candidates may be siblings: compare exact trees, never claim ancestry.
	delta, err := managedGit(bare, "diff", "--no-ext-diff", "--no-textconv", "--full-index", "--unified=40", r.CandidateSHA, c.Candidate, "--")
	if err != nil {
		return managedReviewFragmentPlan{}, err
	}
	if managedDiffHasBinary(delta) {
		return planManagedReviewFragments(c, available, 2)
	}
	return differentialFragmentPlan(c, old, r, oldPlan, j, delta, available)
}

func (s *Store) validateFragmentReuseSources(parent IndependentReview, c managedReviewContext, p managedReviewFragmentPlan) error {
	if len(p.Reused) == 0 {
		return nil
	}
	parts := strings.Split(filepath.ToSlash(parent.Context), "/")
	if len(parts) < 5 || parts[0] != ".swarm" || parts[1] != "managed" {
		return fmt.Errorf("invalid managed review location")
	}
	bare := filepath.Join(s.root, parts[0], parts[1], parts[2], "repository.git")

	type originalProof struct {
		plan    managedReviewFragmentPlan
		replies map[int]string
	}
	// Recheck once per source review per validation, never cache across mutations.
	checked := map[string]originalProof{}
	for _, ref := range p.Reused {
		encoded, _ := json.Marshal(ref.Review)
		key := hash(encoded)
		if proof, ok := checked[key]; ok {
			if ref.Original.Index < 0 || ref.Original.Index >= len(proof.plan.Packets) || !reflect.DeepEqual(proof.plan.Packets[ref.Original.Index], ref.Original) || proof.replies[ref.Original.Index] != ref.Reply {
				return fmt.Errorf("historical observation substituted")
			}
			continue
		}
		// The current plan digest anchors this original record. It was retrieved
		// from the durable event before the new plan was adopted; never query the
		// single-connection Store again while checking proofs inside a transaction.
		origin := ref.Review
		if !strings.HasPrefix(origin.Context, strings.Join(parts[:3], "/")+"/") || origin.FragmentJournal == nil || origin.State != "changes_requested" || origin.Producer != parent.Producer || origin.Attempt != parent.Attempt || origin.Contract != parent.Contract || origin.BatchProviderDigest != parent.BatchProviderDigest || origin.FragmentJournal.ModelConfigDigest != parent.FragmentJournal.ModelConfigDigest || origin.FragmentJournal.WorkflowDigest != parent.FragmentJournal.WorkflowDigest {
			return fmt.Errorf("historical provider, method, model or provenance changed")
		}
		path, err := safeReport(s.root, origin.FragmentJournal.Plan)
		if err != nil {
			return err
		}
		raw, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		var header struct {
			Version int `json:"version"`
		}
		if json.Unmarshal(raw, &header) != nil || header.Version > 2 {
			return fmt.Errorf("nested historical reuse is not supported")
		}
		op, oj, err := s.readFragmentJournalAnchor(origin)
		if err != nil {
			return err
		}
		if ref.Original.Index < 0 || ref.Original.Index >= len(op.Packets) || !reflect.DeepEqual(op.Packets[ref.Original.Index], ref.Original) {
			return fmt.Errorf("historical packet substituted")
		}
		replies, err := validateManagedFragmentJournal(oj, op, oj.Attempt, oj.ProviderDigest)
		if err != nil || replies[ref.Original.Index] != ref.Reply {
			return fmt.Errorf("historical reply absent from anchored journal")
		}
		path, err = safeReport(s.root, origin.Context)
		if err != nil {
			return err
		}
		raw, err = os.ReadFile(path)
		if err != nil {
			return err
		}
		var old managedReviewContext
		if strict(raw, &old) != nil || fragmentScope(old) != fragmentScope(c) {
			return fmt.Errorf("historical task contract changed")
		}
		delta, err := managedGit(bare, "diff", "--no-ext-diff", "--no-textconv", "--full-index", "--unified=40", origin.CandidateSHA, c.Candidate, "--")
		if err != nil {
			return err
		}
		if delta != p.ChangeDiff {
			return fmt.Errorf("current change set substituted")
		}
		checked[key] = originalProof{op, replies}
	}
	return nil
}

type managedHistoricalObservationLabel struct {
	Packet      int    `json:"packet"`
	Candidate   string `json:"origin_candidate"`
	Review      string `json:"origin_review"`
	ReplyDigest string `json:"origin_reply_sha256"`
}

func addHistoricalImpactQuestion(bundle *managedFragmentFinalEvidence, ref managedFragmentReuse, inspection managedFragmentInspection) {
	bundle.Historical = append(bundle.Historical, managedHistoricalObservationLabel{ref.Packet, ref.Original.Candidate, ref.Review.ID, hash([]byte(ref.Reply))})
	next := 0
	for _, f := range inspection.Findings {
		if f.Artifact == 0 {
			next = len(f.Needs)
		}
	}
	bundle.Questions = append(bundle.Questions, managedFragmentQuestion{Packet: ref.Packet, Artifact: 0, Need: next, Text: "Réexaminer les impacts et dépendances des changements actuels sur ce groupe d’observations historiques. Citer un extrait exact de ligne ajoutée ou retirée dans changes_since_observations, et expliquer son effet sur ce groupe ; un ancien avis seul ne suffit pas."})
}
