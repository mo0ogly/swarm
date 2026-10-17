# M18 — sources Swarm liées à la référence de résumé

16 octobre 2026, reprise « next ». Les quatre lots de
[l'incrément 14](INCREMENT-MEMOIRE-14.md) sont réalisés et vérifiés localement.
La vraie CLI et l'API Swarm produisent des sources consultables par l'opérateur ;
un convertisseur les relie à la référence extractive M17. Aucun résumé n'est
admis dans un contexte d'agent et aucun appel de modèle réel n'est ajouté.

## État observé et décision

Le dialogue interactif conserve questions, sessions, statuts et checkpoints dans
`agent_dialogue_turns`, mais pas toutes les réponses. Les journaux bornés ne
permettent donc pas de promettre une conversation complète. Les échanges de
cadrage possèdent question et réponses dans `Work.Tasks`, produits par
`agents_store.go` et consultés par `dialogueHits` / `resolveDialogue` dans
`dialogue_context.go`. Cette source effectivement persistée est retenue.

Lire SQLite directement en Python aurait créé un second lecteur du contrat
privé. Le choix est une opération commune Go, exposée par CLI/API, puis une
conversion explicite hors moteur. Ni reconstruction des messages du fournisseur,
ni admission automatique ne sont nécessaires pour ce lot. Codex reste seul
responsable de l'implémentation et des contrôles, sans revue indépendante.

## Contrat livré

`memory summary-source --input requete.json` et POST authentifié
`/api/v1/project-memory?action=summary-source` partagent le même lecteur : schéma 1,
travail, tâche cible, révision attendue et `DialogueRef` explicitement choisies.
Une seule ligne SQLite fournit révision et `Work`, afin que contrat courant et
échanges appartiennent au même instantané. Identité et révision de la ligne sont
comparées au corps conservé. Une tâche de cadrage ne peut pas être la cible.

Le paquet `swarm_summary_source_v1` contient une chaîne JSON `payload` et le
SHA-256 de ses octets UTF-8 exacts. Son contenu comprend le projet, la portée,
la révision, le cadrage du travail, l'instantané complet de la seule tâche cible,
puis les questions/réponses choisies et leurs références. Chaque texte possède
une empreinte. L'identité du projet est aussi incluse dans le texte de
`current-task-snapshot` : changer de projet ne donne pas une filiation implicite.
La révision générale reste hors de ce texte ; un changement sans effet sur les
sources choisies ne change pas artificiellement ce préfixe.

Le destinataire est `local_operator`, avec `provider_history_complete: false`.
L'instantané peut contenir des références et verdicts historiques de cette tâche,
sans leur attribuer une validité actuelle. Aucun rapport voisin, résultat d'outil
ou checkpoint privé n'est chargé. Les textes peuvent déjà avoir été bornés par
leur producteur ; l'export ne les raccourcit pas davantage. Il ne remplace pas le
contrat de construction d'un contexte exécutant.

Une révision périmée donne `revision_conflict` (CLI 3, HTTP 409). Référence
modifiée, doublon, réponse vide, tâche incorrecte et entrée mal formée sont
refusés sans écriture. GET sur cette action donne 405, sans retourner le catalogue.
Session, origine et CSRF du serveur restent requis. La commande d'inspection
refuse un stockage ancien avec `storage_upgrade_required`, sans le migrer.
La capacité de transport existante de la mémoire reste 1 Mio ; aucune politique
de déclenchement, taille de résumé ou rétention n'est créée.

`tools/memory-evaluation/swarm_source.py` vérifie paquet, textes, paires,
références et portée. Les annotations sont obligatoires, liées à l'empreinte du
paquet et protègent explicitement l'instantané de tâche. Les états sémantiques
restent choisis par l'opérateur ; leur justesse n'est pas déduite des empreintes.
Le convertisseur crée un nouveau dossier 0700 avec quatre fichiers 0600 : paquet
original, annotations originales, historique M17 et provenance. Il parse les
mêmes octets que ceux qu'il conserve. La provenance lie les fichiers et le
SHA-256 de l'historique cité par le reçu M17. Les sorties existantes sont refusées ;
une interruption d'écriture peut laisser un dossier partiel, sans préparation
réussie. L'outil de résumé M17 reste inchangé.

## Vérification et preuves

Les preuves sont conservées sous [`evidence/m18-20261016/`](evidence/m18-20261016/).
Les préimages distinguent ce lot des nombreux changements préexistants du dépôt.

| Critère | Résultat / preuve |
| --- | --- |
| M18.1 origine et schéma avant réalisation | PASS local ; premier test public rouge dans `source-red.json/.log`, choix et limites dans l'incrément. |
| M18.2 lecteur public et refus sans écriture | PASS ; six tests Go de source, dont quinze sous-cas de refus, parité CLI/API, session/origine et empreintes de l'état inchangées. `source-final.json/.log`. |
| M18.2 compatibilité de stockage | PASS ; l'inspection d'une fixture 34 conserve 34 et refuse l'ouverture. Contre-exemple avant correction dans `inspection-red.json/.log`. |
| M18.3 conversion, citations et provenance | PASS ; huit tests Python, dont vraie CLI, modes privés, sortie existante conservée, annotations périmées et sources altérées. `bridge-final.json/.log`. |
| Référence M17 inchangée | PASS ; ses 24 tests Python distincts restent verts. `reference-regression.json/.log`. |
| M18.4 parcours applicatif | PASS ; cinq contrôles de la vraie CLI/API et des outils Python, deux échanges de cadrage réellement produits par un fournisseur scripté. `native-flow/result.json` et originaux voisins. |
| Construction et contrôles | PASS ; `build-final.json`, `static-final.json` (`go vet ./internal/engine`), `config-final.json` et `audit-final.json` (diff, sources, liens, environnement). |

**10 tests Go distincts** passent : six tests de la fonctionnalité et quatre
contrôles existants des contrats mémoire/contexte/inspection. Les quinze sous-cas
de refus ne sont pas comptés comme quinze tests supplémentaires. **32 tests
Python distincts** passent : huit du convertisseur et 24 de la référence.
Les essais préliminaires ne s'ajoutent pas à ces nombres.

La recette finale construit son propre projet, lance deux cadrages via la vraie
CLI avec un fournisseur local scripté, ferme puis rouvre les lectures et compare
le paquet API authentifié au paquet CLI. Les cinq sources obtenues sont
l'instantané de tâche et deux paires question/réponse. La référence M17 conserve
la citation de l'intention future et l'incident ouvert avec leurs états déclarés.
Travail, agents, usages et nombre d'invocations restent inchangés par les lectures
et la conversion. Une modification des critères par `task update` rend la requête
ancienne périmée en CLI/API ; les anciennes annotations sont refusées avant
création du dossier de sortie. Le changement du contrat empêche la réutilisation
du parent avec la cause vérifiée `parent_or_sources_changed`. FALLBACK conserve
les cinq messages courants complets.

Le binaire de recette provient de `go build ... ./cmd/swarm`, empreinte
`1097db24113d8cedc2954813075623515b850b954f4ec5c3cc043da213ac79e1`.
La recette finale compte **deux invocations de script et zéro modèle réel**.
Deux recettes échouées, conservées et diagnostiquées, ont produit trois autres
invocations scriptées : cinq au total pour ce lot, toujours zéro modèle réel.
Le collecteur consigne les entrées et leur stabilité pendant chaque contrôle.
L'évaluation finale distingue les changements ultérieurs du seul script de
recette des entrées Go/Python réellement consommées ; les tests restés valides
ne sont pas relancés pour un changement documentaire ou un script non chargé.

Pas de nouvel état partagé mutable introduit : un seul instantané SQL, une
conversion locale et la classification d'ouverture existante. Aucun nouveau
résultat `-race` n'est revendiqué. Aucun écran modifié : pas de recette navigateur
ni de qualification visuelle nouvelle. Pas de test global Go, de fournisseur
réel, de migration réelle ou de déploiement. Le serveur utilisateur reste
inchangé, au PID 2186583 et à son adresse initiale.

Les docs [FR](../../PROJECT-MEMORY.md), [EN](../../en/PROJECT-MEMORY.md) et la
[recette](../../../tools/memory-evaluation/README.md#préparer-la-référence-depuis-des-sources-swarm)
décrivent commandes, format, refus, propriétaires et limites. La consultation
ne crée pas une politique d'exécution à ajouter dans Admin.

## Replan et RETEX

Trois frontières ont nécessité un diagnostic lié au besoin initial :

1. GET avec la nouvelle action renvoyait le catalogue 200. Le contre-exemple
   `method-red.json/.log` est conservé ; le garde commun refuse désormais cette
   méthode avant la branche catalogue. Le test et la vraie API vérifient 405.
2. L'ouverture CLI migrante intervenait avant le lecteur. Le nouveau nom de
   commande manquait dans `cliStorageInspection` : sur une fixture 34, le test
   rouge observait sortie 0 et version 35. Le classement commun d'inspection
   corrige cette cause ; le même test vérifie refus explicite et version 34.
   Aucun identifiant de mission ou contournement de migration n'est ajouté.
3. La première recette attendait une réponse avec `Agent.attempt_id`. Le producteur
   stocke `Agent.id` dans `BrainstormAnswer.attempt` ; l'agent avait terminé.
   Les originaux `native/failed-agent.json`, `failed-work.json`, l'identité de
   fixture et la préimage du script sont conservés. Le consommateur suit
   maintenant la clé réellement produite. La seconde recette échoue ensuite
   avant l'API : sa fonction locale `http` masque le module `http.cookiejar`.
   `native-final/failure.json` et sa préimage conservent cet échec de méthode.
   Le renommage en `api_request` corrige la précondition. La recette finale
   vérifie ensuite l'API et la chaîne entière ; aucun échec inchangé n'est relancé.

Les premiers fichiers supposés pour une lecture de résultat ou une recherche
de type étaient absents ; l'inventaire et la recherche des symboles ont fourni
les chemins réels. Ces erreurs d'inspection ne sont pas des défauts produit ni
des contrôles réussis. Les originaux d'échec sont conservés sans renotation.

Apprentissage réutilisable consigné dans la recette commune : suivre la référence
du producteur et tester l'ouverture d'une nouvelle inspection sur un stockage
ancien. Un corps de lecteur sans écriture ne prouve pas que son ouverture reste
sans migration. La documentation est destinée au mainteneur et à l'opérateur,
sans ajout de consignes aux agents ni prétention de changement de leur contexte.
La revue locale porte sur l'instantané SQL, les consommateurs, la provenance,
les refus, les copies et l'absence d'invocation induite ; elle n'est pas indépendante.

## Critères encore ouverts

MEM-C16 et MEM-C17 restent **PARTIAL** : les sources de cadrage Swarm sont
raccordées à une référence extractive, sans historique fournisseur complet,
proposition LLM, admission, remplacement de contexte ou consommation d'un reçu
par un agent. Les huit appels M6/M7 restent consommés et leurs résultats intacts.
La fidélité d'une paraphrase et la justesse des annotations restent non qualifiées.

La prochaine frontière est le contrat de proposition/admission et son consommateur
réel : autorité, sources revérifiées au moment d'usage, reçu lié au contexte
effectivement transmis, refus conservant les originaux. L'évolution devra définir
ses contrôles et éventuelles politiques Admin/API/CLI/Go avant activation ; aucun
droit ni nouvel appel réel n'est inféré du présent raccordement en lecture seule.
