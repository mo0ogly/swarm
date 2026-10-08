# Résultat — réorganisation du dépôt Swarm

## Résultat et état

504 fichiers Go de la racine sont regroupés dans `internal/engine` ;
`cmd/swarm/main.go` appelle le moteur et `resources.go` intègre les ressources
canoniques. Compilation, installation native et retour arrière (deux cas D04 passés),
Docker, contrôles CLI/HTTP, npm et vet sont vérifiés ; la suite Go complète est
incomplète : la commande séquentielle a dépassé 25 minutes et trois groupes du
harnais ont atteint leur limite de 240 secondes. Une recette rééquilibrée doit
être rejouée avant publication finale.

## Identité et périmètre

- Session : Codex native, demande utilisateur « organise mon repo ».
- Dépôt : `/home/fpizzi/workspace/swarm` ; branche `codex/repository-organization`.
- Base : `a09ae41d09c8bd229a7b7b299184ad7c9ac14eaf` ; candidat : `a09ae41d09c8bd229a7b7b299184ad7c9ac14eaf+sha256:efe034c579fe69b57825b6e82000abd974dc6eff27c4369864bb876b002af024`.
- État : snapshot de réorganisation ; recette d’installation neuve et correction des méthodes en cours avant publication finale.
- Les travaux préexistants sont préservés ; aucun état de mission n’est modifié.
- La structure est organisée, mais le moteur reste un paquet : aucun découpage
  métier en plusieurs modules n’est présenté comme réalisé.

Le snapshot comprend les changements locaux d’installation, de session web et de
formation nécessaires à une livraison cohérente. Les travaux de benchmark et de
publication scientifique restent hors de ce commit.

## Constats et ajustements

Les directives `go:embed` ne peuvent pas remonter vers un parent. Un paquet de
ressources à la racine conserve les fichiers canoniques sans les dupliquer.
Les scripts de build, d’installation et les harnais doivent viser `./cmd/swarm` ;
les recettes de tests ciblées doivent viser `./internal/engine`.

Les 1 068 tests/examples/fuzz enregistrés sont identiques avant et après migration.
La partition du harnais reste complète et sans doublons, avec 19 groupes dont
3 isolés. Les tests Go restent avec le code pour conserver l’accès aux fonctions
privées. Les lecteurs de fixtures et commandes de tests utilisent explicitement
la racine du dépôt, sans changer le répertoire de travail des processus d’agents.

Un test DOM optionnel échouait déjà sur les fichiers exacts de `HEAD` : sa fixture
ne fournissait plus les scopes ni le constructeur de boutons. La fixture est
adaptée au contrat actuel ; les assertions existantes passent et la recette
exécute réellement deux tests Go. Le DOM est simulé ; ce n’est pas un navigateur.

Les manifestes antérieurs restent inchangés. Le nouveau manifeste est un snapshot
d’intégrité et marque les exigences historiques comme à requalifier : les anciennes
captures et revues ne certifient pas ce candidat.

## Exigences et vérification

| ID | Vérification exécutée | Résultat | Preuve et limite |
| --- | --- | --- | --- |
| ORG-01 | `go list ./...` ; `sh build.sh /tmp/swarm-repo-organization-aulsuzw3/swarm` | PASS | Trois paquets ; binaire identifié et un seul Go à la racine |
| ORG-02 | Build Docker ; conteneur sans réseau externe ; version, init, commande invalide, session, API et ressources web | PASS | Empreinte du JavaScript intégré identique au source ; image locale uniquement |
| ORG-03 | Inventaire enregistré ; 6 tests ciblés ; `npm test` ; `go vet ./...` | PASS | Aucun test Go perdu ; scripts et assertions exécutés |
| ORG-03 | `go test -json -count=1 -timeout 25m ./...` | TIMEOUT | Aucun test individuel en échec avant le dépassement ; suite incomplète |
| ORG-03 | Harnais exhaustif de 1 068 cas | TIMEOUT | 16 groupes terminés ; 3 groupes dépassent 240 s, sans échec fonctionnel antérieur |
| ORG-03 | `SWARM_TEST_BINARY=/tmp/swarm-repo-organization-aulsuzw3/swarm python3 -m unittest discover -s tests -p test_swarm_local.py` | PASS | 7 tests ; redémarrage, persistance et processus étrangers |
| ORG-03 | Harnais de campagne Python ; recette d’états DOM et Go ; recette CLI `tests/smoke.py` | PASS | Campagne : 4 tests passés et 1 skip opt-in ; pas de fournisseur IA réel |
| ORG-04 | `python3 tools/check_distribution.py` ; `python3 tools/agent-workflows/check.py` | PASS | Liens de documentation et 9 méthodes vérifiés ; cartes FR/EN |
| ORG-05 | Empreintes avant/après ; `git check-ignore` ; `git diff --check` | PASS | Aucun fichier hors périmètre modifié ; documentation Skynet et données privées exclues |

Les logs, inventaires et sauvegardes se trouvent dans
`/tmp/swarm-repo-organization-aulsuzw3`. Les contrôles Docker utilisent un
répertoire tmpfs isolé, sans volume de mission, avec `--network none`.
Le test CLI utilise des racines temporaires et ne lance aucun fournisseur IA.
Le premier probe Docker supposait à tort que `work list` renvoyait un objet :
il a été corrigé pour vérifier le tableau réellement retourné, puis réexécuté.
La suite préliminaire a été interrompue ; un premier run du candidat a été
relancé après correction d’un chemin absolu de fixture trouvé en auto-revue.
Ces exécutions interrompues ne sont pas comptées comme des succès.

## Revue, récupération et limites

Auto-revue dans la session de l’auteur, sans revue indépendante ni acceptation
par le moteur. La sauvegarde contient les sources avant déplacement, les fichiers
adaptés, le diff antérieur et les empreintes. Restaurer uniquement ce périmètre,
sans écraser les travaux apparus depuis ni toucher aux bases `.swarm`.

Le site pizza en cours d’utilisation reste lancé. Le serveur Swarm déjà lancé
utilise son binaire antérieur : les tests portent sur le candidat construit pour
les fixtures, sans prétendre avoir mis à jour le processus actif.
