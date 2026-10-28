# M18 — sources Swarm inspectables pour la référence de résumé

16 octobre 2026, reprise « next ». Codex seul ; `replan` et APEX.
MEM-U08/U13, MEM-C16/C17 ; M17 conservé sans renotation.

## Décision et frontière

Les tables de tours du dialogue interactif conservent question, session, statut
et checkpoint, pas toutes les réponses. Les reconstruire depuis des journaux
bornés inventerait une complétude. Les échanges de cadrage dans `Work` conservent
question et réponse, résolus par `DialogueRef`. Cette seconde source est retenue.

Un export SQLite brut ajouterait un lecteur privé concurrent de l'autorité Go.
Le choix est une consultation publique commune CLI/API : révision attendue,
tâche cible et références explicites du même travail, lues dans un seul instantané.
La sortie contient le cadrage courant du travail, l'instantané de la seule tâche
cible et les questions/réponses choisies, avec empreintes et limites explicites.
Elle vise l'opérateur local ; ce n'est pas le contexte d'un exécutant délégué.

Une conversion Python vérifie le paquet, exige des annotations liées à son
empreinte et protège au moins l'instantané de tâche. Elle prépare le format M17
et conserve paquet, annotations et provenance dans un dossier nouveau. Aucun
état sémantique n'est inféré du texte. Les empreintes identifient les documents,
sans authentification ou autorisation d'admettre leur contenu.

| Lot | Propriétaire / dépendance | Critère vérifiable |
| --- | --- | --- |
| M18.1 | Codex ; M17 | Schéma de source, origine réellement persistée, critères ci-dessous et tests d'interface avant réalisation. |
| M18.2 | Codex ; M18.1 | `memory summary-source`, API locale commune, révision/scope/références contrôlées ; refus sans écriture, aucune source par défaut. |
| M18.3 | Codex ; M18.2 | Convertisseur avec citations exactes, paquet/annotations conservés et empreintes vérifiées ; annotations périmées, manquantes et paquet corrompu refusés. |
| M18.4 | Codex ; précédents | Vraies CLI/API et référence M17 dans une racine isolée, source réellement produite par le parcours de cadrage avec fournisseur scripté ; persistance et absence de nouvelle invocation vérifiées. Docs FR/EN, revue locale et RETEX. |

L'export ne joint ni rapports des tâches voisines, ni sorties d'outils, ni état
privé des gardes. Il ne promet pas un historique fournisseur complet ou un texte
plus complet que les réponses conservées. Il n'admet aucun résumé et ne modifie
ni contexte, révision, réservations ni compteurs. Pas de politique, migration,
interface nouvelle ou appel de modèle réel. Le consommateur moteur et la qualité
LLM restent ouverts ; les huit appels M6/M7 restent consommés.

## Résultat local

M18.1–M18.4 vérifiés : dix tests Go distincts, huit tests du convertisseur,
24 tests de non-régression M17 et cinq contrôles du parcours CLI/API réel dans
une racine isolée. Une modification publique des critères invalide les anciennes
annotations et le parent de résumé ; les originaux courants restent disponibles.
Deux contre-exemples produit (GET retournant le catalogue, ouverture d'inspection
migrante) et deux défauts de recette sont diagnostiqués et vérifiés après correction.
Voir [résultats et RETEX](RESULTATS-MEMOIRE-14.md). Codex seul, aucune revue
indépendante, aucun appel réel, résumé admis ou déploiement. Les critères globaux
MEM-C16/MEM-C17 restent partiels ; proposition/admission et consommation effective
constituent la frontière suivante.
