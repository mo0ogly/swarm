# Plan T3 — CLI Administration miroir du moteur (reprise)

## Identité de cette tentative

Mission w-843bb3ce22cff2965c5e77b6 ; tâche plan-843bb3ce22-T3 ; agent
2f02ea7a-22c1-4d3c-bcbf-1f38560853c5 ; tentative a-28e6f888ed773e88f1928606 ;
départ 2/2 ; révision du travail 110. Racine de travail vérifiée :
`/home/fpizzi/workspace/swarm-engine-contract/source` (`pwd` et
`git rev-parse --show-toplevel` concordants). Révision de base
`097e745a9b1404f16ed6fdf4ad32124a3f661e05`, branche `codex/engine-review-contract`,
arbre de travail non nettoyé (nombreux fichiers modifiés/non suivis d'autres
tâches du plan, hors périmètre de T3 — préservés, non touchés).

Tentative précédente : `auto-540941014719667e1a6e`, état `interrupted`, départ
1/2, révision du travail 108 (voir `docs/T3-cli-admin.md`, section « Périmètre
et identité »). Cette tentative n'est pas une restauration implicite : elle
reprend la même tâche depuis l'état réellement observé sur le disque, pas
depuis le rapport précédent seul.

## Historique du producteur initial

`run_limits_cli.go` et `run_limits_cli_test.go` existaient déjà avant cette
tentative (non suivis git, présents sur disque), produits par la tentative
`auto-540941014719667e1a6e`. Aucune modification de code applicatif n'a été
nécessaire ni effectuée par cette reprise : les trois critères assignés
(req-8, req-9, req-10) étaient déjà couverts par ce code et par
`docs/T3-cli-admin.md` existant. Cette reprise relit ce code, rejoue le test
ciblé, et complète l'identité/historique dans les deux livrables.

## Critères assignés (état à l'ouverture de cette tentative)

| ID | Critère | État avant reprise |
| --- | --- | --- |
| req-8 | CLI refuse les mêmes valeurs invalides que le moteur (mêmes cas testés) | Couvert par `TestRunLimitsCLIRefusesSameInvalidValuesAsEngine`, non revérifié par cette identité de tentative avant aujourd'hui |
| req-9 | Délégation au moteur confirmée par lecture de code, sans logique de validation dupliquée | Documenté dans `docs/T3-cli-admin.md`, non relu par cette tentative avant aujourd'hui |
| req-10 | Doc d'usage CLI à jour | `docs/T3-cli-admin.md` existant, identité de tentative obsolète (référence l'ancienne tentative) |

## Actions de cette tentative

1. Vérification `pwd` / `git rev-parse --show-toplevel` — racine confirmée.
2. Vérification d'existence (sans `cat` aveugle) de `docs/T3-cli-admin.md`,
   `docs/plan-843bb3ce22-T3.md` (absent), `run_limits_cli.go`,
   `run_limits_cli_test.go` — tous présents sauf le plan.
3. Lecture complète de `docs/T3-cli-admin.md`, `run_limits_cli.go`,
   `run_limits_cli_test.go`.
4. Exécution unique : `go test ./... -run '^TestRunLimitsCLI' -count=1 -v -timeout 60s`.
5. `git rev-parse HEAD` + `git status --porcelain` — révision de base
   confirmée identique à celle du rapport existant (097e745a...), code T3
   toujours non suivi et non modifié depuis le rapport précédent.
6. Rédaction de ce plan et mise à jour de `docs/T3-cli-admin.md`.

## Preuve du test rejoué

Commande : `go test ./... -run '^TestRunLimitsCLI' -count=1 -v -timeout 60s`
(exécutée deux fois : une fois filtrée via rtk, une fois via `rtk proxy` pour
la sortie brute complète). Code de sortie : 0 dans les deux cas.

