# T1 — REQ-QW1 : Résumé avant lancement (web + CLI)

Date : 2026-10-03. Racine : `/home/fpizzi/workspace/swarm-action-skills` (worktree,
`git rev-parse --show-toplevel` = racine courante). Révision candidate :
`1d9570bd4ef617130c6be96b7ec88844fdbcd00e` (HEAD), état non commité au moment de cette
tâche : fichiers T0 + présent livrable uniquement (`git status --short`). Aucune
modification de code dans cette tâche.

Point de départ : inventaire T0 (`docs/plans/clear-launch-recovery/T0-inventaire.md`,
section REQ-QW1) — triplet moteur/CLI/web déjà câblé pour le résumé avant lancement,
avec une lacune ouverte : « modèle demandé/réel/inconnu » non confirmé. Cette tâche
vérifie cette lacune au code et la tranche, plutôt que de la relire sans conclure.

## Ce qui est affiché aujourd'hui avant lancement

**CLI** (`mission_cli.go:98-122`, action `preview`) imprime, dans l'ordre :
- `%d départ(s) possible(s) maintenant · concurrence réelle %d/%d`
- `preview.ConcurrencyDetail`
- « Autorisation examinée une fois : » puis Portée / Budget / Reprises / Validations
  (`preview.Contract.*`, construits par `missionLaunchContract`, `mission_status.go:102-128`)
- une ligne « Départ : » par élément de `preview.Departures`
- une ligne « Attente : … — … » par élément de `preview.Waiting`
- une ligne « Limite : » par élément de `preview.Limits`

**Web** (`web/mission.js`, seul fichier référençant `departures`/`requested_slots`/
`contract.scope`/`concurrency_detail` — confirmé `rg -ln` sur `web/*.js`) : rend le même
objet `MissionLaunchPreview` que la CLI.

**Couverture par rapport au critère req-4** (« objectif, rôle, fournisseur, modèle
demandé/réel/inconnu, profil, skills, périmètre et limites ») :

| Donnée attendue | Montrée aujourd'hui ? | Preuve |
| --- | --- | --- |
| Objectif / périmètre | Oui, dans `Contract.Scope` | `mission_status.go:103-111` (`w.Scope`/`w.Objective`/`w.Title` + nb tâches + dossier) |
| Limites | Oui | `preview.Limits` imprimé ligne à ligne, `mission_cli.go:119-121` |
| Budget / reprises / validations | Oui | `Contract.Budget/Recovery/Validation`, `mission_status.go:112-126` |
| Rôle | **Non** | `profile.Role` n'apparaît dans aucun champ de `MissionLaunchPreview` ni dans la boucle d'impression `mission_cli.go:106-121` |
| Fournisseur | **Non** | `profile.Provider` est lu pour validation (`mission_status.go:142-144`) mais jamais copié dans `preview` ni imprimé |
| Profil / skills | **Non** | `profile.Skills`/`profile.Instruction`/`profile.Timeout` non repris dans `MissionLaunchPreview` (struct listée en T0, `mission_status.go:47-67`) |
| Modèle demandé/réel/inconnu | **Non** — voir section suivante | `mission_status.go:146` |

Le critère req-4 n'est donc **pas** satisfait tel quel : 3 des 7 informations demandées
(rôle, fournisseur, profil/skills) ne sont ni dans la structure retournée au client ni
dans le rendu CLI/web existants. Ce n'est pas une absence inventée : c'est vérifié par
lecture directe du chemin complet moteur → CLI.

## Lacune confirmée : modèle demandé / réel / inconnu

`missionLaunchPreview` appelle la résolution de modèle uniquement pour valider, puis
**jette le résultat** :

```go
// mission_status.go:146
if _, _, err = resolveModel(provider, profile.Level, "work"); err != nil {
    return preview, err
}
```

`resolveModel` (`provider_models.go:202-253`) retourne un `*ModelRoute{Level, Model,
Effort, Billing, PolicyHash, Reason}` — c'est précisément le « modèle réel » (résolu)
distinct du « modèle demandé » (`profile.Level`, un palier comme `simple/standard/exigeant`,
cf. `model.go:169`). Deux cas concrets prouvent que l'écran actuel ne peut pas afficher
ce couple :

1. **Modèle réel résolu et perdu** : la ligne 146 ignore la variable `route` (`_`) — même
   quand la résolution réussit, le modèle réellement choisi n'est jamais transporté
   jusqu'à `preview`, donc jamais imprimé par `mission_cli.go` ni rendu par `web/mission.js`.
2. **Inconnu silencieux, pas de statut explicite** : quand l'adaptateur n'a pas de
   politique de modèles (`effectiveModelPolicy(p) == nil`), `resolveModel` renvoie
   `(p, nil, nil)` sans erreur (`provider_models.go:204-208`) — le modèle réel est
   indéterminé, mais rien ne distingue ce cas de « modèle résolu et affiché » : aucun
   champ `Unknown`/`inconnu` ne porte cette information jusqu'à l'écran. Ceci viole
   directement req-5 (« statut inconnu explicite … jamais de valeur inventée ») : la
   valeur n'est pas inventée, mais elle n'est pas non plus signalée comme inconnue —
   elle est simplement absente.

