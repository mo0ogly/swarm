# Réorganisation du dépôt Swarm — 8 octobre 2026

## Demande et état constaté

Le dépôt concerné est `/home/fpizzi/workspace/swarm`, branche `feat/billing-bench`,
base `a09ae41`. Il contient 504 fichiers Go à la racine, tous dans le paquet
`main`. Des modifications locales préexistantes et des fichiers non suivis sont
présents ; ils doivent être conservés. Les guides de formation se trouvent dans
`docs/training/casa-pizza`. Les données de mission et la documentation Skynet sont
privées et ne font pas partie de la réorganisation.

Les points d’entrée inspectés sont `main.go`, `build.sh`, `Makefile`,
`install.sh`, `Dockerfile`, `swarm`, `swarm.sh` et `.github/workflows/ci.yml`.
Dix fichiers Go intègrent des ressources avec `go:embed`. Plusieurs tests et
harnais lisent les sources directement ou compilent le paquet racine.

## Choix

1. Déplacer tout le paquet dans `cmd/swarm` : simple, mais mélange durablement
   l’entrée CLI, le moteur et ses tests.
2. Séparer `cmd/swarm` et `internal/engine`, avec un paquet de ressources à la
   racine : retenu. La CLI appelle un seul point d’entrée exporté ; les tests
   restent avec le moteur et gardent accès à ses fonctions privées. Un fichier
   `resources.go` intègre les ressources canoniques à leur emplacement actuel.
   Cela évite des copies générées et des liens symboliques dans le build Docker.

Cette étape organise le code sans changer les règles métier. Extraire le moteur
vers plusieurs paquets métier demande une étude distincte des dépendances et ne
fait pas partie de cette migration de chemins.

## Exigences et tâches

| ID | Effet attendu | Périmètre | Dépendance | Contrôle |
| --- | --- | --- | --- | --- |
| ORG-01 | Une entrée CLI courte et un moteur sous `internal/engine` | Go, ressources | Inventaire et sauvegarde | `go list ./...`, compilation CLI |
| ORG-02 | Les commandes de build et d’installation restent utilisables | Build, Docker, harnais | ORG-01 | Build versionné, installation native, contexte Docker |
| ORG-03 | Les tests découvrent les mêmes cas et les bons fichiers | Tests Go et scripts | ORG-01 | Inventaire avant/après, tests ciblés, suite Go, npm, vet |
| ORG-04 | La structure est compréhensible et les guides accessibles | README FR/EN, contribution, carte du dépôt | ORG-01 | Liens de distribution et méthodes |
| ORG-05 | Les travaux existants et les informations privées sont préservés | Tous les fichiers hors migration | Sauvegarde | Comparaison des empreintes, règles d’exclusion, diff final |

Responsable : session Codex native autorisée par la demande utilisateur.
Séquence : sauvegarder → déplacer et adapter → vérifier → corriger les écarts
liés à la migration → publier un compte rendu local. Aucun agent ni fournisseur
IA externe n’est lancé pour cette migration.

## Récupération et preuves

La sauvegarde avant migration est conservée hors du dépôt sous
`/tmp/swarm-repo-organization-aulsuzw3`. Elle contient les sources déplacées,
les fichiers adaptés, le diff antérieur et un inventaire des empreintes.
Le retour arrière doit restaurer uniquement ces fichiers et leurs chemins,
sans écraser les travaux apparus depuis ni toucher aux bases `.swarm`.

Les manifestes de qualification antérieurs restent historiques. Un nouveau
snapshot de chemins ne constitue pas une nouvelle acceptation des missions ni
une revue indépendante. Les résultats exécutés et les limites seront consignés
dans `docs/reports/repository-organization-20261008.md`.

Le serveur actuellement lancé utilise son binaire existant. Une compilation
réussie du candidat ne prouve pas que ce serveur a été mis à jour.

## Ajustement vérifié du harnais existant

La recette optionnelle `node tests/audit_acceptance.cjs --case states` échouait
avant cette migration : reproduction sur les trois fichiers exacts de `HEAD`
avec une erreur `p.scopes.find` dans `Planning.roles`. Le test DOM ne fournissait
ni les scopes de planification ni le constructeur de boutons maintenant utilisé.
Le périmètre ORG-03 inclut la mise à jour de cette fixture (`scopes: []` et
constructeur de boutons du DOM simulé). La recette réexécutée passe, avec les
assertions DOM existantes et deux tests Go réellement exécutés. Ce contrôle
utilise un DOM simulé ; il ne prouve pas un parcours navigateur.
