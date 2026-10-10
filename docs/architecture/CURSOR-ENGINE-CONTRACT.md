# Contrat du moteur : références Cursor de janvier et février 2026

[English](../en/CURSOR-ENGINE-CONTRACT.md)

[Évolution APEX lancée le 10 octobre : architecture, lots et étude eBPF](CURSOR-ENGINE-EVOLUTION-20261010.md).

Références retenues : [Cursor — Scaling long-running autonomous coding](https://cursor.com/blog/scaling-agents), 14 janvier 2026, et [Towards self-driving codebases](https://cursor.com/blog/self-driving-codebases), 5 février 2026.

Clarification utilisateur du 10 octobre 2026 : les deux publications sont retenues. La correction précédente avait exclu février à tort. Janvier reste la référence des exigences JAN-1 à JAN-7 ; février complète l’architecture et explicite son évolution. Les différences entre versions doivent être documentées, sans déclarer une conformité par addition de leurs descriptions. Cette correction documentaire ne modifie pas le moteur.

Ce document distingue une règle de conception, son contrôle par le moteur et sa
preuve d'exécution. « Conforme Cursor » n'est pas une certification : l'article
décrit une expérience de recherche, avec des compromis. Les tests déterministes
de Swarm ne prouvent pas à eux seuls qu'une mission réelle se termine sans aide.

## Répartition des responsabilités

| Rôle | Responsabilité | Effet autorisé |
|---|---|---|
| Responsable racine | Possède l'ensemble du besoin ; décompose et adapte le travail. | Décisions structurées de planification, aucune commande de codage. |
| Responsable de périmètre | Possède la partie explicitement déléguée ; peut déléguer à son tour dans les bornes. | Tâches et décisions dans son périmètre, aucune commande de codage. |
| Exécutant | Réalise une tâche bornée dans sa copie du dépôt. | Modifications locales et remise au responsable propriétaire. |
| Vérificateur indépendant | Examine le résultat et les preuves sur la révision candidate. | Avis motivé, sans outil d'écriture ni pouvoir d'autoacceptation. |
| Contrôleur déterministe | Exécute les commandes de contrôle et applique les règles de publication. | Reçus, refus et transition atomique si toutes les conditions sont réunies. |

La publication de janvier sépare planificateurs et workers, autorise des sous-planificateurs récursifs et prévoit un juge en fin de cycle, puis un démarrage frais de l’itération suivante. La revue indépendante d’un candidat dans Swarm ne remplace pas ce juge global. Les garanties supplémentaires de preuve et d’acceptation de Swarm doivent être distinguées de ces exigences. L’absence d’un mécanisme démontré de jugement global et de renouvellement des cycles reste un écart à traiter, pas une dérogation autorisée.

Les copies isolent les modifications et les références Git. Elles ne constituent
pas une frontière de sécurité contre un processus malveillant disposant des droits
du compte hôte. Les contrôles décrits ici s'appliquent aux opérations publiques du
moteur ; un administrateur capable de modifier sa base ou ses exécutables reste
hors de cette garantie.

## Exigences de la référence de janvier à vérifier

Cette matrice définit la cible ; elle ne décrit pas des fonctions déjà livrées.
Les plafonds locaux et les garanties de Swarm ne sont pas des prescriptions de
Cursor. Toute preuve doit distinguer examen du code, tests simulés et exécution
avec un fournisseur réel.

| ID | Comportement attendu | Preuve attendue | État de cet audit |
|---|---|---|---|
| JAN-1 | Les planificateurs explorent continuellement le code et créent les tâches. | Montrer les informations du dépôt réellement accessibles au planificateur et une adaptation fondée sur une découverte nouvelle. | Partiel : décisions événementielles présentes ; exploration continue du dépôt non démontrée par un prompt sans outils. |
| JAN-2 | La planification peut être récursive et parallèle. | Observer deux sous-planificateurs actifs simultanément, propriétaires de périmètres distincts, puis leurs décisions persistées. | Délégation présente ; parallélisme réel des sous-planificateurs non démontré dans cet audit. |
| JAN-3 | Les workers terminent leurs tâches sans coordination directe entre workers et publient leurs changements. | Observer deux tâches indépendantes en parallèle, les refus des canaux latéraux et la remontée de leurs résultats. | Tests ciblés réussis ; chemin d’intégration central de Swarm à distinguer du fonctionnement décrit par Cursor. |
| JAN-4 | Un juge de fin de cycle décide si le travail doit continuer. | Verdict portant sur l’objectif et les résultats du cycle, avec décision persistée de poursuite ou de fin. | Non démontré : une revue de candidat ou une clôture déterministe ne suffit pas. |
| JAN-5 | L’itération suivante démarre avec un contexte frais. | Observer le passage entre deux cycles, le nouveau contexte et la conservation de l’objectif, des preuves et des limites. | Partiel : projections fraîches par activation ; transition globale entre cycles non démontrée. |
| JAN-6 | La coordination permet d’augmenter le débit sans attente centrale excessive. | Mesurer temps de calcul, attente des verrous, contrôles, revue, publication et débit pour plusieurs niveaux de concurrence. | Non mesuré ; plafond de 16 workers par mission et voie de publication sérialisée identifiés. |
| JAN-7 | Les prompts et le choix de modèle conviennent à chaque rôle. | Examiner les prompts effectifs et comparer les comportements sur des missions longues avec modèles et coûts identifiés. | Routes distinctes présentes ; adéquation et endurance non démontrées. |

Les tests ciblés de coordination réussis le 10 octobre 2026, y compris avec
`-race`, ne couvrent pas à eux seuls JAN-1 à JAN-7. Une absence de preuve ne
prouve pas une absence de fonction ; elle interdit de déclarer l’alignement
complet. L’audit du moteur reste à compléter sur ces points.

## Apports de février et articulation des deux références

Février décrit une hiérarchie continue, des copies propres aux workers et des remises au propriétaire. Il retire le juge dans une évolution intermédiaire et l’intégrateur central. JAN-4 et JAN-5 restent des exigences Swarm conservées dans ce cadrage ; elles ne sont pas présentées comme des invariants du système final de février. Comparer une hiérarchie continue avec des points de jugement et des cycles explicites avant de choisir leur articulation.

| ID | Complément à vérifier | Preuve attendue | État |
|---|---|---|---|
| FEB-1 | Propriété récursive et remise au responsable | Retour durable, propriétaire exact, réactivation après redémarrage | Partiel : mécanismes présents, recette réelle à compléter |
| FEB-2 | Remise contenant limites et découvertes | Contenu réel exploité dans une décision suivante | Non démontré |
| FEB-3 | Fraîcheur pendant le travail continu | Renouvellement observé sans perte d’objectif ni de preuves | Non démontré |
| FEB-4 | Débit et compromis de qualité explicites | Mesures et politique distincte de publication validée | Non mesuré ; contrôles Swarm maintenus |

## Règles à vérifier dans le moteur

| Invariant | Refus attendu | Preuve à conserver |
|---|---|---|
| Un planificateur ne code pas dans une mission hiérarchique | Un départ explicitement demandé avec rôle planner/subplanner est refusé, jamais transformé silencieusement en worker. | Réponse CLI et HTTP, absence d'agent, de réservation et de copie résiduelle. |
| Délégation exclusive | Une exigence déjà confiée ne peut être réattribuée ou traitée par le mauvais responsable. | Décision rejetée, révision et événements non consommés. |
| Isolation des exécutants | Deux tentatives n'utilisent pas une même copie pour écrire. | Chemins, références et contenu observés sur deux copies réelles de test. |
| Remise au propriétaire | Périmètre étranger, ancienne tentative, fichier d'une autre copie ou empreinte incorrecte sont refusés. | Même état avant/après refus ; retour valide reçu une seule fois par son propriétaire. |
| Réactivation | Un retour courant réveille son responsable ; le rejeu ne duplique pas l'événement. | Réouverture du Store, identités de tentative et d'événement conservées. |
| Clôture descendante | Un parent ne se ferme pas avec enfant ouvert, tâche inachevée ou preuve périmée. | Refus de clôture sur chaque cas et contrôle des preuves fraîches. |
| Acceptation indépendante | Contrôles réussis sans avis favorable, avis périmé, inconnu ou défavorable ne suffisent pas. | SHA candidat, tentative, contrat, contexte de revue et reçus empreintés. |
| Reprise bornée | Quota connu, limite atteinte ou appel déjà réservé ne donnent pas droit à une tentative supplémentaire. | Compteurs et historique inchangés, attente durable et motif explicite. |

Les détails du quota et de sa levée explicite sont dans le
[guide des attentes fournisseur](../PROVIDER-QUOTAS.md).

Les anciens lancements manuels sans organisation hiérarchique restent compatibles.
Ils ne constituent pas une mission autonome démontrant ces propriétés ; le départ
autonome exige l'organisation complète.

## Ce qui reste à démontrer

1. **Qualité des remises.** Le contrat courant transporte des constats et des
   artefacts. Il ne rend pas encore obligatoires des champs séparés pour changements,
   limites, écarts et décisions attendues. Un bon routage ne prouve pas cette qualité.
2. **Autonomie réelle.** Il faut observer un besoin, une délégation, une production,
   un refus motivé, une correction et une clôture avec un fournisseur réel. Les
   réponses simulées et les interventions de l'hôte sont comptées séparément.
3. **Reprise visible.** Après une levée manuelle d'attente sans échéance, les traces
   historiques doivent rester consultables sans être présentées comme un nouveau
   refus actif. L'autorisation d'un nouvel essai ne garantit pas la disponibilité.
4. **Débit et concurrence.** Mesurer le verrou d'intégration pendant contrôles et
   revue, ainsi que la taille du contexte cumulatif. La sérialisation actuelle et
   le refus de contexte trop grand doivent rester visibles.

## Définition de « terminé »

Une sortie normale du processus ne suffit pas. Pour un résultat accepté, conserver
la révision vérifiée, les commandes réellement exécutées avec dates et codes de
sortie, le rapport, l'avis indépendant, la décision, la fraîcheur des preuves, les
livrables et le nombre d'interventions externes. Un correctif écrit par l'hôte peut
renforcer Swarm ; il ne doit pas devenir une réussite autonome inventée dans la
mission qui l'évalue.

## Taille des preuves et transport du dossier

La limite du prompt de revue reste de 192 Kio, consignes comprises. Le moteur
conserve le diff depuis la base, les rapports, les reçus et les sources du même
candidat. Il réduit d'abord les lignes inchangées autour des modifications et
retire uniquement les sources nouvelles déjà reproduites intégralement dans le
diff, après comparaison exacte.

Si l'échappement JSON dépasse encore la limite, les textes sont transportés en
blocs UTF-8 intégraux, identifiés par champ, nombre d'octets et SHA-256. Les autres
métadonnées restent en JSON. Le contexte canonique conservé sur disque et son
empreinte ne changent pas. Aucun résumé ne remplace une preuve, aucun changement
antérieur n'est retiré du diff. Si ce transport dépasse encore le plafond, la
revue reste bloquée avant tout appel fournisseur. Les tests reconstruisent le
dossier dans un processus distinct et comparent tous ses champs au contexte
conservé ; ils vérifient aussi le refus d'un dossier réellement trop grand.

Un rapport nouveau déjà reproduit intégralement dans le diff peut aussi être
référencé à cet endroit : le moteur exige une identité exacte du texte, et le
transport conserve sa taille et son empreinte. Un rapport modifié ou partiellement
présent reste joint en entier. En cas de refus, le journal indique la taille
réelle et la part occupée par les consignes, sans exposer le contenu.
