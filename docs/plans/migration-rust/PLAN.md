# Migration intégrale de Swarm vers Rust — plan actif

Décision utilisateur : remplacement complet, bascule unique des écritures.
Développement direct dans ce dépôt ; agents Codex autorisés, coordination racine.
Préserver les modifications de Claude, les missions suspendues et les preuves.
Méthode : [APEX](../../../.claude/skills/apex/SKILL.md),
[prompt adopté le 22 octobre 2026](PROMPT-METHODE-CODEX.md).
L'option de service Rust anticipé en lecture seule attend une confirmation.
Ni commit/push, installation, restart ni fournisseur payant autorisés ici.

## Priorité opérationnelle — cadence corrigée du23octobre

Décision confirmée par l’utilisateur le 23 octobre 2026 ; règles applicables à la suite de la migration.

- Résultat suivant : `mission status WORK --json` utilisable de bout en bout sur un scénario ordinaire explicite, avec le contrat historique réellement produit.
- Assembler les producteurs déjà disponibles ; traiter rapports/Agent/runtime/version manquants dans cette unité, sans déclarer chaque composant comme un lot livré.
- Aucun champ inventé pour débloquer la commande ; les limites et refus réels restent visibles.
- Pendant le code : contrôles ciblés sur le comportement modifié ; conserver les preuves valides inchangées.
- À la fin : recette publique consolidée, profil réellement livré, revue indépendante requise, documentation et bilan court.
- Docstrings au fil du code ; documentation FR/EN concernée dans le même incrément, avec une synthèse finale unique.
- Améliorations non bloquantes conservées dans AFTER-CUTOVER avec propriétaire et reprise ; défauts réels et contrats requis traités dans le parcours.
- Agents : tâches indépendantes et fichiers réservés, contexte utile réduit ; aucune multiplication automatique.
- Suivi : capacité utilisateur vérifiée, temps de build/tests et reprises ; tokens indisponibles signalés, aucune estimation en semaines déduite des sous-lots.
- Clôture : parcours public exécuté avec résultats observables, refus/reprise pertinents et contrôles requis satisfaits. Une bibliothèque, un SDK ou un dossier de preuves seul ne clôt pas ce parcours.
- [Diagnostic de coût](evidence/20261023-g1-status-contract/COST-REPLAN.md) : constats conservés ; l'effet de cette cadence reste à mesurer.

### Reprise publique du 23 octobre — fichiers réservés

`go` confirme la reprise du développement après le diagnostic de coût.
Une unité : JSON historique public sur Work ordinaire, sans planning, Agent ni profil.
Racine : exports `lib.rs`/`root_store.rs`, CLI `mission_status.rs`/aide, recette et documentation.
Agent status_ordinary_aggregate : `root_store/mission_status_historical.rs` et ses tests.
Agent status_runtime_delivery : `mission_status_runtime.rs`, réglages/catalogue/defaults
`rust/crates/swarm-engine/config/`, tests settings, traductions de ces champs.
Réglages schéma38 ; schéma SQLite inchangé37. Trois seuils stockage persistés,
valeurs initiales héritées du Go ; aucun seuil caché dans le consommateur.
Gel des sources avant recette finale ; branches non prises en charge refusées.

## Pilotage par parcours

