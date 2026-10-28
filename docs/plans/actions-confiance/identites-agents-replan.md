# R01/R13 — identité des agents et autorité de création

16 octobre 2026. Propriétaire du chantier : Codex, développement direct sans
délégation. Méthode : `replan` et APEX pour la tranche de réalisation à venir.
Statut : IA1, audit initial et sept contre-exemples reproduits en environnement
isolé ; **contrôle de création des agents non implémenté**. Voir le
[rapport IA1](verification-identites-agents-20261016/RAPPORT-IA1.md) pour les
preuves, la couverture partielle des scénarios et les limites.

Reprise du 17 octobre : le noyau privé IA2.2 est écrit et vérifié localement
(100 noms de tests/sous-tests, plus 14 contrôles existants). Le
[rapport IA2.2](verification-identites-agents-20261016/ia2-contract-kernel-20261017/RAPPORT-IA2.2.md)
conserve aussi deux défauts introduits, reproduits puis corrigés avant livraison.
Le [contrat concret FR/EN](../../architecture/AGENT-IDENTITY-CONTRACT.md) décrit
types, source Trust, délégation, révocation, rejeu et contexte de projet.
**Aucun lanceur n'est raccordé : F01–F07, IA2.3 et IA3–IA5 restent ouverts.**

Décision du 17 octobre : service local isolé pour le développement/pilote,
autorité distante prévue via configuration initiale et UX Admin. Voir IA3c.2f
en fin de plan ; choix adopté, service et écrans non livrés. Les mentions plus
anciennes de frontière à choisir conservent leur état historique.

Précision utilisateur du 17 octobre : aucun fournisseur d’identité disponible.
Le parcours doit fonctionner avec une authentification humaine locale par défaut,
configurable depuis l’UX initiale puis Admin, sans fournisseur externe imposé.
Voir la reprise IA3c.2f-04 ci-dessous : décision adoptée, assistant et gestion
des comptes encore à réaliser ; aucune activation ni identité humaine vérifiée
déduite de cette décision.

Décision ultérieure du 17 octobre : migration complète Go → Rust avec une seule
bascule finale. Le [plan de migration](../migration-rust/PLAN.md) conserve cette
décision et les preuves des ports de données et du protocole d’authenticité. Les critères d’identité et
le compte humain local configurable restent requis ; la migration de langage
ne les clôt pas. Le moteur Go actuel demeure la référence utilisable.

## Besoin et périmètre

Demande utilisateur : empêcher un attaquant de créer un agent présenté comme
légitime ; empêcher aussi un agent légitime de créer un agent non autorisé.
Une création hors plan nécessite une explication et une décision humaine
explicite avant tout lancement. Ces exigences appartiennent à la gestion des
identités de l'application et couvrent les usages développement et métier.

L'ajout conserve R01/R13 et les preuves historiques des identités de confiance.
Il étend leur périmètre au cycle de vie des agents. Il est également un préalable
à [M20, propriété du cadrage](../deepagents-integration/INCREMENT-MEMOIRE-16.md) :
propriétaire logique, auteur attesté et droits d'usage sont trois informations
distinctes. Un nom, un fournisseur, un identifiant, un rôle, une filiation ou
une empreinte de contenu ne constitue pas une preuve d'identité ni un droit.

## État observé et première frontière manquante

| Source inspectée | Observation | Conséquence |
| --- | --- | --- |
| `internal/engine/agents_store.go`, `prepareLaunch` | `event_id` lie les reprises au contenu ; validation de rôle, tâche, révision et gardes de lancement. | L'idempotence n'authentifie pas le demandeur ; raccorder identité et autorisation au contrat de création. |
| `internal/engine/console_cli.go`, branche `agent` ; `web_server.go`, lancements et reprises | Les chemins rejoignent la préparation d'un lancement. | Une validation dans le seul écran serait contournable ; inventorier aussi les autres appelants. |
| `trust_identity_context.go`, `trust_workload.go` | Contexte authentifié séparant `human`, `service` et `local`, sessions et révocation. | Réutiliser ces producteurs ; ne pas accepter d'identité déclarée dans le corps de la demande. |
| `agents_process.go`, `spawnAgent`, `supervise`, `providerEnvironment` ; `agent_build_cache.go`, `workerLaunchEnvironment` | Processus fournisseur et environnement préparés ; le lancement général n'applique pas le confinement du helper Trust. | Un agent exécuté sous le même compte système ne doit pas pouvoir lire ou modifier le plan de contrôle et ses secrets. Cette séparation reste à construire et à vérifier. |
| `model.go`, `brainstorm.go`, `dialogue_context.go`, `memory_summary_source.go` | Attribution technique partielle et chemins de diffusion du cadrage. | Légitimité de l'agent et autorisation d'utiliser chaque source doivent être contrôlées séparément. |

Sonde locale en lecture seule de capacité : `bwrap --unshare-user --unshare-pid
--ro-bind / / --proc /proc -- /bin/true` sort avec le code 0. Cela établit
seulement la disponibilité de cette primitive sur cette machine, sans prouver
le confinement d'un fournisseur, la protection de `.swarm` ou la portabilité.
La télémétrie eBPF et le helper scellé existant ne prouvent pas cette protection.

L'audit des appelants demeure ouvert : `console.go`, `console_dialog.go`,
`dispatcher.go`, `assist_action.go`, `dialogue_restore.go`, `pilotage.go`,
`mission_status.go` et les reprises de lancements préparés. Un inventaire de
symboles ne vaut pas preuve de leur couverture effective.

## Invariants requis

1. Le moteur authentifie le demandeur et attribue lui-même l'identité d'une
   instance d'agent, liée à sa mission, sa tâche, sa tentative et son autorisation.
   Distinguer l'agent logique, ses instances et le service fournisseur. Un
   fournisseur partagé n'est pas une identité individuelle d'agent.
2. Être authentifié ne donne aucun droit de création implicite. Toute création
   nécessite une autorisation courante, limitée à son contenu exact, au périmètre,
   au plan/version concernés, aux capacités et aux ressources accordées. Le
   parent ne délègue que les droits explicitement délégables ; un enfant ne les
   étend pas et ne reçoit pas automatiquement les secrets du parent.
3. Un contenu de cadrage, un template, une mémoire, un message ou une sortie
   d'outil peut proposer une création ; seul un plan adopté et une autorité
   vérifiée peuvent la permettre. Importer un plan ou une ancienne preuve ne
   crée pas une autorisation locale active.
4. Hors plan : bloquer le lancement, conserver une demande motivée et présenter
   l'écart, le demandeur, l'agent proposé, sa mission, ses droits, son coût/sa
   réservation prévus et le risque de l'exception. Une absence de réponse reste
   en attente ; un refus reste un refus. La demande ne réserve pas implicitement
   une exécution ni ne consomme un appel fournisseur.
5. L'approbateur est une personne authentifiée et habilitée, distincte du
   demandeur. Un service, un champ `human:true`, un texte « approuvé » ou la seule
   session partagée du cockpit ne constitue pas une décision humaine attestée.
   Le bootstrap de cette autorité doit être explicite, jamais déduit du premier
   appelant. Une permission absente ne se résout pas en désactivant le contrôle.
6. L'accord porte sur le contenu exact présenté. Modifier tâche, parent, rôle,
   fournisseur/modèle, outils, périmètre de données, droits ou réservations rend
   l'accord incompatible. Une validation humaine n'efface ni les refus de schéma,
   ni les limites du fournisseur, ni les interdictions non dérogeables.
7. Revalider identité, révocation, délégation, plan et accord au point qui engage
   la création, puis au départ effectif du processus. Après révocation prise en
   compte, aucun nouveau départ. Définir aussi le traitement des agents déjà en
   cours et de leurs descendants ; ne pas déduire leur arrêt d'un statut affiché.
8. Une autorisation ne permet qu'un effet prévu. Réserver et appliquer de façon
   atomique, réconcilier les départs incertains après panne ; aucun double agent,
   même sous concurrence ou après redémarrage. Garder les consommations cumulées.
9. Les identités et droits contrôlent aussi lecture, résumé et transmission du
   cadrage et de la mémoire. Un agent légitime n'accède pas aux autres périmètres
   par un historique automatique, une citation ou un brief adopté.
10. Le journal ajoute demande, refus/attente, décision attribuée, application et
    effet observé. Il conserve les originaux sans jetons ni secrets. Protéger le
    stockage, les clés et les mécanismes d'ancrage contre les processus agents :
    « append-only » dans l'API ne garantit pas l'inaltérabilité du fichier hôte.

## Matrice de scénarios à reproduire

Chaque ligne décrit une recette attendue. L'état observé après IA1 est conservé
dans le rapport : **4 FAIL, 4 PARTIAL et 10 NOT TESTED ; aucun scénario entièrement
PASS**. Le succès exige le contrôle commun et les parcours publics concernés ; un refus
doit conserver sa cause sans créer d'agent, de processus ou de réservation de
lancement. Une demande d'exception explicite peut ajouter sa propre trace.

| ID | Scénario | Résultat attendu |
| --- | --- | --- |
| IDA-01 | Attaquant anonyme, jeton inventé, signature invalide ou identité suspendue. | Refus avant toute création ; aucun rôle déduit des champs déclarés. |
| IDA-02 | Copie du nom, `event_id`, rôle, fournisseur, parent ou identifiant d'un agent légitime. | Aucune appropriation de l'identité ni des droits ; collision de contenu refusée. |
| IDA-03 | Agent authentifié demandant un enfant absent du plan ou sans droit de délégation. | Pas de lancement ; demande motivée à une personne habilitée pour une exception admissible. |
| IDA-04 | Création prévue au plan adopté, identité et délégation valides. | Un seul agent, au contenu exact autorisé ; preuve de réception de son périmètre et de ses droits. |
| IDA-05 | Enfant demandant davantage de droits, changeant de périmètre ou transmettant une autorisation non délégable. | Refus de l'extension ; aucune promotion par filiation. |
| IDA-06 | Document, mémoire, template, réponse fournisseur ou message se présentant comme ordre officiel. | Proposition non autorisante ; passage obligé par le contrôle d'identité et d'admission. |
| IDA-07 | Service ou agent tentant d'approuver sa propre exception, y compris avec rôle d'approbateur déclaré. | Refus ; seule une personne authentifiée et habilitée peut décider. |
| IDA-08 | Changement d'outils, mission, parent, modèle, accès aux données ou réservations après aperçu/accord. | Accord incompatible ; nouvelle présentation et nouvelle décision nécessaires. |
| IDA-09 | Rejeu d'un accord consommé, expiré, révoqué ou provenant d'un autre projet ; deux demandes concurrentes. | Aucun droit transporté entre projets ; effet unique ou conflit, jamais un enfant supplémentaire. |
| IDA-10 | Révocation/suspension, changement de plan ou de politique entre demande, accord, mise en attente et départ. | Revalidation au point d'engagement et avant processus ; refus frais sans remise à zéro de l'historique. |
| IDA-11 | Panne avant/après réservation ou départ ; réponse perdue ; reprise du serveur. | Réconciliation avec le résultat durable avant nouvelle action ; statut inconnu conservé tant que l'effet n'est pas établi. |
| IDA-12 | Même demande via CLI, API, assistant, reprise de dialogue, lancement préparé ou dispatch automatique. | Même décision d'autorité ; aucun contournement par une voie secondaire. |
| IDA-13 | Agent légitime lisant, résumant ou transmettant cadrage/mémoire d'un autre propriétaire ou usage interdit. | Refus au consommateur, y compris historique automatique ; aucun contenu interdit dans le prompt ou les logs. |
| IDA-14 | Agent lisant les secrets du contrôle, modifiant registre/politiques/logs, lançant directement un binaire ou réutilisant un PID. | Frontière d'exécution vérifiée et attribution résistante à la réutilisation de PID ; aucun agent ainsi promu légitime. Distinguer processus OS et instance admise par Swarm. |
| IDA-15 | Plan, archive, ancien agent ou preuve importés avec droits historiques. | Historique conservé ; aucune autorité locale réactivée sans admission actuelle. |
| IDA-16 | Refus humain, absence de réponse ou indisponibilité du service d'identité. | Attente/refus explicites, sans approbation par délai ni lancement de repli. |
| IDA-17 | Rafale de demandes d'exception ou arborescence récursive prévue mais excessive. | Limites de ressources appliquées et explicables, persistées et administrables ; aucun élargissement automatique des budgets. |
| IDA-18 | Journal interrompu, altéré ou incomplet ; tentative d'effacer refus et filiation. | Lacune/altération détectée selon l'ancre vérifiée ; aucune clôture fabriquée ni preuve régénérée à partir d'un récit. |

### Compléments de couverture après IA1

Le [contrôle de couverture](verification-identites-agents-20261016/COUVERTURE-RISQUES.md)
retrouve les demandes initiales et les risques historiques R01/R13. Il précise
six variantes à intégrer aux critères IA2/IA3 et à la qualification IA5, sans
modifier IDA-01–18 ni transformer les PASS historiques de Trust en validation
des agents :

| Référence | Variante à définir et vérifier | Rattachement |
| --- | --- | --- |
| COV-ID-01 | Vol/copie d'un jeton valide ou d'une autorisation, usage par une autre instance/processus et service privilégié utilisé comme intermédiaire. Définir l'ancrage attesté, pas seulement le secret présenté. | IDA-01/02/05/09/14 |
| COV-ID-02 | Réexercer au consommateur agent les mauvais issuer/audience/type de jeton, rôles hors plafond, rotation/révocation des clés, révocation distante et expiration pendant le verrou. | IDA-01/07/09/10/12/16 |
| COV-ID-03 | Homonymes entre émetteurs/projets, clone, reconnexion et remplacement d'une instance ; aucun transfert ou rétablissement implicite des droits. | IDA-02/09/15 |
| COV-ID-04 | La cible de retrait des droits, arrêt des instances actives/descendants et réconciliation est explicitée dans le contrat IA2. Son application, la course au départ et l'extinction réelle restent à prouver avant IA3. | IDA-10/11/16 |
| COV-ID-05 | Couvrir aussi les activations sans ligne `Agent`, les précontrôles et les métadonnées ; aucun exécutable fournisseur interdit lancé avant admission. | IDA-03/06/12/14/17 |
| COV-ID-06 | Récupération après compromission et restauration d'une ancienne sauvegarde/ancre authentique, sans réactiver droits ou accords révoqués ; préciser autorité et modèle de menace. | IDA-09/10/15/18 |

Ces variantes ne sont pas de nouveaux défauts runtime démontrés. Leur état reste
NON TESTÉ pour le nouveau contrat. Codex en est propriétaire dans IA2/IA3/IA5 ;
les critères détaillés figurent dans le contrôle de couverture. L'inventaire
ne constitue pas une garantie d'absence d'autres risques.

## Choix et séquence de réalisation

Deux approches comparées : ajouter seulement des champs auteur/parent dans les
requêtes, ou raccorder l'admission des agents à l'identité authentifiée et à une
frontière de contrôle protégée. La première garde le problème d'usurpation et
les voies de contournement. La seconde est retenue comme cible ; le mécanisme
de confinement fournisseur sera choisi et vérifié avant toute garantie hôte.

| Lot | Dépendance / propriétaire | Livrable et preuve de sortie |
| --- | --- | --- |
| IA1 Audit et contre-exemples | M19 conservé ; Codex | Audit initial livré : inventaire des créations, reprises, invocations de rôles et accès au contrôle ; sept contre-exemples CLI/API/processus, racines isolées, aucun modèle réel. Les voies inventoriées seulement et scénarios non testés restent explicités dans le rapport IA1. |
| IA2 Contrat d'identité et de délégation | IA1 ; Codex | Liens principal/instance/plan, droits exacts, révocation, import et bootstrap ; tests invalides et refus sans mutation. Réutiliser `identity-context-v1` sans assimiler service et humain. |
| IA3 Admission et décision humaine | IA2 ; Codex | Opérations publiques communes CLI/API, demande expliquée, décision humaine exacte, consommation atomique, rejeu et reprise ; protection du plan de contrôle testée. |
| IA4 Droits du cadrage et interface | IA3 et M20 ; Codex | Propriétaire, auteur, consultation/résumé/transmission contrôlés ; attentes et refus visibles. Configuration éventuelle commune Go/API/CLI/Admin, FR/EN, thèmes et clavier. |
| IA5 Qualification et RETEX | IA3/IA4 ; Codex | Matrice IDA-01–18, recettes réelles avec fournisseurs scriptés, preuves de persistance/concurrence/effet, docs FR/EN ; distinguer contrôle local, revue indépendante et déploiement. |

Reprise après « c'est lancé ? » puis « fais » : IA2 est engagé en développement
direct. Le [contrat sémantique IA2](identites-agents-contrat-IA2.md) distingue
principal, agent logique, instance, canal, autorisation et effet. Il fixe la cible
de révocation des instances actives et descendants, sans confondre retrait des
droits et arrêt vérifié. L'administration de la source d'identité doit aussi être
protégée ; les rôles Trust et le step-up actuels ne valent pas automatiquement
droits de création. IA2.2/IA2.3 restent requis pour le contrat concret et les
preuves ; aucun contrôle applicatif nouveau n'est déclaré opérationnel.

Premier résultat IA2.3 : une expérience hôte isolée compare le même programme
scripté avec/sans `bwrap`. Dix variantes d'accès au contrôle sont refusées,
l'écriture autorisée dans le workspace persiste et registre/clé/SQLite restent
inchangés. Aucun modèle réel ; réseau fournisseur, ancrage d'identité, approbation,
révocation et intégration dans les lanceurs restent ouverts. Le contrat IA2
référence les originaux de cette expérience ; ce PASS local ne clôt pas F07.

Les opérations publiques, leurs schémas et paramètres restent à définir depuis les
producteurs réels ; ce document n'invente aucun endpoint ni commande. Les
invariants obligatoires ne doivent pas posséder de mode permettant leur
contournement. Toute politique opérateur nouvelle doit être persistée et
éditable dans Admin avec la même validation en Go/API/CLI.

Condition de fermeture : demande reçue → décision attribuée → application
autorisée → effet constaté pour les cas permis, refus sans effet pour les autres,
et protection des accès directs démontrée dans le périmètre annoncé. Une suite
OIDC verte, une règle écrite ou un test de primitive OS ne ferme pas ce chantier.
Le propriétaire technique reste Codex ; une configuration d'identité réelle ou
un déploiement manquant sera remis explicitement à l'administrateur compétent.

## Reprise IA1 — conversation dédiée, 16 octobre 2026

Périmètre retenu : audit et contre-exemples avant correction, Codex seul,
`replan` puis `audit-pdca` en mode audit. Aucun lancement de modèle réel,
aucune modification applicative ni intervention sur une mission existante.
Référence de départ : `4e58f7cbf8200a58f69f1f4d03d657c3313fb642` avec changements
locaux conservés ; les empreintes des entrées réellement compilées feront foi.

Coordination des fichiers : ce chat possède la présente section et
`verification-identites-agents-20261016/`. Le chantier mémoire conserve
`docs/plans/deepagents-integration/`, les sources et tests mémoire. Les sources
communes de lancement, de stockage, de contexte et de Trust sont lues seulement
pendant IA1 ; annoncer et coordonner leur modification avant IA2/IA3. Le chat
mémoire « Comparer Deep Agents à Swarm » a été consulté en lecture seule pour
rétablir ce passage de relais ; aucun message ne lui a été envoyé.

