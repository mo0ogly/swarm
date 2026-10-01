# T1 — Inventaire des mécanismes existants (Lot 0)

Statut : mis à jour par la 4e tentative (agent `auto-86f3303ff0451e280da8`,
attempt `a-d2c72df363d076d4e5134554`, départ 4/4, révision du travail 28).
REQ-ADM-01 à 04, `RunLimits`, mission supprimée, i18n, thèmes, CI, diagnostic,
tarifs : contenu des tentatives précédentes relu et conservé (toujours vrai à
la vérification de cette tentative, non re-vérifié ligne à ligne). REQ-ADM-05/
06/07 et les 5 quick wins, déclarés « non trouvés » par les rapports
précédents faute de définition transmise, sont **maintenant définis dans le
CONTRAT COURANT / BRIEF COMMUN de cette tentative** : les sections
correspondantes ci-dessous ont été corrigées et complétées par lecture de code
ciblée sur les lacunes, pas par une reprise complète de l'inventaire.

## Portée et méthode

Lecture seule sous `/home/fpizzi/workspace/swarm-engine-contract/source`
(vérifié `pwd` et `git rev-parse --show-toplevel` en début de tentative,
branche `codex/engine-review-contract`). Aucune commande d'écriture
applicative exécutée. Seuls `docs/T1-inventaire-existant.md` (ce fichier) et
`docs/plan-843bb3ce22-T1.md` ont été modifiés par cette tentative — voir
« Aucune modification de code applicatif » pour la preuve `git status`.
Exploration par `rg` ciblée sur les lacunes identifiées (ADM-05/06/07, quick
wins), pas de nouveau balayage général du dépôt.

## REQ-ADM — points d'extension identifiés

| ID | Compréhension | Mécanisme existant réutilisable | Preuve |
| --- | --- | --- | --- |
| REQ-ADM-01 | Explicite (brief) : rubrique Administration web des limites agents | Pattern déjà en place pour budgets/quotas/pricing/providers : `Store` (SQLite) + handler HTTP dédié enregistré une seule fois dans `web_server.go`, JS dédié dans `web/` | `provider_admin.go:134 registerProviderAdmin(mux,...)`, appelé une fois en `web_server.go:386`; `web/providers.js`, `web/quotas.js`, `web/pricing.js` existants |
| REQ-ADM-02 | Explicite (brief) : équivalent CLI de l'Administration | Pattern déjà en place : fonctions `*CLI(pos, input, out)` sur `*Store`, distinctes des handlers web mais lisant/écrivant le même `Store` | `budget_commands.go:72 budgetCLI`, `quotas.go:172 quotasCLI`, `pricing.go:198 pricingCLI` |
| REQ-ADM-03 | Explicite (brief) : réutilisation des mécanismes existants, aucune seconde source de vérité | **Confirmé par lecture de code** — voir section dédiée ci-dessous | voir « Confirmation REQ-ADM-03 » |
| REQ-ADM-04 | Explicite (brief) : unité, bornes, valeur héritée vs effective, portée projet/mission/rôle/tâche | Mécanisme déjà existant pour les tarifs (`pricing.go`) ; pour les limites agents, `RunLimits.normalized()`/`tightened()`/`cappedBy()` portent déjà bornes, valeur par défaut (0 = héritée) et plafonnement par une valeur supérieure (fournisseur/mission/tentative précédente) | `run_limits.go:14 normalized` (bornes+défaut), `:48 tightened` (mission ne peut que durcir, jamais relâcher), `:76 cappedBy` (plafond par une valeur antérieure) |
| REQ-ADM-05 | Explicite (brief) : effet distinct sur futurs départs vs tentatives en cours | **Confirmé par lecture de code**, déjà vrai pour `RunLimits` : les limites sont figées au lancement, une modification de configuration n'affecte que les nouvelles tentatives | `run_limits.go:5` commentaire de code : « Limits are frozen in each launch; configuration changes affect new attempts only. » ; matérialisé par `executionDirectives()` (`run_limits.go:29-44`) qui fige les valeurs de `RunLimits` dans le texte de cadrage transmis à chaque tentative au lancement |
| REQ-ADM-06 | Explicite (brief) : preview changement, validation moteur, historique, retour à la config précédente | **Partiel.** Preview : pattern déjà réutilisable (`previewQuotas`, probablement `previewBudget` cité par la tentative précédente). Validation moteur : `tightened()` refuse une limite mission supérieure à la limite fournisseur (erreur), `normalized()` refuse une valeur hors bornes (erreur). **Historique et retour à la config précédente : aucun mécanisme trouvé** dans `budgets.go`/`quotas.go`/`run_limits.go` — recherche `History`/`Rollback` vide (seule occurrence : `tx.Rollback()` transactionnel SQL en `budgets.go:109`, sans rapport avec un historique de configuration). **Gap réel à combler en T2+**, pas une absence de recherche. | `quotas.go:143 previewQuotas` ; `run_limits.go:48 tightened`, `:14 normalized` ; `rg -n "History\|Rollback" budgets.go quotas.go run_limits.go` → aucun résultat pertinent |
| REQ-ADM-07 | Explicite (brief) : pas de secret affiché, pas de reset conso, pas d'auto-hausse budget, pas de suppression de protection, coût non rapporté ≠ 0 | **Majoritairement conforme par absence de mécanisme à risque, plus un pattern positif direct.** Secrets : `provider_admin.go:56` exclut explicitement secrets/permissions des exécutables du payload d'administration. Reset conso : recherche `Reset` vide dans `budgets.go`/`quotas.go`/`run_limits.go`/`agents_store.go` — rien à dupliquer ni désactiver. Auto-hausse budget : recherche `Increase.*Budget`/`autoRaise` vide — aucun mécanisme existant. Coût non rapporté ≠ 0 : `quoteRate` renvoie `Status:"unknown"` par défaut plutôt qu'un montant implicite — pattern déjà conforme, réutilisable tel quel pour les futurs coûts liés à `RunLimits` si pertinent. Suppression de protection : **NOT TESTED**, aucune recherche ciblée effectuée sur ce point précis dans cette tentative. | `provider_admin.go:56` ; `rg -n "Reset" budgets.go quotas.go run_limits.go agents_store.go` → vide ; `rg -n "Increase.*Budget\|autoRaise" *.go` → vide ; `pricing.go:152-160 quoteRate` (`Status: "unknown"`) |