[Classement des surfaces et inventaire](CHEMIN-CRITIQUE-SPEC.md#décision-applicable--22-octobre-2026).
Contrats : données, JSON CLI consommé, refus sécurité/identité/budget/droits,
lancement et supervision. Autres surfaces : réécriture acceptée par parcours.
Les compteurs ci-dessous portent les groupes, pas des pourcentages de charge.
Premier corpus exécuté :2/2 scénarios de lecture Work identiques, sur binaires
identifiés ; source Go précompilée non qualifiée, aucune acceptation globale.

| Groupe | Prochaine unité de livraison | État de vérification/acceptation |
|---|---|---|
| G1 Cockpit | suivre mission et décisions | lectures/logs/status final, avis négatifs et acceptation explicite localement vérifiés555/556/558/560/563, revue bornée601 ; retry/closed/managed/cockpit/revue globale ouverts |
| G2 Terminal | session et contrôle autorisé | ouvert |
| G3 Supervision | conduire planner→worker→contrôles→reviewer | phases asynchrones ordonnées : six signaux/progression main1067 sur1066 ; reprise registered1067, manual/refus/révocations1070–1071 PASS ; concurrence Go complète, managed/web/fin/revue indépendante globales ouverts | Pont HTTP1170 et conducteur serveur1218 localement vérifiés ; managed/multi-tâches et revue globale ouverts.
| G4 Planification | préparer le plan de mission | ouvert ; composants natifs, parcours global à qualifier |
| G5 Préparations | préparation jusqu'au démarrage | ouvert ; dépend G4 |
| G6 Fournisseurs | configurer/résoudre/tester connexion | ouvert ; contrats secrets et budgets requis |
| G7 Confiance | autoriser, vérifier et révoquer | ouvert ; aucune protection globale supposée |
| G8 Projet/mémoire | éditer et partager avec provenance | ouvert |
| G9 Coûts/limites | configurer et observer consommation | ouvert ; réservations/refus contractuels |
| G10 Automatisation | déclencher et suivre un parcours | ouvert ; dépend G1/G3 |
| G11 Pages | compléter les parcours des groupes servis | ouvert ; FR/EN, thèmes, clavier et ressources réelles |
| G12 Divers CLI | recheck, aide et complétude | version/alias/texte/JSON906 vérifiés sur905 ; --server et complétude ouverts |

Ordre : G3/G1 → G4/G5 → G6/G9 → G8/G7 → G2/G10 → complétude G11/G12.
R6/R7 préparés en parallèle. Une recette complète en fin de lot vertical ;
entre modifications, tests ciblés. Risque A : instantané et revue indépendante ;
B : tests moteur ciblés puis recette finale ; C : parcours et inspection visuelle.
Replan immédiat aux anomalies ; trois échecs sur le même point imposent réduction.

## Lots actifs et ownership

| Lot | Objectif/parcours et risque | État, preuve, suite | Fichiers propriétaires |
|---|---|---|---|
| 201 réduit | liaison à l'instance Root ; A | composants vérifiés, non clos ; [preuves](evidence/20261022-root-instance-201/PLAN.md) ; revue indépendante ouverte | racine, intégration seulement |
| G3/G1 initial | lancer/suivre worker et arrêter conducteur ; A | [213 PASS](evidence/20261022-root-instance-201/watch-public213/result.json), revue196 ; périmètre partiel, suite rôles/reprise/web | racine ; CLI watch/status, tests et recette permanents |
| G3 W2 reprise | reprendre queued avec nouvelle génération ; A | [224 PASS](evidence/20261022-g3-watch-restart/PLAN.md), même admission et règlement unique ; revue indépendante favorable | moteur continuation_evidence ; CLI continuation_states ; recette differential_bench ; exports/docs racine |
| G3 W3 planification | watch planifie puis lance le worker ; A | [plan W3](evidence/20261022-g3-watch-planning/PLAN.md), [244 PASS](evidence/20261022-g3-watch-planning/public244/result.json) :16+12 gardes/3 Roots, régressions W1/W2 [243](evidence/20261022-g3-watch-planning/public243/result.json) ; revue indépendante favorable, async/registered/reviewer/validation/web ouverts | CLI continuation_states ; recette continuation_evidence ; docs/intégration racine |
| G1 W4 bilan JSON | porter les producteurs historiques puis agréger ; A | [suivi W4](evidence/20261023-g1-status-contract/PLAN.md) : DTO36/projections/lecteurs, sept lecteurs matériels corrigés, raccord résultats ordinaires290 ; recette299 PASS13 gardes/11 lectures-refus (Gate+ENOSPC/reprise), revue favorable. JSON ordinaire public309 PASS13 gardes/17 lectures-refus ; runtime réel/schema38, profil normal308. Agent/profil/planning et qualification globale ouverts | racine lecteurs/exports ; continuation_states tâches ; continuation_evidence dépenses ; revue lecteur250 favorable |
| Banc différentiel | premier corpus lecture Work ; A pour données/JSON | 14 gardes PASS ;2 lectures exactes sur binaires réels [194](evidence/20261022-root-instance-201/differential-public194/bench/result.json), source Go non qualifiée | differential_bench : banc et recette gelés, résultats213 intégrés |
| Diagnostic188 | causes externes du harnais ; B | reproduit puis corrigé, 2 tests ciblés PASS [190](evidence/20261022-root-instance-201/ledger-errors190/result.json) | continuation_states : ledger_sdk.rs seulement |
| R6/R7 préparation | restauration, rollback, installation ; A | build natif/version905–906 et restauration froide schema37/50tables924 vérifiés ; Go/anciensschémas/Rootactif/autorité/install globaux ouverts | racine : build_native, cutover_snapshot et recettes permanentes ; note191 conservée |

Fichiers partagés lib/root_store/settings/service/catalogue : changements additifs
annoncés ici avant écriture et gel pendant une recette d'un autre agent.
Réservation racine pour ce lot G3/G1 : swarm-cli/src/lib.rs (modules), mission.rs
(routage watch/status), swarm-cli/Cargo.toml (features Tokio rt/time explicites).
Incrément terminé par les agents : continuation_states possède mission_watch.rs ; continuation_evidence
possède mission_status.rs et éventuel mission_status_read.rs ; differential_bench
possède rust_mission_watch_public.py et recipe mission-watch-public.json.
Gel commun et intégration effectués ; normal209, tests204, Rustdoc206, recette213 PASS.
Serveur web et fichiers Live restent hors de cette première tranche CLI.
Status de conduite texte + extension explicite status-native JSONv1 sont prévus ;
le JSON historique ordinaire mission status est désormais vérifié309 ; ses branches
Agent/profil/planning restent refusées tant que leurs producteurs manquent.
[Préparation W4](evidence/20261023-g1-status-contract/PLAN.md) : carte22 et écarts
réels conservés, sans alias ni nouveau grant.
Claude conserve ses zones telemetry_*.rs, live*.rs, web/live* ; ne pas les écraser.

## Demande utilisateur — avancement visible (23 octobre, transmise par Claude)

L'utilisateur veut connaître l'avancement et le temps restant sans les demander.
À chaque fin de lot, publier ici : trois taux (existe / vérifié / accepté) par
CLI, web, données et bascule, depuis la matrice régénérée ; le découpage de
G1–G12 et R6/R7 en tranches estimées ; la vitesse observée et une fourchette de
durée marquée comme estimation. Consigne détaillée : [prompt § 9](PROMPT-METHODE-CODEX.md).
Outil : `python3 tools/migration-progress/avancement.py --record` (binaire Rust
reconstruit) régénère [AVANCEMENT.md](AVANCEMENT.md) (taux, vitesse, et taille
du code Rust/Go par domaine : mission ×3,0, planification ×2,2, travaux ×3,2 au
23 octobre — signal de surcoût à faire baisser) et l'historique ;
compléter `tools/migration-progress/preuves.json` à chaque preuve PASS, et la
matrice pour les équivalents renommés (51 opérations Rust sans homonyme Go).
Première estimation de Claude, faible confiance : 3 à 8 semaines de travail
continu, médiane ≈ 5 ([§ 3 quater](CHEMIN-CRITIQUE-SPEC.md)) ; à remplacer par celle de Codex.
Console live tel31 (Claude, 23 oct., instantané figé) : 17 étapes de tests PASS,
Clippy FAIL sur 16 lints des fichiers en cours mission_status_*/mission_worker_adoption
(aucun Live), puis recette dérivée sans Clippy 6/6 PASS dont navigateur ; preuves
`.cache/ev-tel31-231532` et `ev-tel31b-231532`. Recette complète à rejouer après
Clippy PASS du lot Codex. `snapshot.py` refuse désormais tout `include_str!` non déclaré.

## Limites et décisions qui conditionnent la suite

- Baseline de qualification :199/review28 ; review96 du lot201 reste bloquée,
  sans revue indépendante ni conformité globale. Tests locaux ≠ acceptation.
- Go branche le superviseur sur os.DevNull : nouveaux fichiers stdout/stderr
  durables différés dans [AFTER-CUTOVER](AFTER-CUTOVER.md), pas prérequis de watch.
  Les propositions101/120/189 contraires sont historiques ; erreurs et reçus restent requis.
- W2 reprend seulement queued Binding4 jamais enregistré, par reçu d'autorité ;
  contexte original et garde de lancement restent requis. La pause/unpause ne
  rafraîchit pas ce contexte ; registered et réautorisation stale restent ouverts.
  Le helper pré-claim peut sortir avec code2 sans cause interne remise au conducteur
  (223, stdout/stderr null hérités Go) : diagnostic sûr à compléter, racine G3,
  critère : refus réel transmis sans secret, état/réservation inchangés.
- Observer queued seul n'autorise pas sa reprise. Ne pas employer reconcile_agent
  comme récupération globale : il annule les queued. Examiner les générations,
  les branches planner/reviewer/validation et les effets avant nouveau lancement.
- Les instructions initiales restent sous le cap32k après extraction de la référence
  migration APEX ; recette213 prouve planning/worker fictifs. Les références
  conditionnelles doivent être fournies aux rôles sans outils, pas supposées reçues.
- Schéma37 accepté côté Rust ; migrations1..36 non portées. Bundle Work Go ≠
  snapshot complet Root. Build/install historiques encore Go ; aucune bascule faite.
- Retenir les durcissements supplémentaires dans AFTER-CUTOVER, avec provenance,
  propriétaire, état réel et condition de reprise, sauf défaut réel/contrat requis.

## Critères de clôture conservés

RM-01 référence exacte et changements concurrents ; RM-02 parité stricte limitée
aux contrats adoptés ; RM-03 données/historique/réservations et reprise ; RM-04
processus et budgets ; RM-05 identité/droits/mémoire ; RM-06 parcours navigateur ;
RM-07 livraison autonome sans Go ; RM-08 restauration et retour arrière ; RM-09
revue, recette globale et RETEX. Tous restent ouverts à l'échelle de la migration.

| Lot historique | État actuel | Condition restante |
|---|---|---|
| R0 | partiel | référence finale à figer |
| R1/R2 | partiels | données et migrations complètes |
| R3/R4 | partiels | moteur, supervision et autorités complets |
| R5 | partiel | CLI/API/UX complets et vérifiés |
| R6 | préparation commencée, recette non exécutée | migration/restauration et qualification globale |
| R7 | préparation commencée, aucune installation | livraison/rollback et RETEX sous autorité |

Les18 livrables P01–P18 restent détaillés dans
[PILOTAGE-CHARGE](PILOTAGE-CHARGE.md) ; leur présence n'est pas une mesure de charge.
[REPLAYS](REPLAYS.md) décrit les recettes permanentes. Aucune preuve requise en
échec ne disparaît avec ce changement de méthode.

## Historique préservé

[Plan antérieur intégral, octets originaux](evidence/20261022-method-adoption/PLAN-through-201.txt).
Les anciens numéros de lignes et liens relatifs de cet artefact se rapportent à
son ancien emplacement PLAN.md ; ils ne décrivent pas l'état courant.
[Manifeste de conservation](evidence/20261022-method-adoption/manifest.json).
Les sorties détaillées demeurent dans leurs dossiers evidence, sans réécriture.

### Suite active — bilan après annulation Agent (23 octobre)

Le producteur public de profil seul manque : ne pas ajouter une fonctionnalité
uniquement pour une preuve. Parcours existant choisi : agent prepare manuel →
annulation avant exécution → status historique de l'Agent interrompu et rapport
éventuel réel. Aucun fournisseur lancé. Propriétaire moteur : status_ordinary_aggregate
(`root_store/mission_status_historical.rs`, `mission_status_results.rs`, éventuel
`mission_status_reports.rs`) ; recette `rust_mission_status_contract.py` :
status_terminal_recipe. Racine : modules/exports, manifeste, documentation.
Racine corrige aussi les commentaires manquants des validateurs/tests privés de
`mission_status_runtime.rs`, sans changement de comportement, avant le prochain gel.
Critères : identité/état/résultat réels, scan rapport réel, refus sans données
inventées, causes et association conservées ; recette publique consolidée puis revue.

### Résultat public309 — reprise du besoin initial

Capacité utilisable ajoutée : `mission status WORK --json`, DTO historique réel
sur Work vide/todo sans Agent/profil/planning. Recette309 PASS13 gardes/17
lectures et refus ; intégrité du dossier vérifiée. Sourceproduction308 qualifiée
par normal308 ; SDK308214 entrées moteur identiques.168 sorties imbriquées
archivées et vérifiées membre par membre dans public-final310, aucun RootDB.
Revue indépendante finale favorable à consulter dans HANDOFF-public-review du suivi W4 ;
aucune conformité globale/bascule/install/interaction navigateur déduite.
Suite requise G1 : branches Agent/profil et historique d'exécution, puis planning,
supervision active, fenêtre visitée>200 et raccord cockpit ; ce sont des contrats
restants avant bascule, pas du durcissement optionnel après bascule.

### Résultat terminal325 et suite327

Capacité publique vérifiée : annuler avant départ puis lire l'Agent interrompu,
sa tentative originale, rapport partiel et réservation libérée sans usage/coût
fabriqués. Recette325 PASS13gardes/22lectures, intégrité VERIFIED ; revue favorable
production320/recette325.18tests lecteurs/Clippy/Rustdoc320 PASS ; profils normal
etSDK320 qualifiés.321 et323 restent FAIL de harnais,316 conserve contre-exemple
compatibilité ; correction générale des liens contenus, O_PATH et tri lstat incluse.
Suite : worker manuel terminé→rapport réellement soumis→JSON result_to_review.
Propriétaires inchangés : moteur status_ordinary_aggregate, driver status_terminal_recipe,
modules/exports/manifeste/documents racine. Aucun fournisseur payant, installation,
commit/push ou mission privée autorisés. Pas de port attribution/incomplete-report
si la branche submitted ne les consomme pas. Revue/recette finale par parcours.

### Ressource complémentaire autorisée — SKYNET GLM (23 octobre)

L'utilisateur autorise `skynet_harness` pour aider la migration. Capacité locale
identifiée : `/home/fpizzi/.local/bin/skynet_harness`, wrapper Claude Code vers
passerelle SKYNET ; alias configuré `glm-5.3-flash`. Clés utilisées uniquement via
environnement, jamais copiées dans dépôt/prompts. Première mission : revue ciblée
lectureseule du producteur completed/submitted et consommateur pur, outils/MCP
désactivés, contexte borné, pas d'écriture ni autorité d'acceptation. Preuve328
en cours ; accessibilité réelle/qualité/coût non déduits de la configuration.
Cette autorisation explicite complète les interdictions d'appels fournisseurs
non autorisés ; elle n'autorise ni autre service, installation, ni budget Swarm
implicite. Les résultats GLM sont des avis à vérifier sur les sources/tests.

### Accès direct SKYNET — précision de version

L’utilisateur confirme que le service interne est GLM 5.3. L’alias local
`glm-5.3-flash` utilise actuellement l’identifiant de compatibilité `glm-5.2`
dans la configuration LiteLLM ; l’API reprend cet identifiant. Ces métadonnées
ne démontrent pas la version réelle du backend et ne contredisent pas, à elles
seules, l’information fournie par l’utilisateur. Ne plus déduire la version du
modèle de ce seul nom. Accès HTTP direct vérifié au service SKYNET, sans Claude
Code, clés conservées dans l’environnement. Premier essai : réponse HTTP valide
mais aucun texte final avec 32 jetons de sortie, consommés en raisonnement ;
confirmation avec une allocation de sortie distincte conservée dans
`evidence/20261023-g1-status-contract/skynet-review328/`. Les consommations
renvoyées sont des observations, pas une facture ni un tarif démontré.
La revue complémentaire par wrapper a terminé en un tour ; ses deux remarques
restent des hypothèses à vérifier contre les producteurs et gardes amont.
Aucune acceptation de migration n’est déduite d’un avis GLM.

### Skill personnelle SKYNET — incrément329

Demande : faciliter les délégations GLM internes par une skill/règle.
Réalisé : `~/.codex/skills/skynet-agents/SKILL.md`, helper direct `scripts/call.py`,
tests rejouables `scripts/test_call.py` et métadonnées Codex. Propriétaire : racine.
Règle persistée dans `~/.codex/AGENTS.md`, incluse par `~/.claude/CLAUDE.md` ;
symlink `~/.claude/skills/skynet-agents` vérifié vers l’unique copie. La règle
apparaît dans les nouvelles instructions reçues ici ; cela ne démontre aucune
mise à jour des autres sessions ni des contextes embarqués Swarm.

Vérifié : réponse exacte SKYNET_DIRECT_OK par le helper, fin stop, usage déclaré
261 jetons (52 entrée,209 sortie, dont203 raisonnement), sans prix supposé.
Sept tests locaux nommés PASS : succès/contexte/empreintes, vide/tronqué/malformé
et reprise, refus HTTP sans retry, redirection non suivie, anciennes preuves
préservées/refus de clé dans le prompt, réponse masquée, ENOSPC/reprise atomique.
Format skill valide. Preuves, commandes, sorties, empreintes et revue explicite
politiques/fonctions/erreurs/documentation :
`evidence/20261023-g1-status-contract/skynet-skill329/`.

Replan final : inventaire signale une docstring manquante sur redirect_request ;
corrigée. Échec original conservé ; équivalence AST exécutable vérifiée pour cette
seule modification documentaire. Les tests/appel restent liés aux empreintes
antérieures, sans prétendre avoir exécuté les nouveaux octets de commentaire.
Le lanceur avait poursuivi deux contrôles après cet échec ; ils ne prouvaient pas
la clôture. Revue finale complète désormais conservée.

Limites : tâches sans outils ni accès implicite au dépôt, contexte à contrôler
pour les secrets inconnus, aucune acceptation Swarm. Endpoint/modèle/allocation
explicites ; timeout facultatif, aucun retry automatique. Version GLM5.3 selon
l’utilisateur, identifiant API de compatibilité glm-5.2. Gain migration non mesuré.
Pas de changement applicatif : pas de build/restart, version serveur, traduction
UI ou release note produit à modifier pour cette skill personnelle.
Suite initiale inchangée : recette du parcours worker terminé/soumis.

### Copie locale GLM demandée pour LIMIER

Skill dupliquée dans `/home/fpizzi/workspace/dremio_ai/.claude/skills/skynet-agents`.
Quatre fichiers identiques par SHA256 à la source personnelle ; caches Python exclus.
Exclusion ciblée `/.claude/skills/skynet-agents` dans les deux .gitignore, modifications
préexistantes préservées. Aucun fichier de cette skill déjà suivi dans les dépôts.
Preuves et suivi LIMIER : `.apex/skynet-agents-local-2026-10-23/`. La copie est
indépendante, sans synchronisation automatique ; aucune intégration applicative.

### Délégation GLM migration330 — lancement autorisé

Deux tâches par API directe, contexte figé et empreintes conservés dans
`evidence/20261023-g1-status-contract/glm-migration330/` :
- completed-contract : contrôle du raccord worker terminé/soumis et proposition
  du manifeste de recette ; pas d’écriture, intégration racine.
- cooldown-port : proposition de lecteur partagé et affichage cooldowns, sans
  affaiblir l’admission ni dupliquer les validations ; pas d’écriture.
Racine : compilation/recette du parcours completed et fichiers partagés. Aucun
agent Codex supplémentaire, aucune installation ni mission Swarm lancée.
Allocations de sortie explicites par tâche ; pas de retry ni watchdog global.
Les résultats seront examinés et vérifiés avant intégration ; gain non mesuré.

Replan330 : les deux appels ont épuisé respectivement4096/6144 jetons de sortie
en raisonnement seul, sans texte final (preuves originales conservées). Aucun
patch obtenu ou avis exploitable n’est annoncé. Diagnostic : allocation de sortie
consommée par reasoning_content ; contrôler l’effort explicite avant toute reprise,
pas simplement augmenter l’allocation. Référence officielle :
https://z.ai/blog/glm-5.3 (reasoning_effort low/high/max ; défaut max déclaré).
La prise en charge par SKYNET reste à tester. Échec documentaire séparé : le chemin
anglais réel est docs/en/RUST-MIGRATION.md ; traduction complétée avant compilation.

GLM330 repris avec effort low explicite : deux réponses finales obtenues,
11,119s/16,920s et1288/2399 jetons de sortie déclarés. Pas de benchmark contrôlé
ni économie Codex mesurée. Ajout du témoin completed au manifeste intégré ;
proposition cooldown non applicable telle quelle (type Box, contrat expiration,
composition des causes close). Suite et limites dans glm-migration330/INTEGRATION.md.
Huit tests du helper avec effort explicite PASS ; aucun default forcé.
La copie indépendante LIMIER de la skill reste à sa version initiale ; contrôler
les empreintes et propager le réglage d’effort avant délégation complexe depuis
cette copie, sans écraser d’éventuelles modifications locales.

### Résultat public completed331/332

19tests lecteurs PASS, Clippy/Rustdoc PASS ; normal331 profil défaut qualifié,
SDK331 mêmes entrées moteur. Recette332 VERIFIED/PASS13gardes/25lectures, incluant
worker local réel puis task submit, statut result_to_review/completed/submitted/
missing_gate, refus sortie /dev/full et reprise ; précédent Work annulé préservé.
Manifeste corrigé à partir de la proposition GLM et du vrai producteur de stdout.
Revue indépendante finale confiée au relecteur existant, sans nouvelle compilation.

API modèles : max_model_len déclaré524288, compaction locale1000000 distincte ;
aucun remplissage testé. Skill personnelle actualisée et copie LIMIER propagée
après contrôle exact du manifeste initial, sans écraser de changement local.
Les tâches GLM sont terminées ; leur gain Codex n’est pas mesuré.

### Correction du contexte SKYNET demandée

API annonce524288 ; ancien réglage de compaction1M corrigé dans le dépôt source
`/home/fpizzi/workspace/skynet-install`, branche feature/s1-glm-context-limit.
Menu/defaults/validation Linux et documentation cohérents ;6tests ciblés, Bash-n,
ShellCheck error PASS, avis GLM bornés. Copie installée rendue depuis les sources,
blocs routes/clés préservés ; fichier contexte524288 et menu public vérifiés.
Aucune session/proxy redémarrés, pas de commit/push. Preuves source :
docs/reviews/evidence/s1-glm-context-limit/final/local-delivery.json.
La skill personnelle et sa copie LIMIER décrivent maintenant le réglage corrigé.

Revue finale du parcours completed331/manifeste332 : approve borné reçu et
rapport additif conservé dans HANDOFF-public-review.md. Aucune installation
Rust ni acceptation globale déduite de ces preuves.

Revue locale quatre axes liée aux quatre entrées effectives modifiées normal320→331 :
DECLARED_REVIEW_COMPLETE, références archivées public331. Premier collecteur
refusé par homonyme de module dans le chemin local ; chargement du chemin canonique
explicite corrigé et vérifié, échec original conservé. Pas de rejeu des tests.
Baseline globale inchangée ; aucune conformité globale déduite.

### Reprise GLM333 — cinq délégations internes autorisées

Cinq propositions sans outils ni écriture partagée : lecteur cooldown/admission ;
projection planning historique ; clipping Unicode ; résultats completed non soumis ;
recette publique cooldown. Contextes sources figés et empreintes sous
`evidence/20261023-g1-status-contract/glm-migration333/`. Intégration séquentielle
et fichiers partagés réservés à la racine ; aucune source produit modifiée pendant
les appels. API glm-5.2, service GLM5.3 selon utilisateur ; effort low explicite,
6144 jetons de sortie par tâche, cinq appels, aucun retry/watchdog global.
Critère : cinq réponses finales examinées, code proposé distingué du code intégré
et testé ; les garde-fous de statut non couverts restent ouverts.

GLM333 :5appels terminés,4réponses finales et1tronquée. Revue racine dans
glm-migration333/INTEGRATION.md ; aucun patch appliqué sans examen. Le contexte
planning fourni par racine était incomplet (decisions/scopes exclus) ; les
hypothèses associées sont refusées. Recette proposée rejetée pour faux PASS/SKIP.
Reprise concrète : port cooldown à continuation avant close et projection statut ;
témoin valide absent puis actif attendu observable en test334 avant correction.

### Candidat335 — cooldowns visibles sans autorisation de lancement

Contre-exemple334 :1test exécuté, échec attendu après témoin absent valide ;
refus provider_cooldown empêche actuellement l'observation. Proposition GLM
adaptée : continuation avant close conservée, aucun digest/enum inutile ajouté.
Fichiers racine : agent_launch.rs, mission_status_reports.rs et historical.rs ;
docstrings et documentation FR/EN/version mises à jour. Test actif, expiration
exacte, clearance, corruption et reprise ajouté ; qualification ciblée en cours.
Pas d'installation ni de clôture globale ; fichiers partagés gelés pour les tests.

### Résultat335/336 — premier portage issu des cinq GLM333

Cooldowns visibles dans le statut sans lever le refus de lancement :20lecteurs
et1admission PASS, Clippy/Rustdoc PASS. Natif335 sans features fixture, SDK336
même moteur, recette publique336 VERIFIED/PASS13gardes/31lectures ; corruption
refusée sans fuite, restauration/absence et DB inchangée vérifiées. Revue locale
quatre axes normal331→335 : DECLARED_REVIEW_COMPLETE4fichiers, baseline globale
inchangée. Parseur admission interrompu sur faux zéro ; stdout nommé1PASS
conservé/diagnostiqué sans rejeu. Recette permanente et docs FR/EN à jour.
Relecture GLM distincte du candidat transmis, sans outils et sans autorité
d'acceptation ; elle ne modifie aucun fichier. Pas d'installation/publication,
ni migration globale terminée. Autres propositions GLM et préconditions de reprise
sont consignées dans glm-migration333/INTEGRATION.md.

Relecture GLM distincte336 terminée : approve borné, aucun défaut introduit
constaté dans le contexte fourni ; sans outils ni nouvelle exécution indépendante.
Risque inventaire recette qualifié par les vrais inputs normal335 : helpers
error/model/decode/local_storage inchangés par rapport331 (ROOT-RESOLUTION.json).
Correction d'une phrase de revue : instant cooldown tiré de system_timestamp,
pas d'une tolérance supervision. Sérialisation de tous champs optionnels non
nouvellement testée ; pas de parité Go universelle. Les5workers ont déclaré
244772tokens entrée/24058sortie ; revue distincte59977/2723. Aucun décompte
Codex par scénario témoin : économie nette NON MESURÉE, aucune promesse chiffrée.

### Vague GLM337 — cinq portages ciblés

Suite335/336 préservée ; cinq propositions : attribution rapports (Go complet),
projection planning (scopes/decisions complets), supervision lease, événements,
clipping Unicode avec oracle uniseg0.4.7. Contextes réduits aux interfaces utiles,
items complets et empreintes sous glm-migration337 ; aucune source produit écrite
par GLM. Intégration/routage partagé propriétaire racine, Claude préservé.
Effort low,6144sortie par tâche,5appels sans retry/watchdog global. Vérifier
les réponses puis code/tests : une réponse n'est pas un portage accepté.

GLM337 terminé :4réponses complètes/1tronquée. Revue INTEGRATION.md conservée.
Ne pas dupliquer planning déjà porté ; événements/Unicode non intégrables sur
hypothèses. Supervision : reproduction338 avec vraie claim publique avant
correction racine (unités ms/ns, cause typée, pas de faux booléen par défaut).
Aucun autre agent/SQL privé/processus serveur modifié.

### Candidat339 — projection des baux natifs

Témoin338 : absent valide puis vraie claim publique ;1test exécuté, échec attendu
mission-status-live-supervision-unported conservé. Intégration racine dans seul
historical.rs, sans doublon module proposé : types corrigés, ms→ns exact, aucune
observation manquante remplacée par false. Cadence persistée réellement consommée,
libellé adapté aux valeurs personnalisées. Tests public Root API et limites exactes,
cycle error/recovery/stop et refus temporels invalides ; docs FR/EN/version à jour.
Les autres contrats/branches non portées gardent leurs refus. Vérification en cours.

### Replan cadence après question utilisateur — 23 octobre

La cadence précédente a confondu vague GLM et fin de lot : recette336 complète
après chaque portage local, malgré la méthode adoptée (ciblé entre incréments,
recette complète à fin de parcours/lot). Correction immédiate : ne pas lancer de
recette340 complète maintenant.22lecteurs339, Clippy/Rustdoc et natif339 qualifiés
sont conservés, sans rejeu. Le manifeste permanent est étendu au bail natif ;
il sera exécuté à la prochaine clôture du parcours G1 ou si un défaut/dépendance
concrète exige le rejeu élargi. Recette336 reste une preuve historique des anciennes
entrées, pas une preuve fonctionnelle des nouvelles. Relecture GLM339/340 suivie
jusqu'au résultat ; aucune promesse de recette340 PASS. Propriétaire racine.

Résultat339/341 :22lecteurs PASS (prise réelle via API Root, seuils ns, cycle
error/recovery/stop), Clippy/Rustdoc PASS, natif339 défaut qualifié. Revue locale
normal335→339 DECLARED_REVIEW_COMPLETE2fichiers. Relecture large340 tronquée
conservée ; relecture341 réduite au diff/lecteur réel, même allocation, approve
borné obtenu. Remarques : stale→absent suit Go (opportunité UX après bascule) ;
validation historique stopped inchangée prouvée par SHA du lecteur.
Pas de recette340 complète ni installation ; manifeste étendu prêt à fin G1.
Les autres propositions337 restent à reprendre selon INTEGRATION.md ; leur
réception ne compte pas comme portage. Aucun gain net Codex chiffré.

### Campagne342 — cinq tâches sur un même portage Unicode

GLM internes : automate graphemes, largeur/itérateur, générateur tables, clip,
oracle différentiel Go ; interfaces nouvelles convenues dans prompts. Root seul
écrit/assemble terminal_clip et trace ; aucun Cargo dependency imaginé. Uniseg
Go0.4.7 en cache lu seulement, Go produit non restauré. Défaut candidat :
U+2E3B vaut4cellules/3octets, invalide le commentaire de garde actuel ; vérifier
l'oracle avant correction. Allocations6144 sortie par tâche, effort low,5appels,
aucun retry/watchdog global. Tests ciblés, recette complète en fin de parcours.

Campagne342 :5réponses finales. Oracle Go mesuré : ASCII600/600 ; U+2E3B
600octets/800cellules ; é1200/600. Témoin Rust343 rouge après contrôle valide.
Intégration344 : automate/largeurs/tables/clip/oracle assemblés sans dépendance.
Corrections de propositions : boucles non avançantes, mauvais état initial et
types usize/isize, constante U+2E3B erronée, générateur incomplet/noms divergents,
Go oracle non compilable et stdout non flushé. Originals conservés, aucun patch
exécuté aveuglément.615cas Go +1112064scalaires conservés en fixture permanente ;
qualification ciblée en cours. Pas de recette complète à chaque campagne.

344 :23lecteurs PASS ;8tests Unicode PASS dont615cas et SHA de largeur de
1112064scalaires.3tests outils génération/refus/récupération PASS. Clippy refuse
seulement deux if sur même ligne dans données générées ; générateur corrigé
(saut de ligne), source344 figée et échec conservés.345 : ciblage table/trace
et statique, sans relancer les23lecteurs ni recette complète.

Campagne346 : deux relectures GLM séparées et ciblées (algorithme ; générateurs/
oracle), après assemblage342. Pas de nouveau développement doublon ni recette
complète par vague. Résultats et limites liés aux contextes empreintés conservés.

### Campagne348 — historique des tentatives terminées

Deux tâches ciblées GLM (port de garde/historique ; contre-exemples/test concret),
consommateurs Go/Rust réels fournis, pas de modèle supposé manquant. Root seul
intègre reports/tasks si justifié. Port Unicode acquis en ciblé conservé ;
aucune recette complète entre ces campagnes.4096sortie/tâche, effort low,
pas de retry ni watchdog global ; réception distincte d'intégration/acceptation.

Unicode342→347 terminé en ciblé :23lecteurs344PASS,8Unicode615cas/tous les
scalaires PASS ; trace345/ClippyPASS et Rustdoc347PASS.6tests outils PASS,
oracle avec chemin à espaces/environnement Go explicite et sources imprévues
refusées : JSON identique à la fixture. Revues346 approve borné ; petites
corrections outils post-revue couvertes par revue racine/tests, pas annoncées
indépendamment approuvées. Tables/code runtime après correction doc identiques
à345 ; réutilisation documentée. Pas de nouvelle recette globale/profil livré.

348 : proposition de port reçue ; proposition tests a épuisé4096raisonnement
sans texte. Pas de retry inchangé ; racine écrit les assertions sur la vraie
fixture. Reproduction349 rouge après contrôle valide (ancienne completed
refusée).350 supprime seulement la garde d'historique, garde récente/ownership
conservées. Deux consommateurs réels (historical/results), pas seulement un
contrairement à une phrase GLM. Source SQL dernier rowid vérifiée ; tests ciblés
en cours. Pas de nouvelle politique, aucun remboursement/réservation/état SQL.

Résultat350/351 :23lecteursPASS, Clippy/RustdocPASS, revue GLM approve borné.
Historique completed conservé sans remplacer la dernière tentative ; acteurs
actifs et branches incohérentes toujours refusés. Preuve multi-workers CLI et
profil production restent ouverts, pas de recette complète par vague. Revue
racine quatre axes liée aux octets350 dans history350/source-review-local.json.
Skill SKYNET complétée sur campagnes cohérentes, interfaces réelles et cadence
de recette ; validation structurelle PASS. Copie LIMIER propagée après égalité
octet pour octet avant écriture ; preuve skill-campaign-method352.json. Cela
ne recharge pas les contextes des processus déjà démarrés.

G1/R6 — découverte fenêtre agents Go (preuve history350/go-history-window.txt) :
s.agents utilise agentHistory(recent=true) LIMIT200 ; spendingAgents utilise
recent=false sans limite. Rust retient actuellement toutes les lignes. Aucun
contrat au-delà de200agents déclaré conforme par nos cas à2tentatives. Racine
doit porter la séparation vue récente/comptabilité complète et la source de
paramétrage Admin attendue avant clôture G1, avec201agents/cas tâche hors fenêtre
et coûts conservés. R6 doit préserver l'ordre rowid relatif lors migration et
restauration, sans perdre les tentatives/certificats. État ouvert, pas de plafond
ou nouveau refus improvisé pour cacher l'écart.

### Campagne353 — séparation vue récente et comptabilité

Reprise G1 sur preuve Go LIMIT200, deux propositions GLM ciblées : consommateurs
réels et contrat de paramètres persistés. Aucun code accepté par réception ;
Root réserve settings/catalogue/lecteurs si intégration justifiée, préserve web
Claude.4096sortie/tâche, low, aucun retry/watchdog ; pas de recette complète.
Attendu : vue bornée sans perte comptable, historique absent distinct d’illimité,
201tentatives et tâche hors fenêtre ; readiness non observée jamais inventée.

353 : deux réponses complètes. La proposition consommateurs (260s, contexte
228Ko) réintroduisait200en dur, copiait les données privées et proposait un nouvel
événement de fixture public : rejetée. Lenteur observée, aucun retry ; prochains
contextes limités aux fonctions/contrats consommés. Contrat cap retenu après
revue racine, groupe complet exigé et noms work_view pour Admin existant.
354 : contrôle2événements passe puis201échoue sur refus-pagination (preuve
originale conservée).355 : source vue empruntée, dépenses complètes, réglagesv39
null/0/200explicites ; tâches hors fenêtre restent refusées si readiness non
portée. Script édition arrêté sur marqueur Rustdoc //! mal lu (aucun test lancé
après échec), reprise à l’étape exacte corrigée, pas de double intégration.
Critères ciblés en cours ; G1/profile/recette globale non clôturés.

