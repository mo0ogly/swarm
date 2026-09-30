# RETEX — préparation Administration depuis le web

Observations du 29 septembre 2026, mission `w-843bb3ce22cff2965c5e77b6`.
Pilotage réalisé depuis les écrans, sans appel direct aux API.

## Faits observés

- Une autorisation de l’équipe en préparation ne suffit pas au départ : un second lancement dans le cockpit est nécessaire. Le texte promettant une autorisation unique est donc trompeur dans ce parcours.
- Le mode de validation automatique demande une commande pour chaque critère, y compris les critères documentaires. La mission a été lancée en revue humaine ; aucun contrôle factice n’a été ajouté.
- T1 a consommé deux tentatives. Le rapport de la seconde déclare les exigences ADM-05/06/07 et les quick wins non définis dans le contexte reçu, alors que leurs définitions figurent dans le brief adopté visible dans la préparation.
- La vérification indépendante s’est interrompue sur une citation exacte introuvable (critère 2). Cela ne constitue pas une validation.
- Le responsable a dépassé son délai sans réponse finale ; le cockpit signale 72 événements système, dont 71 événements de réflexion, et zéro message assistant.

## Correction appliquée depuis les écrans

Le plan a été révisé : définitions explicites ajoutées dans le périmètre T1/T2,
complément du rapport existant demandé plutôt qu’un nouvel inventaire. Révision
vérifiée et appliquée ; historique et limites conservés. Démarrage réautorisé,
mission reprise, motif de correction transmis au responsable.

## Corrections produit à instruire

1. Transmettre aux workers les définitions des exigences qui leur sont assignées,
   avec la révision du brief ; tester le parcours préparation → consigne réelle.
2. Afficher honnêtement les deux étapes autorisation/lancement, ou les réunir.
3. Distinguer clairement contrôles exécutables et revue documentaire.
4. Ne pas afficher « avis enregistré » comme si une revue interrompue était un avis exploitable.
5. Limiter le contexte des décisions de reprise au rapport, aux lacunes et aux
   exigences concernées ; un délai dépassé n’autorise pas une hausse silencieuse.

## Limites

Aucune tâche acceptée à la rédaction de cette note. La révision des consignes
ne prouve ni leur réception par une prochaine tentative, ni la correction du
moteur, ni la réussite de la mission. La suite reste à vérifier dans le cockpit.

## Reprise autorisée et cause source confirmée

L’opérateur a explicitement autorisé une troisième tentative pour T1, toujours
limitée à 20 appels. Le plafond a été changé de 2 à 3 via la préparation, puis
la révision vérifiée, appliquée et réautorisée. Le cockpit confirme une tentative
active ; aucune acceptation n’est déduite de ce démarrage.

Lecture du code : `agents_store.go`, construction du prompt vers les lignes
778–816. En mode hiérarchique, le prompt remplace le contexte de travail par
`scope.Objective`, puis le brief commun n’est ajouté que si `w.Planning == nil`.
Ce choix explique la perte du brief dans ce parcours. La correction durable doit
transmettre les définitions pertinentes aux workers avec leur provenance, sans
exposer les contextes privés des autres périmètres ni saturer les prompts.
La copie locale des définitions dans T1/T2 contourne le manque pour cette mission ;
elle ne constitue pas une correction du moteur ni une preuve de résultat.

## Résultat de la troisième tentative

La session affichée dans le cockpit s’est terminée après 2 appels d’outils
(2 réponses ; coût transmis 0,25 USD). Son message explique « already done —
attempt 2/2 » et cite l’identité de l’ancien agent, alors qu’il s’agit du troisième
départ autorisé. Elle considère les lacunes documentées comme un travail terminé,
sans compléter le rapport. Le conducteur refuse le relais : aucun rapport produit
par cette tentative, un rapport antérieur ignoré. Aucun critère validé.

Conclusion : augmenter encore le nombre d’appels ne traite pas cet échec. Il faut
vérifier la construction de la consigne de reprise, la fraîcheur du profil réutilisé
et l’éventuelle réutilisation du contexte fournisseur. L’écran prouve la confusion
avec la tentative précédente ; il ne permet pas à lui seul d’attribuer cette
confusion à un identifiant de session réutilisé. Pas de quatrième essai autorisé.

## Quatrième tentative : livraison observée

Après autorisation explicite de l’utilisateur, reprise enregistrée par le nouveau
bouton « Préparer un essai correctif », puis mission reprise depuis le cockpit.
Tentative `a-d2c72df363d076d4e5134554`, agent `auto-86f3303ff0451e280da8` :
14 appels sur 20, 14 réponses, coût producteur transmis 0,80 USD. Les deux rapports
ont été réécrits avec la bonne identité et les définitions ADM-05/06/07 et les cinq
quick wins. Le conducteur a reconnu la livraison et soumis T1. C’est une avancée
réelle par rapport au refus précédent, pas une acceptation de la tâche.

La revue indépendante a ensuite expiré à 90 secondes sans message final (75
événements système). Le moteur disposait déjà d’un réglage `review-timeout`, absent
du cockpit. Ajout du bouton « Délai de vérification », avec motif, contrôle 1–900
secondes, maintien des appels consommés/plafond et aucun redémarrage implicite.
Recette navigateur isolée FR/EN et clair/sombre : enregistrement à 300 secondes,
refus de 901, conservation des tâches, appels et historique ; PASS. Délai de la
mission réglé à 300 secondes puis reprise explicite de la revue depuis l’UX,
sans cinquième tentative de production. Verdict encore attendu à ce point.

Autre limite observée dans le parcours humain : « Vérifier les preuves » demande
un fichier `.evidence.json`, sans proposer de construction de l’évaluation dans
la modale. Cette friction reste distincte de la livraison et de la revue IA ;
aucun fichier de preuves ni PASS n’a été inventé pour la contourner.

## Verdict exploitable et arrêt des appels

Après correction des règles de citation, le quatrième appel de revue a rendu un
avis `changes_requested` : critère 2 satisfait ; critère 1 inconnu faute de
transmission du livrable distinct ; critère 3 refusé car un seul document autorisé
alors que le moteur impose également un rapport de suivi. Aucun cinquième départ
producteur. Mission mise en pause depuis le cockpit, revue 4/40 appels consommés.

Le correctif moteur transmet le livrable Markdown déclaré avec empreinte et
invalidation si modifié. Tests ciblés de transmission réelle et fraîcheur, `go vet`
et suite complète `go test ./...` : PASS (349,354 s pour la version finale).
La clarification documentaire du critère 3 est sauvegardée dans la préparation,
révision 19, sans application aux tâches. Les critères 1 et 2 et les plafonds du
plan restent inchangés. T1 reste non acceptée ; mission 0/8, sept tâches non parties.
La construction d’une évaluation humaine depuis l’UX reste à traiter ; le formulaire
actuel exige encore un `.evidence.json`. Aucun succès artificiel n’a été enregistré.

