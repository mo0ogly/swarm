# T2 — Résultat courant : limites administratives appliquées au lancement

## Périmètre et identité

Mission w-843bb3ce22cff2965c5e77b6 ; tâche plan-843bb3ce22-T2.
Production initiale : Claude, auto-b20a4cd7f84cb5da2842, tentative a-e72cd608b58c139aeede6936.
Correction et exécution des contrôles : superviseur Codex, après fin des agents,
le 29 septembre 2026. Révision de base 097e745, modifications locales identifiées
par les empreintes ci-dessous. Aucune nouvelle tentative de production.
L'historique des rapports incomplets est conservé dans RETEX-ADMIN-PREPARATION.md.
Ce document décrit exclusivement le code courant corrigé, et non l'ancien état.

## Résultat observé

Les quatre critères moteur ont une preuve d'exécution sur SQLite temporaire réelle.
Les tests ne lancent pas une IA payante : ils utilisent le vrai Store.prepareLaunch,
les vraies réservations et la vraie table agents, avec un fournisseur de test.
Le parcours Administration CLI/web reste dans T3/T4 ; il n'est pas déclaré livré ici.

| Critère | Preuve et résultat |
| --- | --- |
| 1. Refus de valeur invalide | TestRunLimitsConfigRejectsInvalidValue : refus des valeurs négatives, hors bornes et portée invalide ; héritage zéro autorisé. PASS. |
| 2. Persistance après redémarrage | TestRunLimitsConfigPersistsAfterRealRestart : fermeture réelle de SQLite puis openStore et relecture de 42 appels / révision 1. PASS. |
| 3. Historique et rollback | TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected : trois révisions conservées, retour à la valeur 30, rollback_of=1. TestRunLimitsConfigConcurrencyAndReplay : conflit de révision, rejeu identique idempotent et conflit d'événement. PASS. |
| 4. Futurs départs et tentatives en cours | TestRunLimitsActualLaunchFreezesConfiguration : premier agent réellement réservé à 30 appels ; changement du rôle à 12 ; premier agent relu en base toujours à 30 ; second agent réellement réservé et relu à 12. Sa consigne contient la limite appliquée. PASS. |

## Implémentation du critère 4

Dans agents_store.go, Store.prepareLaunch lit configuredRunLimitsWith(tx, work,
r.Role, r.TaskID) après acquisition du writer SQLite, puis applique
limits = limits.cappedBy(adminLimits) avant les plafonds du plan et de reprise.
La même transaction enregistre l'agent et sa consigne avec Agent.Limits.
Les quatre portées se lisent dans l'ordre projet, mission, rôle, tâche.
Les plafonds fournisseur et demande explicite restent prioritaires : une préférence
900 ne relève pas un plafond 100 ou 7. Aucun agent existant n'est modifié.
Le terminal natif refuse les contraintes administratives qu'il ne sait pas mesurer.

### Code exact du test qui traverse le lancement réel

