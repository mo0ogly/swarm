# Paquet de revue G6 — remise aux tiers désignés

9 novembre 2026 · [English](../../../en/plans/actions-confiance/paea/REVIEW-PACKAGE-20261109.md)

Lot G6 du [plan de clôture des écarts](GAP-CLOSURE-PLAN-20261103.md) : « Paquet
de revue, contrats/vecteurs, collecte et comparateur B2 avec mêmes protections
métier ». Avec ce document, les quatre objets G6 locaux sont livrés : contrats
et 14 vecteurs, collecte (incréments 14-25), comparateur
[B2](B2-COMPARATOR-20261109.md) et le **paquet de revue** — le dossier fermé
remis aux tiers désignés (porte H3) au moment de leur acceptation de mission.

## 1. Ce que le paquet est — et n'est pas

`tools/paea/review_package.py` assemble l'inventaire fermé des livrables de
préparation E3 présents dans le dépôt, chaque entrée avec son empreinte SHA256
exacte et sa taille en octets, publiés atomiquement via
`recipe.py::publish` dans `paea-review-package.json` (schéma v1). Trois
dossiers, la structure exigée par [PILOTE-OPERATIONNEL](PILOTE-OPERATIONNEL.md)
(conception / réalisation / qualification) :

- **conception** (7 entrées) : protocole E3, guide intégré, dossier des pilotes
  opérationnels, plan de clôture, tableau de bord, inventaire des voies G3a,
  livrable comparateur B2 ;
- **réalisation** (21 entrées) : les sept modules `tools/paea/`, le driver de
  test pilote, le helper de contention, les fixtures `contract-v1.json` /
  `payment-registry-v1.json` / `payment-request-v1.json`, et les sources
  `rust/crates/swarm-protection-contract/src/*.rs` ;
- **qualification** (5 entrées) : les recettes maintenues `paea-contract`,
  `paea-pilot-lab`, `paea-pilot-contention`, `paea-laboratory-evidence` et
  `paea-b2-comparator`.

Le manifeste porte des étiquettes honnêtes explicites : il **prouve**
l'existence et l'identité en octets au moment de l'assemblage ; il ne prouve
ni qualité, ni acceptation, ni évaluation indépendante, ni préenregistrement,
ni corpus réservé. Les portes de remise restent H3 (désignation évaluateur +
seconde implémentation) et H4 (critères métier, essais humains). Il ne
remplace jamais le préenregistrement ni la campagne comparée.

## 2. Contrat de l'assembleur

- Tout livrable manquant ou vide **refuse** l'assemblage
  (`review-package-deliverable-missing` / `-empty`) ; un glob qui ne matche
  rien est le même refus qu'un chemin absent — un inventaire à moitié n'est
  jamais publié.
- **Double collecte** : deux passes complètes doivent produire les mêmes
  empreintes ; toute source modifiée pendant l'assemblage (arbre partagé,
  session concurrente) refuse avec les chemins divergents listés
  (`review-package-source-changed-during-assembly`) et rien n'est publié.
- Evidence dans le dossier de `SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE` (créé par
  le script, doit être neuf) — jamais sous /tmp.
- Le paquet est **reproductible** : deux exécutions sans changement de source
  produisent des manifestes identiques octet pour octet.

## 3. Résultat réel (9 novembre 2026)

- 33 livrables (7 + 21 + 5), 324 890 octets, manifeste sha256
  `f981d9b7e2d4d7a85ca65b0e8a4ebbb71b4bc63c564ba91c52af43ce148b378b` — trois
  exécutions successives byte-identiques, dont la voie publique
  (`review-package-run1/`, `run2/`, campagne de recette `rp-recipe-campaign2/`
  sous `.swarm-fixtures/`) ;
- exécution publique : recette `tools/verification/recipes/paea-review-package.json`
  PASS (evidence `rp-recipe-evidence2/`), vérification indépendante
  `{"integrity": "VERIFIED", "recorded_status": "PASS"}` ; le premier
  lancement sans `SWARM_PAEA_REVIEW_PACKAGE_EVIDENCE` est conservé FAIL
  (`rp-recipe-evidence/`) — le refus sans variable fait partie du contrat ;
- variantes de refus vérifiées : livrable manquant → refus ; livrable vide →
  refus ; variable d'evidence absente ou dossier déjà existant → refus.

### Requalification après le merge 197d663f6 — 8 novembre 2026

Le merge `197d663f6` puis les documentations de requalification ont changé
cinq entrées déclarées de la recette (`GAP-CLOSURE-PLAN-20261103.md` FR/EN,
`REVIEW-PACKAGE-20261109.md` FR/EN, `DASHBOARD-T20261101.md` FR —
inventaire par diff des `inputs.before.json`, 44 entrées des deux côtés).
Le run `rp-recipe-evidence2/` reste VERIFIED sur son instantané gelé
(manifeste `f981d9b7…`, 9 novembre) ; il n'est pas transposé à l'arbre
courant. Rejeu `rp-run3` sous
`.swarm-fixtures/requal-merge-20261108/rp-run3/` (evidence `rp-run3-obs/`,
créé par le script) : PASS, `source_stable:true`, 33 livrables /
330 442 octets, manifeste sha256
`a29de65beba8f6540463b257f8721cd12944ae22320df2617422efda9eb38d5f`,
vérification indépendante `{"integrity": "VERIFIED",
"recorded_status": "PASS"}`. Un lancement intermédiaire avec dossier
d'evidence pré-créé est conservé FAIL (`rp-run2/`) : la variante de refus
« dossier déjà existant » reste vérifiée sur la voie publique après le
merge. L'ancien manifeste `f981d9b7…` reste l'offre historique de l'époque
de son assemblage.

## 4. Remise — condition concrète de reprise

La remise effective attend la désignation H3 (l'utilisateur a confirmé le
25 octobre ne disposer ni d'évaluateur indépendant ni d'équipe tierce ; aucun
rôle n'est inventé). Quand les tiers sont désignés : régénérer le manifeste
(les empreintes de ce jour deviennent l'offre), joindre
`paea-review-package.json` + les livrables listés + [E3-PROTOCOL](E3-PROTOCOL.md)
comme mission, et conserver la preuve d'acceptation de chaque tiers. Toute
modification ultérieure d'un livrable invalide l'empreinte du manifeste — le
paquet se régénère, il ne se met pas à jour à la main.

## 5. Ce qui reste ouvert dans G6

- Le **préenregistrement** (porte H3/H4) : identité et acceptation de
  l'évaluateur, corpus réservé avec provenance, volumes/ordre/seeds/règles
  d'arrêt, critères métier. Aucun de ces éléments n'est fabrique localement.
- La campagne comparée (candidat vs B2 vs seconde implémentation) : porte
  PA-12, après préenregistrement.
- Une matrice d'invariants régénérée sera jointe au paquet au moment de la
  remise (note delta du 7 novembre, section « Suivi du 7 novembre (fin) »).
