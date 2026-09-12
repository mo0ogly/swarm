# Swarm : compagnon local de reprise

Compagnon Go pour conserver des travaux entre sessions Claude/Codex.
Interface française, terminal 80 colonnes et Markdown ; sortie JSON pour outils.
Le cockpit peut lancer explicitement des agents Claude/Codex et suivre leurs processus.
`work list` et `resume` ne lancent aucun agent et ne choisissent pas le travail.
Voir [la procédure du cockpit](COCKPIT.md) pour le lancement et le pilotage.

## Construire et commencer

Go 1.24+ ; SQLite embarqué via `modernc.org/sqlite` (sans cgo), versions figées
par `go.mod` / `go.sum`. La compilation initiale télécharge les dépendances ;
le binaire construit fonctionne hors ligne, sans Python ni serveur SQLite.
Depuis ce dossier :

```sh
go test -race ./...
mkdir -p bin
CGO_ENABLED=0 go build -trimpath -o bin/swarm .
```

Depuis la racine du projet :

```sh
./tools/swarm-companion/swarm init
./tools/swarm-companion/swarm work list
./tools/swarm-companion/swarm work create --input work.json
./tools/swarm-companion/swarm resume IDENTIFIANT
```

`--root CHEMIN` cible une autre racine ; il ne fait pas de recherche implicite
vers le dépôt Git parent. Copier seulement `bin/swarm` suffit sur une machine
Linux de même architecture. Les autres plateformes doivent être compilées et
testées séparément. Aucun hook ne compile ni n’accède au réseau.

## Requêtes versionnées

`--input -` lit stdin ; `--json` produit du JSON sans texte décoratif.
Les champs communs aux mutations métier sont `schema_version: 1`, un `event_id`
unique et `expected_revision` (0 à la création, révision courante ensuite).
La sortie contient le travail, l’historique, les gates recalculées et la fiche.
Les dates sont UTC. Les identifiants utilisent lettres ASCII, chiffres, `_`, `-`.

Création (`work.json`) :

```json
{
  "schema_version": 1,
  "event_id": "creation-api-1",
  "expected_revision": 0,
  "title": "Faire évoluer le contrat API",
  "objective": "Client et serveur utilisent le même schéma",
  "scope": "Contrat, client, serveur et tests associés",
  "criteria": ["Tests de contrat PASS sur les fichiers courants"],
  "next": "Définir la tâche de contrat"
}
```

`task add TRAVAIL` : ajouter `id`, `title`, `deliverable`, `criteria`,
`owner`, `depends` (liste optionnelle de tâches existantes), `next`.
`task update TRAVAIL` : `id`, `status`, éventuellement `owner`, `blocker`, `next`.
Quitter `running` exige `outcome`: `completed`, `failed` ou `interrupted`.
`submitted` exige `completed`. `blocked` exige un motif. Une réouverture de
`accepted` passe par `todo`, puis une nouvelle tentative et une nouvelle gate.

`checkpoint TRAVAIL` : `summary` obligatoire, `next`, `memory` (chemins relatifs
de fichiers existants). `ooda TRAVAIL` : `observation`, `orientation`, `decision`,
`owner`, `next` obligatoires ; `result` facultatif. Ne pas enregistrer de pensée
privée ni de transcription intégrale.

`gate TRAVAIL --input gate.json` attend un objet :

```json
{
  "schema_version": 1,
  "event_id": "gate-contract-1",
  "expected_revision": 4,
  "task_id": "contract",
  "phase": "delivery",
  "document": {"method_version": "2", "scope_id": "contract"}
}
```

L’objet `document` doit être complété avec `artifacts`, `domains`, `checks` et
`results` selon [le contrat méthode 2](../agent-workflows/references/scoring.md).
L’exemple ci-dessus montre l’enveloppe, **pas une preuve acceptable**.
`evaluate --input evidence.json --phase delivery` calcule sans persister.
Les phases sont `entry`, `validation`, `delivery`, `audit`.
Codes de sortie : 0 succès ; 1 évaluation calculée mais bloquée ; 2 erreur,
entrée invalide ou conflit. Une gate bloquée est persistée si son entrée est valide.

## Reprise et fiabilité

`work list` et `work show ID` sont des lectures. `resume` sans identifiant
présente la liste ; avec identifiant il régénère `.swarm/views/ID.md`.
Les mutations régénèrent également la fiche ; une erreur de rendu n’annule pas
une transaction déjà enregistrée. Relancer `resume` répare la vue.

La fiche présente objectif, contexte Git, tâches actives, livrables, preuves,
blocages, gates, prochaine action, dernières décisions OODA et mémoire utile.
Qualité, progression des critères et nombre de tâches acceptées sont distincts.
Une donnée absente reste « non renseigné » ; la prochaine action n’est pas inventée.
Une tâche enregistrée en cours n’est jamais affichée comme processus vivant.

SQLite est la source canonique : état et événement sont écrits dans une même
transaction, avec contrôle de révision et déduplication des événements.
Les fichiers Markdown sont des vues dérivées datées/révisionnées : les relire
via le moteur pour obtenir l’état courant. Les preuves sont revérifiées à la
reprise et à l’acceptation ; une empreinte valide établit l’identité d’un fichier,
pas la vérité de son contenu ou l’exhaustivité du périmètre.