## 2026-09-29 — Réviser le contrat sans refaire la production

Défaut confirmé dans `revisePreparedMissions` : une révision invalidait la gate
mais conservait `IndependentReview`, y compris un refus attaché à la tentative
courante. Le conducteur écartait alors une nouvelle revue de cette même tentative.
La révision rematérialisait aussi le plafond du plan (3) sans préserver la reprise
exceptionnelle déjà autorisée (4), même lorsque le plafond du plan ne changeait pas.

Correction : conserver le verdict dans `PreviousReviews`, invalider la revue des
tâches impactées, préserver la reprise si le plafond du plan est inchangé, et refuser
une modification pendant une revue active. Les tentatives et appels consommés restent
intacts. Les modifications de budget seules ne retirent pas un refus de revue.
Tests ciblés : `TestPreparedRevisionPreservesRecoveryAndArchivesReviews`,
`TestTaskDefinitionArchivesOnlyChangedReviewContract`,
`TestTaskDefinitionRejectsActiveIndependentReview` ; réussite constatée.
Cette correction ne constitue pas une validation de T1 : celle-ci reste à obtenir
par le parcours normal sur le rapport existant et le contrat documentaire clarifié.

### Défaut supplémentaire reproduit sur le parcours web réel

Après clarification du plan, « Soumettre le rapport » passait artificiellement par
`running`, puis `submitted`. `Store.apply` créait ainsi une nouvelle `Attempt`
sans agent. Le vérificateur exigeant un producteur associé à la tentative courante,
il ne démarrait pas. Ce défaut explique une attente silencieuse supplémentaire ;
le parcours du conducteur, lui, préservait déjà l'identité de production.

Correction du parcours opérateur : conserver la tentative terminée quand son agent
et son rapport sont attribuables. Pour une ancienne soumission erronée, la réparation
via le même bouton exige l'événement `task.submit`, ses dates, le rapport attribué
et l'absence de tout agent sur l'entrée synthétique. L'entrée déplacée reste dans
`legacy_report_submissions` et l'événement initial reste intact. Aucun appel de revue
ni aucune véritable production ne sont effacés. Les états avec agent réel, événement
absent ou autre rapport sont refusés dans les tests.

Recette réelle : après réparation par le web, la revue T1 démarre effectivement
(5/40 appels affichés), sans cinquième production. Ce démarrage n'est pas encore
un verdict favorable ni une acceptation.

### Résultat réel obtenu : T1 acceptée, compteur 1/8

La revue indépendante du 2026-09-29 à 19:25:53 UTC (appel 6/40) donne `passed`
sur les trois critères, pour l'agent et la tentative d'origine. Elle rappelle
sa limite : lecture des traces exportées, pas d'accès direct au journal natif.
L'assistant superviseur a lu le journal natif et rédigé l'évaluation documentaire
`docs/plan-843bb3ce22-T1.documentary.evidence.json` : aucune exécution de test
applicatif n'y est inventée. Les six contrôles portent sur l'entrée, les trois
critères, la revue documentaire et la livraison. Empreintes des deux documents
liées à la gate, puis fraîcheur de l'avis indépendante revérifiée par le moteur.

Depuis le cockpit : aperçu PASS 6/6, enregistrement de l'évaluation, puis action
« Accepter après revue » / « Confirmer la validation ». Résultat observé :
**1/8 tâches validées**. Pas de dérogation, pas de cinquième production, pas de
hausse des limites, pas de mutation directe de la base. La consommation des
six appels indépendants reste conservée.

Limite UX restante : la gate documentaire doit encore être préparée sous forme
`.evidence.json` avant son enregistrement par l'écran. Ce parcours est fonctionnel
pour un opérateur technique, mais ne mérite pas encore le qualificatif « simple
pour un non-expert ». La création de cette preuve a été une intervention du
superviseur, pas une démonstration d'autonomie de bout en bout.

Après redémarrage sur le binaire final testé (SHA-256
`6c9d9d9be0620efdd024fdc657dd5692a5e1026c95dc55a609d59bbac960c201`),
l'écran conserve 1/8. Le clic « Reprendre la mission » est suivi du départ
**T2 — Moteur de configuration des limites agents**, affiché « En cours »,
Claude Sonnet. Le passage à la tâche suivante est donc observé dans la mission
réelle, et non seulement dans une recette isolée. T2 n'est pas déclarée terminée.


### T2 — Tester le point d'entrée, pas seulement la fonction de calcul

Le producteur déclarait REQ-ADM-05 couvert avec une copie de `RunLimits`,
mais aucun lancement ne lisait la configuration. La revue a correctement
refusé ce résultat. Correction directe du superviseur : résolution dans la
transaction de `prepareLaunch`, test de deux réservations persistées et de
l'immuabilité de la première. Les plafonds déjà autorisés restent prioritaires.
T6 a ensuite atteint son plafond avec un rapport partiel : pas de relance
inchangée ni de validation de ce rapport comme livraison.


### Reprise explicite et migration — correction complémentaire du superviseur

Le refus précédent restait attaché à la tentative même après remise d'un rapport
corrigé. Le moteur archive maintenant ce refus lors d'une soumission explicite du
même rapport attribuable, dont l'empreinte a changé. Un rapport refusé inchangé
est rejeté ; les appels déjà consommés et l'identité du producteur sont conservés.
Le test `TestCorrectedReportSubmissionPreservesRefusalAndSpend` passe pour ces
deux branches. Cela autorise une nouvelle revue, jamais une acceptation implicite.

La suite complète a détecté 11 échecs de migration avec les anciennes fixtures
qui abaissent le numéro de schéma en conservant des tables plus récentes.
La migration v23 réutilise désormais les tables présentes sans les supprimer.
`TestRunLimitsMigrationKeepsExistingConfiguration` vérifie qu'une configuration
à 42 appels et son historique survivent à la réouverture. Le test de migration
v22 compare désormais le schéma courant, tout en vérifiant le minimum v22
et la sauvegarde précédente. Les contrôles de migration ciblés passent : code 0,
`ok swarm.local/companion 1.686s`, journal `/tmp/swarm-admin-migration-targeted.log`.
La suite complète précédente était en échec ; elle n'est pas présentée comme verte.


### Résultat définitif des contrôles du superviseur

`go test ./... -count=1 -timeout 12m` : code 0.

```text
ok  	swarm.local/companion	368.466s
```

