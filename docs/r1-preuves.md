# R1 — Fiabiliser les états des preuves

Lot étroit, mission w-5757437164b50815bee50a0c, candidat 21e306154d38476ccf79682b23767e2a77e56414.
Périmètre : **uniquement** `evidence_projection.go` et `evidence_contract_test.go`. `mission_status.go` non touché (hors lot).

## Origine des changements

Diff lu avant reprise entre cette copie et `/home/fpizzi/workspace/swarm/.swarm/managed/w-5757437164b50815bee50a0c/copies/p4-reprise-2/{evidence_projection.go,evidence_contract_test.go}`.
Le candidat p4-reprise-2 contenait déjà exactement les 4 correctifs demandés sur ces deux fichiers, avec tests associés. Réutilisés tels quels après lecture complète du diff (`diff -u`, deux fichiers) ; aucune autre partie de p4-reprise-2 n'a été touchée.

Vérification post-application : `diff -u evidence_projection.go .../p4-reprise-2/evidence_projection.go` → exit 0 (identique). Idem pour `evidence_contract_test.go`.

## Défauts corrigés dans `evidence_projection.go`

1. **Agrégation `Controls.State` dépendait de l'ordre du slice** : un item `unknown` traité après un item `failed` écrasait l'état `failed`. Un item `failed` traité après pouvait aussi écraser un `unknown` sans qu'un `failed` antérieur soit protégé. Remplacé par une passe qui accumule `hasFailed`/`hasUnknown`/`hasPassed` indépendamment de l'ordre, puis résout : `failed` > `unknown` > `passed` > défaut `unknown`.
2. **Liste de contrôles vide déclarée `passed`** : l'ancien code initialisait `e.Controls.State = "passed"` avant la boucle ; une liste vide (`Controls: nil`) ne rentrait jamais dans la boucle et restait donc `passed` par défaut — un succès non démontré. Le nouveau code n'a plus d'état par défaut `passed` ; le cas par défaut (aucun item, ou uniquement des inconnus) est `unknown`.
3. **`exit_code` renseigné à 0 pour un historique `unknown`** : l'ancien code prenait toujours `exit := r.ExitCode` et l'exposait via `&exit`, y compris quand `execution == "unknown"` (commande jamais démarrée, historique ancien). Cela affichait un faux `exit_code: 0`. Le nouveau code ne fixe `exitCode *int` que pour `execution` dans `{executed, not_executed}` ; reste `nil` (→ `null` JSON) pour `unknown`.
4. **Reviewer configuré au niveau du `Work` mais sans verdict pour la tentative courante rapporté `not_configured`** : indistinguable de « aucun reviewer configuré ». Ajout d'un `else if w.Planning != nil && w.Planning.Reviewer != nil` qui rapporte `state: "pending"` avec `reviewer: "reviewer://<provider>"` et une limite explicite, uniquement quand aucun `IndependentReview` n'existe encore pour la tâche.

## Tests ajoutés dans `evidence_contract_test.go`

