# M20 — raccord aux identités et aux droits d'usage

17 octobre 2026. Codex, chantier mémoire ; `replan` puis `spec-builder`.
**Spécification cible fondée sur le code livré, pas contrôle mémoire activé.**
Les sources identité restent en lecture seule : la conversation dédiée poursuit
IA3b. Aucun endpoint, champ public, rôle ou autorisation nouveau n'est déclaré
disponible par ce document.

## Socle désormais disponible

Le [contrat d'identité](../../architecture/AGENT-IDENTITY-CONTRACT.md) et les
sources privées fournissent des objets concrets :

| Producteur inspecté | Contrat réel | Emploi mémoire attendu |
| --- | --- | --- |
| `agent_identity_contract.go` | `agentIdentityPrincipal`, `agentIdentityActor` : principal, émetteur, sujet, catégorie et éventuelle instance. | Attribuer depuis le contrôle authentifié ; jamais depuis un nom ou le fournisseur. |
| Même fichier | `agentIdentityPermission` reconnaît `memory.read`, `memory.summarize`, `memory.transmit`, avec ressource exacte. | Trois droits distincts ; aucune implication générale entre eux. |
| Même fichier | `agentIdentityRequest` lie portée, octroi, opération, ressource et `ContentDigest`. | Construire le digest depuis les sources et la destination réellement résolues ; ne pas accepter celui du demandeur comme preuve de correspondance. |
| `agent_identity_authority.go` | Évaluation, délégation atténuée, révocation et reprise d'une décision. | Réutiliser le noyau ; une décision favorable n'est pas une transmission effectuée. |
| `agent_identity_registry_store.go` / `agent_identity_registry_types.go` | Instantané durable, tête Trust, checkpoint privé et lecteur de plan courant. | Ne pas reconstruire l'autorité depuis la mémoire ou l'archive à vérifier. |
| `agent_identity_runtime_linux.go` | Liaison runtime privée et contrôle d'appelant. | Fournir l'instance attestée, sans accepter un identifiant déclaré par l'agent. |

Ces composants privés ne prouvent pas le bootstrap, la protection hôte ni le
raccord aux parcours publics. Les résultats IA2/IA3 gardent leurs reçus ; aucun
de leurs tests n'est relancé ou compté comme test mémoire à cette reprise.

## Attribution, propriété et permission

- **Propriétaire du cadrage** : responsabilité logique stable de la mission ou du
  périmètre, avec autorité de gestion explicitement attribuée. Le modèle doit
  rattacher cette responsabilité à un principal habilité ; le fournisseur ou
  l'agent remplaçable ne suffit pas.
- **Auteur d'une intervention** : acteur attesté à sa création, avec provenance
  et événement d'enregistrement. La révocation ultérieure retire des droits
  courants ; elle ne réécrit pas l'auteur historique.
- **Usage** : décision courante sur une source et un destinataire précis. Être
  propriétaire ou auteur ne donne pas automatiquement tous les usages.

Types persistés, opérations et migration restent à définir. Sans preuve, les
anciens échanges gardent leur attribution inconnue ; une ligne d'agent ou un
`attempt` ne devient pas rétroactivement une identité authentifiée. Toute
attribution ultérieure est un événement distinct, motivé et autorisé.

## Contrat du consommateur à réaliser

Le consommateur doit résoudre ensemble l'appelant et sa liaison d'instance,
l'autorité et ses révisions, la portée, les sources originales/versions et leurs
empreintes, la propriété/règle d'usage, l'opération et la destination réelle
(projet, tâche, périmètre et instance lorsque disponibles). Le digest doit lier
le contenu effectivement consulté, résumé ou transmis à tous ces paramètres.

Le format canonique d'une ressource mémoire exacte et de cette proposition reste
à spécifier côté mémoire puis à vérifier avec le noyau. Les ressources actuelles
interdisent `*?[]` : ne pas prendre un JSON arbitraire ou un préfixe de chemin pour
un sélecteur autorisant. Contrôler chaque source d'une liste ; un refus ne donne
ni paquet partiel présenté comme complet ni contexte vide présenté comme succès.

Lire ne permet pas de résumer ou de transmettre. Une synthèse qui lit ses
sources nécessite les droits de lecture et de résumé correspondants. Une
transmission contrôle aussi l'autorité du destinataire à recevoir ce contenu.
Ces règles sont des exigences, pas des permissions déjà appliquées.

## Deux contraintes établies dans le code

### Fraîcheur du plan et fraîcheur du texte

`agentIdentityPlanProjectionDigest` retire volontairement `response`, `answers`,
`contexts` des tâches et `project_memory` du travail. Un plan inchangé ne garantit
donc pas un texte mémoire inchangé. Préserver cette séparation : vérifier les
empreintes de contenu et les décisions d'usage en plus du plan et de l'autorité.

Deux options : demander une décision puis charger le texte séparément, ou
résoudre texte et proposition dans le même consommateur fermé que l'évaluation.
La première laisse une course entre autorisation et diffusion. Retenir la seconde :
préparer les données dans la transaction possédée par le consommateur, évaluer
sur ces données puis vérifier l'engagement avant de les exposer.

Le lecteur `evaluateAgentIdentityRegistryMessage` réserve l'écrivain SQL pendant
la décision et suit **Trust → SQLite → runtime**. Son callback fermé n'appelle
pas Store et ne réentre pas en SQL. Le consommateur mémoire ne doit pas appeler
`memorySummarySource` depuis ce callback, ni utiliser une décision détachée comme
autorisation durable de relire d'autres octets.

### Le cadrage précède l'adoption du plan

`agentIdentityRecordedPlanDigest` exige aujourd'hui `Work.Plans` ou
`Work.Planning`, sinon `agent_identity_plan_unavailable`. La recette publique
M21 conservée contient pourtant deux cadrages sans aucune de ces formes de plan :
[`preplan-observation.json`](evidence/m20-20261017/preplan-observation.json).
C'est un usage initial à prendre en charge explicitement, sans déduire un droit
du comportement historique.

Un raccord au seul lecteur actuel créerait un cycle : le cadrage prépare le
plan, mais l'accès au cadrage exigerait déjà ce plan. Le chantier identité doit
fournir une autorité explicite de préparation avant plan ou un parcours humain
autorisé de mise en place. À prouver avant activation : aucune exception
`si aucun plan, autoriser`, ni octroi implicite par cookie ou rôle Admin.

## Critères et prochaine réalisation

Tous ces critères restent **NON TESTÉS pour les droits mémoire**.

| Critère | Déclencheur / effet requis |
| --- | --- |
| M20-U01 Attribution | Conserver acteur attesté et événement ; un champ auteur falsifié ne les remplace pas. |
| M20-U02 Héritage | Lecture/import ancien ne fabrique ni auteur, ni propriétaire, ni octroi local. |
| M20-U03 Droits distincts | Lecture seule refuse résumé/transmission ; sans lecture, aucune source exposée. |
| M20-U04 Destination | Changer tâche, instance, périmètre ou projet rend la décision incompatible ; pas d'autorité par fournisseur commun. |
| M20-U05 Texte modifié | Une réponse/version/annotation changée interdit l'emploi de l'ancien digest, même à plan inchangé. |
| M20-U06 Révocation | Expiration, révocation ou réattribution prise en compte avant effet bloque le nouvel usage ; auteur et réception historiques restent conservés. |
| M20-U07 Copies indirectes | Historique, brief, assistant, RETEX, import et résumé contrôlent leurs sources effectives. |
| M20-U08 Course et panne | Modification pendant attente ou échec d'engagement ne diffuse pas un autre état ; aucune réentrée SQL sous runtime. |
| M20-U09 Avant plan | Autorité de préparation explicite ; absence donne un refus expliqué, jamais un permis implicite. |
| M20-U10 Effet | Lier demande → décision → application → réception au contenu exact ; un texte journalisé ne prouve ni permission ni réception par le bon agent. |

Suite côté mémoire : définir ressource/proposition exactes et modèle d'attribution,
puis un consommateur interne fermé avec refus et concurrence. Dépendances côté
identité : autorité avant plan, émetteur/checkpoint protégés et frontière hôte.
Migration et raccord aux treize voies viennent ensuite, avec API/CLI/interface,
documentation FR/EN et recette d'effet. Les sources communes ne sont pas modifiées
tant que la session identité y travaille et que cette séparation n'est pas levée.

## Réalisation partielle M24 — données de demande exactes

`memory_usage_binding.go` réalise une première brique privée pour le pipeline
extractif M19, sans modifier les producteurs existants ni les fichiers identité.
`memorySummaryUsageBinding` reconstruit l'attachement et le paquet source depuis
un même `Work` possédé par le consommateur. La liaison porte sur le projet local,
la révision, la tâche et la portée, le principal/l'instance destinataire, le
paquet source, la sélection et les octets de l'attachement. Ces données ne sont
ni une authentification de la destination ni une adoption de plan.

Depuis la correction M25, une ressource exacte vaut `memory:v2:` + SHA-256 du
tuple JSON versionné `swarm-memory-resource-v2`, `project_context_digest`, `work`,
`task`, `id`, `kind`, `role`. `task` désigne la tâche source réelle : celle du
snapshot pour l'instantané opérateur, celle de `DialogueRef` pour un dialogue.
Deux tâches ne partagent donc pas leur ressource snapshot ; le même dialogue
conserve sa ressource quand seule la tâche destinataire change. La v1 privée
confondait les snapshots de tâches d'un même travail ; sa preuve d'échec reste
dans M25. Aucun octroi v1 n'est converti ou autorisé implicitement.
Ce tuple utilise les IDs produits par `memorySummarySourceForWork`, sans accepter
une citation renommée. La proposition `swarm-memory-usage-v2` conserve les
empreintes des contenus indépendamment du digest de plan. Le hash du chemin
projet reste une liaison locale ; il ne crée aucune racine de confiance.

`requests(eventID, grantID)` construit des `agentIdentityRequest` séparées. Toutes
portent le digest complet de proposition : modifier le contenu, la sélection,
la tâche ou le destinataire rend une ancienne décision incompatible. Les
événements enfants sont `memory-` + SHA-256 du tableau JSON versionné
`swarm-memory-usage-event-v1`, événement parent, ressource, opération.

Cette préparation lit et reconstruit toutes ses sources : elle demande lecture
et synthèse séparément pour chacune ; seuls les IDs reçus selon le reçu M19
demandent aussi transmission. L'instantané opérateur et les omissions ne sont
pas des sources transmissibles. Cette liste décrit le pipeline M19 ; elle ne
déclare pas que `memory.read` implique `memory.summarize` ou `memory.transmit`.
Toute entrée invalide renvoie une liaison vide ou aucune demande, jamais un
paquet partiel utilisable. Une liaison modifiée après construction est refusée.

Cette brique ne renvoie pas les textes sources et n'effectue ni SQL, migration,
autorisation, effet ou écriture de reçu. Aucun chemin public ne la consomme
encore. La portée et le destinataire restent des données à résoudre et vérifier
depuis le contrôle protégé ; la vérification de l'autorité du destinataire à
recevoir reste distincte. M20-U04/U05 progressent au niveau des liaisons et du
noyau pur ; les dix critères de conformité publique restent ouverts, y compris
propriété/auteur persistés, avant-plan, révocation avant effet, transaction et
réception. Recettes et résultats : `INCREMENT-MEMOIRE-20.md` et
`RESULTATS-MEMOIRE-20.md`.

## M25 — verdict complet et droit de lecture du destinataire

`evaluateMemorySummaryUsage` reconstruit la liaison sur le `Work` fourni au
moment du contrôle, puis évalue tous les droits du demandeur et le droit de
lecture du destinataire sur les seules sources transmises. Son `memoryUsageFrame`
privé contient instantané, autorité, deux appelants et instant de contrôle ; il
doit provenir du futur consommateur protégé, jamais d'une déclaration JSON.
Le principal et l'instance destinataire doivent correspondre exactement à la
proposition ; le noyau vérifie ensuite octroi, instance/canal, portée et fraîcheur.

Les événements de lecture destinataire utilisent un domaine distinct
`swarm-memory-receive-event-v1`. Un seul refus renvoie une décision entièrement
vide, même après plusieurs contrôles favorables. Aucun acteur absent ne devient
un opérateur implicite. Le destinataire n'obtient ni snapshot opérateur ni source
omise, et n'a pas besoin du droit de synthétiser pour simplement recevoir.

La décision privée conserve tous les verdicts et l'expiration la plus proche
des octrois ou sessions. Son réemploi reconstruit les sources et réévalue les
deux acteurs avant de comparer l'ensemble de la preuve. Une preuve partielle,
modifiée, réordonnée ou liée à une autre autorité est refusée. Les refus du noyau
conservent leur cause. Ce verdict vaut pour l'instant fourni ; le consommateur
devra encore contrôler le temps et l'autorité au point réel d'engagement.

Il ne s'agit pas d'un engagement atomique : aucun texte libéré, reçu consommé,
journal durable ou effet. La provenance protégée du cadre, l'affectation réelle
du destinataire à la tâche, propriété/auteur persistés et l'autorité avant plan
restent à raccorder. Aucun handler ou lanceur public ne consomme cet évaluateur.
M20-U01–U10 restent ouverts pour la conformité applicative. Voir
`INCREMENT-MEMOIRE-21.md` et `RESULTATS-MEMOIRE-21.md`.