`go vet ./...`, `git diff --check` et `go build -o /tmp/swarm-admin-launch .` : codes 0.
Les neuf fichiers source contrôlés correspondent encore à leurs empreintes ci-dessus.
La validation fonctionnelle indépendante de T2 reste à enregistrer dans Swarm.


### Installation et reprise observées dans le cockpit

Binaire installé après suite verte (368.466 s), PID 2576792,
SHA-256 `761403e8a3b47f80c5bd7a3b3805513e3294b30fc8275fc46584fc28e1430192`.
Soumission du rapport corrigé par le formulaire utilisateur : T2 passe de
Bloquée à À vérifier. Mission reprise via son bouton ; revue indépendante
observée en cours, compteur 10/40 contre 9/40 avant, zéro agent de production
actif. Pas de troisième tentative de production, pas de modification directe
SQLite et pas d'acceptation forcée. Le verdict reste à attendre.


## Archive du rapport T2 avant clarification de la remise

Le texte suivant conserve les étapes antérieures, y compris leurs conclusions
invalidées. La remise courante est docs/T2-contrat-moteur.md.

# T2 — Contrat moteur : configuration des limites agents

Producteur : agent `auto-b20a4cd7f84cb5da2842`, attempt `a-e72cd608b58c139aeede6936`, départ 2/3, révision 87.
Mission `w-843bb3ce22cff2965c5e77b6`, tâche `plan-843bb3ce22-T2`.
Racine vérifiée : `/home/fpizzi/workspace/swarm-engine-contract/source`, branche `codex/engine-review-contract`.

Ce rapport est une vérification locale du producteur (DO/CHECK du PDCA), pas une
acceptation moteur ni une revue indépendante.

## 1. Contrat moteur (REQ-ADM-01/03/04/05/06/07 côté moteur)

Fichiers : `run_limits_admin.go` (implémentation), `run_limits_admin_test.go`
(tests ciblés), migration de schéma v23 dans `store.go`, `schemaVersion=23`
dans `model.go`.

- **Pas de seconde source de vérité** (REQ-ADM-03) : même base `.swarm/state.db`
  / `*Store` que budgets/quotas/pricing/provider-admin ; deux nouvelles tables
  seulement (`run_limits_config`, `run_limits_history`), aucun nouveau fichier
  ni service.
- **Schéma des paramètres** : type `RunLimits` existant réutilisé tel quel
  (silence, temps outil, plafond d'appels, tentatives répétées, erreurs
  consécutives). 0 = hérité (convention déjà en usage, pas une nouvelle
  sémantique).
- **Portées hiérarchiques** (REQ-ADM-04) : `project < mission < role < task`.
  `effectiveRunLimits(mission, role, task)` résout la valeur effective en
  surchargeant uniquement les champs non nuls de chaque portée, de la plus
  générale à la plus spécifique ; relit le store à chaque appel (pas de cache).
- **Validation** (`validRunLimitsOverride`, `validRunLimitsScope`) : mêmes
  bornes que `RunLimits.normalized()` ; portée invalide, identifiant de
  mission/clé manquant ou valeur hors bornes sont refusés avec une erreur
  explicite. Une valeur tout-zéro (héritage pur) est acceptée.
- **Historisation append-only** (`run_limits_history`) : chaque écriture
  ajoute une ligne, jamais de réécriture. Concurrence optimiste via
  `expected_revision` ; rejeu idempotent d'un `event_id` identique (même
  contenu → pas de nouvelle révision ; contenu différent → refusé).
- **Rollback** (REQ-ADM-06) : `rollbackRunLimits` relit l'historique, réapplique
  les valeurs d'une révision passée comme **nouvelle** révision (jamais de
  suppression/réécriture), et marque `rollback_of` avec la révision d'origine.
- **Distinction futurs départs / tentatives en cours** (REQ-ADM-05) :
  `effectiveRunLimits` ne lit et ne modifie jamais un `Agent` en cours
  d'exécution ; `Agent.Limits` reste une copie de valeur figée au lancement.
  Le branchement de cette résolution dans le chemin de lancement réel n'est
  **pas** fait dans cette tâche (Lot 1, moteur uniquement) — limite explicite,
  voir §4.
- **Garde-fous** (REQ-ADM-07) : aucun secret dans ce schéma ; aucune remise à
  zéro de consommation ; aucune hausse automatique de budget ; aucune
  suppression de protection. (Tarifs/coût non rapporté : hors périmètre T2,
  non traité ici — voir T1.)

## 2. Défauts trouvés et corrigés pendant cette tentative

L'attempt précédente (1/3) avait laissé le code et les tests écrits mais
l'exécution des tests inachevée (compilation du binaire de test non
terminée). Cette tentative a exécuté réellement les tests et corrigé deux
défauts réels révélés par cette exécution :

1. **`rollback_of` jamais enregistré** : `configureRunLimits` insérait
   toujours `NULL` en dur pour `rollback_of`, quel que soit l'appelant.
   `rollbackRunLimits` ne transmettait pas la révision d'origine. Correction :
   ajout du champ `RollbackOf` à `RunLimitsConfigChange`, `rollbackRunLimits`
   le renseigne, `configureRunLimits` l'insère (`NULL` si 0, sinon la valeur).
2. **Blocage (deadlock) sur le rejeu idempotent** : le pool de connexions du
   `Store` est plafonné à une connexion (`db.SetMaxOpenConns(1)`, `store.go`).
   Sur un rejeu d'`event_id` identique, `configureRunLimits` retournait
   `s.currentRunLimitsConfig(...)`, qui interroge `s.db` — alors que la
   transaction `tx` en cours détenait encore l'unique connexion (le
   `defer tx.Rollback()` ne s'exécute qu'après évaluation de la valeur de
   retour). Le test `TestRunLimitsConfigConcurrencyAndReplay` restait bloqué
   indéfiniment sur ce chemin. Correction : nouvelle fonction
   `currentRunLimitsConfigWith(q rowQuerier, ...)` paramétrée par un lecteur
   (`*sql.DB` ou `*sql.Tx`) ; le chemin de rejeu lit désormais via `tx`, pas
   via `s.db`.

Ces deux corrections comptent comme les deux corrections autorisées avant
OODA ; les tests passent maintenant, aucune OODA nécessaire.

## 3. Preuves — critères assignés

Commande unique pour les 4 critères (identifiée en T1, cf. suivi local) :

```
go build ./...
go test ./... -run TestRunLimits -v
```

Sortie observée (après corrections, horodatage local de cette tentative) :