Critères IA1 : inventorier les producteurs et chemins secondaires de lancement,
distinguer la session cockpit d'une identité individuelle, reproduire les
premières frontières absentes par CLI/API et processus scripté, conserver les
contrôles existants et les limites non testées. La présence d'un contre-exemple
établit un défaut ; elle ne rend pas les autres scénarios conformes. IA2–IA5 et
les droits effectifs de mémoire restent ouverts.

Résultat de la reprise : sept contre-exemples aboutis, huit invocations de
travail de fournisseurs scriptés, aucun modèle réel ; quatre tests Go et leurs
neuf sous-tests PASS pour les contrôles existants. Les 1 038 entrées de source,
tests et ressources inventoriées sont inchangées. Les défauts applicatifs
restent ouverts pour IA2/IA3 ; ne pas convertir les tests verts de Trust en
validation de la création des agents. Le rapport IA1 conserve aussi les deux
échecs de prérequis de la recette et leur diagnostic, sans les compter comme
tests de protection réussis.

## Suite IA2.3 — canal d'instance, 17 octobre

L'[expérience du canal](verification-identites-agents-20261016/ia2-runtime-binding-20261017/RAPPORT-IA2.3-CANAL.md)
passe 13 contrôles sur deux cages scriptées. Une connexion authentique copiée
de B à A ne transfère pas l'identité B lorsque chaque message est attribué par
le kernel. PID/montage sont épinglés avant exécution ; le namespace utilisateur
n'est pas supposé stable. Source d'autorité, persistance et raccord Go restent
ouverts, ainsi que les constats IA1 et la qualification globale.

## Documentation consolidée — 17 octobre

