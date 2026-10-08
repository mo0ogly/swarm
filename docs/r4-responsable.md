# R4 — Reprendre le responsable sans boucle

Date de la tentative : 19 septembre 2026
Périmètre : chemin de décision hiérarchique du responsable (`planning.go`), lot R4 uniquement. Ne refait ni P0-P3 ni R1-R3.

## Verdict borné

Deux exigences du lot étaient déjà couvertes par le code existant, une ne l'était pas.

1. **Acquittement sans opération des anciens événements de tentative** — déjà implémenté dans `planning.go` (`decide`, ligne ~408-424) : quand `Operations` est vide, le contrôle de fraîcheur de tentative (`retour d'une tentative périmée`) et le contrôle des artefacts sont contournés, l'événement est marqué décidé. Aucun code changé ici ; un test nouveau le démontre et prouve en plus que ce chemin reste borné (voir ci-dessous).
2. **Guider vers `retry` plutôt que `task` pour une exigence déjà confiée** — **manquant**. Contrairement à l'opération `delegate` (qui refuse déjà une exigence "déjà confiée à une tâche"), l'opération `task` n'avait aucun contrôle équivalent : rien n'empêchait le responsable de créer une nouvelle tâche pour une exigence déjà portée par une tâche **bloquée**, au lieu d'utiliser `retry` sur cette tâche. Corrigé.