### Limites agents (`RunLimits`) — objet central de la rubrique Administration

`RunLimits` (`run_limits.go:6`) est le type qui porte les limites d'exécution
d'un agent (normalisation en `:14`, durcissement en `:48 tightened`, plafonnage
en `:76 cappedBy`). Il est déjà consommé par `task_actions.go:38`, `model.go:161`,
`loop_guard.go:20/37`, `agents_store.go:32/121/171/521/862` et
`interruption_record.go:23`. **Constat clé (confirmé, inchangé depuis la
tentative précédente)** : contrairement à budgets/quotas/pricing/provider-admin,
`RunLimits` n'a **aucune** fonction `*CLI` ni `register*Admin` dédiée
aujourd'hui — il est configuré uniquement via la création de mission/tâche, pas
via une surface d'administration autonome. C'est très probablement le vide
fonctionnel que REQ-ADM-01/02 demandent de combler, **en répliquant le pattern
existant** (ajouter des méthodes sur `*Store` + `runLimitsCLI` +
`registerRunLimitsAdmin`) plutôt qu'en créant un nouveau mécanisme de stockage.

**Découverte utile pour cette tentative** : `executionDirectives()`
(`run_limits.go:29-44`) est la fonction qui génère, à partir des valeurs
courantes de `RunLimits`, le texte « CADRE D'EXÉCUTION » transmis à chaque
tentative de tâche (celui reçu par cette tentative elle-même). C'est un
mécanisme moteur légitime déjà existant, pas une anomalie : il illustre
concrètement REQ-ADM-05 (les valeurs sont figées dans le texte au moment du
lancement d'une tentative) et donne un point d'ancrage direct pour REQ-ADM-04
(bornes/valeur effective) si une future admin `RunLimits` doit prévisualiser
l'effet d'un changement sur le texte réellement transmis aux agents.

### Confirmation REQ-ADM-03 — absence de seconde source de vérité

Pour les quatre familles déjà administrables (budgets, quotas, pricing,
provider-admin), le web et le CLI appellent la **même** méthode de `*Store` ;
aucune valeur n'est dupliquée dans un fichier de config séparé :

- Budget : web `web_server.go:100-105` (`kind == "budget"/"budget-preview"`) et
  CLI `budget_commands.go:72 budgetCLI` appellent tous deux
  `s.previewBudget`/`s.configureBudget` (`budget_commands.go:38,60`), qui lisent/
  écrivent via `writeBudget`/`reserveBudget` (`budgets.go:116,130`) sur la table
  SQLite du `Store`.
- Quotas : web `web_server.go:90-98` et CLI `quotas.go:172 quotasCLI` appellent
  toutes deux `s.previewQuotas`/`s.configureQuotas` (`quotas.go:143,165`).
- Pricing : CLI `pricing.go:198 pricingCLI` et web `pricing.go:237 registerPricing`
  partagent `s.rateCatalogue`/`s.saveRate` (`pricing.go:60,110`).
- Provider admin : `provider_admin.go:134 registerProviderAdmin` expose
  `s.providerAdminState`/`s.changePolicies` (`:22,58`) ; enregistré une seule
  fois (`web_server.go:386`), pas de route dupliquée trouvée.
- Aucun fichier YAML/JSON statique dupliquant budgets/quotas/tarifs/limites n'a
  été trouvé en dehors de la base `Store` (recherche `rg` sur `Makefile`,
  `package.json`, `scripts/*` sans résultat de génération parallèle pour ces
  familles).

**Conclusion vérifiée, confirmée à nouveau par cette tentative** : pour les
mécanismes déjà administrés, il n'existe qu'une seule source de vérité (le
`Store` SQLite), lue et écrite indifféremment par le CLI et le web.
Recommandation pour T2+ : appliquer strictement le même schéma pour
`RunLimits` (pas de nouveau fichier de config, pas de second stockage).
Limite inchangée : cette confirmation porte sur les 4 familles déjà
administrées ; elle ne préjuge pas d'un futur écart si une implémentation de
`RunLimits` admin introduisait par erreur un second stockage — à vérifier de
nouveau lors de la revue de la tâche qui l'implémentera.

## Gestion mission supprimée (hors quick wins, dans le périmètre local)

Mécanisme unique trouvé, dans le même `Store`/SQLite (pas de second stockage) :
table `mission_trash` (`lifecycle.go:24`), alimentée par une opération de
suppression (`lifecycle.go:740 INSERT INTO mission_trash...`), listée par
`lifecycle.go:142-155`, restaurée via `trashRestorePreview` (`:268`), purgée par
`lifecycle.go:796 DELETE FROM mission_trash`. Commentaire de code confirmant
l'intention produit : « Deleted missions come only from the recoverable trash;
archived missions... » (`lifecycle.go:110`). Mécanisme distinct de l'archivage
zip (`archive.go`, export/import de bundle) — à ne pas confondre.

Côté web, `web/lifecycle-manager.js:27` affiche déjà un message dédié quand une
mission sélectionnée est à l'état `trash` (« Cette mission est récupérable :
Restaurer remet toutes ses données internes en service. ») et
`web/cockpit.js:197` gère l'état vide global (« Aucune mission active... »).
Voir aussi QW2 ci-dessous pour le cas précis d'un lien direct vers une mission
supprimée.

