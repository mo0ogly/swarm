# B2 — comparateur workflow durci (lot G6)

9 novembre 2026 · [English](../../../en/plans/actions-confiance/paea/B2-COMPARATOR-20261109.md)

Lot G6 du [plan de clôture des écarts](GAP-CLOSURE-PLAN-20261103.md) : « Paquet
de revue, contrats/vecteurs, collecte et **comparateur B2 avec mêmes
protections métier** ». Sur les quatre objets de G6, deux sont livrés
(contrats/vecteurs, collecte — incréments 14-25), un est bloqué humain
(préenregistrement : désignation évaluateur/corpus réservé/volumes/ordre par le
commanditaire, H3). Ce document livre le troisième : le **comparateur B2** —
le workflow déterministe durci exigé par le [protocole
E3](E3-PROTOCOL.md) (« Comparateur principal : workflow déterministe B2
disposant des mêmes protections métier connues. Ne pas affaiblir B2 pour
fabriquer un avantage de Swarm. »).

## 1. Parti pris : ne rien réimplémenter

B2 ici n'est pas une imitation du labo — c'est le labo lui-même, piloté par
script. `tools/paea/b2_comparator.py` importe
[tests/paea_pilot_lab_test.py](../../../../../tests/paea_pilot_lab_test.py)
sans modification (importlib par chemin de fichier) et n'exécute que les
primitives existantes de la recette qualifiée :

- **canonicalisation CLI épinglée** avec re-vérification du hash avant/après
  chaque sous-processus (`PilotLab.report`) — rapport authentique du vrai CLI
  public `protection canonicalize`, jamais un modèle Python ;
- **autorité d'époque** (`lab.authority` approve/revoke, `BEGIN IMMEDIATE`) —
  le même journal `ordering` persisté ;
- **puits à intent stable** (SQLite `effects(intent PRIMARY KEY)` ; git :
  marqueur `refs/paea-lab/intents/<intent>` liant tout le contenu canonique
  par blob, transaction `update-ref --stdin` unique) ;
- **oracle séparé en lecture seule** (`pilot_oracle.py`, processus distinct,
  `mode=ro`/`query_only`, jamais consulté depuis les réponses ni les journaux
  de décision) ;
- **réconciliation déclarée** (`RECONCILED_COMMITTED` /
  `RECONCILED_NOT_COMMITTED` / `UNKNOWN` avec cause) ;
- **témoin avant chaque faute** (`witness`) — un effet valide sous les mêmes
  préconditions précède toute injection.

Le contrat d'usage du coureur est donc identique pour B2 et pour le candidat
évalué : mêmes fixtures `contract-v1.json`, même binaire CLI épinglé et
re-vérifié à chaque `tearDownClass`, même dossier d'evidence neuf créé par
`setUpClass`, mêmes refus pré-effect (exit 2, cause sur stderr). Toute
modification future du fichier pilote change le comparateur aussi — voulu :
un seul contrat d'admission.

## 2. Scénarios (trois familles × deux domaines, ordre fixe)

| Scénario | Domaines | Ce qui est mesuré (comportement B2 observé) |
|---|---|---|
| T04-substitution | payment, git | contenu substitué refusé pré-effect (`laboratory-approval-missing`, exit 2), puits inchangé à l'oracle, JSON équivalent accepté ensuite |
| T06-T21-crash-recovery | payment, git | sortie 86 post-effect → un effet/marqueur exact, `RECONCILED_COMMITTED`, rejeu nouvelle clé transport même rapport accepté, conflit d'intent refusé (`laboratory-intent-conflict`), `UNKNOWN`+cause déclarée |
| T05-T17-revocation-order | payment, git | révocation avant premier usage → refus (`laboratory-epoch-revoked`), zéro effet, `RECONCILED_NOT_COMMITTED`, ordre durable `approve,1→revoke,2→approve,2→commit,2`, ordre inverse (commit avant revoke) tenu |

Chaque essai est un `PilotLab` neuf sur un sous-dossier d'evidence frais
(`T04-payment`, …, `T05-T17-git`), témoins compris. Les essais sont
séquentiels et déterministes ; aucun cumul d'assertions n'est compté comme des
essais indépendants ([GUIDE-INTEGRATION
§16](GUIDE-INTEGRATION.md)).

## 3. Rapport fermé et étiquettes honnêtes