355 :24lecteurs et10settings PASS, Clippy/Rustdoc PASS. Profil normal355 qualifié
sans features de fixture, sans installation.357 : recette Admin ciblée ajoutée
au lanceur browser existant, deux champs FR/EN/thèmes/enregistrement/réouverture,
preuve ressource anglaise effectivement servie étendue aux champs nommés.
Erreur script recette avant toute écriture (marqueur use-strict placé avant
commentaire) corrigée après lecture ; node-check dépendant a été appelé à tort
après cet échec et a refusé fichier absent. Prochaines étapes dépendantes
séparées, sources applicatives inchangées ; originaux du diagnostic conservés.

353→359 : ciblage terminé :24lecteurs et10settings PASS, Clippy/Rustdoc PASS ;
358 garde hors fenêtre PASS (201valides, vue200refusée, reprise complète).
Admin357 servi réellement :4FR/EN×thèmes, desktop/mobile, preview pur, save/reopen,
historique, aides/Escape/focus PASS ; inspection visuelle FR390clair/EN1440sombre
sans débordement observé. Locale protégée = inventaire effectif normal355.
Revue356 request_changes préservée ;359 gardes/SQL exacts résout les2points,
aucun défaut introduit démontré. Contexte omis corrigé, pas de validation fictive.
Revue racine quatre axes liée aux15fichiers dans window360/source-review-local.json.
Normal355 qualifié précède seulement l’ajout cfg(test)358 ; production identique
avant cfg prouvée, mais inventaire brut/collecteur courant reste à G1.
Aucune recette globale ni installation. G1 garde readiness hors fenêtre et autres
branches non portées ouvertes ; R6préserve ordreSQL, migration globale non clôturée.

### Campagne361 — résultat soumis hors fenêtre

Reprise : le résultat submitted court-circuite initial_task_state ; l’absence
d’Agent récent n’exige donc pas readiness dans cette branche. Deux tâches GLM
port de garde/observation et recette publique,2048sortie low chacune sans retry.
Root réserve reports/historical/tests statut ; aucun fichier web partagé.
Attendu : vue0 → revue/processus inconnu, identité tentative non projetée,
comptabilité complète, restauration vue200 ; autres readiness toujours refusées.

361 : garde reçue ; recette tronquée (2048sortie dont1365texte), aucun retry.
Root écrit la recette sur Driver existant. Correctif ne duplique pas les gardes :
une branche commune Agent présent ou task submitted garde les refus review/
recovery. Texte GLM «full history prouve toujours completed» corrigé : absent
partout signifie processus inconnu, jamais preuve d’exécution.362 rouge après
contrôle full201valide, ancien refus conservé.363 raccord submitted sans readiness,
tests ciblés et parcours public à vérifier ; pas de recette globale par campagne.

363 :24lecteurs/Clippy/Rustdoc PASS, profil normal363 qualifié.364premier
parcours arrêté avant le cas visé : Fixture modèle fournit providers.json
non canonique (env_allow/limits absents) ; le writer légitime les normalise et
la vérification de préservation stricte du helper completed refuse. Aucun défaut
produit prouvé. Précondition corrigée par preview/apply public de normalisation,
avec valeurs d’origine préservées, sans provider lancé ; original364conservé.
Reprise365sur même ancien binaire pour reproduire ensuite le vrai refus hors
fenêtre ; aucune simple répétition ni augmentation de budget GLM.

### Incident367 — écriture heartbeat pendant bootstrap helper

365après normalisation fournit un nouvel échec distinct : helper Root bootstrap
Prepare/BUSY5, aucun fournisseur commencé, receipt original préservé et diagnostic
sûr dans gate367. Hypothèse : parent next=now écrit immédiatement un heartbeat
BEGIN IMMEDIATE pendant les réouvertures Root à politique bootstrap0délibérée.
Root réserve agent_supervision.rs/agent_execution_store.rs/agent_storage_retry.rs ;
aucune modification du gate/telemetry de Claude ni de la politique bootstrap.
Test discriminant prévu Control.observe : pulse valide, canal pending aucune
écriture, reprise EOF, owner invalide refus/reprise et stop avant exec observé.
GLM367avis ciblé ; aucune répétition inchangée du parcours public.

368test s’arrête dans le témoin : assert logs1faux (pulse ne journalise pas,
seule la ligne Agent change). Source relue, contrôle corrigé ;369témoin pulse
réel valide puis assertion rouge précise : pending change total3au lieu2.
Cause propre parentwriter prouvée, pas preuve que chaque BUSY externe disparaît.
370 : observation propriétaire DEFERRED/rollback pendant canal pending, mêmes
contrôles owned/stop, heartbeat après EOF. Bootstrap0et politiques temps/droits/
budgets inchangées. Avis GLM367 plausible borné ; mappings d’erreur de l’avis
non repris aveuglément (stop n’est pas un refus, erreurs typed conservées).

370 :4tests contrôleur dont nouveau cas complet,3tests storage_retry, Clippy
et Rustdoc PASS. Profil normal en construction ; reprise publique uniquement
après cette précondition corrigée. EOF = fin phasehelper, jamais preuve exec.
Heartbeat non renouvelé pendant phasehelper ; les seuils existants peuvent
montrer une observation ancienne sans prouver la disparition du propriétaire.
Recette permanente soumise ajoute pureté des réglages autour du read et conserve
aussi la cause primaire si l’écriture du rapport d’échec échoue.

372public nouveau normal370 : vrai worker completed, rapport soumis, vue0
présentée sans refus et reprise settings réussie. Assertion de recette trop
forte refuse attempt_id retenu : source Go graph_draft_projection.go84–110 et
Rust attempt() confirment fallback dernière tentative Task. Ce n’est pas une
identité Agent inférée. Recette/Rustdoc/doc corrigées, aucun changement runtime
pour «faire passer» ; original372conservé. Soumis view0 doit garder Taskattempt,
processus unknown, Agentid absent, rapport/gate et toutes dépenses conservés.

373public PASS : processus local réellement completed, task.submit public,
vue Agent0réglée preview/apply, processusunknown/Agentidabsent/Taskattempt
conservé comme Go, mêmes appels/tokens/coûts et settings/history purs par read ;
restauration settings et projection positive vérifiées. Rootfixture conservé.
Normal370profil sans fixture.375fige les Rustdoc corrigées et notesFR/EN
actuelles ; code lecteurs/tests identiques à370après inversion exacte des seuls
commentaires (final375/reader-source-reuse.json). Pas de recette globale par vague.

375normal courant qualifié322inputs, Rustdoc courant PASS.376public sur ce
binaire final PASS (370preuve de parcours réutilisée seulement avant dernière
compilation ;376vérifie l’exécutable exact). Revue racine377canonique6fichiers
effectifs DECLARED_REVIEW_COMPLETE sur inventaires355→375 ; portée locale
explicite, ne remplace pas global199/review96/G1indépendant. Source GLM371
questions cleanup/journal résolues374surfinish/RootREAD_ONLY/SELECT exacts,
pas de verdict global inventé. Source366reste avis incomplet, tests locaux et
parcours376 distincts. Manifeste ciblé soumis ajouté pour run/verify pérenne,
sans exécuter les autres parcours/SDK de la recette G1.

378recette ciblée pérenne PASS, outil verify : integrityVERIFIED.379met à jour
le tableau G1/G3 et le suivi W4 ; pas de clôture G1globale, global199/review96
conservés ouverts.31tests nominatifs ciblés (24lecteurs363 +4contrôleur/3stockage370),
Clippy et Rustdoc PASS ; normal375 SHA83f423e2b157f2974fac015918bd49c7d9d4b3fe75a2a99be131fdf13bfb0f81.
Binaire candidat, pas d’installation. Revue GLM366incomplète et371questions
conservées,374questionscleanup/journal résolues par sources, pas d’approbation
globale fabriquée. Corpus public376 puis manifeste378 utilisent ce binaire exact.

Amélioration capitalisée : planning_native différait déjà ses renouvellements
et revalidations après gate_eof ; ce principe n’avait pas atteint le heartbeat
worker. Les autres contrôleurs ont été relus (reviewer Observe IMMEDIATE sans
DML, helper Rootreadonly/deferred) : aucun même défaut démontré, pas de patch
aveugle. Propriétaire Root, critère futur : vérifier phase/canal/type réel de
transaction pour chaque nouveau contrôleur, sans inventer délai ou exec depuisEOF.

### Reprise380 — décision d’abandon sans Agent

Écart observé : result_presentation et project_tasks savent lire abandoned,
mais supported refuse ce parcours agrégé. Choix : raccorder l’abandon sans Agent
avec les gardes existantes ; différer l’historique Agent/review et waived faute
de parcours qualifié. Pas de changement writer, droit, budget ou politique.
Racine réserve reports/historical, recette et documentation ; GLM380 propose
sans outils ni autorité d’acceptation. Témoin todo puis abandon public/reopen ;
RED381 avant raccord, état de preuve à compléter. Recette ciblée, pas G1 entière.

381 RED attendu : todo valide puis task.update abandoned public ; refus
mission-status-task-history-unported.382raccord réutilise la garde terminale
complète, pas l’else proposé par GLM380 qui aurait omis review/recovery.
Le producteur Go/Rust abandon sans Agent garde process_state not_started ;
assertion ultérieure non exécutée381 corrigée avant GREEN, aucune modification
de ce producteur. Pas d’Agent/acceptation/readiness inventé. Vérification en cours.

386recette publique FAIL sur attente trop forte de notre harnais : abandon puis
réouverture CLI réussis, reads purs ; seule recovery_preview évolue (révision2→4
et correction=motif conservé). Source mission_status_tasks recovery_preview
confirme la provenanceWork.387corrige l’attente à partir du Work retourné public,
sans modifier moteur/binaire normal385. Original386préservé ; replay frais requis.
Apprentissage : une reprise réussie conserve l’histoire, elle ne réinitialise
pas révision/décision. Comparer la projection complète avec ces deux changements
justifiés, pas supprimer les champs historiques pour faire passer le test.

388recette ciblée PASS + verify integrityVERIFIED, trois status reads purs,
no-Agent/no-provider. Profil normal385 qualifié et archivé, code lecteur identique
aux25tests382 ; seulement notesVersion modifiées avant build385. Revue389
quatre axes déclarée complète sur les3fichiers effectifs375→385, sans remplacer global199/review28
ni fermer review96. Revue GLM384 favorable sur contrat fourni ; pas de test exécuté par GLM ni acceptation globale.

### G1 — lecture d’une exécution ordinaire,400 vérifié
`mission status --json` présente maintenant la préparation queued incertaine et le vrai superviseur d’un worker local actif. Recette publique400 PASS/verify VERIFIED :8 lectures/refus purs, fenêtre0 refusée puis restaurée, rapport précoce non validé, annulation de la première tentative, réouverture/exécution de la seconde puis soumission en revue.27 lecteurs396, Clippy et Rustdoc PASS ; profil normal398 sans fixture, non installé. GLM390 proposition et393/397 revues bornées, pas d’acceptation indépendante globale. [Revue et preuves401](evidence/20261023-g1-status-contract/active401/review-notes.md). Limites : planning, reviews/validation, états terminaux supplémentaires et recette G1 complète restent ouverts ; baseline199/review28 et review96 inchangées.

### G1 — résultat accepté relu,411 vérifié
Vrai worker local terminé/soumis, gate et acceptation explicites de l’opérateur sur fixture ; statut frais→preuve changée/périmée→octets restaurés/frais, décision enregistrée inchangée. Dépendant terminal abandoned pour isoler la carte de dépendances sans disponibilité profilée ; waiting_on vide/frais puis parent stale explicite. Agent hors fenêtre et restauration des réglages conservent les coûts complets.8 lectures/refus purs,28 lecteurs406, Clippy/Rustdoc406 et Rustdoc final407 PASS. Profil normal407 sans fixture, non installé. Deux appels GLM402/405, proposition partiellement corrigée et revue favorable bornée ; aucune acceptation indépendante globale. [Preuves412](evidence/20261023-g1-status-contract/accepted412/review-notes.md). Suite : disponibilité todo profilée, planning/revues/validation et états terminaux supplémentaires ; baseline199/review28/review96 conservés.

### G1 — todo avec profil sauvegardé,418 vérifié
La recette accepted existante conserve maintenant le vrai dépendant todo : manuel→pause/attente→reprise/manuelle, sans Agent ni tentative nouvelle. Preuve acceptée modifiée/restaurée et dépendances réelles toujours vérifiées.10 lectures/refus purs,29 lecteurs414 et Clippy/Rustdoc PASS ; profil normal416 sans fixture, candidat non installé. Aucun nouveau script/manifeste : le parcours réutilise la recette permanente. Automatic readiness reste explicitement refusée (assist/capacité/workspace à assembler), tout comme planning/revues et qualification G1 complète. [Preuves419](evidence/20261023-g1-status-contract/profile419/review-notes.md).

### G1 — statut avant première activation hiérarchique,424 vérifié
Besoin automatique : mission start exige une organisation hiérarchique valide. Raccord vérifié : mission vide→planning enable→configuration prête→start autorisé mais sans conducteur actif→pause/reprise mission et planning.19 lectures/refus purs dans la recette accepted existante,30 lecteurs422 et Clippy/Rustdoc PASS ; normal423 SHA d9549190… sans fixture,323 entrées effectives archivées, candidat non installé. Cooldown actif du planning explicitement refusé puis restauration vérifiée. RED420 et compilation421 conservés ; aucune fiction SQL ni nouvel outil de recette. L’organisation prête est structurelle, pas une revue effectuée. Planning avec tâches/activations/scopes tenus, readiness automatique, revue/validation et cockpit restent ouverts, ainsi que review96/baseline199, R6/R7 et RETEX. Bilan exact : [revue426](evidence/20261023-g1-status-contract/planning426/review-notes.md).

