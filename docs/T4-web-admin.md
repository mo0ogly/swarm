# T4 — Web Administration (lecture/édition, preview, historique, rollback)

Date : 2026-09-30. Mission w-843bb3ce22cff2965c5e77b6, tâche plan-843bb3ce22-T4,
tentative a-6d399840bdcc9aaea0fe857d (agent 07be2e8a-6306-47ae-9d2e-388230685a80),
départ 4/3. Révision candidate HEAD `097e745a9b1404f16ed6fdf4ad32124a3f661e05`
(diff local présent : 89 fichiers, `git status --porcelain` non vide).

## Résultat en deux phrases

L'écran Administration (`web/admin.js` + section `#admin` de `web/index.html` +
câblage `web/cockpit.js`) est fonctionnel et couvre les 4 critères assignés
(req-14 à req-17), vérifié par 12 tests Go ciblés et une recette navigateur
FR/EN × thèmes clair/sombre (20 assertions, 0 erreur console). Rien n'est bloqué ;
aucune modification supplémentaire n'a été nécessaire cette tentative.

## État hérité vs constaté

Le rapport `docs/plan-843bb3ce22-T4.md` précédent (départ 2/3) s'arrêtait avant
la construction de l'écran web ("Prochaine action : construire l'écran web").
Cette tentative a constaté que l'écran existe déjà dans l'arbre de travail
(fichiers non commités, issus d'une tentative intermédiaire non reflétée dans
l'ancien rapport) : `web/admin.js` (150 lignes), routes `run_limits_web.go`
(175 lignes, inchangé cette tentative), tests `run_limits_web_test.go`
(190 lignes), recette `tests/run_limits_admin_ui.cjs` (146 lignes), et le script
npm `test:admin` déjà déclaré dans `package.json`. Aucun fichier applicatif n'a
été modifié pendant cette tentative : seul le contrôle a été rejoué et ce
rapport + `docs/plan-843bb3ce22-T4.md` ont été mis à jour.

## Preuves par critère

| Exigence | Constat | Commande / preuve | Résultat |
| --- | --- | --- | --- |
| req-14 — Aucun secret affiché | Scan du texte DOM de l'écran Administration (hors la phrase d'avertissement elle-même) contre les motifs `api[_-]?key\|bearer\|secret\|password\|mot de passe` (`tests/run_limits_admin_ui.cjs:39-41`) | `npm run test:admin` (recette ci-dessous) | PASS (FR + EN) |
| req-15 — FR/EN × thèmes clair/sombre | Capture d'écran de l'état admin et du mode observation, en FR et EN, thème `etat` (clair) et `sombre`, dans une racine `.swarm` isolée (`fs.mkdtempSync`) | `npm run test:admin` → 8 captures dans `test-results/admin/` | PASS (8/8 captures produites, 0 erreur `pageerror`) |
| req-16 — Clavier + erreurs/vide | État vide vérifié (portée rôle sans valeur/historique) ; `Échap` ferme la modale sans écrire et restitue le focus au bouton d'origine ; révision périmée (409) refusée sans écriture silencieuse | `npm run test:admin` | PASS (FR + EN) |
| req-17 — Preview/validation/historique/rollback | Aperçu affiché sans écriture (appel `POST /api/v1/run-limits/preview`, validateurs partagés avec `configureRunLimits`) ; application en révision 1 ; rechargement reflète un changement externe (révision 2) ; retour à la révision 1 confirmé avec historique conservé (révision 3) ; résolution de l'effectif futur départ (REQ-ADM-05) | `npm run test:admin` + `go test -run TestRunLimits ./...` | PASS (12 tests Go + 10 assertions navigateur par langue) |

## Commandes rejouées cette tentative (2026-09-30)

```
go build ./...                          → exit 0
go test -run TestRunLimits ./...        → "Go test: 12 passed in 1 packages", exit 0
node tests/i18n_test.cjs                → PASS catalogue parity, exit 0
go build -o bin/swarm .                 → exit 0
npm run test:admin                      → status PASS, 20 checks, 0 errors, exit 0
```

Artefacts recette : `test-results/admin/result.json` (status `PASS`, `errors: []`,
racine temporaire `/tmp/swarm-run-limits-ui-Si8oCi`) et 8 captures PNG
(`admin-fr-etat.png`, `admin-fr-sombre.png`, `admin-en-etat.png`,
`admin-en-sombre.png`, `observation-{fr,en}-{etat,sombre}.png`).

## Réutilisation, pas de seconde source de vérité (REQ-ADM-03)

`run_limits_web.go` est un adaptateur HTTP pur, calqué sur `provider_admin.go` :
- `POST /api/v1/run-limits/preview` n'écrit rien, appelle uniquement les mêmes
  validateurs que `configureRunLimits` (`validRunLimitsScope`/`validRunLimitsOverride`).
- `POST /api/v1/run-limits/apply` délègue à `s.configureRunLimits` (T2, non modifié).
- `POST /api/v1/run-limits/rollback` délègue à `s.rollbackRunLimits` (T2, non modifié).
- Aucune ligne de `run_limits_admin.go` (T3, CLI) ni `run_limits.go` (T2, moteur,
  actuellement modifié par une autre tâche — non touché ici) n'a été modifiée par T4.

## Limites et risques résiduels

- Le diff de travail contient 89 fichiers modifiés/non suivis au total (autres
  tâches du plan en cours : T2/T3/T5/T6, quick wins). Cette tentative n'a lu/joué
  que les fichiers du périmètre T4 listés ci-dessus ; aucun autre fichier n'a été
  modifié.
- `run_limits.go` apparaît modifié (`M`) dans l'arbre de travail par une tâche
  tierce (T2) au moment de ce contrôle ; T4 s'appuie dessus en lecture seule via
  les mêmes validateurs, sans le modifier.
- Cette vérification est un contrôle personnel du worker (DO/CHECK du PDCA), pas
  une revue indépendante. Aucune acceptation n'est déclarée ; le moteur Swarm
  reste seul décisionnaire.
- Script de contrôle reproductible fourni séparément :
  `tools/verification/t4_admin.py` (base isolée, pour le futur vérificateur).

## Prochaine action

Aucune lacune constatée sur req-14 à req-17 avec les preuves ci-dessus. Prochaine
action recommandée au responsable : lancer la revue indépendante sur la révision
candidate ci-dessus (avec le diff non commité inclus) avant intégration, en
incluant T2/T3/T5 dont T4 dépend.
