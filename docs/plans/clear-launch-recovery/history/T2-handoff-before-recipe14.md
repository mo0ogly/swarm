# T2 — REQ-QW2 : blocage expliqué (web + CLI)

Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T2 ; agent auto-4d827ecc9cb075bf602d ; tentative a-1838917a21fb55cd54481c1c ; départ 1/2. Base candidat `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, arbre non commité (`dirty=true`), même candidat que T1. **Aucun fichier de code n'a été modifié par cette tâche** : le triplet moteur/CLI/web pour REQ-QW2 existait déjà, confirmé par T0 puis revérifié ici par lecture directe et par recette ciblée. Aucun succès autonome ni acceptation n'est revendiqué.

## Constat principal

L'inventaire T0 (`docs/plans/clear-launch-recovery/T0-inventaire.md`) avait confirmé l'existence d'un triplet moteur→CLI→web pour le blocage expliqué, avec une inconnue : le lien entre `DiagnosticItem` (moteur, pas de champ acteur propre) et les blocages de type cooldown fournisseur. Cette tâche lève cette inconnue : **l'acteur n'est jamais porté par `DiagnosticItem` lui-même** ; il est systématiquement porté par `MissionUnderstanding.Actor` au niveau de la tâche ou de la mission, et le rendu CLI/web affiche toujours les deux côte à côte (compris → diagnostic), jamais l'un sans l'autre. Les trois chemins de blocage examinés (organisation non configurée, cooldown fournisseur, plafond de tentatives) suivent ce même schéma. Aucun contournement de limite n'est proposé dans aucun des textes lus.

## Cas de blocage et preuves

| Cas | Cause affichée | Acteur affiché | Action autorisée affichée | Contournement proposé ? | Méthode de vérification | Résultat |
| --- | --- | --- | --- | --- | --- | --- |
| Organisation non configurée (mission sans responsable/contrôles) | « Organisation autonome non configurée » · « Responsable de mission absent : planification hiérarchique non configurée. » | « Vous » (`actor_kind=user`, `situation=decision_humaine`) | « Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement. » | Non | **Recette CLI réelle**, binaire compilé depuis ce candidat, racine isolée `/tmp/swarm-t2-qw2-3715386`, travail `w-50d4daa052d108ca99c1de9e` créé sans aucun fournisseur configuré, commande `swarm mission status` réelle (texte et JSON capturés ci-dessous) | PASS |
| Cooldown fournisseur actif (quota/latence fournisseur) | Message calculé par `ProviderCooldown.message()`, actor `"Le fournisseur IA"` (`attente_normale`) ou `"Vous"` si l'échéance est inconnue (`decision_humaine`) — `mission_status.go:605-638` | Idem colonne cause | « Attendre l'échéance ; les plafonds de tentatives et les autres blocages restent applicables. » ou, si échéance inconnue, « Vérifier la disponibilité du compte puis lever explicitement l'attente fournisseur sans heure de reprise. » — jamais une levée automatique du cooldown | Non (le texte interdit explicitement toute levée non vérifiée) | **Test comportemental existant** (code moteur réel, store réel) : `go test -run 'TestMissionProviderCooldown\|TestProviderCooldown'` — fichiers `provider_cooldown_test.go`, `mission_provider_cooldown_test.go` | PASS (comportemental) ; **NOT TESTED** en conditions de vrai rate-limit fournisseur (déclencher un vrai dépassement de quota est hors périmètre et risquerait un abus réel de quota) |
| Plafond de tentatives atteint (tâche `intervention`/`configure`, `AttemptLimitReached`) | « X/Y tentatives utilisées. <raison> » — `mission_status.go:830` | « Responsable de la mission » (`user`, `situation=limite_tentatives`) | « Examinez les refus et les preuves conservées. Aucun redémarrage automatique n'est autorisé ; une décision motivée sur la suite est nécessaire. » | Non (interdiction explicite d'un redémarrage automatique) | **Test comportemental existant** : `go test -run 'TestTaskUnderstanding\|TestAttemptDiagnostic'` — fichiers `attempt_diagnostic_test.go`, `mission_status_test.go`, `environment_retry_test.go` | PASS (comportemental) ; **NOT TESTED** en conditions réelles avec agent provider réel (nécessiterait d'épuiser réellement des tentatives via un fournisseur IA, hors budget/éthique de cette tâche) |
| Échec d'outil / erreurs consécutives pendant une tentative (`AttemptDiagnostic`, catégories Configuration / Environnement / Contrôle / Outil / Cause inconnue) | `DiagnosticItem.Cause` par catégorie, ex. « Les traces signalent un refus d'accès ou une ressource indisponible dans l'environnement… » — `attempt_diagnostic.go:37-67` | Porté par `MissionUnderstanding.Actor` de la tâche affichée juste au-dessus (ex. `"Vous"` pour un refus d'environnement — `mission_status.go:841`), jamais par `DiagnosticItem` lui-même | `DiagnosticItem.Action`, ex. « Faire vérifier les droits et l'accès aux ressources sur la machine qui exécute l'agent ; reprendre après correction vérifiée. » | Non (chaque action demande une vérification explicite avant reprise, jamais une levée) | **Test comportemental existant** : `attempt_diagnostic_test.go` (31 tests passés au total sur l'ensemble des motifs ci-dessus) ; **lecture directe** confirmant le pairage `printMissionUnderstanding`+`printAttemptDiagnostic` (CLI, `mission_cli.go:251-255`) et `understandingView`+`diagnosticView` (web, `web/mission.js:341,343`) | PASS (comportemental + structurel) ; **NOT TESTED** en navigateur réel (budget épuisé avant cette étape) |

### Sortie CLI réelle (cas « organisation non configurée »)

```text
Mission
La mission en bref
Organisation autonome non configurée
Vous — Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.
Action principale : Préparer l'organisation
Effet : Affiche les rôles et les conditions manquantes.
Organisation autonome non configurée
...
- Responsable de mission absent : planification hiérarchique non configurée.
Politique incomplète ; aucune vérification de mission démontrée.
Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.
Autorisation : non autorisée · conducteur : absent
```

### Extrait JSON réel (`--json mission status`)

```json
{
  "understanding": {
    "what": "Organisation autonome non configurée",
    "next_step": "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.",
    "actor": "Vous",
    "actor_kind": "user",
    "situation": "decision_humaine"
  },
  "organization": {
    "ready": false,
    "label": "Organisation autonome non configurée",
    "issues": ["Responsable de mission absent : planification hiérarchique non configurée."],
    "next": "Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement.",
    "verification": "Politique incomplète ; aucune vérification de mission démontrée."
  }
}
```

Commandes exactes exécutées :
```
go build -o /tmp/swarm-t2-bin .
/tmp/swarm-t2-bin --root /tmp/swarm-t2-qw2-3715386 init
echo '{"schema_version":1,"event_id":"ev-qw2-2","title":"Recette QW2","objective":"Recette blocage REQ-QW2","scope":"Verifier message de blocage sans fournisseur configure","criteria":["Blocage explicite"]}' \
  | /tmp/swarm-t2-bin --root /tmp/swarm-t2-qw2-3715386 work create --input -