Le correctif a d'abord été écrit de façon trop large (toute exigence déjà portée par n'importe quelle tâche, peu importe son état) ; il a fait échouer deux tests préexistants légitimes (`TestPlanningSharedBudgetAndInheritedChecks`, `TestPlanningHandoffIdentityAndNewDiscovery`) qui créent délibérément plusieurs tâches partageant une même exigence tant qu'aucune n'est bloquée (lot initial, découverte lors d'une remise). Le correctif a été resserré pour ne déclencher que lorsqu'une tâche existante du périmètre portant cette exigence est **`blocked`** ; les 24 tests `TestPlanning*` passent alors, y compris les deux régressés.

## Changement appliqué

`planning.go`, `applyPlanningOperation`, cas `"task"` (juste avant `checkScopeTask`) :

```go
// A requirement already confided to a task that is now blocked must be
// resumed via "retry" on that task, never re-delegated to a fresh "task"
// op: otherwise unbounded new tasks pay for fresh activations on the same
// requirement instead of correcting and retrying the one already stuck.
// Tasks that are not blocked may still legitimately share a requirement
// (e.g. an initial batch, or a discovery handoff from a running task).
for _, task := range w.Tasks {
    if task.ScopeID == id && task.Status == "blocked" {
        for _, req := range task.Requirements {
            if seen[req] {
                return fmt.Errorf("exigence déjà confiée à la tâche bloquée %s ; utiliser retry", task.ID)
            }
        }
    }
}
```

Aucun autre fichier de production modifié.

## Tests fournis

Ajoutés dans `planning_test.go` :

- `TestPlanningTaskForConfidedRequirementRefusedGuidingRetry` — un contrôle bloque une tâche (« Intégration échouée : contrôle X en échec »), génère l'événement `attempt_ended` correspondant ; une seconde opération `task` réutilisant la même exigence est refusée ; la même décision, remplacée par une opération `retry` sur la tâche bloquée, réussit et aucune tâche dupliquée n'est créée.
- `TestPlanningStaleAttemptEventAcknowledgedWithoutOperationBeforeCurrentReturn` — une tentative échoue (`attempt-1`), puis une seconde tentative la remplace avant que le responsable n'ait traité le premier retour (`attempt-2`). Une décision qui tente une opération `retry` sur l'événement périmé (`attempt-1`) est refusée (« retour d'une tentative périmée »). La même entrée, décidée sans opération, est acceptée (acquittement) et ne coûte qu'une seule décision. Le retour courant (`attempt-2`) est alors traité seul ; une tentative de `retry` au-delà de `plan_max_attempts` (2 tentatives déjà consommées) reste refusée — preuve que la reprise est bornée et ne boucle pas.

### Preuve que le test correctif n'est pas tautologique

```sh
git stash push -- planning.go   # retire uniquement le correctif, tests en place
TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-r4-gocache \
  go test -count=1 -run '^TestPlanningTaskForConfidedRequirementRefusedGuidingRetry$' -v .
git stash pop
```

Résultat observé sans le correctif : **1 failed** — `planning_test.go:492: second task for an already-confided requirement was accepted instead of guiding retry`. Avec le correctif restauré, le même test passe. `TestPlanningStaleAttemptEventAcknowledgedWithoutOperationBeforeCurrentReturn` ne dépend pas du correctif (chemin de code distinct) ; il documente un comportement préexistant correct.

## Contrôles réellement exécutés

| Commande | Résultat observé |
| --- | --- |
| `go build ./...` avec caches sous `/dev/shm` | code 0 |
| `go vet ./...` avec caches sous `/dev/shm` | code 0, aucune sortie |
| `go test -count=1 -run '^TestPlanning' -v .` | code 0, **24 tests passés** (22 préexistants + 2 nouveaux) |
| `go test -count=1 -run '^(TestManaged\|TestEvidenceContract\|TestCursorContract\|TestAutomaticValidation\|TestOrganization)' -v .` | code 0, 36 tests passés (aucune régression hors périmètre) |
| `go test -count=1 -run '^TestPlanningTaskForConfidedRequirementRefusedGuidingRetry$' -v .` sur `planning.go` **non corrigé** (`git stash`) | code non nul, **1 failed** (preuve ci-dessus) |
| `TMPDIR=/dev/shm GOTMPDIR=/dev/shm node tests/final_acceptance.cjs --case planner` | code 0, 2 tests Go exécutés et passés, message d'acceptation affiché |
| `node tests/final_acceptance.cjs --case proofs` | code 0, message R1 inchangé |
| `node tests/final_acceptance.cjs --case git` | code 0, message R2 inchangé |
| `node tests/final_acceptance.cjs --case diagnostic` | code 0, message R3 inchangé |

Commande exacte du cas ciblé de ce lot :

```sh
TMPDIR=/dev/shm GOTMPDIR=/dev/shm node tests/final_acceptance.cjs --case planner
```

## Refus conservés

Aucun refus existant n'a été affaibli. Vérifié explicitement par la suite `TestPlanning*` complète (24/24) : le refus de créer une tâche pour une exigence hors périmètre (`TestPlanningRejectsUnownedRequirementAndUnprovedClosure`), le refus de clôture sans preuve fraîche (`TestPlanningClosureRequiresFreshEvidenceAndReopens`), le refus de décision avec bail périmé (`TestPlanningLeaseFencesOldOwnerAndPause`), le refus de dépassement de budget de décisions/activations (`TestPlanningRejectsUnownedRequirementAndUnprovedClosure/budget`, `TestPlanningNeverBypassesMissionFinancialCap`) et le refus de rejouer une décision différente sur le même événement (`TestPlanningAtomicDecisionReplayAndRestart`) passent tous inchangés. Le nouveau refus (« exigence déjà confiée à la tâche bloquée … ; utiliser retry ») s'ajoute sans en retirer aucun.

## Limites

- Le déclencheur du nouveau refus est `task.Status == "blocked"` au moment de la décision. Une exigence confiée à une tâche `todo`, `running` ou `accepted` peut toujours recevoir une seconde tâche : ce n'est pas un bug résiduel mais un choix délibéré, confirmé nécessaire par deux tests préexistants (`TestPlanningSharedBudgetAndInheritedChecks` : lot initial de deux tâches sur la même exigence ; `TestPlanningHandoffIdentityAndNewDiscovery` : découverte de portée pendant qu'une tâche tourne encore). Le correctif ne couvre donc que le cas « tâche bloquée, en attente de correction », qui est le cas décrit par la mission (« exigence déjà confiée » après un retour de tentative en échec).
- Le correctif ne modifie pas le budget `plan_max_attempts` (fixé à 2 lors de la création de la tâche, `planning.go` ligne 562) ; il ne fait qu'empêcher de contourner ce budget en créant une tâche parallèle. Une fois `plan_max_attempts` atteint, `retry` reste lui aussi refusé (« reprise bornée exigeant une nouvelle correction explicite ») — démontré dans le second test, pas de mécanisme de dérogation ajouté ici.
- Aucun fournisseur IA réel n'a été invoqué ; les événements d'intégration échouée et de tentative sont simulés directement sur l'état (`s.mutate` + `planningAttemptEnded`), au même niveau que les tests préexistants du fichier. Le chemin réel bout-en-bout (`managed_integration.go` → `task.Blocker` → événement `attempt_ended`) n'est pas ré-exercé ici ; il est couvert par les tests R3 (`docs/r3-diagnostic.md`), non modifiés.
- La suite Go complète (`go test ./...`) n'a pas été relancée ; seules les suites ciblées listées ci-dessus l'ont été, conformément au périmètre étroit demandé. Les limites de sandbox (sockets Unix/TCP refusés) consignées en P0 restent valables et non revérifiées dans ce lot.

## Handoff au responsable

Changements à relire : `planning.go` (un bloc de 10 lignes dans `applyPlanningOperation`, cas `"task"`), `planning_test.go` (2 tests nouveaux), `tests/final_acceptance.cjs` (cas `planner` ajouté, usage mis à jour). Aucun contournement de contrôle, aucune modification de `.swarm/state.db`, aucun push.

Risque principal : le correctif initial (non resserré) aurait cassé deux comportements légitimes existants si le resserrement au statut `blocked` n'avait pas été appliqué et vérifié par la suite complète `TestPlanning*` — c'est documenté ci-dessus précisément pour qu'une revue puisse vérifier que le compromis (bloqué seulement, pas toute exigence) est le bon.

Prochaine action recommandée : revue indépendante du diff, en particulier de la portée du nouveau refus (`blocked` uniquement) ; puis exécution de la suite Go complète dans un environnement autorisant les sockets locaux pour confirmer l'absence de régression hors suites ciblées.
