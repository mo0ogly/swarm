# Analyse de la mémoire Deep Agents et de l’existant Swarm

État courant : audit technique ciblé M1 exécuté ; résultats et limites dans la
seconde partie. Proposition M2 dans `DECISION-MEMOIRE.md`. Les observations du
premier lot ci-dessous sont conservées comme historique.

## Références et portée

Lecture du 15 octobre 2026. Dépôt officiel Deep Agents fixé au commit
`29d646fd2d832a44c476964333eddecb0ec1c0cd`, SDK déclaré `0.7.25`.
Swarm : `4e58f7cbf8200a58f69f1f4d03d657c3313fb642` et modifications locales
préexistantes. Empreintes et chemins dans `sources.json`.

Ce rapport décrit un premier parcours statique du code et de tests identifiés.
Il ne certifie ni une exécution du SDK ni l’instance Swarm en cours. Les points
non suivis jusqu’à leur effet restent ouverts. Les liens amont utilisent le SHA
fixe ; la documentation web a été consultée à la date ci-dessus et peut évoluer.

## Points d’entrée vérifiés

| Sujet | Source principale | Rôle observé |
| --- | --- | --- |
| Assemblage SDK | `libs/deepagents/deepagents/graph.py` | Paramètres `memory`, `backend`, `checkpointer`, `store` et installation du middleware. |
| Chargement mémoire | `libs/deepagents/deepagents/middleware/memory.py` | Lecture des sources et composition du prompt. |
| Stockage | `libs/deepagents/deepagents/backends/{state,store,filesystem,composite}.py` | État de conversation, store à namespace, disque ou routage ; configuration déterminante. |
| Application Code | `libs/code/deepagents_code/agent.py`, `memory_guard.py` | Choix utilisateur/projet, option auto-save et protection d’une partie gérée. |
| Tests mémoire | `libs/deepagents/tests/unit_tests/middleware/test_memory_middleware.py` | Ordre, fichier absent, état déjà chargé, injection et assemblage avec backends. |
| Documentation | [Memory](https://docs.langchain.com/oss/python/deepagents/memory), `libs/ARCHITECTURE.md`, `openwiki/concepts/backends.md` | Intentions, exemples et différences de durée/portée à confronter au code. |
| Swarm | `internal/engine/retex.go`, `agents_store.go`, `project_profile.go`, `dialogue_context.go`, `store.go` | Publication/chargement du guide, consignes, références et historique. |

## Constats établis par lecture

### MEM-01 La mémoire de base est un ensemble de fichiers injectés dans le prompt

`MemoryMiddleware.before_agent` télécharge les sources configurées et produit
`memory_contents`. `_format_agent_memory` les compose dans l’ordre de `sources`,
avec leur chemin, et retire les commentaires HTML. `modify_request` ajoute ce
contenu au message système. Le middleware inspecté ne réalise pas lui-même une
recherche sémantique dans des souvenirs.

Sources : [chargement](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/middleware/memory.py#L279)
et [composition](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/middleware/memory.py#L245).
Tests repérés : `test_format_agent_memory_preserves_order` et
`test_load_memory_from_backend_multiple_sources`.

Conséquence pour l’étude : distinguer ce mécanisme du chargement progressif des
skills et des fonctions de recherche proposées ailleurs. Aucune base vectorielle
n’est choisie pour Swarm sur la base de ce constat.

### MEM-02 Écriture encouragée par le prompt et écriture garantie sont différentes

`MEMORY_SYSTEM_PROMPT` demande au modèle d’utiliser `edit_file` pour retenir des
informations durables. Le middleware charge et injecte la mémoire ; le chemin
inspecté ne constitue pas un service déterministe qui extrait et valide seul
chaque nouvel apprentissage.

Deep Agents Code choisit des sources utilisateur et projet. Dans
`agent.py:3055`, `memory_auto_save` et le mode interactif déterminent le texte
d’instructions mémoire employé. L’option désactivée change le prompt ; ce seul
chemin ne prouve pas l’interdiction technique de toute écriture sur ces fichiers.
`ManagedMemoryGuardMiddleware` protège une section gérée spécifique ; son
étendue doit être suivie avant toute généralisation.

Sources : [prompt mémoire](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/middleware/memory.py#L105),
[assemblage Code](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/code/deepagents_code/agent.py#L3055).

À vérifier : modification réelle, droits, protection des sources et relecture
lors de la conversation suivante ; aucun effet d’apprentissage réel démontré ici.

### MEM-03 La persistance dépend du backend et du stockage concret

`create_deep_agent` choisit `StateBackend()` si aucun backend n’est fourni et
transmet séparément `checkpointer` et `store`. `StateBackend` utilise les fichiers
de l’état de conversation. `StoreBackend` utilise un `BaseStore` et une fonction
de namespace fournis par l’appelant ; le namespace est validé. L’extension
`libs/code/examples/extensions/memory_store.py` illustre un routage `/memories/`.

Sources : [backend par défaut](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/graph.py#L657),
[contrat StoreBackend](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/backends/store.py#L90).

Une mémoire partagée entre threads ne démontre pas une conservation après arrêt
du processus : cela dépend du store réellement configuré. Un namespace choisi
par l’application ne constitue pas à lui seul une preuve d’autorisation métier.
La recette doit vérifier ces propriétés séparément.

### MEM-04 Un état déjà chargé n’est pas relu par ce middleware

Les méthodes synchrone et asynchrone retournent sans téléchargement lorsque
`memory_contents` est déjà présent dans l’état. Le test
`test_before_agent_skips_if_already_loaded` vérifie ce contrat.

Source : [garde](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/middleware/memory.py#L294).

Cela établit une garde de chargement, sans prouver que toute modification de
fichier est ignorée par tous les chemins du produit. À suivre : cycle de vie de
cet état, nouveau thread, reprise et éventuels mécanismes de rafraîchissement.
Ne pas annoncer une mémoire toujours fraîche sans cette vérification.

### MEM-05 Une source absente est ignorée et un commentaire de test est trompeur

Le code ignore `file_not_found` et lève une erreur pour les autres échecs de
téléchargement. Dans `test_load_memory_handles_missing_file`, la docstring
annonce une erreur ; les assertions vérifient en réalité le chargement du seul
fichier présent. Le décalage concerne ce commentaire précis, pas l’ensemble de
la documentation. Le test n’a pas été exécuté lors de cette lecture.

Sources : [branche d’erreur](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/deepagents/middleware/memory.py#L300),
[test](https://github.com/langchain-ai/deepagents/blob/29d646fd2d832a44c476964333eddecb0ec1c0cd/libs/deepagents/tests/unit_tests/middleware/test_memory_middleware.py#L312).

Question de transposition : Swarm doit-il distinguer source optionnelle et
source obligatoire ? Décision à prendre après comparaison de son contrat actuel.

### MEM-06 Swarm possède déjà une publication et un chargement de connaissances

`retex.go:50` exige un fait sourcé, une tâche acceptée et des preuves fraîches pour
qualifier une leçon. `retexGuide` prépare puis publie une note et l’index du guide
de terrain, avec garde de révision et traitement des modifications externes.
`agents_store.go:957` peut joindre le contenu de `.claude/field-guide/index.md`
au prompt. Ce chargement concerne l’index ; il ne prouve pas l’injection de tous
les fichiers de leçons qu’il référence.

Les profils projet peuvent aussi inclure l’index et les instructions par rôle.
`project_profile.go` conserve les empreintes des sources dans le contexte et
refuse certaines entrées invalides. `dialogue_context.go` construit un manifeste
avec les échanges inclus/exclus et des références empreintées.

`checkpoint` valide et conserve des chemins `w.Memory` ; `agents_store.go` les
présente comme pièces à consulter. Leur présence ne prouve pas que leurs corps
ont été lus par l’agent. `archive.go` inclut les références explicites disponibles
au bundle d’export et signale des pièces manquantes.

Enfin, `revise-instructions` et `web/instruction-revisions.js` constituent un autre
chemin d’apprentissage des consignes de tâches futures. Ils ne sont pas à
confondre avec une mémoire générale entre missions.

Sources Swarm : fichiers du manifeste, notamment `retex.go:50`, `retex.go:207`,
`agents_store.go:946`, `agents_store.go:957`, `project_profile.go:132`,
`project_profile_catalog.go:68`, `dialogue_context.go:26`, `store.go:839`,
`archive.go:187`. Tests repérés :
`TestRetexTaskExportAndVerifiedGuide`, `TestGuideContractNestedBudgetLinksAndSymlinks`,
`TestContextPreviewFreezesAndRejectsDrift` et tests des révisions de consignes.

Conséquence : l’adaptation devra partir de ces producteurs et consommateurs,
plutôt que déclarer la mémoire Swarm inexistante. Vérifier aussi le traitement
actuel d’un index absent, illisible ou trop volumineux dans les différents chemins.

## Couverture et prochaines vérifications du premier lot

| Dimension | État | Prochaine preuve |
| --- | --- | --- |
| Chargement et injection SDK | Lecture faite | Assertions sur le contenu réellement transmis ; ordre, Unicode et commentaires. |
| Source absente ou inaccessible | Branche lue et test repéré | Exécuter absence optionnelle, refus et contenu invalide dans un environnement isolé. |
| Écriture et relecture | Instructions et backends repérés | Suivre `edit_file`, relire dans une nouvelle conversation et comparer avec l’état déjà chargé. |
| Persistance et isolation | Contrats des backends repérés | Deux threads, deux namespaces et redémarrage avec stockage durable identifié. |
| Couche Deep Agents Code | Assemblage lu | Étendue de la protection, auto-save et droits effectifs ; ne pas confondre prompt et contrôle. |
| Guide Swarm et contexte reçu | Producteurs et consommateurs repérés | Recette publique isolée RETEX → guide → prompt reçu ; source modifiée et rejet. |
| Recherche et consolidation | Non attribuées au chemin étudié | Localiser et analyser les implémentations référencées avant toute proposition de réemploi. |
| Tests amont exécutés | NOT TESTED | pytest est présent ; langchain, langgraph et langchain_anthropic manquent. Préparer un environnement isolé sans clés ni appels externes. |

## Décisions encore ouvertes au premier lot

Aucun format de mémoire, moteur de recherche, base de données ou mode de
consolidation nouveau n’est retenu. La documentation décrit plusieurs usages ;
M1 doit distinguer leurs configurations et implémentations. M2 comparera ensuite
les apports démontrés aux mécanismes Swarm existants.

## Itération M1 avec exécution isolée du 15 octobre 2026

Le premier lot statique ci-dessus est conservé comme historique. Cette itération
exécute le SDK à la même révision, avec des modèles factices qui enregistrent les
messages reçus, puis contrôle le contexte Swarm jusque dans l’entrée standard
d’un processus fournisseur local. Elle ne mesure pas la qualité d’apprentissage
d’un vrai LLM et ne qualifie pas une instance de production.

### Référentiel et protocole

Environnement Python séparé sous `/tmp/swarm-deepagents-memory-env-944lMS` ; source
SDK importée depuis le checkout amont fixé. Les commandes Python ne reçoivent
aucune clé fournisseur ; la télémétrie est désactivée, les plugins pytest sont
explicitement sélectionnés et les tests bloquent les sockets réseau. Un
sous-processus frais vérifie le stockage disque avec un modèle factice, sans
appel de modèle distant.

Les tests Go utilisent les racines temporaires du projet. Les sondes ajoutées
pour l’audit sont compilées par `go test -overlay` depuis le dossier de preuves :
aucun fichier applicatif Swarm ou amont n’a été modifié. Un des tests d’un autre
chantier a changé pendant l’audit ; les sources mémoire sélectionnées sont
inchangées. Une dernière exécution ciblée est liée à un nouveau référentiel Go
complet. Voir `evidence/m1-20261015/source-drift.json`.

### Résultats et limites

| Contrôle | Résultat observé | Preuve originale |
| --- | --- | --- |
| Tests mémoire amont synchrones et asynchrones | 55 tests réussis | `evidence/m1-20261015/upstream-memory-ready.log` |
| Écriture par `edit_file`, disque et nouveau thread | Fichier modifié ; la nouvelle conversation reçoit la nouvelle valeur | `upstream-paths.log`, scénario `test_disk_edit_new_thread_and_same_thread_snapshot` |
| Conversation déjà chargée | Ancienne valeur conservée dans son message système, y compris au tour suivant | Même scénario ; confirme MEM-04 |
| Store partagé et namespaces | Nouvelle conversation de même identité reçoit la modification ; autre identité et thread séparés reçoivent leur propre mémoire | `upstream-paths.log`, scénario `test_store_write_shared_across_threads_and_namespaces_isolated` |
| Réutilisation d’un thread avec une autre identité | L’ancien instantané reste présent ; le changement de namespace seul ne recharge pas la mémoire | `upstream-paths.log`, scénario `test_reusing_thread_with_new_identity_keeps_prior_memory_snapshot` |
| Écriture refusée par règle de fichier | `ToolMessage` en erreur et octets inchangés | `upstream-paths.log`, scénario `test_denied_memory_edit_does_not_mutate_file` |
| Redémarrage de processus | Un nouveau Python charge le fichier disque dans le message réellement reçu par le modèle factice | `upstream-paths.log`, scénario `test_filesystem_memory_loaded_in_fresh_process` |
| Backend d’état par défaut | Mémoire conservée dans son thread, absente d’un autre thread sans ensemencement | `upstream-boundaries.log` |
| Lecture d’outil interdite et mémoire configurée | `read_file` est refusé mais la même source mémoire configurée est injectée au démarrage | `upstream-boundaries.log` |
| Deux modifications concurrentes d’un même fichier partagé | Deux succès rapportés ; une seule modification subsiste après entrelacement déterministe | `upstream-boundaries.log` |
| Constructors des exemples web archivés | Les formes `StateBackend(rt)` et `StoreBackend(rt, namespace=...)` lèvent `TypeError` avec le SDK fixé | `doc-constructors.log` et `upstream-memory-doc.md` |
| Contrôles Swarm existants | 9 tests de premier niveau réussis : contexte, recherche, RETEX, guide, profils et révisions | `swarm-existing.log` |
| RETEX jusqu’au prompt préparé | Le lien de la leçon publiée est présent ; le corps n’est pas implicitement injecté | `swarm-prompt-ready.log` |
| Index implicite trop volumineux | L’aperçu réussit en omettant l’index ; le chemin ne produit pas de motif d’omission explicite | `swarm-prompt-ready.log` |
| Entrée standard du fournisseur local | Index reçu ; corps de leçon reçu seulement lorsqu’il est explicitement dans le profil projet | `swarm-delivery.log` |

Les 8 scénarios complémentaires amont sont des sondes locales du contrat,
distinctes des 55 tests du dépôt. Les compteurs Go finaux, les commandes exactes,
les horodatages, les codes retour et les empreintes sont dans
`evidence/m1-20261015/resultats.json`. Les succès ci-dessus constatent des
comportements ; ils n’en font pas tous des propriétés souhaitables pour Swarm.

### MEM-07 La portée d’un thread doit rester liée à son propriétaire

Le scénario de réutilisation de thread confirme un instantané mémoire conservé
alors que le contexte d’identité change. Ce résultat ne démontre pas une fuite
par un serveur déployé : son contrôle d’accès peut empêcher cette réutilisation.
Il démontre que le namespace du backend ne remplace pas la propriété du thread.
Swarm doit conserver ses identités de mission, tâche, rôle et tentative lors de
l’admission et de la transmission d’une source.

### MEM-08 Les règles des outils ne filtrent pas les sources mémoire configurées

`MemoryMiddleware` appelle le backend de téléchargement directement. Le test
montre qu’une source choisie via `memory=` peut atteindre le prompt malgré un
refus du même chemin par l’outil `read_file`. Ce sont deux frontières distinctes,
ce qui concorde avec la séparation des appels backend et des outils médiés.
La sélection des sources doit donc elle-même être autorisée et validée.

### MEM-09 Une mémoire partagée nécessite une règle de concurrence explicite

`StoreBackend.edit` lit, remplace puis écrit le fichier. Deux backends indépendants
sur le même namespace peuvent lire le même original avant de publier chacun sa
version ; la sonde reproduit une modification perdue sans échec rapporté.
La documentation web mentionne elle aussi les conflits d’écriture sur un fichier.
Ce comportement concerne cette configuration et cet entrelacement, pas tous les
stores possibles. Pour Swarm, préserver les révisions, verrous et règles de
propriété au lieu de transposer une simple écriture de fichier partagé.

### MEM-10 Documentation mémoire et configuration exécutable doivent être rapprochées

La page mémoire archivée possède des exemples complets dont les constructeurs
ne correspondent plus au SDK fixé. Les formes abrégées de la même page utilisent
les constructeurs actuels. Les deux appels en décalage ont été exécutés sans
modèle : l’incompatibilité est confirmée et conservée, sans modifier l’amont.

La consolidation en arrière-plan décrite sur cette page est une recette avec un
autre agent et un déclencheur à configurer. Elle ne constitue pas un service de
consolidation automatiquement installé par `MemoryMiddleware`. Les évaluations
`libs/evals/tests/evals/test_memory.py` et `test_memory_multiturn.py` distinguent
par ailleurs rappel, préférences durables et informations transitoires ; elles
utilisent des modèles et juges réels et n’ont pas été exécutées dans cet audit.

### MEM-11 Le chemin implicite du guide Swarm peut omettre sa mémoire sans l’expliquer

Le défaut observé est circonscrit : dans un lancement sans source explicite du
profil pour ce guide, `agents_store.go` ne joint l’index que si sa lecture réussit
et si sa taille est au plus 16 000 octets. La sonde avec un index trop grand
obtient néanmoins un aperçu réussi. Le marqueur du guide est absent du prompt.
Une absence de contenu devient ainsi difficile à distinguer d’un choix de mémoire.

Le comportement des profils explicites est différent : les sources sont lues,
empreintées et gardées avant lancement, et certaines erreurs bloquent l’envoi.
Il faut préserver cette distinction dans la correction proposée, sans affirmer
que toutes les sources mémoire Swarm sont silencieusement ignorées.

### MEM-12 Le corps des leçons est déjà transmissible par les profils Swarm

La sonde de livraison utilise le fournisseur `capture-fixture`, un processus
local qui écrit son entrée standard dans une fixture. Elle confirme que le corps
d’une leçon est reçu lorsqu’il figure dans `ProjectProfile.Instructions` pour le
rôle `worker`. Le chemin implicite transmet l’index, pas le corps de toutes les
notes liées. L’envoi ne prouve ni lecture autonome des liens ni usage sémantique.

`web/project-profile-settings.js` propose actuellement la sélection de profils
prédéfinis et `web/project-profile.js` montre chemins, tailles et empreintes.
La documentation `docs/PROJECT-PROFILE.md` réserve la configuration personnalisée
au CLI. L’API HTTP expose la lecture du profil et la sélection par ID, sans
opération HTTP d’édition complète du profil dans ce gestionnaire. L’écart à traiter
est donc une sélection guidée et la visibilité du chargement, plutôt qu’un
nouveau stockage supposé nécessaire.

### Incidents de méthode et récupération

- Le premier lancement pytest échouait avant collecte : ses filtres amont
  importaient `pytest_cov`, absent de l’environnement minimal. Le log initial
  est conservé. Après inspection des filtres et installation isolée des deux
  plugins requis, les 55 tests ont réellement été exécutés.
- La première sonde de publication RETEX omettait l’état « vérifié ». Le moteur
  l’a refusée. Le source et le log initiaux sont conservés ; une seconde fixture
  passe par `retex-status` avant promotion et vérifie ensuite l’envoi. Il ne
  s’agit pas d’une correction de l’application.
- Un changement d’un test étranger a motivé un dernier contrôle ciblé avec un
  référentiel de sources renouvelé, sans toucher au travail de son propriétaire.

### Limites conservées et passage à M2

Le SDK est testé avec des modèles factices et un store en mémoire pour le cas
partagé ; le disque est vérifié après changement de processus. Aucun store de
production, compte fournisseur, campagne d’évaluation LLM ou UI navigateur n’est
qualifié. La couche Deep Agents Code et sa protection de bloc géré restent
analysées statiquement ; elles ne sont pas présentées comme testées ici.

L’audit technique ciblé permet de préparer le choix M2. La qualité de ce qu’un
LLM décide de mémoriser, l’utilité des connaissances et les erreurs de rappel
restent des critères d’une future expérience autorisée. La proposition concrète
est consignée dans `DECISION-MEMOIRE.md`, distincte de l’existant vérifié.


## Complément documentaire après M1

L’utilisateur a fourni la référence conceptuelle LangChain mémoire, désormais
archivée dans `evidence/m1-20261015/langchain-memory-concepts.md` avec son empreinte.
Elle complète le cadrage sans modifier les observations du SDK. Le réexamen M2
figure dans `DECISION-MEMOIRE.md` ; les modèles référencés par cette page ne sont
pas présentés comme audités ni intégrés.


## Itération M2 sur les usages mémoire et les modèles référencés

L’analyse de `memory-agent` et du template archivé est maintenant réalisée à
révisions fixes. Les 7 sondes passent avec les limites explicites de leur
périmètre : graphe réel et modèle factice pour le premier ; adaptateur réel et
fabrique langmem espionnée pour certaines vérifications du second. Les
décalages documentaires et les limites reproduites sont détaillés dans
`CADRAGE-MEMOIRE.md`. Huit tests Swarm supplémentaires vérifient les acquis de
dialogue, reprise et correction des consignes. Les sources M1 n’ont pas changé.

Ce complément ne qualifie pas la pertinence sémantique d’un LLM. Les neuf
exigences de cadrage et les dix cas de `cas-evaluation.json` décrivent la cible
du parcours futur, distincte des vérifications exécutées. Les références de M2
sont dans `evidence/m2-20261015/sources.json` et `resultats.json`.

## Complément de couverture : mémoire procédurale

La question utilisateur sur la mémoire procédurale a montré une lacune dans le
passage du cadrage au premier incrément : le guide entier était archivé, mais
la proposition de réalisation détaillait surtout les expériences RETEX. La
lecture de la page ne prouve pas l’audit de tous ses liens ni l’implémentation
de ses trois types de mémoire durable. Le choix M2 est donc rouvert ; les
observations et propositions historiques restent conservées.

Le complément de `CADRAGE-MEMOIRE.md` inventorie chaque axe du guide et inspecte
les optimiseurs de prompts LangMem, leur exemple de lecture/écriture/réemploi et
les révisions de consignes Swarm. Révision LangMem :
`48e3c11f5bb527282c7d5339c6a87a0b35abccfc`. Sources et extraits sélectionnés :
`evidence/m2-procedural-20261015/`. Lecture statique ; aucun test LangMem, appel
LLM, entraînement, changement de prompt actif ou code applicatif modifié.

Limite de méthode conservée : une recherche a supposé un fichier de test amont
absent et a renvoyé une erreur. L’inventaire réel a ensuite identifié le lanceur
d’exemples `tests/test_docstring_examples.py` ; aucun test absent n’est présenté
comme lu ou exécuté. Les recherches de code excluent désormais les volumineuses
cassettes enregistrées lorsque le contrat recherché est du Python.

## Complément : guide pratique LangGraph réellement visé

Le texte fourni par l’utilisateur est `Memory`, source documentaire
`src/oss/langgraph/add-memory.mdx`, et non `Memory overview` de la page conceptuelle.
Les vérifications répétées de l’URL conceptuelle avaient confirmé son contenu,
mais ne répondaient pas au désaccord sur le document visé. Responsable : Codex ;
condition de reprise maintenant satisfaite : texte intégral fourni et source
pratique identifiée. La prévention consiste à comparer titre, sections et texte
en cas de désaccord, sans réaffirmer l’identité du contenu sur la seule URL.

Le complément du cadrage examine checkpoints, sous-graphes, store et Runtime,
recherche sémantique, réduction/suppression/résumé, historique et migrations.
Deux sondes exécutées sur les bibliothèques LangGraph installées distinguent
projection et état conservé, puis état courant, anciens checkpoints et store.
Résultat : 2 tests collectés, 2 réussis ; aucun LLM, backend de base externe ou
parcours Swarm exécuté par ces sondes. Sources et originaux dans
`evidence/m2-howto-20261015/`. Les variantes de bases externes et la qualité des
résumés restent non testées. Trois exigences et quatre cas produit proposés sont
ajoutés, sans les compter parmi les tests exécutés.
