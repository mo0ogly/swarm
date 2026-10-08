# T0 — Inventaire ciblé (REQ-QW1 à REQ-QW4)

Date d'inspection : 2026-10-03. Racine inspectée : `/home/fpizzi/workspace/swarm-action-skills`
(worktree, `git rev-parse --show-toplevel` = racine courante). Lecture seule : aucune
modification de code, aucune commande d'écriture exécutée. Source principale :
`docs/RETEX-ADMIN-PREPARATION.md` (1292 lignes, lu intégralement) + lecture directe du
code Go et JS cité ci-dessous avec chemin:ligne.

## Hypothèse du brief — verdict

> « Des écrans partiels de pré-lancement, blocage ou bilan existent déjà dans le
> moteur/CLI/cockpit ; à confirmer par l'inventaire T0. »

**CONFIRMÉE pour QW1, QW2, QW3 et QW4**, et plus largement que « partiel » : pour
chacune des quatre exigences, un triplet moteur + CLI + web existe déjà et est câblé
de bout en bout (struct Go → commande CLI → rendu web), avec preuve sourcée ci-dessous.
Le RETEX documente leur livraison le 1er octobre 2026 (« sept améliorations de conduite
livrées », `docs/RETEX-ADMIN-PREPARATION.md:1055-1102`), sur la mission
`w-843bb3ce22cff2965c5e77b6`, dépôt/branche différents de ceux cités en tête du rapport T2
archivé (`docs/RETEX-ADMIN-PREPARATION.md:242`, `/home/fpizzi/workspace/swarm-engine-contract/source`),
mais le code correspondant est bien présent dans **cette** racine (`swarm-action-skills`),
confirmé par lecture directe des fichiers ci-dessous. Consé­quence pour les lots suivants :
le travail restant par exigence est probablement un **complément ciblé**, pas une
construction depuis zéro — à faire valider par le planificateur avant tout découpage de
T1-T4, conformément à la procédure prévue en cas d'écart de périmètre.

## REQ-QW1 — Résumé avant lancement

**Existant (moteur) :**
- `model.go:162-174` — `LaunchProfile{Skills, Limits, Provider, Role, Workspace, Instruction, Level, Timeout, Capture, Updated, Actor}`.
- `mission_status.go:47-67` — `MissionLaunchPreview{Organization, Token, Revision, RequestedSlots, Immediate, EffectiveConcurrency, SharedWorkspace, ConcurrencyMode, ConcurrencyDetail, Departures, Waiting, Limits, Contract}` et `MissionLaunchContract{Scope, Budget, Recovery, Validation}`.
- `mission_status.go:102-128` — `missionLaunchContract(...)` construit les textes Scope/Budget/Recovery/Validation (périmètre, plafond financier, règle de reprise, mode de validation).
- `mission_status.go:133-140+` — `(s *Store) missionLaunchPreview(work, profile, slots)` : lecture seule, réévaluée avant toute confirmation (commentaire ligne 130-132).

**Existant (CLI) :** `mission_cli.go:74-122` — `mission preview WORK` affiche concurrence
réelle, portée/budget/reprises/validations du contrat, départs, attentes, limites.
`mission_cli.go:89-93` — réutilise `w.Profile` existant si pas de `--input`.

**Existant (web) :** `web/mission.js` — seul fichier du dépôt référençant
`departures`/`requested_slots`/`contract.scope`/`concurrency_detail` (confirmé par
`rg -ln` sur `web/*.js`) : rendu du même aperçu de lancement que la CLI.

**À créer / lacune identifiée :** le brief demande explicitement un « modèle
demandé/réel/inconnu ». `LaunchProfile.Provider` (`model.go:165`) est une chaîne unique
— aucun champ distinct « modèle demandé » vs « modèle réellement résolu » n'a été trouvé
dans `MissionLaunchPreview`/`MissionLaunchContract`. Le RETEX documente un cas réel où
l'alias demandé ne correspond pas forcément au modèle exécuté
(`docs/RETEX-ADMIN-PREPARATION.md:908-911`, alias Sonnet 5 vs Sonnet 5.5/Opus 5.5). Cette
distinction reste à vérifier dans le code de résolution de provider (non lu dans cette
passe, budget de lecture atteint) avant de conclure qu'elle est absente ou seulement non
affichée.

## REQ-QW2 — Blocage expliqué (cause / acteur / action autorisée)

