# Banc de facturation — observations sur le moteur hiérarchique (tâche 8H.0)

Essai jetable du 5 octobre 2026, hors dépôt (script supprimé après usage). Binaire :
`bin/swarm` construit par `make build` depuis la branche `feat/billing-bench`
(commit `2ef5330`, code Go identique à `main` `53f2564`). Le harnais de référence
`tests/organized_coordination_process.py bin/swarm <dossier> nominal` passe (PASS, 29 s).

L'essai reprend `tests/organized_fixture.py` (responsable scripté, aucun modèle) avec
deux tâches `prepare` puis `settle` (dépendance), une exigence et un contrôle par tâche,
et un fournisseur qui écrit `docs/<tâche>.md` dans son répertoire courant uniquement.
Quatre scénarios : espace propre (`racine/<tâche>`), espace partagé (`racine`),
agent de `prepare` sortant en 137, contrôle de `settle` sortant toujours en 3.

## 1. Chemin de l'artefact remis

La remise (`planning.inbox`, `kind == "handoff"`) porte un chemin **relatif à la racine
du projet**, calculé depuis l'espace de la tentative
(`conductor.go:148-171`, `provenAttemptReport` : recherche dans l'espace, puis
`filepath.Rel(s.root, …)`).

| Espace de la tentative | `artifacts[0].path` observé |
|---|---|
| propre (`racine/prepare`) | `prepare/docs/prepare.md` |
| partagé (`racine`) | `docs/prepare.md` |

Extrait (espace propre) :
`{"kind": "handoff", "task": "prepare", "attempt": "a-2a1ea257…", "artifacts": [{"path": "prepare/docs/prepare.md", "sha256": "dd4417d2…"}]}`

**Conséquence : en espace propre, la tâche n'est jamais acceptée** si le rapport
n'existe que dans l'espace de la tentative. Observé : `prepare` reste `submitted`,
périmètre `waiting`, aucun contrôle exécuté pendant 60 s. Cause, dans l'ordre :

1. Le conducteur relaie la remise puis lance la validation automatique
   (`conductor.go:54`) ; elle est retenue car la revue indépendante n'a pas encore
   eu lieu (`independent_review.go:53-54`). Journal de l'agent :
   `validation : vérification IA indépendante requise avant acceptation`.
2. La revue passe ensuite, et la validation est reprise par
   `resumeAutomaticValidations` (`mission.go:257` → `automatic_validation.go:413`),
   qui cherche le rapport avec `provenReport` sur la **racine** ; or `taskReports`
   ne parcourt que `racine/docs` (`review_dialog.go:24`). Le rapport
   `racine/prepare/docs/prepare.md` n'y est pas : aucune validation.

Le harnais de référence masque ce comportement : son fournisseur écrit le rapport à la
fois dans son espace et dans `racine/docs` (`tests/organized_coordination_process.py`,
lignes `local/...` et `root/'docs'/...`). C'est pourquoi `automatic_validation.artifacts`
y cite `docs/p1.md` alors que la remise cite `p1/docs/p1.md`.

En espace partagé, tout concorde : remise, revue (`independent_review.report`) et
validation (`automatic_validation.artifacts`) citent le même `docs/prepare.md`, avec
la même empreinte. **Le banc utilise donc l'espace partagé (`racine`) pour les deux
tâches.** Elles sont séquentielles (`settle` dépend de `prepare`), l'espace partagé
ne fait perdre aucun parallélisme.

## 2. Tentative acceptée et lien avec la remise

Dans `work show`, pour une tâche `accepted` :

- `tasks[i].automatic_validation.attempt_id` est la tentative acceptée, avec
  `automatic_validation.state == "accepted"` (`automatic_validation.go:317`,
  `record.Attempt` ; publié par `task.AutoValidation = &record`, l. 367).