```go
//go:build linux

package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestRunLimitsActualLaunchFreezesConfiguration(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	r.Workspace = filepath.Join(s.root, "first")
	if err := os.Mkdir(r.Workspace, 0700); err != nil {
		t.Fatal(err)
	}
	changes := []RunLimitsConfigChange{
		{Schema: 1, EventID: "project", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}},
		{Schema: 1, EventID: "mission", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{MaxToolCalls: 50}},
		{Schema: 1, EventID: "role", Scope: ScopeRole, Mission: w.ID, Key: "worker", Values: RunLimits{MaxToolCalls: 40}},
		{Schema: 1, EventID: "task", Scope: ScopeTask, Mission: w.ID, Key: "t1", Values: RunLimits{MaxToolCalls: 30}},
	}
	for _, c := range changes {
		if _, err := s.configureRunLimits(c); err != nil {
			t.Fatal(err)
		}
	}
	first, created, err := s.prepareLaunch(w.ID, r, false)
	if err != nil || !created {
		t.Fatalf("launch: %v", err)
	}
	if first.Limits.MaxToolCalls != 30 || !strings.Contains(first.Prompt, "30 appels d'outils") {
		t.Fatalf("configuration not frozen in launch: %+v", first.Limits)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "role-update", Scope: ScopeRole, Mission: w.ID, Key: "worker", Revision: 1, Values: RunLimits{MaxToolCalls: 12}}); err != nil {
		t.Fatal(err)
	}
	persisted, err := s.agent(first.ID)
	if err != nil || persisted.Limits.MaxToolCalls != 30 {
		t.Fatalf("existing reservation changed: %+v %v", persisted.Limits, err)
	}
	w, err = s.get(w.ID)
	if err != nil {
		t.Fatal(err)
	}
	w = applyTest(t, s, w, "task.add", Request{ID: "t2", Title: "next", Deliverable: "report", Criteria: []string{"proof"}})
	r.TaskID = "t2"
	r.Revision = w.Revision
	r.EventID = newID("agent-")
	r.Workspace = filepath.Join(s.root, "next")
	if err = os.Mkdir(r.Workspace, 0700); err != nil {
		t.Fatal(err)
	}
	second, created, err := s.prepareLaunch(w.ID, r, false)
	if err != nil || !created {
		t.Fatalf("next launch: %v", err)
	}
	if second.Limits.MaxToolCalls != 12 {
		t.Fatalf("next launch ignores changed config: %+v", second.Limits)
	}
	saved, err := s.agent(second.ID)
	if err != nil || saved.Limits != second.Limits {
		t.Fatalf("limits not persisted: %v", err)
	}
}

func TestRunLimitsLaunchCannotRaiseCeilings(t *testing.T) {
	s := storeTest(t)
	w, r := setupAgent(t, s)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "large", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 900}}); err != nil {
		t.Fatal(err)
	}
	a, _, err := s.prepareLaunch(w.ID, r, true)
	if err != nil || a.Limits.MaxToolCalls != 100 {
		t.Fatalf("provider ceiling lost: %+v %v", a.Limits, err)
	}
	r.Limits = &RunLimits{MaxToolCalls: 7}
	a, _, err = s.prepareLaunch(w.ID, r, true)
	if err != nil || a.Limits.MaxToolCalls != 7 {
		t.Fatalf("explicit ceiling lost: %+v %v", a.Limits, err)
	}
}
```

## Sorties capturées directement par le superviseur

Commande : go test ./... -run '^TestRunLimits' -v -count=1 -timeout 60s
Code de sortie : 0.

```text
=== RUN   TestRunLimitsConfigRejectsInvalidValue
--- PASS: TestRunLimitsConfigRejectsInvalidValue (0.02s)
=== RUN   TestRunLimitsConfigPersistsAfterRealRestart
--- PASS: TestRunLimitsConfigPersistsAfterRealRestart (0.02s)
=== RUN   TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected
--- PASS: TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected (0.03s)
=== RUN   TestRunLimitsConfigConcurrencyAndReplay
--- PASS: TestRunLimitsConfigConcurrencyAndReplay (0.02s)
=== RUN   TestRunLimitsActualLaunchFreezesConfiguration
--- PASS: TestRunLimitsActualLaunchFreezesConfiguration (0.06s)
=== RUN   TestRunLimitsLaunchCannotRaiseCeilings
--- PASS: TestRunLimitsLaunchCannotRaiseCeilings (0.05s)
PASS
ok  	swarm.local/companion	0.211s
```

Suite complète après correction des migrations :
commande go test ./... -count=1 -timeout 12m ; code 0.

```text
ok  	swarm.local/companion	368.466s
```

