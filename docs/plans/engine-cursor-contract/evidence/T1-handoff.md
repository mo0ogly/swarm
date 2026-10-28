# T1 — Confirmer les écarts et décider l’architecture de l’évolution

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PASS documentaire, revue indépendante non effectuée** — Matrice JAN-1..JAN-7 et FEB-1..FEB-4 avec faits, hypothèses et preuves distincts.
2. **PASS documentaire, efficacité réelle non démontrée** — Comparer cycles explicites et hiérarchie continue avec points de jugement ; les deux doivent couvrir le juge global et le renouvellement requis.
3. **PASS documentaire, recette réelle non effectuée** — Décision motivée sur reprise, débit, qualité, effort et compatibilité. Examiner juillet Cursor et les sources complémentaires avant de décider.
4. **PASS documentaire, activation native non effectuée** — Documenter la séparation avec la finalisation historique et le lancement public en copie isolée sans remise à zéro ; architecture cible, migrations, reprise et risques consignés.

## Preuves et portée

La matrice JAN/FEB distingue constat historique et preuves actuelles. La décision consolidée compare cycles globaux et hiérarchie continue selon correction, reprise, débit, coût, effort et compatibilité ; elle identifie les composants, la migration et ses risques. Les sources primaires de janvier/février/juillet et Google/OpenAI/Anthropic ont été relues le 13 octobre. L’étude eBPF distingue instrumentation Go et collecteur optionnel non implémenté. Cela satisfait la portée documentaire de T1 ; ni l’efficacité réelle ni l’acceptation native ne sont déduites de cette documentation.

- [CURSOR-ENGINE-CONTRACT.md](../../../architecture/CURSOR-ENGINE-CONTRACT.md)
- [Décision et architecture FR](../../../architecture/CURSOR-ENGINE-EVOLUTION-20261010.md)
- [Décision et architecture EN](../../../en/CURSOR-ENGINE-EVOLUTION-20261010.md)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).