## i18n FR/EN

- Go : `i18n.go:27 uiText`, `:101 uiEngineText`, source unique `locales/en.json`
  (320 Ko).
- Web : `web/i18n.js` (4,1 Ko, moteur) + `web/i18n-en.js` (320 Ko, données de
  traduction). Les deux fichiers `locales/en.json` et `web/i18n-en.js`
  apparaissent modifiés ensemble dans l'arbre de travail (`git status`, état
  préexistant à cette tentative, non produit par elle), ce qui suggère une
  synchronisation manuelle ou un script non trouvé par cette recherche.
  **Limite inchangée** : aucun script de génération reliant les deux fichiers
  n'a été localisé (recherche dans `Makefile`, `package.json`, `scripts/*`
  sans résultat) — à vérifier explicitement avant d'ajouter de nouvelles
  chaînes pour ne pas créer une vraie seconde source de vérité côté i18n.

## Thèmes

- CLI/terminal : `terminal_style.go:170 terminalLightTheme`, `:183 toggleTerminalTheme`.
- Web : `web/wattson_themes.css` (7,9 Ko), présent une seule fois dans `web/`.
Les deux thèmes (clair/sombre) existent déjà des deux côtés ; toute nouvelle UI
d'administration doit réutiliser ces tokens plutôt qu'en créer de nouveaux.

## Recette navigateur / CI web

- Outillage Puppeteer déjà en place et déjà décidé comme réutilisable :
  `tests/*_ui.cjs` (ex. `budget_ui.cjs`, `pricing_ui.cjs`, `quotas_ui.cjs`,
  `i18n_ui.cjs`, `task_models_ui.cjs`, `role_models_ui.cjs`), lancés via
  `package.json` (`test:budgets`, `test:pricing`, `test:quotas`, `test:i18n-ui`,
  `test:task-models`, `test:role-models`), chacun avec le pattern
  `node tests/X_ui.cjs bin/swarm test-results/X`.
