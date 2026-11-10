# Pilotage de la charge de migration — 22 octobre 2026

## Discipline de coût —23octobre2026

Prochaine unité : parcours historique `mission status --json` utilisable de bout
en bout. Assemblage des producteurs existants et dépendances dans le même lot ;
contrôles ciblés pendant le code, preuves inchangées réutilisées, puis recette
publique/revue/qualification finale consolidées. Affectations indépendantes et
contexte agent réduit. Suivi par capacité utilisateur et temps mesuré ; usage GLM déclaré conservé par campagne ; économie de tokens Codex et
estimations en semaines non validées restent explicites. Droits,
budgets, données et documentation de fonctions restent requis. Aucun gain de
vitesse ni changement du serveur installé n'est encore démontré par cette règle.

## Pilotage adopté — reprise du 22 octobre 2026

Le [classement G1–G12](CHEMIN-CRITIQUE-SPEC.md#décision-applicable--22-octobre-2026)
sépare contrats et réécriture par parcours. Le tableau actif en tête du
[PLAN](PLAN.md) remplace les extrapolations de charge ; les comptes anciens
ci-dessous restent des inventaires historiques par noms.
Lot201 est réduit et non clos ; le parcours suivant est G3/G1, niveau A.
Premier corpus différentiel :2/2 lectures Work exactes sur binaires identifiés194,
source Go précompilée encore non qualifiée. Premier parcours CLI worker/watch et
status natif213 vérifié sur trois fixtures ; reprise queued224 vérifiée sur contexte frais,
avec8 contrôles API et3 régressions W1. Rôles automatiques, registered, web et
JSON historique status restent ouverts. G1–G12 ne sont pas acceptés globalement.
[Préparation R6/R7](evidence/20261022-root-instance-201/cutover-preparation191.md) :
schéma37 accepté, upgrade1..36 non porté ; build/install historiques encore Go.
Migration/restauration/rollback/installation restent à réaliser et vérifier.
[Après bascule](AFTER-CUTOVER.md) conserve les durcissements différés ; les
refus de sécurité et la gestion des erreurs restent dans les contrats requis.

W3 : [244](evidence/20261022-g3-watch-planning/PLAN.md) vérifie watch planner→worker,
16+12 gardes et3 Roots, régressions W1/W2 sur5 Roots, refus/reprise de copie SQLite. Revue indépendante suivie
dans le lot ; async, reviewer/validation, registered et web ouverts. Aucun groupe
global clos ;25 tests231 et statiques233 ne donnent pas un pourcentage de charge.


## Raccord W4 vérifié — 23 octobre 2026

Le [bilan W4](evidence/20261023-g1-status-contract/PLAN.md) conserve la portée :
lecteurs/projections, correction de sept lecteurs de preuves, puis résultats
ordinaires sans historique Agent. Quatre nouveaux tests API290 passent ; profil
normal292 et SDK298 sans fixture,212 entrées moteur identiques ; recette299 :
13 gardes et11 lectures/refus sans mutation, preuve périmée + vraie sortie
ENOSPC + restauration, revue indépendante favorable et statiques291/300 PASS.
Le JSON CLI complet, rapports/revue/readiness avec Agent, runtime/version et les
autres parcours restent ouverts. Aucun des18 livrables globaux ni G1–G12 n'est
clos par ces preuves ; le service installé est inchangé.

## Pilotage courant vérifié — 22 octobre, après lot193

[Registre chiffré et sources](evidence/20261022-pilotage-194/status.json). Correction du suivi : le titre courant du PLAN était resté au lot180, le tableau de charge au181. L’historique ci-dessous est conservé ; ses états anciens ne sont plus le statut courant.

**11 lots récents183–193 documentés et vérifiés dans leur périmètre ;18 livrables restants identifiés, tous ouverts.** Ce sont des unités de suivi inégales, pas une fraction de charge ni le décompte de toutes les actions depuis le début. Les neuf critères RM n’ont pas d’acceptation globale démontrée. Le serveur installé n’a pas été remplacé par ces lots.

| Lot | Action vérifiée dans le périmètre du bilan | Preuve |
|---|---|---|
| 183 | Lancement automatique sous consentement et profil de mission | [bilan](evidence/20261022-183/handoff47/result.json) |
| 184 | Cycle natif et phase planner | [bilan](evidence/20261022-184/handoff25/result.json) |
| 185 | Diagnostics de cycle et conservation des causes | [bilan](evidence/20261022-185/handoff07/result.json) |
| 186 | Coûts déclarés des workers et caches régénérables exclus | [bilan](evidence/20261022-186/handoff23/result.json) |
| 187 | Garde de coût configurable consommée à l’admission | [bilan](evidence/20261022-187/handoff20/result.json) |
| 188 | Niveau du profil manuel et traductions réellement embarquées | [bilan](evidence/20261022-188/handoff28/result.json) |
| 189 | Profil effectif par tâche et liaison v2 parent/effectif | [bilan](evidence/20261022-189/handoff26/result.json) |
| 190 | Intention v1 créée par ancien Rust exécutée par nouveau Rust | [bilan](evidence/20261022-190/handoff13/result.json) |
| 191 | Ordre complet priorité/profondeur/identifiant, lecture seule | [bilan](evidence/20261022-191/handoff08/result.json) |
| 192 | Écriture auditée des priorités, rejeu et rollback SQL | [bilan](evidence/20261022-192/handoff19/result.json) |
| 193 | Frontières de workspace : refus sans mutation et reprise | [bilan](evidence/20261022-193/handoff12/result.json) |

**Inventaire public recompté sur les sources actuelles**, référence Go553593a (ne fige pas les changements Go non committés) :

| Surface historique | Total | Entrée même nom/chemin présente | Absente du dispatcher inspecté |
|---|---:|---:|---:|
| Noms CLI, alias inclus |44|17|27|
| Chemins HTTP centraux |93|4|89|

Présence ne prouve aucune parité de sous-commandes/méthodes/effets. Absence au même nom ne prouve pas l’absence de tout composant interne ou remplacement. Les listes exhaustives de ces entrées et leurs SHA sont dans le registre. Ce ne sont pas93 tests réseau. Les délégations de ressources statiques et les deux routes OTEL hors référence sont distinctes ; les sources Live courantes ne sont pas le candidat figé193.

| ID | Livrable restant — propriétaire Codex | Critère de clôture | Dépend de |
|---|---|---|---|
| P01 | Figer la référence complète, y compris changements Go non committés et ressources | Référence approuvée et inventaire exact des comportements ; RM-01 | — |
| P02 | Rapprocher commandes, sous-commandes et remplacements de noms | Chaque comportement CLI relié à un consommateur et à une preuve ou un manque | P01 |
| P03 | Raccorder les méthodes/contrats des API historiques | Parité des corps, refus, effets, erreurs et reprises, chemin par chemin | P01 |
| P04 | Implémenter la sélection des tâches réellement admissibles | Sélection sous dépendances, fraîcheur, profil, consentement, capacité et coût | — |
| P05 | Implémenter la FIFO globale persistante des workspaces | Concurrence intermissions, équité, acquisition/libération et reprise vérifiées | — |
| P06 | Raccorder dispatch et boucle mission watch | Départ réel, résultat partiel, annulation et redémarrage sans double départ | P04, P05 |
| P07 | Porter les quatre choix opérateur de reprise/relais/horloge restants | Politiques persistées, validation commune et Admin/API/CLI → consommateur | P06 |
| P08 | Compléter les branches de revue/correction et leur publication dans le conducteur | Cas non couverts et effets connus traversent le parcours public de bout en bout | P06 |
| P09 | Porter le cycle Git géré, end et fragments/orphelins | Création, candidat, revue, intégration et reprise sans perte de preuve | — |
| P10 | Compléter mémoire/autorités/confinement dans les branches de lancement restantes | Contrats protégés vérifiés sans transformer les refus non portés en succès | — |
| P11 | Exposer les priorités de tâche dans HTTP/UX | Sauvegarde, réouverture et valeur effectivement utilisée par la sélection | P03, P04 |
| P12 | Normaliser les données Work JSON/BLOB et leur migration Rust | Copie de données existantes, historique et contraintes conservés ; corruption refusée | P01 |
| P13 | Qualifier sauvegarde, restauration et compatibilité de retour arrière | Reprise vérifiée sans perte d’historique ni reset de consommation | P12 |
| P14 | Intégrer les changements concurrents Live et qualifier les parcours navigateur | Binaire commun, ressources servies, FR/EN, thèmes, clavier et erreurs vérifiés | P03 |
| P15 | Établir la qualification globale et la revue requise | Matrice complète, recettes publiques et revue ; aucun contrôle requis ouvert | P02, P03, P06, P07, P08, P09, P10, P11, P13, P14 |
| P16 | Effectuer la bascule Rust seule par le chemin supporté | Version/binaire actif, ressources et notes FR/EN vérifiés sous autorité de livraison | P15 |
| P17 | Exécuter le retour arrière de recette et le smoke de livraison | Effet de restauration/bascule observable, preuves et données préservées | P13, P16 |
| P18 | Clore le RETEX final et transmettre les limites/apprentissages | Bilan réalisé/vérifié/accepté, incidents et apprentissages persistants | P17 |

P01/P02/P03 sont aussi les manques de pilotage à résoudre, pas une raison d’attendre pour travailler sur P04/P05. Ce découpage ne prouve pas que chaque sous-comportement est recensé : la matrice complète reste ouverte et aucune durée globale n’est publiée sans elle. Revue du présent incrément : documentation/mesure uniquement ; aucun choix opérateur, code produit, binaire ou version installée changé. Les erreurs de collecte restent des échecs. Réutilisation du collecteur archivé sans l’altérer ; lecture Git/Python seulement, pas de Go exécuté.

---

Propriétaire : Codex. État : diagnostic de pilotage établi, estimation de charge ouverte.
Référence historique Go : `553593a2ffa3823d16784a6fedc291eebf390837` ; **390 fichiers applicatifs de production et 442 de tests** (racine, cmd, internal). Les 927 fichiers Go du dépôt comprennent aussi 95 copies historiques sous docs : 72 de production et 23 de tests. Le précédent total462 de production incluait donc ces copies et ne doit pas servir de dénominateur applicatif. Ce commit ne couvre pas à lui seul les changements non committés de référence requis par RM-01.

## Défaut et décision

Les estimations conversationnelles « 5 à 10 jours » sont retirées : aucun découpage chiffré ni débit mesuré ne les fondait. Ne pas les réutiliser comme engagement. Le nombre de lots, fichiers ou tests ne mesure pas une charge pondérée. Les suppressions Go de l'index ne prouvent pas la migration.

Cause observée : R0 conserve explicitement « référence finale non figée » ; la table R0–R7 est historique, tandis que les preuves ultérieures sont ajoutées chronologiquement. Il manque une matrice courante des comportements et de leurs preuves. Les améliorations supplémentaires et les réparations de notre méthode ont aussi été mélangées au coût de parité. Ne pas inventer leur durée respective.

Options : extrapoler les lots/tests (rejeté, unités non comparables), ou reconstruire la couverture comportementale contre une référence figée (retenu). Conserver les preuves antérieures ; séparer parité obligatoire, corrections bloquantes et améliorations supplémentaires. Cette décision ne supprime aucun critère ni contrôle requis.

## État consolidé minimal, sans pourcentage de charge

| Périmètre | Réalisé ou preuve disponible | Reste avant clôture |
|---|---|---|
| R0 référence | Besoin et RM-01..09 définis | Référence complète et matrice comportementale à figer |
| R1 contrats | Contrats d'identité qualifiés par périmètre | Exhaustivité des autres contrats à rapprocher de la référence |
| R2 données | Primitives SQL, transactions et formats qualifiés localement | Migration des données moteur, dont Work JSON/BLOB, restauration et corruptions |
| R3 moteur | Transitions, workers, planner et contrôles : preuves locales dans PLAN | Conduite complète, reviewer de production, publication et reprise, autres branches |
| R4 processus et sécurité | Primitives Linux et portes vérifiées par périmètre | Cycle complet, révocation et couverture des autorités/mémoire/confinement |
| R5 CLI/API/UX | CLI et surfaces natives partielles ; voir inventaire API ci-dessous | Parité exhaustive des commandes, routes et parcours navigateur |
| R6 qualification globale | Recettes de composants réutilisables selon leurs entrées | Recette globale et revue non closes ; ne pas confondre avec absence de tout test |
| R7 bascule | Préparation et binaires locaux distincts | Livraison Rust seule, installation autorisée, retour arrière et RETEX |

Ces huit lignes sont des domaines inégalement volumineux, pas huit unités de charge. Aucun des neuf critères RM n'a ici une acceptation globale démontrée ; cela ne signifie pas que le travail réalisé vaut zéro.

## Témoin de surface vérifié à la lecture

`swarm-web/src/service.rs::route` déclare 18 chemins API distincts ; les autres chemins de ce dispatcher refusent `web-route-unported`. Ce compteur mesure des chemins déclarés, pas leur parité ni leurs méthodes/branches ; le transport live/events possède son traitement dédié. Il ne constitue donc pas un pourcentage.
SHA-256 de la source lue : `fef0b3d578ca4ad32fc73f4d452a8708b21ad6c58db20a8ba28eabee7cd6667e`.

- `/api/v1/action`
- `/api/v1/admin-advice-budget`
- `/api/v1/admin-advice-budget/apply`
- `/api/v1/admin-advice-budget/preview`
- `/api/v1/live/agent`
- `/api/v1/live/events`
- `/api/v1/live/fingerprint`
- `/api/v1/live/overview`
- `/api/v1/native-capabilities`
- `/api/v1/native-session`
- `/api/v1/native-settings`
- `/api/v1/native-settings/apply`
- `/api/v1/native-settings/describe`
- `/api/v1/native-settings/preview`
- `/api/v1/quotas`
- `/api/v1/quotas/apply`
- `/api/v1/quotas/preview`
- `/api/v1/works`

## Premier rapprochement reproductible des interfaces publiques

[Inventaire source lié aux SHA](evidence/20261021-pilotage/inventory.json), [recette de collecte](evidence/20261021-pilotage/inventory_recipe.py), [28 refus exécutés et vérifiés](evidence/20261021-pilotage/cli-missing-replay.json), [recette de rejeu](evidence/20261021-pilotage/probe_recipe.py).

| Surface de référence | Dénominateur | Entrée Rust au même nom/chemin | Entrée absente au même nom/chemin |
|---|---:|---:|---:|
| CLI : noms de premier niveau, alias inclus | 44 | 16 | 28 |
| HTTP : chemins enregistrés, boucles et constantes résolues | 93 | 3 | 90 |

**Présence ne veut pas dire parité réalisée, vérifiée ou acceptée.** Sous-commandes, méthodes, corps JSON, effets, erreurs et reprises ne sont pas comptés ici. Les remplacements sous d'autres noms restent à rapprocher : ne pas traiter ces absences comme autant de fonctionnalités internes entièrement inexistantes. Les sept noms CLI Rust supplémentaires et quinze chemins API supplémentaires sont inventoriés dans le JSON, sans les compter comme port des anciennes interfaces. Les 16 noms CLI ont encore une couverture partielle. Les trois chemins communs sont /api/v1/action, /api/v1/quotas et /api/v1/works.

Les 28 noms CLI absents ont réellement répondu code2 / command_failed avec le diagnostic « Commande non encore portée dans ce binaire Rust de développement. », stdout vide et fixture inchangée. Binaire figé SHA2afec2de… du profil normal149/profile05-8 : pas une reconstruction du checkout150 ni un test des variantes valides des commandes présentes. Les compteurs HTTP sont une lecture de sources, pas 93 tests réseau.

Collecte initiale85 : elle ne prenait que les bases littérales. Collecte complète93 : cinq boucles développées (quatre preview/apply, une files/source/budget) et deux constantes résolues, dont automationExternalPath. Le collecteur a d'abord refusé cette dernière au lieu de l'omettre ; [diagnostic conservé](evidence/20261021-pilotage/collection-initial-refusal.txt), correction générale de résolution des constantes, collecte suivante réussie. Une expression future inconnue reste un échec, pas une route disparue silencieusement. Le scanner reste borné aux syntaxes relues, sans garantie générale de complétude d'un parseur Go.

### Noms CLI présents par périmètre

`agent`, `budget`, `checkpoint`, `evaluate`, `gate`, `init`, `ooda`, `planning`, `pricing`, `providers`, `quotas`, `resume`, `task`, `task-model`, `web`, `work`.

### Noms CLI encore absents sous le même nom

`aide`, `automation`, `autonomy`, `connections`, `console`, `control`, `dispatch`, `doctor`, `evidence`, `exchange`, `export`, `help`, `import`, `laboratory`, `lifecycle`, `memory`, `mission`, `plan`, `prepare`, `process`, `profile`, `project-profile`, `role-model`, `run-limits`, `skills`, `trust`, `validation`, `version`.

La migration ne peut donc pas être présentée comme une simple finition : la couche publique demeure largement à raccorder, en plus du reviewer de production, des données et de la bascule. Ces compteurs ne fournissent pas un nombre de jours ou un pourcentage de charge totale ; les adaptations de noms et les composants internes réutilisables doivent être évalués avant de pondérer.

### Rejeu

Depuis la racine du dépôt :

```sh
python3 docs/plans/migration-rust/evidence/20261021-pilotage/inventory_recipe.py --reference 553593a2ffa3823d16784a6fedc291eebf390837 --output /tmp/swarm-migration-inventory.json
python3 docs/plans/migration-rust/evidence/20261021-pilotage/probe_recipe.py --inventory /tmp/swarm-migration-inventory.json --binary .cache/rust-migration-binaries/swarm-rust-2afec2de178259c1c5ddfb1e6a7f3bfd08b4014ccdba64fe92a991134932032e --binary-sha256 2afec2de178259c1c5ddfb1e6a7f3bfd08b4014ccdba64fe92a991134932032e --output /tmp/swarm-migration-cli-refusals.json
```

Le binaire doit être disponible et correspondre à la preuve de profil ; son absence est une précondition manquante. La collecte relit les sources Rust actuelles et les lie à leurs SHA ; le rejeu observe seulement le binaire figé. Ne pas comparer des sources changées et un ancien profil comme une qualification courante.

## Condition d'une estimation défendable

Inventorier chaque comportement de la référence Go (commandes/sous-commandes, API/méthodes, parcours UI, données et reprises), associer son consommateur Rust et une preuve qualifiée ou un manque explicite. Décomposer les manques en livrables vérifiables et dépendances ; estimer des fourchettes d'effort avec hypothèses et incertitude, puis les recalibrer avec le débit réellement observé. Les temps historiques et le coût abonnement ne sont pas reconstruits ici. Propriétaire Codex, état ouvert ; aucun nouveau délai ni pourcentage annoncé avant cette base. Le document CHEMIN-CRITIQUE-SPEC reste une proposition de préparation, pas une matrice déjà validée.

## Actualisation du 22 octobre — lots164 à181

La collecte de surface du 21 octobre ci-dessous reste une preuve historique, pas un inventaire actuel : `mission` a depuis été partiellement porté, ainsi que la route HTTP de planification. Elle ne fournit toujours pas de pourcentage de charge.

| État | Avancement constaté | Limite restante |
|---|---|---|
| Vérifié localement | Chaînes worker → contrôles natifs → reviewer → décision du superviseur, reprise et clôture sur fixtures | Pas de preuve de fonctionnement autonome complet en production |
| Vérifié localement | Juge global, renouvellement, quotas, historique et protections des sous-scopes | Ne constitue pas le conducteur automatique complet |
| Vérifié localement | HTTP de planification, pause/levée/arrêt de mission, révocation d'un vrai processus planner possédé, erreurs de sortie après commit | Commandes de mission start/resume/watch et HTTP global restant à porter ; pas de preuve sur tout processus hostile |
| Intégration181 en cours | Profil natif sans feature de fixture, 47 tests Rust ciblés, Clippy/Rustdoc et quatre recettes publiques passent sur le candidat figé ; changements partagés télémétrie/ressources inclus | Recette supplémentaire pause/reviewer10 réussie après correction de ses préconditions ; aucune acceptation des parcours UX de Claude ni installation déduite |
| Ouvert | Parité du conducteur, profils/consentement et parcours CLI/API/navigateur ; espaces Git gérés et reprises restantes | Conserver une matrice comportementale et des preuves de chaque branche |
| Ouvert | Migration des données Work/JSON/BLOB, restauration cohérente et corruptions ; qualification globale | Les primitives déjà testées ne clôturent pas ces contrats |
| Ouvert | Livraison Rust seule, version installée, bascule et retour arrière, puis RETEX | Pas de livraison déclarée à partir du seul binaire de recette |

Preuves détaillées : PLAN.md, lots164–181 et `evidence/20261022-181/`. Il reste un travail conséquent d'intégration et de parité ; aucune durée ou fraction globale fiable n'est établie. Propriétaire : Codex pour migration et qualification ; les modifications concurrentes de Claude sont préservées.

## Dernière preuve locale distincte

150-A : [source01](evidence/20261021-150/source01/result.json), PASS enveloppe et deux étapes, sans Go observé et sans watchdog global. Observation courante des sources séparée du TTL d'admission ; ne prouve ni contrôleur reviewer livré ni migration complète.

Aucun comportement produit modifié : recettes de mesure ajoutées avec docstrings, exécution et preuves ; aucun build/install ni version de serveur modifié. Les notes produit/version_history restent inchangées pour cette raison. Revue politiques : aucun seuil opérateur introduit. Revue erreurs : aucune erreur d'exécution transformée en succès ; inconnues d'estimation conservées ouvertes. Vérification : liens locaux et git diff --check ; les recettes de mesure ont été exécutées ; aucune nouvelle recette moteur requise pour ce suivi. Pas d'installation, de commit ou de publication.

## Vérification exécutée des27 noms absents — pilotage194

Les27 refus ont été exécutés sur normal193 (même SHA de lib.rs que la collecte),
code2, stdout vide, code structuré command_not_ported, fixture inchangée et
binaire stable : [résultat](evidence/20261022-pilotage-194/cli-refusals-current.json).
Le rejeu historique attendait command_failed et a échoué : original conservé
dans cli-refusals.json, aucun résultat réécrit. Le contrat actuel est explicite
dans CliError::code. L'outil pérenne tools/rust-migration/probe_cli_surface.py
exige --expected-code, --binary-sha256 et une sortie fraîche ; aucune liste de
codes alternatifs admis ni fallback. Cette recette ne vérifie que les refus de
noms absents, jamais la parité positive des17 noms présents. Sources web actuelles
différentes de normal193 : aucun test HTTP actuel déduit du profil193.

## Matrice195

La [matrice détaillée](MATRICE-COMPORTEMENTS.md) relie les137 entrées et45 opérations mission/planning à leurs consommateurs/écarts. Les137 entrées et45 opérations ne sont pas des unités additives. Mission11 :4 contrats différents,1 vue renommée partielle,5 opérations absentes,1 watch explicitement non porté. Planning34 :9 entrées présentes,25 non raccordées ; aucune parité positive déduite. Trois gardes de collecte et recette pérenne PASS, intégrité VERIFIED. P01–P03 restent ouverts pour les autres comportements.

## Actualisation196 : livraisons parallèles vérifiées

Trois agents autorisés, fichiers distincts, intégration racine :38 opérations CLI supplémentaires (19même nom/5renommées partielles/14absentes),93 contrats HTTP avec27 sources, et noyau queue lu par vrai Root/CLI. La [matrice](MATRICE-COMPORTEMENTS.md) décrit83 opérations sur10 familles, plus137 entrées publiques non additives. Aucun des18 livrables globaux déclaré clos : P02/P03 approfondis et P05 première observation seulement.

P01 source15 :398 fichiersGo production et487tests,71modifiés/53ajoutés par rapport au commit ; source01 conservée, défaut de collecte d’ajout de nom reproduit13/corrigé14 et reprise15. Assets/graphe/référence approuvée toujours ouverts. Normal196 non installé : queue10 deux étapesPASS, qualité06 six testsRust/Clippy/Rustdoc. Inventory09 trois étapesPASS/13testsPython. Capture16 deux étapesPASS/une garde ajout/refus/reprise. Preuves dans evidence/20261022-parallel-196/.


## Actualisation197 : FIFO native appliquée, preuve locale bornée

Deux agents autorisés ont livré transitions de file, sélection des candidats et
revues bornées ; la racine a intégré vrai bail/consentement/readiness/cycle Agent.
Normal39 :292 entrées effectives par défaut, sans fixture, non installé.
final43–46 (quatre recettes/six étapes) PASS ; quality21 conserve17 tests Rust nommés,
Clippy strict et Rustdoc privé. Intégrité finale vérifiée ; SDK40/41 liés au normal39.
P05 couvre désormais inscription debout, acquisition Binding3, libération terminale
sûre et historique des générations. Le défaut de nettoyage après pause est reproduit
avant correction. P05 reste partiel : toutes variantes de permission, reprise
incertaine/crash et acceptation complète d’équité ne sont pas établies. P04/P06
dispatch/watch restent non raccordés ; ces API SDK n’ajoutent aucune opération CLI
ou parité HTTP. Les inventaires137/83/93 restent des niveaux de référence qui se
recouvrent, jamais un pourcentage d’achèvement. Les pannes cleanup/annulation, owner/réservations exacts, progression même
workspace et exécution historique v2 passent. Inconnus/crash/panne de completion,
rejeu terminal après nouvelle génération de même tâche et4 permissions restent ouverts.
Sources et preuves : 20261022-fifo-197 dans le plan.

Scellé197 : normal65 SHA8e5af7aa /292 entrées effectives par défaut, SDK66/67/68,
quality69 18 tests Rust nommés/Clippy strict/Rustdoc privé. Cinq recettes finales
(completion-after70 et final71–74) passent7 étapes, intégrité vérifiée. Pause et
preuve de release terminale absente ont leurs témoins avant/après ; la régression
autocommit60 est diagnostiquée/corrigée, jamais dispensée. Sorties imbriquées
archivées/vérifiées séparément dans retained76. Retry de production61 PASS sur
normal55 pour le code d’observation inchangé. Aucune acceptation P05/globale,
installation, parité HTTP supplémentaire ou conducteur/dispatch/watch complet.

### Point198 — P04 progresse, P06 en préparation

Normal24 ab349179/293sources effectives sans fixture, SDK26 aux200mêmes sources moteur ;20tests nommés10 réutilisés sur corps identiques, Clippy/Rustdoc27, Clippy SDK27, recette publique28 (1étape PASS/integrity) ;4comportements et3conflits originaux vérifiés,116sorties imbriquées conservées29. P04 reste partiel ; aucun des18livrables globalement clos, ni pourcentage/charge déduit de ces compteurs. Nouvelle suite199 : preuve worker existant indépendant du conducteur après entrée réelle fournisseur inerte, puis raccordement watch restant à implémenter. Pas de service installé, modèle payé ou qualification combinée des23sources concurrentes exclues.

### Point199 — prérequis d'indépendance vérifié, watch produit ouvert

Public23 : deux étapes PASS/intégrité VERIFIED, douze gardes et deux parcours
inertes réels (sortie gracieuse puis SIGTERM du conducteur). Même worker survivant,
vrai waitpid CLI, originaux de fin liés au résultat public, réservation réglée et
FIFO libérée, rejeu sans nouveau provider ni acceptation. SDK10 default qualifié
sur les mêmes 200 entrées moteur de normal198/24 ; trois tests Rust08 et Clippy11
PASS. Retained24 conserve 115 sorties sans base Root. P06 progresse par ce
prérequis B1 ; aucun des 18 livrables globaux n'est déclaré clos. B2 reste à
raccorder, notamment une route session sûre compatible avec forbid unsafe.
L'analyse P07 conserve quatre axes manquants distincts : budget de chaîne,
éligibilité temporelle/multiplicateur, marge d'horloge rapport, relais
nombre/cadence ; le seuil de coût est déjà porté. Aucune nouvelle politique
opérateur, installation ou qualification combinée Live déduite.

### Point200 — bootstrap produit sûr vérifié, conducteur encore ouvert

Normal05 c734f0c4 : 294 inputs default sans fixture ; SDK10 aux mêmes 200 sources
moteur. Cinq tests CLI06, trois SDK11, Clippy/Rustdoc12 PASS. Public14 : deux étapes
PASS et intégrité VERIFIED, quatorze gardes, quatre refus publics avant Root puis
gracieux/SIGTERM réels via le helper produit sans unsafe. Retained15 : 129 sorties
avec originaux de fin et statuts noyau. Le code du driver watch et ses canaux
Root-bound ne sont pas livrés par ce prérequis ; aucun P01–P18 clos ni pourcentage
inféré. Vérification du remplacement de .swarm à chemin constant requise avant
raccordement, hypothèse non reproduite au présent bilan. Pas d'installation,
qualification combinée Live ou acceptation globale.


## Point201 — instance Root, refus historiques et reprise mesurés

Normal65 `8bd7ebc0` (301 entrées effectives, default sans fixture) et SDK77
partagent 201 entrées moteur et la config Cargo. Les 67 tests Rust nommés,
Clippy/Rustdoc85, injections publiques répertoire88/DB seule89, cycle normal/
SIGTERM98 et compatibilité v1/103–v2/104 passent dans leurs portées. Les recettes
ont leur vérification d’intégrité et rétention imbriquée90/99/105 ; aucune base
utilisateur ni modèle payé. Les compteurs ne closent aucun des 18 livrables.

Sous verrou, le refus CLI92 expose Prepare/SQLite5 en 4440712ns, avec reprise et
SQL/DB/settings inchangés ; l’ancien61 mesurait5017838602ns. Aucun seuil ni
engagement de performance ; l’ordre du handler zéro et la conservation du réglage
persisté sont testés74. L’initialisation de témoin et les anciens producteurs
v1/v2 sont explicites. Leur nouveau grant refuse sans mutation ; lecture,
annulation/reconcile/replay, fermeture du pair et réouverture du terminal annulé
sont vérifiés. Binding3 et un ancien completed restent des critères distincts.

La revue96 couvre 62 fichiers avec106 axes ouverts ; elle ne clôt ni les ajouts
Live de Claude, ni le timeout zéro durable de TelemetryStore, ni la continuité
planning/reviewer. Cette dernière et le conducteur produit sont les suites
concrètes (plans100/101). Les binaires sont non installés ; le transport de
binding en développement n’est pas qualifié par normal65. Baseline199 conservée.

Preuves : [synthèse fonctionnelle94](evidence/20261022-root-instance-201/functional-handoff94/result.json), [revue96](evidence/20261022-root-instance-201/review96/README.md).


Actualisation201, preuves118–125 : la compatibilité historique v3 est maintenant
vérifiée par `legacy-v3-118` (12 gardes et parcours réel), avec 169 sorties
conservées dans `legacy-v3-retained119`. Cela complète les preuves v1/v2 ; cela
ne vérifie ni ancien completed, ni replay public `mission.start` v3, ni nouvelle
admission. Le composant d'identité par descripteurs passe six tests116 mais reste
réservé aux tests Linux, sans raccord aux rôles. Après quatre lints de tests
conservés en FAIL121, les corrections122/123 et le gel124 passent Clippy strict125
(lib/tests engine, sans exécuter le reproducteur reviewer). Ce dernier reste
non exécuté et bloqué par l'outil115. `continuation-plan120.md` est adopté et en
implémentation pour observer les intentions avant dispatch ; pas encore qualifié.
Ces résultats n'avancent pas la baseline199 et ne clôturent aucun lot global par
simple décompte de tests. Les sources/profils65 et124 restent distincts.

### Campagne361–379

Deux résultats vérifiés sur le parcours ciblé : résultat soumis hors fenêtre
et démarrage worker sans écriture parent pendant helper.6appels GLM bornés,
dont1sortie tronquée conservée ; usage réellement déclaré et durées dans
[evidence/20261023-g1-status-contract/final379/campaign-usage.json](evidence/20261023-g1-status-contract/final379/campaign-usage.json).
Aucun chiffre d’économie Codex ni gain de performance inventé. Un scope public
ciblé est maintenant rejouable par manifest378 ; la recette complète G1 ne
repasse pas à chaque campagne. Les échecs de fixture et le coût des vérifications
restent visibles dans le plan, pas assimilés à du portage fonctionnel.

23octobre, décision d’abandon sans Agent : parcours CLI abandon→consultation
nonvalidée→réouverture vérifié388 sur normal385.25lecteurs, Clippy/Rustdoc382 PASS ;
aucun fournisseur ni nouvelle recette complète G1. Reprise386 due à l’attente du
harnais (histoire/révision conservées), pas à un défaut moteur ; original gardé.
Pas de nouveau pourcentage global ni d’économie nette de tokens démontrée.

## G1 actifs ordinaires —400
Une capacité supplémentaire vérifiée : consultation JSON d’un Agent queued/incertain puis d’un vrai superviseur local actif,8 lectures/refus et parcours jusqu’à soumission, sans validation implicite.27 tests lecteurs et statiques396, profil398, recette400 PASS. Trois appels GLM complets (proposition390, revues393/397),51 531 tokens déclarés ; économie nette Codex non mesurée. Coût de reprise visible :392/394 même assertion terminale (édition partielle après rustfmt),395 lint closure identité,399 mauvaise précondition ready/running ; originaux conservés et causes corrigées. Ces résultats ne ferment ni G1 ni les autres groupes et n’établissent aucun délai global.

## G1 résultat accepté —411
Capacité vérifiée : consultation des preuves acceptées actuelles/périmées/restaurées, sans réécriture de la décision et avec dépendances réellement projetées ;8 lectures/refus sur CLI normale407.28 lecteurs406 + Clippy/Rustdoc,407 Rustdoc PASS ; deux appels GLM402/405,17 370 tokens déclarés. Aucun taux global ni gain net Codex déduit. Coût de reprise conservé :404 assertion d’ancien périmètre,408 profil todo non porté,409 mauvais nom wire,410 publisher fresh utilisé à tort pour remplacer un rapport.411 corrige le harnais et passe ; aucune protection moteur retirée. Prochaine limite identifiée : disponibilité publique des tâches todo avec profil de lancement sauvegardé.

## G1 profils manuels/pause —418
La limite de408 est levée pour les branches manuelles/assistées et pause : la même recette maintenue couvre maintenant un vrai todo profilé,10 lectures/refus publics.29 lecteurs414 + Clippy/Rustdoc, normal416 qualifié ; candidat non installé. Pas de nouveau script ni manifeste. GLM413 proposition exploitable ;415 revue tronquée (2048 raisonnement sans réponse) conservée,417 réduit au helper sans relever seulement le budget. Aucun gain net Codex estimé. Automatic readiness, planning/revues et G1 global restent ouverts.

GLM417 termine avec642tokens de sortie dont274de texte final, aucun défaut concret identifié dans le helper. Total campagne413/415/417 :19 563tokens déclarés, dont11885pour l’appel415 inexploitable. Réduction du contexte attestée, aucune économie nette ni estimation globale inférée.

## G1 planning initial —424
Une recette existante étendue, sans nouveaux fichiers de test :19 lectures/refus contre10 auparavant. Planning configuré avant activité, autorisation distincte du conducteur actif et deux pauses/reprises vérifiés.30 lecteurs, Clippy/Rustdoc et normal423 qualifié. RED public420 et erreur de nom de champ421 conservés ; aucun retest global ni fournisseur payant. Planning avec tâches/activations, readiness automatique et qualification G1 complète restent ouverts ; pas de pourcentage ou de délai global déduit de ce sous-parcours.

## G1 attente fournisseur —430
La recette existante passe de19 à26 lectures/refus : projection Go des attentes initiales, invalides refusés et restauration ; pas de nouvel outil de recette.30 lecteurs et statiques428, profil normal429 ;323 entrées archivées. GLM427 :1518 tokens déclarés, aucun gain net Codex ni coût monétaire déduit. Planning actif, automatic readiness et G1 global restent ouverts ; ce sous-parcours ne donne ni pourcentage ni délai global.

## G1 planning sans tâche après activation —437
Un parcours utilisateur supplémentaire vérifié :claim/renew/décisions vides/pause/reprise/plafond sauvegardé ;38 lectures/refus purs et refus de la quatrième activation.30 lecteurs434 et statiques PASS, normal435 sans fixture, candidat non installé. GLM432 :3462 tokens déclarés, avis partiellement corrigé ; aucune économie nette Codex mesurée.433 RED et436 attente de motif interne au lieu du code public conservés, reprise437 PASS. G1 et migration complète restent ouverts ; ces compteurs ne donnent ni pourcentage ni durée globale.

## G1 premières tâches planning —444 / campagne cinq GLM440
Cinq appels GLM simultanés prouvés (chevauchement9,9s), cinq réponses complètes,20607 tokens déclarés ; appel isolé439 précédent4752 tokens distinct, non compté comme campagne cinq. Avis intégrés après correction ou explicitement écartés dans wave440/integration.md. Un parcours supplémentaire vérifié : deux tâches publiques, configure/dépendance en attente/pause/reprise ;43 lectures/refus purs,30 lecteurs442, Clippy/Rustdoc, normal443 sans fixture, non installé. Aucun gain net Codex ni délai/pourcentage global déduit. G1 reste ouvert. Préférence de vague cinq conservée dans PLAN, avec consommation bornée par les tâches autorisées.

## G1 délégation —450 / cinq GLM446
Cinq appels simultanés prouvés, cinq réponses complètes,21066 tokens déclarés. Une capacité vérifiée en plus : délégation publique et statut de tâches enfant,50 lectures/refus purs (dont43 antérieurs),31 lecteurs448, Clippy/Rustdoc et normal449 sans fixture. Proposition de changement de clôture écartée, pas de gain net Codex mesuré. Candidat non installé ; G1 ouvert, aucun pourcentage/délai global déduit.

## G1 décisions initiales —458 / cinq GLM452
Cinq appels initiaux simultanés : quatre complets, recette incomplète4455 tokens (reasoning seul). Reprise ciblée485 tokens sans hausse de sortie ; total six appels,21090 tokens déclarés. Pas d’échec effacé ni de gain net Codex démontré. Progression réelle : nouveau writer scoped initial dans transaction Root, statut abandon/reprise root/enfant,54 lectures/refus purs +6 refus et2 rejeux.31 lecteurs455+9 lifecycle existants, normal456 sans fixture, non installé. Coût de méthode visible : hypothèse writer porté invalidée453, garde native ajoutée après revue454, attente de rejeu corrigée457. G1 ouvert ; aucun pourcentage/délai global déduit.

### Raccord automatic462–472

Une fonction produit raccordée et son parcours public, pas8tâches migrées.
8GLM complets/20 886tokens déclarés ; Root filtre les propositions, intègre et vérifie.
44lecteurs ciblés,qualification normale468,60lectures/refus publics470.471 change
seulement le nom d’un résultat pour décrire le planning réellement exercé. Aucune
économie Codex nette, durée restante ou clôture globale déduite de ces nombres.

### Vague473 — complément de recette
8appels simultanés,7complets/1tronqué sans reprise ;61276tokens déclarés. Aucune fonction produit nouvelle ;6observations publiques enfant ajoutées,66au total sur même normal468. Coût de contexte partagé excessif conservé dans wave473/integration.md ; prochaines tâches par extraits propres au rôle. Aucune économie nette ou durée restante déduite.

### Raccord477–484
Huit GLM simultanés,7réponses complètes/1tronquée sans retry,20 008tokens déclarés. Extraits par rôle contre contexte commun473 ; tâches différentes donc pas d’économie nette Codex mesurée. Un parcours produit réellement raccordé (rapport soumis d’un worker planifié),46lecteurs et73observations publiques. Contexte result-review vide payé/inutilisable et fixture480 invalidée sont conservés ; précondition d’ancres/taille avant HTTP ajoutée à la méthode. Pas de pourcentage ni durée globale déduite.

### Correction récursive485–489
Pas d’appel GLM supplémentaire. Le contrôle final a découvert notre refus repository trop large : test RED485 (46PASS/1FAIL), correction486 (47PASS), statiques/profile487 puis recette488 (73lectures/refus). Ces reprises corrigent le candidat477, pas de nouvelles fonctions migrées. Dépense et originaux conservés ; aucune économie Codex nette ou durée globale déduite.

### Vague490 — préparation visible et réparation500
8GLM simultanés,8complets sans reprise,16 824tokens déclarés, propositions relues/corrigées. Un parcours produit étendu (queued délégué) et une régression de479 corrigée (masquage actif).49tests ciblés,77+8lectures/refus publics.498 modifie seulement Rustdoc ;499 produit même binaire que495, donc réutilisation des recettes496/497. Aucun appel GLM ou recette publique supplémentaire pour ce commentaire ; pas d’économie nette Codex ni délai global déduit.

### Cycle501–509
8GLM/7complets/1tronqué, 26457tokens déclarés, sans retry payé. Parcours regroupé : processus délégué vivant observé, annulation avant départ, échec réel et soumission existante.50tests/82+8lectures-refus. Publieur fixture mal choisi502 diagnostiqué et corrigé503, originaux conservés. Next510 lancée sur commande CLI manquante ; aucun pourcentage/délai ni économie nette déduit.

### Vague510 — commande logs complète
8GLM/7complets/1tronqué,25646tokens déclarés sans retry. Une commande CLI reliée,2unitaires lecteur/2publication-causes/7existants,23lectures/refus publics. Hypothèses de harnais corrigées sans modifier protections ; frontière200 couverte en unité,4logs réels publics. Vague5208appels achevés/6complets/2tronqués, intégration séparée. Pas de pourcentage, délai ou économie nette déduit.

### Reprise GLM571–573
8rôles,15appels réels,7réponses complètes/1tronquée/7HTTP500 ;64416tokens
déclarés, usage des500 inconnu. Réduction de concurrence8→2 a permis cinq
réponses ; un500 avec un seul appel interdit de conclure à une limite fixe2.
Aucun gain net ou progrès produit chiffré : propositions relues, non intégrées.
Voir evidence/20261023-g1-status-contract/wave572/campaign-summary.json.

### Incrément574–601 — premier handoff automatiquement soumis
Un parcours G3 raccordé,6unitaires/9CLI/2guardsPython et2workers natifs publics,
4parcours navigateur Admin. Origines des reprises distinguées : faute de champ
Rust574, course du harnais577, consentement révoqué579, snapshotSQL manquant581,
pause planning587, métadonnées cache590, couplage de politique inutile596.
601 conserve les résultats et ne les convertit pas en pourcentage de migration.
GLM584/585/595 trois revues complètes,17843tokens déclarés, aucune facture
ou économie nette déduite. Vague610 traitée séparément (8rôles,7complets/1HTTP502).

### Incrément621–659 — contrôles réellement raccordés au watch
Parcours worker→rapport→contrôle avec refus/rejeu/révocation locale vérifié ;
reviewer/managed/recovery/web et acceptation globale encore ouverts. Source10inputs
revus,3tests internes/9CLI/4guardsPython ; trois vrais workers dans recettes
finales657/658, pas de pourcentage de migration déduit des volumes. Les reprises
625édition,626parentFKfixture et637replay sont distinguées, preuves originales gardées.
Les vagues GLM610/620/630/640/650 restent suivies séparément : réponses tronquées
ouHTTP non assimilés à acceptation. Usage déclaré dans chaque campaign-summary ;
pas de facture ni d'économie Codex inventée. Prochaine unité : reviewer intégré
au même parcours, pas une collection indépendante de preuves maximales.

###661–702 — premier parcours complet automatic reviewer
Le raccord réel est vérifié sur693 : planner→worker→rapport→contrôles→reviewer,
rejeu sans réexécution ; trois recettes refus/reprise/révocation et13parcours
manuels. Deux défauts de diagnostic reproduits690 avant692, garde relecture
canonique après quality failure reproduite669 puis697 passe. Les anciennes
preuves/faux départs sont conservés, aucun pourcentage déduit des tests/lots.
7008appels/7complets/1tronqué/92584tokens déclarés ;720contexte réduit, bilan
conservé dans son dossier. Aucun gain Codex ni facture fournisseur inventé.
Suite concrète : origine manuelle, consent planner, SIGINT puis G3 restants.

###709 — trois vagues de huit, effet mesuré
7007/8,7208/8,7406/8réponses complètes =21/24 ;190779tokens déclarés,3sorties
tronquées, aucunHTTPerror/retry. Nombre de rôles != HTTPsimultanés (2ici).
Résultats produit : deux corrections erreurs avant/après, vrai reviewer auto
avec financement/rejeu/refus/révocation ; ancienplanner refuse déjà stalelease.
Pas d'économie Codex ni d'estimationjours déduite des appels, aucune conformité
globale de migration déduite de ces recettes. Prochain besoin : async/arrêt Go.

###730 — variante manuel→reviewer assemblée, limite restante explicite
5vagues700/720/740/760/780 :40appels,36réponsescomplètes/4tronquées,272754tokens
déclarés sansretry/HTTPerror ; aucune économie Codex ou facture calculée.
716 vrai refus origineavantfix ;722defaultprofile et729positif+DBcopyrefus vérifient
origine physiqueDir+DB writer avantgrant, puis reviewerauto sansaffaiblirbail.
727/72813parcours manuels réexécutés car permission native commune changée.
Une limite de scénario reste visible : contrôle initial exécuté manuellement,
prochaine unité = enlever cette intervention par garde producteur commune.

###743 — contrôle initial manuel supprimé du parcours
731différenceGo reproduite,737/739parcours entier sans taskvalidate manuel ;
3recettes740–742 gardent autoproducer/rejeu/revocation.155missiontests (1ignored),
2policyruntimeguards, Clippy/Rustdoc. Échecs732helper/734fixturesversions traités
avec preuves, pas recréditésPASS. Sixvagues48appels GLM :44complets/4tronqués,
303342tokens déclarés sansretry/HTTPerror ; avis faussementalarmants réfutés par
vraiscallers etpolitiquesNone/Some, pas denouveauplafond opérateur. Build/version
Rust autonomeR7 reste à préparer ; aucune installation prétendue.