```
=== RUN   TestRunLimitsCLIRefusesSameInvalidValuesAsEngine
--- PASS: TestRunLimitsCLIRefusesSameInvalidValuesAsEngine (0.01s)
=== RUN   TestRunLimitsCLIAppliesShowsHistoryAndRollsBack
--- PASS: TestRunLimitsCLIAppliesShowsHistoryAndRollsBack (0.02s)
PASS
ok  	swarm.local/companion	0.057s
```

2/2 PASS, cohérent avec le rapport existant (9/9 sur la famille complète
`TestRunLimits`, non rejoué ici — hors périmètre borné de cette reprise, qui
ne devait relire que `run_limits_cli.go` et `run_limits_cli_test.go`).

## Résultat des trois critères (cette tentative)

| Critère | Résultat |
| --- | --- |
| req-8 | PASS — `TestRunLimitsCLIRefusesSameInvalidValuesAsEngine` rejoue via la CLI les quatre refus + l'acceptation zéro-valide de `TestRunLimitsConfigRejectsInvalidValue` (run_limits_admin_test.go), rejoué ci-dessus. |
| req-9 | PASS — relecture de `run_limits_cli.go` confirmée : aucune borne, portée ou règle de révision n'y est testée ; chaque `case` appelle directement `s.currentRunLimitsConfig` / `s.runLimitsHistory` / `s.configureRunLimits` / `s.rollbackRunLimits` / `s.effectiveRunLimits` (run_limits_admin.go). Voir extrait de code dans `docs/T3-cli-admin.md`. |
| req-10 | PASS — `docs/T3-cli-admin.md` mis à jour (identité de cette tentative, historique du producteur initial) ; section « Usage CLI » inchangée, toujours alignée sur le code relu ci-dessus. |

## Délégation moteur — confirmation de lecture (cette tentative)

Relecture de `run_limits_cli.go` (125 lignes) : la fonction `runLimitsCLI`
parse uniquement `pos` (arguments positionnels) et `input` (chemin JSON), sans
aucune comparaison de borne numérique, de nom de portée valide ou de révision
attendue. Chaque branche du `switch action` retourne directement l'erreur du
appel moteur (`s.configureRunLimits`, etc.) sans la réécrire. La seule logique
locale est la correspondance `scope`/`mission`/`key` entre l'URL positionnelle
et le corps JSON (si le corps répète un champ, il doit correspondre à
l'argument — ce n'est pas une règle de validation métier mais une garde de
cohérence d'entrée, absente du moteur car celui-ci n'a pas d'arguments
positionnels séparés du corps).

## Limites et prochaine action

- Aucun fichier applicatif modifié par cette tentative (conforme à la
  consigne de reprise : « ne pas refaire l'exploration ni modifier le code
  applicatif »).
- Suite complète `TestRunLimits*` non rejouée par cette tentative (seul
  `TestRunLimitsCLI` était dans le périmètre autorisé) ; le rapport existant
  en gardait déjà une preuve datée (9/9 PASS).
- Revue indépendante et gate de delivery restent à faire par le responsable
  du plan — aucune commande Swarm de coordination exécutée ici, aucune
  acceptation déclarée, aucune modification de `.swarm/state.db`.
- Prochaine action : transmission au responsable du plan pour revue
  indépendante ; si accepté, T3 peut clôturer avec les deux livrables
  (`docs/T3-cli-admin.md`, ce plan) comme preuve datée de cette tentative.


## Contrôle supplémentaire du superviseur

Après la fin normale de la reprise, Codex a ajouté la protection de consultation
pour les trois sous-commandes et leurs cas dans le test partagé.
Commande : go test ./... -run 'TestInspection|TestRunLimitsCLI' -count=1 -v -timeout 90s.
Code 0 ; sortie réelle :

```text
=== RUN   TestRunLimitsCLIRefusesSameInvalidValuesAsEngine
--- PASS: TestRunLimitsCLIRefusesSameInvalidValuesAsEngine (0.02s)
=== RUN   TestRunLimitsCLIAppliesShowsHistoryAndRollsBack
--- PASS: TestRunLimitsCLIAppliesShowsHistoryAndRollsBack (0.03s)
=== RUN   TestInspectionCLIRefusesImplicitStorageUpgrade
--- PASS: TestInspectionCLIRefusesImplicitStorageUpgrade (0.08s)
=== RUN   TestInspectionDoesNotCreateMissingStore
--- PASS: TestInspectionDoesNotCreateMissingStore (0.00s)
PASS
ok  	swarm.local/companion	0.168s
```