**Existant (moteur) :**
- `agents_store.go:84-105` — `AttemptDiagnostic{AgentID, AttemptID, ObservedErrors, ConsecutiveErrorLimit, LimitReached, ConsecutiveLimitReached, LimitReason, Summary, Items}` et `DiagnosticItem{Category, Label, Count, Cause, Consequence, Action, ActionKind, Unknown, Traces}`.
- `attempt_diagnostic.go:72` — `buildAttemptDiagnostic(agent, attempt, failures, reason, limit)`.
- `attempt_diagnostic.go:131,145` — `fallbackAttemptDiagnostic(a Agent)`.
- `mission_status.go:11-32` — `MissionTask.Diagnostic *AttemptDiagnostic` (optionnel).
- `mission_status.go:33-39` — `MissionUnderstanding{What, NextStep, Actor, ActorKind, Situation}` porte le « qui agit » au niveau tâche, à côté du diagnostic.

**Existant (CLI) :** `mission_cli.go:249-251` et `mission_cli.go:282-296` —
`printAttemptDiagnostic` affiche Cause / Conséquence / Action disponible / Traces
techniques par item.

**Existant (web) :** `web/mission.js:26-65` — `diagnosticView`, `diagnosticText`,
`copyDiagnosticButton` (bouton « Copier le diagnostic »).

**À créer / lacune identifiée :** `DiagnosticItem` n'a pas de champ « acteur
responsable » propre à l'item (seulement `ActionKind` + texte libre `Action`). L'acteur
vient du niveau tâche (`MissionUnderstanding.Actor/ActorKind`), pas du diagnostic
lui-même. Reste à vérifier si cette séparation suffit pour tous les types de blocage
cités au RETEX (quota fournisseur, `review-timeout`, validation humaine manquante,
`ProviderCooldown` — `mission_status.go:82`, non comparé ici au diagnostic) ou si un
blocage spécifique (ex. cooldown fournisseur) emprunte un chemin de présentation distinct
non encore croisé avec `AttemptDiagnostic`.

## REQ-QW3 — Bilan par tentative (lectures/écritures/tests/erreurs/répétitions/appels/coûts)

**Existant (moteur) :**
- `mission_insights.go:44-55` — `SpendingRow{Kind, ID, Label, Calls, Tools, UnknownTools, Input, Output, MissingUsage, Cost}`.
- `mission_insights.go:56-61` — `MissionSpending{Rows, Controls, Retries, Note}` — note engine explicite : « Les appels d'outils, les appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n'est pas un zéro… » (`mission_insights.go:254`).
- `mission_insights.go:253-332` — `(s *Store) missionSpending(w, agents)` construit les lignes :
  - une ligne `kind="worker", id=TaskID` par tâche (agrégée sur tous les agents de la tâche, `Calls++` par agent → **par tâche, pas par tentative individuelle**) ;
  - une ligne `kind="planner"` ou `kind="reviewer", id=scope` par portée de planification/revue, lue depuis `planning_calls` (`WHERE state!='released'`) ;
  - `out.Retries` compte globalement les agents ayant un `Previous` non vide (reprise), sans ligne dédiée ;
  - `out.Controls` compte globalement les contrôles `Executed` depuis les events `task.auto-validation`, **agrégé pour toute la mission, pas par tâche**.

  **Réponse à la décision ouverte du plan** (granularité actuelle des données
  coûts/appels) : ni purement « par tentative », ni purement « agrégée » — c'est un mix
  : **par tâche** pour le travail des exécutants, **par portée** (planner/reviewer) pour
  la supervision, et **agrégée pour toute la mission** pour les contrôles automatiques et
  les reprises. Aucune ligne n'isole une tentative précise parmi plusieurs tentatives
  d'une même tâche dans cette structure.

**Existant (CLI) :** `mission_cli.go:55-71` — `mission spending WORK`.

**Existant (web) :** `web/mission-insights.js:17-22` — `spendingView`/`spending()`
(« Où vont les appels et les coûts ? »).

