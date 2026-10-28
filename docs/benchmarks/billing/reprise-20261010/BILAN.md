# Bilan de reprise — campagnes de facturation, 10 octobre 2026

## Résultat

**31/31 combinaisons F4e rejouées et mesurables sur le moteur main `3faa8a9`.** Elles correspondent aux 26 anciennes erreurs SQLite F4e et aux cinq anciens délais sans décision. Dans ce nouveau lot : aucune erreur SQLite remontée par le banc, aucun délai global, aucune erreur de nettoyage.

| Mesure | Observation |
|---|---:|
| Arrêt attribué au moteur avec preuve de prepare périmée | 31/31 |
| Mutation après acceptation prouvée | 31/31 |
| Événement dependency_stale observé | 31/31 |
| Tentative de règlement | 0 |
| Paiement, paiement inexact, doublon, faux succès | 0 chacun |
| Factures restant impayées | 372 (12 par cas) |
| Anciennes combinaisons DÉLAI désormais mesurables | 5/5 |
| Appels LLM réels dans ce rejeu | 0 |

Les 372 impayés sont attendus : le candidat a été modifié après acceptation. Le moteur retient le règlement ; le protocole ne lui demande pas de réparer le lot puis de payer. Les missions restent en attente de reprise causale, leur clôture n'est pas présentée comme acquise.

Durée observée par exécution : médiane 20.906 s ; minimum 16.725 s ; maximum 24.397 s. Ces durées comprennent les attentes et contrôles du banc ; elles ne sont pas une mesure de latence interne du moteur ni une comparaison de performance contrôlée.

## Pièces et reproduction

- Journal nouveau : `../resultats/rejeu-exclusions-F4e-20261010.jsonl` ; SHA256 `2c9cdb90b7b51beed1070ee89c77410ec3417a984b65a9970f05734822f51e5c`.
- `bilan.json` : 31 identifiants de mission, racines conservées, empreintes des audits publics privés.
- Dans chaque racine conservée, `replay-public-audit.json` (permission 0600) contient les sorties CLI publiques work/planning/mission recueillies après arrêt. Ce sont des instantanés supplémentaires ; les empreintes de projection du journal initial sont des observations prises à un autre instant et ne doivent pas être confondues avec eux.
- Identités du candidat, du banc, de l'analyse et du lanceur enregistrées dans le journal. Aucune modification du banc ou du binaire n'a été détectée pendant le lot.
- Tests Python du banc : 211 tests, OK, deux intégrations sautées par défaut. Les deux intégrations ont été exécutées séparément contre le binaire courant : OK (23,443 s). Vérificateur de configuration et `git diff --check` : OK.

```sh
python3 docs/benchmarks/billing/reprise-20261010/audit.py \
  --results docs/benchmarks/billing/resultats/rejeu-exclusions-F4e-20261010.jsonl \
  --binary /home/fpizzi/workspace/swarm-training-main-integration/bin/swarm-current-main \
  --out /tmp/billing-replay-audit.json --cli-timeout 30
```

## Limites et attribution

Les 31 cas ciblent les anciennes exclusions, une seule nouvelle observation par combinaison. Ils ne remplacent ni les 9 000 résultats historiques ni les 1 700 vérifications, et ne donnent pas un taux de panne en production. Ils n'injectent pas explicitement un verrou SQLite : la récupération sous contention est couverte par la qualification moteur antérieure, pas démontrée causalement par ces seuls rejeux.

Les cinq délais historiques ne deviennent pas rétrospectivement des erreurs SQLite ou des timeouts LLM. Plusieurs correctifs séparent les candidats. On observe maintenant une progression jusqu'au diagnostic de dépendance périmée ; attribuer sa disparition au seul correctif D5 serait injustifié.

**F7 / tentative / graine 1010 reste non testé dans cette reprise** : le banc historique lit directement l'identité du conducteur dans la base Swarm pour sa preuve. L'API publique actuelle ne fournit pas les deux identités nécessaires à cette preuve. Ce cas conserve sa place dans le bilan des 27 anciennes erreurs SQLite.

Préparateur, responsable et reviewer sont scriptés. La supervision a lancé le protocole et vérifié les données publiques ; aucun relecteur indépendant humain ou LLM n'a audité ce nouveau bilan. La recette ne démontre pas l'autonomie d'un collectif ni la sûreté d'un système bancaire réel.

## Suite préparée

Le document `S-COLLECTIF-PREFLIGHT.md` reprend le volet réel : raccords de création des tâches et de profils, règles de règlement, isolation des adaptateurs, consommation par rôle, puis deux essais pilotes avant une campagne plus large. La conception de ce collectif existait ; son exécution n'est pas encore implémentée dans le banc.
