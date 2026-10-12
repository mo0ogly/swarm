//go:build linux

package engine

import (
	"strings"
	"testing"
)

func TestInstructionRevisionOperatorNextRemainsComposable(t *testing.T) {
	s, w, _ := revisedInstructionFixture(t)
	original := *w.Tasks[0].InstructionBase
	addition := w.Tasks[0].InstructionRevisions[0].Addition
	v := w.Tasks[0].InstructionRevisions[0]
	const operator = "Importer le paquet opérateur immuable avant de modifier le code."
	updated := applyTest(t, s, w, "task.update", Request{Schema: 1, EventID: "operator-input-update", Revision: w.Revision, ID: "future", Next: operator + "\n\n" + w.Tasks[0].Next})
	w = updated
	got, err := s.planningChange(w.ID, "reject-instructions", PlanningRequest{Schema: 1, EventID: "operator-reject-after-input", Revision: w.Revision, Task: "future", InstructionRevisionID: v.ID, Reason: "Rejeter uniquement cet apprentissage IA"})
	if err != nil {
		t.Fatal("operator input made rejection unavailable", err)
	}
	if !strings.Contains(got.Tasks[0].Next, operator) || !strings.Contains(got.Tasks[0].Next, original) || strings.Contains(got.Tasks[0].Next, addition) {
		t.Fatal("lost operator/original input or retained rejected learning", got.Tasks[0].Next)
	}
	if *got.Tasks[0].InstructionBase != original || got.Tasks[0].InstructionRevisions[0].BeforeSHA != v.BeforeSHA || got.Planning.Activations != w.Planning.Activations || got.Planning.Decisions != w.Planning.Decisions || reviewContract(&got.Tasks[0]) != reviewContract(&w.Tasks[0]) {
		t.Fatal("changed original history, budgets or contract")
	}
}

func TestInstructionRevisionOperatorNextAllowsFurtherLearning(t *testing.T) {
	s, w, _ := revisedInstructionFixture(t)
	updated := applyTest(t, s, w, "task.update", Request{Schema: 1, EventID: "operator-input", Revision: w.Revision, ID: "future", Next: "Paquet externe vérifié à importer.\n\n" + w.Tasks[0].Next})
	updated = planningDo(t, s, updated, "resume", PlanningRequest{Scope: "root", Reason: "Nouvelle entrée publique attribuée pour la fixture"})
	w, r := planningClaim(t, s, updated, "root")
	r.Operations = []PlanningOperation{{Kind: "revise-instructions", ID: "future", Next: "Distinguer la revue courante des avis historiques avant de décider."}}
	got, err := s.planningChange(w.ID, "decide", r)
	if err != nil {
		t.Fatal("operator input disabled further learning", err)
	}
	if !strings.Contains(got.Tasks[0].Next, "Paquet externe") || strings.Count(got.Tasks[0].Next, got.Tasks[0].InstructionRevisions[0].Addition) != 1 || len(got.Tasks[0].InstructionRevisions) != 2 {
		t.Fatal("lost or duplicated instructions")
	}
}

func TestInstructionRevisionLegacyOperatorPrefixReconciles(t *testing.T) {
	s, w, _ := revisedInstructionFixture(t)
	v := w.Tasks[0].InstructionRevisions[0]
	// Reproduce the live pre-fix persisted T7 state in this isolated fixture only.
	changed, err := s.mutate(w.ID, "fixture-legacy-prefix", "legacy-prefix", w.Revision, []byte(`{}`), func(w *Work) error {
		w.Tasks[0].Next = "Entrée opérateur historique conservée.\n\n" + w.Tasks[0].Next
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	got, err := reopened.planningChange(w.ID, "reject-instructions", PlanningRequest{Schema: 1, EventID: "reject-legacy", Revision: changed.Revision, Task: "future", InstructionRevisionID: v.ID, Reason: "Rejet après relecture des entrées opérateur"})
	if err != nil {
		t.Fatal("recognizable persisted composition cannot recover", err)
	}
	if !strings.Contains(got.Tasks[0].Next, "Entrée opérateur historique") || strings.Contains(got.Tasks[0].Next, v.Addition) {
		t.Fatal("legacy input lost")
	}
}

func TestInstructionRevisionOperatorReplacementRetainsLearning(t *testing.T) {
	s, w, _ := revisedInstructionFixture(t)
	v := w.Tasks[0].InstructionRevisions[0]
	original := *w.Tasks[0].InstructionBase
	const replacement = "Nouvelle base opérateur avec des sources réellement jointes."
	w = applyTest(t, s, w, "task.update", Request{Schema: 1, EventID: "replace-operator-base", Revision: w.Revision, ID: "future", Next: replacement})
	if w.Tasks[0].Next != v.Addition+"\n\n"+replacement || *w.Tasks[0].InstructionBase != original {
		t.Fatal("replacement lost learning or original history")
	}
	got, err := s.planningChange(w.ID, "reject-instructions", PlanningRequest{Schema: 1, EventID: "reject-after-replacement", Revision: w.Revision, Task: "future", InstructionRevisionID: v.ID, Reason: "Retirer uniquement cet apprentissage IA"})
	if err != nil || got.Tasks[0].Next != replacement {
		t.Fatal("replacement not preserved after rejection", err)
	}
}

func TestInstructionRevisionAmbiguousOperatorEditIsAtomic(t *testing.T) {
	s, w, _ := revisedInstructionFixture(t)
	addition := w.Tasks[0].InstructionRevisions[0].Addition
	_, err := s.mutate(w.ID, "task.update", "ambiguous-operator-edit", w.Revision, []byte(`{}`), func(changed *Work) error {
		return s.apply(changed, "task.update", Request{ID: "future", Next: "Début modifié.\n\n" + addition + "\n\nFin remplacée."})
	})
	if err == nil {
		t.Fatal("ambiguous learning placement accepted")
	}
	got, err := s.get(w.ID)
	if err != nil || got.Revision != w.Revision || got.Tasks[0].Next != w.Tasks[0].Next || got.Tasks[0].InstructionOperatorBase != nil {
		t.Fatal("refused edit changed persisted composition", err)
	}
}
