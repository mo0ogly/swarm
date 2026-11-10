# Reference guide — Pour définir actions, graphe, décisions et validations

Les chemins et commandes conservent leur contexte de travail d’origine.

## Définir le modèle

Partir du résultat utilisateur et de ce qui doit être préservé. Choisir un sujet
et une intervention utiles (créer, faire évoluer, corriger ou migrer), sans
reproduire une quantité arbitraire de modèles ou un catalogue concurrent.

Pour un modèle de travail, préciser au minimum :

| Élément | Ce qu’il faut rendre observable |
| --- | --- |
| Entrées | Données exigées, paramètres, inconnues, préconditions et périmètre |
| Actions | Effet attendu, responsable, sorties et critères identifiables |
| Groupes | Regroupements lisibles ; conserver les IDs des actions partagées |
| Flèches | Prérequis réels et orientations de décision, avec leurs conditions |
| Validations | Contrôles déterministes, revue, décision humaine, effet d’un refus |
| Reprise | Erreur, arrêt, nouvelle précondition, idempotence si effets répétés |
| Fin | Critères du modèle remplis et effets externes confirmés si applicables |

Utiliser le format public existant, avec des identifiants stables. Garder les
paramètres opérationnels configurables dans les mécanismes du produit ; les
valeurs par défaut et les limites doivent rester visibles et explicables.

Pour le développement : réutiliser les cinq fondations, instancier le cycle de
six étapes par fonctionnalité, partager les tâches communes sans duplication.
Justifier les étapes non applicables. Les noms de groupes ou phases n’autorisent
pas à eux seuls une transition ou un départ d’agent.

Pour un processus métier : séparer définition versionnée, dossiers/exécutions et
supervision. Identifier les habilitations et effets externes avant de proposer
une exécution ; une fixture de facture ne constitue pas un paiement bancaire réel.

