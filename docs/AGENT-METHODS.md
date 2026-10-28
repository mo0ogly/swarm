# Méthodes de travail pour Codex, Claude et Swarm

[English](en/AGENT-METHODS.md) · [Guide utilisateur](../GUIDE-UTILISATEUR.md)

Ce dépôt fournit **treize méthodes** pour cadrer, réaliser, vérifier et améliorer
Swarm : parcours guidé, examen et amélioration, construction et examen de
spécification, revue du code, diagnostic, vérification, replanification et retour
d’expérience. Elles partagent des critères explicites, des décisions tracées et
des preuves observables, sans dépendance à une autre installation.

La préférence permanente de l’utilisateur est une amélioration autorécursive :
toute amélioration découverte a de la valeur et reste tracée, même sans panne.
Qualifier son origine et son bénéfice attendu, décider dans les droits du rôle,
réaliser ou transmettre, vérifier l’effet puis réexaminer le besoin et capitaliser
les leçons vérifiées. Une anomalie déclenche `replan`. Un report conserve son
propriétaire et sa condition de reprise. Ce contrat de méthode ne prouve pas une
amélioration autonome du moteur installé et n’accorde aucun droit supplémentaire.

Les preuves de compilation Go couvrent le code des paquets, y compris à la racine,
les ressources go:embed et les octets effectivement substitués par un overlay.
Une liste de dossiers supposée ne suffit pas. Avant chaque injection de défaut,
établir un témoin valide avec les mêmes préconditions : un refus commun ne prouve
pas la couverture de tous les défauts. Ces règles sont aussi dans le contrat
embarqué ; reconstruire le binaire pour les futures tentatives, sans changer les
preuves historiques ou les processus déjà démarrés.

Pour la migration Rust, [APEX](../.claude/skills/apex/SKILL.md) intègre la méthode adoptée le 22 octobre 2026 : contrats stricts et banc différentiel, réécriture par parcours, lots verticaux, preuves selon le risque, durcissement différé, plan court, fichiers propriétaires et préparation R6/R7. La bascule des écritures reste unique ; le service anticipé en lecture seule attend une confirmation explicite. Cette consigne persistante ne met pas à jour les contextes déjà démarrés. Les huit points sont conservés dans la référence conditionnelle APEX `references/rust-migration.md` : ils sont chargés pour la migration, sans gonfler toutes les instructions natives. Une préparation sans outils doit recevoir cette référence explicitement.

## Catalogue lisible

