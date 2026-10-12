# Audit de clôture — besoin conservé, livraison non terminée

État au 12 octobre 2026 : **objectif actif**, développement direct Codex, mission
historique conservée en pause. Cet audit ne valide ni une recette réelle ni une
acceptation native. Il reprend les critères de `evolution-20261010.json`, la reprise
T1–T8 et UX-GRAPH-01 ; les exemples ci-dessous ne constituent pas leur totalité.

| Lot | Preuve disponible | Lacune qui empêche sa clôture |
| --- | --- | --- |
| T1 | Contrat JAN/FEB, corpus de recherche, architectures et décisions de reprise existants | Rapport T1 consolidé ; couverture cumulative et revue indépendante du candidat actuel encore requises |
| T2 | Cycles, judge/renew effectivement reçus et appliqués en fixtures ; fraîcheur, rejeu et fin explicite contrôlés | Rapport T2 consolidé ; recette fournisseur réel et revue indépendante du candidat actuel non démontrées |
| T3 | Exploration attribuée, adaptation du plan et sous-planification concurrente couvertes par tests ciblés conservés | Rapport T3 consolidé ; comportement réel de plusieurs sous-planificateurs non démontré pour cette reprise |
| T4 | Remise anticipée, candidat immuable, revue/publication exclusive, conflits ; correction du déplacement et des sous-dossiers vérifiée | Rapport T4 consolidé ; effet du correctif sur le serveur natif historique encore non vérifié |
| T5 | Budgets, hiérarchie, cycles et délais administrables ; API/CLI et recettes FR/EN ; transport sans seconds plafonds 60/300 | Rapport T5 consolidé ; délai éditeur désormais hérité d’Admin et vérifié ; qualification cumulative restante |
| T6 | Vérifications ciblées, `-race`, compilation/vet et mesures de deux niveaux de concurrence | Rapport T6 consolidé ; coûts/tokens/cache réels non mesurés ; anciens échecs de suite complète non transformés en PASS |
| T7 | Parcours CLI/API/navigateur sur stockage isolé et HTTP local ; compteurs/probes réellement observés | Aucun modèle réel ni avis indépendant du candidat actuel ; rapport T7 consolidé et explicitement partiel ; recette cumulative réelle à organiser dans les droits accordés |
| T8 | Phase RETEX dans le produit et RETEX intermédiaire de reprise documentés | RETEX local et quick wins réalisés ; bilan cumulatif final dépend de T7 |
| UX-GRAPH-01 | Tâche et critères GRAPH-01..GRAPH-09 ajoutés, surfaces existantes identifiées | Implémentation et recette locale FR/EN : plein écran, déplacement, ports, retrait/reconnexion et cycles refusés ; déploiement natif et revue indépendante non effectués |

Le critère historique T6 demandait notamment `go test ./...` et `go vet ./...`.
L'utilisateur a ensuite demandé uniquement les contrôles nécessaires et le dépôt
l'applique dans `AGENTS.md`. Cette instruction prévaut ; elle ne transforme pas
les échecs historiques en réussite et n'efface pas leur trace. Une vérification
plus large exige une dépendance ou un risque concret, pas le volume des tests.

## Prochaine séquence

1. Terminer la remise du correctif d'appartenance physique : sources, preuves,
   architecture FR/EN et reçu lié au candidat. Ne pas réécrire les chemins privés
   de la mission ni redémarrer un service partagé sans précontrôle autorisé.
2. Réaliser UX-GRAPH-01 sur les composants existants et les opérations publiques
   de brouillon/aperçu/application, avec contrôles moteur préservés.
3. Terminer les lacunes de politiques et les rapports nommés en distinguant chaque
   critère PASS/PARTIAL/NOT TESTED. Un fichier créé ne prouve pas son acceptation.
4. Recette cumulative, revue réellement indépendante selon le mode autorisé,
   corrections et quick wins nécessaires, puis RETEX final.

Les obligations de revue/fournisseur réel ne sont pas retirées pour faciliter la
clôture. La reprise directe n'autorise pas de relance de la mission historique,
de remise à zéro, de fausse autonomie ou d'acceptation par simple documentation.

Mise à jour du 13 octobre : rapports nommés consolidés en `evidence/T1-handoff.md` à `T7-handoff.md`, sans fabrication d’acceptation. Les quick wins de recette du graphe et la suppression du délai éditeur fixe sont réalisés. La demande de recette Codex isolée avec modèle réel et revue indépendante est en attente de décision ; la mission historique n’est pas relancée.
