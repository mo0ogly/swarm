# T3 — QW6 : état actuel et historique — rapport (tentative 2/2)

Mission w-01567e073c1ed2f3d4c71c9e ; tâche plan-01567e073c-T3 ; agent 1426ec9c-5a9d-4d45-85ec-316341a236eb ;
tentative a-008c49f126275727492dfa40 ; départ 2/2. Racine vérifiée : `/home/fpizzi/workspace/swarm-action-skills`
(`pwd` = `git rev-parse --show-toplevel`), branche `codex/clear-launch-recovery`. HEAD `cc3069dc7bb61b90168d21f945cb2eb5e27578ed`
(cf. `docs/plan-01567e073c-T2.md`), candidat non commité : `web/*.js`, `locales/en.json`, `docs/plans/clear-launch-recovery/RETEX-CLOTURE.md`
(diff en cours, hérité de T1/T2, préservé — non touché par cette tentative).

Tentative précédente `auto-21e1c20e88473e830f46` : interrompue à 12 lectures/outils, **aucun rapport écrit** (constat
confirmé : `docs/plan-01567e073c-T3.md` absent au début de cette tentative). Ses opérations sont historiques et non
reprises telles quelles ; cette tentative relit directement l'état courant.

## Outcome en deux phrases

Code de présentation `mission_status.go`/`mission_insights.go` inspecté : la logique attendue pour req-7 et req-8
existe déjà dans le candidat (clôture ne masque pas l'historique, blocage courant distinct d'un ancien incident,
aucune suppression d'événement) et la suite de tests existante passe sans régression. **Aucun test dédié ne
démontre ce scénario précis** (recherche ciblée : zéro résultat) et **req-9 (FR/EN, deux thèmes, clavier/focus,
CLI isolé) n'a pas pu être rejoué** dans le budget restant : livraison **PARTIELLE**, à compléter ou arbitrer par
le responsable.

## Analyse (req-7, req-8) — lecture de code

- `mission_status.go:574-588` : quand `root.State == "closed"`, les branches qui réinjectent `Planning.Failure` ou
  un plafond de planification comme blocage actif sont explicitement conditionnées à `root.State != "closed"` —
  un ancien `Planning.Failure` ne peut donc pas réapparaître comme blocage sur une mission clôturée. **req-7**.
- `mission_status.go:594-598` : quand clôturée et tout validé, `EvidenceStage` devient
  « Résultats validés et responsabilité racine clôturée dans cette mission. », remplaçant le message par défaut
  qui parle de parcours « non encore vérifié ». **req-7**.
- `mission_status.go:776-782` (commentaire source : « A completed mission can retain unconsumed exchanges for
  provenance. Do not turn their historical state into a new instruction or mutate it. ») : si la mission est
  « terminée », les échanges non consommés sont présentés comme conservés dans l'historique, jamais réinterprétés
  en instruction active. Aucune suppression d'événement nulle part dans `missionChanges`/`lastRefusalRevision`
  (lecture seule sur la table `events`, `LIMIT 500`/`201`, jamais de `DELETE`). **req-8** (historique intact).
- Blocage réellement courant (tâche non close, `t.Blocker` non vide, ou tentative échouée en attente de reprise)
  reste affiché via `missionDispatchState`/`taskUnderstanding` indépendamment de l'état de clôture de la mission —
  la clôture de la mission racine n'efface pas l'état `intervention`/`waiting` d'une tâche encore ouverte. **req-8**.

Cette lecture de code est cohérente avec les critères, mais **reste une lecture statique** : aucune mission
fixture clôturée-avec-ancien-incident n'a été construite et rejouée dans cette tentative (budget insuffisant pour
fabriquer un scénario fiable sans risquer un test non compilable en fin de budget — voir Limites).

## Contrôles exécutés

| # | Commande | Résultat | Preuve |
| --- | --- | --- | --- |
| 1 | `go build ./...` | exit 0, aucune erreur | sortie commande ci-dessous |
| 2 | `go test ./... -run 'Mission' -count=1` | 79 tests passés, 0 échec (suite existante, 1 paquet) | sortie commande ci-dessous |
| 3 | `rg -n "termine\|closed\|historique\|conservé" mission_status_test.go mission_insights_test.go` | 1 résultat, aucun ne couvre le scénario clôture/historique de req-7/req-8 | sortie commande ci-dessous |
| 4 | `rg -n "EvidenceStage\|clôturée\|root.State\|échanges conservés"` sur les mêmes fichiers de test + `planning_test.go` | aucune correspondance sur le scénario ciblé | sortie commande ci-dessous |

Sorties brutes :
```
$ go build ./...
(aucune sortie, exit 0)

$ go test ./... -run 'Mission' -count=1
Go test: 79 passed in 1 packages

$ rg -n "termine|closed|historique|conservé|Conservé" mission_status_test.go mission_insights_test.go
mission_insights_test.go:114:   if e = missionCLI(s, []string{"mission", "recovery", w.ID, "t1", a.ID}, "", false, &out); e != nil || !strings.Contains(out.String(), "Ce qui sera conservé") {

$ rg -n "EvidenceStage|clôturée|root.State|Planning.*closed|exchanges conservés|échanges conservés" mission_status_test.go mission_insights_test.go planning_test.go
planning_test.go:405:   if w.Planning.Scopes[0].State != "closed" {   (test sur l'état de clôture lui-même, pas sur sa présentation dans mission_status)
```

**NOT TESTED — req-9** : aucun parcours web FR/EN, deux thèmes, clavier/focus, ni CLI sur racine isolée n'a été
rejoué dans cette tentative. Aucune capture produite. Raison : budget d'exploration épuisé (jalon « arrêt
exploration à l'appel 9 ») avant d'atteindre cette vérification ; pas de contournement tenté.

## Limites et écarts déclarés

- **req-7 / req-8** : PARTIEL. Logique présente et lue dans le candidat courant, suite existante au vert (pas de
  régression), mais **aucune preuve comportementale nouvelle** (pas de fixture « mission close + ancien incident +
  tâche réellement bloquée + historique » exécutée). Ne pas déclarer PASS comportemental sur cette seule lecture.
- **req-9** : NON TESTÉ, intégralement — aucune démonstration FR/EN, thèmes, clavier/focus, CLI isolé.
- Budget de cette tentative (jalons superviseur : 12 appels, 3 réservés contrôles/rapport, 3 max exploration
  initiale, arrêt exploration à l'appel 9) atteint sans marge pour construire une fixture de test fiable ; tenter
  une telle fixture dans les derniers appels risquait un échec de compilation non corrigible dans le budget
  restant — non tenté plutôt que livré fragile.
- Aucun fichier applicatif modifié par cette tentative : seul ce rapport a été écrit. `git status` inchangé pour
  le reste (diff T1/T2 préservé intact).
- Aucune suppression, réécriture ou modification de la base `.swarm`/événements n'a été effectuée ou proposée.

## Prochaine action et responsable

Le responsable doit décider entre :
1. Accepter la preuve partielle actuelle (lecture de code + suite existante au vert) comme suffisante pour req-7/req-8
   et planifier séparément la vérification req-9 (FR/EN, thèmes, clavier, CLI isolé) et le test de régression
   manquant pour le scénario clôture/historique ; ou
2. Rouvrir T3 avec un budget dédié à la construction d'une fixture de test (mission close + ancien incident +
   tâche bloquée courante + historique) et au parcours web/CLI complet.

Aucune tâche n'est créée par cette tentative. Avis indépendant et acceptation moteur restent à obtenir séparément
sur ce candidat (HEAD `cc3069d` + diff non commité) ; cette tentative ne s'auto-valide pas.
