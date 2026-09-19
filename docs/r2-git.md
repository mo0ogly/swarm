# R2 — Attribuer les preuves à la révision Git

Lot étroit, mission w-5757437164b50815bee50a0c, candidat 21e306154d38476ccf79682b23767e2a77e56414.
Copie de travail : `.swarm/managed/w-fff7368010d114f6e2578985/copies/r2-git-1`, HEAD au début de la tentative : `4e1d9b2000d41958fbf9c6350ba9bd798b7c2a3c`.
Périmètre : reçus d'intégration gérée (`managed_integration.go`), leur schéma (`model.go`), leur projection CLI/web (`evidence_projection.go`, `view.go`, `web/evidence-contract.js`) et les tests associés. Aucune reprise de P0–P3 ni des lots R1 existants au-delà de l'ajout du nouveau champ.

## Défaut constaté avant correction

Le chemin d'intégration gérée (`managed_integration.go`) calcule bien un commit Git réel pour la révision candidate testée (`commit-tree`, variable `candidate`), l'exécute, vérifie l'absence de mutation (`diff --exit-code HEAD`), puis publie ce SHA comme pointeur `Planning.Repository.Candidate` et comme `refs/swarm/candidates/<agent>` dans le dépôt bare. Le fichier `receipt.json` déposé sur disque contenait déjà ce SHA sous la clé `candidate_commit`.

