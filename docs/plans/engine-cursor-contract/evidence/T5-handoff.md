# T5 — Exposer les politiques et adapter les prompts par rôle

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PARTIAL** — Chaque nouveau réglage est visible, modifiable et enregistré dans Admin ; moteur et CLI lisent la même valeur.
2. **PARTIAL** — Aperçu sans écriture, validation, révision concurrente, historique et restauration vérifiés.
3. **PARTIAL** — Paramètres de politique absents des constantes Go ; les bornes techniques sont distinguées des choix configurables.
4. **PARTIAL** — Routes modèles et prompts effectifs identifiés par rôle ; aucune montée de gamme ni relance silencieuse.
5. **PARTIAL** — Parcours clavier/mobile, thèmes clair/sombre, FR/EN et erreurs vérifiés. Architecture documentée dans le même candidat, avec schémas et limites à jour.

## Preuves et portée

Les politiques des contrats modifiés sont administrables et testées. Le chargement d’éditeur hérite maintenant du délai navigateur courant, avec recette dédiée. Cela ne vaut pas déclaration d’absence de constante technique dans tout le dépôt.

- [WEB-REQUEST-POLICY.md](../../../architecture/WEB-REQUEST-POLICY.md)
- [web-requests-verification-receipt.json](direct-20261012/web-requests-verification-receipt.json)
- [api-policy-verification-receipt.json](direct-20261012/api-policy-verification-receipt.json)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).