### G1 — attentes fournisseur initiales,430 vérifié
GLM427 ciblé,1518 tokens déclarés ; port de la projection Go sur les observations strictes existantes.26 lectures/refus sans mutation dans la recette accepted maintenue, contre19 en424 ; quota inconnu/échéance UTC/pause/expiration/levée/données invalides/restauration.30 lecteurs428, Clippy/Rustdoc PASS ; normal429 sans fixture,323 entrées effectives archivées, non installé. Limite technique de date hors représentation refusée avec cause ; aucun quota ou budget levé. Planning avancé, automatic readiness, reviews/validation, cockpit et G1 global restent ouverts, baseline199/review96 inchangées. [Revue431](evidence/20261023-g1-status-contract/cooldown431/review-notes.md).

### G1 — activations du planning sans tâche,437 vérifié
Claim root→renouvellement→décisions vides→pauses/reprises→trois activations consommées→quatrième claim refusé :38 lectures/refus purs, sans Agent/modèle/SQL privé ni conducteur déduit du bail.30 lecteurs434, Clippy et Rustdoc PASS ; normal435 SHA4c58abce… sans fixture,323 entrées archivées, non installé. GLM432 avis3462 tokens ; ses propositions de refuser les lectures au plafond et d’admettre un état paused sans producteur sont écartées. RED433 et échec d’assertion436 conservés, code public budget_exhausted vérifié en437. Planning avec tâches/scopes multiples/closed, automatic readiness, reviews/validation, cockpit, G1 complet, baseline199/review96, R6/R7/RETEX restent ouverts. [Revue438](evidence/20261023-g1-status-contract/activation438/review-notes.md).

### Méthode active — campagne440, cinq GLM
Demande utilisateur maintenue pour les vagues de cette migration : cinq agents GLM internes en parallèle, contributions distinctes, intégration et vérification par Root. L’appel isolé439 n’était pas une vague de cinq ; ne plus le présenter ainsi.440 conserve les cinq requêtes, contextes, empreintes et réponses ; cinq simultanés vérifiés,20607 tokens déclarés. Distinguer préparés/lancés/terminés/intégrés et ne pas déduire d’économie nette Codex. Ce nombre ne crée pas une autorisation d’appels ou de consommation illimités. Suite de parcours442–444 qualifiée, voir revue445 ; baseline199/review96 et G1 global restent ouverts.

### G1 — premières tâches hiérarchiques,444 vérifié
Cinq GLM parallèles440 (cinq réponses complètes,20607 tokens déclarés), contributions distinctes ; garde taskless remplacée par garde todo sans Agents/attempts/review/recovery, bail inconditionnel et validation structurelle conservés. Les propositions de faiblesse bail/revue sont rejetées, pas de changement du lecteur de preuves. Recette accepted existante :43 lectures/refus purs, deux todo créés par decide public, première à configurer et dépendant en attente, pause/reprise sans lancement/acceptation. RED441 conservé ;30 lecteurs442, Clippy/Rustdoc PASS ; normal443 sans fixture SHA d4b0e203…,323 entrées archivées, non installé. [Revue445](evidence/20261023-g1-status-contract/todo445/review-notes.md). Tâches avec historique/planning multi-scopes/closed, automatic readiness, reviews/validation/cockpit, G1 global, baseline199/review96/R6/R7/RETEX restent ouverts.

### G1 — délégation multi-périmètres,450 vérifié
Cinq GLM parallèles446 (cinq réponses complètes,21066 tokens déclarés). Statut raccordé sur arbres ready/waiting, contrôle de chaque bail, validation structurelle existante inchangée. Recette maintenue :50 lectures/refus purs ; délégation root→child de req-1, claim enfant, deux todo/dépendance, pause/reprise, aucune authenticité/autorité/exécution/acceptation inventée. RED447 conservé ;31 lecteurs448, Clippy/Rustdoc PASS ; normal449 SHA aabbdbf2… sans fixture,323 entrées archivées, non installé. [Revue451](evidence/20261023-g1-status-contract/tree451/review-notes.md). Les suggestions GLM de modifier la clôture ne sont pas intégrées sans contre-exemple ni contrat Go. Historique planning, closed, automatic readiness, review/validation/cockpit/G1 global, baseline199/review96, R6/R7/RETEX restent ouverts.

### G1 — abandon et reprise hiérarchiques,458 vérifié
Vague cinq GLM452 simultanés : quatre réponses complètes, une incomplète préservée puis reprise ciblée ; six appels au total,21090 tokens déclarés. RED453 découvre le writer hiérarchique non porté. Nouveau chemin scoped worker initial todo/abandoned, settings explicites, transaction Root et garde native idle existante ; aucun élargissement des départs/acceptations. Recette458 :54 lectures/refus purs, abandon/reprise root et enfant, six refus (contrat/stale/start), deux rejeux sans réappliquer l’état ; consommation/ownership conservés.31 lecteurs455+9 lifecycle existants, Clippy/Rustdoc PASS ; normal456 SHA1e756ad7… sans fixture,323 entrées archivées, non installé.457 attente de rejeu corrigée selon le contrat Work courant, moteur inchangé. [Revue459](evidence/20261023-g1-status-contract/decision459/review-notes.md). Conflits natifs concurrents non nouvellement exécutés, historique d’exécution/revue, closed, automatic readiness/review/validation/cockpit/G1 global, baseline199/review96/R6/R7/RETEX restent ouverts.

## Essai GLM460 — huit appels autorisés

Huit appels simultanés terminés :6 complets/2 tronqués, sans reprise payante.
Usage et chevauchement mesurés dans [le bilan460](evidence/20261023-g1-status-contract/wave460/integration.md).
Essai de capacité uniquement ; aucune accélération ni économie démontrée.
Propositions relues contre missionDispatchState/profileFor ; faux diagnostics rejetés.
Préparation suivante : automatic readiness sur parcours public sans Agent/histoire ;
producteurs complets à raccorder avant toute suppression du refus actuel.
Normal456 et preuves459 conservés ; aucun code produit/install/restart dans cet essai.

### Reprise GLM461 — contexte réduit

Huit appels simultanés,8/8 réponses complètes sans retry ; [bilan et défauts écartés](evidence/20261023-g1-status-contract/wave461/integration.md).
Contextes ciblés62Ko ; usage et comparaison460 mesurés, aucune économie globale déduite.
Prototype de prédicat corrigé après erreur de compilation,7cas standalone PASS ; non intégré.
Prochaine frontière produit inchangée : automatic readiness et ses vrais producteurs, baseline normal456.

## Parcours automatic462 — intégration et recette

Méthode corrigée : huit GLM fournissent code/tests/docs ; Root intègre et vérifie.
RED466 : premières tâches planifiées, start public valide puis status refuse automatic.
463 chemin binaire de harnais incorrect,464 enveloppe budget incorrecte,465 organisation
requise au start : originaux conservés, préconditions corrigées ; aucun garde retiré.
Fichiers réservés : nouveau readiness, raccord historique/root_store, recette existante,
manifestes et docs/version FR/EN. Réglages existants ; pas de nouvelle politique.
Contrôles ciblés puis build normal et recette publique ; copies gérées/histoire ouvertes.
Baseline456 conservée, pas de livraison installée ; preuves462– en cours.

### Résultat automatic462–472

Disponibilité initiale raccordée, y compris premières tâches planifiées via start public.
44 tests lecteurs467/Clippy/Rustdoc PASS ; normal468 sans fixture (324entrées archivées).
Recette470 PASS :60lectures/refus purs, budget insuffisant/restauré et pause/reprise.
471 réexécute le même binaire après correction du nom d’indicateur du harnais.
[Revue bornée472](evidence/20261023-g1-status-contract/decision472/review-notes.md) ;
propositions GLM filtrées, erreurs de harnais et vrai RED466 conservés. Pas installé.
Copies gérées, histoire, revue/cockpit/G1 complet et R6/R7/RETEX restent ouverts.

### Statut automatique délégué —473–476
Huit GLM simultanés :7complets/1tronqué sans retry ; contributions corrigées par Root.
Recette existante paramétrée root/enfant, namespaces séparés et relecture publique
Work après chaque observation pour propriété/compteurs/absence d'exécution ou revue.
475 PASS66lectures/refus purs, dont6nouveaux enfant ; intégrité VERIFIED.
Normal468 inchangé,44lecteurs/statiques467 réutilisés sans prétendre les réexécuter.
Aucune fonction moteur ajoutée, ni installation/version/restart. [Revue476](evidence/20261023-g1-status-contract/decision476/review-notes.md).
Suite produit : copies gérées/histoire/revue/cockpit et G1 global ; baseline199,
review96,R6/R7/RETEX restent ouverts. Pas de pourcentage ou gain de tokens déduit.

### Parcours477 — rapport réel d’un worker délégué
Replan : copies gérées exigent enable/claim/decide repository non portés ; dépendance conservée.
Suite choisie : délégation publique→worker natif inerte→rapport soumis→statut sans acceptation.
RED478 prouve mission-status-planning-unported après vraie soumission, sans SQL privé.
479 raccord borné submitted/completed sans review/recovery, validation arbre/baux gardée.
Éligibilité vérifiée sur Agents complets avant fenêtre d’affichage ; window ne devient pas garde.
Réservés : reports/historical, recette accepted et manifestes, docs/version/plans FR/EN.
Huit GLM477 à contextes par rôle ; Root filtre et vérifie. Tests480 en cours, pas installé.
Baseline normale468 ; baseline199/review96/R6/R7/RETEX restent ouverts.

### Rapport délégué soumis —477–484 vérifié
Un parcours produit raccordé : délégation publique→worker local terminé→rapport réellement
soumis→statut « à examiner », sans acceptation. Garde sur historique complet avant affichage.
483 PASS73lectures/refus purs, dont7nouveaux : contrôle initial, non-soumis refusé,
soumis, refus/reprise de sortie, fenêtre masquée/restaurée avec coûts complets conservés.
46lecteurs481/Clippy/Rustdoc PASS ; normal482 SHA9dfc750c… sans fixture,324entrées archivées.
RED478 et fixture480 invalide conservés ;481 corrige uniquement sa propriété req-1.
8GLM477 simultanés,7complets/1tronqué sans retry,20 008tokens déclarés ; propositions filtrées,
contexte vide result-review diagnostiqué et remplacé par lecture source Root.
[Revue484](evidence/20261023-g1-status-contract/decision484/review-notes.md), pas installé.
Actifs/planning non soumis/accepted/review/recovery, copies gérées/closed/cockpit, G1 global,
baseline199/review96/R6/R7/RETEX restent ouverts ; aucun pourcentage/délai déduit.

### Replan485 — borne managed trop large détectée après484
La nouvelle garde repository non bornée aux Agents pourrait retirer une lecture
initiale metadata auparavant permise par supported. Le candidat484 reste historique ;
sa qualification ne clôt pas cette découverte. Propriétaire Root, test discriminant
485 contrôle todo valide→managed sans Agent ; correction générale prévue : refus
managed limité aux nouvelles lectures avec Agents. Aucun bootstrap/lancement managed
autorisé ; runtime public managed non établi. Nouvelle qualification requise.

### Qualification finale489 après correction du garde485
RED485 prouve le retrait indu d’une lecture de metadata initiale sans Agent.
486 restreint repository refusal aux historiques avec Agents ; nouveau test conservé.
47lecteurs486/Clippy/Rustdoc PASS ; normal487 sans fixture,324entrées archivées.
Recette488 PASS73lectures/refus purs, dont7nouveaux planned-worker, intégrité VERIFIED.
Sources/rapports484 et RED478/480/485 conservés ; candidat final non installé.
[Revue489](evidence/20261023-g1-status-contract/decision489/review-notes.md).
Un parcours produit raccordé,8GLM477/7complets/1tronqué/20 008tokens sans retry.
La lecture de metadata testée au garde pur ne prouve aucun bootstrap/worker managed.
Suite : états actifs/planning accepté/revue/reprise/closed/copies gérées/cockpit/G1 global ;
baseline199/review96,R6/R7/RETEX ouverts.

### Reprise490 — tentative déléguée préparée et réparation de fenêtre
8GLM490 simultanés/8complets, extraits par rôle vérifiés non vides avant HTTP.
RED491 rejoue recette active antérieure : Agent actif masqué devenu faux succès
après suppression du deuxième supported479. RED492 vrai enfant queued encore refusé.
493 sépare full supported de supported_view (Agent requis en running/blocked),
ajoute queued/running planifié seulement ; revue/reprise/managed/autres actifs refusés.
Réservés reports/historical, recette accepted existante, docs/version/plans FR/EN.
494 lecteurs/statiques en cours ; prochaine recette accepted + active existante
pour requalifier parcours nouveau et régression du lot antérieur. Pas installé.
Baseline487/489 conservée ; G1 global/baseline199/review96/R6/R7/RETEX ouverts.

### Tentative déléguée préparée —490–500 vérifié
8GLM490 simultanés/8complets/sans retry,16 824tokens déclarés ; contextes non vides
vérifiés avant HTTP. Queued/running planifié raccordé, sans identité vive ni départ
implicite ; rapport précoce non vérifié, réserve conservée, puis vraie soumission.
RED492 conservé. RED491 révèle régression479 : Agent actif masqué rendu manual/not_started.
Correction générale supported_view séparée de full supported, prouvée par recette active.
49lecteurs494/Clippy/Rustdoc PASS ;498 corrige une ligne Rustdoc, corps identiques.
Normal499 (324entrées) produit exactement le même binaire que495 : preuves496/497
réutilisées,77+8lectures/refus purs et intégrité VERIFIED, pas de rejeu neuf prétendu.
[Revue500](evidence/20261023-g1-status-contract/decision500/review-notes.md), non installé.
Planning avec vrai processus starting/running/stopping, retries/accepted/review/recovery,
managed/closed/cockpit/G1 global,baseline199/review96/R6/R7/RETEX restent ouverts.

### Campagnes continues — demande utilisateur après500
Enchaîner GLM vers migration complète ; ne pas terminer le besoin global sur un compteur
de tests ou une documentation. P01–P18/G1–G12 restent critères de clôture ; surface
présente n’est pas parité. Campagne501 regroupe cycle délégué actif/cancel/failed,
puis API/CLI, revue/validation, Git géré/données, qualification et RETEX.
Réservés501 : reports/historical, recette accepted, manifestes et docs/version FR/EN.
Sources Live/UX de Claude préservées. Bascule/install restent décisions séparées
à concrétiser après critères de sécurité/données/recette, pas à demander maintenant.

### Campagne501 — intégration du cycle réel504–
RED502 cancellation produit ; live/failed harnais publieur exclusif refusé.
503 corrige seulement helper fixture.publish, puis vrais refus live/failed reproduits.
504 planned_pair généralise les paires actives/failed/interrupted ordinaires,
managed/review/retry/accepted/closed et full/view guards conservés ; Rustdoc consommateurs
mis à jour, y compris commentaire périmé mission.rs (fichier réservé, doc seule).
Recette accepted enrichie et callback test facultatif run_observed pour réutiliser
les assertions route/usage/stdin/PID au lieu de les dupliquer.505 tests/statiques en cours.
Suite immédiatement prévue : lectures CLI/API manquantes après clôture du parcours.

### Cycle délégué501–509 — vérifié, migration poursuivie
Préparation→vrai processus local observé→soumission, annulation publique avant
départ et échec réel7 : nouveaux états raccordés par planned_pair, guards conservés.
50lecteurs505/Clippy/Rustdoc PASS ; normal506 sans fixture,324entrées effectives.
Recettes507/508 PASS82+8lectures/refus purs ; réserver released après annulation,
estimated après échec, aucune validation/acceptation fabriquée.
8GLM501/7complets/1tronqué sans retry ; harnais502 corrigé503, vrais RED conservés.
[Revue509](evidence/20261023-g1-status-contract/decision509/review-notes.md), non installé.
Vague510 déjà lancée sur agent logs historique (lecture/pagination/export exclusif).
G1 review/recovery/accepted/closed/managed/cockpit et P01–P18 restent ouverts.

### Reprise510–511 — historique CLI des journaux
8GLM510 suivis :7complets/1tronqué sans retry. Propositions sans outils, non acceptées
automatiquement ; plusieurs fausses API et confusion curseur0/négatif écartées.
Réservés : root_store/agent_log_history.rs, engine lib/root_store additifs, CLI
agent_logs.rs/agent.rs/lib.rs/Cargo.toml/lock, recette permanente, docs FR/EN/version.
Legacy vérifié : tableau JSON toujours, dernier200 pour0 ; premier200 pour négatif,
pagination avant pour positif ; export NDJSON create_new0600. Limite de réponse
protocolaire, aucune politique de rétention/lancement. RED public avant intégration.
Ne raccorde ni agent list (cockpit complet), ni agent history (bundle importé).

### Journaux510–524 — vérifié, suite520 en cours
8GLM510/7complets/1tronqué,25646tokens déclarés sansretry. Commande CLI logs
portée : protocole200, tableauJSON, NDJSONcreate_new0600, close avantpublication.
RED511/515/519/521 conservés ; causes/hypothèses harnais réparées516/521/522.
2lecteurs+2CLI+7causes517/Clippy/Rustdoc ; normal518 sansfixture, binaire
3bc709458e2e46a4ae7edaef031bab366f711a1a7ec2f12ed5035ce5732ebc15. Recette522 PASS23lectures/refus purs + vrai EFBIG/fichier
partiel conservé et reprise nouvelle destination + stdoutENOSPC/récupération.
[Revue524](evidence/20261023-g1-status-contract/decision524/review-notes.md).
Publication des sorties assistant en agent_logs non établie (4lignes lifecycle
observées) : suite concrète audit du producteur Go/Rust, propriétaire Root.
Vague5208GLM suivis,6complets/2tronqués : revue réelle→acceptation explicite→
fresh/stale, sans retrait de gardes non reliées. Global non terminé/non installé.