Le rapport `paea-b2-report.json` (schéma v1, publié atomiquement via
`tools/verification/recipe.py::publish`) porte par essai : scénario, domaine,
décision (`refused-then-accepted`, `unknown-recovered-once`,
`revoked-refused-then-current-grant-commits`), durée mesurée, code de sortie,
notes. Compteurs `refused_pre_effect`/`recovered_unknown`/`deviations` ;
latences p50/p95 plus-proche-rang des durées d'essai.

Les étiquettes honnêtes du protocole sont inchangées et rapportées telles
quelles : `authority='simulated'`, `isolation='same-uid'`,
`independent_evaluation='not-established'`,
`protected_invariants='not-established'`, `scope='isolated-laboratory'`. Un
comparateur B2 vert n'établit ni auteur, ni autorité réelle, ni oracle
véridique, ni évaluation indépendante.

Règle de publication : **toute déviation refuse la publication** (exit 1,
`DEVIATION` sur stderr, evidence conservée). Une déviation n'est jamais
notée ; elle est conservée avec sa cause. Les refus pré-effect
(dossier d'evidence déjà existant, hash de binaire faux, binaire absent ou
illisible) refusent avant tout essai et sans résidu.

## 4. Résultat réel (9 novembre 2026)

Exécution par la voie publique (recette
`tools/verification/recipes/paea-b2-comparator.json`, variable épinglée
`SWARM_PAEA_TEST_BINARY=bin/swarm` sha256
`a166e2b8759ba1d16db50651ed552c732e3340e5cb288d0297d421eee33d09c9`, binaire
livré du 9 novembre) :

- six essais, **0 déviation**, publication OK
  (`PAEA_B2_REPORT_PUBLISHED trials=6 p50=1.976s p95=2.350s`, hash du rapport
  `a0d2e550f9e606ae484449daf88d1ac827a170f4046ebe802d0edff9e3522c7c`) ;
- six lignes `PAEA_PILOT_BINARY_STABLE` (une par essai) ; vérification
  indépendante de l'evidence de recette : `{"integrity": "VERIFIED",
  "recorded_status": "PASS"}` ;
- variantes de refus vérifiées (exécutions directes de développement) :
  dossier existant → refus, hash faux → refus, binaire manquant →
  `binary-unreadable`, sans résidu ;
- chemin de déviation sondé : un hash de binaire faux au niveau
  `setUpClass` (binaire remplacé entre essais) est enregistré comme déviation
  avec cause `ValueError: existing-binary-hash-mismatch` et la publication
  est refusée ;
- evidence retenue sous `.swarm-fixtures/b2-recipe-run2/` (campagne) et
  `b2-recipe-evidence2/` (recette, non effaçable — originaux) ; jamais sous
  /tmp.

## 5. Ce que ce comparateur ne conclut pas

- Il mesure le **workflow durci B2**, pas Swarm. Aucun avantage de Swarm
  n'est déduit de ces six essais ; le candidat sera mesuré sur les mêmes
  scénarios avec le même contrat.
- L'indépendance des essais n'est pas établie : un essai = un puits neuf
  dans le même laboratoire same-UID ; aucune prétention de répétition
  statistique au-delà des durées mesurées.
- L'extension contention de T05/T17 du driver (processus réels
  chevauchants) n'est **pas** reprise dans le coureur B2 — le scénario B2
  reste l'ordre persisté ; la contention réelle reste qualifiée par la
  recette pilote complète.
- Le préenregistrement G6 (désignations humaines H3) reste la porte restante
  avant toute campagne comparée ; le paquet de revue est le dernier objet
  local de G6.

## 6. Suite concrète (reprise)

1. Assembler le **paquet de revue** G6 (dernier objet local) : inventaire
   fermé des livrables E3, empreintes, correspondances exigence→preuve.
2. La campagne comparée (candidat vs B2) attend le préenregistrement (H3)
   puis l'évaluateur désigné ; aucun cumul d'assertions n'en tient lieu.
3. Toute modification de `tests/paea_pilot_lab_test.py`, `pilot_lab.py`,
   `pilot_oracle.py` ou des fixtures `contract-v1.json` invalide ce résultat
   — la recette B2 les déclare en inputs et le refusera
   (`recipe-source-changed-during-archive` / evidence non reproductible).
