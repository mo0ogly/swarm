# Chemin critique de la migration Go → Rust — spécification de préparation

Statut : **proposé, à arbitrer par l'utilisateur et Codex**. Ce document ne
modifie ni ne remplace `PLAN.md` (propriétaire : Codex) ; il en extrait l'état
au 21 octobre 2026 et propose un découpage des travaux restants. Auteur : Claude,
auto-revue seulement. Les citations renvoient à `PLAN.md` (lignes à la date de
lecture ; elles dériveront — repère stable : le numéro de lot cité).

## Décision applicable — 22 octobre 2026

Méthode adoptée par l'utilisateur : [prompt](PROMPT-METHODE-CODEX.md), intégrée
à [APEX](../../../.claude/skills/apex/SKILL.md). Le statut « proposé » de
l'introduction qualifie l'inventaire historique ; le classement ci-dessous est
adopté. Les comptes par noms ne prouvent pas la couverture des comportements.

| Groupe | Réécriture acceptée par parcours | Contrats en parité stricte |
|---|---|---|
| G1 Cockpit | présentation, navigation, rapports | données exposées, droits, JSON CLI |
| G2 Terminal | interface terminal | identité/processus, commandes et contrôle autorisé |
| G3 Supervision | présentation des diagnostics | lancement, supervision, réservations et refus |
| G4 Planification | éditeur, assistance, parcours | Work persisté, validations, JSON CLI |
| G5 Préparations | parcours de préparation | données produites, consentements, budgets, JSON CLI |
| G6 Fournisseurs | configuration et assistance | secrets, résolution effective, droits, budgets |
| G7 Confiance | présentation et laboratoire | identité, preuves, autorisations et refus |
| G8 Projet/mémoire | consultation et édition | persistance, provenance, partage et droits |
| G9 Coûts/limites | présentation et estimation | calculs contractuels, réservations, limites et refus |
| G10 Automatisation | éditeur et dialogue | déclenchements autorisés, écritures, budgets, JSON CLI |
| G11 Pages | pages et navigation | contrats des opérations appelées |
| G12 Divers CLI | aide et présentation | JSON, effets de recheck et refus |

Un groupe mixte conserve ses frontières de contrat ; une opération de lecture
protégée n'est pas automatiquement de niveau C. Niveau A : sécurité/identité/
budget/données/processus avec revue indépendante ; B : autre logique moteur ;
C : interface/aide avec parcours et inspection visuelle pertinente.
Ordre adopté : G3/G1, G4→G5, G6/G9, G8/G7, puis G2/G10 et complétude G11/G12 ;
les dépendances réelles peuvent imposer un ajustement documenté.
R6/R7 sont préparés en parallèle (migration/restauration, rollback, installation).
Bascule unique des écritures maintenue. Lecture seule anticipée non activée,
confirmation utilisateur encore nécessaire. Aucun droit de livraison ajouté.

## 1. État réel

Synthèse `PLAN.md:43` : « Huit lots R0–R7 : six partiels, R6/recette globale et
R7/bascule non commencés. » Les 445 tests Rust « ne signifient pas que la
majorité des fonctions Go est migrée » (`PLAN.md:44-45`).

| Lot | État | Formulation du plan (`PLAN.md:842-851`) |
|---|---|---|
| R0 | partiel | « Cadrage réalisé, référence finale non figée » |
| R1 | partiel | « identitycontract porté et vérifié ; autres données ouvertes » |
| R2 | partiel | R2.1–R2.5 « vérifiés sur fixtures ; migrations moteur ouvertes » |
| R3 | partiel | R3a–R3ag « par périmètre … supervision et moteur complet ouverts » |
| R4 | partiel | « Primitive Linux R3v qualifiée ; superviseur, autorités/confinement/mémoire complets ouverts » |
| R5 | partiel | « … natifs vérifiés … serveur authentifié et Work CLI complets ouverts » |
| R6 | non commencé | « Non commencé » |
| R7 | non commencé | « Non commencé » |

En cours : **lot 148** (porte reviewer : registre de jobs chargé seulement sous
`cfg(test)`, claim24/guard26 PASS, quality28 FAIL sur fixture parallèle ; suite :
« Candidat gate … vraie paire Unix et enfant propre » — `PLAN.md:15667-15907`).
Clos localement sans revue indépendante : 145 (quotas), 146A, 147.

## 2. Règles opposables