**À créer / lacune identifiée :** aucun champ ne distingue lectures/écritures/tests par
type d'outil (Bash/Read/Write/Edit) comme le fait le RETEX à la main
(`docs/RETEX-ADMIN-PREPARATION.md:624-629` : décompte manuel 32 Bash/26 Read/1 Agent…).
`SpendingRow` donne seulement un total `Tools` (appels d'outils) sans sous-catégorie. Si
REQ-QW3 exige ce détail par catégorie d'outil, c'est à construire. Pas de champ « erreurs »
ni « répétitions » distinct non plus dans `SpendingRow` — à confirmer en croisant avec
`AttemptDiagnostic.ObservedErrors`/`ConsecutiveErrorLimit` (REQ-QW2), qui pourrait déjà
couvrir une partie de ce besoin sans qu'il soit relié à `SpendingRow` aujourd'hui (lien non
vérifié dans cette passe).

## REQ-QW4 — Reprise ciblée

**Existant (moteur) :**
- `mission_insights.go:18-26` — `RecoveryPreview{Work, Task, Revision, Agent, Kept, Redone, Correction, Criteria, Limits}`.
- `mission_insights.go:85+` — `recoveryPreviewFor(w, t, a)` remplit `Kept` (historique des tentatives/rapports/avis, critères et dépendances du plan), `Redone` (vérification des conditions de lancement et du candidat, contrôles/revue requis), `Correction` (= `t.Next`), `Criteria` (copie de `t.Criteria`), `Limits` (texte explicite : « Aucun budget ni plafond n'est augmenté. Ce résumé n'autorise aucun départ et ne valide aucun résultat. »).
- `mission_insights.go:105` — `(s *Store) recoveryPreview(work, task, agent)`.
- `mission_status.go:11-13` — `MissionTask.RecoveryPreview RecoveryPreview` (toujours présent, pas de pointeur).
- Séparément : `mission_insights.go:27-43` — `MissionChange`/`MissionChanges{From, To, First, More, Items}`, calculées depuis le dernier marquage de visite (`markVisit`, exposé par `mission_cli.go:19-36`, action `seen`).

**Existant (CLI) :**
- `mission_cli.go:37-54` — `mission recovery WORK TASK [AGENT]` → `printRecoveryPreview`.
- `mission_cli.go:55-71` — `mission changes WORK` / `mission spending WORK`.
- `mission_cli.go:19-36` — `mission seen WORK` (marque le repère de visite).

**Existant (web) :**
- `web/mission-insights.js:23-26` — `recoveryPreviewView`/`recoveryPreview(id)` (section
  « Avant une relance », « Ce qui sera conservé » / « Ce qui sera refait » / « Critères
  inchangés » / « Correction attendue »).
- `web/mission-insights.js:33-44` — modale de relance, appel `GET /api/v1/recovery-preview?work=...&task=...&agent=...`.
- `web/mission-insights.js:5-16` — `changes()` (« Depuis votre dernière visite »).

**À créer / lacune identifiée :** le brief demande « changements depuis le **refus** ».
`MissionChanges` est scopé « depuis la dernière visite d'un opérateur » (`markVisit`), pas
littéralement « depuis le refus/blocage » d'une tâche précise. Reste à vérifier si un
refus (revue `changes_requested`, blocage moteur) marque automatiquement une frontière de
visite exploitable pour cette vue, ou si REQ-QW4 a besoin d'un filtrage distinct ancré sur
l'événement de refus plutôt que sur la dernière consultation humaine — non tranché dans
cette passe.

## Inconnues restantes (datées 2026-10-03, non inventées)

1. Distinction « modèle demandé / réel / inconnu » dans le chemin de lancement (REQ-QW1) :
   présence/absence non confirmée — nécessite lecture du code de résolution provider
   (hors budget de cette tâche).
2. Lien entre `DiagnosticItem` (REQ-QW2) et les blocages de type cooldown fournisseur
   (`ProviderCooldown`, `mission_status.go:82`) : non croisé.
3. Détail par type d'outil (Bash/Read/Write/Edit/tests) dans le bilan (REQ-QW3) : absent
   des structs lues ; à confirmer qu'aucun autre fichier ne le porte déjà avant de le
   qualifier de lacune définitive.
4. Lien entre `AttemptDiagnostic.ObservedErrors`/erreurs consécutives et le bilan
   `SpendingRow` (REQ-QW3) : non vérifié.
5. Portée exacte de « changements depuis le refus » (REQ-QW4) vs « changements depuis la
   dernière visite » existant : à clarifier avec le planificateur avant de décider si
   `MissionChanges` est réutilisable tel quel ou doit être adapté.
6. Équivalents côté cockpit (rendu visuel réel, pas seulement le JS source) non ouverts
   dans un navigateur dans cette tâche — lecture seule du code source uniquement.

## Limites de cette inspection

Lecture seule stricte, aucune commande de build/test exécutée, aucun fichier modifié hors
le présent livrable. Budget d'exploration : 18 appels d'outils consommés sur 20 alloués.
Les fichiers `tools/verification/*.py`, `web/planning.js`, `web/admin.js` et le code de
résolution de provider cités indirectement par le RETEX n'ont pas été ouverts dans cette
passe — toute affirmation à leur sujet dans ce document reste une inconnue explicite, pas
une conclusion.
