# Swarm moteur — contrat Cursor et vérification obligatoire

État : contrat à démontrer, jamais certification de conformité par déclaration.
Références retenues : [Cursor — Scaling long-running autonomous coding](https://cursor.com/blog/scaling-agents), 14 janvier 2026, et [Towards self-driving codebases](https://cursor.com/blog/self-driving-codebases), 5 février 2026.

Clarification utilisateur du 10 octobre 2026 : les deux publications sont retenues. La correction précédente avait exclu février à tort. Janvier reste la référence des exigences JAN-1 à JAN-7 ; février complète l’architecture et explicite son évolution. Les différences entre versions doivent être documentées, sans déclarer une conformité par addition de leurs descriptions. Cette correction documentaire ne modifie pas le moteur.

## Résultat attendu par l'utilisateur

Un lancement depuis Swarm doit constituer une équipe réelle : responsable du besoin, responsables de périmètre si nécessaire, exécutants dans leurs copies Git, vérificateur indépendant. L'utilisateur ne doit pas compenser les manques du moteur par une supervision Codex ou valider des résultats sans preuve. Une tâche terminée par son exécutant attend ses contrôles et sa revue avant de devenir acceptée. Une mission incomplète ne peut pas être close.

## Principes repris et décisions propres à Swarm

La publication de janvier décrit des planificateurs qui explorent continuellement le code et créent des tâches, avec des sous-planificateurs récursifs. Les workers terminent leurs tâches sans coordination directe entre eux, puis poussent leurs changements. Un juge décide en fin de cycle s’il faut poursuivre ; l’itération suivante démarre avec un contexte frais. Les copies isolées et les remises structurées sont détaillées dans la publication de février ; leur mise en œuvre Swarm reste à vérifier dans les transitions du moteur.

Swarm conserve une revue indépendante et une acceptation stricte avant publication d’un candidat validé. Cette revue par livraison ne démontre pas l’existence du juge de fin de cycle demandé par la référence de janvier. Le juge global et le démarrage frais des cycles doivent être vérifiés séparément ; leur absence reste un écart ouvert. La sérialisation actuelle des contrôles, de la revue et de la publication doit être mesurée et corrigée si elle empêche le débit visé. Aucune de ces différences ne vaut autorisation de s’écarter de la cible utilisateur.

## Couverture à compléter avant une déclaration d’alignement

La [matrice JAN-1 à JAN-7](../../architecture/CURSOR-ENGINE-CONTRACT.md#exigences-de-la-référence-de-janvier-à-vérifier)
précise les comportements et les preuves attendus contre la publication demandée.
La matrice FEB-1 à FEB-4 du même contrat complète cet audit. Le juge est retiré dans une évolution décrite en février ; JAN-4/JAN-5 restent conservés pour Swarm, à articuler explicitement avec la hiérarchie continue.
Les invariants C1–C3 et V1–V5 ci-dessous sont le contrat historique de Swarm ;
ils ne couvrent pas à eux seuls cette matrice, notamment le juge de fin de cycle,
le renouvellement global du contexte et la planification parallèle.

`plan.json` conserve sa source historique dans `source` et indique la cible
initiale dans `requested_reference` et les deux références retenues dans `requested_references`. Ses huit tâches restent historiques : leur
réussite ne vaut pas validation des exigences JAN-1 à JAN-7. Aucune nouvelle
mission ni autorisation d’exécution n’est créée par cette révision documentaire.

## Invariants non négociables

| Code | Invariant moteur | Preuve attendue |
|---|---|---|
| C1 | Le responsable racine possède le besoin ; un sous-responsable possède exclusivement son périmètre ; aucun des deux ne code. | Refus des rôles exécutables planner/subplanner, refus d'exigence étrangère ; essai de délégation récursive puis remontée de retour. |
| C2 | Chaque exécution travaille dans une copie isolée ; les échanges passent par son responsable. | Deux copies distinctes, fichiers et références non confondus ; refus Parent/canal transversal ; propriétaire retrouvé après redémarrage. |
| C3 | Retour structuré : changement, preuves, limites, écarts, décisions à prendre. Le responsable se réveille et adapte les tâches existantes. | Échec injecté, nouvelle consigne justifiée, pas de doublon et acquittement des événements anciens sans nouvelle validation. |
| V1 | Toute mission autonome a un vérificateur disponible. La présence d'un dépôt géré ne peut désactiver cette obligation. | Création/configuration, aperçu, démarrage web/CLI et lancement direct refusent absence ou incompatibilité. Aucun agent ni consentement résiduel après refus. |
| V2 | Le producteur, le processus de revue et le processus de tests ont des identités distinctes. | Reçu de revue lié au producteur, identifiant de session distinct, appel sans outils d'écriture ; aucun verdict autoattribué par le producteur. |
| V3 | Tests et revue portent sur la même révision immuable, la même tentative et le même contrat. | SHA réel, contexte de revue et reçus empreintés ; verdict inconnu/refus/erreur/périmé => pas d'acceptation ni publication. |
| V4 | Toute évolution du candidat ou du contrat rend les anciennes preuves inapplicables. | Course entre revue et publication, modification de rapport, nouvelle tentative, modification de politique ; contrôles et revues renouvelés sur candidat final. |
| V5 | L'acceptation est atomique ; aucun endpoint ni import d'état ne contourne ces règles. | Tests gate/task update/intégration/publication/reprise, transaction CAS, preuve après réouverture du Store. |
| R1 | Reprise bornée et durable ; crash et quota ne fabriquent jamais de succès. | Arrêt après réservation d'appel, redémarrage, budget épuisé, fournisseur indisponible ; aucun appel payant dupliqué ni budget relevé. |
| U1 | Web et CLI exposent les mêmes faits : terminé, en revue, corrections demandées, accepté, périmé. | Contrat JSON indépendant FR/EN ; rôle, propriétaire, SHA, tests, revue et décision consultables. |
| P1 | L'autonomie est mesurée dans une mission réelle distincte des fixtures. | Besoin -> délégation -> code -> tests -> refus motivé -> correction -> revue -> acceptation -> clôture, avec chronologie et interventions externes comptées. |

## Scénarios qui doivent échouer

1. Démarrer sans vérificateur, y compris ancien reviewer_required absent/faux.
2. Déclarer accepté parce qu'un rapport affirme que les tests passent.
3. Publier avec tests réussis mais revue absente, interrompue, inconnue ou défavorable.
4. Réutiliser un avis obtenu sur SHA A pour accepter SHA B.
5. Utiliser le retour d'une tentative antérieure pour clore une tâche réessayée.
6. Déclencher deux appels de revue à la suite de deux conducteurs concurrents ou d'un redémarrage.
7. Clore le périmètre parent avec un enfant non terminé ou un résultat non accepté.
8. Faire disparaître une erreur ou une preuve périmée lors du changement de langue.

## Organisation et exécution

Trois périmètres : gouvernance, validation, preuve-et-livraison, sous responsabilité racine. Dépendances explicites à l’intérieur de chaque périmètre (le moteur refuse les dépendances transversales) ; la racine attend la fermeture de tous les enfants, et chaque intégration rejoue les contrôles cumulatifs et leur revue. Copies isolées et revue indépendante pour chaque candidat. Claude utilise sa configuration existante, sans montée en gamme implicite. Plafonds existants conservés ; aucune nouvelle mission pour contourner une tentative épuisée. Les huit tâches sont détaillées dans plan.json. Le responsable peut demander une correction dans le budget ; il ne peut supprimer ces exigences ni fabriquer une acceptation.

Le correctif préalable de lancement/revue est développé dans une copie isolée et sera soumis à une revue séparée avant de servir de moteur de cette mission. Cette intervention externe est comptée ; elle ne constitue pas une preuve d'autonomie de Swarm. L'ancien serveur 18788 et la mission w-fff7368010d114f6e2578985 restent conservés en pause. Son R5 actif peut finir sa copie sans autoriser d'autres départs. Ses anciens résultats acceptés ne sont pas réputés relus indépendamment.

## Livraison et limite d'autorisation

Livrer un candidat local, une matrice exigences/preuves/verdicts, les références exactes des contrôles et avis, et un RETEX mesuré. Pas de push, déploiement ou suppression de mission automatique. Aucun succès global tant que les scénarios réels ne passent pas. Ne pas qualifier de test novice une saisie JSON experte, de test visuel une assertion DOM seule, ni de fournisseur réel une fixture. Les budgets/coûts non rapportés restent inconnus.

Huit preuves par résultat : SHA testé ; commandes/dates/codes de sortie ; rapport et limites ; identité et réserves du vérificateur ; décision motivée ; fraîcheur ; livrable/diff ; interventions humaines ou hôte.
