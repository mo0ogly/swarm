# Audit fonctionnel Go → Rust — 24 octobre 2026

[English](../../en/plans/migration-rust/AUDIT-FONCTIONNEL-20261024.md) · [Matrice structurée](evidence/20261024-functional-parity-aki6hq9e/functional-matrix.json) · [Candidat et entrées exactes](evidence/20261024-functional-parity-aki6hq9e/inputs.json)

**Verdict : non, toutes les fonctionnalités discutées et historiquement livrées
ne sont pas disponibles à l’identique dans le Rust installé observé.** Le cœur
déterministe et le contrôle humain ont des témoins positifs ; mémoire versionnée,
apprentissage des consignes, Trust et plusieurs parcours produit restent incomplets.
Cela ne démontre ni perte de données ni contournement des protections : les
parcours non portés sont généralement refusés explicitement.

## Périmètre, méthode et référence

Demande utilisateur : auditer le moteur après migration, en vérifiant les acquis
de nos échanges, pas seulement la présence de fichiers. Skill `audit-pdca`, mode
**audit-only** avec `replan` lors des écarts de preuve. Aucune correction moteur,
compilation, installation, mission réelle, délégation ou appel fournisseur payant.
La préférence d’installation locale des développements ne transforme pas cet
audit en autorisation de modifier/installer le travail concurrent.

Binaire réellement exécuté : **v0.1.1-dev.50**, commit compilé
`d566ed35c42a362f59e48a025d58b2b68a25b179`, modifié=true. SHA256
`0880206e3a0b780daaabf8697f3867adc96012d935098a9f8a341ceba0083a54` ; PID initial 1642977.
Image copiée depuis son descripteur `/proc/PID/exe`, hash comparé à l’état du
lanceur, puis sondes exécutées sur cette copie figée. API avec session valide,
sans journaliser son secret ; identité du processus contrôlée avant/après.

Référence Go : **885 fichiers de source**, dont 398 de production et 487 de tests,
plus go.mod/go.sum, retenus par l’outil existant sans exécuter Go. 71 fichiers
existants modifiés et 53 ajoutés par rapport au commit de référence : comparer
uniquement HEAD aurait perdu des acquis non committés. Cette archive n’est pas
une baseline approuvée, un graphe de compilation ni un inventaire des embeds.
Les preuves Go des 13/16 octobre restent historiques. Les données privées d’une
mission réelle ne sont pas utilisées comme fixture.

Un instantané de **542 sources/manifests Rust** a été conservé. Des raccords Trust
ont ensuite changé dans une autre session : [dérive observée](evidence/20261024-functional-parity-aki6hq9e/source-drift.json)
et [diffs attribués aux chemins complets](evidence/20261024-functional-parity-aki6hq9e/source-drift-patches.json).
Les sources en cours et le dev.50 testé sont deux candidats distincts. Le raccord
Trust en cours ne devient pas une fonction installée prouvée par cet audit.

## Résultats réellement observés

- **44 observations CLI FR/EN** : deux témoins `version`, 40 refus typés
  `command_not_ported`, deux refus de forme pour `prepare create`. Le refus de
  forme n’est pas, seul, une preuve d’absence ; il est rapproché du module et
  des capacités déclarant seulement list/methods/show/history. Root de fixture
  absent avant/après, stdout vide pour les commandes non portées.
- **23 observations HTTP authentifiées** : quatre réponses 200 et 19 réponses 404,
  dont 14 API et cinq pages. Les témoins capabilities/version évitent de confondre
  route absente et session refusée. Les remplacements natifs sont examinés avant
  toute conclusion : une ancienne URL absente ne signifie pas seule fonction absente.
- **Trois recettes maintenues réexécutées sur le binaire figé**, neuf scénarios nommés :
  contrôle humain/automatique/gate antérieure (3), délégation/remise et refus sans
  contrôle (2), rejet de proposition et reprise (4). Tous passent. Les processus
  « fournisseur » sont des programmes locaux de fixture ; aucun modèle réel.

[Sondes brutes indexées](evidence/20261024-functional-parity-aki6hq9e/public-probes.json) ·
[Validation humaine](evidence/20261024-functional-parity-aki6hq9e/human-validation.result.json) ·
[Supervision publique](evidence/20261024-functional-parity-aki6hq9e/supervision.result.json) ·
[Refus et reprise](evidence/20261024-functional-parity-aki6hq9e/recovery.result.json).
Les sorties/erreurs, argv et entrées de recette sont conservés à côté. Les unités
observation/scénario ne sont pas additionnées en taux de migration.

## Matrice des contrats issus de nos échanges

`PASS` couvre le témoin indiqué ; `PARTIAL` ne qualifie pas tout le domaine.
`FAIL` indique ici un écart de disponibilité/parité, pas un accès interdit réussi.
`NOT TESTED` et les propositions futures restent ouverts ou hors comparaison.