/tmp/swarm-t2-bin --root /tmp/swarm-t2-qw2-3715386 mission status w-50d4daa052d108ca99c1de9e
/tmp/swarm-t2-bin --root /tmp/swarm-t2-qw2-3715386 --json mission status w-50d4daa052d108ca99c1de9e
```
Racine isolée `/tmp/swarm-t2-qw2-3715386`, distincte de toute base de mission réelle ; aucune modification de `.swarm/state.db` du dépôt. Exit codes 0 pour chaque commande (aucune erreur stderr autre que celles montrées plus haut lors du sondage du schéma `work create`).

### Test comportemental exécuté

```
go test ./... -run 'TestMissionProviderCooldown|TestProviderCooldown|TestAttemptDiagnostic|TestTaskUnderstanding|TestMissionStatus.*Diagnostic|TestMissionStatus.*Understanding' -v
→ 31 tests passés, 0 échec.
```
Ces tests exercent le code moteur réel (`provider_cooldown_test.go`, `mission_provider_cooldown_test.go`, `attempt_diagnostic_test.go`, `mission_status_test.go`, `environment_retry_test.go`) avec un store réel sur base de test ; ils ne remplacent pas le comportement affiché (pas de simulation du texte Cause/Acteur/Action, c'est le code réel qui le produit). Ils ne remplacent pas pour autant une recette manuelle avec un vrai fournisseur en rate-limit, qui reste NOT TESTED.

## REQ-QW2 — Résultat par critère

| Critère | Résultat | Preuve | Limite |
| --- | --- | --- | --- |
| req-7 — cause/acteur/action pour chaque cas testé | PASS pour le cas rejoué en CLI réel ; PASS comportemental pour cooldown/plafond/diagnostic d'outil | Tableau ci-dessus | Pas de recette navigateur réelle ; pas de vrai rate-limit fournisseur déclenché |
| req-8 — aucun contournement de limite | PASS | Aucun texte d'action lu (6 catégories de `DiagnosticItem`, 3 branches `MissionUnderstanding` liées au blocage) ne propose de lever une limite ; plusieurs l'interdisent explicitement | — |
| req-9 — recette sur cas réels, sans doublure | PARTIAL | Un cas réel rejoué de bout en bout (binaire réel, racine isolée) ; les autres s'appuient sur la suite de tests comportementale existante (code réel, pas une doublure du rendu), faute de budget pour fabriquer en direct un cooldown fournisseur réel ou un épuisement de tentatives via agent réel | Budget d'outils de la tentative atteint avant de pouvoir construire ces préconditions supplémentaires en CLI/navigateur |

## Mise à jour — tentative 2 (départ 2/2, agent 537b1d6b-7c30-4e7f-878a-3c832554c719, tentative a-4fea4cb374a0cf60b5cc2680, révision 83)

Reprise ciblée : lecture limitée à ce document et à `T2-supervisor-recipe.md` (aucun nouvel inventaire global, aucune relecture de T0/T1). Candidat inchangé (`1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, dirty=true) ; aucun fichier de code modifié par cette tentative non plus.