### Reprise520/523/525 — revue finale et acceptation
8GLM5206complets/2tronqués sansretry. RED523 : vrai reviewer passé et contrôles
natifs, status encore refusé. Réservés reports/historical/results Rustdoc, recette
review-status/docs/version ; fichiers Live/UX préservés. Relier fallthrough Go
passed puis acceptation publique explicite au lecteur freshness déjà porté.
Autres avis/reprise/Git/closed gardés refusés, migration globale toujours ouverte.

### Amélioration532 — éviter les copies de caches
Cause reproduite : shutil.move recopie target quand le parent destination manque.
Outil rename-only persistant +3tests (inode/refus existant/EXDEV) et recette.
Propriétaire Root ; aucun nettoyage ni gain chiffré. Prochain transfert réel
utilisera l’outil avec target inactif et preuve inode. FR/EN/méthode actualisées.
Version produit inchangée par cet outil de méthode, aucun serveur livré/redémarré.

### Replan549–554 — preuve antérieure et méthode corrigées
56lecteurs/7projections545, Rustdoc546/normal547 ; recette548 avis réels PASS10
lectures, active550 PASS8. Ancienne accepted549 attendaitreview1 sans contrôles
requis : Go impose review_blocked. Preuve publique553 contrôles réels→review1
sans acceptation. Actualisation554 du témoin antérieur, originaux conservés.
Driver552 corrige coordonnées relatives avant cwd enfant, recette dédiée PASS.
Qualification finale à relancer sur changement de motif statique, sans préjuger
parité globale. Prochain parcours produit : G3 conduite automatique validation/
reviewer, encore absents de watch ; ne pas confondre API opérateur et autorité auto.

### Reprise570 — huit GLM, raccord de handoff automatique (G3)
Besoin : watch transmet le vrai rapport du worker terminé, puis reste soumis aux
contrôles/reviewer à raccorder ; aucune acceptation implicite. Risque A.
Cause observée : reap ne soumet rien, agent terminé laisse sa tâche bloquée.
Choix : relais moteur sous bail et task.submit canonique, plutôt que requête
opérateur construite en CLI (ne prouve pas l’autorité automatique).
8 appels GLM570 textuels distincts : sélection rapport, relais sous bail, watch,
réglage v40 proposé, tests de sélection, parcours public, Admin, revue des gardes.
Root intègre ; fichiers proposés ne sont pas encore modifiés. Partagés réservés
additifs : engine_settings.rs/config/catalogue, lib/root_store exports, docs FR/EN,
version ; préserver les fichiers Live/UX de Claude. Aucun serveur redémarré.
Critère : vrai worker→rapport courant unique→soumis, ambigu/stale refusés, reprise
et réglage sauvegardé/réouvert réellement consommé ; controls/reviewer restent ouverts.
Étape préalable : recettes556/558/560/563 PASS retenues, revue consolidée à terminer.

### Replan570 — huit appels arrêtés sur DNS interne indisponible
8/8 lanceurs terminés en URLError errno-2, aucune réponse exploitable ni usage
reçu ; aucun retry. DNS public témoin valide, DNS SKYNET introuvable ; seule
connexion Wi-Fi gérée active. Preuves et tâches conservées dans
[evidence wave570](evidence/20261023-g1-status-contract/wave570/integration.md).
Propriétaire réseau utilisateur/administrateur ; Root reprendra les huit tâches
après restauration DNS/routage interne constatée. Aucun changement produit,
aucune installation, aucune modification du VPN/DNS partagé. G3 reste ouvert.

### Reprise571 — réseau SKYNET rétabli, huit appels demandés
DNS interne à nouveau résolu avant lancement, précondition570 modifiée.
Réutilisation exacte des huit tâches/contextes570 vérifiée par SHA256 ;
preuves neuves wave571, un appel par rôle sans retry automatique.
Root suit les résultats et vérifie les propositions ; aucune acceptation implicite.

### Replan572 — six HTTP500, deux réponses utiles ; essai à concurrence réduite
571 terminé :2réponses complètes/6HTTP500 ; métadonnées /models200.
Hypothèse capacité de route, non établie : reprendre seulement les6échecs
avec2appels simultanés et sorties572 neuves, sans boucle de retry.
Huit rôles conservés, tentatives supplémentaires et usage inconnu des500 tracés.

### Résultat571–573 — sept réponses complètes, revue GLM encore ouverte
DNS rétabli ;8appels571 →2complets/6HTTP500. Six reprises572 à concurrence2
→5complets/1tronqué ; revue573 fortement réduite →HTTP500 en appel unique.
Cause500 serveur inconnue : ne pas attribuer une limite fixe2 au service.
15tentatives,7réponses complètes,64416tokens déclarés ; usage500 inconnu.
[Relecture et décisions](evidence/20261023-g1-status-contract/wave572/integration.md),
[comptage](evidence/20261023-g1-status-contract/wave572/campaign-summary.json).
Aucun processus GLM restant ; aucun code proposé appliqué ni accepté.
Suite Root : raccord moteur canonique/rapport courant, politique persistée et
parcours public ; revue finale risqueA sur candidat, G3 toujours ouvert.

### Intégration574 — handoff sous bail, candidat G3
Root réserve mission_report_relay.rs nouveau, exports root_store/lib additifs,
mission_watch.rs, engine_settings.rs/config/catalogue v40, locale EN et artefacts
générés, tests/recette/docs FR/EN/version. Pas de fichiers Live/UX modifiés.
Sélection moteur dans le workspace réel ; marge persistée2s sur installation
neuve,0strict, absence historique refusée sans adoption. Fence transactionnel
revalide provenance originale/receipt terminal/consentement et rapport courant.
Candidat à tester : worker réel→soumis ; contrôles/reviewer auto toujours ouverts.

### Replan596 — indépendance des réglages, preuves publiques593/588 acquises
593 relais public PASS : vrai worker, reprise génération, ambiguïté refusée,
marge0refuse puis2soumet ; Admin5884parcours FR/EN, deux thèmes, sauvegarde/aide.
Revue595 constate un couplage inventé au groupe affichage : retiré596 ; groupe
partiel toujours invalide, absence indépendante permise, ancien skew absent
toujours refusé. Known Work waiting-error préservé par épilogue execute :
signalement595 #1 non démontré, paquet de revue incomplet. Profil final à refaire
pour ce changement source ; preuves antérieures conservées avec leur portée.

### Vague610 — huit GLM demandés, contrôles automatiques G3
Huit rôles : autorité, claim, registre, gate, sélection, watch, régressions,
parcours public. HTTP2simultanés ; pas de retry. GLM sans outils/dépôt.
Réservations proposées : task_validation/mission_authority.rs, runtime.rs,
mission_store.rs, process.rs, mission_validation.rs, CLIhook et tests ; Root
intègre exports/docs/version et préserve fichiers Live/UX de Claude.
Objectif observable : rapport soumis→vrais contrôles→receipt sans acceptation ;
consentement/bail revalidés avant permission, refus et reprise avec preuves.
Le chemin manuel ne doit pas acquérir implicitement l'autorité automatique.

### Relais574–601 — parcours local vérifié, G3 non clos
Moteur sous bail/provenance/consentement/receipt terminal ; rapport unique courant
→soumis, jamais accepté. Configv40persistée/Admin ;0strict,2défaut modifiable,
absence refusée ; aucun couplage affichage596. Planning.paused et known Work589.
6unitaires/9CLI596,Clippy/Rustdoc ; profil597 d3d17d9fcb8d617e937219153a5fbda87d2bd5b01d03c79e6af807464757a144.
Recette598 PASS :2workers réels, reprise watch, ambiguïté/refus0/reprise2,
Agent/dépenses inchangés, aucune soumission manuelle. Admin5884parcours,
ressources597servies et catalogue599 vérifiés ; UI inchangée réutilisée.
[Revue601](evidence/20261023-g1-status-contract/decision601/review-notes.md),
16inputs revus depuis518, y compris lecteurs520–560 (101lectures sur555).
Défauts originaux574/577/579/581/587/590 conservés, aucune installation.
Vague6107complets/1HTTP502 suivis ; suite contrôles automatiques sous vraie
autorité, reviewer, parallélisme, R6/R7/RETEX et anciens blocages restent ouverts.

### Replan620 — interfaces réelles après relecture610
610 :7réponses/1HTTP502, propositions non appliquées telles quelles : bail SQL
non possédé, causes effacées, filtre première validation inversé, limite session.
[Décisions610](evidence/20261023-g1-status-contract/wave610/integration.md).
Huit contributions620 bornées : autorité, binding, digest, claim, permission,
sélection, parcours ordinaire, politique fixture. Corps sources complets fournis ;
Root seul intègre, préserve manuel/LiveUX. Deux HTTP simultanés, pas de retry.
Critère inchangé : vrai rapport→contrôles sous bail/consentement→receipt sans
acceptation. Provenance, révocation et incertitude grant restent obligatoires.

### Candidat624 — contrôles mission raccordés, non qualifiés
Root intègre sous bail emprunté : binding/digest normalisés, writer permission,
gate enfant observe epoch/consentement/Root, sources/publication/review alignés.
Chemin manuel préservé ; aucune acceptation ni reviewer automatique. Watch appelle
la sélection après relais sans drapeau once/session. Trois tests SQL/digest ajoutés.
[Limites et reprise624](evidence/20261023-g1-status-contract/decision624/replan.md).
Source gelée pour compilation ; profil et parcours natif toujours requis.

### Vague630 — huit GLM sur le candidat réel626
6206complets/2tronqués, aucun retry ni code tronqué accepté.630 : autorité,
permission, receipt, manuel/recovery, sélection/watch, heartbeat, fixture, recette.
Contexte ciblé et interfaces réelles ; deux HTTP simultanés, Root intègre/vérifie.
Source626 gelée pendant compilation, propositions630 ne modifient aucun fichier.

### Vérification627–632 — métadonnées/CLI acquis, parcours candidat
6273tests digest/SQL PASS après ajout parent FK de fixture ; 6289tests CLI PASS.
Alerte heartbeat630 non démontrée : thread run_mission_cycle déjà indépendant.
Recette permanente mission-native-controls créée ; fixture contrôle autorisé
avant preflight sur vrai rapport, planning actif, aucun task.validate manuel.
Source finale631 à compiler/qualifier avant parcours ; révocation/reviewer et G3 ouverts.

### Vague640 — huit GLM, prochaine tranche reviewer
6301complet/3tronqués/4HTTP500 ; absence revue globale conservée. Pas de retry.
640 nouvelles tâches reviewer : politique, claim, binding, permission, budget,
publication, hook watch, fixture publique. Mappings bornés sur sources réelles,
aucune exécution GLM ni intégration reviewer avant contrôle du candidat courant.
Deux HTTP simultanés, huit rôles ; Root poursuit recette contrôles indépendamment.

### Replan637–638 — rejeu terminal distinct d'une permission neuve
635parcours natif contrôles PASS et intégrité vérifiée ; 636six parcours manuels
PASS.637reproduit refus du rejeu exact du receipt auto sans bail actif.638 limite
la garde permission aux vraies admissions ; replay reste original terminal vérifié,
actif/incertain refusé. Test public maintenu ajouté ; profil/recette finaux à refaire.

### Vague650 — huit revues bornées du candidat contrôles638
Nouvelles préconditions : parcours natif635 et six manuels636 PASS, défaut637
reproduit et638corrigé. Huit revues ciblées : writer, enfant, hash/consommateurs,
replay, watch/Work, politiques/doc/erreurs, assertions natives, heartbeat/stop.
Contexte producteur fourni, notamment vrai thread heartbeat ; deux HTTP
simultanés, pas de retry. Avis non assimilés à exécution ni acceptation globale.

### Recette646 — révocation en cours de contrôle, candidat
Fixture rendezvous choisie avant preflight, vrai peer noyau lié au slot permis.
Témoin même contrôle tenu puis libéré avant faute sur Root neuf : mission stop
public doit arrêter enfant réel, conserver claim/receipts/incertitude, aucun PASS
ni acceptation. Pas de watchdog global ni injection SQL d'autorité. Recette
mission-controls-revocation ajoutée, non exécutée à cette écriture.

### Amélioration656 — erreurs de lecture des preuves de fixture
Les deux nouveaux lecteurs SQL pouvaient masquer une erreur primaire par close.
Reader partagé read-only/nonblocking conserve causes indépendantes ; deux tests
avec témoin positif couvrent lecture+close et close après lecture valide. Recettes
contrôles/révocation mises à jour dans le même incrément, preuves antérieures
non relabellées. Aucun changement du moteur/binaire643 ni nouvelle politique.

### Contrôles621–659 — première tranche automatique vérifiée
Binaire64340bde36...,3tests métadonnées/9CLI/Clippy/Rustdoc ; quatre guardsPython
et recettes657/658 PASS+intégrité. Vrai rapport relayé→vrais contrôles→receipt,
replay exact sans nouveau bail/process ; révocation publique après permission
arrête enfant réel, wait/TERM/claim/incertitude conservés, aucun PASS ni acceptation.
Six parcours manuels636PASS sur634, portée conservée.659revue Root des10inputs
597→643, sources/preuves archivées ;6505revues GLM complètes/3tronquées sans
revue globale fabriquée. Suite : reviewer auto sous même budget/droits conservés,
foreground cancellation, autres G3 puis R6/R7/RETEX. Rien installé ni publié.

### Vague660 —24octobre, huit GLM pour raccorder le reviewer
659tranche contrôles vérifiée ; prochaine unité entière : reviewer réel après
contrôle sous bail/consentement/budget inchangés. Huit propositions sur autorité,
claim, source/guard, permission, process, runtime, sélection/watch, fixture.
Root réserve reviewer_native/{mission_authority nouveau,claim,guard,gate,process,
runtime,stale,mod}.rs et CLIwatch ; ajouts ciblés, hooks Live/UX de Claude
conservés, sources comparées avant éditions et gelées durant recettes. Deux
HTTP simultanés, pas de retry ; APIplanned clairement distinctes des existantes.

### Candidat661–662 — reviewer sous autorité mission, non vérifié
Root ajoute registre reviewer distinct et domaine source, claim/permission sous
bail réel ; gate/source courants, budget existant sans réinitialisation. Wrappers
manuels/SDK conservés ; sélection et hook watch après contrôles, sans autoaccept.
La tâche déjà possédée par review reste à ce lifecycle au lieu de tenter nouveau
contrôle Fresh. Sources à figer/compiler, recette native avant toute déclaration.

### Replan669/670→673 — doublon canonique et observation concurrente
669sur643 confirme event_conflict au relay d'un worker déjà soumis après échec
qualité ; pas de double écriture observée.673 utilise dernière soumission
canonique pour ne pas la répéter ; parser partagé refuse toujours incohérence.
670sur666 échoue avant review : BUSY Prepare d'une lecture Root pendant starting.
Fixture observe activité DB avant nouveau probe read-only ; aucun refus devenu
succès, aucun retry d'exécution. Test2étendu, un guard SQL ajouté. Recettes et
compilation à reprendre avec nouvelles préconditions ; reviewer natif non prouvé.

### Vague680 — huit revues du candidat compilé et corrections673
6607complets/1tronqué adaptés par Root, aucun pseudo helper accepté.680porte
canonicaldedup, writer, source/recovery, budget, observerBUSY, watch, fixture,
privatechild. Sources concrètes et limites666/669/670 transmises ; deux HTTP
simultanés, pas de retry. Aucune réussite native reviewer encore revendiquée.

### Replan676/677→683 — fence avant/après son propre stage
Les premiers parcours sur675 refusent leur propre task.submit dans le second
check précommit.683 reconnaît seulement intention exacte event/revision/path/hash
créée par la commande ; anciens rapports déjà livrés restent exclus au préflight.
Aucun fence de bail/provenance ni parser réduit. Nouvelle compilation/profile
puis parcours natif requis ; pas encore de succès reviewer revendiqué.

### Replan686/687→688 — première invocation reviewer réelle, refus citation
Reviewer inerte réellement exécuté ; pass sans citation refusé par parser.688
retourne citation exacte du material report et retient stdin complet/hash/privacy.
Pas de garde moteur réduit. Deux améliorations erreurs découvertes : sous-motif
statique protocole absent du rendu, revision17 du résultat masquée par snapshot15
du contrôle. Root doit corriger contrat général avec garde avant/après, puis
reprendre succès/rejeu/budget et refus. Aucun reviewer favorable encore qualifié.

### Vague700 — huit rôles GLM après premier parcours reviewer689
Skill personnelle skynet-agents ; huit missions de revue textuelle, deux appels
HTTP simultanés sans retry automatique. Périmètres : erreurs Work, confidentialité
du refus, négatif protocole, origine producteur, autorité planner (hypothèse),
readback, rapport après contrôle échoué, financement/replay. Sources et empreintes
dans wave700 ; Root seul intègre. Réservés Root : native_diagnostics.rs,
root_store.rs, tests des parcours concernés et documentation de migration.
690 reproduit deux défauts d'erreur ;689 parcours complet positif passe sur684.
Aucun résultat global ni installation/acceptation revendiqués.

### Replan690→692 — causes sûres et dernier commit connu
Deux reproductions compilées échouent comme attendu : motif quote-absent perdu,
revision connue9 masquée par snapshot5.692 ajoute projection du vocabulaire
ReviewReplyError et parcours typé de tous les enfants Root, avec cycles et
max seulement dans la première identité Work. Aucun rollback/état courant déduit.
Tests ciblés, matrice erreurs partagée, Clippy/Rustdoc/profil puis parcours requis.

### Recettes694/695 — refus réel et lecture historique reviewer
Fixture quote absente fixée avant preflight, témoin complet valide avant refus.
Assertions : motif sûr, dernière révision publiée, job/receipt réels,1réservation.
695 ajoute au positif le replay public exact de review.claim après sortie watch,
sans bail neuf, exécution ni financement supplémentaire. Résultats encore requis.
700 terminé :7complets/1tronqué,92584tokens déclarés, pas d'erreur HTTP ; les
avis sans reproducteur restent hypothèses. Réduction du report et déduplication
des erreurs par adresse seule rejetées (mauvais défaut/perte de causes typées).