- `TestEvidenceContractControlAggregationIsOrderIndependent` : rejoue les permutations `{failed,unknown}`, `{unknown,failed}`, `{passed,unknown,failed}`, `{failed,passed}` → exige `Controls.State == "failed"` dans tous les cas (priorité failed indépendante de l'ordre). Teste aussi liste avec un seul `unknown` (→ `unknown`, `exit_code == nil`), liste `nil` (→ `unknown`, jamais `passed` par défaut), liste avec un seul `passed` (→ `passed`).
- `TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured` : `Work.Planning.Reviewer` configuré sans `IndependentReview` → `ReportReview.State == "pending"` (jamais `not_configured`). Contre-cas : `Work` sans `Planning` → `not_configured` conservé.

## Preuve de non-tautologie (falsification)

Avant de conclure, les deux nouveaux tests ont été exécutés contre l'ancien code (via `git stash` temporaire sur `evidence_projection.go` seul, fichier de test inchangé) :

```
=== RUN   TestEvidenceContractControlAggregationIsOrderIndependent
    evidence_contract_test.go:105: failed control masked by order [...]: got state unknown
--- FAIL: TestEvidenceContractControlAggregationIsOrderIndependent (0.01s)
=== RUN   TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured
    evidence_contract_test.go:144: a work-level reviewer configuration must not be reported as not_configured : {State:not_configured ...}
--- FAIL: TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured (0.01s)
```

Confirme que les tests détectent réellement les régressions visées et ne sont pas tautologiques. Le stash a ensuite été restauré (`git stash pop`) et comparé octet à octet (`diff`) avec la copie corrigée conservée à part (`/dev/shm/evidence_projection.go.fixed`) : identique.

## Commandes exécutées et résultats

Toutes lancées depuis `/home/fpizzi/workspace/swarm/.swarm/managed/w-fff7368010d114f6e2578985/copies/r1-preuves-1` avec `TMPDIR=/dev/shm GOTMPDIR=/dev/shm`.

```
go build ./...                                            → silencieux, exit 0
go vet ./...                                               → silencieux, exit 0
go test ./... -run 'TestEvidenceContract' -v               → 6 PASS (dont les 2 nouveaux), exit 0
node tests/final_acceptance.cjs --case proofs              → exit 0
node tests/final_acceptance.cjs --case nope                → "unknown final acceptance case: nope", exit 2
node tests/final_acceptance.cjs                             → usage, exit 2
```

Détail des 6 tests `TestEvidenceContract*` :
- `TestEvidenceContractReportClaimIsNotExecutedControl` — PASS
- `TestEvidenceContractExecutedControlAndStaleness` — PASS
- `TestEvidenceContractDoesNotCallAnUnstartedCommandExecuted` — PASS
- `TestEvidenceContractControlAggregationIsOrderIndependent` — PASS (nouveau)
- `TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured` — PASS (nouveau)
- `TestEvidenceContractSameTruthCLIAndWeb` — PASS (non-régression CLI/web)

## `tests/final_acceptance.cjs`

Fichier inexistant dans le dépôt avant ce lot (recherché dans cette copie et dans toutes les copies sœurs de w-5757437164b50815bee50a0c : absent partout). Créé avec un unique cas `proofs`, sur le modèle éprouvé de `tests/audit_acceptance.cjs` (cas `evidence` existant, scope plus large incluant le DOM web) :

- Vérifie la présence de `go.mod`, `evidence_projection.go`, `evidence_contract_test.go` avant toute exécution.
- Exécute `go test -json -count=1 -run '^TestEvidenceContract' .` (commande réelle, pas de simulation).
- Refuse le succès si le flux JSON ne contient aucun enregistrement `run`+`Test` et `pass` de paquet (garde-fou zéro test).
- Refuse le succès si les deux tests ajoutés dans ce lot n'apparaissent pas explicitement comme `pass` dans le flux JSON (empêche qu'un renommage ou une suppression silencieuse des tests ciblés passe inaperçu).
- N'inclut pas le cas `evidence` déjà couvert par `audit_acceptance.cjs` (DOM `web/evidence-contract.js`) : hors périmètre de ce lot, non modifié ici.

## Limites et écarts assumés

- `mission_status.go` incomplet n'a pas été repris, conformément à la consigne ; son état reste tel quel dans cette copie (hors périmètre R1).
- Le fichier `tests/final_acceptance.cjs` est nouveau dans le dépôt (aucune trace antérieure dans les copies sœurs de la mission source) ; il ne couvre que le cas `proofs`. Si d'autres lots doivent y ajouter des cas, ce fichier est le point d'extension attendu par la consigne, mais son existence même est une création de ce lot, pas une reprise.
- Aucun test de bout en bout web (`web/evidence-contract.js`) n'a été relancé ici : hors périmètre déclaré (uniquement les deux fichiers Go). Le cas `evidence` de `tests/audit_acceptance.cjs`, qui couvre ce DOM, reste disponible mais n'a pas été ré-exécuté dans ce lot.
- Aucun commit, push ni modification de la base Swarm n'a été effectué. Le `git stash`/`git stash pop` utilisé pour la preuve de falsification a été appliqué uniquement sur `evidence_projection.go`, dans cette copie de travail, et restauré avant la fin (vérifié par diff).

## Prochaine action

Lot R1 terminé et vérifié sur son périmètre déclaré (les deux fichiers ciblés + rapport + script d'acceptation). Aucune clôture globale de mission déclarée ici : relais du présent rapport au responsable pour intégration, comme prévu par le cadre d'exécution.
