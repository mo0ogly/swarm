# T3 — CLI Administration miroir du moteur (REQ-ADM-02)

## Périmètre et identité

Mission w-843bb3ce22cff2965c5e77b6 ; tâche plan-843bb3ce22-T3 ; agent
2f02ea7a-22c1-4d3c-bcbf-1f38560853c5 ; tentative a-28e6f888ed773e88f1928606 ;
départ 2/2 ; révision du travail 110. Racine de travail :
`/home/fpizzi/workspace/swarm-engine-contract/source` (vérifiée par `pwd` et
`git rev-parse --show-toplevel`). Révision de base
097e745a9b1404f16ed6fdf4ad32124a3f661e05, branche `codex/engine-review-contract`
(identique à celle de la tentative précédente — aucun commit intermédiaire).

Producteur initial du code et de ce document : tentative
`auto-540941014719667e1a6e`, départ 1/2, révision du travail 108, état final
`interrupted`. Le code (`run_limits_cli.go`, `run_limits_cli_test.go`) et la
documentation d'usage ci-dessous n'ont pas été modifiés par la présente
tentative ; seules l'identité et la preuve de rejeu ont été mises à jour
(détail dans `docs/plan-843bb3ce22-T3.md`). Le rapport de la tentative
précédente n'est pas, par lui seul, une preuve de réussite de la tentative
courante ; les trois critères ont été revérifiés ci-après et dans le plan.

Entrée de gate (« T2 accepté ») : évidence documentaire lue dans
`docs/T2-contrat-moteur.md` — quatre critères moteur PASS sur SQLite réelle,
correction et contrôles exécutés par le superviseur Codex le 29 septembre 2026.
Aucune commande Swarm de coordination n'a été utilisée pour vérifier cet état
(hors périmètre outillé de cette tentative) ; l'entrée de gate repose donc sur
la lecture de ce rapport, pas sur une requalification moteur.

## Résultat observé

Sous-commande CLI `swarm run-limits` ajoutée (`run_limits_cli.go`), qui
n'implémente aucune règle : elle parse les arguments positionnels et un corps
JSON, puis délègue chaque opération à la même méthode `*Store` que T2 a
écrite et testée dans `run_limits_admin.go` :

| Action CLI | Méthode moteur déléguée (run_limits_admin.go) |
| --- | --- |
| `run-limits show` | `Store.currentRunLimitsConfig` |
| `run-limits history` | `Store.runLimitsHistory` |
| `run-limits apply` | `Store.configureRunLimits` |
| `run-limits rollback` | `Store.rollbackRunLimits` |
| `run-limits effective` | `Store.effectiveRunLimits` |

| Critère | Preuve et résultat |
| --- | --- |
| 1. CLI refuse les mêmes valeurs invalides que le moteur (mêmes cas testés) | `TestRunLimitsCLIRefusesSameInvalidValuesAsEngine` (run_limits_cli_test.go) : rejoue via la CLI les quatre refus et l'acceptation zéro-valide de `TestRunLimitsConfigRejectsInvalidValue` (run_limits_admin_test.go) — max_tool_calls hors bornes, silence_seconds négatif, portée inconnue, portée mission sans identifiant de mission, et l'override tout-zéro accepté. PASS. Preuve manuelle en plus (binaire réel) ci-dessous : même message d'erreur, même code de sortie. |
| 2. Délégation au moteur confirmée par lecture de code, sans logique de validation dupliquée | Extrait de code ci-dessous : `runLimitsCLI` ne contient aucun test de borne, de portée ou de conflit de révision — ces règles restent uniquement dans `validRunLimitsScope`, `validRunLimitsOverride` et `configureRunLimits` (run_limits_admin.go), non modifiés par cette tâche. |
| 3. Doc d'usage CLI à jour | Section « Usage » ci-dessous, alignée sur `main.go`'s texte d'aide (`swarm --help`) et vérifiée par exécution réelle du binaire. |

## Extrait de code — délégation sans logique dupliquée

`run_limits_cli.go` (nouveau fichier), cas `apply` :