```
=== RUN   TestRunLimitsConfigRejectsInvalidValue
--- PASS: TestRunLimitsConfigRejectsInvalidValue (0.01s)
=== RUN   TestRunLimitsConfigPersistsAfterRealRestart
--- PASS: TestRunLimitsConfigPersistsAfterRealRestart (0.01s)
=== RUN   TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected
--- PASS: TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected (0.02s)
=== RUN   TestRunLimitsConfigConcurrencyAndReplay
--- PASS: TestRunLimitsConfigConcurrencyAndReplay (0.01s)
PASS
ok  	swarm.local/companion	0.084s
```

`go build ./...` : exit 0. `go vet ./...` : exit 0.
`go test ./... -run TestRunLimits -v` : exit 0.

| ID | Critère | Test | Résultat | Preuve |
| --- | --- | --- | --- | --- |
| req-4 | Refus testé de valeur invalide | `TestRunLimitsConfigRejectsInvalidValue` | **PASS** | log ci-dessus, l. 1-2 |
| req-5 | Persistance vérifiée après redémarrage réel | `TestRunLimitsConfigPersistsAfterRealRestart` (ferme `s.db`, `openStore` réel, relit) | **PASS** | log ci-dessus, l. 3-4 |
| req-6 | Historique et rollback fonctionnels et testés | `TestRunLimitsHierarchyResolutionAndFrozenAttemptUnaffected` (historique 3 entrées, `rollback_of=1` vérifié, révision non réécrite) | **PASS** | log ci-dessus, l. 5-6, après correction §2.1 |
| req-7 | Distinction futurs départs / tentatives en cours (REQ-ADM-05) | copie de valeur uniquement | **PARTIAL à cette étape** | ne prouvait pas le lancement réel ; correction du superviseur ci-dessous |

Test additionnel non assigné mais couvrant la même surface (concurrence
optimiste + rejeu idempotent) : `TestRunLimitsConfigConcurrencyAndReplay` —
**PASS** après correction §2.2 (préalable : ce test bloquait indéfiniment
avant la correction, ce qui aurait invalidé toute preuve de req-4/6/7 tant
que le binaire de test ne terminait pas).

Limite : ces tests sont des tests unitaires en base réelle (fichier SQLite
temporaire, pas de double), pas une recette bout-en-bout moteur↔CLI↔web —
hors périmètre T2.

## 4. Limites et prochaine action

- **Non fait dans T2** : brancher `effectiveRunLimits` dans le chemin de
  lancement réel (`Provider.Limits`/`Launch.Limits`). C'est le point
  d'extension pour T2→lots suivants (CLI en Lot 2, web en Lot 3) : ils doivent
  appeler `effectiveRunLimits` au moment de la résolution pré-lancement, sans
  jamais relire/modifier un `Agent.Limits` déjà figé.
- Pas de commande CLI ni d'écran web pour ce moteur — hors périmètre T2
  (REQ-ADM-02 = Lot 2, REQ-ADM-01/06 web = Lot 3).
- Tarifs fournisseur (partie du schéma des paramètres mentionnée dans le
  scope) : non traités ici, cf. T1 pour l'état des mécanismes existants.
- Ce rapport est une vérification locale du producteur (DO/CHECK), pas une
  acceptation par le moteur ni une revue indépendante. Revue indépendante et
  gate fraîche requises avant les Lots 2 et 3 (critère de delivery du plan).

## 5. Décisions de conception (rappel, cf. suivi local pour l'historique complet)

- Pas de nouvelle base de données ; mêmes fichiers `.swarm/state.db` / `*Store`.
- `RunLimits` réutilisé tel quel comme type de valeur à chaque portée.
- Rollback = nouvelle révision qui recopie une valeur historique, jamais une
  suppression/réécriture.
- `effectiveRunLimits` ne lit et ne modifie jamais un `Agent` en cours.


## Correction du superviseur — 29 septembre 2026

Intervention directe de Codex après la fin des agents, mission en pause ;
ne pas attribuer cette modification à la tentative Claude précédente.
Elle remplace la limite « branchement non fait » du rapport initial.

Le chemin partagé `Store.prepareLaunch` lit maintenant les quatre portées
administratives via sa propre transaction SQLite, après réservation du writer.
Les valeurs sont figées dans `Agent.Limits` et la consigne enregistrée.
Une modification administrative n'altère aucune tentative déjà enregistrée.
Les plafonds fournisseur, demande explicite, plan et reprise restent applicables.
Une préférence administrative supérieure à ces plafonds ne les relève pas.
Le terminal natif refuse une configuration administrative qu'il ne sait pas mesurer.

`TestRunLimitsActualLaunchFreezesConfiguration` réserve réellement deux agents,
avec SQLite temporaire et fournisseur de test, sans appel IA : le premier reçoit
30 appels et conserve 30 après une modification de configuration ; le second
reçoit 12. Les valeurs relues depuis la table des agents sont vérifiées, ainsi
que la consigne transmise. Ceci remplace le test insuffisant d'une simple copie Go.
`TestRunLimitsLaunchCannotRaiseCeilings` confirme que la préférence 900 reste
plafonnée à 100 par le fournisseur et à 7 par la demande explicite.

Contrôles exécutés directement par le superviseur :
- `go test ./... -run TestRunLimits -count=1 -timeout 60s` : code 0,
  `ok swarm.local/companion 0.189s`.
- `go test -race ./... -run TestRunLimits -count=1 -timeout 90s` : code 0,
  `ok swarm.local/companion 2.601s` ; journal `/tmp/swarm-admin-launch-race.log`.
- `go vet ./...` et `git diff --check` : codes 0.
- Suite complète terminée ; résultat définitif ci-dessous.

Ces contrôles sont des tests d'intégration du moteur sur SQLite isolée, pas
une démonstration du parcours Administration web/CLI livré (T3/T4 à réaliser).
L'ancienne revue défavorable est conservée ; aucune acceptation implicite.


### Contre-épreuve du superviseur

Simulation isolée par `go test -overlay` : suppression du seul appel qui applique
les limites administratives, sans modifier le dépôt. Commande :
`go test -overlay /tmp/swarm-admin-regression-overlay.json ./... -run '^TestRunLimitsActualLaunchFreezesConfiguration$' -count=1 -timeout 60s`.
Résultat attendu et observé : code 1, `configuration not frozen in launch`,
`MaxToolCalls:100` au lieu de 30. Le test détecte donc exactement le défaut
initial ; il ne se contente pas de vérifier le calcul déconnecté du lancement.
Journal : `/tmp/swarm-admin-launch-regression.log`. L'overlay ne sert ni au
build livré ni à la suite complète.


### Reprise explicite et migration — correction complémentaire du superviseur

Le refus précédent restait attaché à la tentative même après remise d'un rapport
corrigé. Le moteur archive maintenant ce refus lors d'une soumission explicite du
même rapport attribuable, dont l'empreinte a changé. Un rapport refusé inchangé
est rejeté ; les appels déjà consommés et l'identité du producteur sont conservés.
Le test `TestCorrectedReportSubmissionPreservesRefusalAndSpend` passe pour ces
deux branches. Cela autorise une nouvelle revue, jamais une acceptation implicite.

