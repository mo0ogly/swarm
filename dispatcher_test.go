//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func dispatchTest(tasks []Task, agents []Agent) dispatchInputs {
	in := dispatchInputs{
		work: &Work{Schema: 1, ID: "w-test", Tasks: tasks}, agents: agents,
		autonomy: autonomyAuto, slots: 2,
		depsReady: map[string]bool{}, priority: map[string]int{},
		profile: &LaunchProfile{Provider: "fixture", Role: "worker", Workspace: "/tmp/ws"},
	}
	for _, t := range tasks {
		in.depsReady[t.ID] = true
	}
	return in
}

func dispatchedTasks(list []dispatchDecision) []string {
	out := []string{}
	for _, d := range list {
		out = append(out, d.TaskID)
	}
	return out
}

func TestDispatchRequiresAutonomy(t *testing.T) {
	for _, level := range []string{autonomyManual, autonomyAssisted} {
		t.Run(level, func(t *testing.T) {
			in := dispatchTest([]Task{{ID: "t1", Status: "todo"}}, nil)
			in.autonomy = level
			list, reason := planDispatch(in)
			if len(list) != 0 {
				t.Fatalf("niveau %q : aucun départ automatique attendu, %v", level, dispatchedTasks(list))
			}
			if !strings.Contains(reason, "autonomie") {
				t.Fatalf("motif attendu sur le niveau d'autonomie : %q", reason)
			}
		})
	}
}

func TestDispatchRespectsPause(t *testing.T) {
	in := dispatchTest([]Task{{ID: "t1", Status: "todo"}}, nil)
	in.paused = true
	list, reason := planDispatch(in)
	if len(list) != 0 {
		t.Fatalf("départs suspendus : %v", dispatchedTasks(list))
	}
	if !strings.Contains(reason, "suspendus") {
		t.Fatalf("motif attendu sur la suspension : %q", reason)
	}
}

func TestDispatchFillsSlotsOnly(t *testing.T) {
	tasks := []Task{}
	for _, id := range []string{"t1", "t2", "t3"} {
		tasks = append(tasks, Task{ID: id, Status: "todo", Profile: &LaunchProfile{Provider: "fixture", Role: "worker", Workspace: "/tmp/ws-" + id}})
	}
	in := dispatchTest(tasks, nil)
	list, _ := planDispatch(in)
	if len(list) != 2 {
		t.Fatalf("deux créneaux, deux départs : %v", dispatchedTasks(list))
	}
	// Une tentative vivante occupe un créneau.
	in.agents = []Agent{{ID: "a1", TaskID: "t1", Status: "running", Heartbeat: now(), CWD: "/tmp/ws-t1"}}
	in.work.Tasks[0].Status = "running"
	list, _ = planDispatch(in)
	if len(list) != 1 {
		t.Fatalf("un seul créneau libre : %v", dispatchedTasks(list))
	}
}

func TestDispatchWaitsForDependencies(t *testing.T) {
	in := dispatchTest([]Task{{ID: "t1", Status: "todo"}, {ID: "t2", Status: "todo", Depends: []string{"t1"}}}, nil)
	in.depsReady["t2"] = false
	list, _ := planDispatch(in)
	if got := dispatchedTasks(list); len(got) != 1 || got[0] != "t1" {
		t.Fatalf("dépendance non acceptée : %v", got)
	}
}

func TestDispatchNeedsLaunchProfile(t *testing.T) {
	in := dispatchTest([]Task{{ID: "t1", Status: "todo"}}, nil)
	in.profile = nil
	list, reason := planDispatch(in)
	if len(list) != 0 {
		t.Fatalf("sans profil de lancement : %v", dispatchedTasks(list))
	}
	if !strings.Contains(reason, "profil") {
		t.Fatalf("motif attendu sur le profil : %q", reason)
	}
}

func TestDispatchPrefersTaskProfile(t *testing.T) {
	tasks := []Task{{ID: "t1", Status: "todo", Profile: &LaunchProfile{Provider: "codex", Role: "planner", Workspace: "/tmp/t1"}}}
	list, _ := planDispatch(dispatchTest(tasks, nil))
	if len(list) != 1 || list[0].Profile.Provider != "codex" || list[0].Profile.Role != "planner" {
		t.Fatalf("le profil de la tâche prime sur celui du travail : %+v", list)
	}
}

