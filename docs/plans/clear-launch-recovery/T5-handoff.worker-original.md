# T5 — Recette globale et RETEX (4 parcours, web + CLI)

Mission `w-115c11e8f802a4f98c3def32` ; tâche `plan-115c11e8f8-T5` ; agent
`auto-2a6914c3316e81db08fe` ; tentative `a-1637354a070e743f69f201a7` ; départ 1/2 ;
révision du travail 295. Ce rapport est rédigé par le worker T5, en lecture seule :
aucun code n'a été modifié, aucune commande de mutation n'a été exécutée.

## SHA et revue indépendante

- Base Git : `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`. Confirmé par `git rev-parse HEAD`
  exécuté dans cette tentative, identique à la base déclarée par T1 à T4 et par
  `T5-candidate.json`.
- Arbre de travail **dirty** (`git status --short` : mêmes fichiers modifiés/non
  suivis que listés en contexte de départ). `git diff --check` : aucune sortie,
  code 0 — pas de marqueur de conflit ni d'espace parasite en fin de ligne.
- Candidat exact : `candidate_id = a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77`,
  48 fichiers empreintés (`docs/plans/clear-launch-recovery/T5-candidate.json`),
  confirmé identique à celui utilisé par `T5-global-checks.md`
  (« Candidat : a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77 »).
- **Revue indépendante** : les quatre tâches dépendantes ont chacune reçu un avis
  favorable et une acceptation moteur distincts (T1 revue 23 → r165 ; T2 revue 24 →
  r170 ; T3 revue 25 → r175 ; T4 revue 40 → r292, gate r293, acceptation r294 —
  sources : `RETEX-supervision.md`). **Ce rapport T5 lui-même n'a pas encore été
  soumis à une revue indépendante en contexte séparé.** Conformément au contrat de
  la tâche (« validation : revue indépendante en contexte séparé sur ce SHA avant
  toute déclaration de livraison »), ce document ne constitue pas une déclaration
  de livraison ; il est le livrable à soumettre à cette revue.

## Contrôles moteur courants (attribués, non rejoués par ce worker)

Source : `docs/plans/clear-launch-recovery/T5-global-checks.md`, en-tête
« CURRENT CANDIDATE FULL SUITE: PASS », exécutés par le superviseur Codex sur le
candidat `a0b89b5c...4abcceddbc7e77` (48 fichiers, même base) :

| Contrôle | Résultat |
| --- | --- |
| `go test ./... -timeout 40m` | PASS, 473,034 s, sortie 0 |
| Tests ciblés reprise archivée/citations/processus indépendant | PASS, 0,456 s |
| `go vet ./...` | PASS |
| Race ciblé réservation SQLite / sortie de contrôles | PASS, 12,799 s |
| `npm test` | PASS |
| `python3 tools/agent-workflows/check.py` | PASS (9 méthodes) |
| `git diff --check` | PASS |

Ce worker n'a pas relancé cette suite (consigne explicite : lire le résultat
courant, ne pas la rejouer). Il a seulement revérifié indépendamment
`git rev-parse HEAD`, `git status --short` et `git diff --check`, avec les
résultats ci-dessus, concordants avec ce qui est déclaré dans T5-global-checks.md.

## Parcours REQ-QW1 — Résumé avant lancement et modèles

Source : `T1-handoff.md`. Recette FR/EN × clair(« etat »)/sombre réellement rejouée
par le superviseur (CUA) sur racine isolée `/tmp/swarm-qw1-visual-joy403z4` et sur
le cas de divergence de modèle `w-eae97600d116e687fb000f42` (racine
`/tmp/swarm-reported-model-ui-89ndh7l7`), aucune mission réelle démarrée.

- CLI public (`bin/swarm ... mission launch-preview`-équivalent) : objectif tiré
  de `Work.Objective`, rôle, fournisseur `claude`, niveau demandé `Automatic`,
  **modèle résolu par la configuration : `sonnet`**, **modèle rapporté par le
  fournisseur : `Unknown before execution`** (volontairement inconnu avant
  exécution, jamais inféré), profil projet et skills absents affichés
  explicitement, périmètre/budget/reprises/validations listés.
- Quatre combinaisons langue/thème rejouées (EN/etat, EN/sombre, FR/sombre,
  FR/etat) : résumé affiché PASS, Échap ferme la modale PASS, focus rendu et
  visible sur `mission-primary` PASS dans les quatre cas (tableau et relevés DOM
  `launch-v4-browser-replay.json`).