Les noms destinés aux utilisateurs sont ceux du [catalogue du README](../README.md#communication-et-méthodes). Les méthodes de spécification couvrent la rédaction du besoin et l’examen du plan. Les identifiants historiques ci-dessous servent uniquement aux commandes natives et à la compatibilité ; ils ne sont pas les noms à afficher dans l’interface.

<details>
<summary>Commandes natives et identifiants techniques</summary>

## Démarrer

Ouvrez Codex ou Claude **dans ce dépôt**, puis utilisez la commande adaptée :

| Besoin | Codex | Claude |
| --- | --- | --- |
| Cadrer PRD, parcours, stories, architecture et plan | `$product-planning` | `/product-planning` |
| Réaliser recherche, conception et tâches autorisées | `$product-delivery` | `/product-delivery` |
| Examiner couverture, conception et preuves | `$product-review` | `/product-review` |
| Analyser, planifier et réaliser un changement | `$apex` | `/apex` |
| Auditer avec des preuves reproductibles | `$audit-pdca` | `/audit-pdca` |
| Rédiger un besoin et ses critères vérifiables | `$spec-builder` | `/spec-builder` |
| Découvrir des tarifs officiels avec leurs conditions | `$pricing-discovery` | `/pricing-discovery` |
| Trouver les trous d'un plan | `$spec-audit` | `/spec-audit` |
| Examiner le code et ses risques | `$code-reviewer` | `/code-reviewer` |
| Diagnostiquer un défaut avant une correction ciblée | `$debug` | `/debug` |
| Vérifier le parcours après une correction | `$verify-fix` | `/verify-fix` |
| Reprendre un plan bloqué | `$replan` | `/replan` |
| Comprendre les résultats et prioriser la suite | `$retex-analyzer` | `/retex-analyzer` |

Exemples à saisir dans la conversation :

```text
$apex --plan-only Définir une reprise après quota, avec les mêmes règles web et CLI.
$audit-pdca Auditer l'acceptation des résultats ; rapport seulement, sans correction.
$audit-pdca --fix Corriger les défauts confirmés dans ce périmètre et rejouer les contrôles.
$verify-fix Vérifier que le refus de lancement ne crée ni agent ni réservation.
$replan Reprendre le plan existant à partir de l'échec observé, en conservant ses limites.
```

Dans Claude, remplacez `$` par `/`. Les indications `--plan-only`, `--resume`,
`--fix` et `--score-only` sont des consignes pour la méthode, pas de nouvelles
options de l'exécutable `swarm`. L'audit est en lecture seule par défaut.
Les méthodes répondent dans la langue de la demande.

</details>

## Depuis l'interface Swarm

Dans **Préparer avec l'IA**, la liste des méthodes existante utilise :

| Méthode de préparation | Contenu fourni à l'IA |
| --- | --- |
| Analyse et planification | Analyse et planification du besoin |
| Parcours de création d’application | Cadrage produit, parcours/stories, architecture, conception et plan ; aucune exécution en préparation |
| Parcours guidé — préparer une évolution | Cadrage, critères et décomposition en tâches |
| Examiner et améliorer — préparer l’examen | Définition du périmètre, des risques et du plan de contrôle |
| Diagnostiquer et corriger un problème | Hypothèses, preuves manquantes et plan de diagnostic ; aucune correction en préparation |

Le fichier de méthode et le contrat commun sont transmis intégralement au modèle.
Les phases d'exécution des méthodes restent interdites dans cette préparation :
elle propose un brief ou un plan, sans lancer ni accepter des agents.
Les autres méthodes s’utilisent dans les sessions natives ou le cadrage des rôles ; elles ne sont pas des boutons supplémentaires de préparation. Le [guide produit](PRODUCT-WORKFLOW.md) précise les étapes du parcours de création d’application.

Le CLI expose le même catalogue :

```sh
swarm --root "$PWD" --json prepare methods
```

Chaque entrée indique `available` et une empreinte `sha256`. `audit_pdca` reste
un alias de `audit-pdca` dans Swarm et `/audit_pdca` dans Claude. Des alias historiques restent acceptés pour les anciennes intégrations ; utilisez les méthodes de spécification pour les nouveaux parcours.

Les cinq méthodes de préparation sont embarquées dans le binaire : elles sont
utilisables dans un projet vide, en natif comme dans Docker. Aucun répertoire
`.claude` n’est créé dans votre projet. Pour chaque fichier, une personnalisation
présente dans la racine du projet remplace la version embarquée ; si ce fichier
est absent, Swarm utilise sa version intégrée. Les fichiers locaux doivent être
réguliers, UTF-8, non vides et de 128 Kio maximum. Les liens symboliques, fichiers
invalides ou accès refusés rendent la méthode indisponible, sans remplacement
silencieux. Une modification ou un retrait change l’empreinte du contexte.
La préparation reste limitée à l’analyse et à la planification.

## Cadrage automatique des agents par le moteur

Chaque modification inclut la documentation concernée dans le même incrément,
y compris pour les méthodes et la configuration. Si aucune mise à jour ne
s'applique, la raison figure dans le suivi existant. Versions FR/EN et schémas
restent cohérents ; leur présence ne prouve pas l'effet ni l'acceptation.

Les règles personnelles communes, importées par `~/.claude/CLAUDE.md`, et les
règles du dépôt imposent aussi les docstrings/Rustdoc/commentaires Go des fonctions
publiques ou non triviales créées ou modifiées : but, entrées/préconditions,
sorties, erreurs, effets et limites. Vérifier les liens et exemples exécutables.
Ces fichiers persistants cadrent les prochains contextes qui les lisent ; cette
mise à jour ne modifie pas le contexte d’un processus Claude déjà lancé ni les
instructions embarquées du moteur. Le contrat embarqué n’est pas changé ici.

Pendant toute la migration, chaque incrément conserve en plus une revue des
politiques codées en dur, des contrats documentaires et des chemins d’erreur,
liée aux empreintes des fichiers examinés. Les recherches automatiques fournissent
des candidats ; la revue distingue données de test, invariants, limites techniques
et politiques opérateur. Une politique doit être suivie de sa configuration
persistée à son consommateur réel et à son réglage Admin. Vérifier aussi les
pages FR/EN, schémas, notes d’évolution et exemples concernés, ou justifier
explicitement l’absence de changement. Tout point non vérifié reste ouvert ;
aucun scanner ni commentaire ne garantit à lui seul la conformité. Une découverte
impose de réexaminer les lots antérieurs concernés, sans altérer leurs preuves.

Le collecteur des qualifications de migration impose aussi une référence précédente
explicite et une revue fraîche sur les quatre axes ; une couverture absente ou
un constat ouvert refuse la qualification de cet incrément. Les constats antérieurs
restent à traiter dans le plan avant toute acceptation globale.

Pour les nouvelles tentatives, le moteur inclut les méthodes adaptées au rôle dans
le contexte envoyé au fournisseur. Il ne dépend pas de la découverte des commandes
par Codex ou Claude pour fournir ce cadrage.

| Rôle | Méthodes incluses | Limites |
| --- | --- | --- |
| Planificateur et sous-planificateur | Parcours guidé, examen et amélioration, construction et examen de spécification, replanification, diagnostic, planification produit | PLAN et ACT ; sans outils ni modification de code |
| Exécutant | Parcours guidé, examen et amélioration, vérification, diagnostic, modèles de rapport et de suivi, réalisation produit | DO et CHECK dans le périmètre autorisé ; audit seul si demandé |
| Vérificateur indépendant | Examen et amélioration, revue du code, revue produit | CHECK sur les preuves fournies ; sans outils ni modification du candidat |

Le champ `workflow` conserve la version, le rôle, la liste des méthodes et leur
empreinte SHA-256. Le moteur refuse un rôle inconnu ou un cadrage dépassant sa
limite. Les méthodes sont **embarquées à la compilation** : modifier celles d'un
espace exécutant ne remplace pas les consignes du moteur. Une modification de ce
pack exige une reconstruction du binaire pour les futures tentatives.

La préparation utilise le pack embarqué ou des fichiers locaux valides. Les anciennes
tentatives ne reçoivent pas rétroactivement ces métadonnées. Le champ prouve le
cadrage construit, pas l'obéissance du modèle. Les contrôles exécutables restent
nécessaires.

Le [protocole de communication](AGENT-COMMUNICATION.md) détaille les rapports
complets transmis aux planificateurs, les reçus de contexte et les décisions liées
aux retours, ainsi que leurs limites.

## Une source commune, deux points d'entrée

```text
AGENTS.md                         règles chargées pour Codex
CLAUDE.md                         point d'entrée pour Claude
.claude/skills/<nom>/SKILL.md      source canonique des douze méthodes
.agents/skills/<nom>              lien relatif vers la même méthode
.claude/skills/<nom>/agents/      noms et exemples du sélecteur Codex
.claude/commands/                 commandes natives et alias de compatibilité
tools/agent-workflows/CONTRACT.md règles communes, également lues par Swarm
```

Les liens restent internes au dépôt et se transportent avec Git. Les copies qui
ne préservent pas les liens symboliques doivent les rétablir avant utilisation.
Modifiez la source canonique, puis lancez :

```sh
python3 tools/agent-workflows/check.py
git diff --check
```

Le contrôleur vérifie les liens et les sources puis exécute les intégrations
Rust `agent_workflow`. Neuf tests nommés sont requis, dont un pour chaque rôle
planner/subplanner/worker/reviewer ; chacun vérifie le contexte réellement
construit, pas un rôle simplement déclaré. Une collecte vide, un rôle manquant,
un test ignoré/échoué ou des compteurs incohérents refusent. Les codes d’échec
d’origine et les deux canaux sont conservés ; aucun watchdog global n’est ajouté.
Cargo utilise le cache/environnement du lanceur, avec dépendances offline/locked.
Les gardes Python se rejouent par `python3 -m unittest discover -s
tools/agent-workflows -p test_check.py -v` (doubles de sortie Rust, sans modèle).
Ces doubles ne prouvent pas l’exécution native ni une acceptation indépendante.

Une modification des méthodes ou du contrat change leur empreinte. Une préparation
ancienne doit appliquer la nouvelle version et refaire sa validation ; les preuves
précédentes ne deviennent pas actuelles par simple changement de fichier.
Si une méthode n'apparaît pas dans une session native, vérifiez la racine, les liens
et les éventuelles méthodes personnelles du même nom, puis rouvrez la session.

## Ce que cette configuration garantit — et ses limites

Les méthodes demandent des responsabilités distinctes : le planificateur organise,
l'exécutant réalise, le vérificateur indépendant examine, le moteur accepte selon
ses contrôles. Une auto-revue dans la conversation de l'auteur reste une auto-revue.
Le contrat interdit de transformer une limite de tentatives, un quota ou une preuve
manquante en réussite en changeant les règles.

Ces instructions guident l'IA ; **elles ne remplacent pas les verrous du moteur**.
Un test avec un faux fournisseur ne démontre pas une mission autonome réelle.
Une méthode n'accorde pas d'autorisation supplémentaire de délégation, publication
ou déploiement. Le modèle et les identifiants restent ceux du fournisseur/session
configuré ; aucun modèle ni mode de permissions n'est imposé par ce pack.

L'organisation planificateurs/exécutants s'inspire des travaux de Cursor. La revue
indépendante et les conditions d'acceptation sont le contrat propre de Swarm ;
ce pack ne constitue pas une certification Cursor.

## Formats et sources

- Adaptation locale : méthodes APEX, audit-pdca, spec-builder, spec-audit,
  code-reviewer, verify-fix, replan et retex-analyzer de la configuration LIA.
  Les chemins machine, règles produit et automatismes de publication sont exclus.
- [Documentation Codex : découverte des skills](https://learn.chatgpt.com/docs/build-skills).
- [Documentation Claude : skills de projet et commandes](https://code.claude.com/docs/en/skills).
- [Contrat partagé de Swarm](../tools/agent-workflows/CONTRACT.md).

### Budget local et poursuite des autres périmètres

Un sous-planificateur qui atteint son plafond d'activations conserve ses retours
non traités et son historique. Le conducteur ne le rappelle plus et poursuit les
périmètres dont le budget, ainsi que celui de leurs parents, reste disponible.
Ce plafond local ne doit pas créer une panne globale de planification. La
réservation transactionnelle vérifie de nouveau les limites avant chaque appel.
Si aucun périmètre ne peut agir, les sondages ne consomment aucun appel et ne
modifient pas la mission. Un budget global épuisé, un fournisseur indisponible ou
une véritable panne de planification restent des conditions distinctes ; cette
règle ne les efface pas et n'ajoute aucune tentative.

### Ordre des retours en attente

Le conducteur choisit le propriétaire du plus ancien événement non traité dans
l’ordre durable de la boîte de réception. Un message de reprise récent au
coordinateur ne dépasse donc plus les résultats déjà reçus par un sous-planificateur.
Les périmètres fermés, réservés, au plafond ou en attente d’intégration restent
exclus du départ ; leurs événements sont conservés. La réservation revérifie les
limites avant l’appel. Cet ordre ne clôture aucun périmètre à la place du
planificateur et ne rend pas de budget aux tentatives précédentes.

### Retour au coordinateur après clôture d’un enfant

Le contexte du coordinateur distingue les exigences encore possédées des exigences
déléguées. `task_capacity_remaining` indique des places de création disponibles,
pas des tâches restant à terminer. `descendant_validation` transmet les identités
des tâches descendantes, leur état, la fraîcheur de leur acceptation calculée par
le moteur et l’identité/révision de leur revue éventuelle. Un statut « accepted »
sans preuves actuelles produit `accepted_fresh: false`. Ce résumé ne remplace pas
une revue indépendante : il permet au coordinateur de proposer la clôture ; le
moteur contrôle de nouveau toutes les preuves avant de l’appliquer.

### Reprise exceptionnelle après livraison incomplète

Après trois tentatives consommées, `planning authorize-recovery` permet une seule
reprise explicitement confirmée par l’opérateur. Pour un résultat refusé avant la
revue payante, fournir `result_commit` (SHA du résultat immuable) à la place de
`review_id`, avec `attempt_id`, `confirm_recovery: true`, motif et nouvelle consigne.
Le moteur vérifie l’attribution Git, la dernière tentative arrêtée, le refus de
complétude réel et la disponibilité du vérificateur. Il conserve les trois essais,
ajoute au plus une tentative, ne lance aucun agent lors de l’autorisation et
n’accorde aucune validation. Les contrôles et la revue du résultat corrigé restent
obligatoires. Une seconde dérogation, un résultat remplacé ou une revue déjà rendue
sur ce résultat sont refusés par cette voie. L’opération est disponible par le CLI
et l’API publique ; le formulaire existant de refus de revue reste inchangé.

### Budget d’un lancement préparé

La reprise explicite par `agent resume-launch` conserve l’identité et la copie préparées. Si le nombre d’appels d’outils demandé dépasse le plafond du fournisseur inchangé, le moteur réduit ce nombre au plafond autorisé. Un budget plus petit reste inchangé. Le manifeste initial conserve le budget demandé ; la tentative enregistre le budget réellement appliqué. Les autres paramètres, la révision du travail, les preuves et l’identité du fournisseur restent contrôlés. Une double reprise retourne la même tentative.

### Consulter un stockage créé par un ancien moteur

Les commandes courantes de consultation (`work show/list`, `planning show`, `mission status/preview`, `agent show/list/logs/prepared`, listes et historiques de préparation, état des espaces et des échanges) refusent désormais de migrer implicitement un stockage plus ancien. Elles retournent `storage_upgrade_required`, sans changer sa version. Utiliser le CLI correspondant au serveur actif pour consulter cette mission. Pour une mise à niveau explicite, arrêter d’abord les anciens processus, puis lancer `swarm init` avec le nouveau CLI. Les commandes de mutation et le démarrage du serveur conservent leur mécanisme de migration existant : ne pas mélanger leurs versions sur un même stockage.

## Plafonds de planification et de vérification

Lors de `planning enable`, le JSON accepte `max_activations` pour les
planificateurs et `max_review_calls` pour le vérificateur indépendant. Par exemple,
`"max_activations": 12, "max_review_calls": 4` fixe deux plafonds distincts.
`max_review_calls` exige un fournisseur et une valeur entre 1 et 100. Une valeur
invalide ou fournie pour une autre action est refusée avant modification.
L’omission (ou zéro) conserve le comportement historique : le plafond de revue
est celui des activations, limité à 100. Aucun appel IA ne part à la configuration.
Cela ne modifie pas les budgets des missions existantes et ne rembourse aucun appel.

## Recette du moteur

Voir [les six recettes E1–E6](ENGINE-ACCEPTANCE.md), leurs prérequis et la distinction entre fixtures déterministes, contrôle navigateur et essai avec des agents réels.

## Diagnostic structuré

Le parcours « Corriger un défaut reproductible » sélectionne la méthode
« Diagnostiquer et corriger un problème ». Le moteur embarque cette méthode pour
les nouveaux planificateurs, sous-planificateurs et exécutants. Elle s’applique
aux anomalies : reproduire, isoler, tester une cause, corriger si autorisé, vérifier.
Une préparation reste sans outils ; le vérificateur conserve sa méthode de revue
indépendante. L’inclusion et son empreinte prouvent le contexte fourni, pas le
respect effectif de la méthode ni la réussite d’un diagnostic.

La version portable adapte la skill Debug installée localement, sans connecteurs
obligatoires, chemins machine ou dépendance à une autre installation.

### Vérification d'un moteur pendant un changement web parallèle

Les ressources web et les méthodes embarquées partagent actuellement le paquet
racine `resources.go` : une modification web peut invalider des entrées de
compilation des tests moteur. Pour figer une preuve, un overlay de compilation
peut conserver les ressources observées et leur liste explicite, sans modifier
le checkout partagé. Déclarer cette vue et ses empreintes ; elle ne vérifie pas
l'interface web en cours d'édition. Conserver un binaire de test compilé permet
de répéter les contrôles ciblés sans relier le paquet à chaque commande.
Distinguer compilation, tests collectés, résultats partiels et résultat final.
Voir le [diagnostic IA3b](plans/actions-confiance/verification-identites-agents-20261016/ia3-issuer-20261017/RAPPORT-IA3b.md).


### Justifier un délai de vérification

Tant que les limites du code ne sont pas comprises, être prudent avec les
timeouts temporels et les autres blocages ajoutés aux tests : tentatives,
itérations, volume, concurrence ou conditions d’arrêt. Identifier origine,
portée et risque protégé ; observer la progression et mesurer avant d’ajouter
ou de durcir une borne. Distinguer lenteur, blocage réel, limite fonctionnelle
et limite de la méthode. Un seuil arbitraire ne remplace pas un diagnostic ;
aucune protection existante de sécurité, de droits ou de budget n’est contournée.

Un watchdog de test doit être lié à une exigence observable et à un coût
justifié ; ne pas reprendre un nombre par commodité d’une recette précédente.
Le distinguer d’une politique du moteur et d’un objectif de performance. Sans
borne justifiée, utiliser explicitement `-test.timeout=0` pour désactiver le
délai global Go, suivre les événements nommés et diagnostiquer une absence de
progression ; cela ne supprime pas les contrats des opérations individuelles.
Un timeout est une exécution incomplète, pas la preuve d’une assertion métier
fausse. Découper le lot peut aider le diagnostic mais ne justifie pas la borne.
Conserver les échecs originaux et le lien aux mêmes sources/binaire. Le
[replan IA3c.1](plans/actions-confiance/verification-identites-agents-20261016/ia3-issuance-receipt-20261017/RAPPORT-IA3c1.md)
conserve l’objection utilisateur, le délai choisi sans preuve et sa suppression.

### Preuves des contextes natifs

`python3 tools/agent-workflows/check.py --evidence DOSSIER_NEUF` utilise la recette
permanente `tools/verification/recipes/agent-workflows-native.json`. Elle conserve
les sources déclarées Rust et textes embarqués, les commandes, canaux originaux
et leurs empreintes, sans écraser une preuve. Le profil Cargo et les dépendances
externes ne sont pas un graphe hermétique ou un profil production prouvé par
cette archive seule. Les sources sont comparées avant/après ; aucune identité,
conformité d’un modèle, livraison installée ou acceptation n’en découle.

Les anciennes preuves Go/delta restent historiques. `--evidence-baseline` refuse
explicitement : une baseline Go ne peut pas qualifier Rust. Omettre ce paramètre
produit une archive native complète. Les outils Go de conservation historiques
ne sont plus exécutés par ce vérificateur. Un contrôle sans --evidence affiche
les tests réels mais ne conserve pas leur archive.

### Historique des preuves Go107

107 qualifié localement :4tests Python de conservation/rejeu et29consommateurs
Rust (24admissions+5workflows) passent. Après une vraie régression de taille,
les corps mesurés sont31970/31973/31970/18605octets ; cap sauvegardé32000 inchangé.
Les5tests/sous-tests Go réels passent sur1163sources projet+1testmain généré,
archives et JSON originaux conservés, entrées stables avant/après. Une seconde
exécution passe en réutilisant la base avec0fichier nouveau ; les274entrées
manquantes à l’ancien inventaire sont désormais couvertes, sans réécrire celui-ci.
Binaire normal-dev isolé sans fixture/test-util,2HTTP réels PASS ; aucune installation.
Les overlays/GOFLAGS non pris en charge refusent ce profil ; compilation externe
C/système hermétique et revue indépendante non établies. Le défaut du vérificateur
parent-seul et les échecs originaux restent conservés. Preuves107/result.json.
Cette correction de preuve ne termine pas la migration. Suite : garde-fou de
l’organisation hiérarchique et de la revue, puis configuration/créneaux/consentement,
branches gérées et origine/gate avant conduite automatique. R6/R7 et RETEX ouverts.

Vérification native138 : le profil d’intégration Root courant requiert Linux et
la chaîne Rust du projet. Le mode --evidence rejoue aussi huit gardes Python depuis141,
dont un enfant Python possédé qui fournit un vrai statut SIGTERM ; ce dernier
ne prouve pas un arrêt Cargo. Le statut signé original est conservé dans le
diagnostic, et le code CLI suit la convention POSIX128+signal. Les erreurs de
compilation conservent leur code d’origine. Aucun plafond temporel n’est ajouté.

141 : la recette native globale ne dépend plus du minuscule module Go utilisé
pour tester l’ancien archiveur. Trois tests de cet outil restent accessibles
uniquement par legacy-go-source-archive.json (Go requis, non exécutés par la cible
native). Le contrôle de routage des rôles a migré vers un shim Cargo/piège Go
possédé, distinct des neuf vrais tests Rust. Le contrat incorporé référence
désormais la preuve native ; il faut reconstruire pour les futurs contextes.
Les processus et prompts déjà émis ne sont pas réécrits. Cette modification
ajoute8octets au corps worker31970, sous le cap sauvegardé32000 inchangé ;
la mesure runtime après reconstruction reste requise.

## Installation locale par défaut — décision utilisateur du 24 octobre 2026

Pour un développement autorisé, toute avancée vérifiée doit être construite,
installée et activée dans l’instance locale concernée par défaut, sans nouvelle
demande d’accord. Cette autorisation couvre le démarrage ou redémarrage du seul
service local identifié comme appartenant au projet. Utiliser le chemin de
livraison pris en charge ; préserver données, missions, travaux et processus
d’autrui. Vérifier l’identité du binaire réellement exécuté, la version, les
évolutions FR/EN et le parcours affecté avant d’annoncer « installé ».
Un test réussi ou un candidat compilé ne constitue pas une installation.
Si le raccordement de livraison manque ou refuse, appliquer replan et le traiter
dans le périmètre autorisé ; conserver le défaut, sa preuve et sa condition de
reprise sans prétendre que le produit est installé. Cette préférence ne vaut pas
autorisation de commit, push, publication ou déploiement externe/production.

Avant de modifier des consignes embarquées, mesurer leurs corps par rôle avec `python3 tools/agent-workflows/check_workflow_size.py` avant compilation, puis vérifier les véritables contextes Rust avec `python3 tools/agent-workflows/check.py`. Une taille admissible ne prouve ni réception ni respect des consignes. Conserver la politique opérateur : réduire les répétitions sans perdre les obligations ; une capacité insuffisante exige un diagnostic. Le lot1438 a montré qu’un ajout de règle peut casser les rôles alors que le serveur web répond encore.
