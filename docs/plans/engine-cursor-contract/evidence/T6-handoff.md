# T6 — Vérifier les transitions et mesurer débit, contexte et coût

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PARTIAL** — Scénarios normaux, invalides, concurrence, redémarrage et refus d’acceptation sur candidat changé.
2. **PARTIAL** — Tests ciblés, package pertinent, go test ./..., go vet ./... et -race sur synchronisation ; échecs préexistants séparés sans falsifier de manifeste.
3. **PARTIAL** — Comparer plusieurs niveaux de concurrence dans les limites existantes : temps de travail, attente des verrous, revue, publication, tokens/cache et débit accepté.
4. **PARTIAL** — Relier toute preuve au commit et au diff exacts ; ne pas transformer les résultats simulés en autonomie réelle. Architecture documentée dans le même candidat, avec schémas et limites à jour.

## Preuves et portée

Les vérifications nécessaires remplacent la demande historique de suite complète selon l’instruction utilisateur ultérieure. Les anciens échecs sont conservés. Les coûts/tokens/cache réels restent inconnus et ne sont pas remplacés par zéro.

- [verification-receipt.json](direct-20261012/verification-receipt.json)
- [direct-publication-measured-recipe-repaired.jsonl](direct-20261012/direct-publication-measured-recipe-repaired.jsonl)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).
