# T7 — Revue indépendante et recette réelle cumulative

Rapport consolidé de développement direct, 13 octobre 2026. **PARTIAL**, aucune acceptation native ni remise automatique. Le nom du livrable est conservé du plan original ; ce fichier ne fabrique pas une livraison par le moteur.

## Critères conservés

1. **PARTIAL** — Fournisseur réellement disponible et politique de modèle figés ; absence de quota ou configuration bloque l’appel sans essai de contournement.
2. **PARTIAL** — Observer découverte, délégation parallèle, remise, correction, jugement global, contexte frais et reprise avec fournisseur réel.
3. **PARTIAL** — Avis indépendant et contrôles couvrent le même candidat ; échec bloque acceptation et clôture.
4. **PARTIAL** — Rapport cumulatif JAN/FEB avec interventions humaines, coût connu ou inconnu, échecs et critères non démontrés. Architecture documentée dans le même candidat, avec schémas et limites à jour.

## Preuves et portée

Recettes locales et fournisseur HTTP/subprocess factice sont distincts d’une recette avec modèle réel. La revue de cette conversation n’est pas indépendante. L’utilisateur a autorisé ces vérifications finales isolées le 13 octobre (« go fais ca »). La revue du candidat 44a86e6 demande deux corrections P2 ; leurs régressions ciblées passent. La recette réelle Codex a observé des cycles renouvelés, l’exploration et deux sous-planificateurs simultanés ; sa livraison cumulative reste en cours.

- [verification-receipt.json](direct-20261012/verification-receipt.json)
- [completion-audit-20261012.md](../completion-audit-20261012.md)

Les reçus conservent leurs révisions, empreintes et commandes originales. Ils ne sont pas redatés pour créer une revue du candidat courant. Le snapshot de sources courant est dans `docs/engine-cursor-source-manifest.json`. La mission historique reste en pause ; budgets, tentatives et verdicts antérieurs sont conservés.

## Condition de clôture

Recette cumulative applicable et revue indépendante du même candidat, corrections nécessaires vérifiées, puis décision d’acceptation autorisée. Voir [l’audit de clôture](../completion-audit-20261012.md).

Reprise T7 du 13 octobre : saturation réelle de gpt-6.1-sol, changement explicite vers le standard configuré gpt-5.6-sol/medium, appels consommés conservés. Une consigne de recette « FIRST » a entraîné des cycles répétés ; arrêt des départs puis instruction de phase via reprise publique, sans reset. Les deux sous-planificateurs ont été actifs simultanément. La revue indépendante a confirmé F1 aperçu/historique navigateur et F2 perte du motif d’erreur Codex ; rouges reproduits puis 11 tests ciblés / 18 avec sous-cas `-race` PASS. Répertoire courant : `evidence/t7-20261013`. Cela ne prouve pas encore livraison, recette cumulative achevée ni déploiement.

La revue du candidat 61f82bd a trouvé F2b : `tool_result/is_error=true` pouvait être classé comme erreur fournisseur. Rouge conservé, puis prédicat limité aux seules enveloppes fournisseur ; 16 tests ciblés / 23 avec sous-cas `-race` PASS. Un ENOSPC de copie du binaire a été traité par nettoyage du cache Go reconstructible, sans suppression de preuve ni mission.
