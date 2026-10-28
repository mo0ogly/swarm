# File des projets APEX de Swarm

[English](../en/APEX-QUEUE.md) · [Méthode APEX](../../.claude/skills/apex/SKILL.md)

Actualisation du 24 octobre 2026 : onze entrées après les vagues 2 et 3 ;
les familles historiques rapprochées restent distinctes des projets à démarrer.

23 octobre 2026 — registre documentaire créé à la demande de l’utilisateur.
Coordination : Codex racine. Cette file rassemble les plans existants, leurs
dépendances et la prochaine preuve attendue. Leurs exigences, décisions et preuves
restent dans les documents d’origine. Les états ci-dessous sont ceux de leur
dernière lecture documentaire ; ils ne décrivent pas une file native du moteur.

Le classement ne crée aucune mission, ne reprend aucun processus et n’accorde
aucune autorisation d’exécution. Le juge L4 reste une proposition non adoptée ;
AC-02 reste enregistré sans démarrage. Les droits et budgets déjà applicables
restent portés par leurs configurations et décisions autorisées.

## File et dépendances

La migration conserve la priorité déjà décidée. Les entrées G7/G8 sont des
raccords à ce socle : reprendre leurs critères propres sans recréer un projet
concurrent ni compter deux fois le même résultat. Les travaux d’observation déjà
réalisés restent acquis dans leur périmètre ; leur présence ici ne les remet pas
à zéro. L’ordre des projets futurs sera arbitré au moment de leur adoption.

