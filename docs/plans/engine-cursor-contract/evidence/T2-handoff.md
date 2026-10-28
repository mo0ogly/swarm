# T2 — Implémenter le juge global et le renouvellement des cycles

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PARTIAL** — Verdict global de poursuivre ou finir distinct de la revue indépendante de candidat.
2. **PARTIAL** — Transition durable entre cycles, contexte reconstruit, objectif, preuves, tentatives et limites conservés.
3. **PARTIAL** — Rejeu, verdict périmé, panne, quota et limite refusent les effets ou appels doubles.
4. **PARTIAL** — Aucune clôture globale sans les preuves fraîches requises. Architecture documentée dans le même candidat, avec schémas et limites à jour.

## Preuves et portée

Les tests vérifient réception du contexte, décision et effet en fixtures. Le parcours avec vrai modèle n’est pas démontré sur le candidat de cette reprise. La clôture legacy reste soumise à ses contrôles ; judge n’est pas imposé rétroactivement à toute ancienne clôture.

- [JUDGMENT-CYCLES.md](../../../architecture/JUDGMENT-CYCLES.md)
- [verification-receipt.json](direct-20261012/verification-receipt.json)
- [direct-judge-operation-received-context.jsonl](direct-20261012/direct-judge-operation-received-context.jsonl)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).
