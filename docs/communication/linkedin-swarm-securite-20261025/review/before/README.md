# Swarm

<img src="web/swarm-logo.png" alt="Swarm" width="96">

[English](README.en.md) · Français

### Donner un objectif à une équipe d’agents IA. Comprendre qui agit. Vérifier ce qui est livré.

Swarm est un **cockpit local et une CLI pour organiser des workflows agentiques**.
Il relie le besoin, la planification, les agents, les contrôles et les résultats
à une mission persistante. Son interface est disponible en français et en anglais.
Son cœur est un **moteur déterministe en Rust** qui porte l’état et les
transitions autorisées des missions. Les règles et autorisations viennent de
l’humain ; le moteur fait respecter les validations exigées par la politique.
En mode `human`, des contrôles réussis ne remplacent pas la décision humaine.

Quand plusieurs agents travaillent sur un projet, il reste à transmettre les
consignes, gérer les dépendances, comprendre les blocages et examiner les preuves.
Swarm rend ces passages de relais visibles, avec des responsabilités et des
limites explicites. **Un processus terminé, un rapport présent et un résultat
accepté sont trois états distincts.**

**État au 24 octobre 2026 : migration Rust en cours.** Des parcours CLI et des
surfaces web ciblés sont vérifiés. La CLI et le serveur locaux exécutent Rust ;
la couverture fonctionnelle complète et l’acceptation finale restent ouvertes. Les capacités historiques Go ne
constituent pas une promesse de parité Rust. Voir l’[état natif](docs/architecture/RUST-MIGRATION.md),
le [plan actif](docs/plans/migration-rust/PLAN.md) et les [preuves et rejeux](docs/plans/migration-rust/REPLAYS.md).