Contrôles supplémentaires : go vet ./... et git diff --check, codes 0 ;
tests ciblés avec -race, code 0 (5.616 s), journal /tmp/swarm-admin-final-race.log.
La migration v23 conserve les tables existantes et leur historique ;
TestRunLimitsMigrationKeepsExistingConfiguration couvre cette réouverture.
Contrôles de migration ciblés : code 0 (1.686 s).

## Contre-épreuve : le test échoue sans le correctif

Avec un overlay de compilation temporaire supprimant uniquement
limits = limits.cappedBy(adminLimits), sans modifier le dépôt :
go test -overlay /tmp/swarm-admin-regression-overlay.json ./... -run '^TestRunLimitsActualLaunchFreezesConfiguration$' -count=1 -timeout 60s
Code 1 attendu et observé :

```text
--- FAIL: TestRunLimitsActualLaunchFreezesConfiguration (0.04s)
    run_limits_launch_test.go:35: configuration not frozen in launch: {SilenceSeconds:180 ToolSeconds:300 MaxToolCalls:100 MaxRepeatedCalls:4 MaxConsecutiveErrors:3}
FAIL
FAIL	swarm.local/companion	0.054s
FAIL
```

## Révision des fichiers contrôlés (SHA-256)

```json
{
  "agents_store.go": "7fb34953605d2de6d7ed26e0bafc6ab0e6737c1d6e4a222fd9d86bb80c65373f",
  "run_limits_admin.go": "9c2eabbc4b382099490526888486cf37bb516f23ecff81b750cb0efd80f49abf",
  "run_limits_admin_test.go": "901e2594c55e71010deff03b57bed268e8822feb3cb60bdb6d4c66ec1da66b05",
  "run_limits_launch_test.go": "fad62b09fed4d68046e57f633bb9ab2e07a5987a92f5e9884dbf656055dd09b2",
  "review_dialog.go": "b42231a369b278eccd526ede3df6ec971fa32684a23d0ddd71f7bacc4438a224",
  "report_submission_identity_test.go": "18e48e5788c61bcfdd7e8c9c3a925153321e4df54415f53602896ac08976bb44",
  "store.go": "bdbda0b866848dc4c13ef5ae13887c40e02de28d5846e230798347a62722d237",
  "model.go": "ad518b97cad8c6b34d5bf2b2c6fa38c5eadc2e3d835f49cb2f21e545296d20ac",
  "provider_cooldown_test.go": "39d4c23aba994d33f3b5a17855f9c05cb9f8a26cbeb8a9d6f45a3d7c296a1dd7"
}```

## Livraison et limites

Build installé : 761403e8a3b47f80c5bd7a3b3805513e3294b30fc8275fc46584fc28e1430192.
Le moteur applique le correctif ; aucune acceptation de tâche n'a été forcée.
La revue indépendante évalue les pièces présentes ici ; elle n'a pas d'outils
pour rejouer les tests. Cette limitation ne signifie pas que le branchement soit
encore absent : son code et son test de régression sont fournis ci-dessus.
La décision de validation et les écrans CLI/web sont distincts du comportement
moteur démontré par ces tests d'intégration.


## Code complet des contrôles des critères 1, 2 et 3

Source exécutée par les commandes ci-dessus : run_limits_admin_test.go.
Inclut la fermeture et réouverture réelle de SQLite, les valeurs invalides,
les assertions sur chaque révision et rollback_of, ainsi que le rejeu.

