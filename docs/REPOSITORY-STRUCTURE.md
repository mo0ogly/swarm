# Carte du dépôt Swarm

Swarm est une application Go qui fournit une CLI et une interface web locale
pour préparer, organiser et suivre des missions confiées à des agents IA.
Cette carte décrit l’organisation du code, des documents et des outils.

## Où travailler

| Chemin | Contenu |
| --- | --- |
| `cmd/swarm/main.go` | Point d’entrée exécutable ; appelle le moteur |
| `internal/engine/` | Code Go de la CLI, du serveur, du stockage, de l’orchestration et tests unitaires Go |
| `resources.go` | Intégration des ressources canoniques dans le binaire avec `go:embed` |
| `web/` | Interface HTML/CSS/JavaScript servie par le moteur |
| `frontend/` | Sources et chaîne de reconstruction frontend existantes |
| `locales/` | Catalogue de traduction |
| `config/` | Valeurs par défaut publiques ; les secrets restent hors Git |
| `contracts/` | Schémas et contrats de données |
| `tests/` | Harnais CLI, HTTP, navigateur et recettes d’acceptation |
| `testdata/` | Jeux de données utilisés par les tests Go |
| `tools/` | Vérification, méthodes d’agents et outils de développement |
| `scripts/` | Scripts auxiliaires, notamment traduction et captures |
| `deploy/` | Entrée du conteneur et fichiers de déploiement |
| `docs/` | Documentation, captures, plans et rapports |
| `docs/training/casa-pizza/` | Guides de formation FR/EN, captures et application de référence |
| `docs/en/` | Documentation en anglais |
| `benchmarks/` | Scénarios et outils de mesure |
| `bin/` | Binaires construits localement, exclus de Git |
| `test-results/` | Résultats générés par les contrôles, exclus de Git |
| `.claude/skills/` | Sources canoniques des méthodes ; `.agents/skills` les référence |
| `.github/` | Intégration continue et modèles GitHub |

La racine conserve les fichiers d’entrée habituels : README FR/EN, licence,
sécurité, contribution, instructions aux agents, installation, manifests Go/npm,
Dockerfile, Makefile et lanceurs `swarm`/`swarm.sh`.

## Repérer une partie du moteur

Le moteur reste un seul paquet Go. Les préfixes existants permettent de retrouver
les sujets sans créer artificiellement des paquets aux dépendances circulaires.

| Préfixe dans `internal/engine` | Sujet |
| --- | --- |
| `main`, `console`, `*_cli` | Commandes et rendu CLI |
| `web`, `prephase`, `preparation` | Serveur web et préparation |
| `agent`, `agents`, `provider`, `role_model` | Exécution, fournisseurs et modèles |
| `planning`, `conductor`, `supervision`, `pilotage` | Planification et conduite |
| `managed`, `workspace`, `recovery` | Intégration, espaces de travail et reprise |
| `validation`, `evidence`, `review`, `independent` | Contrôles, preuves et revue |
| `graph`, `automation` | Graphes et automatisation |
| `storage`, `model`, `migration` | Stockage et modèles de données |
| `*_test.go` | Tests Go placés avec le code qu’ils vérifient |

La migration de chemins ne constitue pas un découpage métier du moteur.
Les documents et preuves datés peuvent citer les anciens chemins Go à la
racine : pour une source actuelle, chercher le même nom sous `internal/engine`.
Les manifestes antérieurs restent des preuves de leurs propres candidats.

## Construire et vérifier

Depuis la racine du dépôt :

```sh
make build
go test -timeout 25m ./...
go vet ./...
npm test
make smoke
python3 tools/check_distribution.py
python3 tools/agent-workflows/check.py
```

Pour un seul groupe : `go test ./internal/engine -run '^TestVersion'`.
Pour un binaire de développement sans identité de release :
`go build -o bin/swarm ./cmd/swarm`. Le build versionné passe par `build.sh`.
Le paquet racine est désormais un paquet de ressources : `go build .` ne
construit plus l’exécutable. Aucun fichier Go ne doit être déplacé entre paquets
sans adapter les dépendances et les tests.

## Ressources et données privées

`go:embed` ne peut pas lire les répertoires parents de son paquet. C’est pourquoi
un seul fichier Go reste à la racine pour intégrer les ressources à leur
emplacement canonique. Il n’existe aucune copie générée à maintenir.

Les bases et secrets sous `.swarm`, les clés API, certificats, fichiers
`compose.override.yaml` privés et documents d’installation Skynet sont exclus
de la publication. Les données d’une mission ne sont pas des fixtures de test.
Une nouvelle compilation ne remplace pas le binaire d’un serveur déjà lancé.