Mais la structure `AutomaticValidation` (le reçu exposé via l'API/CLI/web) n'avait **aucun champ dédié** pour ce SHA. Le seul endroit où il apparaissait dans ce reçu était noyé dans le champ `Reason`, une chaîne de prose libre : `"Révision intégrée vérifiée : " + candidate`. Aucune projection CLI/web ne l'exposait comme donnée structurée ; en extraire la valeur aurait nécessité un parsing de texte, explicitement interdit par la mission (« Aucun SHA inventé ou extrait de prose »). Le champ existant nommé `Revision` sur ce même reçu est un entier — la révision métier du `Work` (compteur interne), sans rapport avec un SHA Git.

## Changements

1. **`model.go`** — nouveau champ `AutomaticValidation.CandidateSHA string` (`json:"candidate_sha"`), avec commentaire explicite : c'est le commit Git réellement fusionné/committé/checké-out quand les contrôles ont tourné, jamais dérivé de `Reason`, distinct de `Revision` (compteur métier) et de `At` (horodatage du reçu).
2. **`managed_integration.go`** — la construction du reçu (`target.AutoValidation = &AutomaticValidation{...}`) renseigne désormais `CandidateSHA: candidate` avec la variable Git déjà calculée par `commit-tree` quelques lignes plus haut (même valeur que celle publiée comme pointeur candidat et comme `refs/swarm/candidates/<agent>`). Aucune nouvelle commande Git ajoutée : réutilisation de la valeur déjà vérifiée par le code existant.
3. **`evidence_projection.go`** — nouveau champ `EvidenceControl.CandidateSHA string` (`json:"candidate_sha"`), peuplé par contrôle via `valueOrUnknown(a.CandidateSHA)` (même schéma que le `testedRevision` déjà existant pour `Revision`). `unknownEvidenceControl` renseigne `"unknown"` explicitement. `evidenceText` (rendu texte structuré) affiche désormais `sha_candidat=%s` à côté de `révision=%s`.
4. **`view.go`** — le rendu CLI texte de `swarm work show` (fonction distincte de `evidenceText`, ligne « Contrôle %s : ... ») affiche désormais `sha_candidat=%s` à côté de `révision=%s`.
5. **`web/evidence-contract.js`** — le rendu web (`SwarmEvidenceContract.text`) affiche désormais `SHA candidat <valeur ou unknown>` par contrôle, à partir du même champ JSON `candidate_sha`.
6. Historique ancien : un reçu `AutomaticValidation` sans `CandidateSHA` (valeur zéro `""`) reste projeté comme `"unknown"` par `valueOrUnknown`, jamais fabriqué.

## Tests ajoutés

- **`TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy`** (`evidence_contract_test.go`, unitaire, sans dépôt Git) : un reçu avec `CandidateSHA` renseigné expose la même valeur en projection, distincte du champ `Revision` et de l'horodatage `At`/`ObservedAt` ; un reçu « ancien » sans ce champ (valeur zéro) reste `"unknown"`.
- **`TestManagedIntegrationRecordsRealCandidateSHA`** (`managed_git_test.go`, bout en bout, dépôt Git géré réel via `managedFixture`/`managedCompleted`/`integrateManagedAttempt`) : après une intégration réussie, `AutoValidation.CandidateSHA` égale le pointeur publié (`Planning.Repository.Candidate`) **et** égale la révision lue directement dans le dépôt bare par `git rev-parse refs/swarm/candidates/<agent>` (preuve indépendante du chemin de calcul), diffère de la révision métier convertie en chaîne, et se retrouve bien dans la projection `taskEvidence` côté contrôle.
- Complément dans `TestEvidenceContractExecutedControlAndStaleness` (chemin non-Git, espace partagé standard) : vérifie que ce chemin, qui ne fusionne ni ne committe jamais de candidat, ne fabrique pas de SHA — `CandidateSHA` y reste `"unknown"`.
- **`tests/evidence_contract_test.cjs`** (DOM web) : fixture enrichie d'un `candidate_sha` réel, assertion que le texte rendu contient `SHA candidat <valeur>`, que ce SHA diffère de `revision` sur le même contrôle, et qu'un contrôle `candidate_sha:'unknown'` s'affiche bien `SHA candidat unknown`.

## Preuve de non-tautologie (falsification)

Avant de conclure, les deux tests Go ciblés ont été rejoués contre le code non corrigé, en révertant temporairement (par script, hors `git stash`, fichiers restaurés et comparés byte à byte ensuite) uniquement les deux lignes d'affectation ajoutées dans `managed_integration.go` et `evidence_projection.go` — sans toucher aux fichiers de test ni au nouveau champ de structure dans `model.go` :

```
=== RUN   TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy
    evidence_contract_test.go:177: SHA candidat non exposé : {... CandidateSHA: ...}
--- FAIL: TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy (0.01s)
=== RUN   TestManagedIntegrationRecordsRealCandidateSHA
    managed_git_test.go:125: SHA candidat absent du reçu : &{... CandidateSHA: ... Reason:Révision intégrée vérifiée : a75bee883158c160aa599d8adf4eb6c03f31c163 ...}
--- FAIL: TestManagedIntegrationRecordsRealCandidateSHA (0.28s)
FAIL
```

Le deuxième échec montre explicitement le SHA réel (`a75bee883158c160aa599d8adf4eb6c03f31c163`) toujours présent dans `Reason` (texte libre) mais absent du nouveau champ structuré tant que le correctif n'est pas appliqué — confirmant que les tests détectent la régression visée et ne sont pas tautologiques. Fichiers restaurés depuis une copie de sauvegarde sous `/dev/shm`, puis comparés (`diff -u`) : identiques à l'état corrigé avant de relancer la suite verte.

## Commandes exécutées et résultats

Toutes lancées depuis `/home/fpizzi/workspace/swarm/.swarm/managed/w-fff7368010d114f6e2578985/copies/r2-git-1` avec `TMPDIR=/dev/shm GOTMPDIR=/dev/shm GOCACHE=/dev/shm/swarm-r2-go-cache`.

```
go build ./...                                                                     → silencieux, exit 0
go vet ./...                                                                        → silencieux, exit 0
gofmt -l model.go managed_integration.go evidence_projection.go view.go \
       evidence_contract_test.go managed_git_test.go                               → 2 fichiers signalés, corrigés par gofmt -w, puis liste vide
go test -count=1 -v -run '^(TestEvidenceContract.*|TestManagedIntegrationRecordsRealCandidateSHA|TestManagedIntegrationAtomicAndConflict|TestManagedFailedControlNeverPublishes)$' .
    → 10 PASS (dont les 2 nouveaux), exit 0
node tests/evidence_contract_test.cjs                                               → PASS, exit 0
node tests/final_acceptance.cjs --case git                                          → exit 0, message "git acceptance: managed integration receipt carries the real tested candidate commit, distinct from the business revision and timestamps, unknown for legacy receipts, verified end to end"
node tests/final_acceptance.cjs --case proofs                                       → toujours exit 0 (pas de régression du cas R1)
node tests/final_acceptance.cjs --case nope                                         → "unknown final acceptance case: nope", exit 2
node tests/final_acceptance.cjs                                                     → usage, exit 2
```

Détail des 10 tests Go rejoués après correction et gofmt :
- `TestEvidenceContractReportClaimIsNotExecutedControl` — PASS
- `TestEvidenceContractExecutedControlAndStaleness` — PASS (assertion `CandidateSHA == "unknown"` ajoutée pour le chemin non-Git)
- `TestEvidenceContractDoesNotCallAnUnstartedCommandExecuted` — PASS
- `TestEvidenceContractControlAggregationIsOrderIndependent` — PASS
- `TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured` — PASS
- `TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy` — PASS (nouveau)
- `TestEvidenceContractSameTruthCLIAndWeb` — PASS (non-régression CLI/web, DeepEqual sur `TaskEvidence` incluant le nouveau champ)
- `TestManagedIntegrationAtomicAndConflict` — PASS (non-régression du chemin d'intégration gérée)
- `TestManagedIntegrationRecordsRealCandidateSHA` — PASS (nouveau, bout en bout)
- `TestManagedFailedControlNeverPublishes` — PASS (non-régression)

## `tests/final_acceptance.cjs`

Cas `git` ajouté sur le modèle exact du cas `proofs` existant (créé au lot R1) : dépendances de fichiers vérifiées avant exécution (`go.mod`, `model.go`, `managed_integration.go`, `evidence_projection.go`, `evidence_contract_test.go`, `managed_git_test.go`), exécution réelle de `go test -json -count=1 -run <pattern> .` (aucune simulation), refus explicite si le flux JSON ne contient aucun enregistrement `run`+`Test` et `pass` de paquet (garde-fou zéro test), refus si les tests requis (`TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy`, `TestManagedIntegrationRecordsRealCandidateSHA`) n'apparaissent pas explicitement `pass`. Le message de succès et la liste des tests requis sont désormais paramétrés par cas (`spec.required`, `spec.message`) au lieu d'être codés en dur pour `proofs` uniquement — changement minimal nécessaire pour ajouter un second cas sans dupliquer `runAcceptance`.

## Limites et écarts assumés

- Seul le chemin d'intégration Git géré (`integrateManagedAttempt`) produit un `CandidateSHA` réel. Le chemin standard à espace partagé (`automatic_validation.go`) ne fusionne ni ne committe de candidat Git identifiable par tâche ; il continue de projeter `"unknown"`, ce qui est correct et vérifié (`TestEvidenceContractExecutedControlAndStaleness`), mais ce n'est pas une lacune comblée par ce lot — c'est un fait du système hors périmètre.
- Le champ `CandidateSHA` est renseigné une fois par reçu (`AutomaticValidation`), donc identique pour tous les contrôles d'une même intégration ; il n'y a pas de granularité par contrôle individuel puisque tous les contrôles d'un même reçu tournent sur le même commit checké-out.
- Aucune migration de données existantes : les reçus déjà enregistrés en base avant ce lot n'ont jamais eu ce champ et resteront `"unknown"` pour toujours (comportement voulu, pas un bug à corriger).
- `go test ./...` complet non relancé ici (hors périmètre déclaré et connu instable en bac à sable pour des raisons de sockets, déjà consigné au lot P0) ; seuls les tests ciblés du périmètre R2 et leurs non-régressions immédiates (`TestManagedIntegrationAtomicAndConflict`, `TestManagedFailedControlNeverPublishes`, `TestEvidenceContractSameTruthCLIAndWeb`) ont été exécutés.
- Aucun commit, push ni modification de la base Swarm effectué. Les fichiers temporairement révertés pour la preuve de falsification ont été restaurés depuis une copie de sauvegarde (`/dev/shm`) et vérifiés identiques par `diff` avant de poursuivre.

## Prochaine action

Lot R2-git terminé et vérifié sur son périmètre déclaré : schéma du reçu, chemin d'intégration gérée, projections CLI/web (JSON, texte CLI `work show`, DOM web), tests ciblés avec preuve de falsification, et cas `--case git` de `tests/final_acceptance.cjs`. Aucune clôture globale de mission déclarée ici : relais du présent rapport au responsable pour intégration, comme prévu par le cadre d'exécution.
