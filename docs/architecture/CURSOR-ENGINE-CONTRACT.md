# Contrat du moteur : alignement avec le design final de Cursor

[English](../en/CURSOR-ENGINE-CONTRACT.md)

Source : [Cursor — The final system design](https://cursor.com/blog/self-driving-codebases#the-final-system-design), consultée le 19 septembre 2026.

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

Les trois premiers rôles reprennent le design final de Cursor. Le vérificateur
indépendant obligatoire est une exigence propre à Swarm : Cursor a retiré son
juge dans l'évolution décrite. Swarm exige une révision verte avant acceptation ;
Cursor décrit une tolérance à des erreurs transitoires pour augmenter le débit.

Les copies isolent les modifications et les références Git. Elles ne constituent
pas une frontière de sécurité contre un processus malveillant disposant des droits
du compte hôte. Les contrôles décrits ici s'appliquent aux opérations publiques du
moteur ; un administrateur capable de modifier sa base ou ses exécutables reste
hors de cette garantie.

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