- Cas de divergence modèle demandé/rapporté : processus fournisseur de protocole
  isolé (pas de LLM réel) déclarant `system/init.model=fixture-reported-model`
  pendant que la politique résout `sonnet` ; `agent show` CLI confirme les deux
  champs distincts persistés (preuve CLI citée dans T1-handoff.md). Recette
  navigateur FR/EN × clair/sombre sur ce cas : 4 modes passent, focus clavier
  corrigé dans `web/graph.css` après qu'une bordure d'alerte masquait le repère
  de focus (complément superviseur du 3 octobre).
- Limite explicitement conservée : aucune exécution réelle avec un fournisseur
  Claude déclarant un modèle physiquement différent de celui demandé n'a été
  provoquée ; le cas testé reste un fournisseur de protocole isolé.

## Parcours REQ-QW2 — Blocage expliqué (cause/acteur/action)

Source : `T2-supervisor-recipe.md`. Deux cas de blocage réels, recette CLI et
navigateur par le superviseur, sans doublure remplaçant le comportement vérifié.

- **Cas A — organisation absente** (racine `/tmp/swarm-t2-qw2-v2-1046661`, travail
  `w-9812b5ebba819a4e3828d127`) : cause « Organisation autonome non configurée »,
  acteur **Vous/You**, action directe **Préparer l'organisation / Prepare
  organization** ouvrant une modale explicative (pas de création de mission, pas
  d'appel IA). CLI public confirmé : `bin/swarm --root ... --json mission status`,
  code 0, JSON cité avec `organization.ready=false`, `guidance.actor="Vous"`.
  FR/EN × clair/sombre rendus et inspectés ; Entrée ouvre, Échap ferme, focus
  revient au déclencheur (`:focus-visible=true`).
- **Cas B — plafond de 2 tentatives réellement atteint** (mission active
  `w-115c11e8f802a4f98c3def32`, T2, révision 93) : cause **« 1 tâche(s) ont épuisé
  leurs tentatives autorisées »**, acteur **Responsable de la mission**, action
  **Examiner les tentatives et les refus** (ouvre le dossier de reprise, ne lance
  rien). Détail de carte : « Lancer un agent : Plafond de tentatives du plan
  atteint » et « Relancer une tentative : Plafond de tentatives du plan atteint ».
  Aucun contournement de limite appliqué ou proposé (pas de nouveau départ, pas
  de hausse de plafond, pas de remboursement de tentative).
- Relecture indépendante directe du produit : un contrôle moteur ré-exécute
  personnellement `bin/swarm --json planning show` puis `mission status` sur les
  deux racines (cas A et cas B) au lieu de ne lire que les JSON déjà écrits,
  confirmant deux tentatives utilisées sur deux autorisées pour le cas B.
- Correctif associé attribué au superviseur : `result_presentation.go`,
  `currentReviewPresentation` cesse d'ignorer un avis défavorable courant qui
  produisait un faux « rapport détecté, à soumettre ». Tests ciblés cités PASS,
  sortie 0.

## Parcours REQ-QW3 — Bilan par tentative (ledger distinct de la validation)

Source : `T3-supervisor-recipe.md`. Principe vérifié : **l'état du processus
d'une tentative est distinct de la validation enregistrée de la tâche**, qui
couvre toutes les tentatives et ne garantit pas la fraîcheur des preuves —
observé concrètement sur le cas T2 cité : « La tâche porte une acceptation
enregistrée mais sa preuve est devenue périmée par le complément T3 : ne pas
confondre ces deux faits. »

- `metrics_version=1` : Read/Glob/Grep = lectures ; Write/Edit/MultiEdit/
  file_change = écritures ; autres outils = non classés ; répétitions = même
  signature nom+entrée sous un nouvel identifiant ; retransmission d'un même
  identifiant dédoublonnée ; visibilité perdue → mesure partielle, pas un zéro.
- Deux tentatives T2 relevées concrètement via `bin/swarm --json mission
  spending` : tentative interrompue `a-1838917a21fb55cd54481c1c` — 25 outils
  observés, catégories historiques inconnues, coût/usage absents ; tentative
  terminée `a-4fea4cb374a0cf60b5cc2680` — 12 outils observés, 20/16528 jetons
  rapportés, coût fournisseur environ 0,65 USD, catégories historiques inconnues.
- Recette navigateur réelle (superviseur) : bouton « Où vont les appels et les
  coûts ? » dans la Conduite, ouverture par Entrée, Échap ferme et restaure le
  focus au bouton, FR/EN × clair/sombre, les quatre variantes rendues et
  examinées, aucune erreur console relevée.
- Inconnus explicitement conservés, jamais mis à zéro : tests non déduits des
  commandes mixtes (nombre inconnu) ; appels internes du fournisseur non
  mesurés ; coûts/jetons seulement si rapportés par le fournisseur.

## Parcours REQ-QW4 — Reprise ciblée (refus → correction → reprise)

Source : `T4-handoff.md` et `RETEX-supervision.md` (sections « Dossier actuel,
sans mélanger bilan initial et recette externe » et suivantes).

- Séquence réellement documentée et datée : revue r277 non favorable → retour
  explicite du superviseur à **Bloquée** via CLI public à r281 après expiration
  d'une recette précédente (action humaine/superviseur, **pas un nouveau verdict
  IA**) → correction **Next r282** → reprise par bouton web **r283** → fiche
  passée à **À vérifier**.
