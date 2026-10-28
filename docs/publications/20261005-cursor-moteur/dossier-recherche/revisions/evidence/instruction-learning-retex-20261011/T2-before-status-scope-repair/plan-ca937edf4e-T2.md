# T2 — juge global, renouvellement et reprise opérateur

Mission `w-ca937edf4e0abd85f8d3d67b`, tâche `plan-ca937edf4e-T2`, agent
`auto-f2abbcfe6bed66fd1a5e`, tentative `a-a349f9134d355713576b7166`.
La production native a été interrompue le 11 octobre 2026 à 08:36:27 UTC,
code 143, après 1 800 secondes, avec 78 appels/78 résultats sur 80 autorisés.
Son statut demeure **interrupted** ; aucun essai ni budget n’est remboursé.

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

## Correction du refus sémantique — chemins équivalents

La revue indépendante réelle `review-4b2233e89effd7e81afc1b5e` a refusé le
candidat `8213f83900ef26d53a993e9c5d9e89eb06d55d36` : le contrôle des doublons
utilisait le chemin brut au lieu du chemin normalisé. `part.json`, `./part.json`
ou `sub/../part.json` pouvaient charger deux fois les mêmes lignes. Le refus est
conservé, sans acceptation ni remboursement des 14 appels déjà consommés sur 30.

La reproduction rouge exerce les deux alias ; la correction indexe `seen` par
`filepath.Clean(part.Path)`. Les cas ciblés et race passent ; `go vet ./...` passe.
La suite canonique du candidat corrigé couvre encore 1 096 cibles, les 19 groupes
sortent à 0 et les skips restent explicites. Les sorties `source-alias-*.out`
et le refus original sont liés par leurs empreintes dans l’index. Le contrôleur
doit encore vérifier puis soumettre ce nouveau candidat à une revue indépendante.
Cette correction opérateur conserve les 1 049 sources, les tentatives et l’historique.

Le contrôle préalable a refusé un contexte de 138 339 octets avant tout appel
de revue : trois suites successives étaient jointes intégralement. Les deux
suites antérieures restent dans les artefacts et l’index, avec leurs empreintes ;
le contexte de la nouvelle revue fournit la suite complète la plus récente,
qui couvre le candidat corrigé. Aucun reçu n’est tronqué, ni plafond augmenté.

## Diagnostic de citation et documentation FR/EN

La revue réelle `review-6eadd171c67897f10f4ec754`, candidat
`d405c0ee9479fb26521fc6041cc6932f9929a3a1`, s’est arrêtée avec une erreur de
protocole : sa citation omet un espace avant le point-virgule de la pièce française.
La réponse brute et la garde stricte restent conservées ; ce n’est pas un verdict
sémantique accepté. Le parcours de reprise peut remettre au vérificateur les
lignes physiques originales et sa citation rejetée, sans réparer son jugement.

Une lecture locale des deux sources confirme cependant une contradiction :
FR constate les valeurs Go actuelles 20/500, tandis qu’EN disait « never a Go
constant ». La correction EN décrit les mêmes faits que FR : paramètre d’appel
persisté, défaut et borne Go encore présents, exposition Admin confiée à T5.
L’original EN est conservé dans les preuves. Le snapshot est recalculé sur la
correction ; aucun code du juge ni budget n’est changé. Ce diagnostic local et
cette préparation ne remplacent pas la nouvelle revue indépendante nécessaire.


Revue indépendante fraîche du 11 octobre, review-dec9eececb2d49ee384f93de sur d405c0ee9479fb26521fc6041cc6932f9929a3a1 : changes_requested valide, citation physique43/42, contradiction FR/EN confirmée. La version EN conserve désormais les valeurs Go explicites actuelles et attribue leur exposition Admin au lot T5 ; la FR reste intacte. Code juge/checkpoint/alias inchangé depuis la suite canonique1096/19groupes. Contrôles ciblés de sources/juge/checkpoint et checker mécanique déjà PASS ; les nouvelles pièces et le rapport ne prétendent aucune acceptation. Refus sémantique original, erreur de protocole, appels et stash restent conservés.
