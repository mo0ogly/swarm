# Banc de facturation simulé

Banc d'évaluation de Swarm sur un processus à effet irréversible : le paiement
de factures fournisseurs. Il mesure ce qu'empêchent une clé d'idempotence seule,
un contrôle appel par appel et le moteur Swarm complet, sous huit fautes
injectées de façon reproductible. Toutes les données sont synthétiques.

Conception : [`docs/benchmarks/billing/conception.md`](../../docs/benchmarks/billing/conception.md).
Plan d'implémentation et amendements : [`docs/benchmarks/billing/plan-implementation.md`](../../docs/benchmarks/billing/plan-implementation.md).

## Composants

| Module | Rôle |
| --- | --- |
| `bench/ledger.py` | Grand livre SQLite en partie double et invariants comptables |
| `bench/invoices.py` | Factures synthétiques déterminées par une graine |
| `bench/payment_api.py` | API de paiement simulée, seul écrivain du grand livre ; fautes injectables, politique B1, jeton de règlement |
| `bench/preparer.py` | Préparateur scripté : propose un lot, ne paie jamais |
| `bench/check_lot.py` | Contrôle d'un lot contre le grand livre |
| `bench/settle.py` | Règlement déterministe, clé d'idempotence aucune / par tentative / métier |
| `bench/verify_settlement.py` | Contrôle du règlement et revue finale |
| `bench/metrics.py` | Mesures : doublons, paiements inexacts, impayés, violations |
| `bench/harness.py` | Racine isolée par exécution, API, mesure, provenance |
| `bench/run_b.py` | Conditions B0 (sans moteur) et B1 (contrôle par appel) |
| `bench/run_s.py`, `bench/provider_s.py` | Condition S (Swarm) — voir « État » |

## Lancer les tests

Python 3.11 ou plus récent, bibliothèque standard uniquement.

```sh
cd benchmarks/billing
python3 -W error -m unittest discover -s tests -t .
```

Le test d'intégration Swarm est sauté par défaut. Pour l'exécuter :

```sh
make build                       # à la racine du dépôt : produit bin/swarm
cd benchmarks/billing
BANC_SWARM=1 python3 -m unittest tests.test_run_s -v
```

`BANC_SWARM_BIN=/chemin/vers/swarm` permet de mesurer un binaire construit
ailleurs.

## État

- Conditions B0 et B1 : implémentées, relues, avec contrôles positifs : chaque
  faute produit le défaut attendu dans la chaîne sans protection.
- Condition S : à refaire sur le modèle hiérarchique de Swarm. La version
  actuelle de `run_s.py` repose sur des échanges directs entre tâches, que
  Swarm refuse en mission hiérarchique ; le test d'intégration échoue donc à
  l'installation, avec une erreur explicite (« Organisation autonome non
  configurée »).
- Campagne (`campaign.py`, `tables.py`) et préparateur réel
  (`provider_real.py`) : pas encore écrits.

## Limites

- B1 contrôle le bénéficiaire et le plafond, pas le montant de facture ni le
  doublon : c'est une référence minimale, pas toute approche par appel.
- L'API simulée déduplique parfaitement par clé ; une API réelle peut limiter
  la conservation des clés ou rejeter un rejeu dont les paramètres diffèrent.
- Un seul scénario métier ; aucune validation sur un système bancaire réel.