Bascule unique, aucun moteur hybride (`PLAN.md:5-7, 815`) ; natif sans appel Go,
aucun double écrivain, aucune suppression de tests/preuves (`PLAN.md:820-822`) ;
RM-01 référence figée, RM-02 sorties/refus/codes identiques, RM-03 migration de
données avec refus de corruption, RM-04 contrat processus agents, RM-05 droits,
RM-06 parcours navigateur, RM-07 binaire autonome sans Go, RM-08 retour arrière
vérifié, RM-09 aucune preuve locale promue en acceptation globale
(`PLAN.md:828-836`) ; revue de même session non indépendante (`PLAN.md:840`) ;
aucune estimation de durée (`PLAN.md:128-132`).

## 3. Reste à porter, par parcours utilisateur

Routes HTTP natives aujourd'hui : session/capacités, `works` (lecture),
`action` (tâche), budget de conseil Admin, quotas, réglages natifs, `live/*`
(observation, n'équivaut pas à la parité `snapshot|logs|terminal`,
`PLAN.md:118`). Tout le reste répond `web-route-unported`
(`swarm-web/src/service.rs`).

| Groupe | Routes Go à porter | Commandes CLI Go sans équivalent | Dépend de |
|---|---|---|---|
| G1 Lecture cockpit | snapshot, logs, activity, agent-detail, events, exchanges, report, task, visit, runtime-health | console, lifecycle, mission | R3 lecteurs |
| G2 Session terminal | terminal, terminal/control | — | R4 superviseur PTY |
| G3 Supervision & processus | processes, control-diagnostic, recovery-preview, provider-wait | control, process, doctor, autonomy, dispatch | lot 148, R4 |
| G4 Planification | planning, planning-bundle, planning-cleanup, planning-history, graph-drafts/*, graph-performance | plan, preview | R3 planification native |
| G5 Préparations | preparations/* (16), preparation-timeout | prepare | G4 |
| G6 Fournisseurs & connexions | providers/{admin,connections,connections/test,policies,resolve}, assist/* | connections, role-model | R5 config fournisseurs |
| G7 Confiance | trust-{actions,evidence,laboratory,secrets} | trust, evidence, laboratory, verify, validation | R1/R4 autorités |
| G8 Projet & mémoire | project-memory, project-profile/*, skills, retex-evolution | memory, profile, project-profile, skills | R2 stockage |
| G9 Coûts & limites | pricing, pricing/estimate, run-limits/* | run-limits | R3 |
| G10 Automatisation & dialogue | automation/*, dialogue-resume(-preview), web-requests/* | automation, exchange | G1, G3 |
| G11 Pages web | `/`, `/prepare.html`, `/terminal.html`, `/log-console.html`, `/training/casa-pizza/` | aide, version, save | groupes servis |
| G12 Divers CLI | — | recheck-apply/preview | à inventorier |

Inventaire par nom de route/commande de premier niveau seulement ; la parité des
sous-commandes reste à établir (RM-02). La colonne « Dépend de » est une
**hypothèse de l'auteur** déduite des descriptions de lots, pas une citation du
plan : à valider par Codex.

### 3 bis. Mesure du 22 octobre 2026 (Claude, après le lot 201 de Codex en cours)

Méthode reproductible : chaînes `"/api/v1/…"` des sources Go non-test de
`internal/` et `cmd/` (91 chemins) comparées aux bras de `route()` de
`rust/crates/swarm-web/src/service.rs` (25 routes) ; commandes de l'aide de
`./swarm` comparées à celle de `swarm-rust --help`. Comparaison **par nom** :
une fonction portée sous un autre nom (`native-settings`, `native-session`,
`native-capabilities`) n'apparaît pas comme portée ; ce n'est pas une mesure de
parité (RM-02).

État des lots inchangé dans `PLAN.md` : « six partiels, R6/recette globale et
R7/bascule non commencés » ; binaire Rust annoncé « migration partielle ».

**Routes Rust présentes** : `works`, `action`, `planning`, `quotas` (+preview,
apply), `admin-advice-budget` (+preview, apply), `native-settings` (+describe,
preview, apply), `native-session`, `native-capabilities`, `live/*` (overview,
agent, run, fingerprint, events, alerts, alerts/evaluate, annotations,
dataset, scenario).

**Routes Go sans homonyme Rust, par préfixe** (nombre de chemins) :
preparations 14, automation 7, assist 6, run-limits 5, providers 5,
web-requests 4, graph-performance 4, project-profile 3, graph-drafts 3,
trust-actions 2, terminal 2, provider-wait 2, pricing 2, preparation-timeout 2 ;
un chemin chacun : visit, trust-secrets, trust-laboratory, trust-evidence,
task, snapshot, skills, session, runtime-health, retex-evolution, report,
recovery-preview, project-memory, processes, planning-history,
planning-cleanup, planning-bundle, logs, lifecycle, exchanges, events,
dialogue-resume(-preview), control-diagnostic, agent-detail, activity.

**Commandes CLI Go absentes de `swarm-rust`** : version, skills, doctor, aide,
console, project-profile, memory, role-model, run-limits, trust, automation,
connections, prepare, dispatch, autonomy, lifecycle, validation (recheck),
profile, control, exchange, workspace, export/import, plan draft.
**Partielles** : `agent` (manquent start, prepared, resume-launch,
context-preview, preflight, list, logs), `mission` (manquent status, changes,
spending, seen, preview, watch, recovery), `planning` (manquent handoff,
requalify, authorize-recovery, submit/revise-recovered-result), `providers`
(manquent cooldown, relay, init/show au sens Go), `budget` (manquent show,
preview).

**Écart avec le 21 octobre** : seules les routes `live/*` (console, lots
LIVE-UX de Claude) et la planification HTTP native ont été ajoutées ; les
groupes G1–G12 du tableau ci-dessus restent ouverts. Le cockpit web principal
dépend toujours des routes Go.

### 3 ter. Ordre de grandeur demandé par l'utilisateur (22 octobre 2026)

Question utilisateur : « tu arrives à estimer un % de travail fait et du reste
à faire ? ». Réponse de Claude, **à faible confiance, non pondérée par Codex** :
environ **30–40 % fait, 60–70 % restant** jusqu'à une bascule complète. Ce n'est
pas une acceptation, ni une durée (aucune estimation de durée, règle du plan).

| Indicateur | Mesure (22 octobre) | Lecture |
|---|---|---|
| Lots R0–R7 | 0/8 terminés ; 6 partiels, R6 et R7 non commencés | aucun lot clos |
| Routes API web | 25/91 par nom (≈ 30 % avec les équivalents `native-*`) | surface web surtout à faire |
| Commandes CLI premier niveau | 17/40 (43 %) ; communes : agent, budget, checkpoint, evaluate, gate, identity, init, mission, ooda, planning, pricing, providers, quotas, resume, task, task-model, work | ≈ 30–35 % en tenant compte des sous-commandes manquantes (agent, mission, planning, providers, budget) |
| Volume de code | Rust 96 459 lignes src (+40 101 tests) ; Go 83 561 lignes non-test (+90 915 tests) dans `internal/` et `cmd/` | inutilisable comme avancement : Rust plus verbeux et porteur de fonctions absentes du Go |

Pourquoi la fourchette : le cœur moteur (supervision, lancement, portes,
relecteur, planification native, budgets, identité Root) est plus avancé que la
surface, car il concentre la majorité des lots ; restent à l'inverse les
préparations (plus gros bloc API), automatisation, confiance, mémoire/profil
projet, terminal, cockpit web principal, puis R6 (données + recette globale) et
R7 (bascule unique, retour arrière vérifié, installation). La parité RM-02 n'est
pas mesurée : un même nom ne prouve pas les mêmes sorties et refus.

**Demande à Codex** : pondérer les groupes G1–G12 (et R6/R7) selon son
appréciation de l'effort restant, et indiquer les sous-commandes déjà en parité
vérifiée, pour remplacer cette fourchette par un indicateur sourcé ; ou refuser
explicitement tout pourcentage si la pondération n'est pas défendable.

### 3 quater. Taux d'avancement et estimation de durée (23 octobre 2026)

**Taux factuels**, depuis la matrice de Codex
(`evidence/20261022-parallel-196/matrix-integrated.json`, 83 opérations CLI,
93 routes HTTP), complétés des livraisons ultérieures (`mission watch`,
`status`, `status-native`) :

| Niveau | CLI | Web | Données | Bascule |
|---|---|---|---|---|
| Existe en Rust | ≈ 33/83 (≈ 40 %), plus 10 partielles ou renommées | 4/93 au même chemin ; ≈ 25 avec équivalents renommés et Live (≈ 25 %) | schéma 37 ; migrations 1..36 non portées | 0 |
| Vérifié localement | watch/status, lecture Work (banc), quotas, budget, réglages… | Live | — | — |
| Parité acceptée | 0 | 0 | 0 | 0 |

**Estimation de durée demandée par l'utilisateur** (Claude, faible confiance,
pas un engagement de Codex) : **3 à 8 semaines de travail continu des agents,
médiane ≈ 5**. Base : ≈ 3 tranches verticales par jour observées sur une seule
journée de la nouvelle méthode (W1–W3 + status, groupe G3/G1, niveau A) ;
≈ 40 tranches estimées sur G1–G12 plus R6/R7 ; facteur 1,5–2 pour blocages,
recettes en échec, décisions attendues et ressources. Le haut de fourchette
couvre un cockpit web (≈ 25 pages) aussi coûteux que les groupes moteur.

Pour la resserrer : découpage par Codex de chaque groupe en tranches estimées,
et vitesse mesurée sur 3 à 5 lots.

**Mesure outillée (23 octobre, `tools/migration-progress/avancement.py`)**, plus
fine que le tableau ci-dessus (161 opérations CLI Go au niveau sous-commande
au lieu de 83) : CLI 36/161 existent (22 %), 4 vérifiées, 9 partielles ; routes
API 4/93 au même chemin ; pages 1/10 ; parité acceptée 0 partout. 51 opérations
et 21 routes Rust sans homonyme Go ne sont pas comptées tant que la matrice ne
les relie pas : le taux CLI est donc un minorant. Détail : [AVANCEMENT.md](AVANCEMENT.md).

## 4. Séquence proposée

1. **Terminer le lot 148** (porte reviewer → lancement → publication → acceptation)
   — premier sur le chemin critique selon `PLAN.md:15654-15656, 15895`.
2. **G3 supervision** puis **G1 lecture cockpit** : sans eux, aucune conduite
   observable dans le binaire normal.
3. **G4 → G5** planification puis préparations (parcours de démarrage de mission).
4. **G6, G9** (fournisseurs, coûts), **G8** (projet/mémoire), **G7** (confiance).
5. **G2** terminal interactif (dépend du superviseur PTY).
6. **G10, G11, G12** pour la complétude.
7. **R6** recette globale + migration de données (Work JSON/BLOB, restauration
   vérifiée, refus de corruption).
8. **R7** bascule unique : binaire autonome (Build.sh/Makefile sans Go), retour
   arrière vérifié (RM-08), installation, smoke, RETEX.

Chaque groupe sort quand : routes et commandes du groupe répondent avec les mêmes
sorties/refus/codes que Go (RM-02, oracle Go figé), parcours navigateur FR/EN
vérifié (RM-06), recette permanente PASS sur instantané figé, revue indépendante.

## 5. Registre des bloqueurs

| Bloqueur | Source | Propriétaire proposé | Condition de reprise |
|---|---|---|---|
| Originaux de preuves antérieurs au lot 129 absents | `PLAN.md:39-40` | Codex | restauration versionnée ou requalification explicite |
| `check.py` encore sur Go | `PLAN.md:14503-14508` | Codex | équivalent natif avant retrait des oracles Go |
| Hook Go dans le vérificateur de contextes | `PLAN.md:21` | Codex | retrait après équivalent natif |
| Build.sh/Makefile appellent Go | `PLAN.md:15499-15502` | Codex | chaîne de build Rust seule (prérequis R7) |
| Sink synchrone bloqué, publications Root imprécises | `PLAN.md:98-99` | Codex | conception du sink natif |
| Images et fenêtre client positive non portées | lot 146A | Codex | port natif |
| Normalisation Work JSON/BLOB | R6 | Codex | spec de migration de données |
| Sources modifiées en parallèle invalidant les instantanés | green03, quality09 | Claude + Codex | instantané figé des entrées déclarées (méthode éprouvée LIVE-UX-02) |
| Fixture métriques laissant des orphelins / Clippy | lot 148 quality28 | Claude | **corrigé côté Claude le 21 octobre** (groupe de processus tué, Clippy propre) ; relance quality par Codex |

## 6. Décisions ouvertes

- Ordre G1/G3 vs G4/G5 si l'utilisateur privilégie le démarrage de missions.
- Place des lots UX hors chemin critique (LIVE-UX-03 télémétrie, feuille de route
  LangWatch) : en parallèle sans toucher le schéma mission, ou après R7.
- Répartition Codex/Claude par groupe (aujourd'hui : Codex sur le chemin critique,
  Claude sur les lots UX).
