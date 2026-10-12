# T2 — remise au responsable après réparation externe

Mission `w-ca937edf4e0abd85f8d3d67b`, tâche `plan-ca937edf4e-T2`, agent
`auto-a23c60f03a732c02841f`, tentative `a-2f9e1debb1548be96273f0d3` (3e/3).
Le corps ci-dessous jusqu'à « Reprise du précontrôle de contexte » est hérité
de l'agent/tentative précédents (`auto-f2abbcfe6bed66fd1a5e` / `a-a349f9134d355713576b7166`,
statut **interrupted**, interrompue le 11 octobre 2026 à 08:36:27 UTC, code 143,
après 1 800 secondes, 78/80 appels) et conservé comme contexte. Cette tentative
a intégré leur résultat réparé par fast-forward
(`git merge --ff-only 736460f60459595c61df1d953dec77ac132db9e5`) puis a corrigé
le défaut signalé par le refus payé f613 : voir `docs/plan-ca937edf4e-T2.md`,
section « Correction f613 », et son reçu daté
`docs/plans/engine-cursor-contract/evidence/T2-execution/go-test-plan-validation.out`.

Le stash `a73f26244e8e0e59877fb30e8c88a8628b7ad33d`, issu de la base acceptée
`e32388fda21e7c225c3b2416d230d6f5450230cd`, et les rapports partiels d’origine
sont conservés. Codex a examiné puis corrigé une copie indépendante opérateur,
`/tmp/swarm-T2-operator-recovery-20261011-i8yzie71`, HEAD du stash, dirty.
Il s’agit d’une **réparation externe**, pas d’un achèvement autonome du worker.
L’index de preuves identifie cette filiation, les entrées et sorties originales.

## Effet et correction du diagnostic

`planning judge` conserve un verdict global `continue` ou `end`, distinct de
la revue indépendante des candidats. `planning renew` ouvre le cycle suivant
après `continue` sans modifier l’objectif, les exigences, les tentatives,
les compteurs d’activations/décisions ou leurs plafonds. L’historique reste
persisté dans le même corps Work et la même transaction versionnée.

Le rapport natif partiel déclarait son critère 4 PASS tout en admettant que
`end` ne contrôlait que l’identité Git. Ce résultat était insuffisant : un
nouveau test reproduit une clôture sans preuves. La réparation réutilise
`validatePlanningClosure`, extraite du parcours `close` existant : retours
traités, enfants clos, tâches acceptées avec `acceptedFresh`, couverture des
exigences. Un reçu altéré, un avis invalidé ou un processus actif refuse `end`.
Les refus ne modifient ni révision, verdict, historique ni acceptation.
Le juge conserve un verdict ; les contrôles ne produisent pas ce jugement IA.

`global_cycle` est reconstruit depuis l’état durable dans le contexte racine ;
un renouvellement ne transmet pas l’ancien verdict vivant. L’historique complet
reste accessible dans Work/événements, hors de la projection bornée. La réouverture
et le rejeu sont exercés. Le test historiquement cité mais absent,
`TestManagedCheckpointRecovery`, existe maintenant et exerce le parcours CLI
public sur un dépôt géré ; il n’est pas un alias vide. Le contrôle adopté exact
est rejoué, sans changer sa politique ni retirer une obligation.

La qualification des sources utilise un nouveau snapshot d’intégrité,
`docs/engine-cursor-source-manifest.json`. Le manifeste historique demeure intact
et ne devient pas une acceptation des changements. Le moteur ordinaire doit
exécuter ses contrôles et la revue sur son candidat immuable final.

## Preuves locales par critère

