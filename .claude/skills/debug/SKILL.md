---
name: debug
description: "Diagnose unexpected behavior, test failures or repeated blocked attempts by reproducing, isolating and testing a cause before a scoped correction. In preparation, plan the diagnosis only; this method does not authorize execution or retries."
---

# Diagnostiquer et corriger un problème

Adaptation portable de la méthode locale Debug : reproduire, isoler, diagnostiquer,
corriger. Appliquer le contrat partagé de Swarm et répondre dans la langue demandée.
Utiliser cette méthode lorsqu’un défaut ou un blocage est à expliquer ; ne pas
ajouter une investigation à une tâche sans anomalie.

## Limites du rôle

- En préparation, proposer les hypothèses, preuves manquantes et contrôles à
  prévoir à partir des documents fournis. Aucun outil, test, correction ou départ.
- Planificateur et sous-planificateur : analyser les retours fournis et découper
  le diagnostic en tâches bornées ; ne pas réaliser les contrôles ou corrections.
- Exécutant : diagnostiquer avec les outils permis. Corriger seulement si sa tâche
  l’autorise ; une demande de diagnostic seul reste en lecture seule.
- Vérificateur : examiner les preuves fournies sans outil ni correction. Une
  hypothèse plausible n’est pas une cause démontrée ni une validation.

## Investigation utile

1. Distinguer résultat attendu et résultat observé. Consigner le déclencheur,
   les entrées, la révision, l’environnement et la trace exacte, sans secrets.
   Reproduire sur un cas sûr si autorisé ; sinon écrire NON REPRODUIT et pourquoi.
2. Isoler la première frontière où le comportement diverge : interface, CLI,
   moteur, stockage, fournisseur ou espace de travail. Comparer avec un cas qui
   fonctionne et les changements récents pertinents. Lire les fichiers ciblés ;
   élargir seulement si une observation le justifie, sans relire tout le dépôt.
3. Formuler une hypothèse falsifiable et le contrôle minimal qui la départage.
   Conserver le résultat, y compris négatif. Une nouvelle action doit apporter
   une preuve ou tester une hypothèse différente ; ne pas répéter à l’identique.
4. Une cause corroborée permet une correction ciblée si elle est autorisée.
   Garder le scénario qui échouait, rejouer le même contrôle après correction
   puis les régressions pertinentes. Distinguer défaut corrigé et obstacle externe.

## Sortie et reprise

Rapporter : symptôme, reproduction, cause démontrée ou hypothèse restante,
observations qui éliminent les alternatives, correction éventuelle, contrôles
avec résultats et limites, prochaine action et responsable. Utiliser le format
imposé par Swarm ; en session native, un rapport court suffit. Ne pas créer un
arbre de suivi supplémentaire ni changer le format JSON requis.

Si une opération échoue à nouveau sans preuve nouvelle, expliquer la précondition
à changer et transmettre au responsable plutôt que relancer. Des causes multiples
ou un périmètre trop large justifient un découpage proposé, pas de nouvelles tâches
créées hors du moteur. Respecter budgets et tentatives ; aucune hausse implicite,
suppression de verrou, modification de base ou acceptation pour forcer un résultat.
Les preuves de vérification personnelle ne remplacent pas la revue indépendante.
