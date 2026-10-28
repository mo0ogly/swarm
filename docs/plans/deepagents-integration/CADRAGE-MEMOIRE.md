# Cadrage de la mémoire Swarm fondé sur le code

## Résultat du cadrage M2

La mémoire attendue doit aider un agent à reprendre le travail courant,
retrouver une connaissance utile, corriger une connaissance devenue fausse et
réutiliser une expérience sans lui attribuer une validité qu’elle n’a pas.
La sélection de fichiers dans l’UI est un moyen d’accès possible, pas la
mesure du résultat métier.

Ce document sépare l’existant observé, les mécanismes amont examinés et le
contrat fonctionnel proposé. Il ne décrit pas une nouvelle implémentation livrée.
Le 15 octobre 2026, l’utilisateur a autorisé l’approfondissement de ce cadrage.
Les décisions de réalisation et de stockage restent à arrêter à partir de lui.

## Références et vérifications de cette itération

| Référence | Révision ou preuve | Périmètre de vérification |
| --- | --- | --- |
| Deep Agents | `29d646fd2d832a44c476964333eddecb0ec1c0cd` | Acquis M1 : 55 tests amont, 8 sondes et 12 tests Swarm ; limites conservées dans l’audit. |
| memory-agent | `05ed21ef365fc2c24ca5a735f6e65b0e2296c967` | Graphe réel avec modèle factice et store local ; écriture, rappel, correction par ID et namespace observés. |
| memory-template | `c4e05d4c5c519256eb70ab1be763c81df03040ed` | Adaptateur et consommateur réels inspectés ; fabrique langmem remplacée par un espion dans les sondes d’arguments et de cache. Aucun gestionnaire langmem complet exécuté. |
| Swarm | `4e58f7cbf8200a58f69f1f4d03d657c3313fb642` et modifications locales préexistantes | 8 tests ciblés supplémentaires de dialogue, reprise, correction de consignes et atomicité. Sources sélectionnées inchangées. |
| Guide conceptuel | [LangChain memory](https://docs.langchain.com/oss/python/concepts/memory) | Référence de cadrage archivée ; elle ne prouve aucune fonction Swarm. |

Les 7 sondes supplémentaires amont et les 8 tests Swarm passent. Certains succès
reproduisent une limite ou un défaut, ils ne certifient pas un comportement
souhaitable. Sources, commandes et résultats : `evidence/m2-20261015/` ; les logs
exécutés par le lanceur conservé de M1 sont `evidence/m1-20261015/m2-models.log`
et `swarm-m2.log`. Les deux avertissements du template concernent l’ancien
`config_schema`. Aucun appel LLM réel, déploiement ou changement applicatif.

## Usages et acquis Swarm

| Usage | Ce que le code établit | Limite et besoin proposé |
| --- | --- | --- |
| Reprendre une conversation | `agent_dialogue.go` réutilise une identité de session fournisseur validée entre tours et conserve les limites cumulées. | Un wrapper déjà commencé ne redémarre pas implicitement. Le test des arguments de reprise ne démontre pas une restauration réelle après arrêt de Codex ou Claude. |
| Reprendre une nouvelle tentative | `workerRecoveryContext` fournit des opérations historiques de la tentative précédente, un focus de tâche et les lacunes à vérifier. | Le contexte est borné, attribué et ne vaut pas réussite ni fraîcheur. Préserver ce contrat pour éviter répétitions et confusion entre tentatives. |
| Retenir un fait ou une décision | Les décisions, critères et RETEX sont persistés ; un RETEX distingue fait et hypothèse, et sa publication exige des preuves actuelles. | Ne pas confondre une décision de changement avec son implémentation ; extraction et correction générales entre missions non démontrées. |
| Retrouver une expérience | Rapports, événements, journaux, états et RETEX conservent des précédents. `searchDialogue` recherche des échanges dans un travail précis. | Ces traces ne constituent pas à elles seules un service de sélection d’expériences entre missions. La pertinence et l’effet sur la tâche suivante doivent être mesurés. |
| Améliorer une procédure | `reviseTaskInstructions` applique une addition attribuée à une tâche future du même périmètre ; les révisions restent séparées de la base opérateur. | Généraliser une règle à d’autres missions exige une portée et une décision distinctes. Le rejet ne réécrit pas le contexte d’un agent qui l’a déjà reçue. |
| Transmettre une connaissance | Les profils projet peuvent charger le corps des leçons par rôle ; l’index implicite du guide est également utilisé dans certains lancements. | Les omissions implicites et la sélection sont à rendre visibles. Un contenu transmis ne prouve pas que l’agent l’utilise correctement. |

Les termes conversation, faits, expériences et procédures ci-dessus décrivent des
usages. Ils ne correspondent pas à quatre modules Swarm déjà construits.

## Enseignements tirés des implémentations amont

### M2-A01 Un apprentissage simple repose sur une écriture explicite et un rappel

Dans `memory-agent/src/memory_agent/graph.py`, chaque appel au modèle cherche
les souvenirs du namespace de l’utilisateur, les place dans le prompt puis
expose `upsert_memory`. `store_memory` applique les demandes d’écriture et revient
au modèle. La sonde observe le nouveau fait dès l’appel suivant, puis dans une
nouvelle conversation.

`tools.py` écrit `content` et `context` sous un identifiant, ou remplace la valeur
quand cet identifiant est fourni. Sans identifiant, deux contenus identiques
produisent deux entrées : le texte de l’outil confie la déduplication au modèle.
Un identifiant mal formé dans un appel du modèle est accepté par le dispatch
manuel testé, malgré l’annotation UUID. Swarm doit garder une validation moteur
et une attribution des corrections, pas seulement un schéma présenté au modèle.

### M2-A02 Le scope doit venir d’une identité contrôlée

Le namespace `("memories", user_id)` est alimenté par le contexte d’exécution,
avec un `user_id` par défaut dans l’exemple. Les sondes prouvent la séparation
pour deux contextes explicitement fournis. Elles ne prouvent pas une
authentification, ni l’isolation lorsque des appelants partagent la valeur par
défaut. M1 a également démontré la différence entre propriété du thread et
namespace de stockage.

Pour Swarm, l’unité initiale proposée est le projet, avec restriction par rôle,
mission ou tâche quand nécessaire. Une mémoire personnelle commune à tous les
projets réclame un besoin et des droits distincts ; elle n’est pas présumée ici.

### M2-A03 Le traitement différé est une orchestration à vérifier

`memory-template/src/chatbot/graph.py` envoie un travail différé au SDK avec
`after_seconds` et `multitask_strategy="enqueue"`. Le processeur appelle une
fabrique `create_memory_store_manager` de langmem. Cela ne prouve ni réception,
annulation effective ni consolidation durable dans une installation donnée.
Aucun service de ce type n’est lancé par cet audit.

Les sondes relèvent des décalages concrets dans cette révision archivée :

- les paramètres de schéma configurés ne sont pas transmis à la fabrique dans
  `get_store_manager` ; le résultat de langmem lui-même reste non testé ;
- le cache de cette fonction utilise le seul nom du type, et réutilise son
  premier gestionnaire après changement de modèle ou d’instructions ;
- le namespace déclaré au gestionnaire commence par `memories`, tandis que le
  chatbot cherche au préfixe utilisateur ; un souvenir ensemencé au préfixe
  déclaré n’est pas retrouvé par ce consommateur dans la sonde ;
- le test d’intégration importe un `builder` absent du module actuel et échoue
  sur cet import avant toute évaluation de modèle.

La recette de mise à jour différée est donc une inspiration à analyser, pas
une dépendance prête à installer. Les décalages et les espions sont conservés
avec leurs limites plutôt que présentés comme une recette complète réussie.

### M2-A04 Évaluer la mémoire dépasse sa persistance

Les fichiers d’évaluation mémoire de Deep Agents contiennent des cas de rappel,
préférences implicites, consignes explicites, informations transitoires et
mémoire trompeuse. Les adaptateurs de benchmark comprennent notamment des cas
de résolution de conflits. Ce sont des pistes pour le protocole d’évaluation ;
aucune qualité sémantique ou performance de modèle n’est acquise par leur lecture.

## Contrat fonctionnel proposé

Les exigences suivantes sont une cible à spécifier puis réaliser. Aucun nom
ci-dessous n’annonce une table, un endpoint ou une opération déjà disponible.

| ID | Déclencheur et effet attendu | Observable et garde |
| --- | --- | --- |
| MEM-U01 Apprendre | Une source autorisée contient une découverte utile : proposer une connaissance et ses conditions de réemploi. | Lien à la source originale, auteur, périmètre, nature de l’affirmation ; hypothèse et fait vérifié distingués. |
| MEM-U02 Admettre | Le responsable compétent décide de réutiliser la connaissance dans une portée définie. | Décision attribuée, preuves exigées par la politique, contrôle moteur de droits et fraîcheur. Le texte d’un agent ne s’autoaccorde pas une autorité. |
| MEM-U03 Retrouver | Une nouvelle tâche rencontre un contexte pertinent : sélectionner des connaissances admissibles du projet. | Contenu reçu, source, raison de sélection ou d’exclusion et rôle destinataire observables ; un historique stocké n’est pas compté comme rappel. |
| MEM-U04 Corriger | Une preuve ou une décision invalide une connaissance : la remplacer ou la retirer pour les prochains usages. | Relation avec la version antérieure, motif et auteur conservés ; anciens contextes immuables et absence de réemploi silencieux de l’ancienne version. |
| MEM-U05 Généraliser | Une expérience réussie suggère une méthode utile ailleurs : décider de sa portée de réemploi. | Déclencheur, préconditions, diagnostic, action et résultat reliés ; une similitude textuelle n’accorde ni droit ni preuve de réussite actuelle. |
| MEM-U06 Gérer les conflits | Deux sources se contredisent ou deux corrections visent la même version. | Divergence visible ou refus de précondition périmée ; aucun remplacement décidé sur le seul horodatage, aucune écriture silencieusement perdue. |
| MEM-U07 Respecter le présent | Une décision prévoit une évolution différente du code observé. | Conserver séparément état réel et intention adoptée ; leur confusion doit faire échouer la recette. |
| MEM-U08 Préserver les frontières | Une connaissance vient d’un autre projet, est obsolète, inaccessible ou contient une instruction hostile. | Exclusion ou refus explicite selon le contrat ; les permissions et budgets restent ceux du moteur. |
| MEM-U09 Mesurer | Comparer un parcours avec et sans récupération de mémoire sur les mêmes cas. | Rappel utile, affirmation fausse/obsolète utilisée, correction effective, résultat de tâche, délai et consommation observés ; inconnus explicites. |

Une vie possible de la connaissance est : proposition → admission → réemploi,
puis remplacement ou retrait si nécessaire. Ces étapes fonctionnelles servent
à concevoir les validations ; elles ne sont pas une nouvelle machine d’états
implémentée. La correction de la connaissance et la correction d’un résultat de
mission restent deux opérations distinctes.

## Premier parcours vertical recommandé

**Éviter de refaire une erreur technique grâce à une expérience sourcée.**

1. Dans une mission A, un incident, son diagnostic, une correction et son effet
   sont conservés. Une leçon peut être qualifiée avec le parcours RETEX existant.
2. Dans une mission B du même projet, une tâche sur un périmètre pertinent
   retrouve cette expérience et les conditions de sa leçon.
3. Le responsable ou l’agent autorisé reçoit le contenu sélectionné et ses
   références ; la recette vérifie ce qui a été transmis puis l’action produite.
4. Une correction ou un changement du contexte invalide le réemploi de la
   version concernée. L’ancienne expérience reste consultable comme historique.

Ce parcours traite apprentissage, rappel, correction et début de généralisation
avec les producteurs Swarm déjà présents. L’UI montre les choix et leurs effets.
Le parcours ne se réduit pas à ajouter un sélecteur de fichiers.

La priorité proposée porte sur la mémoire du projet. Le premier lot doit
réutiliser les RETEX, profils, preuves, révisions et événements lorsqu’ils
conviennent. Les écarts réels déterminent les ajouts de contrat. Une base
vectorielle, un agent supplémentaire ou un ordonnanceur différé ne sont pas des
prérequis démontrés par cet audit.

## Découpage pour poursuivre sans perdre les acquis

| Sous-lot | Résultat concret | Dépendance et preuve de sortie |
| --- | --- | --- |
| M2.1 Référencer les sources réutilisables | Relier RETEX, rapports d’incident, décisions et versions de consignes aux connaissances candidates. | Le présent audit ; distinguer événements courants et archives, projet et rôle. |
| M2.2 Définir admission et correction | Contrats d’entrée/sortie, propriétaire, préconditions, persistance, remplacement/retrait et droits. | M2.1 ; réemploi des opérations publiques existantes ou écarts explicitement proposés, sans nouvelle politique cachée. |
| M2.3 Définir sélection et livraison | Déclencheur de rappel, conditions de pertinence, contenu reçu et exclusions observables. | M2.1/M2.2 ; référence déterministe avant d’évaluer une recherche plus complexe. |
| M2.4 Fixer la recette comparative | Cas négatifs et positifs, modèle/budget si nécessaire, critères du parcours A puis B. | M2.2/M2.3 ; `cas-evaluation.json` fournit des cas proposés, pas des résultats. |
| M3 Réaliser l’incrément retenu | Plus petit parcours complet choisi après examen des contrats. | Décision de réalisation ; tests de défaut avant correction, code, docs FR/EN et revue selon les droits. |

La collecte déterministe et la préparation de candidats peuvent être rattachées
aux transitions existantes. Une extraction par LLM et un traitement en arrière-plan
nécessitent leurs déclencheurs, réservations, coûts, reprise et politiques
Admin/API/CLI/Go ; ils ne sont pas ajoutés implicitement au chemin courant.

## Ce qui est acquis et ce qui reste à prouver

Les implémentations citées sont lues et les comportements ciblés sont testés
avec des doubles locaux. Le cadrage distingue les quatre usages et possède
neuf exigences observables. La qualité de l’apprentissage, la pertinence sur de
vrais projets et le gain d’une mission à l’autre restent à mesurer.
Le prochain travail est la spécification du parcours vertical, à partir des
sources et opérations réellement utilisables. Aucune nouvelle fonction mémoire
n’est déployée par la rédaction de ce cadrage.

## Accès directs aux sources discriminantes

- [memory-agent : rappel avant chaque appel au modèle](https://github.com/langchain-ai/memory-agent/blob/05ed21ef365fc2c24ca5a735f6e65b0e2296c967/src/memory_agent/graph.py#L19).
- [memory-agent : insertion et correction par identifiant](https://github.com/langchain-ai/memory-agent/blob/05ed21ef365fc2c24ca5a735f6e65b0e2296c967/src/memory_agent/tools.py#L10).
- [memory-template : cache et paramètres de la fabrique](https://github.com/langchain-ai/memory-template/blob/c4e05d4c5c519256eb70ab1be763c81df03040ed/src/memory_graph/graph.py#L35).
- [memory-template : récupération par le chatbot](https://github.com/langchain-ai/memory-template/blob/c4e05d4c5c519256eb70ab1be763c81df03040ed/src/chatbot/graph.py#L28).
- [memory-template : demande de traitement différé](https://github.com/langchain-ai/memory-template/blob/c4e05d4c5c519256eb70ab1be763c81df03040ed/src/chatbot/graph.py#L50).

## Relecture interne du cadrage

Le cadrage est exploitable pour spécifier un incrément ; il ne prétend pas être
une spécification technique prête à exécuter. La relecture est celle de Codex,
sans revue indépendante ajoutée. Trois points restent à fermer avant M3 :

| ID | Importance avant réalisation | Décision ou contrat à préciser |
| --- | --- | --- |
| M2-S01 | Majeure | Identifier le propriétaire et les opérations publiques d’admission, remplacement et retrait ; aucun pouvoir nouveau issu d’un libellé de mémoire. |
| M2-S02 | Majeure | Définir le déclencheur et les préconditions du rappel, son classement et les conditions de validité ; relier le choix au contenu reçu et à la prochaine action. |
| M2-S03 | Majeure | Choisir la représentation persistée après examen des sources réutilisables et de la recette ; définir migration, idempotence, concurrence et politique commune éventuelle. |

Ces points appartiennent aux sous-lots M2.1 à M2.4 du plan. Les dix cas proposés
sont des exigences de recette ; ils ne sont pas additionnés aux tests réussis.

Le document `INCREMENT-MEMOIRE-01.md` précise désormais les choix recommandés
pour ces points : admission opérateur depuis un RETEX vérifié, versions dans le
stockage projet, recherche déterministe avec sélection explicite, correction et
retrait attribués. Il distingue les capacités publiques à ajouter de celles
observées. Le format exact, sa migration et le raccordement aux contrôles de
droits restent à fixer avant modification du moteur ; cette proposition ne ferme
donc pas M2-S01 à M2-S03 comme contrats implémentés ou acceptés.

## Complément M2 : couverture du guide et mémoire procédurale

### Rectification du périmètre

La page conceptuelle LangChain a été lue et archivée, puis relue après la question
de l’utilisateur sur la mémoire procédurale. Cela ne signifie pas que tous ses
liens, bibliothèques, guides et exemples ont été audités ou exécutés. Le premier
lot RETEX a été proposé avant de détailler suffisamment cette distinction.
Ce défaut de cadrage rouvre le choix de priorité avant M3 ; les acquis et la
proposition d’incrément restent conservés.

Le guide distingue la portée courte/longue, puis trois types de mémoire durable :
faits, expériences et instructions. Il distingue également leur formation pendant
le parcours ou en arrière-plan et leurs modes de stockage/recherche. Ces axes
se combinent ; un fichier ou un RETEX ne suffit pas à les couvrir tous.
Source : [guide mémoire LangChain](https://docs.langchain.com/oss/python/concepts/memory).

| Axe du guide | Couverture acquise | Limite à conserver ou travail nécessaire |
| --- | --- | --- |
| Court terme : état, historique, contexte borné | Code Deep Agents et reprise Swarm inspectés ; sondes ciblées M1/M2. | Restauration réelle d’une conversation après arrêt du wrapper non démontrée ; réduction d’historique et persistance ne sont pas interchangeables. |
| Sémantique : faits, profil ou collection | `memory-agent` audité et sondé ; adaptateur du template inspecté. | Extraction/consolidation LangMem et Trustcall non audités de bout en bout ; qualité des faits, contradictions et choix profil/collection restent à évaluer selon les usages Swarm. |
| Épisodique : expériences et exemples | RETEX, rapports, reprises et proposition A → B documentés. | Sélection d’exemples pertinents et effet sur le résultat non qualifiés sur un modèle réel. |
| Procédurale : règles et instructions | Révisions Swarm inspectées et tests antérieurs conservés ; optimiseur LangMem et exemple de réemploi inspectés dans ce complément. | Promotion d’un retour en règle durable, attribution au bon rôle, activation, retour arrière et bénéfice réel restent à spécifier/vérifier. |
| Formation immédiate ou différée | `memory-agent` et programmation du template examinés. | Réception, annulation et effets réels d’un service différé non vérifiés. Aucune nécessité d’ajouter un service démontrée pour le premier parcours. |
| Stockage, namespaces, filtres et recherche | Backends Deep Agents et stores utilisés dans les sondes ; guide LangGraph stores ouvert. | Tous les stores de production et leurs migrations ne sont pas audités ; isolation déclarée par namespace distincte d’une authentification. |
| Évaluation et gouvernance | Provenance, droits, fraîcheur et dix cas proposés dans le cadrage initial. | Cas procéduraux ajoutés ci-dessous ; mesurer correction et régression, pas uniquement restitution ou écriture. |

Les guides liés sur les stores et la gestion des messages ont été ouverts ; cela
ne constitue pas leur audit exhaustif. L’article Reflection Agents est lu pour
distinguer réflexion sur une réponse et apprentissage durable ; ses notebooks
liés n’ont pas été audités intégralement. Les liens généraux sur la mémoire
humaine ne constituent pas des dépendances d’implémentation Swarm.

### Ce que le code procédural amont établit

Complément à SHA fixe : `langchain-ai/langmem`, révision
`48e3c11f5bb527282c7d5339c6a87a0b35abccfc`. Sources et empreintes :
`evidence/m2-procedural-20261015/sources.json`. Lecture statique ciblée,
sans installation LangMem, appel LLM ou nouveau test d’optimisation exécuté.

- `src/langmem/prompts/optimization.py:create_prompt_optimizer` choisit entre
  `gradient`, `metaprompt` et `prompt_memory`. Il construit un composant
  d’optimisation ; ce choix seul n’installe pas un service de mémoire durable.
- `stateless.py:PromptMemoryMultiple.invoke` reçoit un prompt et des trajectoires
  avec retours, appelle un modèle à sortie structurée et renvoie un texte de
  prompt. La méthode n’écrit pas elle-même dans un store.
- Le notebook `examples/intro_videos/procedural_memory.ipynb` montre les étapes
  distinctes : lire le prompt actuel, appeler l’optimiseur avec le retour,
  enregistrer explicitement sa sortie, puis relire la clé dans la fonction de
  prompt de l’agent. Son store est `InMemoryStore` : cet exemple seul ne démontre
  pas la persistance après redémarrage ni l’absence de régression.
- `MultiPromptOptimizer.ainvoke` fait sélectionner des noms de prompts par un
  classifieur, valide ces noms et optimise ceux retenus. Le résultat reste une
  liste de propositions ; cette attribution par modèle ne remplace pas les droits
  des rôles Swarm ni une preuve que le bon responsable a changé de comportement.
- Le module interne distinct `stateful.py:update_general` lit et réécrit une
  valeur du store si la sortie indique une mise à jour. Les gardes d’adoption,
  de concurrence, d’historique et d’acceptation Swarm ne sont pas établies par
  cette lecture ; ne pas copier son écriture directe comme contrat de Swarm.

Les guides officiels expliquent également l’optimisation à partir de trajectoires
et retours : [prompt unique](https://langchain-ai.github.io/langmem/guides/optimize_memory_prompt/)
et [plusieurs prompts](https://langchain-ai.github.io/langmem/guides/optimize_compound_system/).
Leurs exemples de sortie ne sont pas des résultats reproduits ici.

### Correspondance précise avec Swarm

`reviseTaskInstructions` conserve base, ajout, décision, événements sources,
périmètre et empreintes avant/après. Il vise une tâche future `todo` du même
périmètre, exclut un sous-planificateur et n’autorise pas à changer critères,
dépendances ou budgets. Le rejet préserve l’historique ; le lancement fige la
révision utilisée dans la tentative. Les tests M1/M2 déjà exécutés gardent leurs
sources et limites ; ils ne prouvent pas une généralisation entre missions.

Les méthodes communes fournies par `agentWorkflow` sont embarquées par le moteur.
Elles ne deviennent pas réinscriptibles par une sortie d’optimiseur. Le profil
projet peut fournir des consignes par rôle, mais sa présence ne démontre pas que
les retours d’exécution y sont transformés automatiquement en procédures.

Une expérience dit : « dans ce cas, cette action a produit cet effet ». Une
procédure candidate dit : « lorsque ces conditions sont réunies, suivre cette
méthode ». Le passage de l’une à l’autre doit conserver sa justification et sa
portée, puis être contrôlé sur d’autres cas. Une consigne écrite ou transmise ne
prouve pas son bénéfice.

### Exigences procédurales ajoutées au cadrage

| ID | Déclencheur et effet attendu | Preuve nécessaire |
| --- | --- | --- |
| MEM-U10 Proposer une procédure | Un retour explicite ou un incident vérifié révèle une méthode à améliorer : proposer une addition/modification avec conditions et destinataires. | Trajectoire/retour original, version de base, motif, portée ; absence de mise à jour représentable sans instruction vide ni décision inventée. |
| MEM-U11 Contrôler l’évolution | Une procédure candidate est examinée : comparer l’ancienne et la nouvelle sur le cas initial et les variantes de non-régression avant adoption durable. | Décision du rôle compétent, version effectivement reçue, effet observé, conflits et retour arrière ; aucune mutation des droits, critères ou budgets par le texte appris. |

Les cas proposés `MEM-C11` à `MEM-C14` complètent la recette : retour devenu
consigne, absence de changement utile, retour hostile/hors périmètre, régression
et retour arrière. Tous restent `NOT TESTED` pour la future fonction produit.

### Conséquence sur l’ordre du plan

Avant réalisation, comparer deux premiers parcours sur les acquis Swarm :

1. rappeler une expérience RETEX dans une autre mission ;
2. transformer un retour sourcé en procédure candidate, puis la transmettre au
   bon rôle dans un contexte ultérieur et mesurer son effet.

Le premier parcours reste utile ; il n’est plus présenté comme couvrant à lui
seul l’apprentissage mémoire. La préférence technique est de réutiliser les
révisions contrôlées de Swarm pour l’application des consignes et de traiter
séparément leur admission durable. Une intégration LangMem, une recherche
vectorielle ou une réécriture libre du prompt moteur ne sont pas justifiées
automatiquement. Priorité, règles d’admission et protocole de comparaison doivent
être examinés avant M3.

## Complément M2 : guide pratique LangGraph fourni par l’utilisateur

### Document effectivement visé et récupération de méthode

Le texte joint par l’utilisateur contient 1 757 lignes, porte le titre `Memory`
et référence le source `src/oss/langgraph/add-memory.mdx`. Il correspond au
[guide pratique LangGraph](https://docs.langchain.com/oss/python/langgraph/add-memory),
distinct du guide conceptuel `concepts/memory`. Ce guide pratique avait été
ouvert parmi les liens, sans analyse complète de ses sections dans le cadrage.

Les réponses précédentes ont répété une conclusion sur l’identité de l’URL
conceptuelle reçue alors que l’utilisateur indiquait un autre document. Cette
vérification de l’URL ne résolvait pas l’identité du contenu visé. Le texte fourni
est la nouvelle preuve discriminante. Responsable de cette erreur de méthode :
Codex. Prévention : en cas de désaccord sur le contenu, comparer d’abord titre,
sections et texte reçu, au lieu de répéter une conclusion fondée sur l’URL seule.
Les réponses et constats historiques restent conservés ; la distinction des
documents est désormais explicite.

Le texte fourni, le guide officiel récupéré et leurs empreintes sont conservés
dans `evidence/m2-howto-20261015/`. L’analyse ci-dessous recense les sections
fonctionnelles et inspecte les chemins de code discriminants. Elle ne certifie
pas tous les exemples de bases externes ni chaque variante synchrone/asynchrone.

### Ce que cette référence ajoute concrètement

| Section du guide pratique | Contrat à examiner pour Swarm | Limite de cette itération |
| --- | --- | --- |
| Mémoire courte et checkpointer de production | Identifier l’état nécessaire à une reprise, sa persistance et les identités de conversation/tentative ; vérifier le redémarrage et les effets déjà appliqués. | Les exemples InMemory et les configurations de bases ne prouvent pas une restauration de processus fournisseur ni l’idempotence de ses outils. |
| Checkpointer dans les sous-graphes | Distinguer partage du mécanisme de stockage et exposition du contexte entre responsables et enfants. | Propagation LangGraph décrite par le guide ; ne crée aucun droit de délégation ou d’accès dans Swarm. |
| Store durable et accès par Runtime | Rappeler des connaissances entre conversations à partir d’une identité de projet/rôle contrôlée. | Un namespace fourni par l’appelant n’est pas une authentification. Les exemples de stockage ne qualifient pas l’extraction des faits. |
| Recherche sémantique | Comparer la pertinence avec la référence déterministe, les filtres de portée et la traçabilité des résultats. | Aucun modèle d’embeddings appelé ; aucune base vectorielle imposée. |
| Réduction des messages | Construire une projection adaptée au modèle sans confondre omission de contexte et suppression d’originaux. | Test local de la projection ; pertinence et qualité LLM non évaluées. |
| Suppression de messages | Définir ce qui change dans l’état courant et vérifier la validité des séquences message/appel/résultat d’outil. | Retirer un message courant ne prouve pas son effacement des anciens checkpoints. |
| Résumé progressif | Conserver résumé, messages récents, provenance et lien aux originaux ; préserver exigences, décisions et erreurs non résolues. | Le code LangMem est inspecté ; fidélité du résumé et gestion d’une panne de modèle non exécutées ici. |
| État courant et historique des checkpoints | Permettre d’expliquer quel contexte et quelle version ont servi à une tentative. | Consultation d’un historique distincte de reprise réussie ou d’acceptation. |
| Suppression d’un thread | Distinguer checkpoints, connaissances du store, archives et preuves de mission ; préciser droits et effets avant toute fonction de purge. | Sonde locale seulement ; aucune donnée Swarm réelle supprimée et aucune nouvelle politique de rétention adoptée. |
| Initialisation et migrations | Vérifier sauvegarde/réouverture et compatibilité de la représentation retenue. | PostgreSQL, MongoDB, Redis et Oracle sont des variantes présentées ; elles ne justifient pas le remplacement du stockage SQLite Swarm. |

### Comparaison avec les producteurs et consommateurs Swarm

- `store.go`, action `checkpoint`, exige un résumé, conserve `Summary`, `Next`,
  les chemins `Memory` validés et l’état Git. Ce n’est pas, à lui seul, un
  enregistrement de tout l’état de conversation au sens du checkpointer du guide.
- `agent_dialogue.go` réutilise une session fournisseur entre tours ; un wrapper
  possédant déjà des tours enregistrés refuse un redémarrage implicite. Préserver
  cette garde et les consommations : un futur contrat de reprise doit démontrer
  sa continuité, pas seulement supprimer ce refus.
- `workerRecoveryContext` prépare une projection attribuée d’une tentative
  précédente de la même tâche. Elle est bornée, signale ses omissions et ne
  transforme pas les traces historiques en preuves fraîches. Son mécanisme est
  distinct d’une restauration intégrale de conversation.
- `contextManifest` conserve une empreinte du prompt et les références incluses
  ou exclues. `searchDialogue` effectue une recherche textuelle dans un travail
  précis ; cela ne prouve pas une recherche sémantique entre missions.
- `archive.go` conserve les pièces mémoire explicitement référencées et signale
  les absentes. Une modification du contexte courant ne doit pas être présentée
  comme ayant effacé ces archives ou changé leurs preuves.

### Vérifications nouvelles et leurs limites

Sources installées : LangGraph `1.2.15`, langgraph-checkpoint `4.3.0` et
langchain-core `1.6.11`, dans l’environnement isolé M1 conservé. Empreintes des
fichiers inspectés et sources Swarm dans le manifeste de cette itération.
LangMem reste fixé à `48e3c11f5bb527282c7d5339c6a87a0b35abccfc`.

Les **deux sondes LangGraph exécutées passent**, sans modèle ni base externe :

1. `trim_messages` donne au consommateur deux messages sélectionnés, alors que
   les quatre messages de l’état restent disponibles au checkpoint courant.
2. `RemoveMessage` retire un message de l’état courant ; un ancien checkpoint
   permet encore de lire l’original. `delete_thread` efface l’historique de ce
   thread dans `InMemorySaver`, sans effacer une connaissance de `InMemoryStore`.

Source du test, commande, sortie, dates et code retour :
`evidence/m2-howto-20261015/test_context_boundaries.py`, `context-boundaries.log`
et `context-boundaries.json`. Ces tests ne démontrent ni durabilité après arrêt,
ni purge complète de données, ni fonctionnement de la future mémoire Swarm.

Le chemin statique `langmem/short_term/summarization.py` associe le résumé aux
identifiants des messages et traite la frontière des appels/résultats d’outils.
`SummarizationNode` permet une sortie `summarized_messages` distincte de l’entrée
`messages`. Préparer ce contexte et détruire l’historique sont donc des choix de
contrat distincts ; la qualité du texte résumé reste à vérifier.

### Exigences supplémentaires proposées

| ID | Effet attendu | Preuve de sortie |
| --- | --- | --- |
| MEM-U12 Qualifier la reprise | Pour chaque reprise promise, identifier l’état restauré, le périmètre et la prochaine action ; conserver droits, limites et consommation, sans rejouer aveuglément un effet déjà appliqué. | Arrêt/reprise dans une fixture isolée, comparaison de l’état et des effets ; un refus prévu reste distinct d’une restauration réussie. |
| MEM-U13 Maîtriser le contexte | Réduire ou résumer le contexte selon le parcours choisi en préservant les éléments nécessaires, la validité des messages d’outils et la provenance. | Contenu réellement reçu, omissions explicites, originaux consultables selon le contrat ; fidélité du résumé à mesurer. |
| MEM-U14 Définir l’oubli | Une action de retrait, suppression ou purge indique précisément les états, checkpoints, connaissances et archives affectés ; elle contrôle les droits et préserve les preuves requises. | Comparaison avant/après de chaque stockage concerné ; aucun succès d’effacement global déduit d’un seul état courant vide. |

`MEM-C15` à `MEM-C18` précisent ces recettes ; ce sont des attentes produit
`NOT TESTED`, distinctes des deux sondes amont réussies. Toute nouvelle politique
de taille, résumé ou rétention suit MEM-R06 : source persistée commune et Admin,
API, CLI et Go cohérents. Les nombres des exemples LangGraph ne deviennent pas
des valeurs de politique Swarm.

Le socle à préciser avant réalisation comprend donc **état de reprise, projection
de contexte, connaissances durables et cycle de correction**, puis leur usage
pour faits, expériences et procédures. Ce guide complète le choix de premier lot ;
il n’autorise pas l’ajout implicite de tous les backends ou la suppression de
l’historique existant.