- **Écart constaté (inchangé)** : `.github/workflows/ci.yml` (job `checks`)
  n'exécute que `npm test`, qui lui-même ne lance que
  `pilot_graph_test.cjs`, `mission_overview_test.cjs`, `cockpit_refresh_test.cjs`,
  `audit_acceptance_runner_test.cjs` et `test:i18n` (`package.json:5`). Les
  recettes navigateur `test:budgets`/`test:pricing`/`test:quotas`/`test:i18n-ui`/
  `test:task-models`/`test:role-models` existent mais **ne tournent pas en CI**
  aujourd'hui. Pour QW3/REQ-CI-01, il faudra créer une recette du même type
  (`tests/run_limits_ui.cjs` ou équivalent, FR/EN x 2 thèmes) **et** la relier
  explicitement à `ci.yml` (ou à la chaîne `npm test`) — point à traiter dans
  une tâche d'implémentation, pas dans ce lot documentaire.

## Tarifs fournisseur (détail, cf. REQ-ADM-04)

Mécanisme déjà consommé ailleurs, à réutiliser sans collecte externe :
`pricing.go` (`rateCatalogue`, `validateRate`, `saveRate`, `quoteRate`,
`pricingCLI`, `registerPricing`). Aucune valeur codée en dur trouvée à côté.
Confirmé cette tentative : `quoteRate` (`pricing.go:152-160`) initialise
`Status: "unknown"` et renvoie une erreur explicite si les jetons d'entrée/
sortie ne sont pas renseignés — un tarif absent du catalogue produit donc un
état distinct de zéro, pas une valeur 0 implicite. Pertinent aussi pour
REQ-ADM-07 (« coût non rapporté ≠ 0 ») : le pattern existe déjà et peut être
réutilisé tel quel si `RunLimits` admin doit un jour rapporter un coût.

## Diagnostic

Mécanismes existants identifiés (à réutiliser, pas à dupliquer) :
`attempt_diagnostic.go` (`failureCategory`, `buildAttemptDiagnostic`,
`fallbackAttemptDiagnostic`, `requiresEnvironmentVerification`) et
`runtime_health.go` (`runtimeHealth`, `storageGuard`). Voir QW5 ci-dessous pour
le bouton « copier le diagnostic » côté web.

## 5 quick wins Swarm

Définis dans le CONTRAT COURANT / BRIEF COMMUN de cette tentative (absents des
tentatives précédentes, qui les avaient donc correctement signalés comme non
vérifiables). Chaque ligne distingue le mécanisme réutilisable trouvé du gap
réel à combler.

| Quick win | Exigence | Mécanisme existant réutilisable | Gap constaté |
| --- | --- | --- | --- |
| QW1 — version + comparaison | REQ-VER-01/02 | **Aucun trouvé.** Recherche `"version"`/`buildVersion`/`serverVersion` dans `web_server.go` et `*.go` : occurrences trouvées sans rapport (schémas JSON internes de `managed_review_sources.go`, `assist_prompt.go`, fichiers de test). | Endpoint/affichage de version serveur à créer ; vérifier d'abord l'existence d'un identifiant de build (ldflags Go, `git describe`) injecté à la compilation — non recherché dans ce lot (limite). |
| QW2 — lien mission supprimée | REQ-LINK-01 | Partiel : `web/lifecycle-manager.js:27` (message état `trash`) et `web/cockpit.js:197` (état vide global avec renvoi vers « Gérer les missions »). | Cas précis d'un lien direct (`?work=<id>` supprimé) avec message clair + CTA « Choisir une mission » explicite : non confirmé. Recherche `"mission introuvable"`/`404`/`not-found` dans `web_server.go` et `web/cockpit.js` vide, mais limitée à 2 fichiers — absence non prouvée globalement. |
| QW3 — recette CI FR/EN x thèmes | REQ-CI-01 | Voir section « Recette navigateur / CI web » ci-dessus : outillage Puppeteer existant, dont `i18n_ui.cjs` couvre déjà un parcours FR/EN x thèmes générique. | Brancher une recette dédiée à l'Administration `RunLimits` sur `ci.yml` ; le gap est le branchement CI, pas l'outillage. |
| QW4 — résumé pré-lancement | REQ-SUM-01 | Partiel : `web/pilot-actions.js:11-14` construit déjà un bloc `#prepared-launch-summary` (section `notice info field-wide` + i18n) affichant fournisseur, espace de travail, consigne, durée maximale, et une mention « Les plafonds de tentatives et d'outils sont conservés. ». | Contenu actuel ne couvre pas explicitement rôles/modèles par tâche ni la liste des prérequis manquants demandés par REQ-SUM-01 — pattern/structure réutilisable, contenu à étendre. |
| QW5 — copier diagnostic | REQ-DIAG-01 | Backend déjà identifié (`attempt_diagnostic.go`, `runtime_health.go`, section « Diagnostic » ci-dessus). | **Aucun bouton « copier » / `navigator.clipboard` trouvé côté web** (recherche vide sur `web/*.js`). UI à créer en réutilisant `buildAttemptDiagnostic`/`fallbackAttemptDiagnostic` comme source de données plutôt qu'en recalculant côté JS. |