Source du contrôle de non-migration :

```go
package main

import (
	"bytes"
	"database/sql"
	"path/filepath"
	"strings"
	"testing"
)

func TestInspectionCLIRefusesImplicitStorageUpgrade(t *testing.T) {
	s, w := storeTest(t), Work{}
	w = createTest(t, s)
	if _, err := s.db.Exec("PRAGMA user_version=21"); err != nil {
		t.Fatal(err)
	}
	for _, args := range [][]string{{"run-limits", "show", "project", "-", "-"}, {"run-limits", "history", "project", "-", "-"}, {"run-limits", "effective", w.ID, "worker", "t1"}, {"work", "show", w.ID}, {"work", "list"}, {"planning", "show", w.ID}, {"mission", "status", w.ID}, {"agent", "list", w.ID}, {"agent", "logs", "absent"}, {"prepare", "list"}, {"providers", "show"}} {
		var out, errOut bytes.Buffer
		code := run(append([]string{"--root", s.root, "--json"}, args...), &out, &errOut)
		if code != 2 || !strings.Contains(out.String()+errOut.String(), "storage_upgrade_required") {
			t.Fatalf("%v: code%d out%s err%s", args, code, out.String(), errOut.String())
		}
		var version int
		if err := s.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != 21 {
			t.Fatalf("inspection migrated storage: %d %v", version, err)
		}
	}
	backups, _ := filepath.Glob(filepath.Join(s.root, ".swarm", "state-pre-v22-*.db"))
	if len(backups) != 0 {
		t.Fatal("inspection produced migration backup")
	}
	var out, errOut bytes.Buffer
	if code := run([]string{"--root", s.root, "init"}, &out, &errOut); code != 0 {
		t.Fatal(code, out.String(), errOut.String())
	}
	var version int
	if err := s.db.QueryRow("PRAGMA user_version").Scan(&version); err != nil || version != schemaVersion {
		t.Fatal(version, err)
	}
	out.Reset()
	errOut.Reset()
	if code := run([]string{"--root", s.root, "--json", "work", "show", w.ID}, &out, &errOut); code != 0 {
		t.Fatal(code, errOut.String())
	}
}

func TestInspectionDoesNotCreateMissingStore(t *testing.T) {
	root := t.TempDir()
	if s, err := openStoreWithMigration(root, false, false); err == nil {
		s.db.Close()
		t.Fatal("missing store created")
	}
	db, err := sql.Open("sqlite", "file:"+filepath.Join(root, ".swarm", "state.db")+"?mode=ro")
	if err == nil {
		defer db.Close()
		if err = db.Ping(); err == nil {
			t.Fatal("database exists")
		}
	}
}
```

La suite complète finale a réussi ; résultat ci-dessous.


## Pièces complètes pour la revue des trois critères

Code exact des tests CLI exécutés :