### Vague720 — huit rôles ciblés pendant recettes693
Après700 : contexte réduit aux vraies fonctions, erreurs déjà corrigées fournies ;
8 missions (consent planner, origine manuelle, SIGINT, erreurs actuelles, négatif,
replay, recette manuelle, prochaine lacune verticale),2HTTP simultanés sans retry.
Root conserve intégration/recettes ; pas d'accès outils/dépôt au modèle.

### Tranche reviewer661–702 — parcours vertical local vérifié
693 e8f85e1... profil natif sans fixture ;696/697/698 PASS3étapes+intégrité,
699six parcours contrôles et701sept reviewer PASS. Réel planner/worker/rapport/
contrôles/reviewer favorable, replay exact après sortie sans appel/réserve neufs.
Citation absente réellement exécutée et refusée : motif statique conservé, dernière
revision connue17, job/receipt/financement uniques ;690deux RED puis692GREEN.
Rapport échoué non resoumis ; stop public termine contrôle réel sans PASS.
19cause-matrix/9CLI/2guards nouveaux/Clippy/Rustdoc, sources643→693 revues Root.
Aucun pourcentage ni acceptation globale. Suite : origine manuelle/parité,
consentement planner en cours et SIGINT foreground, puis autres G3/R6/R7/RETEX.
Vagues700/720 examinées séparément ; réponses textuelles ne prouvent pas tests.

### Replan703→704 — consentement du planner, précondition réelle
703 : stop puis resume refuse mission-consent-required, protection vérifiée ;
aucune preuve du défaut supposé, sortie originale conservée.704 remplace resume
par mission start public explicite, séquence de consentement différente, et tient
le premier vrai planner déjà permis pour tester la fraîcheur de son ancienne
autorité. Aucun SQLwrite ni bypass. Root : diagnostic et éventuelle correction.
702 archives et revue15inputs terminées ; metadata-proof-supplement lie vrai nom
review-binding.stdout (3tests) manquant dans la sélection initiale du collecteur.
7208réponses complètes/52413tokens déclarés, aucune facture/économie inventée.

### Vague740 — huit rôles, prochain refus réel reviewer
704 réfute l'anomalie planner supposée dans ce cas : stale_lease bloque ancienne
décision après stop/newstart. Pas de correctif de sécurité inventé.7408missions
préparent reviewer tenu/révocation, annulation API/process, origine manuelle,
budget négatif, reprise revue échouée, preuve planner ;2HTTP simultanés, sansretry.
Root réserve tests fixture/recipes/docs ; modèles ne modifient aucun fichier.

### Vérification709 — reviewer révoqué et hypothèse planner réfutée
705/707/708 PASS+intégrité sur693 : ancien planner refuse stale_lease après
stop/newstart ; vrai reviewer tenu puis révoqué termineTERM15/wait/receipt/claim
conservés sans succès ni nouvelle réserve ; réponse protocole invalide comparée
exactement à original_reply. Aucun correctif planner inventé, cause703préservée.
706ajoute variante fixture review_gate avantpreflight, aucune politique produit.
7406complets/2tronqués/45782tokens ; trois vagues700/720/740 =24appels,21complets,
3tronqués,190779tokens déclarés,0HTTPerror et0retry ; pas facture/économie inventée.
Suite Root : cadrer vraie parité arrêt/async foreground depuis code Go (watch
retourne nil surctx.Done, planner/reviewer goroutines) ; ne pas transformer
arbitrairement CtrlC en erreur ni promettre survie ou cancellation sans preuve.
Origine manuelle et autres G3/R6R7 restent suivis. Aucun restart/install/push.

###710 — raccorder la lecture publique et l'acceptation explicite au vrai watch
Suite du parcours, sans nouvelle feature : scénario automatique702 puis recette
status/acceptance existante avec vrai chemin report. Vérifie que hash de reviewer
lié mission est consommé par contrat d'acceptation et statut, pas seulement replay.
Tâche reste nonacceptée avant action explicite fixture ; aucun humain authentifié
ni acceptation globale produit. Résultat encore attendu, Root possède tests/docs.

### Vague760 — huit rôles, producteur manuel vers reviewer automatique
711 valide aussi status→acceptation explicitefixture→report stale/restauration.
760examine provenance déjàdurable (Journal possède directorytuple), capture
DBinstance si réellement absente, writer natif, adaptateur producteur, guards,
parcours public, paritéGo et arrêtasync. Aucun manualMissionLease inventé, aucun
upgradewriter aprèscoup. Deux HTTPsimultanés sansretry ; Root réserve futurs
agent_execution_store/provenance/reviewer adapter/tests/docs après diagnostic.

###711PASS et reprise712→714 producteur manuel
7113étapes PASS+integrité : automaticreview→status→explicitfixtureacceptance→
report stale/restauration, DBreadnonmutation/funding/process conservés.
712fixture missionstart level0 refusé avant watch ;714 prend vrais champs Agent.
Aucun déficit runtime manuel prouvé avant nouveau résultat ; pas de code changé.
7608réponses complètes ; rejeter sauts DBidentity optionnelle, mutation lateJournal,
manualMissionLease ou taskreview manuel proposé au lieu d'autoreview. Root garde
vrais garde physique et parcours watch. Preuve originale avant éventuelle715.

###716RED→718 — vraie parité origine manuelle du producteur
716 réunit worker manuel réel/rapport/contrôles/modeauto, watch refuse origine
auto obligatoire.718 ajoute provenanceDir+DB à permission avantgrant, gateenfant
refuse Root différent/absence, reader reviewermanuel vérifie provenance+receipt
terminal/currentattempt ; autoworker garde originbinding strict, reviewer reste
sous vraie missionlease. Capture dans writer existant, pas de policy ni backfill.
5sources+version réservées Root ; testsmetadata/profil puis parcours essentiels.

###Vague780 — huit revues bornées du vrai candidat718
Schema/permissionwriter/childgate/manualadapter/autocompat/fixture/refus/erreursdoc,
code effectif et sha fournis ;2HTTPsimultanés, aucunretry. Les tests purs de
métadonnées ne constituent ni exécution ni droits. Root intègre seulement défaut
reproduit, conserve Scope original et refuse backfill/histoireunknown.

###Replan718→721 — frontière de module privée
718échoue compilation avanttest, helpers Root pub(super) nonaccessibles frèrecrate.
721place provenance dans sous-arbreRoot, conserve helpervisibility plutôtqu'expose
SQL arbitraire ; callers crateinterne inchangés fonctionnellement. Anciennouveau
fichier déplacé, snapshot718 intact. Nouveau build/profil/parcours requis.

###723 — refus physique réel après témoin origine manuelle
Recette permanente mission-manual-origin : vrai manualworker→controls→autoreview
positif, puis Rootinactive fraîche, renameSQLite original gardé + copie octets
identiques nouveauinode mêmeDir sansSQLwrite. Watch doit refuser instance avant
claim/débit/reviewer ; garder copie refusée/restaurer original. Pas Rootlive ni
metadatafakepositive. Profil722 attendu après721compile ; origine718E0603gardée.

###Replan724→729 — absence de table après refus avant claim
724 vrai manuel autoreview positif et refusDBdifférent corrects ; assertionfixture
interrogeait table optionnelle noncréée.729 sqlite_schema absenceexplicite seulement
compte0, type/SQLerreurs refusés ; conserve positif dèsretour. Pascodeproduit changé,
rejoue nouveau729 puis régressions communes725–728 sur même722.

###730 — origine manuelle réelle localement vérifiée
7224e866801... sansfixture,3metadata/Clippy/Rustdoc.729manualworker→rapport
soumis/contrôles explicites→autoreview passe puis mêmeDir/nouveauSQLiteinode
refuse avantclaim/débit, Work/finances conservés, originalrestauré.725/726auto
origins PASS,727sixcontrols/728septreviewers manuels PASS, toutes intégrités.
6inputs693→722 revus Root, aucun DTOActor/snapshot changé ni backfill legacy.
Limite découverte : initialmanualcontrols toujours explicites dans cette preuve ;
prochaine unité doit raccorder ces contrôles au watch via même garde producteur,
pas prétendre parcours100%auto de cette variante. Rapport manuel soumis par API
reste action explicite. Autres G3/SIGINTasync/registered/web/R6R7/RETEX ouverts.

###731RED→732 — premier contrôle du worker manuel dans watch
731sans taskvalidate manuel reproduit refus avantclaim.732garde commune Origin
Requirement interne : relayerAutomaticWorker, contrôles/reviewerNativeWorker.
NativeManual exige tablepermission+receipt original+tentativecourante et mêmeDirDB ;
automatic garde admission originale. Tous exigent consentmode courant, writers
gardentlease réelle ; aucune nouvelle politique/flag public. Root réserve common
producer/relay/deuxauthority/Rootmod/tests/docs pour parcours entier.

###Vague800 — huit rôles, garde commune des trois phases
Relaisauto compat/nativeorigin/controllease/reviewlease/fixture/GoParité/erreurs/
politiquedoc, contexte réduit et code732effectif.2HTTPsimultanés sansretry.
Vérification mission_732 élargie à famille : gardeprovenance critique maintenant
consommée par relais/validation/review, motifs et métadonnées d'autorité liés ;
aucune suite complèteworkspace ni recette identique répétée sansnouvellecause.

###Replan732→734 — helper de cause déplacé avec le contrat
732E0425avanttest, wrapperprivé caused manquant après extraction.734copie exact
arbretypémission, pas flatten/messageparse ; gardes unchanged. Compiler/profil
+rejeu731 et contrôles métier restent requis.

###Replan734→736 — schéma historique des tests runtime
Famillemission_734 :153PASS/2FAIL/1ignored, versionsfixtures37/38 périmées car
familles39/40 encoreprésentes.736projette réelcatalogue vers37 et suit version
des nouvellesvaleurs ; seuilsfixture explicites au lieu defaultopérateur copié.
Gardepreview/history/absence/refus/zero intacte, pascorrectifproduit. Contrôle
2tests ciblés puis familleprécédente sansrépéter sous-lotsfonctionnels inconnus.

###738 — transmission de méthode GLM et extraction
Skill personnelle skynet-agents et copie Dremio existante actualisées : transmettre
les vraies entrées None/Some et les déclarations mod/cfg/helpers, distinguer
producteurmanuel/commandeauto. Preuves before/after738. Cela ne modifie ni requêtes
GLM déjàenvoyées ni code contexte embarqué ; prochainscontextes doivent relire.
Erreurs718/732conservées, pas visibilitéhelpers élargie.

###739 — recette entière avec premiers contrôles réellement automatiques
Nouveau manifeste mission-manual-automatic-controls : worker manuel réel puis
submit explicite, watch admet contrôles natifs et reviewer dans l'ordre, vérifie
receipt producer/report, aucune commande taskvalidate manuelle. Positifpuis
copieinactiveSQLite autreinode refuse avantclaimeffetfinancier. Profils737/
sourcescommunesconservés, gardeoptionalabsenceSQL jamaiscatchallfallback.

###743 — parcours manuel sans contrôle manuel localement vérifié
7375439747... sansfixture ;739manualworker→submit→watchcontrols→reviewer,
receipt etordre réels, copieSQLite refuse avantclaim/debit, Work/financement
conservés (bailsupervisionSQL légitime distinct).740/741/7423étapesPASS+verify.
155missiontestsPASS+1ignoredledgerfixture,2guardsruntime ciblés/Clippy/Rustdoc.
7342fixturesversions périmées conservées ;736 ancien37 construit parcatalogue,
seuilsfixture explicites, pas de règleproduit changée.6inputs722→737revusRoot.
R7préparation suivante identifiée : build.sh et swarm_version dépendent encore
deGo, binaireRust versionnonporté/identitydevelunknown ; préparerbuild/version
natifs sans basculeinstallation/restart ni droits supplémentaires.

###744→820 — préparation R7 version/build natifs (sans bascule)
744versionRust/JSON refusecommand_not_ported sansRootmutation ; build.sh et
swarm_version utilisentencoreGo. HuitpropositionsGLM bornées : compiledidentity,
CLIentrée, runtimeconsumer, nativebuild, profil/metadata, JSONdiagnostic, recette,
erreurspolicy.2HTTPsimultanés sansretry. Root réserve nouveau version_identity,
CLIversion, runtimeversion ettoolbuild-native/qualifier ; pas de serveur/install
ni modificationdéfautlauncher. Cible : binaireportable vérifie ses vraisfaits
compilés/coldstart, aucun commit/tag imaginé ; --server/policy/HTTP/UI restent
ouvertstantquenonvérifiés. Le cadrage ne prouve pas livraison ni R7 complet.

###840 — reprise R7 et revue de l'identité compilée
Racine reprend version_identity/CLIversion/runtimeversion/profil natif ; nouvelle
vague de8GLM texte,2HTTPsimultanés,sansretry. Décodeur etconsommateur modifiés,
nonencorecompilés : aucune livraison revendiquée. Critère : version/--version
JSON ettexte sanscréationRoot,metadata compilée immuable auxenvruntime,erreurs
conservées. Build natif explicite puisprofil ; Go/defaultlauncher inchangés.

###842–860 — assemblage version natif et revue du build explicite
842 trois tests identité PASS ;843 profil natif sansfixtures qualifié (334entrées).
CLI version/alias/texte raccordés ; diagnostic runtime emploie le comparateur
commun. Tool build_native explicite capture Gitréel/date, profil etconsommation
metadata ; pasencoreexécuté. Vague860 :8rôles/2HTTP/sansretry, revuecontrats/
Rootpolicy/build/preuves/reprise. Go/défautlauncher/serveur restentinchangés.

###846–848 — build natif réel et correction de la recette
846 build_native qualifié : Git553593a2… modifié, métadonnées réellement
consommées malgré envruntime falsifiée, profil110artefacts/334entrées, sansGo
niinstallation.847 Rootconfiguré/corrompu/restauré vérifiés ; dernièreassertion
ENOSPC fausse : attendaitcommand_failed, parcours réel classecorrectement
storage_unavailable/StorageFull/errno28. Cause : recette ne lisaitpas priorité
diagnosticcode. Correction attend le vrai contrat, garde échec847 ; reprise848
ne change pas le produit.2testsmetadataPython et3testsRust PASS ; pasacceptation
globale. Vague8608réponsescomplètes, revue limitée auxsourcesfournies.

###880 — vague suivante : clôture bornée version et reprise G3/R6
8rôles GLM texte/2HTTP/sansretry : commentaires et événementswatch, SIGINT
reproducteur/contrat, revue recetteversion/build, inventaire R6 etconsumerR7.
848 recetteversion PASS2étapes/intégrité ; inclut12assertions,Rootpersisté,
refuscorruption/reprise etENOSPC.849 contrôlesmoteur/Clippy/Rustdoc encours,
aucune clôtureavant résultats. Les propositions SIGINT/R6 ne sont pas exécutées.

###849→850 — test runtime corrigé pour le profil injecté
8495/6runtimePASS,1FAIL : ancien test exigeait commitnull même avecmetadata
injectée ; le binaire846 retournait légitimementlecommitcompilé. Originepreuve
conservée ; test compare désormais au vrai binary_version() plutôtqu’audefault
inconditionnel. Troisgardesdécodeur et843plainCargo couvrentencoreunknown.
AideCLI FR/EN expose version etalias ; aucuneactivationserveur/install.
Reprise850 des contrôles ciblés après cette précondition corrigée.

###850→851 — documentation extraite et contrat de recette
8506runtime+19diagnostics+3identitéPASS ;Clippy refuse un commentaireSemVer
resté àl'ancienemplacement, désormais supprimé. Rustdocwatch corrige ancienne
affirmationreviewer/contrôlesabsents ; flagsphase préservés avec leur consommateur
réel, sans inventercapabilitéglobale. DocCLI distingue nominalRO/journalrecovery.
8testsarchive/profilPASS, manifestcache hérité promettait3 : compteur misà8
etgardesconfig explicites. Nouvellequalificationàfaire, aucunéchec effacé.

###851 — portée du contrôle Clippy
La sortie complète850 contient18diagnostics :1commentaire déplacé corrigé et
17styles de tests préexistants (insights/tasks/spending/adoption/historical/
reports/native_diagnostics). --all-targets n'était pas la portée de qualification
précédente ; le contrôle de production lib+bins reste à exécuter. Échec global
all-targets conservé/ouvert, aucun PASS global revendiqué ; nettoyage de ces
fixtures propriétaireRoot avant qualification globale, sans bloquer le parcours
version par une refonte cosmétique des autres lots. Fonctions testées inchangées
horscontrat identité déjà revérifié850 ; nouveau build doit inclure aides/commentaires.

###900 — vague de8 pour supprimer la dette de vérification observée
HuitGLM/2HTTP/sansretry : sixpropositions sur les17styles tests réellement
refusés850, revuecontratlectureversion ettransmissionméthode. Root intègre
après851terminé, préservevaleurs/assertions etrefuse toute copie sanscontexte.
Permet contrôler all-targets sans masquer l'échec ni ajouter deallowglobal.

###902 — correction bornée des17styles test observés
8516runtime+19diagnostics+3identitéPASS, Clippyproduction etRustdocPASS.
Root intègre13initialisations de fixtures équivalentes,3expect_err et1slice
borrow ; aucune assertion ni valeur changée. GLMreports proposaitencoreune
réaffectationDefault, GLMtasks ne corrigeaitpasclone : propositions rejetées,
correction liée auxsourcesréelles. Fichiers : insights/spending/tasks tests,
worker_adoption tests, historical/reports tests etnative_diagnostics tests.
Validation ciblée decesfamilles puisClippyall-targets avantqualificationfinale.

###903 — correction de l’édition et du chaînage902
902scriptrefuseavanttouteédition parcequehistoricalfixtureancienne n'a pas
espacesstandard ; mes commandes suivantes avaient malgréce refus démarréle
contrôle. Groupevérification1333419 identifié/interrompu, originaux conservés
avecinterruption.json ; aucunPASS.903relectureoctets exacts, substitution
explicitehistorical, autres13sitescontrôlésavantécritureatomique. Le script
903réussit ; prochaineétape lancée dansappel séparé, uniquementaprès réussite.
Les statementsde902étaientciblesannoncées, pasréalisationeffectiveavant903.

