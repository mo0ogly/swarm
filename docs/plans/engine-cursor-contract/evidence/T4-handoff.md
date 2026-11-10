# T4 — Corriger publication, conflits et richesse des remises

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PARTIAL** — Workers isolés sans coordination directe ; remise unique au responsable exact avec changements, limites, découvertes et décisions attendues.
2. **PARTIAL** — Responsable exploite une découverte et se réactive après redémarrage.
3. **PARTIAL** — Mesurer la voie exclusive ; réduire l’attente centrale selon T1 sans accepter un candidat différent de celui contrôlé.
4. **PARTIAL** — Publication concurrente, conflit, preuve périmée et échec de revue ont des refus et reprises observables ; aucune autoacceptation. Architecture documentée dans le même candidat, avec schémas et limites à jour.

## Preuves et portée

La production parallèle et la publication exclusive restent distinctes. Le déplacement réel des racines est testé sur fixtures renommées ; le correctif n’est pas installé sur le serveur historique.

- [PROJECT-RELOCATION.md](../../../architecture/PROJECT-RELOCATION.md)
- [managed-path-verification-receipt.json](direct-20261012/managed-path-verification-receipt.json)
- [direct-publication-staging-race.jsonl](direct-20261012/direct-publication-staging-race.jsonl)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).