| Position de suivi | Plan source | État documentaire | Prochaine tranche et sortie attendue | Dépendances et condition de reprise |
|---|---|---|---|---|
| Programme PAEA adopté | [PA-00…PA-13](actions-confiance/PAEA-APEX.md) | Base dev75 : 14 tests publics réussis, 618 invocations, 167 refus attendus ; provenance conservée. Incrément 10 : receipt-report v2 lié aux octets exacts et sept tests écrits, non compilés/non exécutés/non installés. | Livraison 15 : 5 tests Python/45 Rust PASS sur checkout et socle publié ; 18 tests CLI/973 appels, sept tests de laboratoire, 237 assertions navigateur natif. Commit 462abbeb9 poussé ; dev80 activée, 22 contrôles UI de l’instance réelle PASS. | Compilation/push autorisés par « fini push compile ». Correspondance native livrée ; Incrément 16 installé dev82 : résolution de dettes hors ligne, 37 tests de contrat, 33 appels publics, 8 scénarios et 26 contrôles live PASS ; sources non commitées. Incrément 17 : menu de résolution livré dev87, 452 assertions natives et 30 contrôles live qualifiés, API commune et nombres exacts. PA-02/03 protégés, registre authentifié, campagnes UX, E2/E3 et RETEX restent ouverts. 0/17 invariants de bout en bout. |
| Socle prioritaire | [Migration Rust](migration-rust/PLAN.md) | En cours ; qualification globale et bascule non closes. | Terminer les parcours publics du plan actif, puis qualifier le même candidat et préparer restauration/bascule avec les preuves requises. | Ordre et propriétaires du plan de migration ; cette file ne modifie pas ses travaux en cours. |
| Raccord G7 | [Identité R01/R13, IA1–IA5](actions-confiance/identites-agents-replan.md) | Partiellement réalisé ; noyaux privés et preuves bornées, raccord public ouvert. | Authentification humaine locale, contrôle protégé et admission exacte aux lancements/usages ; refus et révocation prouvés dans le parcours retenu. | Distinguer parité G7 et clôture IA1–IA5 ; IA3 fournit la sortie protégée consommée par M20. Toute protection requise garde sa preuve et son refus, sans attendre uniquement AC-02. |
| Raccord G8 | [Mémoire MEM-P1…P5](deepagents-integration/PLAN.md) | Socle mémoire réalisé en partie ; suite de cinq étapes non clôturée. | Attribuer propriétaire/auteur, distinguer lecture/résumé/transmission et qualifier le contenu effectivement reçu avec révocation/reprise. | P1 et sortie protégée IA3 selon les critères P2–P4 ; P4 alimente IA4. Ne pas attendre la clôture entière IA5 ou MEM-P5 pour ce raccord. Préparation isolée et activation publique restent distinctes. |
| Observation à consolider | [LIVE-UX-03 et télémétrie](telemetrie-agents/SPEC.md) | Plusieurs lots réalisés et vérifiés localement ; couverture et revue requise incomplètes. | Compléter les raccords reviewer/planification et qualifier les consommateurs réels ; conserver la provenance déclarative des données fournisseur. | Coordination avec lancement, supervision et web Rust ; inventaire des acquis avant tout nouveau lot. Ne pas inclure implicitement le juge par modèle. |
| Observation élargie à consolider | [Feuille de route LangWatch L1–L11](langwatch-feuille-de-route/SPEC.md) | Plusieurs lots décrits comme réalisés localement le 22 octobre ; qualification indépendante non acquise. | Inventorier les preuves par lot, les limites du transcript, les alertes et le lien livraison/candidat ; définir la prochaine tranche avec son propriétaire. | L1 rejoint LIVE-UX-03 ; le juge par modèle L4 reste dans JUGE-L4. Conserver les autres dépendances L1–L11, droits et réglages persistés ; aucune double réalisation ni délégation implicite. |
| Qualification reportée | [R20 — observation eBPF](actions-confiance/qualification-globale-replan.md#décision-utilisateur-du16octobre--revue-autorisée-ebpf-reporté) | Essai noyau positif non qualifié ; report utilisateur du 16 octobre conservé. Le périmètre T8 convenu a son verdict historique distinct. | Après reprise autorisée, fixer candidat, lanceur et périmètre ; vérifier collecte réelle, attribution, exclusions, pertes, nettoyage et reprise selon le contrat. | Demande ultérieure et environnement BPF explicitement autorisé ; aucun privilège ni essai ajouté. Codex possède la recette, l’opérateur l’autorisation d’environnement. AC-02/A5 inventorie les preuves pertinentes sans absorber ni clore R20 par déduction. |
| Proposition historique à réexaminer | [Graphe et automatisation A01–D04](graphe-automatisation-20261005/PLAN.md) | Proposition du 5 octobre ; adoption et réalisation de l’ensemble non établies par cette relecture. | A01 doit comparer chaque exigence à l’état Rust et aux acquis, avant de préciser les contrats A02 et le design A03. | Raccord G4/G5 et autres parcours concernés de la migration ; Codex racine conserve la cartographie. La référence Go du document est historique. Aucun redémarrage des seize lots ni nouvelle priorité adoptée. |
| Projet futur enregistré | [AC-02 — sécurité d’exécution](actions-confiance/securite-execution-apex.md) | A0–A7 non démarrés ; relecture AC02-RV-01…06 conservée, dépendances 05/06 précisées dans le document. | A0 cartographie le Rust final ; A1 cadre les raccords et les quatre parcours de sécurité ; la suite conserve YAML d’échange, politiques SQLite et application Rust. | Migration et qualification Rust, puis demande explicite de démarrage. A0/A3 réutilisent les frontières déjà prouvées ; cinq vues d’architecture distinctes des quatre vues de sécurité. |
| Proposition à arbitrer | [JUGE-L4 — juge par modèle](telemetrie-agents/JUGE-L4-SPEC.md) | Proposé, non adopté et non réalisé. | Résoudre les décisions D1–D4 puis préparer un lot borné avec admission moteur, réservation, réponse stricte, annulation et recette isolée. | Adoption explicite et contrats de lancement/budget disponibles ; aucun appel payant ou rôle nouveau autorisé par cette inscription. Pas de dépendance ajoutée à AC-02. |
| Amélioration différée | [AC-01 — diagnostic privé du bootstrap](migration-rust/AFTER-CUTOVER.md) | Proposition différée ; pas un plan détaillé prêt à exécuter. | Cadrer création protégée, conservation et diagnostic sûr, puis démontrer refus, effets partiels et reprise. | Après bascule, besoin produit validé ou défaut reproduit selon la décision d’origine ; décider persistance/Admin et visibilité avant activation. |
| Améliorations historiques à rapprocher | [QW5–QW8 — lancement clair](clear-launch-recovery/RETEX-CLOTURE.md#quick-wins-proposés--4-octobre-2026) | Quatre propositions conservées ; couverture actuelle à établir avant de les déclarer restantes. | Comparer accès au bon réglage, état/historique, effort attribué et prévol de revue aux acquis ; retenir seulement les écarts démontrés. | Codex racine ; rapprochement avec budgets, supervision, comptabilité et revue Rust. Aucun doublon avec LangWatch/QW2 ; mission source clôturée, aucune reprise automatique. |

## Préparer la prochaine entrée avec APEX

À chaque reprise, ouvrir le plan source et relever son état actuel. Une évolution
de source invalide la réutilisation automatique de l’état affiché dans cette file.
Préserver les IDs, historiques, consommations, refus et preuves existantes.

La fiche de la prochaine tranche doit préciser :

- **Besoin et source** : objectif observable, IDs d’exigences et plan canonique ;
  distinguer acquis vérifiés, travaux partiels et propositions.
- **Périmètre et dépendances** : résultat borné, exclusions, préconditions et
  raccords aux autres plans ; ne pas adopter toute une feuille de route d’un coup.
- **Responsabilité** : propriétaire d’intégration, réalisateur et fichiers
  attribués ; revue indépendante dans un autre contexte lorsqu’elle est requise.
- **Autorisation et moyens** : décision de démarrage applicable, droits,
  fournisseurs et budgets effectifs ; aucune capacité déduite d’un nom de rôle.
- **Vérification et reprise** : témoin valide, refus sans mutation, panne,
  réouverture et effet observé selon le contrat ; `replan` en cas d’anomalie.
- **Clôture** : candidat et entrées exactes, contrôles réellement exécutés,
  avis requis, documentation FR/EN et RETEX. Une preuve locale ne remplace ni
  l’acceptation indépendante ni une validation humaine exigée.

**Analyse → plan borné → vérification des préconditions et de l’autorisation →
exécution autorisée → vérification et revue → RETEX et clôture.** Cette suite
décrit la méthode ; elle ne crée aucune commande ni nouveau statut public Swarm.

## Tenir la file à jour

Les libellés du registre distinguent proposition, attente de prérequis, prêt
techniquement, autorisé, en cours, vérifié localement et clôturé. « Prêt » ne vaut
pas « autorisé ». Un report conserve cause, propriétaire et condition de reprise ;
une vérification en échec maintient le critère ouvert.

Mettre à jour le plan source et ses preuves, puis cette synthèse FR/EN dans le
même incrément. L’avancement réel reste déterminé par le parcours public et les
preuves applicables ; cette file n’est pas un ordonnanceur automatique et ne
définit aucun plafond de concurrence, délai, budget ou nouvelle politique moteur.

Choix retenu : un registre relié aux plans, plutôt que des copies de plans ou des
missions natives créées pour les représenter. Cela préserve les historiques et
rend visibles les dépendances sans lancer de travaux supplémentaires.

La file est accessible depuis les README FR/EN et le suivi après bascule. Elle
transmet l’organisation aux prochaines lectures ; elle ne modifie ni contexte
d’agent déjà démarré, ni état natif, ni code, build ou version livrée. Les contrôles
des incréments portent sur les liens, la cohérence des entrées et les diffs.

Le [cadrage des dépendances Identité/AC-02](actions-confiance/securite-execution-apex.md#9-dépendances-et-livrables-partagés-précisés)
précise parité G7 et clôture des cibles Identité, puis autorité IA3 → usages M20
→ raccord IA4, sans attendre toute IA5 pour préparer la mémoire. Les propriétaires
et critères de remise sont nommés ; leurs effets fonctionnels restent non vérifiés.
Les critères et protections nécessaires restent ceux des plans sources.

## Vague 2 — rapprochement des suivis antérieurs, 24 octobre 2026

Origine : demande « vague suivantes », après vérification des sept entrées
initiales. Cette vague porte le registre à dix entrées ; elle ajoute LangWatch,
le report R20 et la proposition graphe/automatisation. Elle préserve les acquis
sans transformer une mention ancienne de travail en cours en reprise autorisée.
Codex racine possède cette consolidation documentaire ; le propriétaire de toute
nouvelle réalisation reste à confirmer dans sa tranche avant démarrage.

| Suivi rapproché | Dernier état documentaire pertinent | Remise aux chantiers actuels et prochaine preuve |
|---|---|---|
| [Actions de confiance T8](actions-confiance/qualification-globale-replan.md#16octobre0729utc--verdict-et-clôture-du-périmètre-convenu), [RETEX](actions-confiance/RETEX-actions-confiance-20261016.md) | Verdict indépendant favorable du candidat du 16 octobre et RETEX du périmètre convenu ; R20 reporté. | Conserver les exigences et originaux pour G7 et AC-02/A0 ; aucune preuve Go ne qualifie automatiquement Rust, IA3–IA5 ou les nouveaux parcours. |
| [Suites Cursor QW1–QW3](engine-cursor-contract/suites-retex-20261013.md), [RETEX](engine-cursor-contract/RETEX-suites-20261013.md) | Trois suites vérifiées dans leurs campagnes historiques ; endurance et gain universel non démontrés. | Ne pas les réinscrire comme travaux non faits. Réutiliser leurs contrats et preuves pertinentes pour la parité, en contrôlant la fraîcheur ; aucune campagne fournisseur nouvelle. |
| [Revue proportionnée RD1–RD6](review-efficiency-apex/PLAN.md), [fin du journal](review-efficiency-apex/EVIDENCE.md#29-septembre--reprise-dun-avis-périmé-et-arrêt-demandé) | Correctifs et campagnes historiques distincts ; la fin d’EVIDENCE indique ensuite la mise à la corbeille de la mission et de ses huit tâches. Le PASS E6 isolé ne clôt pas la mission principale. | La mention historique « suivre la revue en cours » ne vaut plus instruction active. Conserver code et preuves ; toute reprise doit repartir d’un état public courant et d’une demande applicable, sans recréer la mission. |

Choix : rattacher les sorties et les états finaux aux plans sources plutôt que
relancer les plans entiers. Le défaut de lecture d’un ancien état est traité par
des renvois réciproques ; cela ne prouve aucun effet sur un contexte déjà démarré.
Les contrôles de cette vague portent sur les dix entrées, les renvois FR/EN, les
états terminaux et le diff ; aucun critère fonctionnel n’est requalifié.

Cette vague ne couvre pas exhaustivement les anciens dossiers d’installation,
de version, de supervision, de budgets et de clôture. Leur rapprochement reste
documentaire, à partir de leur dernier état et de leurs preuves ; ils ne sont
ni déclarés clos ni relancés par leur absence de cette file.

## Vague 3 — autres suivis historiques, 24 octobre 2026

Cette vague porte le registre à onze entrées avec le suivi groupé QW5–QW8.
Codex racine possède le rapprochement documentaire des familles suivantes.
Un résultat reste lié à sa mission, son candidat et sa date ; un état ancien
non clos n’est ni une reprise active ni une preuve de défaut dans le Rust actuel.

| Famille et sources | État documentaire conservé | Raccord et prochaine preuve |
|---|---|---|
| [Lancement clair](clear-launch-recovery/RETEX-CLOTURE.md) | Clôture T0–T5, 6/6, le 3 octobre ; QW5–QW8 restent des propositions de ce suivi. | Entrée QW5–QW8 ci-dessus : matrice exigence → capacité actuelle → preuve → reste avant nouveau lot, sans reprendre la mission close. |
| [Clôture observable](cloture-observable/RETEX-final.md) | 5/5 et racine close le 4 octobre ; amélioration de la fraîcheur/publication proposée, non démontrée par le seul RETEX. | Comparer les contrats avec la parité Rust et les contrôles de revue, sans annoncer le défaut historique encore présent aujourd’hui. |
| [Supervision fiable](supervision-fiable/RETEX-suivi.md), [clôture T5](supervision-fiable/T5-diagnostic-superviseur.md#clôture-publique-vérifiée) | La fin de T5 établit 5/5 et racine close après les contrôles interrompus. | Les attentes R3/R4 du suivi antérieur sont historiques ; aucune nouvelle campagne ni gouvernance entre sous-agents livrée par cette lecture. |
| [Budgets, preuves, effort — T6](budgets-preuves-effort/T6-consolidation-superviseur.md) | Ancienne mission 5/6 ; consolidation T6 non validée dans ce dernier rapport. | La clôture de supervision ne clôt pas cette autre mission. Une reprise exige son état public courant et la demande applicable ; aucune mission de remplacement. |
| [Version et nouveautés — V4](version-history/V4-handoff.md#qualification-native-du-responsable--postérieure-au-rapport-worker) | Le complément final apporte des PASS techniques ; décision indépendante R5 encore attendue dans ce suivi. | Raccorder version/CLI/web/installation au candidat Rust et aux recettes actuelles ; ne pas réutiliser un numéro ou une ancienne installation comme preuve courante. |
| [Lanceur/on-premise](install-launcher-onprem/HANDOFF.md), [diagnostic IA](ai-connection-debug-20261008/HANDOFF.md) | Contrôles locaux et activations historiques décrits ; santé du fournisseur utilisateur et livraison Rust actuelle non prouvées par ces pages. | Contrats installation/lancement et configuration/fournisseur ; vérifier le parcours concerné sur le bon candidat seulement dans une tranche autorisée. |
| [Accueil Projets](dev-launcher-20261010/PLAN.md), [vues produit](ks-product-views.md) | Exigences et tranches historiques ; livraison actuelle complète non établie par cette lecture. | Cartographie avec les parcours Rust et la proposition graphe ; modèles de cadrage et modèles exécutables restent distincts. |
| [Installation neuve et méthodes](fresh-install-methods-20261008.md), [organisation](repository-organization-20261008.md) | Corrections et contrôles historiques conservés, dont suivi des contextes du 16 octobre ; aucune qualification courante déduite. | Vérifier les chargeurs et contextes réellement reçus sur le candidat retenu ; préserver checkout canonique, preuves et contrats de distribution. |
| [Sauvegarde générale du 17 octobre](commit-general-20261017.md) | Demande et incidents Git datés ; aucune nouvelle sauvegarde constatée ou autorisée par cette inscription. | Conserver l’historique ; aucune opération Git, copie de dépôt ou publication déclenchée ici. |

Ce registre rapproche des familles de plans, sans copier chaque rapport de tâche,
fixture ou dossier de preuves. Les historiques ne sont pas requalifiés ; les
anciens plafonds, modèles et commandes de livraison ne deviennent pas des valeurs
par défaut. Cette vague ne prouve ni l’état natif courant des missions, ni la
couverture de tout document du dépôt. Toute source nouvellement modifiée demande
une nouvelle lecture avant de réutiliser l’état résumé ici.

## Audit de parité fonctionnelle — reprise du 25 octobre 2026

L’[audit FR](migration-rust/AUDIT-FONCTIONNEL-20261024.md) et sa
[version EN](../en/plans/migration-rust/AUDIT-FONCTIONNEL-20261024.md) rapprochent
23 contrats de nos échanges du Rust installé dev.50. Témoins positifs : contrôle
humain, délégation/remise et refus/reprise dans les scénarios testés. Huit findings
FPA-01…08 conservent les manques et la qualification restante ; mémoire, consignes,
Trust et parcours produit ne sont pas globalement en parité.

Les corrections rejoignent G7/G8/G4/G5/G10 et les autres lots concernés, sans
nouveau projet ni démarrage par ce registre. Codex racine possède l’intégration ;
BIBA-RUST et les fichiers réservés gardent leurs propriétaires. La synchronisation
des plans est distincte de la conservation effective des fonctionnalités. AC-02
et les autres fonctions futures restent hors comparaison des acquis Go livrés.

Reprise du25octobre : FPA-03 reste vérifié ; FPA-02a restaure les références
historiques Work.memory et corrige le délai SQLite initial, installé dev65 avec
le parcours de remise S1 et trois courses de retrait d'autorisation corrigées.
[Rapport/RETEX](migration-rust/FPA02A-MEMOIRE-20261025.md).
G8 reste ouvert pour catalogue/admission/sélection versionnés et sources fraîches ;
aucune clôture MEM-P1…P5, droits M20 ou activation AC-02 n'est déduite.