- Défi fraîcheur `84a714566ebedd5184fcffcbd2d2ac40`, délai alloué 300 s, recette
  CUA achevée en **104,852 s / 105 s** selon les sources (T4-handoff.md et
  RETEX-supervision.md concordent) ; trois observations postérieures au défi
  (fiche refus / correction / reprise), captures
  `t4-inspector-84a714566ebedd5184fcffcbd2d2ac40-*.png`, reçu complet
  `T4-full-fresh-recipe.json`/`.log`, code de processus 0, confronté aux
  événements durables CLI.
- Attribution stricte conservée : la recette web FR/EN × clair/sombre et le
  pilotage CUA sont réalisés par le superviseur externe Codex/CUA, **jamais** par
  le producteur T4 ni par le vérificateur sans outils ; ce pilote externe « ne
  prouve pas une autonomie totale » (citation T4-handoff.md).
- Contrôle moteur en lecture seule : il vérifie le reçu lié aux captures et
  événements sans prétendre muter la tâche pendant les contrôles à révision
  figée.
- Ancien PARTIAL du producteur (bilan initial à 08:58, 12 outils : 3 lectures,
  2 écritures, 7 non classés) distingué explicitement des preuves ultérieures
  attribuées au superviseur (recette complémentaire, correctifs SQLite
  `BUSY`/`BUSY_SNAPSHOT`, correctif de citation JSON de la revue r288).
- Clôture effective : 40ᵉ appel de revue favorable à r292, gate r293,
  acceptation normale r294 (6/6 contrôles, 100/100, preuves fraîches).

## RETEX quantifié

Source : `bin/swarm --json mission spending w-115c11e8f802a4f98c3def32`, sortie
réellement exécutée par ce worker, **tronquée à 8000 octets** (option `head -c
8000` appliquée pour rester dans le budget d'outils de cette tentative) : les
lignes `attempts` postérieures à la 3ᵉ tentative de T1 n'ont pas été relues
individuellement par ce worker ; le tableau agrégé (`rows`) ci-dessous, lui,
est intégralement reçu.

| Producteur | Appels enregistrés (recorded_calls) | Outils observés (observed_tool_calls) | Jetons sortie rapportés | Coût rapporté (USD) |
| --- | --- | --- | --- | --- |
| Responsable (root) | 5 | 0 (non observable pour ce rôle) | 18 077 | 0,451902 |
| Vérificateur indépendant | 40 | 0 (non observable pour ce rôle) | 501 680 | 10,9605012 |
| T0 — inspection | 2 | 27 | 5 404 | 0,2851286 |
| T1 — résumé/modèles | 3 | 51 | 30 589 | 1,0946986 |
| T2 — blocage | 2 | 37 | 16 528 | 0,6520226 |
| T3 — bilan par tentative | 2 | 37 | 13 333 | 0,4424448 |
| T4 — reprise | 2 | 37 | 12 131 | 0,4412018 |
| T5 — ce rapport (en cours) | 1 (ce départ) | 3 (reflète un état antérieur à cette lecture) | 0 rapporté à cet instant | 0 rapporté à cet instant |

Totaux globaux déclarés par le moteur : `recorded_control_executions = 125`,
`worker_retries = 5`. Ces deux compteurs sont des mesures moteur distinctes des
appels IA et des appels d'outils ci-dessus (contrat de mesure T3, non
assimilable).