`.swarm/` est ignoré par Git, y compris dans un projet indépendant via son propre
`.gitignore`. Les fichiers de code, hooks et procédures restent versionnables.
Ne pas y stocker des secrets ou des conversations brutes. L’espace est local,
pas un service partagé ; chaque worktree a son propre espace dans cette version.

## Transport et sauvegarde

```sh
./tools/swarm-companion/swarm export ID --output /tmp/travail.zip
./tools/swarm-companion/swarm --root /autre/projet init
./tools/swarm-companion/swarm --root /autre/projet import --input /tmp/travail.zip
```

L’export logique lit état et événements dans une transaction SQLite cohérente.
Il embarque les fichiers explicitement référencés comme preuves ou mémoire, avec empreintes,
et signale ceux qui manquent. Il ne copie pas tout le code ni les fichiers ignorés.
Une preuve modifiée fait échouer l’export au lieu de certifier la nouvelle version.
L’archive est limitée à 128 Mio décompressés ; les entrées JSON à 16 Mio.

L’import garde les identifiants et refuse collisions, versions inconnues,
chemins d’archive arbitraires et pièces altérées. Les pièces sont archivées dans
`.swarm/imports/ID/` ; elles n’écrasent jamais les fichiers du projet cible.
Les gates sont revérifiées contre ce projet : absence ou différence = non valide.
Aucune fusion d’historiques divergents et aucune synchronisation Git automatique.

## Workflows et tests

Lire [WORKFLOWS.md](WORKFLOWS.md) pour les points d’enregistrement APEX/PDCA/KS.
La racine Wattson possède un hook Claude et des instructions Codex. Leur présence
ne prouve pas que chaque fournisseur les a exécutés : voir `VALIDATION.md`.

```sh
go test -race ./...
go vet ./...
go build -o /tmp/swarm-companion .
SWARM_BINARY=/tmp/swarm-companion python3 tests/parity.py
```

Python intervient seulement dans les tests différentiels, comme oracle de la
méthode existante. Le compagnon distribué n’en dépend pas.


## Assistant IA de chaque page

Le cockpit web propose un assistant sur Dialogue IA/APEX, Tâches, Agents,
Décisions, Journaux, Reprise/OODA et Budget. Sélectionner un élément, puis
« Poser une question sur cette page ». La modale montre le contexte et le prompt
exacts avant confirmation. Le moteur reconstruit les faits côté serveur ; le
navigateur transmet seulement les coordonnées de la vue.

Les faits cités sont consultables et les actions proposées ouvrent les formulaires
habituels. Une réponse périmée conserve ses références mais ne permet plus d’ouvrir
une action. Le contrôle de structure et de références ne garantit pas la vérité
d’une interprétation. L’assistant ne valide ni tâche ni gate.

Claude et Codex disposent d’adaptateurs sans outils de réalisation, dans un
répertoire isolé, avec délai maximal de 300 secondes et annulation explicite.
`assistant_timeout_seconds` dans la configuration du fournisseur règle ce délai
entre 1 et 300 secondes. Les fournisseurs inconnus, dont l’adaptateur GLM actuel,
ne peuvent pas répondre à ces questions ; leur lancement comme agent de travail
reste disponible. Un refus reste explicite, sans relance silencieuse.

Questions, réponses/refus, contexte, prompt et usage déclaré sont conservés dans
SQLite (schéma 4), par travail, avec export JSON complet et inclusion dans l’archive.
Seuls les 40 derniers échanges apparaissent dans l’historique courant. Les budgets
comptent aussi les réservations des questions ; ils ne constituent pas une facture.

Conception et preuves : `docs/plans/swarm-page-assistant/` à la racine du projet.
Procédure illustrée : `docs/procedures/procedure_complete_swarm_claude_codex.docx`
(v7, 39 pages ; schémas Mermaid dans `docs/procedures/diagrams/`).

## Fournisseurs et niveaux de modèles

Le menu **Administration fournisseurs** règle trois niveaux par fournisseur,
avec modèle et effort explicites. Le modèle résolu apparaît avant lancement et
reste enregistré avec la tentative. Les questions de page utilisent par défaut
le niveau simple, les travaux et le dialogue APEX le niveau standard. Le niveau
exigeant est un choix explicite ; aucun repli automatique après un refus.
La politique proposée pour Codex utilise Luna, Sol puis Astra. Skynet conserve
son modèle configuré aux trois niveaux, avec sa facturation déclarée sur site.

La même résolution sert les lancements JSON (`level`: `auto`, `simple`,
`standard`, `exigeant`), le web et le terminal. `model_policy_hash` lie un lancement
à la politique montrée dans l’aperçu. Le catalogue est local et l’accès se teste
explicitement depuis une carte fournisseur ; ouvrir l’administration n’appelle
aucune IA. Export/import ne transporte ni secrets ni commandes.

Procédure, limites et relation avec l’administration ML :
[Administration des modèles](../../docs/plans/swarm-provider-routing/ADMINISTRATION.md).