```go
case "apply":
	if len(pos) != 5 {
		return fmt.Errorf("swarm run-limits apply <portée> <mission> <clé> --input changement.json")
	}
	raw, err := readInput(input)
	if err != nil {
		return err
	}
	var r RunLimitsConfigChange
	if err = strict(raw, &r); err != nil {
		return err
	}
	if r.Scope != "" && r.Scope != scope {
		return fmt.Errorf("scope du JSON ne correspond pas à la portée en argument")
	}
	if r.Mission != "" && r.Mission != mission {
		return fmt.Errorf("mission_id du JSON ne correspond pas à l'argument")
	}
	if r.Key != "" && r.Key != key {
		return fmt.Errorf("scope_key du JSON ne correspond pas à l'argument")
	}
	r.Scope, r.Mission, r.Key = scope, mission, key
	entry, err := s.configureRunLimits(r)
	if err != nil {
		return err
	}
	return printJSON(out, entry)
```

Toute borne, portée ou conflit de révision est décidé exclusivement par
`s.configureRunLimits` (run_limits_admin.go:173), qui appelle
`validRunLimitsScope`, `validRunLimitsOverride` et compare `expected_revision`
à la révision courante en transaction — code inchangé par cette tâche. La CLI
ne fait que reformater l'erreur retournée (via le `fail()` déjà existant dans
`main.go`, commun à toutes les sous-commandes).

## Usage CLI

```
swarm run-limits show|history <portée> <mission> <clé>
swarm run-limits apply <portée> <mission> <clé> --input changement.json
swarm run-limits rollback <portée> <mission> <clé> <révision_cible> --input requête.json
swarm run-limits effective <mission> <rôle> <tâche>
```

- `<portée>` : `project` | `mission` | `role` | `task` (mêmes constantes que le
  moteur, `ScopeProject`/`ScopeMission`/`ScopeRole`/`ScopeTask`).