**Interventions humaines/superviseur identifiées dans les sources** (non
quantifiables en nombre exact, le superviseur Codex n'étant pas mesuré comme
un producteur IA dans `mission spending`) :
- Correctifs applicatifs directs par le superviseur Codex sur T1 (après deux
  échecs producteur : diagnostic sans correctif, puis 25 outils sans
  modification), T2 (`result_presentation.go`), T3 (focus clavier
  `web/graph.css`, complément après interruption à 25 outils), T4 (correctifs
  SQLite `BUSY`/`BUSY_SNAPSHOT`, adaptateur de recette fraîche, correctif de
  citation JSON de revue).
- Recettes navigateur FR/EN × clair/sombre × clavier pour les 4 parcours :
  toutes réalisées par le superviseur externe Codex/CUA, aucune par les workers
  producteurs ni par le vérificateur sans outils.
- Une décision de clôture explicite de l'utilisateur (« on finit ce projet »)
  autorisant un relèvement borné du plafond de revues (de 40 à 42, deux revues
  supplémentaires) sans remise à zéro des tentatives ni des plafonds d'outils.

**Limites et inconnus explicitement conservés, jamais remplacés par une valeur
inventée** :
- Coût total réel, requêtes LLM internes des fournisseurs et nombre exact
  d'interventions humaines/superviseur : **inconnus, pas zéro** (formulation
  reprise telle quelle de T5-global-checks.md et RETEX-supervision.md).
- `calls_without_usage` non nul pour le responsable (2/5), le vérificateur
  (2/40) et T0 (1/2) et T1 (1/3 dans la fraction lue) : certains appels n'ont
  pas de mesure de jetons/coût associée ; absence de mesure ≠ coût nul.
- La sortie `mission spending` étant tronquée par ce worker à 8000 octets, les
  lignes `attempts` détaillées de T2, T3, T4 et T5 n'ont pas été relues
  individuellement ici ; le détail par tentative de T2/T3 cité plus haut provient
  de `T3-supervisor-recipe.md`, pas d'une relecture directe par ce worker.
- Erreurs et expirations de recette conservées dans l'historique (non
  effacées par ce rapport) : timeout de revue à 90 s (71 événements
  `thinking_tokens`, aucun résultat final), essais de capture/défi expirés à
  120 s (comptés comme échec, pas comme PASS), `SQLITE_BUSY` sur applications de
  politique concurrentes, interruption de serveur par signal 143, erreur de
  citation JSON de revue (r288), timeouts de suite Go globale sur tests
  d'intégration préexistants avant relance avec délai étendu.
- Réservation SQLite avant mutation : le contrat de mesure et les correctifs
  T4 (élévation lecture→écriture, réservation commune à toutes les mutations)
  montrent une réservation appliquée avant la première lecture d'une
  transaction de mutation, mais ceci ne prouve pas l'élimination de toute
  contention SQLite possible (limite explicitement conservée dans
  RETEX-supervision.md).
- Sortie facultative des contrôles : partage désactivé par défaut, plafonné à
  8 Kio avec indication de taille/troncature ; testé dans les quatre
  combinaisons langue/thème et au clavier (T5-global-checks.md).
- Cadrage de revue sans outils : le mode `evidence_assessment_without_tools` est
  conservé pour distinguer « examiner des preuves et leur couverture » de « les
  rejouer personnellement » (clarification de la revue 34, RETEX-supervision.md).

## Conclusion et prochaine action

Les quatre parcours REQ-QW1 à REQ-QW4 ont chacun une recette FR/EN × clair/sombre
× focus clavier documentée avec observations concrètes, et chacune des tâches
T1 à T4 a reçu une acceptation moteur normale avec revue indépendante favorable
sur sa propre tentative. Aucun des quatre parcours n'est en échec au moment de
cette lecture ; la clause d'arrêt (OODA + remontée planificateur en cas d'échec)
ne s'applique pas ici.

Ce rapport **n'est pas une déclaration de livraison**. Conformément au contrat
de la tâche T5, une revue indépendante de ce document précis, en contexte
séparé, sur le SHA candidat `a0b89b5c7f8ba687ddb729ed6fff82b051456004f202b59b9891cbceddbc7e77`
(base `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, dirty, 48 empreintes), reste
nécessaire avant toute proposition de livraison. Seul le moteur décide de
l'acceptation ; ce worker ne modifie ni la base Swarm ni le statut des tâches.
