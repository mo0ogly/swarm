# T7 — Résumé de lancement et diagnostic copiable

## Résultat et limite actuelle

L’implémentation Claude a été récupérée après son interruption fournisseur 429 : 127 appels d’outils et 127 résultats sont conservés. Codex a repris directement les fichiers et corrigé la recette des thèmes ; cette intervention externe ne transforme pas la tentative Claude en exécution réussie et ne constitue pas une vérification indépendante.

Mission : `w-843bb3ce22cff2965c5e77b6`. Tâche : `plan-843bb3ce22-T7`.
Tentative interrompue : `a-2274571f4df60c2ea5dbff23`, agent `auto-c4a11448f63053bb705e`.
Base Git : `097e745a9b1404f16ed6fdf4ad32124a3f661e05`, modifications non commitées conservées.
État : candidat à examiner, aucune acceptation déclarée.

## Changements

- `web/prephase-conversion.js`, `web/prepare.html` : récapitulatif des plafonds du plan et des rôles responsable/vérificateur, prérequis de fournisseur, résolution des modèles, dossier, validation et prévol.
- `web/mission.js` : bouton copier le diagnostic (cause, tentative, version, action) ; les traces détaillées ne sont pas copiées. Le refus du presse-papiers donne un message explicite.
- `web/prephase-editor.js` : état initial et bouton corrects en éditeur simple, messages distinguant les deux modes. Retrait de la mention sur les petits écrans de l’infobulle.
- Traduction de la dépendance vide « aucune » dans la conversion anglaise.
- `tests/prelaunch_diagnostic_ui.cjs` : deux thèmes réellement contrôlés dans le DOM, captures cadrées, assertion du refus de permission, clic robuste aux rafraîchissements du cockpit.

## Contrôles reproductibles

`python3 tools/verification/t7_prelaunch.py browser` construit un binaire temporaire et exécute la recette sur des racines et bases isolées. Le fichier `test-results/t7-engine/sources.json` lie les résultats aux empreintes des fichiers concernés. `python3 tools/verification/t7_prelaunch.py validation` refuse les preuves périmées.

| Exigence | Preuve contrôlée | Limite |
| --- | --- | --- |
| REQ-SUM-01 / req-20 | Plafonds 20/40 puis 15/25 reflétés, rôles et modèles résolus, prérequis avant/après prévol | Ces plafonds concernent le plan et les rôles de planification/revue, pas tous les paramètres détaillés des exécutants. |
| REQ-DIAG-01 / req-21 | Cause/tentative/version/action FR/EN, absence d’URL et de chemin de travail dans le diagnostic copié, refus du presse-papiers | Frontière presse-papiers système simulée ; pas une preuve de permission du navigateur personnel. |
| req-22 | Catalogue bilingue et captures réelles des deux thèmes ; correction des libellés concernés | La clarté de tous les textes reste un jugement de revue ; les détails de prévol sont désormais dans une modale, avec fermeture et retour du focus testés. Ne pas déclarer ce critère accepté par une simple assertion de traduction. |

Les fournisseurs de la recette sont des fixtures locales. Aucun appel fournisseur réel ni aucune écriture dans la base de la mission réelle pendant ces tests.

## RETEX ciblé

Le coût des 127 appels ne suffit pas à conclure à une boucle du moteur. Une partie correspond aux itérations de recette navigateur et au presse-papiers sans focus : il faut distinguer appels d’outils, requêtes de modèle et jetons avant toute analyse de coût. Le défaut observé dans la recette est précis : cliquer le bouton de thème derrière une modale ne changeait pas le thème, mais le nom du PNG annonçait malgré tout « sombre ».

Prochaine étape : examiner le candidat et les limites ci-dessus, compléter les écarts qualitatifs, puis remettre un rapport via le parcours public de Swarm. La revue Claude est indisponible jusqu’à l’échéance fournisseur annoncée, 2026-09-30 13:00 UTC ; aucun délai effacé et aucune consommation remise à zéro.


## Vérifications exécutées par Codex — 30 septembre 2026

- `python3 tools/verification/t7_prelaunch.py browser` : code 0, dix assertions/parcours annoncés, quatorze captures ; diagnostic anglais vérifié jusque dans les causes/actions, permission de copie refusée également testée.
- `python3 tools/verification/t7_prelaunch.py delivery` : code 0, empreintes actuelles et captures présentes.
- `npm test` : code 0 (graphe, pilotage, rafraîchissement, runner d’audit, i18n).
- `python3 tools/agent-workflows/check.py` et `git diff --check` : code 0.
- Contrôle visuel : résumé clair/sombre et diagnostic anglais sombre examinés. Anciennes captures aux thèmes mal nommés non retenues.
- Deux défauts de recette corrigés : clic thème ignoré derrière la modale ; poignée DOM périmée lors du rafraîchissement. Une dépendance `Pilot.command` manquante dans le double du test unitaire a également été ajoutée.

Empreintes du candidat testé :