| ID | Fonction | État | Preuve et limite | Raccord propriétaire |
|---|---|---|---|---|
| FP-01 | Moteur déterministe, WorkEngine/RootStore | PARTIAL | Mutations publiques, contrôles et rejouabilité exercés dans les recettes ci-dessous ; parité globale non qualifiée. | G3 |
| FP-02 | Validation humaine dans la boucle | PASS | Contrôle réel → pending_human ; refus de l’acceptation implicite ; décision opérateur explicite distincte. Trois variantes humain/automatique/gate antérieure passent. | G3 |
| FP-03 | Superviseur, délégation, exécutant, remise | PARTIAL | Processus factices réellement lancés, remise reçue par le planificateur ; contrôle manquant refusé avant départ. Ni revue modèle réelle ni clôture autonome globale. | G3 |
| FP-04 | Diagnostic, refus et replan | PARTIAL | Quatre scénarios : diagnostic reçu, proposition corrigée appliquée, refus équivalent sans dépense répétée, altération/capacité nulle refusées puis reprise. | G3 |
| FP-05 | Mémoire permanente versionnée, sélection/retrait | FAIL | memory et page/API project-memory indisponibles ; source field-guide explicitement refusée tant que son garde versionné n’est pas porté. Modèles de données présents ≠ service utilisable. | G8 |
| FP-06 | Lancement d’un Work historique avec mémoire | PARTIAL | Le garde de lancement refuse Work.memory, project_memory, sélections et décisions mémoire non vides. Limitation explicite des sources examinées ; pas de suppression de données démontrée. | G8/G3 |
| FP-07 | Apprentissage attribué des consignes et rejet | FAIL | Base, révisions, composition et used_by_attempt existent ; reject-instructions refuse et le raccord revise-instructions historique n’est pas couvert par les décisions examinées. | G4/G3 |
| FP-08 | Actions de confiance, mandats, accords, Biba | FAIL | Pilote public absent de dev.50. Audit Biba antérieur confirmé ; raccords Trust ajoutés concurremment dans le workspace, non qualifiés par ces sondes. | G7 |
| FP-09 | Secrets, preuves et laboratoire Trust | FAIL | Commandes evidence/laboratory/verify et API Trust correspondantes non portées sur le binaire observé ; ne pas les confondre avec les réglages fournisseurs. | G7 |
| FP-10 | Identité des agents et enrôlement protégé | PARTIAL | Bibliothèques natives et bootstrap-check existent. IA3–IA5, service protégé et admission publique étaient déjà incomplets dans le plan Go ; aucune perte d’un raccord anciennement livré n’est affirmée. | G7 |
| FP-11 | Préparations : écrire, dialoguer, adopter | PARTIAL | list/methods/show/history raccordés ; création/édition/dialogue/conversion/adoption non exposés par le module examiné ; prepare.html absent. | G5 |
| FP-12 | Planification et éditeur de graphe | PARTIAL | Neuf actions HTTP déclarées ; commandes avancées, planning-bundle et graph-drafts non équivalents à ce sous-ensemble ; plusieurs reprises historiques à rapprocher. | G4 |
| FP-13 | Fournisseurs, connexions, profils, skills | PARTIAL | Résolution/configuration native et fixture fournisseur fonctionnent ; anciens namespaces profile/skills/connections/role-model et API testées refusés. Ne pas conclure à l’absence de toute configuration native. | G6/G8 |
| FP-14 | Automatisation et reprise de dialogue | FAIL | automation/exchange et API automation/dialogue-resume-preview absents sur dev.50 ; les alertes Live ne remplacent pas les déclenchements métier. | G10 |
| FP-15 | Terminal et anciennes vues de journaux | PARTIAL | terminal.html et log-console.html absents ; journaux/transcript natifs existent mais la parité d’une session terminal interactive n’est pas établie. | G2/G1 |
| FP-16 | Supervision Live, OTLP et LangWatch | PARTIAL | live.html répond 200 ; routes de télémétrie et alertes présentes. Ingestion, sondes, pertes, rétention et raccord reviewer/planification non rejoués dans cet audit. | G1/G3 |
| FP-17 | Budgets, politiques et réglages persistants | PARTIAL | Les recettes utilisent réglages publics et conservent réservations/tentatives ; cela ne qualifie pas tout le catalogue, tous les calculs de coût et tous les parcours Admin. | G9 |
| FP-18 | Données historiques et reprise/migration Root | PARTIAL | Champs mémoire/consignes conservés dans les types ; aucune migration complète, restauration ou non-régression de toutes les données qualifiée ici. Sources Go retenues ≠ graphe compilé approuvé. | R0/R6 |
| FP-19 | Version du binaire réellement exécuté | PASS | CLI, API protégée et image du processus identifiées dev.50, même SHA256 retenu. Aucune reconstruction/installation. | R7 |
| FP-20 | Menus AC-02, YAML/SQLite, confinement futur | NOT APPLICABLE | Plan explicitement enregistré sans démarrage ; sa non-implémentation n’est pas une régression Go→Rust. Même distinction pour JUGE-L4 et droits mémoire futurs. | AC-02 |
| FP-21 | FR/EN, thèmes, focus et interface rendue | NOT TESTED | Refus CLI sondés dans les deux langues ; aucune recette navigateur complète thèmes/clavier/focus sur toutes les fonctions pendant cet audit. | G11 |
| FP-22 | Revue proportionnée, preuves et acceptation | PARTIAL | Contrôles réels n’acceptent pas la tâche ; structures de revue natives présentes. Revue modèle indépendante, grands dossiers et toute la coordination historique non requalifiés ici. | G3/G4 |
| FP-23 | Méthodes APEX/replan et contextes embarqués | NOT TESTED | Les skills et chargeurs existent ; conformité de toutes les obligations effectivement reçues par chaque rôle non établie par la seule présence des documents. | G3/G5 |