###920 — prochainevague de8 : premier R6 en fixture froide
Root réserve tools/rust-migration/cutover_snapshot.py, tests/test_cutover_snapshot.py,
tests/rust_cutover_restore.py etrecettecutover-fixture-restore.json. Helpernouveau
inventorie tousfichiersréguliers/modes et répertoires d'une fixture inactive,
refuseWAL/SHM/liens/spéciaux,restaureversdestinationneuve uniquement. Pasoutil
backupactif ni droittransféré ; sourcesoriginalesconservées.8GLM/2HTTP/sansretry
revuepath/drift/publication/erreurs/tests/parcourspublic/autorité/inventaire.
Cible observable : Rootpublic→snapshot→mutationpubliqueRust→restaurationneuve→
égalitédonnées/configs +refus/reprise. R6Go/anciensschémas/storesliveglobauxouverts.

###921→922 — défaut manifeste reproduit avantcorrection
9216gardes exécutées :5PASS/1FAIL, verify suivait un symlinkmanifest valide
malgré contratliensrefusés. Root trouvececontreexempleque8revues920n'ontpas
identifié ; leurs avis ne prouventpasabsence de défaut.922lstat propriétaire/
fichierrégulier avantlecture, causerefus statique. Succèsrestore retire seulement
sonconteneurstagingvide crééici ; erreurs gardent staging/destinationobservables.
Sources originalespréservées ; reprise gardes ne supprimepaspreuve921.

###940 — vague suivante de8 : reprise du conducteur Rust
909preuves version/build/R6froid liéesaux12entréesmodifiées ;907/908 conducteur
consolidéPASS905. RepriseG3 signaux pendantrôleforeground : Go lancerreviewer/
planner async ; RustbloqueencoreconsommationSIGINTjusqu’auretour. HuitGLM/2HTTP
sansretry : contratGo, contreexemple strace, poll/observer,planner/reviewer/
contrôles,teardown etpreuve. Heartbeatdéjàindépendant : nepasajoutersecondtimer.
Root réserve nouveau testSIGINT, CLIwatch etextension commune éventuelle après
contreexemple ; aucuneAPIdefinitive ni cancellation==paritéGo revendiquée.

###941–943 — défaut SIGINT reproduit puis correctif planner enqualification
941témoinplannertracéPASS puisSIGINT reçu/rt_sigreturn/ppollréentré alorsque
providerretenu ; décisionappliquée aprèslibération, arrêtseulementensuite.
Sourcecoopérative locale942, scopeRootrestaurémêmeaprèsunwind, secondwakefd
danspoll sansremplacerlesdeadline/timerexistants.943listenerTokio dédiéack
avantbail, aucunsecondheartbeat/timer ; planner TERM/grâce/wait/failedreceipt/
feedbackexistants. Stop0 seulementRefusedexactdansWorkRecordedsanscleanup
+signalréel ; autreséchecs restent2, budgets/calls nonremboursés. SIGINTnepause
pasmission etnetouchepasworkersindépendants. Controls/reviewer etasyncGo
restentouverts.4gardeslocales/nestedscope prévues puiscontreexemplepublic941
surprofilréel ; implémentationnonencorecompilée, aucune protection acquise.

###960 — revue du correctifplanner par8GLM
8rôles/2HTTP/sansretry sursourcewake,scopeRoot,poll2FDs,plannerstop,listener
startup/join,stop0strict,recettepublique etpolicies/docs. Source944gelée pendant
compilation ; avisGLMsansoutils ne vautpasexécution. BibliothèqueSourceNone
par défaut ; le seulcalleractive estCLIwatch, aucun grant/cadreagenttransmis.

###946→947 — corriger la preuve de réactivité et garder toutes les causes
9444gardes/9watchtests PASS ;945profilnatif335entrées qualifié.946stop0 et
mission-planning-interrupted observés,sansdécision, maisancienne sonde concluait
déféréausimple ppollaprèsSIGINT. Ce prédicatneprouvepasblocagequandunthread
IOpeutencoreréveillerpoll. Reprise attendmaintenantwatchpidfd sanslibérerprovider,
puisvérifie TERM/reçufailed/event. Échec946conservé ; sourcesonde/hash désormais
sauvées avantfixture. UnitéFD vérifieCLOEXEC/nonblocking réels.
Planner : stop_kind n'estplusperdu siéchecprocess/close indépendant ; gateRefused
estjointmêmeavecstop, cequiinterditstop0surrefusdegate. Gardepure nouvelle,
aucun rembourssement/permission implicite. Contrôles/reviewer async restentouverts.

###947–948 — preuvepublique sanslibération etgardescomplémentaires
947PASS sur945 : conducteur sort0 pendantproviderretenu, TERM/reçufailed/grant
réel/feedback vérifiés,sansdecisionappliquée. CePASS validecorrectif942/943 et
sonde corrigée, pasencorehelpercauses/FDtestsajoutésaprès945.948doitqualifier
cesoctets : causesprocess/stop/gatejointes, CLOEXEC/nonblocking réels,6variantes
runner(first/secondwake ×valid/HUP/reprise) etlistenerstartup/wait/shutdown.
Sonde maintenue choisitSIGINTouSIGTERM, témoinavantchaqueessai. Paswatchdog
ni nouveau délai ; grâce/cadence restentpersistées. Autresrôles/asyncGo ouverts.

###970→971 — traiter aussi le panic du listener
9486gardes/10watchtestsPASS et949profil335 qualifié. Extractionduvraihelper
listener sanschangementdecomportement, puis test970FAIL : un unwind nepassait
pasdansbrancheErr etne réveillaitpas lerôle.971catchauproducteurcommun,
payloadoriginalretenuopaquement, wakeListenerFailure puisretourerreur ; jamais
stop0pourcetincident. Le panic hookduprocessus resteindépendant. Témoinhealthy
précèdeinjection ; aucuneaugmentationbudget/timeout/retry.

###980 — vague suivante : arrêtcontrôles/reviewer
8GLM/2HTTP/sansretry : propositionscodewakesreviewer/contrôles, marqueurtyped
postpublicationfailedreview, causesgate/stop, sondepubliquerôles,listener et
budget/reasons. Root réserve leursprocess/runtime/journal/failure etCLIwatch
déjàpossédés, aucunezoneClaudeLive. Même sourceScope locale/budgets inchangés.
Stop0n’estni verdictfavorable niacceptation ; touteerreurindépendante interdit
lemarqueurclean. Preuveavantàproduire sur949, puiscode etrecettepublique.

###982–986 — extension aux contrôles/reviewer et refus de gate
982/983 sur949 : témoinsvalides puis SIGINTdéféré pendantreviewer/contrôles
retenus, sanswatchdog ; financeettracesréelles conservées.984gardeFAIL : refus
degate absentduprimary, mêmesiReceipt.gate retenaitl'information.985fusion
fatalegate/source/IOaucune causeeffacée ; retourJournalObservationnonserialisé
qualifie transportlocal seulement. Marqueurprivécrééaprèspublicationfailed
résolue, garde/IO/deadline/capture empêchentclean. 986CLI consommemétadonnées
+signalréel,stop0duconducteurneprouveaucunsuccèsderôle/tâche. Reviewerrorreason
distingue execution-interrupted/source-revoked. Aucunbudgetremboursé, source
locale nontransmiseauxworkersindépendants. Implémentation àcompiler/recetter ;
fullGoasync/registered/managed/web restentouverts. Plusieurspropositions980
(typeOptionExecutionInterrupt/signature &mutResult/Rootenum) rejetées ; vrais
traits/typesetconsommateurspriment. Tests marquage/causes/roles àqualifier.

###989–992 — arrêtsréels et classification conservatrice
989reviewer/990contrôles PASS sur988 : sortie0sanslibérermodèle/commande,
TERM/wait/failedoriginal/publication etfinancements conservés, consentement
nonmodifié.991–992 qualification suivante : exit/signalindépendant exclutclean
(vocabulaireOSduvrai wait, passtdout), replyreviewer déjàprésent exclutmarqueur
plutôtquemasquerunrefusdeformat/quote. Observations live nonserialisées ;
5variantsgate/deadline/drain/IO/exit testentlevraiobservercontrôles.
SourceLocalStopetadata n'estpasdroits/authentification ; asyncGobrancheouverte.

###1000 — nouvelle vague avec contrat d’API complet
8GLM/2HTTP/sansretry : revue des vrais consumers etpreuves.SourceAPI fixée
verbatim danschaqueprompt : aucun typeOptionExecutionInterrupt, aucun&mutResult
pourwake, aucuneCloneRootError/newRootenum. Cause980contextemissing+codeinventé
conservée, propositionsrejetées avantintégration. Cechangementdeprécondition
visejustesse despropositions, pas un gain detokensprouvé. Qualificationfinale
desstatuts/réponses/roles etSIGTERM encoreàfaire.

###993 — consolidation du lot arrêtforeground
Sondevérifiecontrolshealthyjusqu'aumarkerpostphaseblocked avantcleanup : nepas
interrompreaccidentellementle reviewerautomatique suivant pour jugerletémoin.
Les6essaisfinaux doiventcouvrir3rôles×2signaux, témoinréelavantchaqueinjection.
Budgetcallsexacts/consent inchangé/TERMwait/reçufailed etsortie0duwatch requis ;
notiondecleannevautpassuccèsderôle. GardeSource exit42/SEGV etexclusion reply
reviewer déjàdisponible àcompiler. Propriétépréséancedeadline/signallocal non
exercée enconcurrencesimultanée : hypothèseconsignée, aucunegarantie globale.

###1001–1002 — reprise : conserver la cause malgré un échec stdout
1001 reproduit une perte du Work/cause dans interrupted_phase quand emit échoue.
Correction Root : joindre l'erreur de sortie à l'erreur originale ; aucune
politique ni schéma modifié. Test témoin puis write/flush BrokenPipe, qualification
1002 à exécuter. Fichiers réservés CLI mission_watch + documentation de ce lot.
Vague suivante : huit GLM textuels, deux HTTP simultanés, sans retry ni watchdog ;
préparer la recette signaux et la continuation sans modifier Live/UX Claude.

###1004 — huit GLM : recette signaux et prochain parcours continuation
Huit rôles textuels préparés sur les vrais producteurs/consommateurs ; deux
requêtes simultanées, aucun retry/watchdog. Root intègre et vérifie. Trois rôles
préparent le parcours reprise de workers déjà enregistrés, sans nouveau droit
sur leurs processus ; les autres examinent recette/causes/poll/publications.
Les propositions ne constituent ni tests exécutés ni acceptation indépendante.

###1006 — recette permanente d’arrêt foreground
1002 :12 tests watch PASS, perte de cause/Work sur write et flush corrigée.
Recette mission-foreground-signals : six couples rôle/signal, témoin valide avant
chaque injection, reçu durable/TERM/wait/consentement vérifiés par la sonde.
Proposition GLM signal-matrix adaptée : suppression des preuves et contrôle de
seule existence result.json rejetés ; assertions de contenu, conservation intégrale.
Qualification du binaire1005 puis recette ; aucun redémarrage/installation.

###1009 — arrêt foreground vérifié, suite reprise de worker actif
1005 profil335 sans fixture ;1006 six couples rôle/signal PASS + intégrité,
1007 contrôles/reviewer et refus quote invalide PASS + intégrité ;1002 douze
tests watch,1003 Clippy tous targets/Rustdoc PASS. Cause1001 conservée et corrigée.
1004 :8 appels GLM,5 réponses complètes/3 length (aucun retry). Rejet de la
suppression de preuves proposée ; allégation busy-loop non établie car vrais
consumers désarment le FD après stop, absent du contexte du relecteur.
Reprise registered : parcours à reproduire, pas encore porté. G3 global ouvert.

###1010 — vague suivante : parcours reprise d’un worker enregistré
Huit GLM / deux HTTP simultanés, sans retry : proposition CLI, fixture publique,
preuves registered, budgets/turns, recette signaux, vrais désarmements FD,
propagation stdout et critères du parcours. Contexte découpé aux frontières
réelles pour traiter les trois réponses1004 tronquées sans augmenter leur budget.
Root réserve mission_watch.rs, fixture watch_public et nouvelle recette reprise ;
aucune zone Live/UX Claude modifiée. Reproduire avant de supprimer le refus.

###1011–1013 — précondition corrigée avant preuve registered
1011 FAIL dans témoin valide : vrai agent.show=starting malgré permission_recorded
et peer natif déjà entré ; publication parent pas encore finalisée. Réutilisation
de worker.wait_running, déjà gardé par deux tests déterministes, sans délai/retry.
Le callback de fixture conserve le worker après sortie du premier watch puis
exerce le second ; erreurs restent dans la cleanup possédée. Correction du nom
d’événement de report planning d’après son producteur (watch-blocked). Sources
1011 conservées ; nouvelle exécution1013 avant tout changement de garde produit.

###1013–1014 — parcours registered reproduit puis garde ajustée
1013 : témoin réel PASS, second watch refuse existing-registration-not-adopted
pendant que le vrai worker reste actif. Garde CLI : registered observable sans
handle enfant possédé, pas candidat à adoption, planning différé ; refus queued
et Refused inchangés. Aucun accès process ou politique ajouté. Tests/docstrings
et notes FR/EN du candidat actualisés ; recette publique après build à exécuter.
1010 : huit GLM, sept complets/un length. Propositions inventant table grants,
row['state'] sur tuple ou promotion de registered en known_child rejetées ;
les contrats réels et la preuve1013 gouvernent l’intégration.

###1017–1018 — sélection réelle des tests de continuation
1017 filtre mission_worker_continuation::tests sélectionne zéro test : qualification
refusée malgré exit0 ; sortie originale conservée. Module réel déclaré dans
mission_supervision_store sous worker_continuation. Corriger le filtre après
--list et exiger les huit noms réellement exécutés, sans modifier le produit.
Clippy CLI1017 PASS ; Rustdoc en cours. La recette publique1016 est PASS et
indépendante de cette précondition manquante, sans clôture globale G3.

###1020 — troisième vague : revue du parcours et assemblage asynchrone suivant
Huit GLM, deux HTTP simultanés, sans retry : revue garde/fixture/ressources/docs/
recette et préparation des frontières asynchrones planner/contrôles/reviewer.
Contexte Python extrait par AST et frontières de symboles Rust, plutôt que lignes
numérotées susceptibles de manquer le consumer (défaut de méthode1010).
1018 : huit vrais tests de continuation PASS ;1017 zéro sélection ne vaut pas PASS.
Recette1016 PASS et intégrité VERIFIED ; aucun nouveau droit process/installation.

###1021–1023 — recette : fermer aussi les ressources partielles
1021 reproduit notify laissé ouvert après refus du second spool ;1022 reproduit
aussi notre Popen enfant laissé actif après refus pidfd. Témoins réels descripteurs
et enfant Python inerte avant injection ; pas de fournisseur ni nouvelle autorité.
Scope de cleanup placé avant allocation : fermeture des ressources déjà ouvertes ;
si pidfd échoue, signal/wait uniquement via notre handle Popen enfant non reaped.
Erreurs originales conservées. Tests permanents et recette à rejouer1023.

###1024–1025 — corriger la précondition des injections de ressources
1023 échoue : double owned incomplet (environment absent), donc lancement/pidfd
non atteints ;1022 ne prouve pas encore le défaut pidfd. Sources/échecs conservés.
Ajouter la vraie entrée environment au double, puis réexécuter le code original
archivé1022 avant le code corrigé : témoins d’allocation et erreurs visées réels.
Ce replan retire l’inférence précédente, sans réécrire les preuves originales.

1025 révèle un second attribut manquant du double, root : pas de nouveau défaut
produit, pas de preuve pidfd valide. Réduction au contrat exact de observe :
owned.ok/work/environment/root ; témoins et deux refus ciblés doivent être seuls
en échec sur l’original avant qualification du correctif. Aucune borne relevée.

###1031 — trois vagues intégrées et preuves conservées
24 appels GLM internes :19 complets/5 tronqués,166226 tokens totaux déclarés
(entrées/sorties/raisonnement, pas économie Codex),2 HTTP simultanés,0 retry.
Arrêt foreground six couples rôle/signal1006 vérifié ; observation registered
1030 sur1029 :4 étapes PASS + intégrité.12 tests watch et8 continuation réels
PASS ;2 gardes ressources corrigées vérifiées dans recette. Clippy/Rustdoc CLI
PASS ; build335 entrées sans fixture. Aucune installation, commit ou restart.
Suite G3 : assembler les appels de rôles en ordonnanceur asynchrone complet ;
Go missionCycleWork lance planner/reviewer hors boucle, Rust drive les attend.
Réutiliser la vérification peer/lease de mission_cycle_runner, conserver une
seule heartbeat, les claims/budgets, la fraîcheur et les erreurs de publication.
Les analyses1020 signalent les contrats encore à fournir ; aucune proposition
thread/Root non liée à l’instance ni acceptation globale n’est intégrée.

###1040 — reprise GLM : planner asynchrone dans watch
État1031 préservé, source directe inchangée ;8 GLM/2HTTP/sansretry, propositions
peer Root lié au bail, job scoped, erreurs/output, source shutdown, recette peer
retenu, tests de bail et concurrence. Root réserve mission_watch.rs, nouveau
module CLI de job si nécessaire, mission_supervision_store.rs, execution_interrupt
et leurs tests/docs. Live/UX Claude et launcher inchangés. Option A : planner
scoped avec Root peer et join avant fermeture ; choisie pour parcourir un appel
modèle retenu sans bloquer dispatch. Option B : sous-processus nouveau protocole
de rôle différé (surface/claims/autorité supplémentaires). D’abord planner ;
contrôles/reviewer asynchrones et parité Go globale ne sont pas déclarés livrés.

