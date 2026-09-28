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
La limite actuelle est100 lots et100 fichiers par lot. Les recouvrements entre
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

## État livré et limite

`task.review_coordination` conserve proposition, empreinte, responsable,
décision et ordre. Son état est **`validated_not_executed`** : la couverture et
la structure ont été vérifiées, pas la pertinence sémantique ni les résultats.
La tâche reste bloquée. Aucun appel de revue ni acceptation ne découle du plan.

L’exécuteur de ces lots sémantiques n’est pas encore raccordé. Les fragments
techniques existants ne sont pas présentés comme cette exécution. Il reste à
contrôler la capacité et le budget de chaque lot, exécuter les revues, traiter
les corrections et faire la revue finale sur le même SHA avant acceptation.