## Écarts prioritaires et corrections à préparer

| Finding | Gravité et impact | Preuve discriminante | Suite et critère observable |
|---|---|---|---|
| FPA-01 | P1 — mémoire permanente inutilisable par son parcours historique | FP-05, API/page/CLI absentes ; garde `versioned-memory-source-guard-not-ported` | G8 : porter catalogue, aperçu/application, sélection, remplacement/retrait et contrôle au consommateur. Reprendre les témoins M01–M33 applicables ; ne pas déclarer acquis les droits M20 encore futurs. |
| FPA-02 | P1 — un Work enrichi en mémoire ne peut pas suivre toutes les branches natives | FP-06, `supported_work` refuse explicitement les champs non vides ; types conservés | G8/G3 : démontrer un vrai Work historique utilisable de bout en bout, puis retrait/fraîcheur/reprise, sans retirer le garde avant le consommateur protégé. Pas de migration destructrice pour masquer le refus. |
| FPA-03 | P1 — boucle d’apprentissage des consignes amputée | FP-07 : structures/composition conservées, rejet opérateur non porté ; opération historique Go `revise-instructions` non raccordée dans les décisions examinées | G4/G3 : création attribuée par le propriétaire, rejet avant réservation, concurrence, conservation de la base et contexte effectivement reçu ; revue/rejeu sur même tâche. |
| FPA-04 | P1 — Trust/Biba, preuves/secrets/laboratoire non équivalents | FP-08/09 et [audit Biba antérieur](evidence/20261024-biba-audit/AUDIT.md) | G7, chantier BIBA-RUST déjà actif : traiter les écarts sur son candidat ; flux admis puis refus, révocation, accord exact, effet simulé et reprise. Ne pas créer un second portage ni qualifier de nouvelles sources par l’ancien binaire. |
| FPA-05 | P1 — préparer/modifier/adopter et naviguer dans les graphes n’est pas couvert par les lectures natives | FP-11/12, modules et capacités ; pages/API absentes | G4/G5 : inventaire opération → schéma → droits → effet → refus/reprise, puis éditeur connecté au même moteur. Un nouveau dessin ou neuf actions n’est pas la parité de l’ancien parcours. |
| FPA-06 | P1 — automatisation et dialogue historiques indisponibles | FP-14 : refus CLI et API, exclusion explicite du dialogue dans l’adaptateur agent | G10 : déclenchement autorisé, identité/budget, idempotence, reprise des dialogues et preuves de contexte ; les alertes Live ne valent pas déclencheur métier. |
| FPA-07 | P2 — interfaces terminal/connexions/profils/skills incomplètes | FP-13/15 : configurations natives présentes, anciens consommateurs absents | G2/G6/G8 : relier chaque ancien contrat à un remplacement qualifié ou garder l’écart visible. Vérifier UI, CLI et erreurs sur le même candidat. |
| FPA-08 | Bloquant pour une déclaration « migration complète » — qualification finale insuffisante | FP-18/21/22/23, baseline Go non approuvée et contrôles globaux non rejoués | R0/R6/R7 : figer la référence réelle, profiler le build livré, qualifier données/restauration, parcours et preuves sur le même candidat, revue requise et RETEX. La synchronisation des plans n’établit pas cette parité. |

Ces findings conservent les historiques et rejoignent les groupes existants,
sans nouvelle mission. Codex racine possède l’intégration ; BIBA-RUST et les
autres fichiers réservés gardent leurs propriétaires. L’autorisation de cet
audit ne comprend pas la réalisation de ces corrections.

## Ce qui était déjà incomplet avant Rust

