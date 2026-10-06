# Rapport — plan-01567e073c-T1 : Inspection ciblée et contrats

Tâche plan-01567e073c-T1 · mission w-01567e073c1ed2f3d4c71c9e · agent auto-832c66f95abe8d67a38a ·
tentative a-5621887914ca22818b315dfd · départ 1/2 · dépôt `/home/fpizzi/workspace/swarm-action-skills`.
Rôle worker, lecture seule (DO+CHECK bornés à l'inspection ; aucune modification de code effectuée).

## Résultat en deux phrases

Les quatre contrats bornés QW5/QW6/QW8/QW7 sont rédigés ci-dessous avec fichiers/contrôles réellement
identifiés dans ce dépôt (aucune capacité inventée), et l'instantané daté des consommations du lot
clôturé w-115c11e8f802a4f98c3def32 (révision 332, horodaté 2026-10-03T20:28:24Z) est reproduit à partir
de `CLOTURE-r332.json`. Rien n'est bloqué : les quatre contrats sont prêts pour revue avant démarrage de T2.

## Identité et portée

- Mission / tâche / tentative : w-01567e073c1ed2f3d4c71c9e / plan-01567e073c-T1 / a-5621887914ca22818b315dfd (1/2).
- Rôle : worker, lecture seule. Périmètre autorisé : points d'entrée budgets, mission status,
  independent review, attempt ledger ; `docs/plans/clear-launch-recovery/RETEX-CLOTURE.md` et
  `docs/plans/clear-launch-recovery/CLOTURE-r332.json`. Aucun inventaire global effectué.
- Révision courante du dépôt : HEAD `cc3069dc7bb61b90168d21f945cb2eb5e27578ed` (2026-10-03 22:56:09 +0200).
- État : `git status --porcelain` → une seule ligne `M docs/plans/clear-launch-recovery/RETEX-CLOTURE.md`
  (46 insertions, 0 suppression — ajout de la section « Enseignements complémentaires » et
  « Quick wins proposés » déjà présente dans le brief adopté). `git diff --check` → exit 0 (aucun
  espace blanc en erreur). Aucune autre modification locale.

## Contrats bornés — QW5, QW6, QW8, QW7

### QW5 — Bouton « Régler cette limite »

- **Fichiers/contrôles existants identifiés** : `budgets.go` (type `BudgetView`, `Policy`,
  `ActualCostScope` — lecture seule, pas de mutation de plafond) ; `budget_commands.go` (+
  `budget_commands_test.go`, commandes CLI budget) ; `managed_review_input_budget.go`,
  `planning_budget.go`, `prephase_budget.go` (+ tests respectifs) ; `web/cockpit.js` (bloc
  `budget-info`, affiche plafond/réservé/imputé/coût rapporté, **aucun bouton de réglage**) ;
  `web/prephase-resources.js` (`budgetView`, affichage seul) ; `tests/budget_ui.cjs`.
- **Constat vérifié (recherche, non supposé)** : `rg -i "Régler cette limite|adjust.?limit|AdjustLimit|SetLimit"`
  sur le dépôt (hors JSON/node_modules) ne retourne **aucune occurrence** en dehors de la citation du
  brief dans `RETEX-CLOTURE.md`. La capacité n'existe pas encore côté code — ni web ni CLI.
- **Résultat attendu** : depuis un blocage budget, bouton ouvrant le réglage exact de la
  mission/portée concernée (consommation, plafond, restant, effet de la reprise) ; confirmation
  explicite avant tout changement ; historique conservé ; reprise expliquée. Commande CLI
  équivalente avec portée correspondante.
- **Périmètre (fichiers autorisés proposés)** : `budgets.go`, `budget_commands.go`, `web/cockpit.js`,
  `web/prephase-resources.js`, fichiers de locale FR/EN concernés, tests Go et `tests/budget_ui.cjs`
  associés. Pas d'écriture directe dans `.swarm/state.db`.
- **Critères de vérification** : le bouton cible la bonne mission et la bonne limite ; aucun
  plafond ne change sans confirmation explicite ; l'historique est conservé ; la reprise nécessaire
  est expliquée ; la CLI indique la commande et la portée correspondantes.
- **Preuve attendue** : captures FR/EN × 2 thèmes × clavier/focus pour le parcours web ; sortie CLI
  avant/après avec code de sortie ; `git diff --check` sur le candidat.
- **Condition de reprise** : si aucun point d'entrée moteur de mutation de plafond n'est trouvé côté
  store/engine lors de l'implémentation, documenter cause/fichier manquant et remonter au responsable
  plutôt que d'inventer un endpoint.

### QW6 — État actuel vs historique

- **Fichiers/contrôles existants identifiés** : `mission_status.go` (commentaire L777 « Do not turn
  their historical state into a new instruction or mutate it. » ; L779 « Échanges conservés dans
  l'historique de la mission terminée. ») ; `mission_insights.go` (plusieurs messages dédiés, ex.
  L638 « Ces événements sont historiques ; l'état actuel de la tâche fait foi. », L626, L723) ;
  `result_presentation.go` (L93 « L'acceptation historique n'est plus fondée sur des preuves
  actuelles. ») ; `mission_status_test.go`, `mission_insights_test.go` ; `web/mission.js`,
  `web/mission-insights.js`.
- **Constat vérifié** : une logique de séparation historique/actuel existe déjà partiellement (messages
  et commentaires ci-dessus). Mais la clôture datée **2026-10-03T20:28:24Z** (`CLOTURE-r332.json`,
  champ `limitations`) liste explicitement : *"Historical planner diagnostics remain displayed even
  after all six tasks are accepted."* — gap résiduel confirmé à cette date, non vérifié corrigé depuis
  dans ce dépôt (aucune commande de recette rejouée en T1, hors périmètre lecture seule). Ne pas
  supposer ce gap résolu ni toujours actif sans rejouer la recette décrite dans
  `RETEX-CLOTURE.md` L18.
- **Résultat attendu** : une mission clôturée affiche son état courant sans ancien blocage actif ;
  un blocage réellement courant reste visible ; les événements historiques ne sont ni supprimés ni
  réécrits. Même distinction en CLI.
- **Périmètre proposé** : `mission_status.go`, `mission_insights.go`, `result_presentation.go`,
  `web/mission.js`, `web/mission-insights.js`, tests associés.
  Mission `w-115c11e8f802a4f98c3def32` reste clôturée : ne pas la relancer pour servir de fixture.
- **Critères de vérification** : mêmes que ci-dessus (identiques au brief QW6).
- **Preuve attendue** : captures FR/EN × 2 thèmes sur une mission clôturée réelle (pas relancée) et
  une mission avec blocage réellement courant ; parcours CLI équivalent sur racine isolée.
- **Condition de reprise** : si le gap cité par `CLOTURE-r332.json` est toujours observable après
  correction, documenter l'écart précis (capture + sélecteur/texte affiché) avant nouvelle tentative.

### QW8 — Contrôle déterministe du dossier avant revue

- **Fichiers/contrôles existants identifiés** : `independent_review_documents.go` (erreur L49
  « preuve documentaire inaccessible : %s ») ; `independent_validation_evidence.go` (erreur L20
  « contrôles moteur courants requis avant la revue ») ; `evidence_contract_test.go`,
  `evidence_projection.go` ; `independent_review.go`, `independent_review_runtime.go`,
  `independent_review_images.go` (+ tests respectifs) ; `web/evidence-contract.js`.
- **Constat vérifié** : un contrôle préalable partiel existe déjà (accessibilité documentaire,
  contrôles moteur courants requis). Reste à vérifier, sans appel fournisseur, la couverture exacte
  vis-à-vis du contrat QW8 (accessibilité réelle au contexte du vérificateur réel + fraîcheur
  contractuelle déterministe) — non confirmé complet ni absent à ce stade, à instruire en tâche dédiée.
- **Résultat attendu** : dossier incomplet ou périmé → motif et action avant tout appel fournisseur ;
  dossier admissible → peut poursuivre la revue indépendante ; le contrôle préalable ne produit aucun
  verdict de qualité ni acceptation. Même règle moteur web/CLI.
- **Périmètre proposé** : `independent_review_documents.go`, `independent_validation_evidence.go`,
  `evidence_projection.go`, tests associés, `web/evidence-contract.js`.
- **Critères de vérification** : motif explicite et action sans appel fournisseur sur dossier
  inadmissible ; dossier admissible poursuit sans verdict anticipé ; pas de critère demandant son
  propre futur verdict (C4).
- **Preuve attendue** : cas dossier admissible et cas dossier inadmissible (preuve absente / périmée),
  CLI et web, avec code de sortie et message exact.
- **Condition de reprise** : si le contrôle existant couvre déjà entièrement le contrat QW8, le
  signaler au responsable comme déjà couvert plutôt que dupliquer une vérification.

### QW7 — Résumé d'effort par tâche

- **Fichiers/contrôles existants identifiés** : `mission_insights.go` (vue par rôle/tâche :
  `recorded_calls`, `observed_tool_calls`, jetons, coût, fallback « coût réel non rapporté » /
  « Jetons non rapportés ») ; `web/mission-insights.js` (rendu correspondant) ; `web/graph.js`
  (L53, L174 : « coût de la tâche : non rapporté ») ; `web/conduite.js` (L92, L164 : « coût réel non
  rapporté ») ; `costs_test.go` (cas « rien de rapporté » → Silent, pas de zéro inventé).
- **Constat vérifié** : une bonne partie du contrat QW7 existe déjà et est testée (coût rapporté par
  tâche, mesures absentes affichées comme « non rapporté », pas de coût nul inventé — cohérent avec
  `costs_test.go`). Non confirmé dans ce périmètre de lecture : affichage conjoint durée + reprises
  par tâche dans un résumé unique accessible à la fois web et CLI (recherche ciblée non faite sur la
  « durée » faute de budget d'appels restant en exploration initiale — à vérifier en tâche dédiée,
  absence non assimilée à zéro ni à un manque confirmé).
- **Résultat attendu** : durée, outils observés, revues, reprises et coût rapporté par tâche ;
  chiffres concordant avec les traces, datés/révisionnés, tentatives distinguées ; mesures absentes
  = « non rapportées » ; aucun coût nul ni total de jetons inventé ; accessible web et CLI.
- **Périmètre proposé** : `mission_insights.go`, `web/mission-insights.js`, `web/graph.js`,
  `web/conduite.js`, `costs_test.go` et équivalent CLI.
- **Critères de vérification** : identiques au brief QW7 (concordance, datation, distinction des
  tentatives, pas de valeur inventée, parité web/CLI).
- **Preuve attendue** : capture web + sortie CLI du même résumé pour une même tâche/tentative, avec
  au moins un cas « mesure non rapportée » visible.
- **Condition de reprise** : si durée/reprises par tâche n'existent dans aucun fichier listé
  ci-dessus, le signaler explicitement comme absence constatée (pas une improvisation de calcul).

## Instantané daté des consommations précédentes

Source : `docs/plans/clear-launch-recovery/CLOTURE-r332.json`, champ `recorded_at`
**2026-10-03T20:28:24.514440+00:00**, `work_id` `w-115c11e8f802a4f98c3def32`, révision **332**,
candidat `a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77`. Valeurs reproduites
telles qu'enregistrées, sans recalcul ni somme ajoutée (C5 — unités distinctes, pas de total inventé).

| Rôle / tâche | Label | Appels enregistrés | Appels d'outils observés | Jetons entrée/sortie | Coût rapporté | Tentatives sans coût |
| --- | --- | --- | --- | --- | --- | --- |
| planner / root | Responsable : root | 5 | 0 | 6 / 18077 | 0.451902 USD (3 tentatives) | 2 |
| reviewer / reviewer | Vérificateur indépendant (agrégat revues 41→45) | 45 | 0 | 128 / 571838 | 12.820693 USD (42 tentatives) | 3 |
| worker / plan-115c11e8f8-T0 | Inspection ciblée lecture seule | 2 | 27 | 6 / 5404 | 0.2851286 USD (1 tentative) | 1 |
| worker / plan-115c11e8f8-T1 | REQ-QW1 — Résumé avant lancement | 3 | 51 | 38 / 30589 | 1.0946986 USD (2 tentatives) | 1 |
| worker / plan-115c11e8f8-T2 | REQ-QW2 — Blocage expliqué | 2 | 37 | 20 / 16528 | 0.6520226 USD (1 tentative) | 1 |
| worker / plan-115c11e8f8-T3 | REQ-QW3 — Bilan par tentative | 2 | 37 | 14 / 13333 | 0.4424448 USD (1 tentative) | 1 |
| worker / plan-115c11e8f8-T4 | REQ-QW4 — Reprise ciblée après refus/blocage | 2 | 37 | 16 / 12131 | 0.4412018 USD (1 tentative) | 1 |
| worker / plan-115c11e8f8-T5 | Recette globale et RETEX | 1 | 12 | 10 / 17226 | 0.7231206 USD (1 tentative) | 0 |

Note du fichier source (`spending.note`, reproduite telle quelle) : « Les appels d'outils, les
appels IA et les contrôles du moteur sont des mesures différentes. Une mesure absente n'est pas un
zéro ; les relances internes du fournisseur ne sont pas toutes observables. Une réservation d'appel
ne prouve pas son envoi au fournisseur. » — `recorded_control_executions`: 154 ; `worker_retries`: 5
(compteur global de reprises, non ventilé par tâche dans ce fichier).

Revue indépendante clôturante (tentative unique, distincte de l'agrégat ci-dessus) : id
`review-666a677c8bd7cf73ece8ed18`, attempt `a-1637354a070e743f69f201a7`, état `passed`, démarrée
`2026-10-03T20:23:55.523301367Z`, terminée `2026-10-03T20:26:02.46631353Z`, usage rapporté : coût
`0.4463234` USD, `output_tokens` 12683, `input_tokens` 4 — source déclarée « événement fournisseur
result », portée « champs bruts du dernier événement ; caches séparés, sans somme ni cumul de
session ». Cette tentative unique ne doit pas être confondue avec la ligne agrégée « reviewer » du
tableau ci-dessus (45 appels, 12.82 USD cumulés sur les revues 41 à 45 du lot).

Mesures explicitement absentes dans la source (ne pas traiter comme zéro) : durée par tâche/tentative
(non enregistrée dans ce fichier), ventilation des 5 `worker_retries` par tâche, contenu détaillé des
journaux `RETEX-supervision.md` et `RETEX-review43-timeout.md` (cités par `RETEX-CLOTURE.md` mais
non ouverts dans cette tâche — hors périmètre assigné à T1, accessibilité à vérifier avant usage en T6
selon la limite déjà déclarée dans le brief).

## Vérifications effectuées

| Contrôle | Commande | Résultat | Preuve |
| --- | --- | --- | --- |
| Racine de travail | `pwd` ; `git rev-parse --show-toplevel` | Identiques, `/home/fpizzi/workspace/swarm-action-skills` | sortie commande |
| Existence RETEX-CLOTURE.md | `test -f ...` | Trouvé | sortie commande |
| Existence CLOTURE-r332.json | `rg --files -g '*CLOTURE-r332*'` | Trouvé, exit 0 | sortie commande |
| QW5 — absence bouton/commande réglage limite | `rg -i "Régler cette limite\|adjust.?limit\|AdjustLimit\|SetLimit"` (hors JSON/node_modules) | Aucune occurrence hors citation du brief | sortie commande |
| Fichiers budgets/mission_status/independent_review/attempt_ledger | `rg --files -g '*budget*' -g '*mission_status*' -g '*independent_review*' -g '*attempt_ledger*' -g '*mission_insights*' -g '*result_presentation*'` | Liste de fichiers ci-dessus | sortie commande |
| Symbole AttemptLedger | `rg -n "AttemptLedger" --type go -l .` | Présent dans `mission_insights.go` et `attempt_ledger_test.go` seulement (pas de fichier `attempt_ledger.go` dédié — à ne pas supposer) | sortie commande |
| QW6 — messages historique/actuel | `rg -n "historique\|historical\|incident" -i mission_status.go mission_insights.go result_presentation.go` | Occurrences listées ci-dessus | sortie commande |
| QW7/QW8 — fichiers effort et preuve | `rg` ciblés sur « résumé d'effort », « dossier de revue », `independent_review_documents.go`, `independent_validation_evidence.go` | Occurrences listées ci-dessus | sortie commande |
| Intégrité du diff local | `git status --porcelain` ; `git diff --stat` ; `git diff --check` | 1 fichier modifié (RETEX-CLOTURE.md, +46/-0), `git diff --check` exit 0 | sortie commande |

Aucun test Go/vet/frontend relancé : aucune ligne de code modifiée dans cette tentative (lecture
seule conforme au périmètre T1) ; non applicable, pas « non testé » par omission.

## Limites et non testé

- Les journaux `RETEX-supervision.md` et `RETEX-review43-timeout.md` restent cités mais non ouverts
  (hors périmètre explicite de T1) ; leur accessibilité réelle reste à vérifier avant usage en T6,
  comme déjà noté dans le brief.
- Couverture exacte de QW8 par le code existant (fraîcheur contractuelle précise, contexte vérificateur
  réel) non confirmée complète ni absente — nécessite inspection dédiée en tâche d'implémentation.
- Présence de « durée par tâche » et « reprises par tâche » dans un résumé unique (QW7) non confirmée
  par recherche de chaîne dédiée (budget d'exploration initial atteint) ; à vérifier en tâche dédiée,
  absence non assimilée à une confirmation de manque.
- Aucune recette web/CLI rejouée (FR/EN, thèmes, clavier) dans cette tâche : hors périmètre lecture
  seule de T1, prévue dans les contrats QW ci-dessus pour T2–T5.

## Prochaine action

Responsable : valider ou amender les quatre contrats ci-dessus avant tout démarrage de T2 (ordre
QW5 → QW6 → QW8 → QW7, dépendances séquentielles du plan). Aucune délégation, acceptation ni
modification de budget effectuée par cette tentative. Revue indépendante de ce contrat à mener dans
un contexte séparé avant lancement de T2.