###1041 — peer lié et arrêt local sans signal inventé
Nouveau with_mission_peer : writable, binding+snapshot epoch/heartbeat, rollback
avant callback et fermeture avec causes. Pas d’opération dans un read transaction
ni Root readonly (propositions GLM rejetées), aucune nouvelle heartbeat. Source
ControllerShutdown locale distincte ; quatre codes techniques privés, pas politique.
Deux tests fixtures publiques/scoped thread et refus wrongRoot/stopped à exécuter.

###1042 — assemblage du planner asynchrone
Job scoped ouvre son propre Root, observation registered revérifiée, écrit
uniquement un buffer JSONL sûr ; main seul publie et conserve Work/erreurs.
Dispatch/observation continuent pendant l’appel ; contrôles/reviewer restent
ordonnés foreground quand le job planner est absent. Join avant bail/Root close ;
stop réel ou ControllerShutdown si erreur main, aucun worker indépendant signalé.
Deux propositions GLM1040 tronquées ; six complètes, codes inventés rejetés.
Compilation/tests ciblés puis vraie sonde provider retenu à exécuter.

###1050 — revue du code réellement assemblé
Huit GLM/2HTTP/0retry : scope/join, peerfence, output/Work, shutdown, concurrence
claim/dispatch, vraie sonde cadence et ordre rôles/doc. Producteurs/consommateurs
réels fournis ; aucune API proposée1040 n’est considérée existante sans source.
1041 deux tests peer et1042 treize tests CLI/job PASS ; warning champ opaque
renommé explicitement _payload sans suppression du payload. Profil1043 en cours.

###1044–1051 — preuve publique et retour de revue
1044 planner retenu : témoin valide, main timedwait après entrée autorisée,
SIGINT/TERM-reçufailed/close PASS sur1043. La revue1050 (8complètes) révèle une
limite de sonde : dispatch observé peut précéder l’entrée. Renforcer à deux
completions timedwait main après entrée pour encadrer un nouveau tour de boucle,
sans borne temporelle ni cap de tentatives. Trace filtrée et liée au source ; pas
attestation de stack ou de performance. Revue latestWork dans interrupted_phase
à reproduire avant correction : ancienne révision pourrait remplacer la plus haute.

###1051–1052 — conserver la plus haute révision et tester shutdown
1051 FAIL : métadonnées interruption rev3 remplacent rev5 déjà connue (3,3,3
au lieu de3,5,3). Correction commune interrupted_phase prend le max sameWork,
pour tous les rôles ; événement d’appel garde sa vraie révision. Une erreur du
stop final ne doit pas perdre la plus haute révision. Nouveau test finish : vrai
transport local/join, erreur main→ControllerShutdown et deux causes, faux Stop
de fixture+interruption typée vs erreur indépendante ; pas preuve de signal OS.
La sonde publique exige maintenant deux waits main complets postentrée, plus
dispatch observé et provider encore possédé/actif. Une garde parser refuse
lignes anciennes/partielles/mauvais futex/timeoutNULL ; aucun watchdog ajouté.

###1058 — planner asynchrone vérifié, suite rôles
Deux vagues de8 GLM :14 complètes/2 tronquées,81296 tokens déclarés,2HTTP/0retry.
Profil1053 :336 entrées/110 artefacts/4 cores sans fixture. Recette1054 :6 couples
rôle/signal et2 témoins de progression main pendant planner retenu ; reprise
registered1054 et contrôles/reviewer/quote invalide1055 PASS, intégrité VERIFIED.
15 tests CLI/job +6 engine (peer/source), Clippy engine/CLI tous targets et
Rustdoc PASS.1051 reproduit puis1052 corrige baisse de révision connue ; cause
et variantes gardées. Source-review fraîche liée aux5 entrées modifiées depuis
1029 ; les preuves historiques et hypothèse temporelle993 restent conservées.
Aucun restart/install/commit. Suite : rendre contrôles/reviewer asynchrones en
conservant claim/reprise/financement et ordre de publication ; Root peer ne vaut
ni identité d’agent authentifiée ni grant. Global G3/R6/R7/RETEX restent ouverts.

###1059 — transmission des améliorations au prochain lot
Après1058, ajout de deux pistes liées au cycle de vie des peers dans
AFTER-CUTOVER : éviter peer inutile sans planner et qualifier ouverture/écritures
concurrentes avant d’étendre les rôles. Hypothèse BUSY non reproduite, pas un
blocage actuel ni une autorisation de relever un timeout. Propriétaire Root,
critères et condition de reprise conservés. Aucun source/contrat exécuté modifié ;
les preuves1058 restent liées à leurs octets et la qualification globale ouverte.

###1060 — reprise : huit GLM pour les rôles asynchrones
Besoin : rendre contrôles et reviewer non bloquants pour observation/dispatch,
sans changer leurs producteurs, droits, financement ou publication. Huit tâches
textuelles bornées, deux HTTP simultanés, zéro retry/watchdog global : abstraction
job, contrôles, reviewer, ordre des phases, cycle peer, sonde publique, arrêt/erreurs
et revue de conception. Comparer un job de phase séquentiel et trois jobs concurrents ;
le plus petit parcours vérifiable prime, sans prétendre porter toute concurrence Go.
Hypothèse bootstrap BUSY1059 transmise avec sources, aucun défaut encore démontré.
Fichiers envisagés : mission_watch.rs, son module planner_job, sondes/recette
et documentation FR/EN ; aucun changement Live/UX de Claude ni installation.

###1061 — choix et assemblage des phases asynchrones
Vague1060 terminée : huit réponses complètes, aucun retry. Retenu : un handle
de phase typée Controls/Reviewer/Planner ; contrôles et reviews drainés dans cet
ordre, publication/join explicites avant phase suivante. Les producteurs engine,
leurs gardes et budgets restent inchangés ; aucun lancement simultané de rôles.
Le main continue observation/dispatch pendant chaque appel ; preuve publique
pending. Module planner_job extrait en role_job, états vides distingués d’un
vrai Work retourné. Hypothèse bootstrap reste ouverte ; pas de retry masquant BUSY.
Préparation1060 a refusé avant appel sur une borne textuelle inexistante : relu
les vraies frontières, corrigé le constructeur ; aucun appel doublé.

###1065 — régression de diagnostic reproduite et corrigée
1064-registered FAIL sur1062 : première synchronisation dispatch publiée avant
le diagnostic de contexte registered, auparavant émis dans la phase planner du
même tour. Cause : observation subordonnée à l’avancement des phases async.
Correction générale : contexte actif observé avant chaque dispatch indépendamment
du job ; appel pending=true ne peut pas lancer le planner. Aucun seuil/retry ajouté,
aucune garde de test assouplie. Rejeu sur nouveau profil requis. Rustdoc execute
et planning_once remise en accord avec les chemins effectifs.

###1069 — banc de recette : attente explicite du rôle métier
1067-manual-controls FAIL sur1066 : helper relay.cycle stoppe au premier blocked,
alors que le contrôle asynchrone est en démarrage. Cause conservée : ConnectionReset
gate + interruption et Work rev13 ; aucun succès propre inventé côté moteur.
Correction du contrat de recette : dispatch synchronisé ET événement du rôle
attendu avant stop. Appelants contrôles/manual-review/révocations actualisés ;
refus attendent toujours vraie sortie non nulle. Garde avec témoin valide puis
marker seul/mauvais rôle/non-synchronisé refusés, et récupération. La recette manual-controls
ajoute une garde nommée dédiée ; les groupes partagés gardent leurs deux tests. Aucun code moteur changé.
La matrice1067 reste liée aux sources originales ; nouveau rejeu après édition.

###1072 — vague1060 intégrée et vérifiée localement
Huit GLM complets,101455 tokens déclarés, deux HTTP simultanés, zéro retry.
Revue finale GLM1074 séparée favorable :59447 tokens, neuf appels au total
160902 tokens déclarés ; avis statique sur octets fournis, pas exécution de tests.
Un job de rôle ordonné sous bail/Root réel ; main disponible pendant les trois
rôles.1066 :336 entrées/110 artefacts/4 cores sans fixture ;16 tests CLI/job,
Clippy engine+CLI tous targets et Rustdoc PASS. Six recettes PASS/intégrité
VERIFIED : six signaux et six témoins de progression, registered, manual, quote
invalide et révocations contrôles/reviewer.1064 et1067 échecs conservés : diagnostic
registered dépendant de phase corrigé1065, helper arrêtant avant reçu corrigé1069.
Revue liée aux quatre entrées compilateur modifiées1053→1066, retrait/renommage
module inclus. Preuves exactes et fixtures archivées1072. Signal probe1067 garde
ses sources originales ; fonctions rows/work/prepare inchangées après1069, portée
de réutilisation explicite. Pas d’installation/restart/commit, ni acceptation globale.
Suite G3 : confronter le parcours combiné multi-tâches au dispatcher Go, puis
récupération managed et raccord web ; ne pas confondre main disponible et rôles
simultanés. Revoir d’abord les priorités et critères du plan, sans durcissement gratuit.

###1080 — campagne huit vagues : 1/8 — assembler parcours multi-tâches et définir mission HTTP
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1080`. Intégration et vérification restent à Root.

Campagne demandée : huit vagues de huit GLM (64 appels au maximum pour ces
vagues), successives et adaptées aux résultats ; aucun retry automatique.
Priorité : parcours multi-tâches G3 et raccords HTTP existants, sans toucher
Live/UX de Claude. Fichiers réservés envisagés : service/mission.rs, ajout mod/route
service.rs, tests/recettes et docs FR/EN ; droits/Root/cadence/budgets inchangés.

###1090 — campagne huit vagues : 2/8 — contrats précis du pont mission HTTP
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1090`. Intégration et vérification restent à Root.

###1100 — campagne huit vagues : 3/8 — revue lectures réelles et recette HTTP/CLI
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1100`. Intégration et vérification restent à Root.

###1110 — campagne huit vagues : 4/8 — admission des commandes mission HTTP
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1110`. Intégration et vérification restent à Root.

###1120 — campagne huit vagues : 5/8 — revue du candidat lecture/écriture assemblé
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1120`. Intégration et vérification restent à Root.

###1130 — campagne huit vagues : 6/8 — recette publique et défaillances du banc
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1130`. Intégration et vérification restent à Root.

###1131 — garde du banc reproduite puis corrigée
Le contexte de substitution d’inode sur fixture pouvait sauter la restauration
si la conservation de la copie refusée échouait. Deux tests, témoin valide avant
injection : RED exit1 conservé, puis PASS2 après tentatives indépendantes et
agrégation de toutes causes. Aucun code moteur modifié, original toujours protégé.
Les trois tests Rust du pont passent sur sources1111 ; profil1122 sans fixture
qualifié,337 entrées. Recette publique en préparation, pas d’installation.

###1140 — campagne huit vagues : 7/8 — revue candidate indépendante et limites
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1140`. Intégration et vérification restent à Root.

###1141 — replan du contrat status et causes secondaires
1132 FAIL : CLI status valide malgré Git indisponible, HTTP500. Hypothèse de
1081 fausse : observation.causes représente des diagnostics déjà exposés dans
le DTO, conservés par CLI seulement si une sortie ultérieure échoue. Correction
au contrat : HTTP200 dégradé identique, causes typées dans extension de réponse
puis NativeJobOutcome ; échec encodage/close les joint, échec observer les garde
dans pending et les traversées CLI. Aucun changement Live. Ajouts partagés dans
service/observation/transport et CLI native_web annoncés, sans politique nouvelle.
1141 exige nouveau profil et gardes de transfert/observation/causes finales.

###1150 — campagne huit vagues : 8/8 — revue finale des correctifs et preuves
Huit tâches GLM bornées, deux HTTP simultanés, zéro retry ; contexte exact
et résultats sous `docs/plans/migration-rust/evidence/20261024-eight-waves-1080/wave1150`. Intégration et vérification restent à Root.

###1153 — variance mesurée et revue contradictoire
1152 FAIL conservé après correction1141 : seul spending.observed_at diffère
entre CLI et HTTP après normalisation des mesures stockage. Champ d’observation
indépendante maintenant déclaré, montants/appels/droits/états gardés stricts.
Garde comparaison refuse aussi un changement de calls. Revue GLM1150 restaurer :
une panne de restauration elle-même ne peut garantir succès ; nouveau témoin
conserve original à state.original-owned.db, groupe les causes, puis récupération
après retrait de l’injection. Pas de retry caché. Revue décodeur1150 : hypothèse
profile/slots acceptés rejetée par shape explicite + deny_unknown_fields, voir
work::decode_mode_from ; la taille du struct serde ne décide pas les champs.
Quatre gardes Python nommées ; requête resume+slots doit refuser dans le rejeu.

###1155 — Work inconnu : défaut de frontière HTTP reproduit
1154 passe les quatre lectures puis FAIL : Work inconnu500 sous la famille
mission-root-unavailable au lieu de404. Cause : le wrapper mission cache la
famille canonique du lecteur Work au classifieur HTTP. Correction générale :
après binding et avant tout producteur, lire le Work sélectionné avec son erreur
Root canonique, sans exception d’identifiant/code textuel. Pas de mutation/droit
nouveau. Reprofilage et rejeu du cas identique requis ; sources1151 conservées.

###1158 — fixture d’écriture complète, garde organisation conservée
1157 passe lectures/refus puis refuse start sur Work ordinaire : organisation
planning manquante, garde correcte. Le banc sépare désormais lectures ordinaires
et écritures sur la fixture publique ready déjà maintenue (planning.flow), sans
worker admis. Ses processus de préparation sont locaux/inertes ; l’API ne doit
ajouter aucun appel/agent/bail ni effacer le financement initial. Avant/après
conservés ; aucun changement du moteur, des guards ni des budgets produit.

###1162 — comptage préservé et révocation attendue
1159 PASS conservé. Renforcement1161 révèle une hypothèse trop forte du banc :
pause/stop doivent avancer les générations et vider holder/lease_until, comme
mission_control_request::revoke le prescrit.1161 FAIL conservé, aucun défaut
moteur observé. Assertion corrigée au contrat exact, tous les autres champs
planning restent égaux ; ledger planning_calls avant/après gardé strict. Rejeu
1163 sur même binaire1156, sans recompilation inutile ni modification de garde.

###1170 — huit vagues exécutées, incrément local qualifié
64 appels GLM :55 complets/9 tronqués,384029 tokens déclarés,2HTTP/0retry.
Pont natif mission :4lectures et6writers CLI,17 assertions publiques +4gardes
1165 sur1156, financement/tasks/rejeu/refus/restauration/reopen conservés.
33web+2CLI1142,3mission1155,Clippy/Rustdoc PASS ;337 entrées/110artefacts sans
fixture. Les10 entrées compilateur modifiées sont revues aux octets, preuves
originales et fixtures conservées1170. Les réutilisations des revues1150 et
tests antérieurs restent bornées ; pas d’acceptation indépendante globale.
Cinq échecs conservés et traités ; les réponses tronquées ne comptent pas comme
travail terminé. Aucune installation/restart/commit/push, Go/LIVE-UX inchangés.
Suite de migration : conducteur serveur sous bail et gestion d’arrêt, puis
récupération managed/parcours multi-tâches. Les autres pistes restent après bascule.

###1180 — vague suivante autorisée : conducteur web, huit agents GLM
Reprise1170 : le pont HTTP est vérifié localement, sans conducteur serveur.
Huit appels textuels SKYNET, deux simultanés, sortie2000low, aucun retry ni
watchdog global. Extraction, raccord serveur, bail/énumération, arrêt/reprise,
recette publique, politiques/erreurs, contrat Go et revue du pont sont séparés.
Aucun GLM ne modifie le dépôt. Racine réserve uniquement ce suivi et
`evidence/20261024-wave1180-web-conductor/` durant la collecte ; fichiers LIVE
et travaux Claude préservés. Le résultat attendu est une proposition raccordable
avec critères de parcours ; cette vague seule ne prouve pas le conducteur livré.

###1181 — vague1180 terminée et propositions examinées
8appels,7réponses complètes/1sans texte final,132154tokens déclarés.
Replan : bridge-review a épuisé sa sortie en raisonnement ; preuve conservée,
aucun retry. Contrôle Root écarte interfaces inventées et mission longue dans
la lane HTTP. list_works/claim/ExecutionInterrupt existent ; le paquet GLM
omet leurs producteurs, à corriger avant toute campagne suivante autorisée.
Suite : session drive réutilisable avec arrêt/sortie injectés et propriétaire
serveur ; preuve attendue start→progrès→pause/stop, concurrence de bail et drain.
[Avis et décisions](evidence/20261024-wave1180-web-conductor/ASSESSMENT.md),
[8reçus et empreintes](evidence/20261024-wave1180-web-conductor/result.json).
Aucun changement produit/build/test/restart ; conducteur toujours non livré.

###1190–1218 — conducteur automatique web, vérifié localement
Serveur seul : start HTTP→worker inerte réel ; six writers réactifs ; CLI et
second serveur exclus par le même bail ; stop/reopen et worker survivant vérifiés.
13assertions publiques+4gardes Python,44tests CLI,Clippy/Rustdoc PASS ;
3cas CLI pause/SIGINT/SIGTERM et6cas signaux rôles PASS, planner web retenu
arrêté localement avec reçu TERM réel. Profil1211 :338entrées/110artefacts
sans fixture ; binaire9e565a…de8db non installé. Causes secondaires CLI et
course pause/révision corrigées avec RED originaux ; même event_id/cadence.
8GLM1192 :7réponses complètes/1incomplète,69840tokens déclarés,0retry.
[Preuves/limites](evidence/20261024-web-conductor1190/decision1218/result.json),
[chronologie](evidence/20261024-web-conductor1190/decision1218/execution-history.md).
Revue indépendante du candidat et acceptation globale ouvertes. Suite G3 :
récupération managed et parcours multi-tâches, puis autres groupes/R6/R7/RETEX.
Aucun Go/LIVE-UX modifié ; aucun restart/installation/commit/push.