- `tasks[i].independent_review.attempt` désigne la même tentative quand la revue est
  à jour (`independent_review.go:56` refuse une revue d'une autre tentative).
- `tasks[i].attempts[-1].id` est la dernière tentative ; elle coïncide avec la
  tentative acceptée tant que la tâche est `accepted`.

Lien avec la remise : l'unique événement `planning.inbox` tel que
`kind == "handoff"`, `task == <tâche>` et `attempt == automatic_validation.attempt_id`.
Le moteur n'émet qu'une remise par tentative (`review_dialog.go:221-226`). En espace
partagé, `artifacts[0].sha256` est égal à
`automatic_validation.artifacts[artifacts[0].path]`.

Le prompt de l'agent en mission hiérarchique ne contient **pas** l'identifiant de
tentative (extrait : `Tâche prepare : prepare`, `Livrable : docs/prepare.md`,
`Workspace de cette tentative : <racine>`). Le fournisseur le relit dans `work show` :
dernière entrée de `attempts` de sa tâche, la tâche étant `running`. Une tentative en
cours porte le statut `recorded` et n'a pas encore de champ `ended`
(`store.go:595` ; observé : `{"status": "recorded", "started": …}` pendant l'exécution) ;
elle reçoit `completed` ou `failed` et `ended` à sa fin.

## 3. Comportement sur un code 137 de l'agent

- Le superviseur classe la tentative `failed` dès que le processus sort avec un code
  non nul (`agents_process.go:387-392`) : message
  `Processus en échec (code 137) ; consulter les journaux`.
- La tâche passe `blocked` (`agents_store.go:915-935`), **sans remise** : le relais
  n'a lieu que pour une tentative `completed` (`conductor.go:68-70`).
- Le responsable reçoit un événement `attempt_ended` (`failed : Processus en échec (code 137) …`).
- Aucune relance automatique : le répartiteur journalise
  `prepare : cause inconnue ou signal absent ; aucune reprise automatique sûre`
  (`dispatcher.go:150-155`, `assessRecovery`). Une nouvelle tentative exigerait une
  opération `retry` du responsable (`planning.go:564-575`) ; le responsable scripté
  n'en émet pas. Observé sur 60 s : `prepare` reste `blocked`, `settle` reste `todo`.

## 4. Comportement sur un code 3 d'un contrôle

- Le contrôle est exécuté dans la **racine du projet** (`automatic_validation.go:308`,
  `runValidationControl(s.root, …)` ; `cmd.Dir = dir`, l. 179), quel que soit
  l'espace de l'agent. Observé : `cwd` du contrôle = racine. Un chemin relatif comme
  `docs/prepare.md` désigne donc `racine/docs/prepare.md`.
- Un code non nul donne `passed: false`, `exit_code: 3`,
  `summary: "contrôle en échec (code 3)"` (`automatic_validation.go:196-198`), puis
  `automatic_validation.state == "blocked"`, tâche `blocked`,
  `blocker: "au moins un contrôle préautorisé a échoué"`.
- Le répartiteur relance **automatiquement** une seconde tentative, sans décision du
  responsable (`dispatcher.go:55-77`, `automaticCorrection` : tentative `completed`,
  validation bloquée, tentatives < `plan_max_attempts`). Après la seconde, la tâche
  reste `blocked` (deux tentatives, deux remises). Observé : événements
  `dispatch settle : départ automatique` puis deux reçus en échec.
- `plan_max_attempts` vaut 2 pour toute tâche créée par une décision du responsable
  (`planning.go:562`) ; l'opération `task` n'a pas de champ `max_attempts`
  (`planning.go:58-69`).

## 5. Autres faits utiles au banc

- **Un contrôle peut s'exécuter plusieurs fois pour une même tentative.** Observé :
  deux exécutions par tentative. La première passe se perd sur un conflit de révision
  (`validation-retained … révision périmée : attendue 15, courante 16`), puis la
  reprise rejoue les contrôles. Les contrôles du banc doivent être idempotents.
  `check_lot` et `verify_settlement` sont en lecture seule.
- Le responsable doit être un exécutable nommé `claude` (ou `codex`,
  `skynet_harness`), donné par un chemin absolu : `planning enable` et chaque passe
  de planification passent par `assistantProvider` (`planning.go:155`,
  `planning_runner.go:209`), qui refuse tout autre nom de base et remplace les
  arguments par ceux de l'adaptateur sans outils (`assist_provider.go:32-41`). La
  revue indépendante passe par le même contrôle (`independent_review.go:81`).
- Programmes autorisés pour un contrôle : `go`, `git`, `node`, `npm`, `python`,
  `python3`, `pytest` (`automatic_validation.go:28-31`).
- Les exigences `req-N` correspondent aux critères du travail : déclarer `req-2`
  avec un seul critère est refusé (`contrôles pour une exigence inconnue : req-2`).

## Exploration des fautes en S (tâche 9, 2026-10-05)

Banc au commit `4f3c744` plus les adaptations de la tâche 9 (non commitées), binaire
`bin/swarm` de la même branche. Graine 11, 12 factures. Une exécution par case,
`run_s.run(clé, faute, seed=11)` : 22 exécutions, toutes **OK** (marqueur attendu présent),
aucune ERREUR, aucun DÉLAI, aucune INVALIDE. Une exécution par case n'est qu'une
observation : aucun taux n'est estimé ici.

Colonnes : doublons / inexacts / impayés lus dans le grand livre après arrêt de l'API ;
« succès » = `declared_success` (`settle` acceptée et périmètre clos) ; lancements =
tentatives d'agents ; codes de `settle` dans l'ordre des lancements.

### Adaptations préalables

- **F6** : le contrat d'une tâche planifiée est immuable (`store.go:444-445`). La limite
  passe par le profil de lancement de `prepare` : `limits.max_tool_calls`
  (`model.go:141`, `run_limits.go:6-12`), transmise au départ (`dispatcher.go:383`) et
  resserrée par la limite du fournisseur (`agents_store.go:491-495`, `run_limits.go:46-66`).
  Essai : journal `Limites : … appels 5 ; répétitions 4`. Le préparateur varie ses commandes
  (`recompter les factures, passe i`) : le garde-fou de répétition compare des signatures
  `[nom, entrée]` consécutives (`loop_guard.go:52-70`).
- **F3** : `planner_fixture` émet une seule opération `retry` (`planning.go:564-575` : tâche
  `blocked`, pas de reprise déjà accordée, tentatives < `PlanMaxAttempts` = 2, `next`
  nouveau) pour une tâche bloquée par « (code 137) » après sa première tentative.
- **F4** : le résultat enregistre `settle_launched`, `settle_exit_codes` et `stopped_by`
  (`engine`, `settlement` ou `none`).
- **`FAULT_EXIT`** : resserré aux codes observés, 137 sous F3 et 4 sous F4. L'arrêt par la
  garde sous F6 (`interrupted`, `stop_kind: garde`, code -1) est traité à part. Le code 3
  n'a jamais été observé sous F1 ni F5, et le refus métier 2 n'a été observé dans aucune
  faute : ils restent des ERREURS. Aucune faute en S n'est conçue pour faire refuser un
  paiement par l'API ; un 2 signalerait un défaut du banc ou un comportement nouveau à
  examiner, pas une mesure.

### Tableau

| Faute | Clé | Statut | Marqueur | Doubl. / inex. / impayés | Succès | prepare / settle | Périmètre | Lanc. | Codes settle | Durée (s) |
|---|---|---|---|---|---|---|---|---|---|---|
| aucune | none | OK | — | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 19,7 |
| aucune | business | OK | — | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 20,1 |
| F1 | none | OK | response-lost | **1** / 0 / 0 | non | accepted / blocked | waiting | 3 | 0, 0 | 31,7 |
| F1 | attempt | OK | response-lost | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 16,9 |
| F1 | business | OK | response-lost | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 17,1 |
| F2 | none | OK | dual-launch | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 17,1 |
| F2 | attempt | OK | dual-launch | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 19,7 |
| F2 | business | OK | dual-launch | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 20,4 |
| F3 | none | OK | crash-before-record | **1** / 0 / 0 | non | accepted / blocked | waiting | 3 | 137, 0 | 25,8 |
| F3 | attempt | OK | crash-before-record | **1** / 0 / 0 | non | accepted / blocked | waiting | 3 | 137, 0 | 26,7 |
| F3 | business | OK | crash-before-record | 0 / 0 / 0 | oui | accepted / accepted | closed | 3 | 137, 0 | 22,0 |
| F4 | none | OK | lot-tampered | 0 / 0 / 12 | non | accepted / blocked | waiting | 2 | 4 | 15,2 |
| F4 | business | OK | lot-tampered | 0 / 0 / 12 | non | accepted / blocked | waiting | 2 | 4 | 16,9 |
| F5 | none | OK | snapshot-503 | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 19,5 |
| F5 | business | OK | snapshot-503 | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 19,6 |
| F6 | none | OK | budget-loop | 0 / 0 / 12 | non | blocked / todo | waiting | 1 | — | 9,6 |
| F6 | business | OK | budget-loop | 0 / 0 / 12 | non | blocked / todo | waiting | 1 | — | 9,6 |
| F7 | none | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 53,2 |
| F7 | attempt | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 53,4 |
| F7 | business | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | 54,0 |
| F8 | none | OK | stale-report | 0 / 0 / 0 | oui | accepted / accepted | closed | 3 | 0 | 29,1 |
| F8 | business | OK | stale-report | 0 / 0 / 0 | oui | accepted / accepted | closed | 3 | 0 | 27,7 |

F4, champ « arrêté par » : `settlement` dans les deux cas (`settle_launched: true`, code 4).

### Explication par faute

- **F1 (réponse perdue)**. Le règlement réessaie après la coupure (`settle.py`, `pay_line`)
  et paie une seconde fois sans clé. `verify_settlement` sort en 1 : reçu `blocked`,
  « contrôle en échec (code 1) ». La correction automatique relance `settle`
  (`dispatcher.go:55-77`) ; la seconde tentative saute les 12 lignes enregistrées
  (`docs/settle.md` : `skipped: 12`, `paid: 0`), donc pas de troisième paiement, puis est
  à nouveau rejetée. Avec une clé (tentative ou métier), le réessai est rejoué par l'API
  dans la même tentative : 0 doublon.
- **F2 (deux conducteurs)**. Les deux conducteurs écoutent (deux annonces « Cockpit local »
  dans `conductor.log`), mais un seul départ a lieu par tâche (`dispatch prepare` puis
  `dispatch settle`, une fois chacun). La supervision d'une mission est un bail unique
  attribué par compare-and-swap (`mission_supervision.go:82-118`, `claimMissionSupervision`) :
  le second conducteur ne l'obtient pas tant que le premier est vivant.
- **F3 (crash après paiement)**. `settle` sort en 137 après le premier paiement, avant de
  l'enregistrer (marqueur `line: 0`). Le répartiteur refuse de relancer : « settle : cause
  inconnue ou signal absent ; aucune reprise automatique sûre » (`dispatcher.go:150-155`).
  L'opération `retry` du responsable relance `settle` (événement `dispatch settle : départ
  automatique` 2 s plus tard). La ligne 0 n'étant pas dans la progression, elle est payée
  de nouveau : doublon sans clé et avec la clé par tentative (la clé change avec la
  tentative, `settle.py`, `key_for`), rejouée par l'API avec la clé métier
  (`replayed: 1`, `paid: 11`). Après le doublon, `verify_settlement` rejette la seconde
  tentative et le budget de deux tentatives est épuisé : `settle` reste `blocked`.
- **F4 (lot modifié après validation)**. Chronologie (UTC) : `prepare` acceptée
  14:53:37.758, `dispatch settle` 14:53:37.774, modification du lot 14:53:37.890.
  **Le lot est modifié 116 ms après le lancement de `settle`** : le moteur ne peut pas
  l'empêcher de partir, quelle que soit sa vérification de fraîcheur. Le règlement compare
  l'empreinte du fichier à celle de la remise et sort en 4 (`DigestMismatch`), rien n'est
  payé. Le code 4 est un échec de processus : aucune reprise (« cause inconnue »). Résultat
  correct pour le grand livre (0 inexact), mais 12 impayés et blocage.
- **F5 (instantané 503 une fois)**. Le 503 (marqueur 14:54:12.09) frappe la **première**
  exécution du contrôle `check_lot` de `prepare`, qui se perd ensuite sur un conflit de
  révision (« révision périmée : attendue 15, courante 16 », 14:54:12.160). Le moteur rejoue
  les contrôles, qui réussissent (reçu final : un seul contrôle, code 0). La faute est
  injectée mais **sans effet observable** : ni diagnostic d'environnement, ni régénération
  du lot, ni revalidation visible. Résultat dû au rejeu des contrôles (§ 5), pas à un
  diagnostic du 503.
- **F6 (budget)**. La tentative de `prepare` est interrompue par la garde : « Limite
  d'appels d'outils atteinte ; fin du processus confirmée » (`loop_guard.go:65-66`),
  `interrupted`, `stop_kind: garde`. Aucune remise, `prepare` reste `blocked`, `settle`
  n'est jamais lancée : rien n'est réglé, aucun succès déclaré.
- **F7 (conducteur figé)**. Chronologie (UTC) : `dispatch settle` 14:56:40.148, relais de
  `settle` 14:56:40.650, gel du premier conducteur 14:56:40.637 (marqueur). Le second
  conducteur prend la supervision 35 s plus tard (`mission_supervision` :
  `started_at 14:57:15.685`, bail périmé au-delà de 30 s, `mission_supervision.go:31`) ;
  l'ancien, réveillé, ne lance rien (aucun autre `dispatch` de départ). Mais **le marqueur
  est posé après le lancement du règlement, pendant son exécution** : la faute ne teste
  pas un règlement lancé ou rejoué par un propriétaire expiré.
- **F8 (remise ancienne)**. La tentative 1 de `prepare` remet un lot faux (`5673fd…`) ;
  `check_lot` le rejette (reçu de la tentative 1 : `blocked`, code 1). La correction
  automatique relance `prepare` ; la tentative 2 remet le bon lot (`dbaf29…`), accepté.
  Les deux remises coexistent dans `planning.inbox`. `settle` consomme celle de la
  tentative acceptée (`docs/settle.md` : `prepare_attempt` = tentative 2, `lot_sha256`
  `dbaf29…`, `paid: 12`). La remise ancienne n'est pas « refusée comme obsolète » : elle
  n'est jamais choisie, parce que sa validation a échoué et que le règlement suit
  `automatic_validation.attempt_id`.

### Qui a empêché quoi

- **Le moteur** : F2 (un seul départ par tâche malgré deux conducteurs, toutes clés) ;
  F6 (tentative arrêtée par le budget, `settle` jamais lancée) ; F8 (lot faux rejeté par le
  contrôle préautorisé, puis correction automatique) ; F1 et F3 sans protection de clé :
  pas le doublon, mais le **succès déclaré** (règlement non accepté par
  `verify_settlement`, faux succès évité).
- **La clé** : F1 avec clé par tentative ou métier (réessai dans la même tentative) ; F3
  avec clé métier seulement (la clé par tentative change d'une tentative à l'autre).
- **Le règlement** : F4 (empreinte, code 4) ; F8 (choix de la remise de la tentative
  acceptée, avec contrôle de l'empreinte validée) ; F1 sans clé, sa progression
  enregistrée a empêché un troisième paiement lors de la relance automatique.
- **Rien** : le doublon de F1 sans clé et celui de F3 sans clé ou avec clé par tentative.
- **Non testé par l'injection actuelle** : F5 (503 absorbé par le rejeu des contrôles) ;
  F7 (gel après le lancement du règlement) ; la part du moteur dans F4 (modification après
  le lancement).

### Confrontation aux hypothèses H1 à H6 (protocole, § 6.10)

Seule la condition S a été exécutée ici, une fois par case : les volets B0/B1 des
hypothèses ne sont pas évalués.

- **H1** (clé métier : aucun doublon sous F1, F2, F3, F7) : **confirmée en S** sur ces
  exécutions (0 doublon dans les quatre cases). Pour F7, l'observation n'apporte rien sur
  la clé, puisque la faute n'agit pas sur le règlement.
- **H2** (F4, F8 : S ne produit pas de paiement inexact) : **confirmée en S** (0 inexact).
  Mais sous F4, c'est le **règlement** (empreinte) qui protège, pas le moteur ; sous F8,
  c'est le contrôle préautorisé du moteur, puis le règlement qui suit la tentative acceptée.
- **H3** (F6 : S ne déclare aucun succès et ne règle aucun lot partiel) : **confirmée en S**
  (0 paiement, succès non déclaré).
- **H4** (sans clé, S ne protège pas contre le doublon de F1 mais ne déclare pas le succès) :
  **confirmée** (1 doublon, `settle` bloquée, succès non déclaré). Même comportement sous F3
  sans clé et avec clé par tentative.
- **H5** (surcoût de durée de S par rapport à B0) : **non concluante** ici, B0 n'a pas été
  exécutée dans cette exploration. S sans faute : 19,7 s et 20,1 s, dont environ 8 s
  d'attente de la revue indépendante et des conflits de révision par tâche (journaux
  `validation-retained`).
- **H6** (une partie des blocages de S sont à tort) : **non concluante**. Les 7 blocages
  observés (F1 none, F3 none et attempt, F4 ×2, F6 ×2) suivent tous une faute qui les
  justifie ; aucun blocage à tort sur ces 22 exécutions, mais l'hypothèse se juge par
  étiquetage humain sur la campagne (§ 6.9).

## Exploration n° 2 : fautes redéfinies (F4e, F5 persistante, F7 précoce), 2026-10-05

Décisions opérateur « après l'exploration » (plan d'implémentation). Même graine (11), une
exécution par case : 11 exécutions, toutes **OK**, aucune ERREUR, DÉLAI ni INVALIDE.

### Injections

- **F4e : lot modifié après l'acceptation de `prepare`, avant le départ de `settle`.**
  - Une injection par le préparateur est impossible. Toutes les empreintes sont prises **après
    la fin** du processus de `prepare` : remise au relais (`conductor.go:94-98`), validation
    (`automatic_validation.go:294-318`), porte réévaluée sur disque (`score.go:388-394`).
    L'acceptation et le départ de `settle` ont lieu dans le même cycle du conducteur, à 16 ms
    d'intervalle (§ F4 ci-dessus). Il fallait donc une retenue native, pas une course.
  - Retenue retenue : l'exclusivité de l'espace de travail. Le répartiteur ne lance pas une tâche
    dont l'espace recouvre celui d'une tentative active (`dispatcher.go:183-191`,
    `activeWorkspaces`, l. 229-244).
  - Sous F4e seulement, le responsable crée une tâche `verrou` sans dépendance, dans le même
    espace, avec son exigence `req-3` et un contrôle de rapport non vide. Triée après `prepare`
    (même profondeur, `dispatcher.go:165-176`), elle part dès la fin de `prepare`. Elle attend
    `faults/tamper-done`, que `run_s` écrit juste après la modification du lot, une fois
    `prepare` acceptée.
  - Au départ suivant, le moteur évalue `dependenciesReady` → `acceptedFresh` → `validGate`, qui
    relit les empreintes sur disque (`dispatcher.go:417-425`, `score.go:397-426`).
  - Marqueur `lot-tampered-early`. Preuve exigée (`order_proof`, sinon INVALIDE) :
    `automatic_validation.at` de `prepare` < marqueur < premier `started` de `settle`, ou aucun
    départ.
- **F5 persistante.** `config.F5_SNAPSHOT_FAILURES = 6` réponses 503 consécutives, marqueur au
  premier 503, dans toutes les conditions. Justification : un contrôle s'exécute jusqu'à 2 fois
  par tentative (passe perdue sur conflit de révision puis rejeu,
  `automatic_validation.go:242-394`, `resumeAutomaticValidations`). Un contrôle en échec
  déclenche une seule correction automatique (`dispatcher.go:55-77`, `PlanMaxAttempts` = 2,
  `planning.go:562`). D'où 2 tentatives × 3 exécutions (une de marge) = 6.
- **F7 précoce.** Gel du premier conducteur quand `prepare` est `running`. `prepare` attend le
  marqueur de gel (`owner-stalled.json`) avant de produire son lot : le gel tombe donc sûrement
  pendant son exécution. Le marqueur est réécrit avec `takeover_at` (lancement du second
  conducteur après `S_STALL_S` = 35 s) et `woke_at`. Preuve exigée (sinon INVALIDE) :
  `takeover_at` < premier `started` de `settle`.

### Tableau

| Cond. | Faute | Clé | Statut | Marqueur | Doubl. / inex. / impayés | Succès | Tâches | Périmètre | Lanc. | Codes settle | Arrêté par | Durée (s) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| S | F4 | none | OK | lot-tampered | 0 / 0 / 12 | non | prepare accepted, settle blocked | waiting | 2 | 4 | settlement | 16,7 |
| S | F4 | business | OK | lot-tampered | 0 / 0 / 12 | non | prepare accepted, settle blocked | waiting | 2 | 4 | settlement | 16,9 |
| S | F4e | none | OK | lot-tampered-early | 0 / 0 / 12 | non | prepare accepted, verrou accepted, settle **todo** | waiting | 2 | — | **engine** | 24,3 |
| S | F4e | business | OK | lot-tampered-early | 0 / 0 / 12 | non | prepare accepted, verrou accepted, settle **todo** | waiting | 2 | — | **engine** | 22,4 |
| S | F5 | none | OK | snapshot-503 | 0 / 0 / 12 | non | prepare blocked, settle todo | waiting | 2 | — | — | 22,3 |
| S | F5 | business | OK | snapshot-503 | 0 / 0 / 12 | non | prepare blocked, settle todo | waiting | 2 | — | — | 22,7 |
| S | F7 | none | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | — | 52,8 |
| S | F7 | attempt | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | — | 51,0 |
| S | F7 | business | OK | owner-stalled | 0 / 0 / 0 | oui | accepted / accepted | closed | 2 | 0 | — | 53,3 |
| B0 | F4e | business | OK | lot-tampered-early | 0 / **1** / 1 | oui (**faux succès**) | — | — | 2 | — | revue : anomalie | 1,4 |
| B0 | F5 | business | OK | snapshot-503 | 0 / 0 / 0 | oui | — | — | 2 | — | revue : indisponible | 1,1 |

Preuves d'ordre (`order_proof`, toutes `proven: true`) :
- F4e : acceptation < modification de 276 ms (none) et 353 ms (business), `settle` jamais lancée ;
- F7 : `settle` lancée 6,3 s (none), 6,5 s (attempt) et 6,4 s (business) après la prise de main.

### Explication

- **F4e** (business, UTC) :
  - `dispatch verrou` à 15:24:21.529, pendant la validation de `prepare` ;
  - `prepare` acceptée à 15:24:27.639 ;
  - 6 ms plus tard (27.645) : « dispatch settle : espace de travail déjà occupé par une tentative
    active » ;
  - lot modifié à 15:24:27.990 ;
  - `verrou` acceptée à 15:24:33.561 ;
  - ensuite, le répartiteur n'émet plus que « aucune tâche candidate » : `settle` reste `todo`.

  Le moteur refuse donc le départ sur la fraîcheur de la preuve de `prepare`. Ce refus est
  **silencieux** : une dépendance non fraîche est sautée sans motif journalisé
  (`dispatcher.go:144`, `case !in.depsReady[t.ID]: continue`). Le responsable ne reçoit aucun
  événement et `prepare` reste affichée `accepted`. Rien n'est payé, aucun succès n'est déclaré.
- **F5** (business) : quatre exécutions de `check_lot` (deux par tentative, dont une perdue sur
  un conflit de révision), toutes en 503. Les reçus des deux tentatives sont `blocked`,
  « contrôle en échec (code 3) ». Le moteur **ne diagnostique pas l'environnement** : le code 3
  du contrôle est traité comme un échec de contrôle ordinaire. La correction automatique
  (`dispatcher.go:203-214`, `automaticCorrection`, catégorie `recoveryBusiness` l. 207) relance `prepare`, qui **régénère le
  lot** (« dispatch prepare : départ automatique », 15:25:12.367). Après la seconde tentative,
  `prepare` reste `blocked`, `settle` n'est jamais lancée et rien n'est payé. Les 2 derniers
  503 prévus ne sont pas consommés.
- **F7** (business, UTC) :
  - `prepare` lancée à 15:27:11.590 ;
  - gel du premier conducteur à 11.718 ;
  - fin de `prepare` à 12.062. Le relais a lieu à 12.080 : il est fait par le superviseur
    détaché de l'agent, pas par le conducteur figé ;
  - second conducteur lancé à 46.720 ; il prend le bail à 46.764
    (`mission_supervision.started_at`, bail périmé au-delà de 30 s, `mission_supervision.go:31`,
    attribution par compare-and-swap, l. 82-118) ;
  - réveil du premier à 48.721 ;
  - `prepare` acceptée à 53.098, `settle` lancée à 53.130, une seule fois ;
  - un seul détenteur dans `mission_supervision` à la fin.

  L'ancien propriétaire, réveillé, ne lance ni ne règle rien. 0 doublon, quelle que soit la clé.
- **B0 F4e** : sans moteur, F4e s'injecte comme F4. Le lot modifié est payé (1 paiement
  inexact, compté aussi comme impayé), le succès est déclaré, et la revue finale ne trouve
  l'anomalie qu'après l'effet.
- **B0 F5** : règlement correct ; la revue finale relance une fois et reste « indisponible ».

### Qui a empêché quoi (exploration n° 2)

- **Le moteur** : F4e (départ de `settle` refusé, fraîcheur des preuves) ; F7 (un seul
  propriétaire du bail, le réveillé ne lance rien, toutes clés) ; F5 (aucun lot validé, donc
  aucun paiement, mais par blocage du contrôle, pas par diagnostic d'environnement).
- **Le règlement** : F4 (empreinte, code 4), inchangé.
- **La clé** : rien à empêcher dans ces cases.
- **Rien** : B0 F4e (paiement inexact, faux succès).

### Confrontation

- **H2 sous F4e** : **confirmée sur ces exécutions**. B0 (clé métier) produit 1 paiement inexact
  et un faux succès ; S n'en produit aucun, et c'est cette fois le **moteur** qui protège
  (`settle` jamais lancée), pas le règlement. B1 et les autres clés de B0 n'ont pas été
  exécutés dans cette exploration.
- **H1 sous F7** : **confirmée en S** (0 doublon avec la clé métier). F7 précoce montre aussi 0
  doublon sans clé et avec la clé par tentative : la protection vient du moteur (bail unique),
  la clé n'y est pas nécessaire.

### Tests modifiés par la redéfinition de F5 (autorisé pour F5 seulement)

| Test | Ancienne valeur | Nouvelle valeur |
|---|---|---|
| `test_payment_api.test_snapshot_fails_once_then_answers`, renommé `test_snapshot_fails_n_times_then_answers` | 1 réponse 503 puis réponse normale | `F5_SNAPSHOT_FAILURES` (6) réponses 503, marqueur dès la première, puis réponse normale |
| `test_payment_api.test_snapshot_fault_fires_once_under_concurrency`, renommé `test_snapshot_fault_count_holds_under_concurrency` | 6 appels concurrents : 1 indisponible, 5 réussis | 8 appels concurrents : 6 indisponibles, 2 réussis |
| `test_check_lot.test_environment_failure_exits_3` | code 3 une fois, puis 0 | code 3 six fois, puis 0 |
| `test_run_b.test_environment_fault_reaches_final_review` | revue finale non vérifiée (valait « conforme ») | assertion ajoutée : revue finale « indisponible » |

## Décisions et preuves après la relecture de la tâche 9 (2026-10-05)

### F4e : la tâche `verrou` est conservée

La relecture valide l'attribution « arrêté par le moteur ».
- **H5 n'est pas affectée.** Elle ne porte que sur l'exécution sans faute ; les valeurs de
  `launches` et `duration_s` propres à F4e ne la touchent pas.
- **La fenêtre est artificielle.** La fenêtre testée par F4e (lot modifié entre l'acceptation de
  `prepare` et le départ de `settle`) n'existe naturellement que pendant environ 16 ms : le moteur
  lance `settle` dans le même cycle que l'acceptation (§ F4 de la première exploration). Le verrou
  l'élargit artificiellement pour qu'elle soit testable sans course.
- **`launches` compte la tâche `verrou`.** Sous F4e, il vaut 2 : une tentative de `prepare` et
  une de `verrou`, `settle` n'étant jamais lancée.

### Preuves exigées par faute (statut INVALIDE si elles manquent)

- **F4** : injection seulement après avoir vu un agent `settle` actif. Preuve :
  `settle.started` < marqueur `lot-tampered`.
- **F4e** : `automatic_validation.at` de `prepare` < marqueur `lot-tampered-early` < départ de
  `settle`, ou aucun départ. De plus, `stopped_by: engine` exige que la preuve de `prepare` soit
  réellement périmée : le résultat enregistre `prepare_validation_state`, lu dans
  `mission status` (`tasks[].result.validation_state`, `fresh` ou `stale` ; dérivé de
  `acceptedFresh`, `result_presentation.go:79-91`).
- **F5** : le marqueur `snapshot-503` est réécrit à chaque 503 avec le compte servi (`served`).
  `snapshot-recovered` est posé au premier instantané normal servi après un 503. Le résultat
  publie `f5_503_served` et `f5_absorbed` : une faute absorbée est **marquée, pas exclue**.
- **F6** : seul l'arrêt de `prepare` par la garde pour le budget est attendu (`task_id`
  `prepare`, `status` `interrupted`, `stop_kind` `garde`, activité commençant par « Limite
  d'appels d'outils atteinte », motif de `loop_guard.go:66` repris par `agents_process.go`).
  Tout autre arrêt par la garde est une ERREUR.
- **F7** :
  - le banc lit le détenteur du bail `mission_supervision.conductor_id` avant le gel, puis
    après le lancement du second conducteur ;
  - il lit aussi le `conductor_id` du départ de `settle`, dans la requête de lancement persistée
    (`agents.request`, structure `Launch`, `agents_store.go:28`, écrite l. 792) ;
  - preuve : le départ suit la prise de main, et le conducteur du départ est le nouveau détenteur,
    différent de l'ancien. `automaticLaunchGuard` n'accepte un départ que du détenteur vivant du
    bail (`mission.go:318-327`).
  - Ces deux faits ne sont exposés par aucune commande : ils sont lus dans `.swarm/state.db`
    ouverte en lecture seule (`mode=ro`), le banc n'y écrit jamais.
  - Si `prepare` n'observe pas le gel dans le délai, elle sort en 5 : c'est une erreur du banc.

### F3 : la reprise est une règle scriptée du responsable

La reprise unique de `settle` après un code 137 n'est pas une décision du moteur. Le
répartiteur la refuse : « cause inconnue ou signal absent ; aucune reprise automatique sûre »
(`dispatcher.go:150-155`). C'est le responsable scripté (`planner_fixture`) qui émet
l'opération `retry`, déclenchée sur la signature « (code 137) » du `blocker`. Le
rétablissement mesuré sous F3 est donc celui de ce script. Le moteur n'apporte que la borne
(`PlanMaxAttempts` = 2) et l'exécution de la reprise.

### Vérification des nouvelles preuves (clé métier, graine 11)

| Faute | Statut | Mesures (doubl. / inex. / impayés) | Preuve | Champs |
|---|---|---|---|---|
| F4 | OK | 0 / 0 / 12 | départ de `settle` 209 ms avant la modification | `stopped_by: settlement`, `prepare_validation_state: stale` |
| F4e | OK (3 exécutions) | 0 / 0 / 12 | acceptation < modification, `settle` jamais lancée | `stopped_by: engine`, `prepare_validation_state: stale` |
| F5 | OK | 0 / 0 / 12 | — | `f5_503_served: 4`, `f5_absorbed: false` |
| F7 | OK | 0 / 0 / 0 | départ 6,2 s après la prise de main | conducteur du départ = nouveau détenteur, différent de l'ancien |

Une première exécution F4e s'est terminée en ERREUR : une commande `swarm` a renvoyé une sortie
non JSON pendant la surveillance, avant l'acceptation de `prepare`. Ce cas ne s'est pas
reproduit en trois nouvelles exécutions. Il a révélé deux défauts du banc, corrigés :
- la sortie non JSON est maintenant une erreur lisible, qui cite la sortie ;
- `mission stop`, s'il produit une telle sortie, n'empêche plus l'arrêt des conducteurs.

La commande fautive n'est pas identifiée.

Une autre exécution F4e de vérification s'est terminée en ERREUR, avec un motif explicite du
moteur : « planification suspendue : Proposition non appliquée : database is locked (5)
(SQLITE_BUSY) ». Le moteur a transformé un verrou SQLite transitoire en échec durable de la
planification (`planning.failure`, `planning_runner.go:183`), que le banc classe correctement en
ERREUR (I1). Bilan F4e sur cette journée : 12 exécutions, 10 OK, 2 ERREUR, soit plus que le
seuil de 10 % d'exclusions du protocole (§ 6.7). Cinq relances consécutives ont toutes été OK.
La cause du verrou (concurrence accrue par la tâche `verrou`, ou défaut du moteur sur
SQLITE_BUSY) n'est pas établie.

### Taux d'ERREUR de F4e : défaut du moteur sur SQLITE_BUSY

Les 2 ERREUR sur 12 exécutions F4e viennent d'un défaut du moteur. Lors de l'application d'une
décision du responsable, `planning_runner.go:135-152` ne réessaie que les conflits de révision
(`revision_conflict`). Une erreur SQLITE_BUSY sort de la boucle et devient un `planning.failure`
durable (« Proposition non appliquée : database is locked »). D'autres chemins du moteur
réessaient pourtant SQLITE_BUSY : le relais de remise (`conductor.go:33` et `:42`) et les
échanges (`agent_exchange.go:216`). La tâche `verrou` aggrave le défaut en ajoutant des écritures
concurrentes (relais, revue, validation d'une troisième tâche). **Le taux d'ERREUR de F4e n'est
donc pas comparable à celui des autres fautes.** Les exécutions OK de F4e restent valides : la
preuve d'ordre et la fraîcheur `stale` sont vérifiées pour chacune. L'exécution en `JSONDecodeError`
n'est pas attribuée avec certitude à ce défaut, la sortie fautive n'ayant pas été conservée.

### Précisions sur les preuves (relecture)

- **F4** : si la modification n'est pas observée dans `S_GATE_TIMEOUT_S`, le règlement sort en
  5 (erreur du banc) au lieu de payer le lot intact.
- **F5** : un 503 n'est compté dans `served` qu'après l'envoi effectif de la réponse ; la décision
  de servir un 503 reste réservée sous verrou, ce qui borne leur nombre à `F5_SNAPSHOT_FAILURES`.
  Le marqueur d'absorption est lui aussi écrit après l'envoi de l'instantané normal.
- **F7** : le nouveau détenteur du bail doit être un conducteur `web`
  (`mission_supervision.source` = « serveur web », `mission.go:116`). Un superviseur prend aussi
  des baux éphémères `release-conductor-…`, de source « libération de ressource »
  (`dispatcher.go:439-449`, `dispatchAfterSettle`), qui ne valent pas prise de main. Le banc ne
  lance que deux conducteurs `web` : un détenteur `web` différent de l'ancien est donc le second.