| Critère | Résultat | Observable |
| --- | --- | --- |
| 1 — verdict global distinct de la revue | PASS local | État Judge séparé, champs Repository/Reviewer conservés ; test de verdict |
| 2 — renouvellement durable et contexte frais | PASS local | Historique conservé, nouveau contexte après réouverture, CLI géré et rejeu |
| 3 — rejeu, péremption, pause/panne, quota | PASS local | Refus atomiques, cycle/révisions contrôlés, compteurs inchangés ; race |
| 4 — aucune clôture sans preuves fraîches, architecture | PASS local | Reproduction rouge puis refus candidat seul, avis invalidé, reçu altéré, processus actif ; cas accepté avec véritables contrôles/revue doubles ; architecture FR/EN et Mermaid |

Les commandes, codes, sources et sorties sont dans
`docs/plans/engine-cursor-contract/evidence/T2-execution/operator-recovery-20261011/index.json`.
Le parcours ciblé final exécute huit tests nommés (sept juge + checkpoint),
avec les sous-cas explicites. La suite canonique avant ajout du checkpoint
couvre 1 094 cibles, tous les 19 processus exit 0 ; la suite finale incluant
ce test couvre 1 095 cibles, tous les 19 processus exit 0 (reçu séparé). Les skips optionnels sont listés, jamais
comptés comme PASS. Aucun résultat du dernier outil tué n’est présenté comme réussi.
Les essais intermédiaires de réparation et les erreurs de fixture restent
conservés dans le dossier de recherche opérateur ; ils ne constituent pas les
preuves finales ci-dessous.

## Limites et remise

Les tests utilisent des fournisseurs doubles ; aucun jugement ou renouvellement
IA autonome réel n’est démontré. Le schéma du fournisseur structuré ne contient
pas encore `judge`/`renew` : T3/T5 doivent intégrer la conduite avec les primitives
publiques, T7 doit l’exercer avec fournisseur réel. Le réglage `max_judge_cycles`
est persisté à `enable` (défaut explicite 20, borne 500) ; son panneau Admin FR/EN
reste attribué à T5. Ce lot ne livre pas cette UX et ne prétend pas à la conformité
globale Cursor. Janvier et février sont conservés avec leurs différences.

Les preuves locales identifient HEAD du stash et les empreintes de sources dirty.
Elles ne prétendent pas avoir exécuté le commit natif que le contrôleur créera
ensuite. Celui-ci doit refaire ses contrôles et la revue indépendante sur son
propre candidat exact. Une remise complète du périmètre T2 n’est pas son acceptation.

Les originaux `original-plan-ca937edf4e-T2.md`, `original-plan-ca937edf4e-T2.delivery.json` et `original-T2-handoff.md` restent archivés dans le dossier de preuves. Les nouveaux reçus sont distincts.

## Reprise du précontrôle de contexte — 11 octobre 2026

La première remise externe, événement
`T2-duration-interruption-external-repair-20261011`, a créé le résultat
`60bdc8ad5b395b5abc972eb701ff244faed45e69` puis a été bloquée avant revue :
le manifeste d’intégrité formait une pièce de diff indivisible trop grande.
À la révision 114, aucun avis T2 n’a été créé et les appels du vérificateur
restaient à 11/30. Cette remise et ses reçus sont conservés.

La correction de packaging remplace la liste unique par quatre tableaux JSON,
liés au manifeste par leurs SHA-256. Les 1 049 entrées sont conservées ; leur
digest cumulé reste calculé sur l’ensemble trié, sans modifier le manifeste
historique ni prétendre renouveler son acceptation. Le test de qualification
reconstruit la liste complète et refuse les pièces altérées, absentes, ambiguës
ou les chemins invalides. Aucun plafond de contexte ou d’appels n’est augmenté.
La nouvelle remise utilise explicitement `revise-recovered-result` après
vérification du précontrôle impayé conservé ; elle exige des contrôles et une
revue indépendants frais. L’observation d’origine reste dans l’index de preuves.

La suite après découpage couvre 1 096 cibles, les 19 processus sortent à 0 ;
les skips restent explicites. Son reçu `source-parts-full-suite.out` et le
parcours ciblé `source-parts-targeted.out` sont conservés séparément des reçus
antérieurs à 1 094 et 1 095 cibles.