- `<mission>` / `<clé>` : identifiant de mission et clé de portée (rôle ou
  tâche selon la portée) ; utiliser `-` pour un champ vide (ex. portée
  `project`, qui n'a ni mission ni clé — même règle que `validRunLimitsScope`).
- `apply --input changement.json` : corps `{"schema_version":1,"event_id":"...",
  "expected_revision":N,"values":{...},"reason":"..."}`, mêmes champs que
  `RunLimitsConfigChange`. `expected_revision=0` exige qu'aucun override
  n'existe encore pour cette portée.
- `rollback ... <révision_cible> --input requête.json` : corps
  `{"event_id":"...","reason":"...","expected_revision":N}` ; revient à la
  valeur de `<révision_cible>` en créant une nouvelle révision (l'historique
  n'est jamais réécrit).
- `effective <mission> <rôle> <tâche>` : résolution hiérarchique courante
  (projet < mission < rôle < tâche), identique à ce que le moteur appliquerait
  à un nouveau départ maintenant.
- `--json` (option globale) : sortie déjà structurée par défaut pour
  `run-limits` (JSON), l'option force le français des messages d'erreur comme
  pour les autres sous-commandes.

### Exemple réel (binaire construit, dépôt temporaire)

```
$ swarm --root "$T" --json run-limits apply project - - --input c.json
{
  "scope": "project",
  "values": { "silence_seconds": 0, "tool_seconds": 0, "max_tool_calls": 50,
              "max_repeated_calls": 0, "max_consecutive_errors": 0 },
  "revision": 1, "actor": "fpizzi/uid:1001", "reason": "test manuel",
  "updated": "2026-09-29T20:37:12.849779655Z"
}
$ echo $?
0
```

Valeur hors bornes, via le même binaire, même règle que le moteur :

```
$ swarm --root "$T" --json run-limits apply project - - --input bad.json
{
  "error": "limite d'agent hors bornes : max_tool_calls doit valoir 0 (héritée) ou 1..10000, reçu 999999",
  "failure": { "code": "command_failed", ... }
}
$ echo $?
2
```

Message identique à celui produit directement par
`validRunLimitsOverride` (run_limits_admin.go:112) : la CLI ne reformule pas
l'erreur moteur.

## Contrôles exécutés

Commande : `go build ./...` — code de sortie 0.
Commande : `go vet ./...` — code de sortie 0 (deux diagnostics `non-constant
format string` corrigés dans `run_limits_cli.go` avant ce résultat).

Commande : `go test ./... -run '^TestRunLimitsCLI' -v -count=1 -timeout 60s`
Code de sortie : 0.

```
=== RUN   TestRunLimitsCLIRefusesSameInvalidValuesAsEngine
--- PASS: TestRunLimitsCLIRefusesSameInvalidValuesAsEngine (0.01s)
=== RUN   TestRunLimitsCLIAppliesShowsHistoryAndRollsBack
--- PASS: TestRunLimitsCLIAppliesShowsHistoryAndRollsBack (0.02s)
PASS
ok  	swarm.local/companion	0.049s
```

Rejoué à l'identique par la tentative `a-28e6f888ed773e88f1928606` (départ
2/2) : même commande, code de sortie 0, 2/2 PASS (sortie brute et détail dans
`docs/plan-843bb3ce22-T3.md`). Suite complète `TestRunLimits*` non rejouée par
cette tentative (hors périmètre borné de la reprise).

Commande : `go test ./... -run '^TestRunLimits' -v -count=1 -timeout 60s`
(famille complète moteur + CLI, non-régression T2) — code de sortie 0, 9/9
PASS : `TestRunLimitsConfigRejectsInvalidValue`,
`TestRunLimitsConfigPersistsAfterRealRestart`,
`TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected`,
`TestRunLimitsConfigConcurrencyAndReplay`,
`TestRunLimitsMigrationKeepsExistingConfiguration`,
`TestRunLimitsCLIRefusesSameInvalidValuesAsEngine`,
`TestRunLimitsCLIAppliesShowsHistoryAndRollsBack`,
`TestRunLimitsActualLaunchFreezesConfiguration`,
`TestRunLimitsLaunchCannotRaiseCeilings`.

Preuve manuelle (binaire réel, dépôt temporaire hors du dépôt de travail,
supprimé après usage) :
- `run-limits apply project - -` avec valeur valide → révision 1, exit 0.
- `run-limits apply project - -` avec `max_tool_calls:999999` → refus, exit 2,
  message identique à celui du moteur.
- `run-limits show project - -` → relit la révision 1 appliquée, exit 0.
- `run-limits show project - -` en tout premier appel sur un dépôt fraîchement
  initialisé (avant tout `apply`) → exit 0, révision 0 (aucun override), donc
  pas de régression « table absente » malgré la migration v23 récente (la
  sous-commande n'est volontairement pas listée dans `cliStorageInspection`,
  pour garder `migrate=true` par défaut sur un dépôt neuf).

## Consultation sans migration implicite — correction du superviseur

Les commandes run-limits show/history/effective sont maintenant listées dans
cliStorageInspection. Sur une ancienne base, elles refusent avec
storage_upgrade_required sans changer le schéma ni créer de sauvegarde.
L'opérateur effectue la migration explicitement via swarm init avant consultation.
Sur un projet déjà initialisé au schéma courant, la lecture fonctionne normalement.
La décision initiale de laisser migrate=true a été corrigée par le superviseur Codex.

## Limites et prochaine action

- Écran web Administration (REQ-ADM-01/04/05/06), quick wins et CI restent
  hors périmètre de T3 (lots distincts du plan).
- `run-limits effective` n'est pas un critère explicite de T3 mais complète le
  miroir (même méthode moteur `effectiveRunLimits`, utile pour REQ-ADM-04 côté
  Lot 3) ; aucune logique propre.
- Revue indépendante et gate de delivery restent à faire par le responsable du
  plan (hors outillage de cette tentative — aucune commande Swarm exécutée
  ici, conformément au cadre d'exécution).
- Fichiers modifiés/ajoutés (empreintes SHA-256) :

```
run_limits_cli.go      fba9eee51d75ac8857d80cfd35c632a0b97353da99c6e0286c4c91f64f9d3f68
run_limits_cli_test.go 5571a45a3dacfadc159b6f000c54599a59d80d09b4e2cca5237ba02cdd7363da
main.go                1b0acf5b40441515507c3a0cd6551ffd898afe6ffbafd62cee29547fd0783d58
```


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