[Vidéos](#vidéos) · [Capacités et maturité](#capacités-et-maturité) · [Architecture](#architecture) · [Contrôle humain](#contrôle-humain-et-validation) · [Identité](#identité-et-autorité) · [Mémoire](#mémoire-et-contexte) · [Supervision](#supervision-et-reprise) · [Journaux](#journaux-direct-et-traçabilité) · [Démarrer](#démarrer) · [Documentation](#documentation) · [Licence](#licence)

## Vidéos

### Identité mémoire et contrôle humain — 1 min 40

[![Film Swarm : moteur Rust, mémoire, identité et validation humaine](docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/apercu.png)](docs/communication/linkedin-swarm-identite-memoire-20261023/Swarm_Architecture_Film_v2.mp4)

Un film en motion design : une mauvaise consigne conservée puis transmise, le
moteur déterministe Rust, les droits, la validation humaine et la supervision.
**Français, 1080p, musique instrumentale originale, sans voix.** Les textes sont
intégrés ; des sous-titres FR et EN sont également fournis. Les raccords encore
ouverts et le plan AC02 non démarré restent explicites. Ce film illustre
l’architecture ; il ne constitue pas une capture de l’application.

[Lire le MP4](docs/communication/linkedin-swarm-identite-memoire-20261023/Swarm_Architecture_Film_v2.mp4) · [SRT français](docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/Swarm_Architecture_Film_v2.fr.srt) · [English SRT](docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/Swarm_Architecture_Film_v2.en.srt) · [Chapitres et sources](docs/communication/linkedin-swarm-identite-memoire-20261023/video-v2/README.md) · [Ancienne vidéo](docs/communication/linkedin-swarm-identite-memoire-20261023/video/README.md)

[Article Word v6](docs/communication/linkedin-swarm-identite-memoire-20261023/Swarm_Article2_Identite_Memoire_v6.docx) : introduction sur la contamination du contexte,
18 références, cinq vues techniques complètes intégrées aux sections de l’article
et accès au film. Le moteur Rust
et la validation humaine restent au centre de l’architecture.

### Explorer le graphe — environ 1 minute

[![Démonstration du graphe Swarm](docs/screenshots/graph-linkedin/Swarm_graphe_apercu_FR_V2.gif)](docs/screenshots/graph-linkedin/Swarm_graphe_LinkedIn_FR_V2.mp4)

Interactions réelles dans un plan pédagogique : dépendances, branches, blocages
et aperçu des contrôles. Explications en français ; aucun agent fournisseur lancé
et aucun contrôle enregistré. Cette capture historique ne qualifie pas le portage Rust.

[Lire le MP4](docs/screenshots/graph-linkedin/Swarm_graphe_LinkedIn_FR_V2.mp4) · [Image fixe](docs/screenshots/graph-tour/graph-tour-fr.png)

### Casa Pizza — quatre tutoriels en français et en anglais

| Module | Durée | Vidéo FR | Vidéo EN |
| --- | ---: | --- | --- |
| Besoin et graphe | 24 s | [MP4](docs/training/casa-pizza/tutoriels/media/01-plan-fr.mp4) | [MP4](docs/training/casa-pizza/tutoriels/media/01-plan-en.mp4) |
| Contrôles Swarm | 24 s | [MP4](docs/training/casa-pizza/tutoriels/media/02-controles-fr.mp4) | [MP4](docs/training/casa-pizza/tutoriels/media/02-controles-en.mp4) |
| Commande client | 30 s | [MP4](docs/training/casa-pizza/tutoriels/media/03-client-fr.mp4) | [MP4](docs/training/casa-pizza/tutoriels/media/03-client-en.mp4) |
| Restaurant et suivi | 42 s | [MP4](docs/training/casa-pizza/tutoriels/media/04-restaurant-fr.mp4) | [MP4](docs/training/casa-pizza/tutoriels/media/04-restaurant-en.mp4) |

Séquences de captures réelles légendées, sans audio, réalisées les 9–10 octobre
2026. Le plan pédagogique reste non exécuté ; la commande fictive est avancée
manuellement. Les contenus de l’application de référence restent en français.
Le kit contient le lecteur hors ligne, les transcriptions et les exercices corrigés.

[Guide PDF FR](docs/training/casa-pizza/Formation_Swarm_Casa_Pizza.pdf) · [Kit FR](docs/training/casa-pizza/Kit_Formation_Swarm_Casa_Pizza.zip) · [Guide PDF EN](docs/training/casa-pizza/Swarm_Casa_Pizza_Training_EN.pdf) · [Kit EN](docs/training/casa-pizza/Swarm_Casa_Pizza_Training_Kit_EN.zip) · [Méthode et limites](docs/training/casa-pizza/tutoriels/README.md)

GitHub affiche les images et le GIF ; ouvrir les liens MP4 pour lire les vidéos.

## Capacités et maturité

| Besoin | Fondations Swarm | État du portage Rust |
| --- | --- | --- |
| Préparer le travail | Besoin, brief, modèles, tâches et dépendances | Composants natifs ; parcours global de préparation à qualifier |
| Organiser et superviser | Planificateur, responsables de périmètre, exécutants, revue indépendante | Lancement/suivi d’un worker, reprise en attente et planification → worker vérifiés sur fixtures CLI ; autres rôles et parcours web ouverts |
| Comprendre l’activité | Graphe, tentatives, rapports, télémétrie et causes d’attente | Lectures et console live ciblées vérifiées ; cockpit complet ouvert |
| Encadrer les départs et la livraison | Dépendances, réservations, budgets, contrôles et fraîcheur des preuves | Contrats portés par incréments ; qualification globale ouverte |
| Valider les résultats | Politique `human` ou `automatic`, contrôles préautorisés et décision humaine lorsqu’elle est requise | Contrôleur natif : `pending_human` conservé en mode `human`, sans acceptation par un PASS ; qualification locale bornée, identité humaine protégée globale distincte |
| Transmettre l’expérience | RETEX vérifié, mémoire versionnée et sélection explicite | Parcours projet/mémoire Rust G8 ouvert ; contexte natif partiellement porté |
| Encadrer l’autorité | Acteur, instance, plan, octrois, délégation et révocation | Composants internes ; raccord public global et approbation humaine protégée ouverts |
| Confiner les agents et leurs accès réseau | Plan AC-02 : fichiers/processus, réseau/secrets, traçabilité et menus de sécurité | **Plan enregistré, non démarré** ; reprise après migration/qualification Rust et demande explicite |

Les preuves locales établissent leur périmètre déclaré. Elles ne démontrent ni
l’autonomie de toute une équipe, ni la compatibilité de tous les fournisseurs,
ni l’acceptation globale de la migration. Le [plan actif](docs/plans/migration-rust/PLAN.md)
conserve les critères encore ouverts.

## Architecture

Le **moteur déterministe Rust (`swarm-engine`, `RootStore`, `WorkEngine`)** relie
l’état de mission, les contrats, les décisions, les réservations et les preuves.
Les superviseurs proposent et arbitrent ; le moteur vérifie puis applique les
opérations autorisées. Les agents exécutants produisent les candidats et leurs
originaux ; contrôles, revue indépendante et jugement global gardent leurs rôles.

[![Architecture interne Swarm : moteur, responsabilités et preuves](docs/diagrams/architecture-overview-fr.svg)](docs/diagrams/architecture-overview-fr.svg)

[SVG en grand](docs/diagrams/architecture-overview-fr.svg) · [PNG](docs/diagrams/architecture-overview-fr.png) · [Source Mermaid](docs/diagrams/architecture-overview-fr.mmd)

Ces cinq vues décrivent l’architecture des **contrats internes Swarm**. Un bloc
fonctionnel ne désigne pas nécessairement une crate ou un service. La maturité
Rust est renseignée séparément dans l’[architecture détaillée](docs/architecture/OVERVIEW.md),
avec une [correspondance composants → sections sources](docs/diagrams/CONTRACT-MAP.md).
Un trait plein décrit une relation du contrat ; il ne certifie pas son déploiement.

## Contrôle humain et validation

Le contrôle humain fait partie du contrat de sécurité du moteur déterministe.
L’utilisateur définit l’objectif, le périmètre, les droits et la politique de
validation. Les superviseurs IA conduisent le travail dans ce cadre.

- En mode **`human`**, les contrôles préautorisés apportent des preuves. Un succès
  laisse le résultat en attente de décision humaine et ne libère pas les tâches
  suivantes. Un avis IA favorable ne remplace pas cette décision.
- En mode **`automatic`**, les contrôles et la revue indépendante prévus restent
  exigés ; le moteur vérifie le même candidat, les critères et la fraîcheur avant
  l’acceptation. Le producteur ne s’accepte pas lui-même.

Les contrôles natifs distinguent `pending_human` et `pending_review`. Leurs preuves
locales ne certifient pas encore le parcours complet d’authentification humaine
protégée. La conduite d’un processus, la revue IA et la validation humaine sont
des responsabilités distinctes.

[Contrat de validation humaine](docs/ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine) · [État Rust139](docs/architecture/RUST-MIGRATION.md#tâches-ordinaires-et-validation-humaine--raccord139-en-qualification).

## Identité et autorité

[![Identité Swarm : source d’autorité, registre et usage exact](docs/diagrams/identity-authority-fr.svg)](docs/diagrams/identity-authority-fr.svg)

[SVG en grand](docs/diagrams/identity-authority-fr.svg) · [PNG](docs/diagrams/identity-authority-fr.png) · [Source Mermaid](docs/diagrams/identity-authority-fr.mmd)

L’initiateur authentifié, le principal, l’agent logique, son instance, le
propriétaire du périmètre et l’auteur d’un contenu sont des notions distinctes.
Le contrat relie source humaine, enrôlement, plan courant, octrois et délégation,
instantanés du registre, tête Trust et demande exacte. L’accord doit rester lié
aux mêmes octets et à l’état courant jusqu’à l’admission ou l’usage.

La vue conserve aussi configuration/témoin, canal local contrôlé, expiration,
révocation et autorité de préparation avant plan. Les raccords encore ouverts
sont identifiés ; une signature ou un cookie ne devient pas un droit métier.
Une signature établit une origine sous ses hypothèses de confiance ; elle ne
prouve ni la vérité ni l’innocuité du contenu.
[Contrat d’identité](docs/architecture/AGENT-IDENTITY-CONTRACT.md).

## Mémoire et contexte

[![Mémoire Swarm : connaissances, contexte et reprise de conversation](docs/diagrams/memory-context-fr.svg)](docs/diagrams/memory-context-fr.svg)

[SVG en grand](docs/diagrams/memory-context-fr.svg) · [PNG](docs/diagrams/memory-context-fr.png) · [Source Mermaid](docs/diagrams/memory-context-fr.mmd)

La **mémoire Swarm** comprend plusieurs circuits : connaissances de projet
versionnées issues de RETEX, contexte préparé puis réellement composé, historique
de la tâche, sélections extractives, révisions de consigne et frontières de dialogue.
Les versions, conditions, sources, empreintes, réceptions et reprises restent
attribuées. Le retrait ne réécrit pas les contextes déjà transmis. Un souvenir ne crée aucun droit :
la mémoire informe le raisonnement, les contrats établissent l’autorité.

Une mauvaise instruction conservée en mémoire peut revenir dans une autre
instance. La provenance, l’admission et les conditions de réutilisation doivent
donc rester examinables ; une mémoire ne crée ni droit ni autorisation nouvelle.
La consigne initiale est conservée, avec des révisions attribuées. Les ajouts du
superviseur sont validés par défaut et rejetables avant réservation ; une tentative
déjà réservée ne reçoit pas de réécriture silencieuse. Ce parcours est distinct
de la validation humaine du résultat.

Les droits de lire, résumer et transmettre portent sur une source et un
destinataire exacts. La reprise d’une conversation demande une décision explicite,
un précontrôle et une nouvelle tentative ; une frontière non confirmée impose
d’examiner les effets. Le statut du portage de ces circuits figure dans la matrice
technique, sans changer le nom ni la place de la mémoire dans l’architecture.
[Mémoire](docs/PROJECT-MEMORY.md) · [Droits d’usage](docs/plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md) · [Consignes révisées](docs/architecture/INSTRUCTION-LEARNING.md).

## Supervision et reprise

[![Supervision Swarm : observation, décision, application et reprise](docs/diagrams/run-supervision-fr.svg)](docs/diagrams/run-supervision-fr.svg)

[SVG en grand](docs/diagrams/run-supervision-fr.svg) · [PNG](docs/diagrams/run-supervision-fr.png) · [Source Mermaid](docs/diagrams/run-supervision-fr.mmd)

Durée, attente d’outil, silence et incidents alimentent `run_observation`, attribué
à la mission, la tâche, la tentative et son propriétaire. Racine et sous-superviseur
reçoivent leurs événements ; ils diagnostiquent et proposent `planning decide`.
Le moteur contrôle droits, bail, génération, révision, événement reçu et budgets.

`continue-run` conserve le processus et ses compteurs ; `stop-run` demande l’arrêt,
puis attend sa confirmation réelle. Les observations de production, les limites
d’inférence structurée et le jugement global de cycle restent distincts. Les
incidents reviennent au propriétaire jusqu’à l’effet vérifié et la reprise du besoin.
La décision de continuer ou d’arrêter un processus ne remplace pas la validation
du résultat : en mode `human`, celle-ci attend toujours la décision humaine requise.
[Supervision](docs/architecture/RUN-SUPERVISION.md) · [Protocole IA/moteur](docs/architecture/AGENT-ENGINE-PROTOCOL.md) · [Cycles](docs/architecture/JUDGMENT-CYCLES.md).

## Journaux, direct et traçabilité

[![Observabilité Swarm : logs, direct, télémétrie et preuves](docs/diagrams/logs-observability-fr.svg)](docs/diagrams/logs-observability-fr.svg)

[SVG en grand](docs/diagrams/logs-observability-fr.svg) · [PNG](docs/diagrams/logs-observability-fr.png) · [Source Mermaid](docs/diagrams/logs-observability-fr.mmd)

Les logs de tentative, le terminal, l’activité moteur, les décisions, le registre
d’appels, les ressources Linux et la télémétrie OTLP conservent leurs provenances.
La console **Suivi → Journaux**, les vues directes et les exports utilisent les
opérations publiques avec leurs filtres, curseurs et limites. Elles restent en
lecture seule ; le direct SSE possède un repli explicite en interrogation.

Les originaux et les empreintes servent à reconstruire **objectif → décision →
tentative → candidat → contrôle → revue → validation humaine si requise → acceptation**. Un log coloré, un appel
réservé ou un coût absent ne devient ni preuve d’exécution, ni facture, ni succès.
[Console](docs/architecture/LOG-CONSOLE.md) · [Traçabilité](docs/architecture/TRACEABILITY.md) · [Registre](docs/architecture/CALL-ACCOUNTING.md) · [Direct natif](docs/architecture/RUST-MIGRATION.md#observation-en-direct-des-agents--live-ux-01-lecture-seule).

## Sécurité et évolutions prévues

Le [plan AC-02](docs/plans/actions-confiance/securite-execution-apex.md) conserve
les contrats de confinement Linux, de médiation réseau et secrets, de traçabilité,
de reprise et de menus de sécurité. Ses **quatre références visuelles** sont
l’éditeur de politique, le panneau des refus, la chaîne d’une connexion et la
chronologie de sécurité. Aucun lot AC-02 n’est lancé.

Les références ont des rôles distincts : [NVIDIA OpenShell v0.1.5](https://docs.nvidia.com/openshell/v0.1.5/how-it-works/policies/schema)
pour le sous-ensemble de politique visé ; [Bromure](https://bromure.io/en/blog/openshell-policies-and-network-lineage)
pour les parcours de sécurité ; [Agent Baseline Software Factory](https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/software-factory/software-factory.md)
pour les frontières et les contrôles. Les catalogues YAML d’Agent Baseline ne
sont pas des politiques de confinement exécutables. Les citer ne certifie pas Swarm.
Voir aussi l’[architecture cible des actions de confiance](docs/architecture/ACTIONS-CONFIANCE.md).

## Voir l’interface

### Console live Rust — fixture du 22 octobre 2026

![Console live native : activité et consommation déclarée sur fixture](docs/publications/20261022-console-live/images/dashboard.png)

Capture réelle de `swarm-rust` sur une racine de fixture : agents, alertes et
consommation déclarée. Vérifications locales ciblées ; revue indépendante et
acceptation globale ouvertes. [Présentation et provenance](docs/publications/20261022-console-live/README.md).

### Cockpit historique Go

Captures historiques du **1er octobre 2026** : mission de huit tâches clôturée
avec décisions humaines. Elles illustrent le cockpit Go, sans prouver la parité
Rust ou une autonomie sans intervention.

![Mission réelle : huit résultats validés et responsabilité racine clôturée](docs/screenshots/mission-complete-fr.png)

![Graphe : orchestrateur, exécutants, vérificateur et dépendances](docs/screenshots/mission-graph-current-fr.png)

[Ouvrir le graphe en grand](docs/screenshots/mission-graph-current-fr.png) · [Guide illustré](GUIDE-UTILISATEUR.md)

## Démarrer

### Application locale Rust

Prérequis : Linux, Git, Bash, Python 3, Rust/Cargo (toolchain validée : 1.96.0) et un compilateur C pour SQLite embarqué. Récupérez les dépendances verrouillées une fois, puis utilisez le lanceur pris en charge :

```sh
cargo fetch --locked --manifest-path rust/Cargo.toml
./swarm.sh build
./swarm.sh start --root /chemin/du/projet
./swarm.sh status
```

`./swarm` lance le Rust installé dans `bin/swarm`. Les commandes non portées refusent explicitement ; elles n’appellent jamais Go. Docker compile les sources Rust avec les dépendances verrouillées ; voir [l’installation](INSTALL.md).

### Développement Rust

Le moteur natif est en cours de construction pour Linux. Depuis la racine :

```sh
cargo build --manifest-path rust/Cargo.toml --locked -p swarm-cli
```

Le paquet `swarm-cli` produit **`swarm-rust`**. Consulter l’[aide native](rust/crates/swarm-cli/src/lib.rs)
et le [contrat de migration](docs/architecture/RUST-MIGRATION.md) avant de choisir
un parcours. Utiliser une racine de fixture isolée pour les essais ; ce build ne
réalise ni installation du service ni migration des missions existantes.

### Installation et usage historiques

La [release expérimentale v0.1.0](https://github.com/mo0ogly/swarm/releases/tag/v0.1.0)
et les guides [Docker/natif](INSTALL.md), [utilisateur](GUIDE-UTILISATEUR.md) et
[CLI](REFERENCE.md) documentent l’édition historique. `install.sh` (natif ou Docker), `build.sh`, `swarm.sh` et la CI construisent exclusivement Rust. Go n’est ni un prérequis ni un repli. L’installation courante figure dans [INSTALL](INSTALL.md) ; voir aussi le
[lanceur local natif](docs/architecture/NATIVE-LOCAL-LAUNCHER.md).

Dans cette édition, préparer un besoin et adopter un plan ne lance pas les agents.
Vérifier les rôles, les connexions, les espaces et les limites avant d’autoriser
l’exécution. Les programmes d’agents et leurs authentifications s’installent
séparément. Les [notes v0.1.0](docs/releases/v0.1.0.md) décrivent la release ;
l’identité du binaire lancé et **Version et nouveautés** permettent de distinguer
binaire, commit et sources locales. Une recompilation ne remplace pas un processus
serveur déjà en cours.

## Données et limites

Les missions et leur historique sont conservés sous `.swarm/` dans le projet,
notamment dans SQLite. Les fournisseurs externes reçoivent le contexte des appels
qui leur sont confiés : **local ne signifie pas que tous les modèles s’exécutent
sur votre machine**. Les secrets et liens de session doivent rester privés.
Les clés API de l’édition historique sont stockées localement avec permissions
restrictives, sans chiffrement au repos.

Les réservations d’espace peuvent sérialiser des tâches. Une consommation non
rapportée reste inconnue ; les quotas des sessions externes ne sont pas observés
automatiquement. Fermer le cockpit n’arrête pas les agents. Une destination ou
une méthode HTTP autorisée ne garantit pas à elle seule l’absence d’exfiltration
ou d’effet métier indésirable.

## Documentation

| Sujet | Référence |
| --- | --- |
| Prise en main | [Guide utilisateur](GUIDE-UTILISATEUR.md), [installation](INSTALL.md), [formation](docs/training/casa-pizza/README.md) |
| Maturité native | [Contrat Rust](docs/architecture/RUST-MIGRATION.md), [plan](docs/plans/migration-rust/PLAN.md), [preuves/rejeux](docs/plans/migration-rust/REPLAYS.md) |
| Architecture et conduite | [Deux UX](docs/PRODUCT-ARCHITECTURE.md), [supervision](docs/architecture/RUN-SUPERVISION.md), [communication](docs/AGENT-COMMUNICATION.md) |
| Contexte et autorité | [Mémoire](docs/PROJECT-MEMORY.md), [identité](docs/architecture/AGENT-IDENTITY-CONTRACT.md), [actions de confiance](docs/architecture/ACTIONS-CONFIANCE.md) |
| Préparation et méthodes | [Catalogue](docs/PREPARATION-TEMPLATES.md), [parcours produit](docs/PRODUCT-WORKFLOW.md), [méthodes](docs/AGENT-METHODS.md), [profil et skills](docs/PROJECT-PROFILE.md) |
| Contrôle et reprise | [Budgets](docs/BUDGETS.md), [contrôles métier](docs/ENGINE-BUSINESS-CONTROLS.md), [contrats bloqués](docs/CONTRACT-REVISION.md), [reprise de tentative](docs/ATTEMPT-RECOVERY.md), [stockage](docs/RUNTIME-RECOVERY.md) |
| Automatisation et API | [Programmes durables](docs/AUTOMATION-PROGRAMS.md), [référence CLI](REFERENCE.md), [préparation](PREPARATION-UX.md), [workflows](WORKFLOWS.md) |
| Évolutions prévues | [File des projets APEX](docs/plans/FILE-APEX.md), [AC-02 enregistré](docs/plans/actions-confiance/securite-execution-apex.md), [suivi après bascule](docs/plans/migration-rust/AFTER-CUTOVER.md) |

Les méthodes fournissent une démarche ; elles n’accordent aucun droit d’exécution.
Leurs sources canoniques sont dans [`.claude/skills/`](.claude/skills/), leurs
commandes de préparation dans [`.claude/commands/`](.claude/commands/).

## Dépôt et vérification

- `rust/crates/swarm-engine` : moteur natif ; `swarm-cli` : CLI ; `swarm-web` : service HTTP.
- `web/`, `frontend/`, `locales/` : interface et traductions ; `tests/` : parcours publics et navigateur.
- `tools/verification/` : [recettes permanentes et conservation des preuves](tools/verification/README.md).
- `docs/` et `docs/en/` : contrats, guides, plans, diagrammes et médias.

Choisir les vérifications selon le contrat modifié et le profil réellement livré.
Les tests de fixtures, le build de production, la revue indépendante et
l’acceptation sont des preuves distinctes. `make test`, `make smoke`, `make test-process` et `make test-install` utilisent les vérifications Rust actuelles ; leur succès ne prouve pas toute la migration. Voir [contribution](CONTRIBUTING.md) et les
[limites de qualification](docs/COMMUNITY-QUALIFICATION.md).

## Licence

Code source disponible sous [PolyForm Noncommercial 1.0.0](LICENSE), selon ses
conditions. Les usages commerciaux non couverts nécessitent une autorisation
distincte ; le projet n’est pas présenté comme « open source » au sens de l’OSI.
Les versions déjà publiées sous Apache 2.0 et les composants tiers conservent
leurs droits : [précisions](docs/LICENSING.md), [notices tierces](THIRD_PARTY_NOTICES.md).
Pour un signalement sensible, consulter [SECURITY.md](SECURITY.md).