La suite complète a détecté 11 échecs de migration avec les anciennes fixtures
qui abaissent le numéro de schéma en conservant des tables plus récentes.
La migration v23 réutilise désormais les tables présentes sans les supprimer.
`TestRunLimitsMigrationKeepsExistingConfiguration` vérifie qu'une configuration
à 42 appels et son historique survivent à la réouverture. Le test de migration
v22 compare désormais le schéma courant, tout en vérifiant le minimum v22
et la sauvegarde précédente. Les contrôles de migration ciblés passent : code 0,
`ok swarm.local/companion 1.686s`, journal `/tmp/swarm-admin-migration-targeted.log`.
La suite complète précédente était en échec ; elle n'est pas présentée comme verte.


### Journal des contrôles directement exécutés par le superviseur

Ces sorties ont été capturées par Codex, après intervention sur le code,
et ne proviennent pas d’une déclaration de réussite du worker.

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

Empreintes des sources contrôlées :

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
}
```


### Résultat définitif des contrôles du superviseur

`go test ./... -count=1 -timeout 12m` : code 0.

```text
ok  	swarm.local/companion	368.466s
```

`go vet ./...`, `git diff --check` et `go build -o /tmp/swarm-admin-launch .` : codes 0.
Les neuf fichiers source contrôlés correspondent encore à leurs empreintes ci-dessus.
La validation fonctionnelle indépendante de T2 reste à enregistrer dans Swarm.


## T2 validée, progression réelle 2/8

Avis indépendant favorable observé le 29 septembre 2026 à 20:31:23 UTC :
quatre critères reconnus après fourniture du code exact de tous les tests.
Les refus successifs sont conservés ; le vérificateur a changé son appréciation
selon les pièces disponibles. Cela démontre le besoin d'un dossier de revue
complet dès la première remise (code ciblé, commandes, sorties, contre-épreuve),
plutôt qu'un résumé PASS ou un document mêlant état courant et anciens défauts.

Gate T2 enregistrée par le formulaire, 7/7 contrôles, puis acceptation normale
« Accepter après revue », sans dérogation. Limite explicite : les contrôles ont
été exécutés directement par Codex puis leurs preuves importées ; ce n'est pas
un reçu d'exécution automatisée émis par le moteur (l'UX l'indique unknown).
Aucun reçu n'a été fabriqué. Sources et rapports sont liés par leurs empreintes.

Le cockpit affiche maintenant 2/8 tâches validées. T2 est « Terminée et validée »
et T3 « CLI Administration miroir du moteur » a démarré automatiquement,
avec un agent Claude/Sonnet actif. T5 attend une preuve CI distante, T6 reste
partielle après son interruption. Aucun succès global 8/8 n'est revendiqué.


### T3 — Ne pas contourner une protection de consultation

La première tentative a livré le CLI mais atteint son plafond avant le handoff.
La reprise ciblée a remis les preuves et terminé normalement. Le superviseur
a constaté une décision erronée : ne pas inscrire les commandes show/history/
effective dans cliStorageInspection, pour migrer implicitement une base ancienne.
Correction : respecter storage_upgrade_required et la migration explicite via init.
Le test existant inclut désormais ces trois commandes et prouve le schéma inchangé.
Tests ciblés : code 0, 0.168 s. Pas de mutation de la base réelle par CLI.


### T5 — Une preuve distante doit remplacer l'ancien état non testé

Le worker avait uniquement reproduit la séquence CI localement. Le superviseur
a poussé le branchement CI sur codex/browser-ci-proof puis déclenché et consulté
le run GitHub 36628140996 (commit c5f1dea), jobs checks/install success.
Un premier ajout au rapport conservait le titre « pas d'exécution GitHub réelle »,
ce qui entretenait une contradiction. Rapport courant normalisé avec réponse
GitHub horodatée et journal brut de l'étape navigateur (PASS bilingual web UI).
L'ancienne limite était réelle pour le worker ; elle ne décrit plus l'état actuel.
La preuve porte sur T5/base097e745, pas sur les changements locaux T2/T3.


### T3 acceptée — 29 septembre, clôture supervisée

Suite complète exécutée : go test ./... -timeout 12m, PASS en 364.374 s ;
go vet et build réussis. Empreintes des cinq sources vérifiées avant installation.
Binaire installé : 7970e68d697fd0bed4261dba008be6bb30c6a9df88353917dad706f6aae6c17b.
Avis indépendant favorable sur les trois critères, gate 6/6, acceptation normale
par le cockpit. Le compteur passe à 3/8. Les contrôles sont ceux du superviseur
et les preuves importées, sans reçu automatique du moteur ni dérogation.

T5 : nouvelle interruption de forme, citation JSON contiguë absente alors que
les valeurs GitHub sont présentes. Ajout du même résultat sérialisé sur une ligne,
sans changement de valeurs, puis reprise explicite de la vérification via UI.
Ce défaut de citation n'est pas un échec de CI ; ne pas relancer la production
pour une erreur du vérificateur. Mission reprise après installation.


### T5 — Blocage structurel confirmé après reprise de citation

L'appel indépendant 18/40 a accepté le critère 3 (couverture FR/EN x thèmes)
mais gardé les critères 1/2 unknown : sa session sans outils ne peut authentifier
les journaux GitHub et le cycle rouge/vert. Aucun défaut de code démontré.
Les sorties distantes ont pourtant été collectées par le superviseur (contexte
distinct du worker), run 36628140996, et sont jointes au rapport.
Ne pas multiplier les mêmes reprises, ni accepter par dérogation.
Correctif du circuit à traiter : fournir au vérificateur un accès de lecture
aux preuves d'exécution authentifiées, ou un reçu produit par un contrôle
moteur réellement exécuté. Ne jamais convertir un rapport importé en reçu
moteur fictif. Etat public conservé : 3/8 acceptées, T5 corrections/preuves
demandées, T4 en attente, T6 partielle.


### 30 septembre — Dépendance circulaire supprimée dans le moteur

Cause vérifiée : runAutomaticValidation exigeait independentReviewGuard avant
d'exécuter les contrôles, alors que la revue sans outils attendait leurs preuves.
Choix : réutiliser les contrôles préautorisés et les reçus existants, plutôt que
donner des commandes arbitraires au vérificateur ou assouplir l'acceptation.

Le moteur exécute les contrôles, conserve pending_review et la gate, transmet
les reçus courants à la revue, puis accepte après avis favorable sans rejouer
les commandes. Une gate importée ne devient jamais un reçu moteur. Tentative,
politique, empreintes, sortie et fraîcheur sont vérifiées ; un échec reste bloqué.
Tests de régression : contrôles avant revue, non-répétition, rejet sans revue,
rejet gate seule, autre tentative et preuves périmées. Tests ciblés PASS 0.169s.
Suite complète go test ./... -timeout 12m : PASS 404.671s. go vet, build,
git diff --check réussis. Binaire installé :
9ace9f7f1ed059a3264a32da1ff17bf1e2f0fb933ef95ba80e145763922a51fe.

T5 : six commandes préautorisées dans l'UX, budget cumulé 300s ; empreinte
du script imposée en argument. Contrôle GitHub authentifié avec gh, contre-
épreuve navigateur en copie isolée vert/rouge/vert, puis FR/EN et deux thèmes.
Essai supervisé du script : GitHub PASS, mutation PASS après ajout d'un script
fixture oublié à la copie isolée (erreur de montage de recette corrigée).
Rapport enrichi du code complet. Soumission et reprise effectuées dans l'UX.
Les reçus et l'acceptation réelle de T5 restent à observer, aucun succès inventé.


### Résultat observé — T5 acceptée automatiquement, 4/8

Le 30 septembre, six contrôles réellement exécutés par le moteur entre
08:06:01 et 08:06:43 UTC, tous executed=true, passed=true, exit_code=0.
Le contrôle de mutation a pris environ 28 secondes ; le navigateur quatre
combinaisons environ 12 secondes. Le reçu conserve les horodatages exacts.

Le vérificateur indépendant a donné un avis favorable aux trois critères,
en s'appuyant explicitement sur engine_controls. Le conducteur a ensuite
accepté T5 sans dérogation et sans rejouer les commandes. Le cockpit affiche
4/8 validées et T4 Administration web a démarré automatiquement, avec un
agent Claude actif. Les tentatives et le budget IA existants sont conservés
(19/40 appels de revue utilisés). Aucun succès 8/8 n'est revendiqué.

Le défaut corrigé était le séquencement contrôle/revue et l'absence de reçus
dans le contexte du vérificateur, pas un besoin de relancer la production ou
de multiplier les appels IA. T6 reste à terminer ; la suite dépend de T4/T6.


### 30 septembre — Appels des exécutants et reprise utile

Mesure en lecture seule des enregistrements moteur :
- T4 première tentative : 60 appels / 59 résultats ; 32 Bash, 26 Read,
  1 Agent et 1 SubagentHandback, aucune écriture observée.
- T4 seconde tentative : 60 appels / 59 résultats ; 31 Bash, 14 Read,
  4 Write et 11 Edit. Le travail n'était donc plus uniquement exploratoire.
- T6 première tentative : 40 appels / 39 résultats ; 25 Bash, 11 Read,
  1 Write et 3 Edit.
Ces nombres sont des outils, pas des appels IA ni des coûts en dollars.

Défaut reproduit : loopGuard.call arrêtait dès calls >= max, avant le résultat
du dernier outil pourtant autorisé. Correctif : attendre ce résultat, sous
les délais existants, arrêter tout appel supplémentaire et conserver les
protections répétitions/erreurs. Un test avec un processus réel écrit un fichier
après 700 ms : ce fichier doit exister après l'arrêt et le résultat est compté.
Ce test est un fournisseur simulé, pas une preuve d'efficacité d'un LLM réel.

Reprise : avant de démarrer le fournisseur, le superviseur joint les opérations
structurées de l'agent précédent de la même tâche/mission, bornées, dédupliquées
et masquées pour les informations sensibles. Aucun résultat brut ni rapport
d'une autre tâche. Le contexte effectivement transmis est enregistré dans le
prompt de l'agent. Il n'est ni une instruction prioritaire ni une preuve de succès.

Les consignes calculent des jalons d'exploration et de finalisation à partir du
budget gelé : pour 40 appels, exploration 10, réserve 8, finalisation dès 32.
Ces jalons sont des instructions au modèle, pas une garantie d'obéissance ni
un mécanisme natif de notification après chaque outil. Aucun plafond augmenté.

Alternative écartée : augmenter les plafonds ou relancer les mêmes consignes.
La décomposition automatique et le retour dynamique au fournisseur restent
des évolutions distinctes ; ne pas les présenter comme implémentés par ce lot.

### Axe d'article — Piloter des agents probabilistes avec un moteur déterministe

**Thèse.** La qualité du résultat ne suffit pas à mesurer la qualité d'un agent.
Il faut aussi examiner son parcours : opérations utiles, explorations répétées,
coût, durée, reprises, collisions et qualité des preuves remises. Un moteur
explicite rend ce parcours observable et gouverne les transitions autorisées ;
il ne rend pas les décisions du modèle déterministes.

**Ce que cette expérience démontre.** Les compteurs et traces ont révélé une
tentative T4 avec 60 appels d'outils sans écriture, puis une autre avec 15
écritures ou modifications. L'absence d'écriture n'est pas, seule, une preuve
d'inutilité : une exploration peut produire une décision utile. Il faut relier
chaque opération à un acquis et à un critère de la tâche. L'expérience a aussi
révélé une faute du moteur : interrompre le dernier outil avant son résultat.
La consommation excessive ne doit donc pas être imputée exclusivement au LLM.

**Prudence sur les coûts.** Appels d'outils, requêtes au modèle, tokens d'entrée,
tokens de sortie et facturation sont des mesures distinctes. Un outil peut
produire beaucoup de texte à réinjecter dans le contexte ; des reprises peuvent
répéter lectures et contexte. C'est un mécanisme possible de surconsommation,
pas une mesure financière démontrée ici. Les caches et le mode de facturation
modifient également le coût. Ne pas écrire « 60 appels = 60 appels IA » ni
extrapoler cette mission à tous les modèles de pointe. « Peu visibles dans
l'interface » est plus exact que « appels cachés ».

**Leçons à développer.**
- Observer les opérations et leurs effets réels, pas seulement le récit de
  l'agent. Un journal d'outils ne révèle pas son raisonnement interne.
- Mesurer le coût et le temps par résultat validé, les lectures répétées,
  les reprises, les changements utiles et les interventions humaines.
- Garder une mémoire de reprise bornée : préserver les acquis sans réinjecter
  tout l'historique ni confondre un ancien rapport avec une preuve actuelle.
- Adapter le découpage au périmètre et aux dépendances ; le parallélisme n'est
  utile que si les tâches ont des frontières et des contrats d'intégration.
- Prévenir les collisions par attribution des espaces, isolation des écritures,
  échanges explicites, contrôle des versions et intégration vérifiée. Un graphe
  visible seul n'offre aucune de ces garanties.
- Distinguer travail produit, contrôles exécutés, avis indépendant et acceptation.
  Un vérificateur IA reste probabiliste ; son verdict n'est pas une preuve absolue.
- Borner l'exploration et préparer une remise exploitable avant l'épuisement du
  budget. Une limite protège la consommation mais peut gaspiller le travail si
  elle coupe une écriture ou force une nouvelle exploration au redémarrage.
- Auditer le moteur lui-même : transitions impossibles, dépendances circulaires,
  doubles départs, résultats périmés, reprise après panne et conservation des
  opérations en cours. Un moteur déterministe peut être déterministement faux.

**Protocole comparatif à réaliser, non encore réalisé.** Même tâche, même
révision initiale, même modèle/configuration et mêmes critères d'acceptation ;
comparer agent seul et orchestration sur plusieurs exécutions. Relever succès,
temps, tokens mesurés, coût disponible, appels d'outils et interventions humaines.
Inclure le coût de coordination et de vérification. L'objectif est le minimum
raisonnable de ressources pour un résultat fiable, pas le minimum d'appels à
n'importe quel prix.

### Installation et expérience à bornes hautes — 30 septembre

Validation du correctif de reprise : suite Go complète PASS (665,463 s),
tests ciblés avec détecteur de courses PASS (5,228 s), go vet et contrôle
workflow PASS, git diff --check sans erreur. Binaire installé :
7971d4f3f9260a2ba9404ef06c6fcdb59d9a014a52ed74e4391f3f0baf16b5fe.

Sur autorisation explicite de l'utilisateur, révision 28 du plan appliquée
par l'interface : T4, T6, T7 et T8 à 100 appels et 3 tentatives au total.
T1/T2/T3/T5 conservées. Administration de cette mission enregistrée via
prévisualisation puis confirmation : 100 appels, outil 900 s, silence 300 s,
répétitions et erreurs héritées, sans reset des compteurs. Ce sont des
valeurs configurées : des plafonds plus stricts restent prioritaires.

Une friction supplémentaire a été observée : l'action « Relancer » impose
encore les limites de la tentative précédente. T4 agent
1d0f0b55-1f50-4020-b5b7-050ddf4f3226 a donc réellement démarré avec 60 appels,
outil 300 s, silence 180 s malgré le plan révisé. La mémoire de reprise est
présente dans le prompt réellement enregistré. Ne pas présenter cette
tentative comme ayant un plafond effectif de 100. Le résultat est encore
à obtenir. Le besoin de distinguer clairement configuration souhaitée et
limites effectives est confirmé ; une hausse autorisée ne doit pas être
silencieusement neutralisée par un ancien contrat de reprise.

### Demande explicite de retirer les plafonds — mode observation

L'utilisateur demande de reprendre sans plafonds moteur et d'analyser ensuite
les appels par agent. Ajout d'un choix administratif réversible, au niveau
mission uniquement : `observation_mode: 1`. Une requête de lancement ou un
fournisseur ne peut pas l'activer à la place de l'opérateur. L'historique
administratif conserve le motif et la révision ; aucun compteur n'est effacé.

Le mode est figé dans chaque nouvelle tentative. Il neutralise les arrêts
quantitatifs d'exécution (appels, répétitions, erreurs, silence, outil et durée
totale), ainsi que le refus d'un départ pour nombre de tentatives atteint.
Il s'applique aussi aux reprises malgré leurs anciens plafonds. L'arrêt manuel,
les dépendances fraîches, l'exclusion des écritures concurrentes, les erreurs
fournisseur et les exigences d'acceptation restent applicables. Les limites des
sessions de planification/revue sont distinctes ; ce lot ne les supprime pas.
Pas de boucle de relances automatiques illimitée ajoutée.

Recettes spécifiques : dépassement d'appels/erreurs conservant les compteurs,
reprise après épuisement sans effacement, réactivation des limites, tentative
active inchangée, fournisseur de test terminant après son ancien délai et
arrêt manuel d'un fournisseur de test actif. Interface : activation puis
désactivation via prévisualisation/confirmation, FR/EN et deux thèmes. Les
simulations ne démontrent pas l'efficience future du modèle réel.

État distinct observé au début de ce lot : T1 avait été rouverte depuis le
précédent contrôle, ramenant les validations courantes à 0/8 par propagation
des dépendances. Son rapport documentaire inchangé a été resoumis et sa gate
réévaluée PASS 6/6 via le web, puis accepté sans dérogation : 2/8 courantes.
T2 et T3 nécessitent une nouvelle revue après modification du moteur ; les
réaccepter sur des empreintes périmées masquerait la portée du correctif.

Activation et reprise réelles le 30 septembre 2026 : configuration mission
enregistrée depuis Administration avec motif utilisateur, puis reprise T6
depuis le cockpit. Tentative `7610d86e-6538-440e-a4a4-51b4767defc8`,
fournisseur Claude/Sonnet, état moteur `running`, `observation_mode: 1`,
premier appel Bash observé. Les valeurs historiques 40 appels / 300 secondes
par outil restent sérialisées mais sont inactives dans ce mode ; ce ne sont
pas les plafonds effectifs de cette reprise. Aucun résultat T6 encore validé
à cet instant. Baseline des 14 tentatives précédentes conservée dans
`docs/observation-baseline-20260930.json`.

Validation du correctif : suite Go complète réussie (678 s), suite ciblée
finale réussie après conservation du rôle de lancement (12,8 s), tests
observation avec détecteur de courses réussis (12,3 s), go vet et npm test
réussis, recette Administration navigateur FR/EN et deux thèmes réussie.
Captures contrôlées visuellement ; contrôles workflow et git diff --check
réussis. Binaire installé : SHA-256
`70f3eb4af0310b8f9711053a608d0cf5dceafb268c84d4d2f53c7db46f748c62`.

Friction du parcours : un formulaire initialement affiché sans configuration
a refusé la prévisualisation pour révision périmée. Rechargement de la portée
puis modification de la révision existante ont permis l'enregistrement.
La garde de concurrence a fonctionné ; l'affichage du chargement mérite une
correction distincte. Les appels d'outils ne sont pas un compteur de requêtes
modèle ni de tokens facturés : le RETEX devra distinguer ces mesures.

### T6 après suppression des plafonds : distinguer exécution et preuve

La tentative `7610d86e-6538-440e-a4a4-51b4767defc8` a fini normalement
(exit 0), avec 53 appels / 53 réponses : l'ancien plafond de 40 n'a pas
interrompu l'exécution. Coût fournisseur affiché : 1,52 USD. La revue a
ensuite classé les critères inconnus car `engine_controls` était vide.
C'est une lacune d'acheminement des preuves, pas un dépassement d'appels ni
un défaut fonctionnel démontré.

Correction opérationnelle depuis le web : politique structurée T6 avec cinq
contrôles (`tools/verification/t6_quickwins.py`), rapport complété et resoumis.
Recette supervisée préalable verte : test Go réel, build et navigateur FR/EN
x deux thèmes, aucune requête externe ni erreur JavaScript, CTA et focus.
Captures clair EN / sombre FR relues visuellement. Le moteur a ensuite
engagé une nouvelle revue indépendante ; ne pas anticiper son verdict.

Réouverture T2 pour preuve périmée : le conducteur actif a immédiatement
lancé une nouvelle tentative `auto-0b1a66811cf645081a7a` avec
`observation_mode: 1`. Cela révèle une friction supplémentaire : renouveler
une preuve documentaire et relancer un exécutant sont deux intentions à
distinguer dans le parcours. La reprise observée exécute build/tests/vet ;
elle ne constitue pas encore une validation de T2.

Résultat confirmé ensuite dans le cockpit (révision 170, 30 septembre à
13:04 heure locale) : T6 « Terminée et validée », contrôles préautorisés et
revue indépendante réussis. Mission à 3/8 validations actuelles, T2 en cours
et T3 à revalider. La correction du passage des preuves a permis de valider
T6 sans quatrième tentative de développement.

### Revalidation T2 et reprise T4 — 30 septembre, 13:17

T2 a terminé sa reprise en 19 appels / 19 réponses, mais sa première revue
restait sans engine_controls. Sept tests Go ciblés ont été préautorisés via
le web et le rapport resoumis, sans deuxième tentative de développement.
Le moteur a enregistré contrôles et revue indépendante réussis (révision180).
La mission passe à 5/8 : T3 retrouve sa validité avec sa dépendance T2 validée,
sans réouverture ni réexécution inutile. T4 a été relancée via le web avec
une consigne de finalisation des quatre critères et des deux rapports,
réutilisant l’écran et la recette existants, mode observation conservé.

### T4 : fin en 25 appels, puis visibilité insuffisante des assertions

La reprise T4 a terminé normalement en 25 appels / 25 réponses ; script
`tools/verification/t4_admin.py` et rapports produits. Sept contrôles ont
été préautorisés via le web. Le vérificateur a reconnu les captures mais
classé trois critères inconnus car le dossier contenait le wrapper Python,
pas les assertions de `tests/run_limits_admin_ui.cjs`. Le résultat agrégé
PASS ne suffisait pas à juger la couverture. Correction : joindre le code
exact de la recette et son SHA-256 au rapport puis resoumettre le même
résultat, sans relancer un worker. Ce coût relève de la préparation du
dossier de revue, distinct des appels de développement et des tests.

### Défaut moteur confirmé : reçu bloqué après rapport complété

La resoumission du rapport T4 enrichi conservait un AutoValidation en état
pending_review. Le moteur reconnaissait les empreintes périmées mais
réutilisait toujours le même reçu, sans rejouer les contrôles. Cause localisée
dans runAutomaticValidation : retour anticipé fondé seulement sur tentative
et politique. Correction : si le rapport lié au reçu a changé, exécuter de
nouveau les contrôles préautorisés ; si son contenu est identique, réutiliser
le reçu. Aucun nouveau worker, aucun compteur remis à zéro.

Test de régression : un premier contrôle, changement du rapport, un seul
contrôle supplémentaire, puis aucun rejeu au sondage suivant ; acceptation
impossible sans revue actuelle. Tests ciblés passés (3,799 s), go vet et build
réussis. Suite complète et test race lancés séparément. Déploiement binaire
SHA-256 b1496b5a81ea54247b14969611ea2a8fe84b8bf922b0d87fd91073ac9ed8ce9d.

Vérification après déploiement : la mission est passée de r191 à r193,
nouveau reçu courant puis revue indépendante T4 en cours. Le test race
ciblé est vert (6,010 s). Ce défaut de renouvellement appartenait au moteur,
pas au modèle producteur : le worker avait déjà fini en 25 appels.

Résultat réel après correctif : révision196, T4 terminée et validée, mission
à 6/8. T7 a démarré automatiquement sous l’agent
`auto-c4a11448f63053bb705e`, avec observation_mode=1. La réparation du
renouvellement des reçus a évité une cinquième tentative de T4. Le verdict
indépendant favorable garde ses limites : jugement documentaire des
assertions et des reçus, pas une exécution personnelle du modèle réviseur.

Validation complète du correctif de renouvellement des reçus :
`go test ./... -timeout 15m` terminé avec succès, 845,575 s ; test race
ciblé 6,010 s ; go vet et build réussis. Le binaire testé a été déployé
avant cette suite, après tests ciblés. T7 a travaillé en parallèle sur les
écrans : ce passage Go ne remplace pas sa future recette navigateur.


## 30 septembre — T7 récupérée après 429, vérification des preuves

T7 s’est arrêtée sur quota fournisseur Claude après 127 appels/127 résultats, et non sur un plafond moteur. Aucune tentative supprimée ou consommation remise à zéro. Le rapport était resté au stade initial alors que la recette avait déjà produit un résultat PASS ; Codex a repris les fichiers, sans attribuer son travail au processus interrompu.

Le PASS masquait deux problèmes vérifiés : les captures « sombre » étaient claires (bouton derrière la modale, sans assertion du thème), et le diagnostic anglais conservait causes/actions en français (seuls les libellés du bouton étaient contrôlés). Correction de la recette, ajout des traductions et des assertions sur le contenu, test du refus de copie, puis recette isolée et npm test réussis. Le presse-papiers système reste simulé, explicitement documenté. Les captures actuelles sont liées aux empreintes des sources par tools/verification/t7_prelaunch.py.

Le compteur public reste 6/8 : cette intervention n’est pas une acceptation indépendante. Une revue qualitative reste requise ; T8 dépend de T7. Le quota fournisseur annoncé expire à 13:00 UTC (15:00 Paris), sans garantie de disponibilité à cette heure. Ne pas relancer toute l’exploration : repartir du rapport T7, du contrôle reproductible et des écarts explicitement listés.


### Suite T7/T8 pendant le quota fournisseur

Les détails techniques du prévol sont maintenant dans une modale ; son ouverture, Échap, fermeture explicite et retour du focus sont contrôlés en FR/EN. Le récapitulatif ne masque plus les arguments de validation automatique manquants. Documentation INSTALL FR/EN complétée avec Administration, lecture CLI, diagnostic, reprise et captures réelles de fixtures isolées légendées. Ces travaux locaux ne sont ni une nouvelle tentative fournisseur ni une acceptation des tâches T7/T8.