func TestDispatchStopsAfterTwoAutomaticFailures(t *testing.T) {
	task := Task{ID: "t1", Status: "blocked", Blocker: "échec"}
	one := []Agent{{ID: "a1", TaskID: "t1", Status: "failed", Origin: originConductor}}
	if list, _ := planDispatch(dispatchTest([]Task{task}, one)); len(list) != 1 {
		t.Fatalf("une seule tentative automatique échouée : la reprise reste permise, %v", dispatchedTasks(list))
	}
	two := append(one, Agent{ID: "a2", TaskID: "t1", Status: "interrupted", Origin: originConductor})
	list, reason := planDispatch(dispatchTest([]Task{task}, two))
	if len(list) != 0 {
		t.Fatalf("deux tentatives automatiques infructueuses : plus de départ, %v", dispatchedTasks(list))
	}
	if !strings.Contains(reason, "tentatives automatiques") {
		t.Fatalf("motif attendu sur le plafond de tentatives : %q", reason)
	}
}

func TestDispatchIgnoresOperatorFailures(t *testing.T) {
	// Les échecs des tentatives lancées à la main n'épuisent pas le plafond
	// automatique : l'opérateur reste libre de confier la reprise au moteur.
	task := Task{ID: "t1", Status: "blocked", Blocker: "échec"}
	agents := []Agent{
		{ID: "a1", TaskID: "t1", Status: "failed", Origin: originOperator},
		{ID: "a2", TaskID: "t1", Status: "failed", Origin: originOperator},
	}
	if list, _ := planDispatch(dispatchTest([]Task{task}, agents)); len(list) != 1 {
		t.Fatalf("reprise attendue après des échecs opérateur : %v", dispatchedTasks(list))
	}
}

func TestDispatchNeverRetriesRefusedHandoff(t *testing.T) {
	// Tentative terminée normalement dont le handoff n'a pas pu être relayé :
	// relancer ne produirait rien de neuf, c'est une décision humaine.
	task := Task{ID: "t1", Status: "blocked", Blocker: "handoff et validation requis"}
	agents := []Agent{{ID: "a1", TaskID: "t1", Status: "completed", Origin: originConductor,
		Relay: "Relais refusé par le conducteur : aucun rapport lisible"}}
	list, reason := planDispatch(dispatchTest([]Task{task}, agents))
	if len(list) != 0 {
		t.Fatalf("un refus de relais ne se relance pas : %v", dispatchedTasks(list))
	}
	if !strings.Contains(reason, "relais") && !strings.Contains(reason, "candidate") {
		t.Fatalf("motif attendu : %q", reason)
	}
}

func TestDispatchSkipsSettledAndActiveTasks(t *testing.T) {
	tasks := []Task{
		{ID: "t1", Status: "accepted"},
		{ID: "t2", Status: "submitted"},
		{ID: "t3", Status: "running"},
		{ID: "t4", Status: "abandoned"},
		{ID: "t5", Status: "todo"},
	}
	list, _ := planDispatch(dispatchTest(tasks, nil))
	if got := dispatchedTasks(list); len(got) != 1 || got[0] != "t5" {
		t.Fatalf("seule une tâche à faire se lance : %v", got)
	}
}

func TestDispatchOrdersByPriorityThenGraph(t *testing.T) {
	tasks := []Task{
		{ID: "profond", Status: "todo", Depends: []string{"racine"}},
		{ID: "racine", Status: "accepted"},
		{ID: "urgent", Status: "todo"},
	}
	in := dispatchTest(tasks, nil)
	in.slots = 1
	in.priority["urgent"] = 9
	in.priority["profond"] = 1
	list, _ := planDispatch(in)
	if got := dispatchedTasks(list); len(got) != 1 || got[0] != "urgent" {
		t.Fatalf("priorité non respectée : %v", got)
	}
}

func TestDispatchRefusesOverlappingWorkspace(t *testing.T) {
	// Deux tâches partageant le même espace de travail ne peuvent pas tourner
	// ensemble : le moteur refuserait le second départ.
	tasks := []Task{{ID: "t1", Status: "todo"}, {ID: "t2", Status: "todo"}}
	in := dispatchTest(tasks, nil)
	list, reason := planDispatch(in)
	if len(list) != 1 {
		t.Fatalf("espaces de travail identiques : un seul départ, %v — %q", dispatchedTasks(list), reason)
	}
	if !strings.Contains(reason, "espace de travail") {
		t.Fatalf("motif attendu sur le recouvrement : %q", reason)
	}
}

func TestAutonomyLevelIsStoredAndFrozen(t *testing.T) {
	s := storeTest(t)
	// createdWork n'impose aucun niveau : on observe ici le défaut réel.
	w := taskTest(t, s, createdWork(t, s))
	if level := s.autonomy(w.ID); level != autonomyAuto {
		t.Fatalf("niveau par défaut attendu autonome, obtenu %q", level)
	}
	if e := s.setAutonomy(w.ID, autonomyAssisted, 3); e != nil {
		t.Fatal(e)
	}
	if level := s.autonomy(w.ID); level != autonomyAssisted {
		t.Fatalf("niveau non conservé : %q", level)
	}
	if slots := s.slots(w.ID); slots != 3 {
		t.Fatalf("créneaux non conservés : %d", slots)
	}
	if e := s.setAutonomy(w.ID, "inconnu", 1); e == nil {
		t.Fatal("un niveau inconnu doit être refusé")
	}
}