## Limites générales de cet inventaire

- REQ-ADM-06 : historique et retour à la config précédente confirmés absents
  pour budgets/quotas/`RunLimits` — gap réel, pas une limite de recherche.
- REQ-ADM-07 : point « suppression de protection » non testé dans cette
  tentative (aucune recherche ciblée effectuée).
- QW1 : existence d'un identifiant de build/version au niveau du binaire
  (hors code source applicatif lu ici) non vérifiée.
- QW2 : recherche de gestion d'un lien direct vers mission supprimée limitée à
  2 fichiers (`web_server.go`, `web/cockpit.js`) — absence non prouvée au-delà.
- Synchronisation `locales/en.json` ↔ `web/i18n-en.js` : mécanisme de
  génération non localisé par cette recherche (limite de recherche inchangée,
  pas preuve d'absence).
- Détail interne de `pricing.go` lu par signatures et corps partiel
  (`quoteRate` lignes 152-160 lues en détail ; reste non relu intégralement).
- Budget d'outils de cette tentative : recherches volontairement bornées aux
  lacunes ADM-05/06/07 et quick wins pour réserver la marge de fin de
  tentative à la rédaction des deux rapports, conformément à la consigne de
  reprise.

## Aucune modification de code applicatif

Confirmé par `git status --short` exécuté en cours de tentative : les seuls
fichiers non suivis pertinents pour cette tâche sont `docs/T1-inventaire-existant.md`
(ce fichier) et `docs/plan-843bb3ce22-T1.md`, tous deux créés/modifiés
uniquement par du contenu documentaire. Les autres fichiers modifiés ou non
suivis listés par `git status` (`INSTALL.md`, `agents_store.go`,
`web/pilot-actions.js`, `docs/QUALIFICATION-CONTEXT-*`, etc.) sont un état
**préexistant** au démarrage de cette tentative (présent dès le premier
`git status` de cette session) — non produits par cette tentative. Aucune
commande d'écriture (`go build`, migration, édition de `.go`/`web/*`) n'a été
exécutée par cette tentative.

## Prochaine action

Transmettre cet inventaire actualisé au planificateur pour :
1. confirmer le découpage T2 (implémentation admin `RunLimits` selon le
   pattern `Store` + `*CLI` + `register*Admin`, avec preview/validation
   réutilisables mais historique/rollback à construire pour REQ-ADM-06) ;
2. décider de l'ordre des 5 quick wins compte tenu des gaps réels identifiés
   (QW1 et QW5 sans aucun mécanisme existant ; QW2/QW3/QW4 partiellement
   couverts, à étendre plutôt qu'à recréer) ;
3. lever les points NOT TESTED listés en « Limites générales » (suppression de
   protection pour REQ-ADM-07 ; identifiant de build pour QW1 ; portée
   complète de la recherche QW2) lors d'une tâche ultérieure ou d'une revue
   indépendante.

## Précisions du superviseur — 29 septembre 2026

La preuve brute des 14 appels de la tentative et les sorties Git avant/après
écriture documentaire sont maintenant reproduites dans
`docs/plan-843bb3ce22-T1.md`, section « Complément de preuve par le superviseur ».
Cet ajout et les précisions ci-dessous sont une revue éditoriale du superviseur,
pas une cinquième production attribuée au worker.

Correction de vocabulaire de stockage : une source canonique par famille ne signifie
pas que toutes les données sont en SQLite. La lecture directe du code confirme que
`pricing.go:saveRate` écrit `.swarm/model-rates.json` et que
`provider_admin.go:changePolicies` écrit `.swarm/providers.json` ; budgets et quotas
utilisent le Store et ses opérations communes. Le CLI et le web passent par les
mêmes services de chaque famille. La conclusion REQ-ADM-03 porte sur l’absence de
stockage parallèle par interface, pas sur un stockage exclusivement SQLite.

Le mécanisme i18n précédemment « non localisé » existe : `package.json` expose
`npm run i18n:build`, qui exécute `node scripts/i18n/build.cjs`. T2+ doit réutiliser
ce générateur ; ne pas éditer un deuxième catalogue indépendant.