Le [contrat opérationnel FR/EN](../../architecture/AGENT-IDENTITY-CONTRACT.md)
décrit le composant réellement présent, les entrées, refus, sources d'autorité
manquantes et responsabilités. Les rapports IA2.2/IA2.3 conservent sources,
empreintes, échecs avant correction, commandes, résultats et schémas rendus.
Le [dossier scientifique actualisé](../../publications/20261005-cursor-moteur/article-scientifique/identites-autorite-cycle-vie.md#8-état-réel-de-swarm-et-preuve-disponible)
sépare précédents, hypothèses, contributions candidates et preuves locales.
L'[index documentaire final](verification-identites-agents-20261016/ia2-runtime-binding-20261017/checks-documentation-final.json)
lie cette version aux contrôles et vérifie leurs entrées ; les anciens index
restent des checkpoints historiques.

Les améliorations SCI-ID-06–10 consolident IA2-K01/K02 et les découvertes de
canal/validation, avec origine, valeur, propriétaire, critère et état. Pas de
nouvelle preuve d'autonomie, de réception par un autre chat ou de transmission
à tous les futurs contextes. Les consignes permanentes existantes ne sont pas
réinstallées ; le chantier mémoire conserve ses fichiers.

Prochaine unité de réalisation, Codex : définir puis éprouver le courtier Go et
son producteur d'autorité protégé, avec registre courant et engagement Trust /
SQLite. Sortie requise : attribution réelle, contrôle frais, refus sans mutation,
révocation et récupération vérifiés, avant raccord de tous les lanceurs IA3.
Cette unité conserve le cas de connexion transférée et l'absence d'auto-ancrage
par le premier message. Aucune nouvelle exécution native ni surveillance
récurrente n'est autorisée par cette documentation. IA5 et le RETEX restent ouverts.

## IA2.3a — portage du canal en Go, reprise du 17 octobre

Besoin : remplacer l'identité d'instance simulée de IA2.2 par une attribution
d'exécution vérifiée, sans accepter l'identité déclarée ou celle de la connexion.
Deux options : rouvrir `/proc/<pid>` depuis les credentials du message, ou
utiliser le pidfd remis par le noyau avec ce message et obtenir ses namespaces
par ioctl. La première laisse une course de résolution du numéro de PID ; la
seconde est retenue, avec refus si le noyau ne fournit pas les primitives requises.
Cette course est une limite identifiée par inspection, pas un exploit reproduit.
La sonde hôte confirme credentials, pidfd et ouverture PID/montage par ioctl.

Propriétaire Codex, fichiers nouveaux `agent_identity_channel_linux.go`,
`agent_identity_runtime_linux.go` et leurs tests ; contrat FR/EN et preuves dans
`verification-identites-agents-20261016/ia2-go-channel-20261017/`. Aucun fichier
mémoire, lanceur, stockage ou Trust préexistant modifié dans cette tranche.
Ordre local prévu : réception hors verrou Trust, puis verrou Trust partagé,
puis registre de liaisons en mémoire, puis évaluation IA2.2 sans effet métier.

Critères : cages A/B épinglées par le contrôle avant ouverture de la porte ;
message de A sur connexion B attribué à A ; refus de l'octroi B et acceptation
de l'octroi A ; sous-processus légitime reconnu ; émetteur disparu, message
tronqué, droits de fichier injectés et registre fermé/révoqué refusés ; fermeture
des descripteurs reçus et absence de course de données. Rejouer les contrôles
IA2.2 pertinents, puis analyse statique et compilation. Conserver commandes,
sorties, sources et entrées exactes, sans annoncer de revue indépendante.

Le registre de liaisons est privé au processus contrôleur ; la source durable
d'autorité, bootstrap humain, protocole Trust/SQLite et raccord aux lanceurs
restent des critères IA2.3/IA3 séparés. Un consommateur interne utilisant encore
un cadre d'autorité de fixture ne prouvera pas leur réalisation. F01–F07 restent
ouverts jusqu'à vérification des parcours concernés.

### Résultat IA2.3a

Le [canal Go et son rapport](verification-identites-agents-20261016/ia2-go-channel-20261017/RAPPORT-IA2.3a.md)
relient une attribution réelle de message au noyau privé IA2.2 dans deux cages
isolées. 133 noms de tests/sous-tests PASS, dont 19 nouveaux et 114 déjà présents ;
les mêmes 19 nouveaux passent sous `-race`. Analyse statique et compilation
réussies. La connexion reste attribuée à B côté pair, mais le message de A est
évalué comme A : son octroi passe, celui de B est refusé. La table de liaison est
tenue par le contrôle, non remplissable par le premier message.

Une nouvelle course IA2-K03 est reproduite avant correction : expiration durant
l'attente du verrou runtime. L'horloge est désormais lue après cette attente et
la résolution des namespaces ; contrôle positif et refus après expiration passent.
Le défaut initial de socket non bloquant partagé dans la fixture et sa correction
sont aussi conservés. Les résultats sont liés à 1 046 entrées inchangées ; aucune
source mémoire/lanceur/Trust préexistante n'a été modifiée par cette tranche.

Ce résultat ferme seulement les critères locaux du portage IA2.3a. IA2.3b doit
fournir l'autorité durable protégée et l'engagement Trust/SQLite, puis IA3 raccorder
les opérations publiques et l'accord humain. Instantané/octrois/cadre sont encore
des fixtures dans ces essais. F01–F07, IDA et le RETEX restent ouverts. Ne pas
assimiler les 133 noms à 133 attaques ou ajouter les 19 répétitions `-race`.

## IA2.3b — source durable et publication récupérable

Reprise autorisée par « ok suite », Codex seul. Fichiers réservés : nouveaux
`agent_identity_registry_*.go` et tests, migration dans `store.go`, version et
écriture durable dans `model.go`, champ privé/validation Trust dans
`trust_actions.go`, contrat FR/EN, plan et preuves
`verification-identites-agents-20261016/ia2-durable-authority-20261017/`.
Lanceurs et sources mémoire inchangés dans cette tranche.

Choix comparés : tête d'autorité recalculée depuis SQLite importé, ou ancre
Trust distincte de l'instantané et publication avec intention durable.
La seconde est retenue : Trust exclusif → transaction SQLite réservée →
intention Trust synchronisée → journal/instantané SQLite → ancre Trust finalisée.
Une intention non finalisée bloque les lectures d'identité ; la récupération
réapplique uniquement sa publication exacte, sans départ de processus.
Le registre produit un instantané possédé, pas une map empruntée.

La fraîcheur de reprise exige un checkpoint de contrôle privé indépendant du
contenu à vérifier. Ni importer la base ni relire ensemble deux sauvegardes
anciennes ne fournit cette autorité. Bootstrap et permission de publication
sont des entrées privées exactes du contrôle, non des rôles Admin implicites ou
une commande publique. Leur attribution humaine réelle reste dans IA3 ; une
fixture de checkpoint ne sera pas présentée comme une ancre externe déployée.

Préalable découvert : `atomicWrite` synchronise le fichier mais pas le répertoire
après rename. Corriger ce contrat commun avec précondition de synchronisation
vérifiable et erreurs explicites ; ne pas prétendre simuler une panne électrique.
Migration version 36 : tables de snapshots et publications, sauvegarde, aucune
attribution ajoutée aux données historiques.

Critères locaux : publication/lecture après réouverture ; deux interruptions
(intention écrite, SQLite engagé) puis reprise exacte sans doublon ; restauration
ou falsification/import refusés face au checkpoint courant ; configuration Trust
modifiée sans réactivation implicite ; écrivains concurrents et erreurs sans
mutation non autorisée ; identité courante évaluée depuis la source durable.
Tests ciblés, `-race`, statique et compilation liés au candidat exact. Les parcours
publics, effets, confinement complet, qualification IDA et RETEX restent ouverts.

### Résultat IA2.3b — module de source durable

Le [rapport IA2.3b](verification-identites-agents-20261016/ia2-durable-authority-20261017/RAPPORT-IA2.3b.md)
conserve **181 noms de tests/sous-tests PASS**, dont 33 du registre ; 52 du
registre/canal/runtime passent aussi sous `-race`. Statique et compilation
réussies, 1 052 entrées inchangées. Schéma 36, tête Trust indépendante et
checkpoint de contrôle, publication par intention → SQLite → finalisation,
reprise exacte et lecture en copie possédée sont implémentés en interne.
Une cage réelle passe puis est refusée après révocation durable de l'octroi.

Découvertes traitées : D01 synchronisation de répertoire ; D02 décodage d'un
maillon dans un objet neuf ; D03 travail absent vérifié avant intention ; D04
garde de version dans la transaction de lecture. Les échecs avant correction
et sources originales sont conservés. Les contextes mémoire ont été testés pour
le changement partagé de stockage, sans modification de leurs sources.

Ce résultat ne ferme pas tout IA2.3 : permis/checkpoint sont émis par le contrôle
simulé, pas par un émetteur humain ou un témoin externe déployé. Une restauration
conjointe est refusée avec le checkpoint courant indépendant ; restaurer aussi
un ancien témoin ne prouve aucune fraîcheur. Codex doit raccorder les producteurs
du vrai plan, permis et témoin ; l'opérateur assure leur enrôlement/déploiement
lorsque le parcours sera concret et autorisé. Le contrat refuse l'absence de
checkpoint, sans auto-adoption d'un import.

La prochaine frontière est l'admission commune IA3 : rattacher les appelants et
l'accord humain exact, réserver/consommer dans le vrai lancement et récupérer
un départ incertain. Le raccord doit respecter Trust → SQLite et ne pas appeler
le publiant depuis une transaction déjà tenue. Les primitives recevant un cadre
fourni ne sont pas la voie publique fraîche. F01–F07, les scénarios IDA, les droits
mémoire et le RETEX restent ouverts ; aucun binaire actif n'a été remplacé.

## IA3a — liaison au plan réellement enregistré, 17 octobre

Reprise « next », Codex seul, `replan` et APEX. La source IA2.3b vérifie
le digest fourni par son permis, sans le comparer au contrat du travail courant.
Cette précondition de fixture doit devenir une vérification du consommateur.
Comparer la révision complète du travail (changée par les événements runtime)
à une projection conservant le contrat et retirant seulement les champs de
résultat/exécution identifiés. Retenir la seconde, avec champs inconnus conservés
par défaut et contrat de projection versionné. Aucun contenu ne crée de droit :
la présence d'un plan enregistré ne prouve pas une adoption humaine attestée.

Fichiers : `agent_identity_plan_linux.go` et tests nouveaux ; publication, lecture
et fixtures du registre ; contrat FR/EN, présent plan et preuves
`verification-identites-agents-20261016/ia3-plan-binding-20261017/`. Aucun lanceur,
fichier mémoire, paramètre opérateur ou stockage vivant modifié.

Critères : reproduire avant correction un scope arbitraire, un contrat changé
et l'absence de plan acceptés ; produire le scope depuis le stockage courant ;
vérifier au même instantané SQLite la publication et la lecture ; refuser sans
mutation avant intention ; une reprise peut finir son histoire exacte après
changement du plan mais ne doit pas en réactiver l'usage ; progression ordinale
du plan depuis la tête authentifiée, sans auto-ancrage d'un import. Les résultats
d'exécution identifiés ne modifient pas le contrat, les définitions/parents/rôles/
réservations et nouveaux champs le modifient. Tests ciblés, concurrence et
revalidation sur cage réelle ; permis humain, témoin externe et lanceurs publics
restent ouverts.

### Replan IA3a — génération perdue et canonisation

Les 36 premiers contrôles passent, mais la relecture identifie une lacune du
choix initial : compter seulement les changements observés à la publication ne
voit pas une modification puis restitution du contenu avant publication. Un
nouveau contre-exemple est conservé avant correction. Une deuxième garde exige
que l'ordre des clés JSON ne change pas l'empreinte annoncée canonique.

Options : ajouter un appel à chaque producteur de Work, avec risque d'oublier
une écriture, ou maintenir le stamp par triggers SQLite dans chaque transaction
d'insertion/modification de Work. Retenir la seconde : projection déterministe
versionnée enregistrée dans le pilote SQLite, schéma 37, génération monotone
et contrôle de concordance entre stamp et corps au consommateur. Les migrations
et producteurs de plan existants doivent être rejoués ; tout échec garde ses
originaux. Ajouter `agent_identity_plan_schema_linux.go`, `model.go` et `store.go`
aux fichiers possédés. Pas de modification des lanceurs ou des sources mémoire.

La génération traite les transitions normales persistées ; elle ne fabrique pas
un témoin externe. Une restauration physique de tout SQLite vers un plan ancien,
sans nouvelle publication d'identité intermédiaire, exige que le futur témoin
indépendant couvre aussi le watermark du plan, pas seulement la tête du registre
d'identités. Cette condition d'IA3 reste ouverte, propriétaire Codex/opérateur.

### Contrôle élargi et anomalie antérieure de reprise de revue

Le motif `TestPlan.*` inclut accidentellement `TestPlanning*`. L'élargissement a
révélé une panique dans `TestPlanningReviewRetryDeliversSupervisorCorrectionAfterReopen`,
reproduite aussi avec les sources initiales par overlay Go, sans modifier le
checkout. La fixture historique v1 déclenche désormais un remplacement complet
de méthode ; le test déréférence à tort sa revue active supprimée. Un essai avec
fixture courante confirme que la protection interdit alors la nouvelle dépense
sur simple reformulation après erreur de protocole inchangée. Cet essai est
rejeté et ses trois fichiers restaurés à l'octet : aucune garde de reprise
assouplie. Preuves et diagnostic dans `ia3-plan-binding-20261017/`.

La sélection finale vérifie `TestPlan[A-Z].*` et les producteurs hiérarchiques
explicitement nommés, conformément au contrat modifié. Le défaut préexistant ne
devient pas PASS et empêche toute revendication de suite globale verte.
Propriétaire Codex, chantier moteur/reprise de revue ; condition : réaligner
ces anciennes attentes sur la reprise protégée avec changement de contrat, puis
vérifier réception, décision, application et effet. Les travaux identité
indépendants continuent. La recette d'attente identité a aussi été corrigée :
observer `sql.DB.Stats().WaitCount` évite qu'un sondage répété du verrou partagé
provoque lui-même le refus du publiant non bloquant. Échecs originaux conservés.

### Replan IA3a — fraîcheur pendant le verrou runtime

Les 231 contrôles ciblés passent sur le premier candidat, mais la relecture
du consommateur révèle que libérer SQLite avant le verrou runtime laisse les
écritures de Work progresser pendant cette attente. Une ancienne portée peut
donc produire une décision après un changement de plan déjà engagé. Le temps
frais ne rafraîchit pas les données du plan. Nouveau contre-exemple sur cage
réelle, avec sources et entrées conservées avant correction.

Options : verrou de plan indépendant à imposer à tous les écrivains, ou garder
une réservation d'écrivain SQLite pendant la décision pure. Retenir la seconde,
plus bornée et commune à tous les écrivains existants. Ordre du consommateur :
Trust partagé → réservation SQLite → instantané/plan courant → verrou runtime
→ temps frais et évaluation pure → COMMIT sans effet métier. Le callback est
fermé et ne réentre pas dans Store ; les méthodes runtime appelées n'accèdent
pas à SQLite. Les inspections/propositions seules restent en transaction de
lecture libérée après copie. Pas de garantie globale pour des callbacks futurs.

Ce choix sérialise la décision avec les modifications du travail ; son coût en
concurrence n'est pas mesuré. Il n'ajoute aucune politique de délai/reprise et
utilise le contexte du consommateur. Les futurs lanceurs devront respecter cet
ordre et leur propre engagement/réconciliation ; une décision positive ne
démarre toujours aucun effet.

### Résultat IA3a — contrat courant, admission encore partielle

Le [rapport IA3a](verification-identites-agents-20261016/ia3-plan-binding-20261017/RAPPORT-IA3a.md)
conserve 232 noms ciblés PASS, dont 32 du plan ; les 84 noms
plan/registre/canal/runtime passent aussi sous `-race`. Statique et compilation
réussies ; 1 056 entrées compilées/testées/embarquées restent inchangées pendant
ces contrôles. Les 181 contrôles précédents sont rejoués ; neuf noms mémoire
paramétrés par version sont corrélés par leur variante relative, à source
inchangée. Les répétitions ne sont pas des attaques supplémentaires. Deux nouveaux schémas FR/EN sont rendus et
inspectés, les six précédents sont réutilisés à sources identiques.

Schéma 37 et triggers communs aux écrivains Work, projection avec champs
inconnus conservés, génération après retour à un ancien contenu, vérification
du plan après attente et reprise historique sans réactivation sont réalisés.
La course lecture SQL → attente runtime est reproduite avant correction ;
la décision pure garde désormais l'écrivain SQL jusqu'à son COMMIT. L'ordre
Trust → SQLite → runtime est vérifié sur ce consommateur fermé, pas sur des
callbacks futurs arbitraires. Les inspections restent sans réservation d'effet.

Les sources mémoire et lanceurs restent inchangés. Les trois fichiers de tests
de reprise de revue modifiés par l'essai rejeté sont restaurés exactement ;
le défaut préexistant garde son diagnostic, son propriétaire et sa reprise.
Aucune suite globale verte, revue indépendante ou activation n'est revendiquée.

La prochaine frontière conserve le vrai émetteur humain du permis, le témoin
indépendant couvrant aussi les amendements de plan, puis appartenance et contenu
exact de la création, consommation/départ/réconciliation dans tous les lanceurs.
Le permis et le checkpoint restent des fixtures ; aucun plan enregistré ne se
transforme seul en droit. IA3 reste partiel ; F01–F07, IDA, IA4, IA5 et RETEX
global restent ouverts.

## IA3b — émetteur humain et autorité explicite, reprise du 17 octobre

Besoin conservé : aucune identité authentifiée, aucun rôle Admin et aucun plan
enregistré ne suffisent à publier des droits. IA3a est vérifié localement ; les
critères globaux, F01–F07 et la reprise de revue préexistante restent ouverts.
Codex reste seul propriétaire du changement, sans lancement natif ou fournisseur.

La demande du 17 octobre impose la documentation dans chaque incrément et sa
transmission intersessions. Règle ajoutée dans les accords personnels partagés
Codex/Claude, le présent dépôt, le contrat embarqué et les pages méthodes FR/EN.
La session reçoit déjà la nouvelle consigne ; l'assemblage embarqué est en cours
de contrôle. Les processus existants ne sont pas mis à jour par cette écriture.

Comparaison : raccorder tout de suite un nouveau réglage d'enrôlement au cockpit
actuel, ou isoler d'abord le consommateur strict d'une autorité d'enrôlement fournie
par le contrôleur. La première option dépend de la protection de l'administration
et du témoin externe encore absents ; elle rendrait circulaire l'authenticité.
Retenir la seconde : produire en interne un permis exact depuis une session
OIDC vérifiée et une autorité explicite privée, avec périmètre, droits, dates et
révision Trust. Le contrôleur doit fournir cette autorité indépendamment de la
requête et du registre ; aucun bootstrap réel ni service de révocation externe
ne sera annoncé comme livré par ce noyau. Les lanceurs publics restent ouverts.

Fichiers annoncés : nouveaux `agent_identity_issuer_linux.go` et test associé
dans `internal/engine/`, contrat FR/EN, contrat IA2 et présent plan ; notes de
recherche concernées et preuves dans `ia3-issuer-20261017/`. Les sources mémoire,
les lanceurs et les anciens tests de revue ne sont pas concernés.

Critères bornés : session humaine OIDC réellement produite par le parcours de
connexion de fixture ; refus des identités locales/services et des rôles seuls ;
liaison exacte issuer/sujet/principal/projet/plan ; droits et dates limités par
l'autorité privée ; refus sans mutation ; fraîcheur après attente SQL ; permis
lié au contenu complet du registre et utilisable par son protocole de publication.
La création exacte hors plan, l'approbateur distinct, le témoin externe du plan,
la persistance/enrôlement des autorités et la consommation/départ restent leurs
critères ouverts, sans être assimilés à cet incrément.

Amélioration de méthode conservée : une lecture a encore supposé le chemin absent
`agent_identity_trust.go`. Cause : nom déduit au lieu de consulter l'inventaire ;
aucun fichier ni contrôle modifié. Reprise effectuée par inventaire `rg --files`
puis lecture du producteur réel `agent_identity_contract.go`. Propriétaire Codex ;
critère : utiliser les chemins effectivement découverts pour la suite.

### Replan IA3b — règle documentaire et capacité du contexte

Le contrôle du pack refuse les contextes planner/subplanner (32 185/32 188 octets
pour une capacité technique de 32 000). L'ajout documentaire est la cause ; la
preuve originale est gardée dans `ia3-issuer-20261017/method-pack.stderr`.
Choix : condenser le contrat commun en gardant règle, droits et preuves ; ni
troncature ni augmentation de capacité. La règle personnelle détaillée persiste.
Vérification attendue : assemblage réel des quatre rôles et réception de la
règle dans le contexte construit, sans appel fournisseur.

### Replan IA3b — validité du permis humain au publiant

Les tests initiaux reproduisent deux accords erronés (2 FAIL / 4 tests) : une
autre session du même humain réutilise le permis ; l'autorité expire pendant
l'attente SQLite mais la publication réussit. Sources figées et sorties originales
conservées dans `ia3-issuer-20261017/issuer-before*`. Le producteur vérifiait
l'identité, mais le publiant ne vérifiait pas cette restriction supplémentaire.
Correction : vérifier sa preuve de session et ses dates après la réservation
SQL puis de nouveau avant l'intention durable, sous le verrou Trust existant.
La reprise reste limitée à finir l'intention historique exacte ; elle ne crée
pas une autorisation nouvelle. Le chemin bas niveau antérieur reste une capacité
privée de contrôleur utilisée par les fixtures, sans preuve humaine et sans
adaptateur public ; ne pas présenter tous les permis du paquet comme issus
d'un enrôlement humain réel. Le raccord public devra exiger le nouveau producteur.

### Améliorations liées à l'émetteur : conservation et frontière avant lecture

- IA3b-I03, Codex : la relecture montre que l'émetteur lit le plan avant de
  résoudre l'appelant. Valeur : ne pas distinguer travail absent et présent
  avant autorisation, ni prendre SQL pour un appel rejeté. Comparaison : masquer
  seulement les erreurs ou autoriser avant lecture puis réauthentifier après
  attente. Retenir la seconde ; contre-exemples et correction à vérifier.
- IA3b-I04, Codex : l'empreinte d'enrôlement et la preuve de session du permis
  sont privées/éphémères ; la tête durable conserve le principal mais pas un reçu
  d'émission humain complet. Valeur : retrouver pourquoi ce principal a pu publier
  des droits après redémarrage. Suite : définir et persister le reçu avec la source
  d'enrôlement/témoin, avant raccord public. Critère : lien vérifiable entre reçu
  d'enrôlement courant, décision d'émission, tête et instantané, sans jeton brut.
  État : découverte par inspection, effet et service non réalisés. Pas de journal
  local présenté comme attestation externe. Ce point rejoint IA3/IA5, sans nouveau
  droit, service déployé ou copie autoritaire implicite.

Le diagnostic documentaire a aussi révélé une faute de séquence de l'assistant :
un contrôle dépendant a été lancé après un script d'édition échoué avant écriture.
Les condensations incomplètes (32 059 puis 32 019 octets) et la relance inchangée
sont conservées. La séquence corrigée lance le contrôle depuis le processus
d'édition seulement après application ; le pack final passe pour quatre rôles.
L'instruction existante de séquencement suffit : pas d'ajout récursif de doublons
au contexte déjà borné. Une recherche /tmp trop large a rencontré les répertoires
système privés ; abandon du balayage, réutilisation de la commande de rendu
tracée dans IA3a et des configurations de navigateur déjà vérifiées.

### Replan IA3b-I03 — autoriser avant la recherche du travail

Les trois variantes (identité absente, humain sans autorité, Admin local) retournent
`agent_identity_work_missing` au lieu du refus d'autorisation attendu ; quatre
noms FAIL avec le parent. Preuve : `lookup-before*`, sources originales conservées.
Correction : contrôle du principal et de l'autorité sous Trust avant SQL, puis
nouvelle résolution et contrôle après réservation, avant lecture du plan. La
portée réellement lue et les dates sont encore vérifiées avant émission. Aucun
masquage uniforme d'erreurs ni droit ajouté. Vérification : ces trois refus
exacts sans mutation, fraîcheur après attente, contrôles du registre inchangés.

### Replan IA3b — vérifications sous charge et fixtures cumulées

Le rejeu final conserve 278 noms PASS et un FAIL :
`TestEngineContractRevisionKillsInferenceOnContractMutation` mesure 2,273 s pour
une garde de 1,8 s. Cette garde inclut la préparation/démarrage du fournisseur ;
la source de ce parcours n'est pas modifiée par IA3b. La compilation instrumentée
concurrente et une charge CPU d'environ 99 % sont observées ; leur causalité exacte
n'est pas encore prouvée. Ne pas augmenter le seuil ni prétendre la latence sous
charge réparée. Reprise : même test seul, après fin des compilations possédées.

Le groupe `-race` de 90 s expire pendant les migrations d'une fixture, après
19 noms PASS et sans verdict FAIL individuel ; ce résultat est INCOMPLET, pas PASS.
La pile montre `registryFixture` → `agentIdentityAuthorityFixture` → nouvelle base
et création d'index SQLite : des stores complets sont recréés pour les variantes
invalides. Options : augmenter le délai, partager/modifier les fixtures, ou
partitionner les cas à source identique en réutilisant le code instrumenté compilé.
Retenir le partitionnement séquentiel, sans limite augmentée et sans réécriture
des cas : tous les noms attendus devront être couverts, parents répétés dédupliqués.

IA3b-I05, Codex, méthode de preuve : réutiliser un nom `final` laisse temporairement
l'ancien manifeste visible pendant une nouvelle exécution. Valeur : éviter un
faux résultat courant. Les anciens essais sont conservés ; la vérification finale
attend les processus terminés et contrôle dates, entrées, noms et états. Les groupes
de reprise ont des noms uniques. Critère : aucun manifeste ancien pris pour résultat
du candidat courant. Pas d'échec passé effacé ou de zéro test présenté comme PASS.

IA3b-I06, Codex, performance des fixtures : éviter de créer une deuxième base
uniquement pour construire un modèle pur de grants dans `registryFixture`.
Valeur : moins de migrations instrumentées et une provenance de caller liée au
vrai Store de fixture. Suite conservée : extraire le modèle pur, vérifier parité
des données et rejouer les variantes concernées. État : amélioration qualifiée par
la pile, non réalisée ; report car le partitionnement permet la vérification sans
modifier des fixtures partagées pendant ce gel. Reprendre à l'entretien des fixtures
identité, avec preuve de parité et de coût ; aucune optimisation produit revendiquée.

### Replan IA3b — entrées embarquées concurrentes et coût des refus

Le test de timing isolé passe avec le même seuil et sans modification de sa source.
Cela ne clôt pas la mesure sous charge. Son manifeste révèle 1 060 entrées au lieu
de 1 058 : `web/signals.css/js` ajoutés et trois fichiers `web/project-memory.*`
modifiés dans le dépôt partagé. `resources.go` embarque `web/*` dans le paquet
également importé pour les méthodes : ces modifications sans rapport avec IA3b
invalident des entrées de compilation. Elles sont préservées, sans édition de
notre part ni qualification de l'interface mémoire.

IA3b-I07, Codex/méthode : figer le jeu web embarqué au compilateur avec un overlay
Go temporaire, et développer le seul glob `web/*` en sa liste explicite identique
au moment de la capture. 136 ressources copiées, aucun fichier du checkout
remplacé. Les vrais sources Go restent dans le dépôt et leurs empreintes sont
vérifiées. La preuve concerne cette vue embarquée figée, pas le dernier état web
de l'autre chantier. Suite structurelle conservée : évaluer la séparation des
paquets de ressources moteur/méthodes et interface pour limiter ce couplage.
Critère : un changement web ne recompile pas les tests d'identité quand aucune
ressource qu'ils consomment n'a changé ; architecture et livraison à préserver.
État : isolation de vérification réalisée, extraction structurelle non réalisée.

La solution de partitionnement préparée est remplacée avant exécution : les
tables de refus sont toutes sans mutation et possèdent déjà une assertion de
non-mutation. Elles partagent maintenant une seule fixture par table, en copiant
profondément chaque snapshot et les listes d'autorité. Chaque variante garde
son refus exact et son assertion de stockage inchangé. Les scénarios de mutation,
reprise et attente gardent des fixtures distinctes. Cela traite la duplication
mesurée sans augmenter le délai de 90 s ni modifier les assertions. La suppression
de la seconde base du helper partagé reste une suite I06 distincte non réalisée.

La vérification compile un binaire instrumenté une seule fois, le conserve et
l'exécute via `go tool test2json`, avec hash du binaire avant/après. Même démarche
pour le binaire normal ; les 47 noms initiaux doivent être retrouvés exactement.
Les anciens résultats et le timeout sont conservés. Une compilation ou un groupe
interrompu ne devient jamais PASS. Aucun fournisseur réel ni mission native lancé.

### Résultat IA3b — noyau humain vérifié, raccord public ouvert

Les 47 noms attendus passent dans le binaire normal et sous `-race`, sans FAIL
ni SKIP, après partage contrôlé des fixtures immuables. Statique et compilation
sur vue embarquée figée passent ; les sources et binaires restent liés aux
empreintes. Les deux schémas FR/EN sont rendus et inspectés. Le rapport IA3b et
son index gardent les deux accords erronés, les trois refus prématurés, la série
278 PASS / 1 FAIL de timing, sa reprise isolée réussie au même seuil et le groupe
instrumenté expiré à 19 noms. Aucune série globale entièrement verte n'est inventée.

La règle documentaire est persistée dans les accords personnels et du projet,
le contrat commun et les pages méthodes FR/EN ; le pack a été assemblé pour ses
quatre rôles. Cela ne met à jour aucun processus déjà lancé et ne prouve pas
l'obéissance future. Aucun commit, déploiement, mission native ou appel fournisseur
réel. Les fichiers mémoire/web de l'autre chantier sont préservés.

Suite : enrôlement protégé et reçu durable avec source/témoin courants, avant
admission publique. Codex porte le code et les vérifications ; l'opérateur doit
désigner le principal humain initial et la frontière indépendante du contrôle.
Les autorisations de création exacte, départ/consommation/réconciliation, mémoire,
F01–F07, IDA, IA4, IA5 et RETEX global restent ouverts. La mesure de latence sous
charge et les améliorations de fixtures/paquets gardent leur propriétaire et leur
condition de reprise ; un résultat local ne les efface pas.


## IA3c.1 — reçu durable d’émission, reprise du 17 octobre

Besoin repris : IA3b-I04, relier l’autorisation humaine au registre après
redémarrage, sans transformer une référence d’enrôlement en attestation. Codex
seul possède cet incrément ; aucun fichier mémoire/web ni environnement vivant
n’est modifié. L’enrôlement protégé et sa révocation indépendante restent ouverts.

Deux options : une nouvelle table/transaction de reçus, ou intégrer le reçu
à la tête déjà liée par empreinte et répliquée dans Trust/SQLite. Décision :
seconde option, pour réutiliser intention → COMMIT → finalisation et son journal
chaîné sans nouvelle migration. Les anciennes capacités privées sans émetteur
restent identifiables par l’absence de reçu ; aucun endpoint public ne les expose.

Plan borné : (1) reproduire la perte du reçu au redémarrage ; (2) conserver
le contenu d’autorité fourni, sa référence, la session sous empreinte et les dates
d’émission/validité, dans la tête liée au candidat exact ; (3) vérifier copie
possédée, cohérence, altération et reprise aux deux frontières de durabilité ;
(4) actualiser contrat FR/EN, schéma et note scientifique. Critère observable :
le même reçu est relu après réouverture et reprise, un reçu invalide ou changé
est refusé, aucun jeton brut n’y est conservé. La fraîcheur d’un enrôlement externe
n’est pas prouvée par ce reçu historique. Résultats encore attendus.

Preuves : `verification-identites-agents-20261016/ia3-issuance-receipt-20261017/`.

### Replan IA3c.1 — sources mémoire concurrentes

Le reproducer `receipt-before-01` échoue comme attendu (reçu absent), mais
`memory_usage_binding.go` et son test changent pendant compilation : cette
tentative ne constitue pas une preuve stable. Cause : les deux chantiers
partagent aussi le paquet Go. Décision : prolonger l’overlay compilateur aux
seules sources mémoire voisines, conservées avec empreintes, sans toucher leurs
fichiers. Rejouer le reproducer sur cette vue, puis tester le candidat identité
réel avec les mêmes voisins. Limite : aucune acceptation du dernier code mémoire
ni de l’ensemble courant n’est déduite. Propriétaire Codex/méthode ; une isolation
structurelle du paquet reste IA3b-I07 ouverte.

Amélioration IA3c.1-I01 (Codex, découverte à la relecture) : vérifier une chaîne
mixte format 1 → format 2, pas seulement les deux formats isolés. Valeur :
conserver l’histoire existante sans downgrade ni rupture silencieuse. Décidée ;
variante à intégrer aux deux tests de reprise après la première exécution stable.
Critère : ancienne publication conservée, reçu exact relu après reprise, deux
instantanés/deux événements, refus de renouvellement avec la session expirée.

Résultat intermédiaire : `receipt-before-02` reproduit le reçu absent (1 FAIL
attendu, aucune entrée modifiée), puis `normal-contracts-01` passe 117 noms,
0 échec/ignoré, sources stables. La variante de chaîne mixte et le contrôle de
retour d’horloge sont maintenant ajoutés ; un nouveau candidat est figé avant
les contrôles finaux. Le résultat intermédiaire n’est pas réétiqueté final.

Prévention liée à IA3b-I05 : le lanceur de vérification IA3c.1 refuse maintenant
d’écraser un identifiant de tentative déjà présent. Les noms préalables,
intermédiaires et finaux restent distincts. Cette garde de méthode conserve les
preuves ; elle n’accorde pas un droit de relance sans précondition corrigée.

Le rendu Mermaid a d’abord échoué sur un chemin Chromium supposé absent ;
`diagram-diagnosis.json` conserve la cause. La reprise utilise la configuration
de navigateur déjà vérifiée du projet : quatre rendus SVG/PNG réussis, deux
images FR/EN inspectées. Aucun composant système installé ou modifié.

Observation liée à IA3b-I07 : la compilation instrumentée IA3c.1 dépasse la
référence précédente de 206 s, avec autre compilation simultanée et pression
mémoire/swap (`compile-observation.json`). Le compilateur progresse ; aucune
preuve d’échec ni résultat de test n’est déduit du temps. Hypothèse de coût :
paquet partagé volumineux et charge hôte, non expérience causale isolée. Action
dans nos droits : sérialiser nos compilations, conserver les sources figées et
les observations, ne pas interrompre l’autre chantier ni augmenter un délai de
test. La séparation structurelle reste ouverte avec le même propriétaire.

### Replan IA3c.1 — lot registre instrumenté trop large

`race-registry-01` atteint 90 s : 22 noms passés, un échec de lot par timeout
durant un cas de rollback ; l’exécution reste INCOMPLÈTE. Les 37 noms du reçu
et les 47 de l’émetteur passent dans leurs lots complets, sources/binaire stables.
Cause observée : cumul des fixtures de registre ; aucune assertion métier
fausse ni course de données n’est observée avant le timeout. La charge hôte
reste un facteur possible, non isolé. Décision : deux lots sémantiques bornés,
même binaire, mêmes sources et même délai, sans nouvelle compilation ou hausse
de budget. Les résultats du lot interrompu ne comptent pas comme acceptation.
Propriétaire Codex/méthode ; le coût structurel des fixtures reste IA3b-I06.

### Replan IA3c.1 — objection utilisateur au délai arbitraire

L’utilisateur conteste les 90 s. Diagnostic : valeur choisie par Codex dans
`-test.timeout`, héritée d’une recette antérieure, sans mesure ni exigence
justifiant ce délai pour le lot courant. Ce n’est pas une politique du moteur,
mais cela reste une borne arbitraire de notre méthode. Le découpage seul
contourne cette borne sans la justifier ; il ne clôt pas le diagnostic.

Décision : supprimer le délai global de la recette finale (`-test.timeout=0`,
qui désactive le watchdog global Go), suivre les événements nommés et diagnostiquer
une absence de progression. Aucun délai de remplacement, aucune politique
produit, consommation ou droits ne sont modifiés. Les opérations individuelles
gardent leurs contrats existants ; leur pertinence n’est pas déduite ici.
Rejouer le lot complet de 117 noms sur le même binaire instrumenté, sans
recompilation, puis le candidat normal final ; conserver le timeout original et
les lots intermédiaires sans les requalifier comme critère de performance.
Propriétaire Codex/méthode. Un résultat complet est encore attendu.

Transmission vérifiée : les accords personnels persistants et `AGENTS.md`
projet portent la règle de délai justifié ; le nouveau contexte utilisateur
reçu dans cette conversation inclut la règle personnelle. Les pages méthodes
FR/EN et la recette IA3c.1 utilisent explicitement zéro pour le watchdog global,
y compris la commande de reproducer conservée pour une autre restauration.
Les fichiers historiques restent inchangés. Le contrat embarqué n’est pas
modifié ; aucun processus déjà lancé n’est présumé avoir reçu cette règle.

Résultat du replan délai : `race-contracts-02` termine les 117 noms dans un
seul lot, 0 échec/ignoré, mêmes entrées et binaire, watchdog global désactivé.
Durée mesurée par le lanceur : 143.5 s (inclut son instrumentation).
Ce résultat retire la borne arbitraire comme critère d’acceptation ; il ne
constitue pas une mesure de performance produit ni une résolution de l’incident
de latence sous charge. Les lots instrumentés intermédiaires restent séparés.

### Résultat IA3c.1 — reçu vérifié et délai arbitraire retiré

Les lots finaux complets passent chacun 117 noms (37 reçu, 47 émetteur, 33
registre), 0 échec/ignoré, en normal et sous `-race`, sans watchdog global.
Les 1064 entrées sont identiques et les binaires inchangés ; vet et compilation
application passent, sans déploiement. Le reçu survit aux deux reprises et à
la chaîne mixte, les altérations sont refusées. Les schémas FR/EN sont inspectés.
Le rapport et ses manifestes conservent chaque anomalie, ses preuves et la
correction de méthode sur les 90 s, transmise aux instructions persistantes.

IA3c.1-I01 vérifié. IA3b-I04 partiel : reçu historique réalisé/vérifié, source
d’enrôlement protégée et fraîcheur indépendante encore ouvertes. Suite : source
courante d’enrôlement et témoin, puis admission ; Codex porte l’implémentation,
l’opérateur désigne principal initial et frontière indépendante. Aucun droit
implicite ni activation réelle. F01–F07, IDA, mémoire, IA4/IA5 et RETEX global
restent ouverts. Voir le [rapport IA3c.1](verification-identites-agents-20261016/ia3-issuance-receipt-20261017/RAPPORT-IA3c1.md).

Précision explicite de l’utilisateur après vérification : prudence avec les
timeouts temporels et les autres blocages ajoutés aux tests tant que les limites
du code ne sont pas comprises. Consigne élargie dans les accords globaux,
`AGENTS.md` et méthodes FR/EN : origine/portée/risque, observation et mesure avant
ajout ou durcissement, distinction lenteur/blocage/limite fonctionnelle/méthode.
Aucune protection de sécurité, droits ou budget n’est contournée. Changement de
consignes et de documentation uniquement : les sources Go et entrées des tests
restent identiques, donc les résultats 117/117 sont conservés sans recompilation.


## IA3c.2a — authentifier une réponse d’enrôlement, reprise du 17 octobre

Besoin : source actuelle d’enrôlement, IA3b-I04 encore partiel après le reçu
historique. La frontière de déploiement (compte système distinct local ou service
séparé) est demandée à l’utilisateur ; aucune activation réelle n’est décidée.
La relecture de COV-ID-04 impose la révocation des droits dérivés et distingue
prise en compte durable, arrêt demandé et extinction observée. Une signature
seule ne prouve ni fraîcheur actuelle ni absence de course avec publication.

Deux voies comparées : réutiliser les clés de preuve Trust, ou un protocole
d’autorité distinct avec clé publique épinglée hors des réponses/imports.
La première reste circulaire avec la configuration cockpit et sa protection
non établie ; seconde retenue pour préparer l’authenticité du futur canal.
Deux déploiements restent compatibles ; leur point de linéarisation est à
choisir/vérifier avant le consommateur. Aucun nouveau réglage d’exécution.

Incrément indépendant : `internal/enrollmentproof/`, Codex seul. Challenge
aléatoire 256 bits, liaison clé/projet/demande complète, enveloppe plate stricte,
signature Ed25519 séparée par domaine, une consommation locale sous concurrence.
Le résultat est un énoncé authentifié opaque, jamais un permis ; aucun consommateur
moteur n’est raccordé. Clé initiale, service durable, révocation/fraîcheur, témoin
plan et admission publique restent ouverts. Les choix crypto définissent le
format ; aucune borne temporelle ou limite métier n’est introduite.

Plan : définir les cas acceptés/refusés ; vérifier producteur et décodeur (clé
épinglée, contexte, mutation, rejeu, concurrence, entrées malformées) ; corriger
avec preuves ; vérifier compilation/vet/race du paquet isolé, sans délai global ;
mettre à jour contrat FR/EN, schéma et note scientifique. Les 117 contrôles
IA3c.1 ne sont pas rejoués : leurs sources ne sont pas modifiées. Ils ne deviennent
pas une acceptation de ce nouveau protocole ni du dernier chantier mémoire/web.

État de méthode conservé : un candidat de protocole a été écrit avant ce suivi
et ses tests ; il n’est pas qualifié vérifié. Son original est conservé dans
`ia3-enrollment-proof-20261017/`. La première relecture identifie une ambiguïté
possible `payload: null` versus chaîne vide ; écrire le contre-exemple et vérifier
ce défaut avant toute correction. Les preuves finales restent attendues.

### Replan IA3c.2a — `null` accepté comme chaîne vide

Le test `TestResponseNullIsNotAString` échoue : `json.Decoder.Decode(&string)`
accepte JSON null sans erreur, transformé en chaîne vide. Une réponse vide
correctement signée reste donc valide après cette altération de type. Correction
décidée : exiger un token JSON de type chaîne pour chaque valeur du cadre plat.
Le refus ne consomme pas le challenge ; la réponse valide doit rester acceptable.
L’original `null-before.jsonl` et les sources avant correction sont conservés.
Limite de preuve de cette première tentative : les empreintes sont capturées
après exécution, avant correction, pas avant/après ; les vérifications suivantes
seront liées aux deux captures et aux mêmes sources stables. Aucun droit accordé
par ce candidat non raccordé. Propriétaire Codex/protocole.

Résultat intermédiaire : 31 noms PASS normal et race, vet/compilation PASS, quatre
entrées inchangées (paquet isolé et module). La relecture justifie trois variantes
additionnelles : empreinte de challenge réécrite sur signature capturée, signature
valide sous clé non épinglée, signature d’un autre domaine. Ces gardes évitent
qu’une erreur commune producteur/vérificateur échappe au simple aller-retour.
Les sources déjà contrôlées restent conservées par empreinte ; les variantes
seront exécutées sous de nouveaux identifiants 02, sans écraser les résultats.

### Replan IA3c.2a — copie de valeur et consommation du challenge

Les 34 contrôles précédents passent en normal/race, mais la relecture de l’API
exportée révèle une copie possible de `Challenge`, donc de son mutex et booléen
de consommation. `copy-before-01` reproduit le double succès (1 FAIL attendu,
quatre entrées inchangées). Correction générale : état interne partagé sous
pointeur privé, copies de valeur comprises ; renforcer aussi le test concurrent
avec des copies et le refus de reconstruction JSON. Pas de clé/identifiant
particulier ni de limite ajoutée. Propriétaire Codex/protocole.

Le premier patch multi-fichiers de l’état partagé a été refusé : contexte supposé
différent du goroutine mis en forme. Aucun fichier n’avait changé (inspection
confirmée). La reprise lit les lignes réelles, applique deux patches exacts puis
relance sous identifiants 03. Il s’agit d’une erreur de méthode corrigée, sans
réécriture de preuve ni modification de sources pendant tests.

### Résultat IA3c.2a — authenticité isolée vérifiée, courant encore ouvert

35 noms PASS normal et race, aucun échec/ignoré, quatre entrées identiques ;
vet et compilation du paquet passent, sans watchdog global. Les deux défauts
du candidat sont reproduits puis corrigés, avec originaux et limites de preuve.
La consommation reste unique sous copies/concurrence. Schémas FR/EN rendus et
inspectés. Le nouveau paquet n’a aucun import moteur/CLI ; les six sources Go
IA3c.1 sont inchangées, leurs tests ne sont pas rejoués ni requalifiés globalement.

Le [rapport IA3c.2a](verification-identites-agents-20261016/ia3-enrollment-proof-20261017/RAPPORT-IA3c2a.md)
et le [contrat cible](identites-agents-enrolement-IA3c2.md) distinguent explicitement
message authentique, source courante, autorisation et effet. Un test conserve la
limite de fraîcheur, sans la présenter comme protection acquise. Suite : choix
opérateur de frontière (compte distinct local ou machine séparée) et principal
initial, service courant/durable et point de linéarisation, puis raccord moteur.
Aucun choix implicite, enrôlement réel, déploiement ni permission nouvelle.
IA3b-I04, F01–F07, IDA, mémoire, IA4/IA5 et RETEX global restent ouverts.


## IA3c.2b — journal courant d’enrôlement, reprise du 17 octobre

Besoin repris : conserver l’état de la source après redémarrage et empêcher
une réponse active après sa révocation locale. Le choix local/compte distinct
ou autorité séparée reste sans réponse ; `go next` autorise le travail commun,
pas une activation ou une désignation implicite du principal initial. Replan
appliqué ; contrat et preuves IA3c.2a relus, sources existantes préservées.

Deux voies : journal fichier et synchronisation/rename propres, ou transactions
SQLite avec chaîne d’événements signés. SQLite retenu, déjà dépendance du projet,
pour réutiliser l’atomicité plutôt que recréer un protocole de fichiers. La tête
attendue reste un témoin explicitement fourni par le contrôle, jamais tiré de
l’import pour lui donner autorité. Restaurer ensemble journal et témoin reste
hors garantie ; aucune séparation OS n’est revendiquée par des champs privés.

Périmètre Codex seul : nouveau `internal/enrollmentstate/`, journal SQLite dédié,
enregistrements opaques signés sous une clé épinglée et un projet exacts. Les
permissions métier du payload restent à valider par l’adaptateur moteur futur.
Un identifiant créé ne peut être réenrôlé après révocation ; un nouvel enrôlement
éventuel exige un autre identifiant et une décision explicite. Les données
historiques restent immuables. Aucun secret privé persistant ni timer/retry caché.

Plan et critères avant code : (1) écrire les tests de contrat ; (2) implémenter
signature/chaîne/CAS et transactions ; (3) vérifier création/lecture active,
révocation irréversible, refus sans mutation, réouverture, replay exact après
réponse perdue, rollback détecté par tête externe, altération et contexte erroné ;
(4) vérifier une lecture protégée sous transaction écrivain et le refus après
révocation, avec contention explicite sans faux succès ; (5) tests normal/race,
vet/compilation ciblés sans délai global ; documents FR/EN et schémas actualisés.
Les sources IA3c.1/2a ne changent pas ; leurs tests ne sont pas rejoués sans cause.

Le verrou SQLite fournit une sérialisation locale du journal, pas une garantie
de révocation/départ distribuée. Les erreurs de contention remontent sans retry
automatique ; aucune limite d’exécution nouvelle n’est inventée. Le service
réel, le témoin protégé durable et le raccord des lanceurs restent ouverts.

### IA3c.2b — contention du candidat et replan

Premier passage `normal-01` : 14 noms PASS, un FAIL, aucun écrivain concurrent
n’ayant réussi ; aucune mutation multiple. Le diagnostic instrumenté suivant
passe : défaut dépendant de l’ordonnancement, cause exacte non isolée par la
première sortie qui ne conservait pas les erreurs individuelles. Cette lacune
de méthode est corrigée dans la fixture. Les sources du premier candidat et
les sorties originales sont conservées.

Inspection du pilote SQLite installé : transaction différée par défaut ; le
no-op UPDATE doit passer par une acquisition différée, et le journal rollback
autorise une contention lecteurs/COMMIT. Deux routes : mutex/retry applicatif
ou réservation immédiate et journal WAL. Retenir `_txlock=immediate` et WAL
comme contrat technique de stockage local pour éviter la promotion de verrou
et la contention lecteur/COMMIT ; pas de retry, attente ou plafond de politique.
Vérifier aussi un lot de calendriers concurrents frais, puis normal/race.
Cette correction de protocole ne prouve pas à elle seule la cause précise du
premier échec ni une disponibilité face à un acteur tenant indéfiniment un verrou.


### Résultat IA3c.2b — critères du journal vérifiés, enrôlement global ouvert

42 noms PASS normal et race, zéro échec/ignoré ; vet et compilation du paquet
PASS, sept entrées inchangées avant/après chaque contrôle final, watchdog global
explicitement désactivé. Le scénario concurrent passe sur 32 fixtures fraîches
à huit écrivains, sans event dupliqué et avec au moins un succès par fixture.
Les refus de SignActive ne retournent aucun octet. Injection de faute à la tête :
aucun événement partiel, puis reprise après suppression de la faute. Création,
réouverture, révocation, retry exact courant, anciens replays, contextes invalides,
JSON canonique et annulation sont couverts. Restaurer le témoin aussi retire
la détection de rollback : cette limite reste un critère futur, pas un PASS global.

Le [rapport IA3c.2b](verification-identites-agents-20261016/ia3-enrollment-state-20261017/RAPPORT-IA3c2b.md)
conserve sources, échec initial et diagnostic partiel, corrections, commandes,
preuves, améliorations I2b-01 à I2b-05 et conditions de reprise. Quatre rendus de
schémas FR/EN sont réussis et les deux PNG inspectés. Dix sources de référence
précédentes sont inchangées ; aucun raccord moteur/CLI du nouveau paquet.
La documentation opérationnelle, le contrat cible et les notes scientifiques
sont actualisés dans cet incrément. Pas de changement de consignes embarquées
ni de processus déjà lancé ; ces documents persistants ne mettent pas un service
ou un autre contexte en exécution.

Suite préparatoire indépendante : schéma métier strict de l’autorité avant tout
raccord, sans enrôler une identité réelle. Activation et séparation OS attendent
le choix opérateur toujours manquant (compte distinct local ou machine séparée),
le principal humain initial et le bootstrap vérifié. Témoin externe durable,
linéarisation à l’usage, admission publique, descendants et droits mémoire restent
à réaliser. F01–F07, IDA, IA3b-I04, IA4/IA5 et RETEX global ne sont pas clos.


## IA3c.2c — schéma d’autorité et génération vérifiables, reprise du 17 octobre

Besoin repris : ne plus laisser le corps opaque du journal devenir une autorité
par sa seule signature. IA3c.2b vérifie le journal local ; service protégé,
bootstrap et moteur public restent ouverts. Codex seul, checkout/branche conservés,
aucun enrôlement réel, compte/service ou fournisseur externe. Replan et APEX lus.

Deux voies : recopier les règles privées du moteur dans un schéma autonome, ou
extraire les primitives communes sans changer le JSON ni les décisions existantes.
Retenir un petit `internal/identitycontract` sans dépendance moteur, avec principal,
portée, permission, texte, digest et syntaxe du nom déjà appliquée par `safeName`.
Le moteur conserve ses types privés et délègue leur validation ; `model.go`
délègue la même syntaxe du nom. La borne de représentation existante de 100 octets
reste inchangée : aucune nouvelle politique opérateur ni limite temporelle.

Nouveau `internal/enrollmentauthority` : document versionné strict, principal
humain complet (ID/kind/issuer/sujet opaques), projet, portée/version/digest du
plan, révision Trust, droits exacts, délégables sous-ensemble, dates explicites
et génération. Les données décodées ne sont jamais une capacité privée moteur.
Le producteur typé valide avant de signer un changement `enroll` ; le consommateur
valide un Statement authentifié, sa demande, sa tête attendue et tout le document.
Pas de rôle implicite, wildcard, défaut de durée ou normalisation d’issuer/sujet.

Amélioration découverte : la génération ne doit pas être seulement un entier
signé. Ajouter `enrolled_sequence` à Record/ActiveReply, dérivé de l’événement
vérifié de création ; génération du document égale à cette séquence, même après
d’autres événements. La tête courante reste un témoin distinct. Ce changement
de cadre de lecture est explicite, sans consommateur moteur existant ; les anciens
cadres ne satisfont pas le nouveau consommateur strict. Le schéma du journal
et les événements signés existants ne changent pas.

Plan avant code : tests du schéma et des refus ; primitives partagées ; schéma
et producteur/consommateur ; raccord séquence du journal ; vérifications locales
normal/race, tests ciblés du moteur pour sérialisation, chaînes, noms et émetteur ;
vet/compilation ciblés. Aucun watchdog global (`-timeout=0`). Sources figées par
empreintes avant/après, pas d’édition pendant compilation. Documentation FR/EN,
contrat cible, schémas, preuves et notes scientifiques dans le même incrément.
La fraîcheur finale, le témoin durable, les descendants et F01–F07 restent ouverts.

### IA3c.2c — preuve de collecte du paquet commun

Les lots 01 et 02 passent (102 puis 104 noms), mais le script n’avait pas ajouté
le paquet commun à ses commandes de test : remplacement textuel visant `-count`
au lieu du véritable argument `-count=1`. Les sources des nouveaux tests étaient
hashées, leurs tests propres non exécutés. Aucun de ces lots ne prouve donc leur
collecte. Correction du lanceur, assertion par paquet d’un nombre de tests nommés
positif et lot 03 sous nouvel identifiant. Pas de source Go modifié pendant tests,
originaux préservés ; la compilation du paquet n’est pas présentée comme ses tests.

### IA3c.2c — inventaire des entrées compilateur corrigé

La précondition de vérification moteur échoue avant lancement : `go list -test`
retourne aussi un testmain généré dans le cache Go, et le lanceur exigeait à tort
un chemin relatif au dépôt pour chaque fichier. Aucun test moteur n’a alors été
exécuté. Script fautif conservé, erreur référencée dans les preuves ; pas de fichier
stderr original capturé par ce premier lanceur. Retenir aussi les entrées générées
sous chemin absolu et empreinte, au lieu de les omettre ; tentative 02 après cette
précondition corrigée. Le résultat ne requalifie aucune protection produit.

### IA3c.2c — dépendances mémoire modifiées après les contrôles moteur

Les 173 contrôles moteur, vet et compilation 02 passent avec 1 068 entrées
inchangées pendant chaque vérification. Au contrôle de réutilisation ultérieur,
trois fichiers du chantier mémoire ont changé (`memory_usage_binding.go`,
`memory_usage_binding_test.go`, `memory_usage_decision_test.go`). Les 02 restent
historiquement valides mais ne couvrent pas le dernier état complet. Préserver
ces fichiers sans les éditer ; replan, nouveau contrôle 03 du nouvel état avec
empreintes avant/après. Aucune relance fondée sur le seul passage du temps :
les entrées compilateur ont effectivement changé. Les sources identité propres
restent stables ; lot local 04 : 114 noms normal/race PASS, 13 entrées inchangées.


### Résultat IA3c.2c — document et génération vérifiés, admission ouverte

114 noms PASS normal/race : 9 communs, 63 d’autorité et 42 du journal, aucun
FAIL/SKIP ; 13 entrées stables. Les 173 noms moteur ciblés passent, vet et
compilation application passent sur 1 069 entrées stables dans chaque contrôle 03.
Les 02 restent historiques avant les nouveaux changements mémoire. Aucun watchdog
global. Génération stable après autres événements et réouverture, fausse génération
pourtant signée refusée, contexte complet exact, schéma canonique et fenêtres
explicites vérifiés. Un ancien témoin peut encore accepter son ancienne preuve :
ce n’est pas une preuve de fraîcheur ou de non-révocation à l’usage.

Le [rapport IA3c.2c](verification-identites-agents-20261016/ia3-enrollment-authority-20261017/RAPPORT-IA3c2c.md)
lie sources, commandes, défauts de méthode, améliorations I2c-01 à I2c-06 et limites.
Quatre schémas FR/EN sont rendus, deux PNG inspectés. Douze fichiers de référence
sont inchangés ; seules trois sources existantes changent comme prévu (contrat
privé, syntaxe de nom partagée et Record du journal). Le nouveau document est
une donnée validée, jamais un permis moteur. Pas de raccord public, enrôlement
réel, déploiement ou mise à jour d’un processus déjà lancé. La documentation
persistante prépare les prochains contextes ; aucun contexte embarqué de méthode
n’est modifié par cet incrément.

Suite : EP-02, demande complète canonique résolue et liée au challenge ; puis
conversion autorisée vers le producteur privé, sous source courante et point de
linéarisation. Le cadre de lecture devra être versionné avant premier client
externe (I2c-06, aucun client réel existant). Choix d’hébergement et principal
initial toujours attendus avant bootstrap/activation. F01–F07, IDA, IA3b-I04,
IA3 public, IA4/IA5 et RETEX global restent ouverts.


## IA3c.2d — demande complète liée au challenge, reprise du 17 octobre

Besoin repris EP-02 : ne plus laisser `request_digest` comme empreinte fournie
sans schéma de demande complète. IA3c.2c vérifie document/génération et compatibilité
privée ; source protégée, bootstrap, témoin durable et admission publique restent
ouverts. Même checkout/branche, Codex seul, sources existantes préservées. Replan
et APEX appliqués ; aucun choix d’hébergement ou principal déduit de « suite ».

Deux voies : hacher une projection de quelques identifiants ou couvrir un cadre
versionné contenant aussi les octets complets du candidat. Retenir un nouveau
`internal/enrollmentrequest`, cadre canonique de publication du registre uniquement,
avec domaine/version explicites : humain complet, empreinte et dates de session,
projet, portée et génération du plan, révision Trust, événement, instantané entier
et son empreinte/révision, racines exactes, checkpoint précédent, référence complète
d’enrôlement (ID, génération, tête et empreinte du document). Aucun jeton brut.
Le champ Snapshot reste le contenu exact lié ; sa validation métier appartient
au moteur, pas à une seconde implémentation des grants.

Le handle préparé possède ses données et son empreinte ; ses copies ne doivent
pas permettre de modifier une demande après création du challenge. Le consommateur
projette ses attendus depuis ce même cadre, compare le digest du document retourné
et les dates explicites, sans prétendre réauthentifier une session ou relire une
source. Le service devra recevoir le cadre complet et vérifier son lien au challenge.

Ajouter seulement une projection privée moteur depuis ses cadres de caller,
autorité et checkpoint, avec les validations existantes et le JSON canonique
complet de son snapshot. Cette fonction pure ne lit ni Trust ni SQL et ne prouve
pas leur actualité : leur résolution et la sérialisation au raccord final restent
à réaliser. Aucun nouveau chemin public ni permis créé. Pas de recopie du moteur,
de modification des chantiers mémoire ou de nouvelle politique d’exécution.

Plan avant code : tests du schéma, substitutions de chaque dimension, propriété
des données/copies Go, liaison challenge et document ; implémentation du cadre ;
projection moteur et tests avec sa fixture d’émetteur humain ; normal/race ciblés,
vet/compilation et contrôles de collecte par paquet, sans watchdog global ; docs
FR/EN, schémas, preuves et notes scientifiques actualisés. La version du cadre de
lecture I2c-06 reste requise avant premier client externe ; aucune activation ici.

### IA3c.2d — premières vérifications et lien de session

62 noms PASS normal/race, puis vet refuse quatre littéraux de types externes
sans noms de champs dans les fixtures. Cause établie et trace conservée ; corriger
les quatre littéraux avec des champs nommés avant le lot 02, sans toucher au contrat.

Amélioration nécessaire constatée : la projection ne doit pas recevoir séparément
un caller vérifié et un autre jeton dont elle hacherait la valeur. Conserver
`sessionDigest` dans le résultat privé de `resolveAgentIdentityCaller`, calculé
avec la même référence de jeton vérifiée par Trust. Le nouveau producteur ne reçoit
alors aucun jeton brut. Adapter `agent_identity_contract.go` et vérifier résolution,
projection et consommateurs concernés ; les frames runtime sans session restent
sans ce champ, refusées pour l’enrôlement humain. Pas de nouvelle autorité implicite.


### Résultat IA3c.2d — liaison locale vérifiée, producteur courant encore ouvert

62 noms PASS normal/race, zéro FAIL/SKIP ; vet/compilation du paquet PASS, 13 entrées
inchangées. 28 noms moteur ciblés PASS ; vet moteur et compilation application
PASS, graphe de 1 080 fichiers stable pendant chaque contrôle. Aucun watchdog
global. La seule source existante modifiée ajoute/conserve sessionDigest dans
le caller privé ; 16 sources précédentes inchangées, plus nouveaux fichiers de
cadre et projection. Les sources mémoire ne sont pas éditées.

Le [rapport IA3c.2d](verification-identites-agents-20261016/ia3-enrollment-request-20261017/RAPPORT-IA3c2d.md)
conserve sources, commandes, vet initial refusé puis corrigé, améliorations I2d-01
à I2d-05 et limites. Cadre immuable sous copies, dimensions toutes liées, fenêtres
explicites et substitution du document exact sont vérifiés. La projection privée
valide les données moteur mais ne lit pas leur source courante : aucune confusion
avec un producteur de permis, une approbation ou une admission publique.
Documentation FR/EN, contrats et notes scientifiques actualisés dans le même
incrément ; schémas rendus et inspectés, sans changement d’un contexte de méthode
embarqué ou d’un processus déjà lancé.

Suite : frontière protégée, témoin indépendant durable, état courant et point
de linéarisation avant raccord. Versionner le cadre de lecture avant premier
client externe (I2c-06). Choix opérateur d’hébergement et principal initial toujours
manquants avant bootstrap réel ; aucune activation induite par « suite ».
EP-02/03 restent partiels au raccord ; F01–F07, IDA, IA3 public, IA4/IA5 et RETEX
global ne sont pas clos.


## IA3c.2e — cadre de lecture versionné, reprise du 17 octobre

Besoin I2c-06 repris : avant un premier client externe, ne pas interpréter une
réponse signée d’un autre format par tolérance implicite. Journal et cadre de
demande IA3c.2d relus ; aucun client/service externe ou choix de frontière activé.
Replan appliqué, Codex seul ; checkout/branche et travaux mémoire préservés.

Deux voies : continuer le décodage local non versionné chez chaque consommateur,
ou définir une version explicite et un décodeur strict propriétaire du cadre.
Retenir `ActiveReply.schema_version = 1`, émis par SignActive ; décodage canonique
commun dans enrollmentstate, refus des versions absentes/0/inconnues, aucun repli
vers l’ancien cadre. Le consommateur d’autorité délègue ce décodage au journal.
Les fixtures forgeant volontairement un cadre valide doivent donner sa version,
pour que leurs tests continuent à exercer la bonne frontière. Aucun changement
de schéma SQLite, événement signé, racine ou protocole de challenge.

Périmètre : journal et décodeur de lecture, consommateur d’autorité, fixtures
autorité/demande ; tests avant code d’un ancien cadre authentique et des versions
inconnues/null/types erronés/omissions, version courante et données opaques vides,
refus sans document ni mutation. Normal/race des trois paquets touchés, analyse
statique et compilation ciblées, sans watchdog global. Pas de source moteur modifiée.
La sélection supportée est explicitement {1} ; aucune négociation automatique ni
downgrade. Négociation/transport entre vrais pairs restent à vérifier avant client
externe ; I2c-06 ne sera pas clos globalement par ces seules fixtures.

Documentation FR/EN, schémas, contrat cible, preuves et notes de recherche dans
le même incrément. Actualité de la source, témoin durable, admission publique et
risques F01–F07 restent ouverts. Ni version ni signature ne créent de droit.

### IA3c.2e — contre-exemple avant correction

Le premier test préalable référençait le futur ErrReplyVersion : échec de
compilation, zéro test exécuté, preuve comportementale absente. Retirer cette
assertion dépendante pour tester d’abord les interfaces existantes ; original
conservé dans les sorties 01. Le lot `version-red-02` exécute dix noms : huit
PASS et deux FAIL (parent et sous-cas `absent`). Une réponse signée pour un
challenge neuf, avec le cadre antérieur sans version, retourne effectivement
un document d’autorité valide. Une omission de version est donc indistinguable
d’une sélection volontaire de ce format ; les autres huit variantes sont déjà
refusées par la comparaison canonique. Source du test archivé sous son empreinte.
Correction : version explicite et décodeur commun, pas de contournement par fallback.


### Résultat IA3c.2e — format local vérifié, interopérabilité externe ouverte

197 noms PASS normal/race : 62 journal, 73 autorité, 62 demande, aucun FAIL/SKIP ;
16 entrées stables. Vet et compilation ciblés passent. Application compilée sur
1 083 fichiers stables pendant le contrôle ; seules des assertions de test ont
ensuite été renforcées et exécutées, aucun code de production modifié. Sources
moteur et mémoire préservées. Ancien cadre authentique sans version refusé ;
causes ErrReplyVersion/ErrDocument préservées, corps opaque vide toujours valide
sans droit, autres fixtures de contenu en version courante explicite.

Le [rapport IA3c.2e](verification-identites-agents-20261016/ia3-enrollment-reply-20261017/RAPPORT-IA3c2e.md)
conserve le contre-exemple, la précondition de test initiale en échec, sources,
commandes et améliorations I2e-01 à I2e-05. Documentation FR/EN et schémas rendus
et inspectés ; aucun contexte de méthode embarqué ou processus lancé modifié.
I2c-06 est réalisé pour le format/refus locaux ; sélection et interopérabilité
entre vrais pairs restent à vérifier avant premier client externe, sans fallback.

L’opérateur est de nouveau sollicité pour la frontière de service ; aucune option
présélectionnée ne vaut décision ni installation. Choix et principal initial
conditionnent le bootstrap réel ; témoins protégés, source courante, linéarisation,
admission publique, F01–F07, IDA, IA4/IA5 et RETEX global restent ouverts.

### Comparaison demandée — hébergement et menace

L’utilisateur demande les choix, risques et conséquences avant de choisir.
Le tableau du [plan d’enrôlement](identites-agents-enrolement-IA3c2.md) compare
service local confiné, autorité distante à administration séparée et préparation
sans activation. Les contrats FR/EN conservent la même limite : isolation de la
clé et authentification du pair ne prouvent pas l’autorisation de la demande.
Recommandation conditionnelle, non décision : local si noyau/administrateur hôte
sont de confiance ; séparation de machine et d’administration si la menace
inclut la compromission de l’hôte Swarm. Aucun choix ni droit d’installation
n’est déduit de la question ou d’une option présélectionnée.

Amélioration I2e-06, origine : explicitation du choix et confusion possible entre
protection de clé et autorisation. Valeur : empêcher une fausse garantie de
séparation. Périmètre : contrat et futurs transports ; propriétaires : Codex pour
les contre-exemples, opérateur pour la frontière. Clarification documentée ; effet
réel ouvert jusqu’au refus observé d’une demande interdite par un pair authentifié,
dans le déploiement retenu. L’indépendance du protocole permettrait une comparaison
locale/distante pour la recherche, mais son interopérabilité réelle reste à prouver.

Une tentative d’édition combinée a échoué avant mutation sur un contexte de patch
inexistant. Relecture du tableau inchangé, suppression du hunk non pertinent et
application ciblée réussie ; original et vérification conservés dans les preuves.
Cette correction documentaire ne nécessite pas de relancer les tests Go inchangés.
Le choix opérateur et le principal initial restent ouverts ; poursuivre les briques
indépendantes ne vaut pas activation ni clôture des risques F01–F07.


## IA3c.2f — décision adoptée : local pilote, autorité distante prévue

17 octobre 2026. Précondition modifiée par la décision explicite de l’utilisateur :
« on fait 1 mais on prévoit 2 possible pour l’avenir ». La frontière choisie n’est
plus en attente : service local sous compte système distinct avec confinement
vérifié pour le développement/pilote ; transport et configuration conçus pour une
autorité sur machine/administration séparées à l’avenir. Le souhait probable des
futurs clients motive cette cible ; ce n’est pas une demande client mesurée.
Le principal humain initial reste à désigner ; le choix de frontière n’active pas
un service et ne fait pas de l’Admin ou du premier appelant son autorité humaine.
Les mentions antérieures « choix en attente » sont l’historique avant cette décision.

Méthode : replan/APEX, Codex seul, checkout et travaux existants préservés.
Comparer deux réalisations : coder un accès local direct au journal, ou garder
un contrat d’autorité commun et un adaptateur de canal. Retenir le second : la
frontière locale doit être réelle dès le pilote, et le mode distant ne doit pas
nécessiter de réécrire les contrôles métier. Écarter un simple booléen « distant »
ou un réglage d’URL sans authentification, autorisation et source courante.

Constat source : install.sh connaît Docker/natif, compose.yaml ne définit qu’un
service Swarm sous l’UID/GID opérateur ; ni l’un ni l’autre ne configure l’autorité
d’enrôlement dédiée. trustRemoteOrigin ne permet qu’une origine HTTP de boucle
locale pour le client Trust existant ; ce n’est pas l’adaptateur distant futur
de l’autorité d’enrôlement. L’Admin propose déjà des réglages révisés et des
prévisualisations ; cela ne prouve pas une configuration d’autorité existante.

Périmètre de cette décision : plan, cible d’enrôlement, contrats FR/EN et guides
d’installation FR/EN. Pas de modification d’install.sh, Compose, frontend ou
source moteur pendant ce cadrage. Aucune création de compte, clé, service ni
modification de processus. Les exigences et leur état sont dans le contrat cible.

| Suite dépendante | Propriétaire / livrable | Critère et reprise |
| --- | --- | --- |
| IA3c.2f-01, décision et parcours | Codex ; contrats FR/EN et installation | Même choix local pilote, distant prévu ; aucune capacité distante annoncée comme livrée |
| IA3c.2f-02, configuration protégée | Codex ; schéma et stockage d’installation, validation commune setup/API/CLI/Admin | Persistance/réouverture, révision attendue, preview sans mutation, refus des sources agents/import ; condition : design des opérations publiques et droits validé |
| IA3c.2f-03, service local | Codex ; commande/service séparé, canal authentifié, clés et témoin protégés | Depuis un fournisseur réel de fixture : ni accès au contrôle ni demande hors droits ; vérification des frontières OS et des erreurs ; aucun compte initial inféré |
| IA3c.2f-04, configuration UX | Codex ; installation initiale et modal Admin commun aux UX | Choix enregistré/état effectif distincts, FR/EN, thèmes, clavier, refus, sauvegarde/réouverture ; dépend de -02 et des états publics réels de -03 |
| IA3c.2f-05, raccord moteur | Codex ; source courante et admission publique | EP-04/05/10 : révocation concurrente, descendant, panne et reprise incertaine, aucun secours permissif ; dépend de -03 |
| IA3c.2f-06, mode distant et migration | Codex/opérateur ; adaptateur et procédure ultérieurs | Authentification mutuelle et droits identiques, partitions/reprise, continuité des preuves et anciennes racines ; aucune remise à zéro ni clé apprise du pair ; non implémenté |

Amélioration I2f-01 : origine décision utilisateur ; valeur : éviter un verrouillage
local du produit et une promesse de sécurité non démontrée. Périmètre : contrat,
installation et UX ; propriétaire : Codex. Décision adoptée et documentée ;
interopérabilité et migration réelles ouvertes jusqu’à une preuve entre deux
déploiements distincts. Amélioration I2f-02 : distinguer configuration souhaitée,
configuration appliquée et preuve d’isolation ; critère : un réglage sauvegardé ou
un healthcheck ne doit pas afficher une protection vérifiée. Documentée, effet
runtime/UX non réalisé. F01–F07 et l’objectif global restent ouverts.

La vérification de cette tranche porte sur cohérence des décisions, liens, rendus
FR/EN et préservation des sources ; aucune recette runtime n’est inventée pour une
modification documentaire. Les contrôles de configuration, UI et moteur seront
requis dans les tranches qui les modifient. Preuves dans
[le rapport de décision](verification-identites-agents-20261016/ia3-deployment-choice-20261017/RAPPORT-IA3c2f.md).


## IA3c.2f-02a — noyau de configuration d’installation, reprise du 17 octobre

Besoin : rendre le choix local du pilote persistant hors d’un projet importable,
sans transformer le réglage en preuve d’isolation ou autorisation. Checkout et
preuves antérieures conservés ; Codex seul, replan/APEX. Les réglages de projet
existants ne forment pas la frontière requise. Deux voies : réutiliser un fichier
sous .swarm du projet, ou fournir un stockage dédié ouvert par l’installation
protégée. Retenir le second ; ne pas assimiler le client Trust loopback existant
à une autorité distante. Aucun endpoint public ni activation par défaut ajouté.

Périmètre annoncé avant édition : nouveau paquet internal/enrollmentconfig
(format, stockage Linux et tests), contrat cible/plan, contrats opérationnels
FR/EN, notes d’installation FR/EN et preuves sous ia3-installation-config-20261017.
Aucun source moteur, frontend, service réel ou mission modifié. Pas de politique
de délai, retry, budget, rétention ou plafond d’historique nouvelle.

Contrat retenu : pins d’installation fournis indépendamment (digest d’installation,
clé publique et principal humain exact), valeurs locales explicites ; distant
connu mais refusé. Snapshot canonique version 1, historique chaîné et checkpoint
indépendant requis. Révisions en chaînes décimales uint64 pour éviter une perte
de précision JavaScript ; overflow est un refus de représentation. Le chemin
socket utilise la limite technique Linux exposée, pas un plafond opérateur.

Le répertoire doit préexister, appartenir au compte attendu et être privé ;
parcours sans symlink et contrôle des ancêtres, ouvert via descripteurs. Le
verrou privé peut être initialisé à l’ouverture du handle ; Preview ne crée ni
fichier de configuration ni événement. Écritures atomiques, fsync fichier puis
répertoire, CAS, retry exact du seul événement courant. Erreur après rename :
résultat durable incertain, sans faux succès ; reprise explicite et observation.

L’authorizer est une dépendance de contrôle obligatoire, appelée sous réservation ;
le principal retourné doit égaler le bootstrap épinglé. Un principal déclaré dans
la demande ne suffit pas ; fixtures d’authorizer ne prouvent pas une session OIDC
réelle. Les données et projections retournées ne sont pas des capacités moteur.
L’ouverture du répertoire sous l’UID du test ne prouve pas un fournisseur confiné.

Critères : codec/types/versions/refus sans résultat partiel ; create/read/reopen ;
preview sans mutation ; CAS et événements exacts/conflits ; authentification refusée
sans mutation ; mauvais pins/checkpoint, rollback cohérent, symlinks/permissions/
propriétaire/type de fichier ; annulation, concurrence et erreur de persistance
avec reprise. Tests écrits avant sources ; nouvelle capacité absente auparavant,
aucun échec de compilation ne sera présenté comme contre-exemple comportemental.
Go normal/race ciblés avec -timeout=0, vet/build du paquet, diff et docs FR/EN.

IA3c.2f-02 reste partiel tant que témoin indépendant durable, opérations publiques,
service et UX ne sont pas raccordés. Ce noyau ne rend aucun réglage actif ; il
ne crée pas une configuration cachée modifiable sans Admin. IA3c.2f-03 à -06,
F01–F07, source courante et admission restent ouverts. Bootstrap réel attend le
principal exact et les preuves d’isolation ; leur absence ne bloque pas les fixtures.


### Replan IA3c.2f-02a — permissions des fixtures

Le lot normal-01 collecte 49 noms : 38 PASS, 11 FAIL, zéro SKIP ; dix ouvertures
de fixture sont refusées, plus le parent du lot de fichiers dangereux. Les
entrées sont stables. Première frontière : répertoire final non privé, avant
la lecture/écriture de configuration. go doc -src testing.T.TempDir montre un
enfant créé avec 0777 sous son parent privé ; une sonde Go isolée mesure 0775
avec l’umask courant. Ce n’est pas une preuve de défaut du contrôle produit.

Correction de précondition : préparer explicitement les répertoires de fixture
en 0700, comme l’installation devra le faire ; conserver les cas négatifs 0755
et parent ouvert. Pas de relâchement des contrôles, pas de retry inchangé ni
hausse de timeout. Original normal-01, source du test et sonde conservés.
Propriétaire Codex ; reprise : nouveaux inputs de fixture puis lot normal/race.


### Replan IA3c.2f-02a — trois contre-exemples du candidat

Après normal/race-02 (52 noms PASS), la relecture ajoute trois tests ciblés.
regressions-red-01 collecte et échoue réellement les trois, entrées stables :
(1) modifier le répertoire de 0700 vers 0770 après Open permet encore Apply ;
(2) Preview accepte une date année 10000 que JSON ne peut sérialiser, alors que
Apply refuse ensuite ; (3) passer l’umask à 0700 après Open permet de remplacer
la configuration par un fichier 0000, déclaré comme succès puis illisible.
Sources antérieures archivées et sorties originales conservées.

Causes : métadonnées du répertoire contrôlées seulement à l’ouverture ; erreur
de sérialisation ignorée dans le calcul de tête et preview moins strict que
l’écriture ; permissions effectives du fichier temporaire non revérifiées.
Corrections générales décidées : recontrôle du répertoire sous réservation et
avant persistance, calcul de tête fallible propagé, validation du candidat complet
pour preview/Apply, contrôle du fichier réellement créé avant toute écriture.
Pas d’adaptation à un ID ou mission, pas de changement de timeout ni contournement
de l’umask. Propriétaire Codex ; reprise : mêmes trois tests puis lot normal/race.

Ces découvertes I2f02a-01 à -03 sont de nouveaux défauts du candidat, pas des
failles publiques démontrées. Leur valeur est de préserver le contrat sous
changement d’environnement et d’empêcher un aperçu ou succès trompeur. État :
reproduits, corrections en cours ; effet à vérifier avec leurs cas conservés.


### Résultat IA3c.2f-02a — configuration souhaitée, raccord encore partiel

70 noms PASS normal/race-05, zéro FAIL/SKIP ; vet et compilation ciblés PASS,
8 entrées stables. Contrôle des workflows : PASS (12 skills et contextes moteur),
sans modifier ses sources. 19 entrées antérieures préservées ; quatre nouveaux
fichiers du paquet. Trois contre-exemples locaux du candidat sont conservés en
rouge puis verts après correction. La preuve de filesystem utilise le même UID
de fixture et une faute fsync injectée, pas un service isolé ni une panne matérielle.

Le [rapport](verification-identites-agents-20261016/ia3-installation-config-20261017/RAPPORT-IA3c2f02a.md)
lie sources, échecs originaux, régressions, améliorations I2f02a-01 à -07 et
limites. FR/EN opérationnels, guides d’installation et schéma de reprise sont
actualisés dans le même incrément. Une reprise exacte ne refait pas le rename :
elle revalide l’autorisation puis synchronise le répertoire. Annulation observée
avant persistance refuse ; annulation concurrente pendant persistance ne prouve
pas une absence de mutation. Le consommateur futur doit observer le résultat.

Le décodeur commun est disponible mais aucun endpoint/CLI/UX ne le consomme.
Le prochain contrat indépendant est le maintien durable du checkpoint hors
configuration, avec reprise après écriture de config visible mais témoin non
confirmé ; aucun max/retry/TTL ne doit remplacer ce diagnostic. Service, opérations
publiques et Admin restent ouverts. Le principal humain initial reste à désigner
pour l’activation, pas pour poursuivre les fixtures. F01–F07 et RETEX global ouverts.


### Replan IA3c.2f-02a — périmètre de correction des schémas

Le diff final révèle qu’un remplacement textuel générique de flèche, destiné
au nouveau schéma, a modifié trois graphes antérieurs dans chaque langue.
Cause : correction appliquée au document entier au lieu du nouveau bloc.
Original conservé dans documentation-before-graph-recovery.diff. Restaurer le
préfixe FR/EN exact depuis l’archive avant modification, conserver uniquement
le nouveau chapitre ; égalité des anciens chapitres vérifiée. Les nouveaux
rendus sont inchangés et restent conformes au bloc actuel. Aucun code touché,
aucun résultat Go élargi ou relancé sans raison. Amélioration I2f02a-08 :
périmètre des transformations documentaires vérifié par le diff et les sections
originales ; propriétaire Codex, correction vérifiée, apprentissage dans le rapport.


## IA3c.2f-02b — témoin durable et reprise coordonnée, 17 octobre

Replan/APEX, Codex seul. Besoin repris : un Head fourni en mémoire ne maintient
pas son actualité durable après redémarrage. IA3c.2f-02a laisse explicitement ce
contrat ouvert. Les huit entrées de sa preuve finale sont inchangées à la reprise.
Comparer : écrire config puis une simple tête séparée, ou préparer une intention
durable puis appliquer et confirmer. La première perd la décision exacte dans
les fenêtres de panne ; retenir la seconde, sans timer, TTL ni retry implicite.

Périmètre avant code : nouveau coordinateur/témoin et tests dans enrollmentconfig ;
extraction interne de lecture/persistance de fichier pour partager les contrôles
existants sans copier un autre Store ; fonctions publiques antérieures conservées.
Plan/cible, contrats FR/EN, guides installation et preuves actualisés dans le même
incrément. Aucune source moteur, UX, mémoire, processus ou mission modifiée.

Le témoin porte schéma/pins, Head confirmé et intention exacte facultative (entrée
et tête cible). Il vit dans un répertoire explicite distinct et non imbriqué,
vérifié avec les mêmes règles Linux ; même inode refusé. Un répertoire séparé ne
prouve pas une indépendance de compte, sauvegarde ou hôte. Restaurer les deux
stockages et leurs protections ensemble reste une limite à annoncer.

Contrat : Coordinator ouvre les deux réservations non bloquantes ; bootstrap
explicite seulement si config et témoin absents. Jamais de témoin appris d’une
config existante. Intention durable avant config ; config et barrière de durabilité
avant confirmation ; aucune View exposée avec une intention ouverte. Read/Preview
ordinaires bloquent en attente de reprise. Recover ou retry exact de la demande
pendante revalide l’autorité et compare les octets/empreintes attendus : base
exacte ou cible exacte seulement, aucune acceptation d’une troisième histoire.
Une configuration manquante hors genèse est refusée, pas reconstruite en silence.

Une erreur d’écriture peut laisser un fichier visible : conserver l’intention et
la cause, retourner un résultat nul. Une confirmation visible mais non acquittée
se reprend par les barrières et le retry courant exact sans seconde mutation.
Les callbacks restent des fixtures ; aucune authentification OIDC ni protection
d’un fournisseur réel n’en découle. API Go interne seulement, pas de raccourci
CLI/HTTP ou activation sans Admin.

Critères : codec strict du témoin, init/read/reopen, rollback du seul config,
absence/mauvais pins du témoin, répertoires identiques/imbriqués/alias, chacune
des fenêtres avant/après config et confirmation, refus d’autorité à la reprise,
conflit/tamper/pending, annulation, concurrence et retry sans doublon. Tests avant
code du nouveau contrat ; archives et sorties originales conservées. Normal/race
du paquet y compris les 70 noms précédents, vet/build ; -timeout=0. Les opérations
protégées publiques et le déploiement restent ouverts après ces seules fixtures.


### Replan IA3c.2f-02b — contexte d’erreur de la coordination

Après 91 puis 99 noms PASS normal/race, un cas ciblé injecte une faute E/S
générique lors de la préparation du témoin. stage-red-01 exécute un test et
l’échoue : la cause et ErrIndeterminate sont préservés, mais aucune indication
« prepare witness » ne distingue l’étape de reprise. Le source et l’original sont
conservés avant correction. Défaut de diagnostic du candidat, pas fausse preuve
de mutation ni faille publique démontrée.

Correction générale : enrichir les frontières préparation du témoin, écriture
configuration, confirmation et barrières avec leur rôle, sans chemins privés ni
écrasement de cause via errors.Is. Propriétaire Codex ; reprise : garde ciblée
et ensemble normal/race. I2f02b-01 conserve origine, valeur de diagnostic,
périmètre et preuve ; aucun retry inchangé ou nouveau plafond.


### Preuve complémentaire justifiée — sortie de processus

Le lot 03 vérifie 100 noms en normal/race. Pour distinguer une reprise durable
d’une reprise du même objet Go, ajouter une fixture enfant du binaire de test :
intention écrite, barrière en faute, sortie os.Exit sans Close, puis reconstruction
par un nouveau coordinateur depuis les fichiers. Aucun autre processus n’est
interrompu. Ce cas prouve réception de l’état après sortie de processus, pas
panne matérielle, redémarrage machine ou isolation d’un compte fournisseur.
Watchdogs parent/enfant explicitement désactivés ; suivre la progression.


### Transmission au dossier scientifique

Actualiser également la note identités-autorite-cycle-vie du dossier scientifique,
restée à IA3c.2e : choix local pilote et distant prévu, noyau de configuration
02a puis témoin coordonné 02b. Capturer la note avant append, préserver le reste.
Les 101 noms incluent les 70 précédents ; aucun cumul trompeur d’attaques.
Déploiement, séparation UID, panne matérielle et supériorité comparative non prouvés.


### Résultat IA3c.2f-02b — témoin local reçu, protection encore partielle

101 noms PASS normal/race-04, zéro FAIL/SKIP ; vet/build ciblés PASS, douze
entrées stables, dont les 70 gardes antérieures rejouées. Six des huit sources
de référence sont inchangées ; deux modifications prévues (commentaire de paquet,
extraction privée I/O), quatre nouveaux fichiers du coordinateur et tests.
Contrôle de configuration des workflows PASS, sans modifier ses sources.

Le [rapport](verification-identites-agents-20261016/ia3-config-witness-20261017/RAPPORT-IA3c2f02b.md)
conserve le défaut de contexte d’erreur FAIL avant/PASS après, les fenêtres de
faute et la sortie de processus enfant sans Close. Contrats et schémas FR/EN,
guides d’installation et note scientifique identité actualisés ; anciens blocs
opérationnels préservés. Pas de binaire déployé, d’authentification humaine réelle
ou de preuve indépendante. Les rendus sont inspectés, pas une validation UX.

Le maintien du témoin est réalisé localement ; protéger les comptes/sauvegardes
et prouver l’usage final restent ouverts. Restaurer les deux fichiers ensemble
retire la détection : EP-06 et F07 non clos. Aucune annulation automatique d’une
intention en cas de refus ; conserver propriétaire et condition de reprise.
Le prochain raccord est le service local protégé, puis ses opérations publiques
et la configuration installation/Admin ; aucun raccourci depuis un projet ou
le cookie cockpit ne doit fournir l’autorité. F01–F07, IDA, IA3–IA5 et RETEX ouverts.

## IA3c.2f-03a — adaptateur local de contrôle, reprise du 17 octobre

Reprise avec `replan` et APEX : le coordinateur est vérifié, aucun service protégé
n’est actif. Comparer un nouveau transport par UID de connexion et la réutilisation
du canal Linux IA2 par message/pidfd : retenir ce dernier, sans copier son code ni
prendre un UID pour une autorité. Codex seul possède deux nouveaux fichiers
agent_identity_config_service_linux.go et son test dans internal/engine ; aucun
fichier mémoire ou lanceur public modifié. Mettre à jour cible, contrats FR/EN,
installation et note scientifique dans le même incrément. Originaux archivés dans
verification-identites-agents-20261016/ia3-local-service-20261017.

Livrer un adaptateur interne pour quatre opérations explicites : read, preview,
create et apply. Apply reprend aussi une intention pendante exacte ; pas de commande
de réparation libre. Requête canonique schéma 1, sans acteur ou date fournis par
le client. Le canal est configuré avant libération de l’émetteur. Le contrôleur
d’autorité de confiance doit maintenir son contrôle pendant toute l’opération,
sur le message attribué et la demande exacte ; pas d’inférence cockpit/Admin/UID.
Le principal doit correspondre aux pins indépendants. Lire exige aussi ce contrôle.
Le coordinateur revalide la demande exacte sous ses réservations. Cette interface
ne constitue pas une implémentation OIDC ou une preuve du confinement réel.

Critères avant code : format strict et refus sans mutation, quatre parcours utiles
avec résultat inspecté, preview sans écriture, CAS et retry, refus de principal,
autorisation absente/non exécutée/erreur aval/second appel, métadonnées tronquées
ou injectées, réponse perdue après mutation et reprise sans doublon, échange entre
processus attribué à l’émetteur réel. Erreurs réseau publiques sans chemins privés,
causes originales côté service. Buffer transport explicitement fourni, aucun
plafond métier, délai ou retry ajouté. Watchdogs Go/enfant désactivés.

Le raccord autonome commande/service, compte système, principal humain initial,
configuration Admin, authentification du service par un client public et contrôle
du fournisseur restent des dépendances ouvertes. Ne pas activer l’adaptateur dans
les opérations publiques avant ces preuves ; EP-01/04/05/07 et F01–F07 non clos.


### Ajustement de preuve IA3c.2f-03a

Le premier candidat passe 40 noms moteur et 101 du coordinateur en normal/race,
vet/build. Ajouter la variante enfant même UID refusé, la fenêtre pending exacte
par apply et annulation/absence de config ; seul le test moteur change. Rejouer
ce sélecteur, conserver les 101 noms config tant que ses entrées restent identiques.
Étendre le manifeste à tous les Go internes/CLI pour couvrir les imports transitifs.
L’archivage du binaire de fixture par rename vers /tmp échoue EXDEV (volumes
distincts), sans toucher le source ou un service. Correction de méthode : move
avec copie inter-volumes puis vérification de hash et d’absence dans le dépôt.
Aucun résultat de test perdu, aucun relancement du même déplacement inchangé.


### Replan lié I2f03a-04 — watchdog hérité du contrôleur de méthode

Le contrôle des workflows passe, mais l’inspection de son lanceur constate un
-timeout 30s global sans contrat temporel justifié. Cause : recette héritée, pas
lenteur ou blocage du code mesuré. Comparer laisser la borne, ajouter une valeur
configurable et la désactiver : choisir -timeout 0, conforme à la consigne déjà
adoptée, sans créer une nouvelle politique produit. L’original check.py est
archivé avant correction ; il s’agit d’un changement de méthode, pas d’une panne
reproduite après 30 secondes. Codex prend aussi ce fichier ; aucun autre fichier
de méthode ou mémoire modifié. Relancer le contrôleur et conserver l’exécution
JSON nommée du test de contextes. Les 44 noms moteur et 101 config ont leurs
preuves distinctes ; ils ne dépendent pas de ce délai Python. Propriétaire Codex,
critère : commande sans watchdog global et test non vide effectivement PASS.


### Replan I2f03a-05 — compilation dans le dépôt partagé

normal-engine-02 exécute 44 noms PASS ; race-engine-02 aussi mais le manifeste
rejette la preuve : work_row.go et work_row_test.go ont changé entre entrée et
sortie. Aucun de nos deux nouveaux fichiers ne change ; conserver stdout et
AssertionError, ne pas annoncer cette course comme preuve liée au candidat final.
Cause établie : mutation de deux sources mémoire du même paquet pendant la
compilation ; pas une course Go observée ni un échec des assertions identité.

Comparer attendre puis relancer en espérant la stabilité et fixer explicitement
ces deux entrées via Go overlay. Retenir un overlay de deux fichiers dans le
même dossier de preuve, sans copie du dépôt ni modification du chantier mémoire.
Conserver les deux textes, leurs hashes et la table d’overlay ; le manifeste
porte les sources effectivement compilées et refuse toute autre mutation.
Rejouer normal/race moteur, vet/build avec ce même overlay. Les 101 gardes config
et leurs entrées propres inchangées restent réutilisables. Cette preuve décrit
le candidat avec cet instantané mémoire ; vérifier l’écart au dépôt lors du rapport
final et ne pas déclarer un serveur courant reconstruit/déployé. Propriétaire
Codex pour stabilité de la vérification, chantier mémoire pour ses deux sources.


### Replan lié I2f03a-06 — faux succès d’un sélecteur vide

L’inspection du même contrôleur montre qu’il n’exige que le code 0 de Go.
Reproducer indépendant du contenu des rôles : exécuter son vrai main avec une
sortie externe contrôlée « no tests to run » et exit 0. collection-red-01 échoue
sur un test : le main imprimait OK. Cause au contrat de preuve, pas au moteur
ni au test de contexte effectivement exécuté dans les lots précédents.

Corriger check.py au même endroit : demander JSON, exiger run et pass du test
nommé ainsi que la fin du paquet, refuser skip/fail et format invalide. Afficher
le compte observé ; préserver le code et le texte d’un échec Go. Nouveau fichier
possédé tools/agent-workflows/test_check.py : quatre tests dont variantes vide,
mauvais nom, skip, incomplet, sous-test échoué, JSON mal formé et mauvais type.
Vérifier mocks puis le vrai contrôleur et ses cinq noms. Les mocks ne prouvent
pas l’assemblage des contextes, le contrôle réel le vérifie séparément. Garder
le FAIL original, puis revenir au raccord identité. Le même propriétaire reste
Codex ; aucun besoin de fournisseur ou nouvelle politique d’exécution.


### Capitalisation méthode et lecture des preuves

Actualiser aussi docs/AGENT-METHODS.md et sa version EN : contrôle nommé, refus
sans test et recette Go ciblée avec watchdog désactivé. Les trois sélecteurs de
la recette sont vérifiés dans internal/engine ; cette édition documentaire ne
modifie pas leurs sources et n’en invente pas une nouvelle exécution. Conserver
les versions de ces deux pages juste avant édition, sans toucher leurs autres
travaux. Un lecteur auxiliaire de manifestes a rencontré KeyError counts en
mélangeant résultats Go et résultat Python hétérogènes ; lecture corrigée par
sélection explicite des quatre manifestes Go et schéma Python séparé. Aucun
résultat manquant transformé en zéro ou PASS ; quatre résultats Go relus avec
succès et comparaison explicite des deux sources d’overlay au dépôt courant.


### Résultat IA3c.2f-03a — raccord privé vérifié, activation ouverte

44 noms moteur PASS normal/race-03, zéro FAIL/SKIP, vet/build PASS avec 830 entrées
effectives stables et l’overlay borné à deux fichiers mémoire. Leur version
courante correspond à l’overlay au contrôle de fin ; conserver cette comparaison,
qui ne prédit pas une modification ultérieure. Les 101 gardes config normal/race-01
sont réutilisées sur douze entrées propres identiques. La course du lot 02 reste
une preuve rejetée, pas rebaptisée verte. Aucun autre fichier moteur modifié.

Contrôleur de méthode : quatre tests Python PASS (variantes incluses) et contrôle
réel à cinq noms Go PASS, avec watchdog désactivé ; FAIL original de sélecteur vide
conservé avant correction. Les versions FR/EN des contrats, installation et
méthodes, schémas et note scientifique sont actualisées. Les rendus sont inspectés,
pas un parcours UX. Prochain contexte : lire le [rapport de raccord](verification-identites-agents-20261016/ia3-local-service-20261017/RAPPORT-IA3c2f03a.md),
ses diffs et checks-final avant intégration ; les processus déjà lancés n’ont
pas reçu ces sources et ne sont pas annoncés corrigés.

Le contrôleur réel, l’établissement du pair service côté client, le point d’entrée
système et la protection compte/confinement restent des préconditions d’activation,
avec le principal humain initial explicitement désigné. Les callbacks ici ne les
remplacent pas. I2f03a-01 à -06 conservent valeur, origine, propriétaire et reprise
dans le rapport ; la taille effective d’une réponse EMSGSIZE reste à mesurer lors
du client public. F01–F07, EP-01/04/05/07, IDA, IA3–IA5 et RETEX restent ouverts.


## IA3c.2f-03b — client local et liaison demande/réponse

Reprise replan/APEX : 03a vérifie le service privé, pas son client. Comparer un
client qui croit la connexion/UID et un client qui compare le pidfd du message à
un processus serveur épinglé indépendamment. Retenir le second : inode pidfs sur
Linux 64 bits, vérification de type de FS et descripteur détenu ; anciens noyaux
ou 32 bits refusés sans repli PID numérique. Le contrat est vérifié dans le code
primaire Linux v6.17 fs/pidfs.c et dans des fixtures ; il n’authentifie ni le code
exécuté ni un humain et reste limité au même démarrage hôte. Les premières URL
kernel.googlesource/freedesktop inaccessibles sont remplacées par le miroir
primaire GitHub versionné, sans considérer une absence de source comme preuve.

Évolution de protocole explicite version 2 avant activation : requête avec pins
publics et challenge aléatoire 32 octets ; réponse liée par empreinte domaine 2
aux octets exacts de cette demande. Refuser la version 1, réponses d’autres
opérations, racines, événements ou bases ; valider View/Projection par le contrat
commun, pas par un booléen/HTTP 200. Les pins du corps ne confèrent aucun droit.

Codex possède deux nouveaux fichiers agent_identity_config_client_linux.go et
son test ; modifie les deux fichiers du service 03a et config.go/config_test.go
dans enrollmentconfig. Aucun lanceur, méthode, mémoire ou service en cours modifié.
Docs FR/EN, schémas, installation, cible et note scientifique dans cet incrément.
Preuves au même niveau que 03a, dans ia3-local-client-20261017.

Client à usage unique, contrôle contexte avant envoi ; fermeture de sa connexion
pour interrompre une attente annulée, sans délai arbitraire. Après tentative de
mutation et sans réponse authentifiée valide, issue incertaine et résultat nul ;
retry métier exact sur nouveau canal, pas de répétition de transport automatique.
Mesurer la limite réelle du socket via SO_SNDBUF en fixture et provoquer EMSGSIZE,
puis vérifier état sauvé et retry sans doublon. Ne pas introduire un plafond métier.

Tests avant code du nouveau contrat : validation partagée des têtes/projections,
format et downgrade, substitution de pins/demande/réponse, serveur exact/rogue
même UID, paire de processus réelle, annulation avant/après envoi, canal réutilisé,
troncature et réponse trop grande, fermeture et non-fuite des descripteurs.
Rejouer les anciens tests service/canal et les 101 gardes config car le contrat
commun change ; normal/race, vet/build ciblés, watchdogs désactivés. Overlay borné
aux deux fichiers mémoire déjà identifiés, entrées effectives vérifiées et état
courant comparé. L’autorité humaine réelle, service système et Admin restent
ouverts ; aucune preuve d’isolement de comptes/fournisseur n’est déduite.


### Reprise du lanceur 03b avant compilation

Le script de préparation de preuve contient une parenthèse excédentaire et
échoue SyntaxError avant écriture ; l’appel suivant échoue faute de run_checks.py.
Aucun test exécuté, aucun résultat PASS. Corriger la syntaxe, compiler le texte
Python avant sauvegarde, puis vérifier le code de sortie et l’existence du
lanceur avant tout appel dépendant. Cause de notre orchestration, pas du client
Go ; les originaux restent dans les sorties d’outils, et la reprise modifie
explicitement la précondition. Pas de relance d’un Go inchangé après timeout.


### Replan IA3c.2f-03b — test qui gérait la fin d’échange à la place du serveur

Le lot 01 passe 69 noms moteur et 106 config en normal/race, vet/build stables.
Le cas EMSGSIZE réduit SO_SNDBUF en fixture : valeur observée 8192 octets, raison
16384 octets, effet confirmé puis retry sans doublon. Mais son goroutine ferme
explicitement le socket après serve : l’adaptateur lui-même ne le ferme pas.
Cela peut laisser le client attendre après une réponse non envoyée, sans timeout
qui diagnostiquerait la cause. Nouvelle garde non bloquante : après le vrai échec
EMSGSIZE, constater POLLHUP via poll(0), sans ajouter de délai de test.

Second diagnostic du candidat : le code public unavailable était converti en
ErrStorage, alors que cette indisponibilité peut provenir de l’autorité ou d’un
autre contrat. Garde ciblée : refuser cette cause inventée. Deux contre-exemples
sont lancés avant toute correction ; ne pas confondre compiler et reproduire.

Options : laisser chaque appelant fermer le socket, ajouter un watchdog, ou
rendre le canal serveur à usage unique et possédé jusqu’à fermeture. Retenir la
troisième, cohérente avec le client : une opération, fermeture sur succès/refus/
erreur, réemploi refusé, causes préservées. Le contrôleur réel conserve ses
obligations d’autorité ; pas de timeout ni retry de transport pour masquer l’échec.
Unavailable devient une erreur générale explicite sans prétendre une faute disque.
Conserver les sources avant correction et le FAIL comportemental ; rejouer les
cas moteur, pas les 106 config tant que leurs douze entrées restent identiques.
Ajouter stale challenge, pins avant autorité et propriété des descripteurs pour
vérifier la frontière nouvelle, puis reprendre le besoin initial de service.


### Deux contre-exemples 03b établis, puis correction

closure-red-01 exécute deux tests, deux FAIL, sources stables : socket encore
ouvert après EMSGSIZE et cause disque inventée pour unavailable. Les sources
originales sont archivées. Correction générale : canal serveur consommé une fois,
fermé par defer sur toute issue ; le test de taille ne ferme plus à sa place.
Unavailable conserve une erreur de service générale. Rejeu final moteur avec
stale challenge, pins avant autorité, double usage serveur, propriété/fermeture
et CLOEXEC des descripteurs ; cas v1, version inconnue et challenge invalide
explicitement distingués. Réutiliser les 106 config normal/race-01 : leurs douze
entrées propres sont inchangées, comparaison enregistrée.


### Précondition de remise du canal conservée

Le client attribue le retour après envoi ; ne pas transformer cette preuve en
confidentialité ou en preuve du destinataire avant envoi. Le canal privé doit
venir d’un lanceur de confiance, lié au pidfd attendu et non divulgué. Comparer
remise privée vérifiée et établissement authentifié avant contenu : retenir la
remise privée pour le raccord local prévu, vérifier confinement et descripteurs
avant activation ; une connexion par chemin seul exige encore cet établissement.
Aucune donnée secrète ou session humaine dans ce protocole de fixture. Conserver
cette condition EP-07, propriétaire Codex pour lanceur/protocole et opérateur
pour compte/confinement ; ne pas annoncer une authentification mutuelle publique.


### Résultat IA3c.2f-03b — retour attribué et demande liée en fixtures

77 noms moteur PASS normal/race-02, zéro FAIL/SKIP, vet/build PASS avec 832 entrées
effectives stables. 106 noms config normal/race-01 incluent les 101 précédents ;
leurs douze entrées propres inchangées permettent la réutilisation après correction
moteur seule. Deux défauts du candidat ont leurs FAIL avant/PASS après, sans
attendre un timeout : fermeture après EMSGSIZE et cause disque inventée.
Le contrôleur commun des méthodes passe avec cinq tests/sous-tests et les mêmes
entrées d’overlay ; aucun fichier de méthode modifié dans cet incrément.

Le protocole transport devient explicitement version 2 ; configuration/journal
restent en version 1 et préservent leur historique. Deux ajouts et quatre
modifications de source annoncés, 34 autres entrées de référence inchangées.
Contrats/schémas FR/EN, installation, cible et note scientifique actualisés.
Les schémas rendus sont inspectés, pas une validation UX ou un déploiement.
Lire le [rapport](verification-identites-agents-20261016/ia3-local-client-20261017/RAPPORT-IA3c2f03b.md),
ses sources/diffs et checks-final avant intégration ; les processus en cours ne
reçoivent pas automatiquement ces changements. I2f03a-03 est maintenant vérifié
pour EMSGSIZE/retry ; le transfert d’historique arbitraire reste ouvert (I2f03b-04).
Le retour authentifié après envoi ne prouve pas la remise privée avant envoi :
conserver I2f03b-05 et EP-07 avant toute activation publique. Service système,
humain initial, confinement, Admin et admission/révocation réelles restent ouverts.


## IA3c.2f-03c — remise privée du canal et cycle du processus

Reprise replan/APEX : checkout propre sur codex/clear-product-method-names,
HEAD f32b9f8fa6e99f75845f57872a7f8a56a0ef9dc6 ; les tranches précédentes ont
été intégrées depuis les preuves conservées. 03b vérifie le retour après envoi,
mais I2f03b-05 attend encore la remise au serveur avant demande métier.
Comparer endpoint par chemin, nouvelle couche réseau et remise privée des FDs
avec preuve séparée de démarrage : retenir la dernière pour la cible locale.
Aucune création de compte ni déploiement, aucun fournisseur ou processus tiers.

Deux ajouts possédés par Codex : agent_identity_config_launch_linux.go et son test,
dans internal/engine ; anciennes sources inchangées. Réutiliser le pinning de
l’image courante (trust_fixed_image_linux.go), le canal IA2 et le client/service
03b. Nouvelle commande interne de lancement Go seulement ; pas de nouvelle
branche CLI ou service système sans source humaine indépendante. Le parent
crée deux socketpairs privées configurées avant exec : métier et démarrage.
Trois phases démarrage version 1, purpose explicite hello/ready/release, challenge
32 octets et digest de pins publics ; aucune demande, principal ou session dans
ce canal. Ready et Release ne donnent aucune autorité métier.

L’image courante est épinglée par FD. Pidfd enfant obtenu atomiquement par
SysProcAttr.PidFD / CLONE_PIDFD, inspecté dans le Go 1.24.3 local ; refus si absent,
jamais PidfdOpen sur un numéro de PID enfant. Pidfd du parent transmis indépendamment
au helper ; comparaison noyau/UID des deux côtés avant libération du canal métier.
Le child ne lit la demande métier qu’après Release exacte, puis applique toujours
son contrôle d’autorité et la coordination protégée. Les FDs reçus sont adoptés,
fermés ou CLOEXEC avant service ; les copies parent sont fermées après exec.

Le lanceur consomme une commande neuve de l’image courante, sans ExtraFiles ni
annulation/WaitDelay préconfigurés ; environnement explicite, attributs système
préservés sauf sa réservation PidFD. Pas de shell, de chemin choisi par projet,
de délai, retry, compte implicite ou relance. Annulation/échec ferme ses canaux,
signale uniquement son enfant via pidfd et recueille Wait ; jamais un autre
processus ou groupe. Succès exige réponse métier valide et fin de l’enfant à zéro.
Une panne après mutation reste une issue incertaine avec résultat nul, sans reset.

Critères avant code : échange/effet persistant entre deux processus, auth encore
requise après ready, pins/UID/nonce/purpose incorrects refusés avant effet, erreur
exec/fin précoce, annulation avant exec et après entrée dans l’autorité via barrière
observée, enfant non nul après réponse, pas de secret d’environnement hérité,
FDs adoptés/fermés et doubles démarrages refusés. Pas de timeout global Go/enfant.
Rejouer les 77 noms moteur et les nouveaux tests process, race ciblée, vet/build ;
réutiliser les 106 config uniquement sur douze entrées propres identiques.
Manifestes de toutes les sources Go/modules, pas d’overlay préventif : le checkout
est propre ; toute mutation pendant compilation invalide la preuve et impose
son propre diagnostic. Docs FR/EN, schémas, installation, cible et note scientifique
au même incrément. Preuves dans ia3-local-launch-20261017.

### Résultat IA3c.2f-03c — canal remis avant contenu, processus recueilli

Lanceur privé et helper Go ajoutés, sans modification des 41 sources/modules de
référence. Démarrage V1 distinct, image épinglée et pidfd enfant obtenu au clone ;
aucun contenu métier envoyé avant ready exact du processus attendu. Le helper
vérifie parent et release avant d’admettre le canal. Métier V2 et stockage V1
inchangés. Annulation par fermeture et signal au seul pidfd enfant ; Wait recueilli,
copies héritées fermées et numéro du pidfd consommé invalidé dans Cmd.

117 noms moteur PASS normal/race-02 (77 précédents + 40 nouveaux), zéro FAIL/SKIP,
vet/build PASS, 834 entrées stables, sans overlay. Les 106 noms config
normal/race-01 de 03b restent réutilisables : douze entrées propres comparées,
aucune différence. Aucun échec comportemental observé dans le candidat 03c ; ne
pas inventer une phase rouge pour un nouveau contrat. La première version a
39 noms nouveaux PASS normal/race-01 ; la finale ajoute le refus d’un autre objet
processus sous le même UID et contrôle fermeture/invalidation des FDs. Les
sources des deux versions sont archivées. Pas de répétition de suite globale.

Un dernier contrôle ajoute dans le helper l’observation des originaux hérités
3–6 avant autorité : fermés ou, si numéro réutilisé par Go, remplaçant CLOEXEC.
Le lot final 03 passe avec les mêmes 117 noms normal/race, vet/build et 834 entrées
stables. Cette modification du test justifie le rejeu ciblé ; le code produit
reste identique au lot 02. Les deux résultats et leurs archives sont préservés.

Docs FR/EN et installation, cible et note scientifique actualisées. Les schémas
FR/EN sont rendus et inspectés. Le rapport conserve recette, manifeste et limites :
[IA3c.2f-03c](verification-identites-agents-20261016/ia3-local-launch-20261017/RAPPORT-IA3c2f03c.md).
Un répertoire docs/diagrams apparu pendant ce travail est extérieur au périmètre
annoncé ; il est préservé. Aucun processus tiers, compte, mission ou binaire déployé
modifié ; aucune revue indépendante ou acceptation globale revendiquée.

| Découverte | Origine, valeur et périmètre | État, propriétaire et reprise observable |
| --- | --- | --- |
| I2f03c-01 / I2f03b-05 | Vérifier le retour après envoi ne prouve pas la remise avant contenu ; protéger le raccord local privé | Réalisé/vérifié en fixtures par Codex ; EP-07 reste partiel tant que compte, confinement et bootstrap réel ne protègent pas les FDs |
| I2f03c-02 | Une réponse valide suivie d’un échec de processus ne remplit pas le contrat de lancement ; conserver effet durable et incertitude | Réalisé/vérifié : erreur originale, zéro résultat, reprise exacte sans doublon ; Codex ; reprendre cette garde au raccord public |
| I2f03c-03 | Cmd peut conserver le numéro d’un pidfd fermé ; éviter une réutilisation ultérieure accidentelle | Invalidation à -1 réalisée/vérifiée, copies héritées fermées ; Codex ; aucune garantie déduite pour des FDs divulgués par un bootstrap compromis |
| I2f03c-04 | Wait collecte aussi les goroutines de flux Go ; un writer arbitraire peut bloquer malgré la fin du processus | Limite documentée, non simulée comme résultat vérifié ; Codex pour le raccord public : définir les propriétaires/contrats de flux et observer leur collecte avant activation, sans timeout de commodité |

Pour I2f03c-04, comparer fichiers OS explicites et collecteur de copie possédé,
annulable et vérifié : retenir les fichiers OS pour le futur helper public à
usage unique, collecteur seulement si un besoin de capture l’exige. Choix conservé
pour ce raccord encore inexistant ; ne pas annoncer l’effet comme déjà réalisé.

Réexamen du besoin initial : ce composant ne désigne pas l’humain ni n’installe
l’autorité. Le principal humain initial, sa source vérifiable et la protection du
bootstrap relèvent encore d’une décision opérateur ; la création/activation d’un
compte et service système exige cette décision et son autorisation. Le code peut
ensuite recevoir pins/UID depuis cette source, vérifier confinement et appliquer
les parcours publics Go/API/CLI/Admin. Ne pas utiliser un cookie cockpit, le compte
actuel ou le premier appelant pour contourner la précondition. F01–F07, IDA,
EP-01/04/05/07, IA3–IA5 et RETEX global demeurent ouverts.

Précision I2f03c-01/EP-07 : le helper à usage unique ne résout pas le cycle du
service permanent ni les droits de changement de compte/signal à un autre UID.
Le pidfd n’accorde pas ces droits. Comparer bootstrap privilégié protégé et broker
authentifié avant le raccord public, à partir de la source d’autorité désignée ;
ne pas rendre tout le cockpit privilégié. Propriétaires Codex/opérateur ; critère
observable : lancement/supervision du compte séparé avec droits réellement
vérifiés dans le parcours retenu. État : contrainte conservée, intégration non faite.

Un patch documentaire groupé a été refusé sur un contexte de ligne erroné, sans
modification partielle (hashes comparés). Le contexte exact a été relu avant le
patch corrigé ; preuve originale dans doc-patch-incident.json du lot 03c. La
reprise ne change aucun source Go ; seuls les contrôles documentaires et leurs
archives sont reconstruits, sans répétition des tests moteur inchangés.

À la vérification finale, README.md/README.en.md et le suivi/preuves
deepagents-integration ont aussi évolué hors du périmètre 03c. Ils sont préservés,
exclus du diff et des archives possédés ; aucun source Go pertinent n’a changé.
L’auto-inspection corrige aussi une phrase FR : le pidfd enfant est reçu par le
parent, comme le montrent le code, le schéma et le texte EN. Source Go inchangé.

## IA3c.2f-03d — précontrôle public du bootstrap protégé

Reprise autorisée par « donc on fais ca ». Le dépôt a évolué sur les chantiers
mémoire/UX depuis 03c ; ses preuves restent historiques, les ajouts lanceur sont
inchangés. Préserver toutes ces modifications. Le principal humain initial et sa
source n’ont pas été fournis ; question précise sur le fournisseur existant
envoyée à l’utilisateur. Ne pas inventer issuer/subject, clé ou identité humaine.

Comparer activation directe depuis le cockpit et précontrôle indépendant de
l’installation : retenir le précontrôle en lecture seule. Il prépare une entrée
réelle pour setup/service sans donner à un fichier le pouvoir de s’autoriser.
Manifeste canonique version 1 : pins publics, UID service et contrôleur distincts,
répertoires configuration/témoin. Empreinte attendue transmise indépendamment ;
elle ne doit jamais être apprise du fichier qu’elle contrôle. Le chargement est
exécuté sous l’UID du service, refuse symlinks/liens multiples/modes non privés,
vérifie les trois répertoires distincts et non imbriqués sans créer de lock.
Une validation de ce format ne prouve pas l’authentification humaine ou la
protection du lanceur ; les états non vérifiés restent explicites.

Fichiers possédés annoncés : internal/enrollmentconfig/store_linux.go (extraction
du parcours de chemin existant) et nouveau path_linux.go ; paquet nouveau
internal/enrollmentbootstrap (format, chargeur et tests) ; internal/engine/main.go
(routage avant ouverture projet), nouveaux agent_identity_bootstrap_linux.go et
son test. Commande publique : swarm identity bootstrap-check EMPREINTE --input
/chemin/absolu/bootstrap.json. Aucun service, compte, clé ou configuration métier
créé par cette commande. Bibliothèque commune réutilisable par setup/API/Admin.
Docs FR/EN, installation, cible et note scientifique dans le même incrément.

Critères : contre-exemples fichier/canon/digest/UID/alias, résultat nul sur refus,
zéro fichier créé, ancien contrôle de chemin inchangé, CLI réelle sans projet ni
authentification promue. Tests avant code ; Go -timeout=0, suivi de progression,
race sur stockage/chargeur et lanceur, contrôle méthodes et compilation. Le
rejeu du paquet config est justifié par l’extraction de sa frontière commune.
Capturer les sources effectives et changements simultanés ; une mutation invalide
la preuve, ne pas la masquer. Preuves dans ia3-bootstrap-check-20261017.

Anomalie de méthode en lecture : noms supposés main_test.go, trust_store.go et
agent_identity_issuer.go absents. Recherche corrigée avec rg --files avant lecture
des producteurs ; aucun défaut produit ou test PASS déduit de ces commandes.
Conserver les sorties originales de ces refus dans le rapport de tranche.

### Résultat 03d — précontrôle public vérifié, source humaine en attente

141 noms bootstrap/config (35 + 106) et 118 moteur (117 précédents + CLI) passent
en normal et race-04, zéro FAIL/SKIP ; vet/build PASS. 848 entrées Go/modules
stables pendant ces lots, sans overlay. La frontière de chemins du stockage est
extraite sans assouplissement, les 106 gardes config sont rejouées à cause de
cette dépendance commune. Les deux sources Go existantes modifiées et cinq
ajouts sont archivés ; les travaux mémoire/UX sont conservés.

Le binaire de fixture exécute la vraie CLI : contrôle valide, mauvais pin refusé,
témoin valide dans des namespaces privés puis alias réel config/témoin par bind
mount refusé. Aucun projet, lock, état métier, compte ou service créé. UID 0 dans
ce namespace est l’utilisateur hôte non privilégié mappé, pas un accès root hôte.
Les trois états humain/confinement/service restent false. Le contrôleur commun
des méthodes passe avec cinq noms ; schémas FR/EN rendus et inspectés.

Contre-exemples de méthode conservés : 02 compte un FAIL et 139 PASS, donc lot
FAIL ; le diagnostic 03 confirme le parent temporaire 0775 interdit. Nos fixtures
manquaient Chmod(0700), comme le pratiquent déjà les fixtures de stockage.
Correction des fixtures seulement ; le produit n’est pas affaibli. Chaque cas
négatif de chargement commence désormais par un Load valide de référence,
évitant que l’échec d’un parent masque toutes les autres injections. Le test CLI
s’exécute depuis un répertoire isolé. Le collecteur 01 a échoué avant tout Go
(variable name écrasée par la boucle d’archivage) : script original conservé,
variable distincte et label neuf ; ne pas compter ce lancement comme test exécuté.

| Découverte | Origine, valeur et périmètre | État / propriétaire / critère restant |
| --- | --- | --- |
| I2f03d-01 | Ouvrir le coordinateur pour un simple précontrôle créerait un lock ; séparer observation et mutation | Helper de chemin commun et CLI sans Store réalisés/vérifiés ; Codex ; conserver le même contrat pour setup/API/Admin |
| I2f03d-02 | Fixture parent 0775 : des cas négatifs PASS peuvent partager une précondition invalide et ne pas tester leur défaut propre | Reproduit, corrigé et vérifié avec témoin valide avant chaque injection ; Codex ; capitalisé dans contrat, tests, rapport et note scientifique |
| I2f03d-03 | Lire tout le fichier avant comparaison d’empreinte peut allouer un gros fichier non approuvé ; origine : inspection statique de Load | Hypothèse de durcissement avant raccord service, non mesurée ; Codex ; comparer empreinte en flux puis lecture liée aux mêmes octets, sans plafond métier caché, si un consommateur exposé doit lire des entrées non approuvées |

Coordination : demande de stabilisation reçue du chantier mémoire M32 pendant sa
qualification finale ; aucune réponse intersession envoyée, comme demandé.
Nos sept sources Go et les paquets communs sont stables après les lots 04 ;
plus aucune modification Go prévue pour terminer cette documentation. Toute
correction réellement nécessaire devra annoncer ses fichiers et un nouveau
point stable. La demande reçue n’est pas une preuve de qualification M32.

Le [rapport 03d](verification-identites-agents-20261016/ia3-bootstrap-check-20261017/RAPPORT-IA3c2f03d.md)
conserve recette et limites. Le précontrôle ne résout ni l’approbation indépendante
du manifeste, ni l’authentification du principal déclaré. La question précise
« fournisseur existant et URL d’émetteur, ou aucun » est en attente de réponse.
Cette valeur est nécessaire pour choisir et raccorder le parcours humain réel ;
ne pas inventer un issuer/subject ni promouvoir le compte/cookie courant. Après
cette désignation : source humaine et contrôle métier, provisionnement/service
séparé, preuve de confinement, admission/révocation publiques puis setup/Admin.
Pas de compte, clé ou service d’autorité créé pour contourner ce préalable.
F01–F07, EP-01/04/05/07, IA3–IA5 et RETEX global restent ouverts.

La qualification vérifie aussi deux contrôles d’aide existants : le routage
partagé main.go a changé et l’aide doit rester utilisable sans projet, sans
mutation de contexte métier. Ce sélecteur borné ne modifie aucun source Go.
Le chantier M32 confirme ensuite par message entrant avoir reçu le point public
de stabilité et lancé sa qualification sans substitution Go ; c’est un reçu de
coordination, pas sa fin ou son acceptation.

### Replan de preuve 03d — ressources réellement embarquées

La coordination M32 mentionne ses assets figés ; l’inspection de resources.go à
la racine et go list -json . montre 274 ressources go:embed, absentes du collecteur
limité à internal/cmd. Les 848 entrées de 04 ne suffisaient donc pas à lier le
binaire complet. Garder ces preuves et leur limite, ne pas les présenter comme
qualification de tous les inputs. Les sources Go sont toujours inchangées.

I2f03d-04, propriétaire Codex : collecteur corrigé pour inclure source racine,
ressources et évolution de la liste EmbedFiles. 137 assets web sont figés dans
une fixture readonly logique et l’overlay ne substitue aucun Go. Autres ressources
hashées et conservées/référencées pour reconstruction. Source primaire du Go local
vérifiée : embed.Files utilise fsys.Actual, donc applique l’overlay aux ressources.

Le lot final 05 passe : 141 noms bootstrap/config et 118 moteur en normal/race,
vet/build, CLI du nouveau binaire et alias de bind mount réels, contrôleur méthodes
cinq noms et deux gardes d’aide. 1 123 entrées effectives stables. Les empreintes
des sources Go coïncident avec 04 ; aucune modification du chantier identité ne
perturbe la qualification M32. Aucune acceptation UI ou service réel déduite.
La capitalisation dans une consigne embarquée commune attend sa fin confirmée,
afin de préserver cette entrée ; la recette corrigée et ce plan sont persistants
dès maintenant. L’activation reste suspendue au choix de source humaine demandé.

### Capitalisation après la clôture de la fenêtre M32

Un message entrant confirme la fin de qualification M32 et la stabilité des sept
sources Go communes. Sa fenêtre est close ; aucune preuve M32 n’est modifiée.
Les deux leçons sont maintenant conservées dans la consigne personnelle
/home/fpizzi/.codex/AGENTS.md, les méthodes FR/EN et le contrat commun embarqué.
Le contexte AGENTS actualisé reçu ensuite contient effectivement ces clauses.
Les processus déjà démarrés ne reçoivent pas cette mise à jour automatiquement.

Le premier ajout au contrat fait échouer le contrôleur : planner 32034 et
subplanner 32037 octets, contre la borne existante 32000. Original conservé dans
CONTRACT-before-frame-fix.md et workflow-learning-06.stderr. Aucun plafond relevé,
aucune troncature : reformulation du paragraphe de preuve, mêmes exigences et deux
leçons incluses, 63 octets de moins. workflow-learning-07 passe avec cinq noms.
Cette anomalie créée par notre amélioration est traitée avant le besoin initial.

La nouvelle donnée embarquée justifie la qualification du moteur et du binaire
avec ce contrat final : mêmes 118 noms identité, plus les gardes existantes de
framing, transmission au processus worker de fixture et réouverture, et aide.
Ce provider est un script local de test, aucun Claude ou modèle réel. Les 141
noms des bibliothèques config/bootstrap restent réutilisés de 05 après contrôle
de leurs dépendances locales effectives, qui n’importent pas resources. Pas de
suite config répétée pour une modification de texte hors de son graphe.

Lot final 07 : 126 noms moteur (118 identité, 5 framing, 1 transmission/réouverture,
2 aide) PASS normal/race, aucun FAIL/SKIP ; vet/build et CLI réelle/alias PASS.
141 noms de bibliothèque 05 réutilisés : graphe local et test vérifié, quinze
entrées inchangées, resources non importé. 1 123 entrées effectives liées au
binaire fixture-07. Le test worker vérifie le préfixe complet transmis au script
local et conservé après réouverture, sans fichier local hostile ni acceptation
implicite de tâche. Les deux leçons atteignent ainsi le consommateur de fixture,
pas seulement un fichier. Aucun modèle réel ou processus existant mis à jour.

I2f03d-05 : les corps de méthodes frôlent la borne moteur 32000 ; cette borne
exclut l’en-tête de métadonnées. Les cadres retournés mesurés sont 32196/32202
(planner/subplanner) et 31355/18794 (worker/reviewer). Ce n’est pas une mesure des
limites fournisseurs. Propriétaire Codex, suivi de cadrage/méthodes : justifier
la borne et réduire les répétitions à contrat constant avant un nouvel ajout
qui l’excéderait ; aucun relèvement arbitraire ou nouvelle politique ici.

## Replan IA3c.2f-04 — défaut local et configuration UX, 17 octobre

Précondition changée : l’utilisateur répond « aucun » pour le fournisseur
d’identité, puis exige une configuration depuis l’UX et un choix par défaut
permettant de lancer le modèle. La question précédente n’est plus en attente.
L’exigence d’un fournisseur humain externe était une hypothèse trop restrictive,
pas une contrainte du contrat générique identitycontract.Principal. Son code
valide les données ; il n’implémente aucune authentification locale humaine.

Décision de réalisation retenue : assistant de premier lancement avec compte
humain local personnel présélectionné, puis réglage Admin ; fournisseur
d’entreprise optionnel. Hébergement local/distant et fournisseur IA restent
deux autres choix. Pas de mot de passe commun ou de premier appelant autorisé
automatiquement. Le défaut sélectionne un parcours, pas des droits. L’exécution
protégée attend toujours l’enrôlement et les frontières vérifiées, avec une
résolution guidée, sans exiger une infrastructure d’identité externe.

| Voie comparée | Valeur, risque et conséquence | Décision et état |
| --- | --- | --- |
| Compte humain local personnel | Sans IdP externe ; stockage des secrets dérivés, sessions, récupération et anti-abus deviennent une responsabilité du produit. Un mot de passe ne prouve pas une présence humaine physique et reste exposé au phishing. L’administration hôte reste dans la base de confiance du pilote. | Défaut retenu pour le parcours initial ; mécanisme et UX non implémentés |
| Passkey/WebAuthn locale | Candidat pour renforcer l’authentification ; dépend d’un client/authentificateur compatible et d’un contexte sécurisé. Enrôlement initial et récupération restent nécessaires. | Amélioration conservée, pas un prérequis matériel ajouté au démarrage ; qualification avant activation |
| Fournisseur d’entreprise | Gestion fédérée et politiques centrales ; nécessite une infrastructure absente ici et une liaison de comptes vérifiée. | Option configurable, aucun fournisseur imposé au pilote |

Sources primaires consultées le 17 octobre : [OWASP Authentication](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html),
[OWASP Password Storage](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
et [W3C WebAuthn](https://www.w3.org/TR/webauthn-3/). Elles motivent la protection
des secrets et sessions et la qualification du renforcement ; elles ne prouvent
pas notre implémentation. L’algorithme de dérivation et ses paramètres seront
choisis avec une bibliothèque éprouvée, mesurés et exposés selon le contrat
produit ; aucune limite arbitraire de test ou nouvelle politique cachée ici.

Périmètre de cet incrément : les sept pages annoncées (plan, contrat cible,
contrats FR/EN, installation FR/EN, note scientifique). Aucune source moteur,
ressource embarquée ou interface modifiée. HEAD observé à la reprise :
06481f15e677285b789cfb81ba8e02e351bfdc27, branche codex/clear-product-method-names ;
travaux sales et changement de HEAD partagé conservés. Le message entrant du
chantier mémoire annonce M33 sur paused/prepareLaunch dans agents_store.go ;
ces blocs restent intacts, aucune réponse interconversation non autorisée.

Suite rattachée aux lots existants, sans nouvelle mission : IA3c.2f-03 conserve
la frontière/service et le contrôle humain réels ; IA3c.2f-04 inclut désormais
l’assistant et les critères UX1–UX7 du contrat cible ; -05 raccorde l’admission
publique et -06 reste la migration distante. Codex possède la réalisation et
ses preuves. Le choix de source humaine est résolu ; l’identité personnelle,
son authentification, le service séparé et le confinement restent à établir par
le parcours. Il n’est plus nécessaire d’attendre une URL d’IdP externe pour
préparer ou implémenter ce parcours. F01–F07, IDA et RETEX global restent ouverts.

Améliorations conservées : I2f04-01, origine exigence UX utilisateur et hypothèse
d’IdP imposé, valeur démarrage autonome du pilote, périmètre setup/Admin ;
compte local par défaut décidé et spécifié, effet non implémenté. I2f04-02,
origine séparation entre défaut et récupération, valeur empêcher une reprise
de contrôle après perte de fichier ; propriétaire Codex service/setup, critère
UX4 (configuration absente après enrôlement ≠ installation vierge), spécifié
non vérifié. I2f04-03, origine comparaison WebAuthn, valeur renforcer l’identité
humaine, propriétaire Codex authentification ; différé jusqu’à qualification
client/authentificateur/récupération, sans bloquer le parcours local par défaut.

Vérification documentaire : sept sections présentes, 22 liens locaux du
périmètre relu résolus, même topologie du schéma FR/EN, trois diagrammes rendus
et inspectés visuellement, git diff --check PASS. Le suivi et les contrats
persistants transmettent le choix aux prochaines reprises ; aucun contexte
embarqué ni processus existant n’est mis à jour par ces pages. La comparaison
avec l’archive 03d inclut aussi des changements d’installation/version et de
contrat commun d’autres travaux : ils sont préservés, non attribués à ce lot.
Preuves et contrôleur reproductible dans
[ia3-default-ux-20261017](verification-identites-agents-20261016/ia3-default-ux-20261017/).
Les critères comportementaux UX1–UX7 restent NOT TESTED : aucun test Go ou
browser inventé pour cette décision documentaire, aucun compte/service créé,
aucun workflow réellement lancé et aucune acceptation sécurité déduite.

## Relecture de cohérence Identité et AC-02 — 23 octobre 2026

Le [rapport croisé](securite-execution-apex.md#8-cohérence-croisée-avec-le-plan-identité-et-la-file-apex)
conserve AC02-RV-05/06 : distinguer parité G7, livraison fonctionnelle Identité et
extension AC-02 ; nommer la sortie IA3 consommée par M20, puis le raccord mémoire
attendu par IA4. Deux ambiguïtés documentaires à clarifier par l’intégration
racine et les propriétaires, pas des cycles effectifs ou des défauts runtime
démontrés. Les décisions et preuves historiques ci-dessus restent conservées ;
aucune protection requise, autorité ou condition d’activation n’est assouplie.

## Dépendances de reprise précisées — 23 octobre 2026

La suite « next » précise le [contrat de remise commun](securite-execution-apex.md#9-dépendances-et-livrables-partagés-précisés).
IA3 fournit l’autorité protégée nécessaire aux usages réels de M20/P2–P4 ; IA4
consomme le raccord mémoire et ne réimplémente pas son contrôle. IA5 conserve
ses critères après IA3/IA4 ; sa clôture entière n’est pas un préalable à M20.
Le chantier mémoire possède les sources, versions, attribution et usages liés ;
Identité possède l’autorité, sa protection et l’interface de ses droits.
L’intégration racine attribue chaque frontière et relie les preuves partagées.

La qualification de parité G7 et la clôture fonctionnelle IA1–IA5 sont distinctes.
Toute protection requise par le parcours Identité reste exigée avant activation,
y compris EP-07/F07 ; elle n’attend pas uniquement le futur AC-02. L’inventaire
de migration doit conserver garantie, preuve et refus pour chaque parcours.
Ces précisions ferment l’ambiguïté documentaire AC02-RV-05/06, sans fermer F01–F07,
EP, IDA ni les critères comportementaux encore non vérifiés. Aucun code lancé.
