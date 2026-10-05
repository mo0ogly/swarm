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
| `bench/run_s.py`, `bench/provider_s.py`, `bench/planner_fixture.py` | Condition S : Swarm en mission hiérarchique, responsable de mission scripté, préparateur et règlement scriptés |
| `bench/campaign.py` | Campagne : grille condition × clé × faute × répétition, JSONL reprenable |
| `bench/tables.py` | Tableaux de l'article : taux, intervalles de Wilson, exclusions, durées, annexes |

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
- Condition S : refaite sur le modèle hiérarchique de Swarm (responsable de
  mission scripté, remise automatique au responsable, contrôles déclarés par
  exigence). Fautes F1 à F8 et F4e adaptées et explorées ; observations et
  preuves d'ordre dans [`docs/benchmarks/billing/observations.md`](../../docs/benchmarks/billing/observations.md).
- Campagne et tables : `campaign.py` et `tables.py` écrits et testés. Une
  répétition générale (2 répétitions, 180 exécutions) passe le critère du
  protocole : [`docs/benchmarks/billing/resultats/`](../../docs/benchmarks/billing/resultats/).
- Préparateur réel (`provider_real.py`) : pas encore écrit.

## Campagne

```sh
cd benchmarks/billing
systemd-inhibit --what=sleep:idle --why="banc swarm" \
  python3 bench/campaign.py --out ../../docs/benchmarks/billing/resultats/campagne-AAAAMMJJ.jsonl \
  --reps 100 --purge-ok --jobs-b 4 --jobs-s 1
python3 bench/tables.py ../../docs/benchmarks/billing/resultats/campagne-AAAAMMJJ.jsonl \
  --out ../../docs/benchmarks/billing/resultats/campagne-AAAAMMJJ.md
```

- `systemd-inhibit` empêche la mise en veille : une veille pendant une
  exécution S fausserait le bail du conducteur (F7) et les preuves d'ordre,
  sans que la durée mesurée (horloge monotone) le montre.
- `--jobs-s 1` : une exécution S parallèle charge la machine et fausse les
  durées, donc la mesure de H5.
- `--purge-ok` supprime la racine d'une exécution OK après écriture de sa ligne
  (`run_dir_purged: true`) ; les racines INVALIDE, DÉLAI et ERREUR restent pour
  diagnostic. Sans l'option, chaque exécution garde sa racine dans le dossier
  temporaire.
- La campagne est reprenable : relancer la même commande saute les cases déjà
  écrites. Ctrl-C laisse un JSONL fait de lignes complètes.
- L'empreinte du banc et du binaire est enregistrée au début et à la fin ; un
  changement est signalé dans le JSONL et en tête des tableaux.

## Limites

- B1 contrôle le bénéficiaire et le plafond, pas le montant de facture ni le
  doublon : c'est une référence minimale, pas toute approche par appel.
- L'API simulée déduplique parfaitement par clé ; une API réelle peut limiter
  la conservation des clés ou rejeter un rejeu dont les paramètres diffèrent.
- Un seul scénario métier ; aucune validation sur un système bancaire réel.