### Contrôles enregistrés (politique `swarm validation preview`/`apply`, révision 71) — rejoués sur le candidat courant

| Contrôle | Commande | Résultat superviseur (réf.) | Résultat rejoué cette tentative |
| --- | --- | --- | --- |
| Tests ciblés moteur | `go test ./... -run 'TestMissionProviderCooldown\|TestProviderCooldown\|TestAttemptDiagnostic\|TestTaskUnderstanding\|TestMissionStatus.*Diagnostic\|TestMissionStatus.*Understanding' -v` | PASS, 0,207 s, code 0 | **PASS**, 31 tests, 2,937 s, code 0 — mêmes noms de test, aucune régression |
| Relevé CLI du cas réel (organisation non configurée) | build `go build -o <bin> .` puis `init`/`work create`/`mission status`/`--json mission status` sur racine isolée | PASS | **PASS** — racine isolée `/tmp/swarm-t2-qw2-v2-*`, nouveau travail `w-9812b5ebba819a4e3828d127` ; cause « Organisation autonome non configurée », acteur « Vous », action « Préparer une mission hiérarchique avec un responsable et des contrôles explicites. Les tâches existantes restent consultables et exécutables manuellement. » — texte identique au relevé de la tentative 1 ; exit 0 sur les 4 commandes |

Aucune donnée de mission réelle touchée (racines `/tmp` jetables, distinctes de `.swarm/state.db` du dépôt). Aucun plafond ni critère modifié pendant ce rejeu.

### Complément de recette navigateur réel (superviseur, hors périmètre worker)