```go
//go:build linux

package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func writeTempInput(t *testing.T, raw []byte) string {
	t.Helper()
	path := filepath.Join(t.TempDir(), "run-limits.json")
	if err := os.WriteFile(path, raw, 0600); err != nil {
		t.Fatal(err)
	}
	return path
}

// TestRunLimitsCLIRefusesSameInvalidValuesAsEngine mirrors, through the CLI
// entry point, every rejection TestRunLimitsConfigRejectsInvalidValue
// exercises directly against the store (run_limits_admin_test.go): an
// out-of-bounds max_tool_calls, a negative silence_seconds, an unknown
// scope and a mission scope without a mission id. Same cases, same refusal;
// the CLI adds no bound of its own.
func TestRunLimitsCLIRefusesSameInvalidValuesAsEngine(t *testing.T) {
	s := storeTest(t)

	run := func(scope, mission, key string, body map[string]any) error {
		raw, err := json.Marshal(body)
		if err != nil {
			t.Fatal(err)
		}
		path := writeTempInput(t, raw)
		var out bytes.Buffer
		return s.runLimitsCLI([]string{"run-limits", "apply", scope, mission, key}, path, &out)
	}

	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e1", "values": map[string]any{"max_tool_calls": 999999}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of out-of-bounds max_tool_calls via CLI")
	}
	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e2", "values": map[string]any{"silence_seconds": -1}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of negative silence_seconds via CLI")
	}
	if err := run("invalid-scope", "-", "-", map[string]any{"schema_version": 1, "event_id": "e3", "values": map[string]any{}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of unknown scope via CLI")
	}
	if err := run(ScopeMission, "-", "-", map[string]any{"schema_version": 1, "event_id": "e4", "values": map[string]any{}, "reason": "test"}); err == nil {
		t.Fatal("expected rejection of mission scope without a mission id via CLI")
	}
	// A valid all-zero override (pure inheritance) must be accepted, exactly
	// as it is directly against the store.
	if err := run(ScopeProject, "-", "-", map[string]any{"schema_version": 1, "event_id": "e5", "values": map[string]any{}, "reason": "no-op"}); err != nil {
		t.Fatalf("all-zero override should be accepted via CLI: %v", err)
	}
}

// TestRunLimitsCLIAppliesShowsHistoryAndRollsBack exercises the full admin
// cycle (apply, show, history, rollback) through the CLI, delegating to the
// same Store methods and asserting the same observable effects T2's tests
// assert directly, plus event_id replay idempotency and revision conflict.
func TestRunLimitsCLIAppliesShowsHistoryAndRollsBack(t *testing.T) {
	s := storeTest(t)
	w := createTest(t, s)

	apply := func(eventID string, rev int, maxCalls int, reason string) (RunLimitsConfigEntry, error) {
		body, _ := json.Marshal(map[string]any{"schema_version": 1, "event_id": eventID, "expected_revision": rev, "values": map[string]any{"max_tool_calls": maxCalls}, "reason": reason})
		path := writeTempInput(t, body)
		var out bytes.Buffer
		if err := s.runLimitsCLI([]string{"run-limits", "apply", ScopeTask, w.ID, "t1"}, path, &out); err != nil {
			return RunLimitsConfigEntry{}, err
		}
		var entry RunLimitsConfigEntry
		if err := json.Unmarshal(out.Bytes(), &entry); err != nil {
			t.Fatal(err)
		}
		return entry, nil
	}

	first, err := apply("task1", 0, 30, "tâche T1 plus stricte")
	if err != nil {
		t.Fatal(err)
	}
	if first.Revision != 1 || first.Values.MaxToolCalls != 30 {
		t.Fatalf("unexpected apply result: %+v", first)
	}

	var showOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "show", ScopeTask, w.ID, "t1"}, "-", &showOut); err != nil {
		t.Fatal(err)
	}
	var shown RunLimitsConfigEntry
	if err = json.Unmarshal(showOut.Bytes(), &shown); err != nil {
		t.Fatal(err)
	}
	if shown.Revision != 1 || shown.Values.MaxToolCalls != 30 {
		t.Fatalf("show mismatch with apply: %+v", shown)
	}

	var effOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "effective", w.ID, "worker", "t1"}, "-", &effOut); err != nil {
		t.Fatal(err)
	}
	var eff RunLimits
	if err = json.Unmarshal(effOut.Bytes(), &eff); err != nil {
		t.Fatal(err)
	}
	if eff.MaxToolCalls != 30 {
		t.Fatalf("effective resolution via CLI ignores task override: %+v", eff)
	}

	// Stale expected_revision conflicts, exactly like the engine test.
	if _, err = apply("task2", 0, 10, "encore plus strict"); err == nil {
		t.Fatal("expected a revision conflict via CLI")
	}
	second, err := apply("task2", 1, 10, "encore plus strict")
	if err != nil {
		t.Fatal(err)
	}
	if second.Revision != 2 || second.Values.MaxToolCalls != 10 {
		t.Fatalf("second apply unexpected: %+v", second)
	}

	// Replaying the exact same event_id with the same content is idempotent.
	replay, err := apply("task2", 1, 10, "encore plus strict")
	if err != nil {
		t.Fatal(err)
	}
	if replay.Revision != second.Revision {
		t.Fatalf("identical replay via CLI must not create a new revision: got %d want %d", replay.Revision, second.Revision)
	}

	var histOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "history", ScopeTask, w.ID, "t1"}, "-", &histOut); err != nil {
		t.Fatal(err)
	}
	var hist []RunLimitsHistoryEntry
	if err = json.Unmarshal(histOut.Bytes(), &hist); err != nil {
		t.Fatal(err)
	}
	if len(hist) != 2 {
		t.Fatalf("unexpected history length via CLI: %d", len(hist))
	}

	rollbackBody, _ := json.Marshal(map[string]any{"event_id": "task-rollback", "expected_revision": 2, "reason": "task-rollback"})
	rollbackPath := writeTempInput(t, rollbackBody)
	var rollbackOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "rollback", ScopeTask, w.ID, "t1", "1"}, rollbackPath, &rollbackOut); err != nil {
		t.Fatal(err)
	}
	var rolled RunLimitsConfigEntry
	if err = json.Unmarshal(rollbackOut.Bytes(), &rolled); err != nil {
		t.Fatal(err)
	}
	if rolled.Revision != 3 || rolled.Values.MaxToolCalls != 30 {
		t.Fatalf("rollback via CLI unexpected result: %+v", rolled)
	}

	// Rolling back to a non-existent revision fails, same as the engine test.
	badRollbackPath := writeTempInput(t, rollbackBody)
	var badOut bytes.Buffer
	if err = s.runLimitsCLI([]string{"run-limits", "rollback", ScopeTask, w.ID, "t1", "99"}, badRollbackPath, &badOut); err == nil {
		t.Fatal("expected rollback to a non-existent revision to fail via CLI")
	}
}
```

