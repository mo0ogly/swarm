# Planifier les lots de revue

Le responsable de la tâche, planificateur ou sous-planificateur, peut désormais
proposer un plan avec l’opération `review-plan` d’une décision `planning decide`.
La décision utilise la révision courante, le bail, la génération et les événements
fournis à son activation. Elle ne crée aucun exécutant supplémentaire.

## Entrées du responsable

Le contexte `review_planning_inputs` expose les inventaires pertinents pour les
événements de l’activation : candidat, empreinte des preuves, chemins modifiés,
critères `task#N`, diagnostic et suggestions de regroupement. Si les preuves ne
sont pas disponibles, le contexte le dit explicitement. Les entrées ne sont pas
tronquées pour faire tenir une activation ; le contrôle de taille reste actif.

`planning scope-preview WORK --input task.json` fournit les mêmes identités.

## Proposition

L’opération porte `id=TASK` et `deliverable` contient le JSON suivant sous forme
de chaîne. Les autres champs de l’opération respectent le schéma habituel.

```json
{
  "candidate_commit": "SHA exact du candidat",
  "evidence_sha256": "empreinte exacte du contexte de preuves",
  "lots": [
    {
      "id": "contrat",
      "kind": "component",
      "objective": "Vérifier le contrat et ses tests",
      "files": ["contrat.go", "contrat_test.go"],
      "criteria": ["tache#1"],
      "depends": []
    }
  ],
  "final_review": "Vérifier les interactions entre lots et les contrôles globaux sur ce candidat."
}
```

Les valeurs sont illustratives : ne jamais les soumettre telles quelles.
Types autorisés : `requirement`, `component`, `dependency`, `specialty`, `volume`.
La limite actuelle est de 100 lots et 100 fichiers par lot. Les recouvrements entre
lots sont autorisés, par exemple pour une revue de sécurité complémentaire.

## Contrôles du moteur

- Responsable propriétaire de la tâche bloquée, aucune revue engagée.
- Bail, génération, événement et révision vérifiés par le contrat de décision.
- Candidat et empreinte des preuves exacts.
- Tous les fichiers et critères couverts ; aucune référence inventée.
- Identités de lots uniques, objectifs explicites, dépendances existantes et DAG
  sans cycle. Le moteur enregistre l’ordre de dépendance.
- Revue finale des interactions explicitement prévue.
- Décision persistée et rejouable sans double consommation.

Un plan déjà enregistré n’est pas remplacé implicitement. Une future procédure
de révision devra préserver l’ancien plan et invalider les avis concernés.

## Exécution et reprise

`task.review_coordination` conserve la proposition, son empreinte, le responsable,
la décision et l’ordre. `validated_not_executed` indique un plan enregistré.
La reprise publique de la revue (`retry-review`) vérifie ce plan sur les preuves
actuelles avant tout appel. Le démarrage passe son état à `execution_started` ;
le verdict et les erreurs restent dans `independent_review` et son journal.

Le moteur exécute les lots dans l’ordre de leurs dépendances. Chaque paquet
porte l’objectif du lot et le texte des critères. Un lot peut nécessiter plusieurs
inspections bornées. Les rapports, contrôles, preuves historiques et autres
pièces communes sont également inspectés ; aucune preuve ne disparaît parce
qu’elle n’appartient pas à un lot. Les recouvrements volontaires sont examinés
pour chaque lot concerné et peuvent donc augmenter le coût.

Avant le départ, le moteur contrôle le transport complet et le budget de toutes
les inspections **plus deux appels finaux**. `planning review-cost` fournit
l’estimation. Un budget insuffisant refuse le départ sans appel facturé.
Les observations et erreurs sont conservées dans le journal durable existant ;
une reprise explicite conserve les inspections acquises du même plan. Le
redécoupage technique qui effacerait les limites des lots est refusé.

La revue finale reçoit le plan, les observations et les preuves originales.
Une inspection ne valide jamais une tâche. La publication reste soumise au
verdict global et aux contrôles sur le même candidat.

## Limites explicites

Cette version exécute séquentiellement les inspections via le vérificateur
configuré. Elle ne crée pas un nouvel agent autonome par lot ni un échange direct
entre vérificateurs : les interactions non prouvées remontent à la revue finale.
Une pièce indivisible trop grande est refusée, sans troncature. Un changement de
candidat ou de preuves invalide le plan ; sa substitution implicite est interdite.
La validation structurelle ne garantit pas la pertinence du découpage proposé.

La capacité est calculée en incluant les consignes et critères de chaque lot.
Quand la capacité locale du client/modèle est démontrée, le protocole de transport
adapté aux jetons peut regrouper davantage de pièces **à l’intérieur d’un même
lot**. Il ne fusionne pas les lots. Le précontrôle couvre aussi les deux appels
finaux ; l’estimation et le départ utilisent le même choix de protocole.