```go
//go:build linux

package main

import "testing"

func TestRunLimitsConfigRejectsInvalidValue(t *testing.T) {
	s := storeTest(t)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 999999}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of out-of-bounds max_tool_calls")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e2", Scope: ScopeProject, Values: RunLimits{SilenceSeconds: -1}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of negative silence_seconds")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e3", Scope: "invalid-scope", Values: RunLimits{}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of unknown scope")
	}
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e4", Scope: ScopeMission, Mission: "", Values: RunLimits{}, Reason: "test"}); err == nil {
		t.Fatal("expected rejection of mission scope without a mission id")
	}
	// A valid all-zero override (pure inheritance) must be accepted.
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "e5", Scope: ScopeProject, Values: RunLimits{}, Reason: "no-op"}); err != nil {
		t.Fatalf("all-zero override should be accepted: %v", err)
	}
}

func TestRunLimitsConfigPersistsAfterRealRestart(t *testing.T) {
	s := storeTest(t)
	entry, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "set1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 42}, Reason: "abaisser le plafond global"})
	if err != nil {
		t.Fatal(err)
	}
	if entry.Revision != 1 {
		t.Fatalf("revision=%d", entry.Revision)
	}
	root := s.root
	if err = s.db.Close(); err != nil {
		t.Fatal(err)
	}
	reopened, err := openStore(root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer reopened.db.Close()
	got, err := reopened.currentRunLimitsConfig(ScopeProject, "", "")
	if err != nil {
		t.Fatal(err)
	}
	if got.Values.MaxToolCalls != 42 || got.Revision != 1 {
		t.Fatalf("not persisted across a real reopen of the store: %+v", got)
	}
	eff, err := reopened.effectiveRunLimits("", "", "")
	if err != nil {
		t.Fatal(err)
	}
	if eff.MaxToolCalls != 42 {
		t.Fatalf("effective resolution after restart ignores persisted override: %+v", eff)
	}
}

func TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)

	base, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if base.MaxToolCalls != 100 {
		t.Fatalf("built-in default changed underneath the test: %d", base.MaxToolCalls)
	}
	// An attempt frozen right now, before any admin change (Agent.Limits is a
	// value copy in the running engine, never a pointer into live config).
	frozen := base

	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "proj1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}, Reason: "réduire le défaut global"}); err != nil {
		t.Fatal(err)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "mission1", Scope: ScopeMission, Mission: w.ID, Values: RunLimits{MaxToolCalls: 50}, Reason: "mission plus stricte"}); err != nil {
		t.Fatal(err)
	}
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "role1", Scope: ScopeRole, Mission: w.ID, Key: "worker", Values: RunLimits{MaxToolCalls: 40}, Reason: "rôle worker plus strict"}); err != nil {
		t.Fatal(err)
	}
	taskEntry, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "task1", Scope: ScopeTask, Mission: w.ID, Key: "t1", Values: RunLimits{MaxToolCalls: 30}, Reason: "tâche T1 plus stricte"})
	if err != nil {
		t.Fatal(err)
	}

	future, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if future.MaxToolCalls != 30 {
		t.Fatalf("most specific (task) override not applied, got %d", future.MaxToolCalls)
	}
	sibling, err := s.effectiveRunLimits(w.ID, "worker", "t2")
	if err != nil {
		t.Fatal(err)
	}
	if sibling.MaxToolCalls != 40 {
		t.Fatalf("sibling task should fall back to role scope, got %d", sibling.MaxToolCalls)
	}
	otherRole, err := s.effectiveRunLimits(w.ID, "planner", "t3")
	if err != nil {
		t.Fatal(err)
	}
	if otherRole.MaxToolCalls != 50 {
		t.Fatalf("other role should fall back to mission scope, got %d", otherRole.MaxToolCalls)
	}

	// REQ-ADM-05: the value frozen before any of these changes stays untouched.
	if frozen.MaxToolCalls != 100 {
		t.Fatalf("changing configuration mutated an already-frozen attempt: %+v", frozen)
	}

	hist, err := s.runLimitsHistory(ScopeTask, w.ID, "t1")
	if err != nil {
		t.Fatal(err)
	}
	if len(hist) != 1 || hist[0].Revision != 1 || hist[0].Values.MaxToolCalls != 30 {
		t.Fatalf("unexpected history after first write: %+v", hist)
	}

	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "task2", Scope: ScopeTask, Mission: w.ID, Key: "t1", Revision: taskEntry.Revision, Values: RunLimits{MaxToolCalls: 10}, Reason: "encore plus strict"}); err != nil {
		t.Fatal(err)
	}
	tightened, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if tightened.MaxToolCalls != 10 {
		t.Fatalf("second override not applied: %d", tightened.MaxToolCalls)
	}

	rolled, err := s.rollbackRunLimits(ScopeTask, w.ID, "t1", 1, "task-rollback", "", 2)
	if err != nil {
		t.Fatal(err)
	}
	if rolled.Values.MaxToolCalls != 30 {
		t.Fatalf("rollback restored wrong value: %+v", rolled)
	}
	if rolled.Revision != 3 {
		t.Fatalf("rollback must create a new revision, not rewrite history: got %d", rolled.Revision)
	}
	after, err := s.effectiveRunLimits(w.ID, "worker", "t1")
	if err != nil {
		t.Fatal(err)
	}
	if after.MaxToolCalls != 30 {
		t.Fatalf("effective limits not rolled back: %d", after.MaxToolCalls)
	}
	hist2, err := s.runLimitsHistory(ScopeTask, w.ID, "t1")
	if err != nil {
		t.Fatal(err)
	}
	if len(hist2) != 3 {
		t.Fatalf("rollback must keep every prior revision in history, got %d entries", len(hist2))
	}
	if hist2[0].RollbackOf != 1 {
		t.Fatalf("latest history entry should record which revision it rolled back to: %+v", hist2[0])
	}

	// Still frozen: unaffected by the tighten-then-rollback sequence too.
	if frozen.MaxToolCalls != 100 {
		t.Fatalf("frozen attempt mutated after rollback: %+v", frozen)
	}

	if _, err = s.rollbackRunLimits(ScopeTask, w.ID, "t1", 99, "task-rollback-bad", "", 3); err == nil {
		t.Fatal("expected rollback to a non-existent revision to fail")
	}
}

func TestRunLimitsConfigConcurrencyAndReplay(t *testing.T) {
	s := storeTest(t)
	first, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 50}, Reason: "r"})
	if err != nil {
		t.Fatal(err)
	}
	// Stale expected revision (still 0) now conflicts with the current revision (1).
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c2", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 60}, Reason: "r"}); err == nil {
		t.Fatal("expected a revision conflict")
	}
	// Replaying the exact same event is idempotent: no new revision.
	replay, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 50}, Reason: "r"})
	if err != nil {
		t.Fatal(err)
	}
	if replay.Revision != first.Revision {
		t.Fatalf("identical replay must not create a new revision: got %d want %d", replay.Revision, first.Revision)
	}
	// Same event id, different content, is refused rather than silently applied.
	if _, err = s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "c1", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 51}, Reason: "r"}); err == nil {
		t.Fatal("expected event_id reuse with different content to be refused")
	}
}

func TestRunLimitsMigrationKeepsExistingConfiguration(t *testing.T) {
	s := storeTest(t)
	if _, err := s.configureRunLimits(RunLimitsConfigChange{Schema: 1, EventID: "before-upgrade", Scope: ScopeProject, Values: RunLimits{MaxToolCalls: 42}}); err != nil {
		t.Fatal(err)
	}
	if _, err := s.db.Exec("PRAGMA user_version=22"); err != nil {
		t.Fatal(err)
	}
	if err := s.db.Close(); err != nil {
		t.Fatal(err)
	}
	next, err := openStore(s.root, false)
	if err != nil {
		t.Fatal(err)
	}
	defer next.db.Close()
	config, err := next.currentRunLimitsConfig(ScopeProject, "", "")
	if err != nil || config.Values.MaxToolCalls != 42 || config.Revision != 1 {
		t.Fatalf("configuration lost: %+v %v", config, err)
	}
	history, err := next.runLimitsHistory(ScopeProject, "", "")
	if err != nil || len(history) != 1 || history[0].Values.MaxToolCalls != 42 {
		t.Fatalf("history lost: %+v %v", history, err)
	}
}
```