Aide du binaire compilé, capturée directement par le superviseur :
commande /tmp/swarm-t3-supervisor --help, code 0.

```text
swarm run-limits show|history <portée> <mission> <clé>
swarm run-limits apply <portée> <mission> <clé> --input changement.json
swarm run-limits rollback <portée> <mission> <clé> <révision_cible> --input requête.json
swarm run-limits effective <mission> <rôle> <tâche>
```


## Vérification finale du superviseur

Commande : go test ./... -timeout 12m ; code de sortie 0.

```text
ok  	swarm.local/companion	364.374s
```

Go vet, compilation et git diff --check : codes 0.
Révision des cinq fichiers CLI contrôlés, inchangés depuis ces vérifications :

```text
fba9eee51d75ac8857d80cfd35c632a0b97353da99c6e0286c4c91f64f9d3f68  run_limits_cli.go
5571a45a3dacfadc159b6f000c54599a59d80d09b4e2cca5237ba02cdd7363da  run_limits_cli_test.go
1b0acf5b40441515507c3a0cd6551ffd898afe6ffbafd62cee29547fd0783d58  main.go
0cae5813813143ceff827b22651224d766d0db94e053e89403c24714d1e8007e  storage_inspection.go
850ae823c4c7f5b7d0cd38852a815d56d5a1b2bb282f6aece05fbf95de6a6e4c  storage_inspection_test.go
```


## Comparaison directe CLI / moteur — critère 1

Voici le test moteur de référence complet, en complément du test CLI déjà fourni.
Dans les deux tests : max_tool_calls=999999 refusé, silence_seconds=-1 refusé,
invalid-scope refusé, portée mission sans mission refusée, tous champs zéro acceptés.
Les entrées et les attentes sont identiques ; le CLI passe par configureRunLimits.

```go
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

```