```json
{
  "locales/en.json": "c6fc5d5c470c7663468cdbb678aaae1902d83de66fc8ffa71ec048d5644ac830",
  "tests/prelaunch_diagnostic_ui.cjs": "62a937fe397c40a55cba86bac677d0c8e680abccade234ae445c0147e89734f5",
  "web/i18n-en.js": "19fe72e01d9a134e5adebfe1348ca972b4994f20dda9101b1282847bc8233381",
  "web/mission.js": "59cae9bdcae6f118f11746e072fa5e38b51b8d0ed7631f50f9d3202385df6943",
  "web/prepare.html": "301246c624aa5994f9c042e1bffe21821b629263efe3a3b44875139fc876c593",
  "web/prephase-conversion.js": "c009b494ffb4d1e594e54cddade342a10068d20e0cb2d87c41079fa22ac801e6",
  "web/prephase-editor.js": "859de925d60b31720a09d47f9d0ccf05b1b572a8f334d47812f54d4123c942d2",
  "web/prephase.js": "29c4c3428c975162ea1dd2cfed6e2ff532036f0a50cbe290871671d00f62825d"
}
```

## Complément de reprise — 30 septembre

Résumé court du prévol et détails dans une modale FR/EN, navigation clavier/focus testée et captures des deux thèmes examinées. Le récapitulatif signale les arguments de contrôles automatiques encore manquants. La documentation INSTALL FR/EN est enrichie en parallèle, avec provenance des captures et distinction entre saisie, enregistrement et lancement. Aucun statut de mission modifié.

## Historique de la tentative Claude (conservé)

# Suivi — T7 — Résumé de lancement, diagnostic copiable et textes utiles

Mission w-843bb3ce22cff2965c5e77b6, tâche plan-843bb3ce22-T7, agent
auto-c4a11448f63053bb705e, tentative a-2274571f4df60c2ea5dbff23, départ 1/3.
Racine de travail : `/home/fpizzi/workspace/swarm-engine-contract/source`
(branche `codex/engine-review-contract`, arbre de travail avec modifications
préexistantes non liées à cette tâche).

## Critères — état initial (non vérifiés)

- req-20 — Résumé fidèle aux limites et rôles effectifs : NON VÉRIFIÉ
- req-21 — Diagnostic copié sans secret ni URL de session : NON VÉRIFIÉ
- req-22 — Textes du parcours compréhensibles en FR/EN, détails techniques dans
  les aides : NON VÉRIFIÉ

## Constat initial (lecture, avant modification)

- T4 et T6 (dépendances de gate d'entrée) ont chacun un rapport daté du
  2026-09-30 avec preuves PASS (`docs/T4-web-admin.md`, `docs/T6-quick-wins.md`).
  Acceptation effective non vérifiée par cette tentative (hors périmètre) ;
  seule la présence de preuves datées est constatée ici.
- `docs/T1-inventaire-existant.md` (Lot 0) documente déjà précisément l'écart
  REQ-SUM-01 (`web/pilot-actions.js` résumé partiel, rôles/modèles par tâche et
  prérequis manquants absents) et REQ-DIAG-01 (aucun bouton copier/
  `navigator.clipboard` trouvé côté web ; backend `attempt_diagnostic.go`
  réutilisable).
- Recherche complémentaire cette tentative : le vrai « résumé pré-lancement »
  couvrant rôles/modèles est `#organization-summary` dans `web/prepare.html`
  (alimenté par `PreparationConversion.updateSummary()` dans
  `web/prephase-conversion.js`), pas le bloc `#prepared-launch-summary` de
  `web/pilot-actions.js` (qui ne concerne que la reprise d'un lancement déjà
  préparé). Il manque les limites effectives (plafonds tâches/appels) et les
  prérequis manquants.
- `locales/en.json` contient des entrées orphelines (jamais appelées dans
  `web/*.js`) correspondant exactement aux thèmes cités dans le périmètre :
  éditeur adapté aux petits écrans, bascule éditeur enrichi, mode Tab, prévol.
  `web/prephase-editor.js:37` (`toggle()`) a un bug réel : les deux branches du
  ternaire appellent `modeMessage` avec le même texte, donc le message affiché
  ne distingue jamais éditeur simple/enrichi ; de plus la branche « simple dès
  le départ » de `initialize()` (petits écrans) n'appelle jamais `showMode()`,
  donc le libellé du bouton de bascule reste incorrect au chargement sur petit
  écran.

## Plan d'action (ciblé, sans redonner les écrans)

1. Étendre `#organization-summary` (REQ-SUM-01) : deux lignes `<dt>/<dd>`
   supplémentaires — limites effectives (plafonds tâches/appels) et prérequis
   manquants — calculées dans `updateSummary()`.
2. Ajouter un bouton « Copier le diagnostic » (REQ-DIAG-01) dans
   `Mission.diagnosticView()` (`web/mission.js`), source de données =
   `AttemptDiagnostic` déjà transmis (cause/tentative/version/prochaine
   action), sans traces techniques ni URL de session dans le texte copié.
3. Corriger le bug de bascule d'éditeur (`web/prephase-editor.js`) en
   réutilisant les libellés déjà traduits mais orphelins, au lieu d'en
   inventer de nouveaux.
4. Ajouter les nouvelles clés de traduction strictement nécessaires à 1 et 2
   dans `locales/en.json`, régénérer `web/i18n-en.js` via
   `npm run i18n:build`.
5. Contrôles : `go build ./...`, `node tests/i18n_test.cjs`, recette
   navigateur ciblée (à écrire/adapter), `npm run test:i18n-ui` si le temps le
   permet sans dépasser le périmètre.

## Prochaine action

Implémenter les points 1 à 4 ci-dessus, puis rejouer les contrôles listés au
point 5 et mettre à jour ce fichier avec les résultats réels.
