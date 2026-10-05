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