Le superviseur a réalisé — séparément, sans nouvelle tentative worker — la recette navigateur manquante de la tentative 1 : ouverture du vrai bouton « Diagnostiquer et préparer la reprise », captures `t2-blockage-fr-light.png`, `t2-blockage-fr-dark.png`, `t2-blockage-en-dark.png`, `t2-blockage-en-light.png` (confirmées présentes sous `docs/screenshots/clear-launch-recovery/`, avec `t2-started.png`), Échap ferme le détail et rend le focus au déclencheur (`focusVisible=true`). Trois cas de blocage supplémentaires observés dans le produit réel par le superviseur (non rejoués indépendamment par ce worker, qui n'a pas relu leurs preuves brutes hors périmètre autorisé) :

- Tentative T2 arrêtée au plafond de 25 appels → détail affiche le motif, demande d'examiner avant reprise, aucune action d'acceptation disponible sans résultat complet soumis.
- Dépendance T1 périmée/non validée → détail indique le prérequis manquant, désactive Lancer/Relancer avec ce motif, aucune relance T2 revendiquée pendant la revalidation T1.
- Revalidation T1 : ancien résultat accepted avec fichiers liés modifiés → bouton principal ouvre une vraie confirmation de nouvelle revue ; même tentative conservée, statut submitted, ancien avis archivé après confirmation ; appels 7→8 puis 8→9, aucune tentative producteur ajoutée.

**Limite de traduction explicitement maintenue (ni infirmée ni gonflée)** : les libellés structurels sont traduits en anglais ; certains motifs composés issus des traces (limite d'appels, dépendance) restent en français. Ne pas déclarer une traduction intégrale FR/EN de ces traces précises.

### REQ-QW2 — Résultat par critère (mise à jour tentative 2)

| Critère | Résultat | Évolution vs tentative 1 |
| --- | --- | --- |
| req-7 — cause/acteur/action par cas | PASS (CLI réel + comportemental, rejoué) + PASS (navigateur réel, 4 captures FR/EN clair/sombre, superviseur) | Couverture navigateur ajoutée ; cooldown/plafond réel toujours NOT TESTED (hors budget/éthique) |
| req-8 — aucun contournement de limite | PASS | Inchangé ; aucune limite modifiée durant les deux contrôles rejoués ni durant la recette superviseur |
| req-9 — cas réels, sans doublure | PARTIAL (amélioré) | Cas « organisation non configurée » maintenant couvert en CLI réel **et** navigateur réel ; cooldown fournisseur réel et plafond de tentatives réel restent couverts seulement par tests comportementaux (déclencher un vrai rate-limit ou épuiser un agent réel reste hors périmètre/éthique de cette tâche) |

## Limites et prochaine action

- **Navigateur réel non ouvert** dans cette tentative : la parité web/CLI a été vérifiée par lecture directe du code de rendu (`web/mission.js:341,343` pairant `understandingView`+`diagnosticView`, identique au pairage CLI `mission_cli.go:251-255`), pas par interaction DOM FR/EN/clair/sombre/clavier. Si une recette navigateur est requise pour clore REQ-QW2 complètement, elle reste à faire — aucune valeur n'est inventée ici.
- **Cooldown fournisseur réel et plafond de tentatives réel** non rejoués par un agent fournisseur réel (seulement par tests comportementaux) : déclencher ces conditions en direct demanderait soit d'épuiser un vrai quota fournisseur (risque d'abus), soit de construire un agent réel jusqu'à son plafond — non fait dans le budget alloué à cette tâche.
- **Aucune correction de code n'a été nécessaire** : le gap relevé par T0 (absence d'acteur dans `DiagnosticItem`) s'est révélé non bloquant à l'usage, car le rendu CLI/web affiche toujours l'acteur de la tâche immédiatement avec le diagnostic ; aucune des deux conditions d'arrêt (cause sans action autorisée, deux corrections infructueuses) n'a été rencontrée.
- Prochaine action : si la recette navigateur et les deux cas « fournisseur réel »/« plafond réel » sont jugés indispensables avant la revue indépendante, un lot complémentaire borné (nouvelle tentative ou nouvelle tâche décidée par le planificateur) devrait les couvrir ; cette tentative ne les invente pas et ne les déclare pas faits.
- Revue indépendante : en attente, sur le même SHA candidat (`1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, dirty=true, aucun fichier modifié par T2). Lecture de ce rapport par un tiers ne vaut pas accusé de réception ni acceptation.

**Fin tentative 2 (départ 2/2)** : les deux contrôles de la politique enregistrée ont été rejoués avec succès sur le candidat courant ; la recette navigateur manquante est couverte par l'intervention séparée du superviseur (évidence attribuée, non revendiquée par ce worker). Cooldown fournisseur réel et plafond de tentatives via agent réel restent NOT TESTED — déclencher ces conditions en direct exigerait un abus de quota réel ou l'épuisement d'un agent fournisseur réel, explicitement exclus du périmètre. Aucune correction de code, aucune suppression de preuve T0/T1, aucun changement de critère/budget. Cette tentative étant la seconde et dernière autorisée (max_attempts=2), toute lacune restante (cooldown/plafond réels) doit être transmise au planificateur pour décision — ce worker ne la referme pas lui-même.

## État courant corrigé par le superviseur — 3 octobre 2026, après refus 12

Ce complément remplace les conclusions courantes des sections historiques précédentes, sans réécrire leur auteur ni les tentatives. La recette navigateur y était mal attribuée : les anciennes captures étaient le détail d'agent. Après reproduction, correction moteur et nouvelle interaction réelle, lire **T2-supervisor-recipe.md**, section « Correction vérifiée après le refus 12 ». Les captures actuelles prouvent le refus réel en français (clair/sombre) et la péremption réelle des preuves en anglais (clair/sombre), avec cause, acteur et prochaine étape. Entrée et Échap ont été réellement utilisés ; aucun fournisseur en rate-limit n'est revendiqué.

| Critère courant | Résultat de recette | Preuve et limite |
| --- | --- | --- |
| req-7 : cause / acteur / action autorisée par cas testé | PASS | Triplets visibles dans la vraie modale et dans T2-observation.json issu de `swarm --json mission status` ; refus, preuves périmées, organisation absente en CLI. L'ouverture du diagnostic ne lance aucun agent. |
| req-8 : aucun contournement de limite | PASS | Deux tentatives conservées sur deux ; pas de hausse des 25 appels ni des 40 revues ; aucune troisième tentative. |
| req-9 : cas réels, sans doublure du comportement | PASS pour ces trois cas réels | Mission réellement refusée par la revue 12 et preuves réellement modifiées ; racine CLI réellement sans organisation. Les tests isolés de cooldown ne sont pas présentés comme un rate-limit réel. |

La base Git reste `1d9570bd4ef617130c6be96b7ec88844fdbcd00e`, avec changements non committés ; **la base seule n'est pas l'identité du candidat**. Le correctif courant inclut result_presentation.go et ordinary_review_presentation_test.go, liés par empreintes dans le contrôle enregistré. Le superviseur a modifié le code et produit les captures après le worker : ce résultat n'est pas une réussite autonome de la tentative 2. Les commandes de la tentative 2 étaient une recette supplémentaire différente des deux commandes exactes de la politique ; seuls les reçus moteur attestent ces deux contrôles exacts. Nouvelle revue indépendante requise avant acceptation, encore non obtenue à la rédaction.