[Identité IA2/IA3](../../architecture/AGENT-IDENTITY-CONTRACT.md)
et le plan mémoire distinguent noyaux internes, émetteurs de fixture et raccords
publics encore ouverts. IA3–IA5, les droits M20 et le confinement AC-02 ne deviennent
pas des fonctions Go autrefois livrées parce qu’un schéma les décrit. AC-02,
JUGE-L4 et les extensions proposées restent futurs. Une destination autorisée,
un type signé ou une consigne ne prouve pas confinement ou absence d’injection.

La base des consignes est conservée ; des révisions autorisées sont possibles.
La sécurité n’est donc pas « toutes les consignes sont impossibles à changer ».
Le contrat à conserver est l’attribution, la décision autorisée, les gardes et
la traçabilité du contexte réellement transmis. Le contrôle humain exigé reste
central et a ici un témoin positif ; il ne doit pas être remplacé par un PASS IA.

## Reprise du 25 octobre 2026

Le service géré reste sur dev.50, même PID et même empreinte : les constats du
24 octobre restent liés à cette image. [Relecture enregistrée](evidence/20261024-functional-parity-aki6hq9e/resume-20261025.json).
Le plan actif distingue le lot1480 vérifié sur une base figée mais non installé
et BIBA-RUST en cours ; cet audit ne les requalifie pas. Les nouveaux portages
ne doivent pas être déclarés disponibles avant leur preuve d’installation.

Les copies durables des sources Go/Rust et des helpers de recette sont indexées
dans [retention.json](evidence/20261024-functional-parity-aki6hq9e/retention.json). Les neuf scénarios déjà
réussis ne sont pas rejoués sans changement de leurs entrées pertinentes ; leur
intégrité et celle des rapports font l’objet du contrôle final. La matrice n’est
pas un inventaire exhaustif de toutes les variantes d’API, de données ou d’UI.

## Limites, revue et reprise

Audit documentaire et comportemental ciblé terminé pour cette matrice ; conformité
globale **non établie**. Aucun score global. Pas de nouveau banc différentiel Go,
de modèle réel, de recette eBPF, de restauration complète, de revue indépendante
ni de recette navigateur universelle. Profil/features et inventaire effectif du
compilateur non reconstruits ; les résultats portent sur l’image installée exacte.
La revue sémantique est celle de l’auteur de cet audit, non un avis indépendant.

Les erreurs de méthode (chemins initialement supposés, puis nom de diff ambigu)
sont conservées et corrigées dans [le replan de méthode](evidence/20261024-functional-parity-aki6hq9e/method-replan.json) ;
aucune n’est utilisée comme preuve d’absence de fonction. Les snapshots originaux
restent intacts. Les preuves volumineuses/fixtures restent dans le cache possédé
référencé par inputs.json ; les résultats et canaux indexés sont durables ici.
Aucun nettoyage de preuve, aucune modification de base live, aucun redémarrage.

Reprise recommandée : achever le portage G7 déjà en cours, puis adopter une tranche
G8 mémoire et une tranche G4 apprentissage avec leurs témoins publics ; conserver
l’ordre du plan actif pour les autres groupes. Comparer une matrice de contrats,
pas le nombre de fichiers/lignes ni les seuls noms de commandes. Après correction,
rejouer le périmètre atteint et ses dépendances sur le nouveau candidat installé.

## Suite d’implémentation du 25 octobre — FPA-03

La demande utilisateur d’exécuter les évolutions avec APEX/GLM autorise désormais
leur réalisation. FPA-03 est livré et vérifié localement en dev59 : ajout attribué,
rejet CLI/API/cockpit, réservation, politique Admin et contexte natif. Voir
[FPA-03 et ses limites](FPA03-CONSIGNES-20261025.md). Le constat dev50 ci-dessus reste
historique ; les autres findings ne sont pas effacés ni déclarés résolus par ce lot.

Reprise du même lot : dev60 corrige aussi la course d’actualisation pendant rejet ; recette autonome finale et installation vérifiées. Les preuves dev59 et refus de méthode restent historiques.

## Suite du 25 octobre — FPA-02a

Dev65 conserve le consommateur des pièces historiques `Work.memory` livré en62,
son manifeste et ses gardes de sources avant départ, puis corrige trois courses
de retrait d'autorisation. Mémoire et S1 sont rejoués sur65 ; les preuves de reprise
SQLite et de non-régression consignes de62 restent distinctes. Installation65
vérifiée ; voir le
[rapport/RETEX FPA-02a](FPA02A-MEMOIRE-20261025.md). FPA-01 et le reste FPA-02
restent ouverts : catalogue/admission/sélection versionnés et leurs sources RETEX
ne sont pas raccordés par ce seul lot. Les observations dev50 ne sont pas réécrites.
