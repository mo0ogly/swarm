# Parallélisme du moteur et dépendances du plan

Le 10 octobre 2026, l’utilisateur demande de poursuivre l’évolution Cursor et
d’améliorer le fonctionnement séquentiel. La mission existante
`w-ca937edf4e0abd85f8d3d67b` conserve ses identifiants et ses preuves.

## Observation

Le plan adopté impose T1 → T2 → T3 → T4 → T5 → T6 → T7. À la révision 51,
T1 est acceptée et T2 travaille dans sa copie isolée. Le nombre d’appels d’outils
indique de l’activité ; il ne mesure ni le débit de livraison ni l’acceptation.
Le graphe explique pourquoi les autres lots restent en attente, même si plusieurs
créneaux de workers sont disponibles.

## Décision

Deux options sont distinguées : supprimer immédiatement des dépendances du plan
actif, ou préciser les preuves de concurrence des lots déjà prévus. La première
exige une analyse des entrées et des écritures communes : T2 et T3 touchent
notamment `planning.go`, et T5 dépend des politiques des lots précédents.
L’opération publique actuelle refuse les changements de définition pendant une
tâche en cours ; le contrat hiérarchique géré impose aussi ses propres limites.
Retirer une flèche ne démontre donc pas que deux lots sont indépendants.

La seconde option est retenue maintenant : compléter les remises de T3, T6 et
T7 par `task update`, uniquement dans leur champ `next`. Cela précise leurs
critères existants sans modifier les dépendances, les critères, les contrôles,
les tentatives ou les budgets. Les événements publics sont conservés dans le
[relevé adjacent](20261010-parallelisme-plan.json). Cette intervention humaine
ne constitue pas une adaptation autonome du graphe.

## Preuves attendues

| Lot | Observable à livrer |
| --- | --- |
| T3 | Deux sous-planificateurs actifs simultanément sur des périmètres distincts ; découverte conduisant à une adaptation persistée du plan par une opération publique prise en charge. Justifier les dépendances par les entrées, les écritures et les preuves requises. |
| T4 | Distinguer travail parallèle et publication ; mesurer les attentes centrales et conserver la revue du candidat exact. |
| T6 | Comparer une exécution séquentielle et concurrente du même scénario : chevauchement réel, temps total, attente, livraisons acceptées, coût connu ou inconnu, conflits et reprises. Utiliser des racines temporaires. |
| T7 | Observer avec fournisseur réel deux travaux indépendants qui avancent ensemble ; conserver les événements de décision et de remise, ainsi que les preuves fraîches. Distinguer interventions humaines, simulations et autonomie observée. |

Un réglage de concurrence doit être persisté et exposé dans l’Admin FR/EN par
T5. Ni un nombre de rôles, ni un test avec doubles, ni une liste de tâches
indépendantes ne suffisent à démontrer le parallélisme en production.

Le plan de cette mission reste séquentiel après cette précision. Une révision
future de son graphe devra être justifiée et appliquée par les opérations
publiques effectivement disponibles, avec isolation et validations préservées.

Le [contrat d’architecture du parallélisme](../../../../architecture/AGENT-PARALLELISM.md)
et sa [version anglaise](../../../../en/AGENT-PARALLELISM.md) détaillent désormais
les besoins, contraintes, décisions, reprises et mesures avec six schémas Mermaid
par langue. Ils distinguent inspection du moteur, cible et preuve réelle.

## Vérification des suites Go

Le [relevé de l’audit Go](20261010-audit-suite-go.md) distingue l’échec du
manifeste historique du temps nécessaire au paquet complet. T6 doit reprendre
ce constat avec provenance : un timeout du paquet ne prouve pas qu’un test
individuel reste bloqué. Préserver le manifeste historique ; ne pas réécrire ses
empreintes pour fabriquer une qualification.
