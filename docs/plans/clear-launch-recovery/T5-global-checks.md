# Contrôles globaux — candidat actuel

CURRENT CANDIDATE FULL SUITE: PASS

Base Git : 1d9570bd4ef617130c6be96b7ec88844fdbcd00e ; arbre dirty ; 48 fichiers identifiés par T5-candidate.json. Candidat : a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77. Les documents/captures sont liés séparément aux reçus.

## Contrôles réellement exécutés par Codex

- go test ./... -timeout 40m : PASS 473,034 s, sortie 0 ; log T5-go-full.log. Tous les fichiers Go sont ceux de ce candidat. Aucun changement applicatif après la suite.
- Tests ciblés de reprise archivée, citations et processus indépendant : PASS 0,456 s.
- go vet ./... : PASS.
- Race ciblé réservation SQLite / sortie de contrôles : PASS 12,799 s sur les mêmes fichiers de concurrence, non modifiés ensuite.
- npm test : PASS, aucun changement frontend après cette exécution ; contrats FR/EN, mesures, reprise et modèles.
- python3 tools/agent-workflows/check.py : PASS, neuf méthodes.
- git diff --check : PASS.

Les PASS à 455,480 s, 992,378 s et 491,140 s sont historiques. Une suite intermédiaire a été interrompue après découverte du défaut de reprise archivée ; elle ne compte pas comme PASS.

## Recettes réelles

Les quatre parcours ont des observations et captures FR/EN × État/sombre, avec ouverture/fermeture et focus. La divergence de modèle utilise un processus fixture explicitement identifié ; elle ne prouve pas le modèle d’un LLM réel. Le fournisseur réel de la tentative T4 rapporte claude-sonnet-5.

T4 : reçu T4-full-fresh-recipe.json, code de processus 0. Trois étapes sur fiches dans la même mission en 105 secondes : non-validation constatée après avis r277 et retour explicite Bloquée r281, correction Next r282 visible, reprise par bouton web r283 et fiche À vérifier. Le retour Bloquée est une action publique du superviseur après recette expirée, pas un nouvel avis du vérificateur. Défi neuf 84a714566ebedd5184fcffcbd2d2ac40 (délai300s), observations toutes postérieures au défi. Les deux tentatives et plafonds restent conservés. Le moteur vérifiera le reçu lié aux captures et événements en lecture seule ; il ne prétend pas modifier la tâche pendant ses contrôles à révision figée.

Le partage de sortie de contrôle est facultatif, désactivé par défaut, 8 Kio maximum avec taille/troncature ; il a été testé dans les quatre combinaisons de langue/thème et au clavier. Les captures control-output-*.png en conservent les observations.

## Limites et attribution

Les tests utilisent des racines temporaires. La mission réelle est observée via CLI public et CUA, sans mutation directe SQLite. La recette navigateur appartient au superviseur externe Codex/CUA, pas au producteur ni au vérificateur sans outils. Elle ne démontre pas l’autonomie générale sans pilote. Les images/JSON attribués ne sont pas une certification cryptographique de l’origine d’une capture. Les erreurs de recette, refus et coûts restent dans le RETEX. Coût total, requêtes LLM internes et interventions humaines non mesurées : inconnus, pas zéro.

T5 doit réaliser sa lecture CLI et écrire son propre rapport en attribuant ces contrôles au superviseur ; ne pas relancer la suite complète. Revue indépendante et acceptation encore requises avant livraison.