Ailleurs dans le code, le même `ModelRoute` est bien affiché quand il est conservé :
`prephase_terminal.go:168-169` imprime `"IA : " + provider + " · modèle " + route.Model
+ " · niveau " + route.Level` dans un contexte différent (terminal de préparation, pas
l'aperçu de lancement). Cela confirme que l'app sait afficher un modèle résolu — le
gap est localisé à `missionLaunchPreview`/`MissionLaunchPreview`, pas à l'absence
générale de la donnée dans le moteur.

**Conclusion sur la décision ouverte T0** : la distinction modèle demandé/réel existe
dans le moteur (`ModelRoute` + `profile.Level`) mais n'atteint pas l'écran de résumé
avant lancement, ni en CLI ni en web. Corriger ce point demande de modifier
`MissionLaunchPreview` (ajouter des champs modèle demandé/réel/statut), `missionLaunchPreview`
(conserver `route` au lieu de `_`), `mission_cli.go` (imprimer la nouvelle ligne) et
`web/mission.js` (rendu équivalent) — soit 4 fichiers moteur+CLI+web, hors du périmètre
d'une correction mineure à 2 essais vu le budget restant de cette tentative. Signalé au
planificateur comme décision ouverte (cf. conditions d'arrêt de la tâche), pas implémenté
ici pour éviter un changement non vérifié sous contrainte de budget.

## Vérification exécutée (recette)

| Vérification | Méthode | Résultat | Preuve |
| --- | --- | --- | --- |
| Mécanique d'aperçu existante (pas de mutation, agrégation 1ère vague, prévalidation profil/plafond, pas de contournement de pause) | `go test ./... -run 'TestMissionLaunchPreview\|TestMissionPreview' -v` | **PASS** — 9 tests passés, 1 paquet | Sortie `rtk`: « Go test: 9 passed in 1 packages » |
| Rendu CLI texte (lignes imprimées par `preview`) | Lecture directe `mission_cli.go:98-122` | PASS (lecture statique, confirmée par les tests ci-dessus qui exercent le même appel) | Extrait cité ci-dessus |
| Recette web FR/EN, clair/sombre, focus clavier | Non exécutée | **NOT TESTED** | Aucun outil navigateur disponible dans cette session (pas d'outil Chrome/Playwright chargé) ; se limiter à une inspection source serait substituer la lecture à une preuve d'exécution, ce que le contrat interdit explicitement |
| Cas « modèle demandé ≠ modèle réel » affiché à l'écran | Non exécutable | **NOT TESTED / bloqué par la lacune ci-dessus** | L'écran actuel n'affiche aucun des deux modèles ; il n'y a rien à exercer avant correction |

Aucun doublure n'a remplacé un comportement testé : les 9 tests ci-dessus utilisent le
vrai code `missionLaunchPreview`/dispatcher, pas un mock du moteur.

## Statut des critères assignés

| Critère | Résultat | Raison |
| --- | --- | --- |
| req-4 — Résumé montre objectif/rôle/fournisseur/modèle/profil/skills/périmètre/limites | **PARTIAL** | Objectif, périmètre, limites, budget/reprises/validations présents ; rôle, fournisseur, profil, skills et modèle absents de `MissionLaunchPreview` et du rendu — prouvé par lecture directe, pas supposé |
| req-5 — Statut « inconnu » explicite si donnée manque, jamais inventée | **FAIL** | Aucune valeur n'est inventée (conforme), mais aucune n'est non plus marquée « inconnu » : le cas `resolveModel` → `route == nil` (politique absente) est silencieux, pas signalé |
| req-6 — Recette FR/EN, clair/sombre, focus clavier, cas modèle demandé≠réel | **NOT TESTED (web) / bloqué (cas modèle)** | Pas d'outil navigateur dans cette session pour la recette UI ; le cas modèle ne peut pas être recetté avant que req-4/5 soient corrigés |

## Prochaine action et limites

- **Décision ouverte pour le planificateur** (pas une invention de valeur, pas une
  correction tentée sous budget insuffisant) : faut-il élargir T1 (ou créer une tâche
  de suite) pour (1) conserver `route` dans `missionLaunchPreview`, (2) ajouter
  Rôle/Fournisseur/Profil/Skills/Modèle(demandé, réel, statut) à `MissionLaunchPreview`,
  (3) les imprimer en CLI, (4) les rendre en web ? Portée estimée : 4 fichiers
  (`mission_status.go`, `mission_cli.go`, `web/mission.js`, tests associés), au-delà des
  « deux corrections » prévues par les conditions d'arrêt de cette tâche.
- **Recette web (FR/EN, thèmes, focus clavier) non exécutée** faute d'outil navigateur
  disponible dans cette session — à faire par une tentative disposant de cet outil, pas
  à simuler ici.
- Revue indépendante requise sur ce SHA (`1d9570bd4ef617130c6be96b7ec88844fdbcd00e` +
  état non commité ci-dessus) avant toute proposition de livraison de T1.
- Aucune acceptation déclarée ; aucune modification de code effectuée par cette tâche.