func TestDispatchLaunchesFromCapturedProfile(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := s.setAutonomy(w.ID, autonomyAuto, slotsDefault); e != nil {
		t.Fatal(e)
	}
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "Suite", Deliverable: "rapport", Criteria: []string{"preuves"}, Owner: "fixture", Next: "lancer"})
	r.Revision = w.Revision
	r.Workspace = filepath.Join(s.root, "ws-t1")
	if e := os.MkdirAll(r.Workspace, 0700); e != nil {
		t.Fatal(e)
	}
	r.Instruction = "Consigne de référence"
	a, _, e := s.prepare(w.ID, r)
	if e != nil {
		t.Fatal(e)
	}
	// Le premier lancement humain fixe le profil réutilisable du travail.
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if current.Profile == nil || current.Profile.Provider != "fixture" || current.Profile.Instruction != "Consigne de référence" {
		t.Fatalf("profil de lancement non conservé : %+v", current.Profile)
	}
	if stored, _ := s.agent(a.ID); stored.Origin != originOperator {
		t.Fatalf("origine du lancement humain : %q", stored.Origin)
	}
	if e = s.supervise(a.ID); e != nil {
		t.Fatal(e)
	}

	// La fin de la tentative libère un créneau : l'ordonnanceur part de lui-même,
	// sans attendre une action humaine.
	agents, e := s.agents(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	found := false
	for _, x := range agents {
		if x.TaskID == "t2" {
			found = true
			if x.Origin != originConductor {
				t.Fatalf("origine automatique attendue : %q", x.Origin)
			}
			if x.Provider != "fixture" {
				t.Fatalf("profil non réutilisé : %+v", x)
			}
		}
	}
	if !found {
		t.Fatal("aucune tentative enregistrée pour t2")
	}
	// Rejeu explicite : une tâche déjà partie ne repart pas.
	again, e := s.dispatch(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if len(again) != 0 {
		t.Fatalf("second départ sur la même tâche : %+v", again)
	}
}

func TestDispatchStaysIdleWhenPaused(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := s.setAutonomy(w.ID, autonomyAuto, slotsDefault); e != nil {
		t.Fatal(e)
	}
	r.Workspace = filepath.Join(s.root, "ws-t1")
	if e := os.MkdirAll(r.Workspace, 0700); e != nil {
		t.Fatal(e)
	}
	if _, _, e := s.prepare(w.ID, r); e != nil {
		t.Fatal(e)
	}
	if e := s.pause(w.ID, true); e != nil {
		t.Fatal(e)
	}
	launched, e := s.dispatch(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if len(launched) != 0 {
		t.Fatalf("suspension prioritaire : %+v", launched)
	}
}

func TestDispatchHoldsAfterOperatorStop(t *testing.T) {
	// Un arrêt demandé reste une décision : l'ordonnanceur ne relance pas
	// derrière l'opérateur, sinon sa reprise explicite se heurte au moteur.
	task := Task{ID: "t1", Status: "blocked", Blocker: "Arrêt demandé par opérateur"}
	agents := []Agent{{ID: "a1", TaskID: "t1", Status: "interrupted", StopKind: originOperator, Origin: originOperator}}
	list, reason := planDispatch(dispatchTest([]Task{task}, agents))
	if len(list) != 0 {
		t.Fatalf("relance après un arrêt opérateur : %v", dispatchedTasks(list))
	}
	if !strings.Contains(reason, "opérateur") {
		t.Fatalf("motif attendu : %q", reason)
	}
	// Un arrêt technique reste repris automatiquement.
	agents[0].StopKind = "delai"
	if list, _ := planDispatch(dispatchTest([]Task{task}, agents)); len(list) != 1 {
		t.Fatalf("un dépassement de délai se reprend : %v", dispatchedTasks(list))
	}
}

func TestSetProfileAllowsDispatchWithoutFirstManualLaunch(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if e := s.setAutonomy(w.ID, autonomyAuto, slotsDefault); e != nil {
		t.Fatal(e)
	}
	// Aucun lancement manuel : le profil est enregistré directement.
	if e := s.setProfile(w.ID, "", LaunchProfile{Provider: r.Provider, Workspace: s.root, Instruction: "Consigne du plan"}, -1); e != nil {
		t.Fatal(e)
	}
	current, e := s.get(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if current.Profile == nil || current.Profile.Role != "worker" || current.Profile.Actor != originOperator {
		t.Fatalf("profil enregistré incomplet : %+v", current.Profile)
	}
	launched, e := s.dispatch(w.ID)
	if e != nil {
		t.Fatal(e)
	}
	if len(launched) != 1 || launched[0].TaskID != "t1" {
		t.Fatalf("départ automatique attendu sans lancement manuel préalable : %+v", launched)
	}
}

func TestSetProfileRefusesUnusableValues(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	cases := map[string]LaunchProfile{
		"fournisseur absent":    {Workspace: s.root},
		"fournisseur inconnu":   {Provider: "inexistant", Workspace: s.root},
		"rôle inconnu":          {Provider: "fixture", Role: "chef", Workspace: s.root},
		"workspace hors projet": {Provider: "fixture", Workspace: "/etc"},
	}
	for nom, profil := range cases {
		t.Run(nom, func(t *testing.T) {
			if e := s.setProfile(w.ID, "", profil, -1); e == nil {
				t.Fatalf("profil accepté à tort : %+v", profil)
			}
		})
	}
	if current, _ := s.get(w.ID); current.Profile != nil {
		t.Fatalf("un profil refusé ne doit rien enregistrer : %+v", current.Profile)
	}
}

// Le seuil de coût est la seule borne qui change le comportement du moteur :
// elle empêche une dépense à venir. Elle n'annule rien et n'accepte rien.
func TestDispatchHoldsTaskOverCostThreshold(t *testing.T) {
	base := func() dispatchInputs {
		in := dispatchTest([]Task{{ID: "t1", Status: "todo"}}, nil)
		in.reserve = 3.0
		return in
	}

	sous := base()
	sous.taskCost = map[string]CostTotal{"t1": {Reported: 4.00, WithCost: 2}}
	if list, _ := planDispatch(sous); len(list) != 1 {
		t.Fatalf("sous le seuil, le départ reste permis : %v", dispatchedTasks(list))
	}

	au := base()
	au.taskCost = map[string]CostTotal{"t1": {Reported: 6.40, WithCost: 2}}
	list, raison := planDispatch(au)
	if len(list) != 0 {
		t.Fatalf("au-delà du seuil, le départ suivant est retenu : %v", dispatchedTasks(list))
	}
	if !strings.Contains(raison, "6.40") || !strings.Contains(raison, "3.00") {
		t.Fatalf("le motif doit chiffrer le dépassement : %q", raison)
	}
	// Le libellé ne doit pas laisser croire à une annulation de ce qui est engagé.
	if !strings.Contains(raison, "n'est pas annulée") {
		t.Fatalf("le motif doit dire que la dépense engagée subsiste : %q", raison)
	}

	// Sans réserve configurée, aucun seuil : ne pas déduire de plafond.
	sansReserve := base()
	sansReserve.reserve = 0
	sansReserve.taskCost = map[string]CostTotal{"t1": {Reported: 99.0, WithCost: 3}}
	if list, _ := planDispatch(sansReserve); len(list) != 1 {
		t.Fatal("aucune réserve configurée : aucun seuil applicable")
	}

	// Un coût non rapporté ne vaut pas un coût nul : il ne déclenche pas le seuil
	// mais ne l'écarte pas non plus pour les tentatives qui, elles, rapportent.
	muet := base()
	muet.taskCost = map[string]CostTotal{"t1": {Silent: 5}}
	if list, _ := planDispatch(muet); len(list) != 1 {
		t.Fatal("des tentatives muettes ne doivent pas retenir un départ")
	}
}

// Une lecture de coût qui échoue n'est pas une absence de dépassement. Tant
// qu'elle était avalée, le plafond disparaissait en silence et l'ordonnanceur
// repartait sans borne ; l'écran des décisions perdait le sujet de la même
// façon. Les deux doivent s'arrêter, comme ils le font déjà pour les autres
// lectures de leur fonction.
func TestCostReadFailureStopsDispatchAndDecisions(t *testing.T) {
	s := storeTest(t)
	w, _ := setupAgent(t, s)
	// La table des réservations n'est lue, dans dispatch, que par la lecture du
	// budget : la faire disparaître isole ce chemin, alors que supprimer la
	// table des agents ferait échouer une lecture antérieure et ne prouverait
	// rien du repli choisi ici.
	if _, e := s.db.Exec("DROP TABLE reservations"); e != nil {
		t.Fatal(e)
	}
	if _, e := s.dispatch(w.ID); e == nil {
		t.Fatal("réserve illisible : l'ordonnancement doit s'arrêter, pas repartir sans plafond")
	}
}
