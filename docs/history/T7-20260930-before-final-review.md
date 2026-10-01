# T7 — Résumé de lancement et diagnostic copiable

## Reprise courante — revue rédactionnelle du 30 septembre 2026

Cette section décrit le candidat courant et remplace les statuts rédactionnels
historiques ci-dessous, conservés pour la traçabilité. Base Git :
`7110a0b805caf4be1bb09a57b776176b8dc52a68`, avec modifications non commitées.
Production conservée : `a-b7e759aea886550cc74b69fa` ; aucune nouvelle tentative.
Les contrôles moteur précédents sont passés (21 secondes, code 0). Le nouvel
examen de ce rapport exige un reçu renouvelé car son contenu a changé.

### req-22 / critère 3 — textes examinés et jugement motivé

Relecture par Codex des captures réelles issues de la recette navigateur, et du
code des assertions `tests/prelaunch_diagnostic_ui.cjs` (résumé et prévol lignes
50–105, diagnostic lignes 150–175). Cette relecture est une intervention externe
du superviseur, **pas un avis indépendant** et pas une acceptation de la tâche.
Le contenu ci-dessous permet d'examiner la rédaction elle-même, au-delà d'une
simple déclaration « traduction présente ».

| Situation | Texte français observé | Texte anglais observé | Analyse de compréhension |
| --- | --- | --- | --- |
| Récapitulatif | Récapitulatif avant autorisation | Summary before authorization | Le titre situe l'étape avant toute permission de départ. |
| Responsabilités | Responsable ; Exécutants ; Revue et acceptation | Owner ; Workers ; Review and acceptance | Trois fonctions séparées, modèles affichés sous chacune ; la revue n'est pas confondue avec la production. |
| Bornes | Tâches (plan) : 15 max · Appels par rôle (responsable/vérificateur) : 25 max | Tasks (plan): 20 max · Calls per role (owner/reviewer): 40 max | L'unité et la portée sont explicites. Les nombres diffèrent car les captures anglaises reprennent le formulaire initial, avant modification. Ce ne sont pas les plafonds détaillés de tous les exécutants. |
| Avant contrôle | Vérifier avant le lancement | Check before launch | Verbe d'action et moment du contrôle explicites ; ce bouton ne prétend pas lancer un agent. |
| Conditions remplies | Conditions vérifiées. Vous pouvez autoriser cette équipe. | Conditions checked. You can authorize this team. | Deux phrases : résultat du contrôle puis prochaine action permise. Ni « démarré », ni « terminé ». La phrase anglaise est aussi vérifiée exactement par la recette. |
| Aide technique | Voir le détail des vérifications | Pre-launch check details (titre de la modale) | Les états ready/verified et mesures en octets figurent dans une modale, non dans le message principal. |
| Effet de l'aide | Ces informations décrivent les contrôles effectués. Elles ne lancent aucun agent. | This information describes the checks performed. It does not start any agent. | Sépare clairement consultation et exécution. |
| Fermeture | Fermer les détails | Close details | Bouton textuel ; Échap et retour du focus au déclencheur vérifiés. |
| Copie | Copier le diagnostic ; Diagnostic copié. | Copy diagnostic ; Diagnostic copied. | Action et résultat distincts. Le refus de copie est également testé et ne donne pas de faux succès. |
| Échec d'un contrôle | Cause : Un test ou contrôle observable a échoué. Conséquence : Le critère couvert par ce contrôle reste non validé. Action disponible : Examiner la trace, corriger la cause, puis rejouer le même contrôle avec les mêmes options avant toute validation. | Cause: An observable test or check failed. Consequence: The criterion covered by this check remains unvalidated. Available action: Inspect the trace, fix the cause, then rerun the same check with the same options before validation. | Cause, impact et prochaine action sont séparés ; aucune validation n'est annoncée prématurément. |

Captures réellement ouvertes pour cette relecture, dans `test-results/t7-engine/` :
`summary-ready-etat.png`, `summary-ready-sombre.png`, `summary-en-etat.png`,
`summary-en-sombre.png`, `diagnostic-copy-etat.png`, `diagnostic-copy-en-etat.png`,
`preflight-details-fr-sombre.png`, `preflight-details-en-etat.png`.

Résultat local de la relecture : **PASS sur les textes du parcours T7 examinés**.
Motifs : verbes d'action explicites, séparation condition/autorisation/départ,
libellés de rôles distincts, diagnostics cause/conséquence/action, explications
techniques dans la modale ou les traces repliées. Le passage historique PARTIAL
correspondait à l'absence de cette relecture, non à un défaut rédactionnel identifié.

Limites : « Preflight not run » reste un terme technique anglais dans la ligne
synthétique des prérequis, mais le bouton adjacent « Check before launch » et
« Run launch checks before authorizing this team. » donnent l'action concrète.
Les diagnostics détaillés restent longs ; leurs traces sont repliées. Cette revue
ne prétend ni tester la compréhension auprès d'utilisateurs, ni auditer tous les
textes du produit. Les noms de fournisseur et de modèle ne sont pas traduits.
Les fixtures et le presse-papiers simulé ne démontrent pas un fournisseur réel.
L'avis indépendant et la décision finale restent à obtenir.

### Identification actuelle des fichiers du parcours

Ces empreintes ont été comparées aux fichiers locaux et au manifeste de la
recette avant cette remise. Elles remplacent les anciennes valeurs du rapport.

```json
{
  "locales/en.json": "1b87a59656a89c4900b68b7fe76c9b8985f4e4fceb4a0c2b968cacfab5bdaa44",
  "tests/prelaunch_diagnostic_ui.cjs": "62a937fe397c40a55cba86bac677d0c8e680abccade234ae445c0147e89734f5",
  "web/i18n-en.js": "990e49a89c1a68bbad8e045a5220790bda704238855c9b145b291ea30ca2db6f",
  "web/mission.js": "d8582ea7325fc9fe9756e87bbfe56d91e3d907062afc2ac7efd6994be0f4ea3a",
  "web/prepare.html": "301246c624aa5994f9c042e1bffe21821b629263efe3a3b44875139fc876c593",
  "web/prephase-conversion.js": "c009b494ffb4d1e594e54cddade342a10068d20e0cb2d87c41079fa22ac801e6",
  "web/prephase-editor.js": "859de925d60b31720a09d47f9d0ccf05b1b572a8f334d47812f54d4123c942d2",
  "web/prephase.js": "29c4c3428c975162ea1dd2cfed6e2ff532036f0a50cbe290871671d00f62825d"
}
```

## Historique — résultat avant transmission des contrôles moteur

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

## Tentative a-b7e759aea886550cc74b69fa (départ 2/3, agent auto-98dfe747755ddc17642a) — 2026-09-30

Révision : HEAD `7110a0b805caf4be1bb09a57b776176b8dc52a68` ; fichiers web et recette de T7 déjà commités, arbre modifié seulement pour docs, `provider_models*.go` et `tools/verification/t5_ci.py` (hors T7). Aucun fichier applicatif modifié par cette tentative ; seuls ce rapport et `docs/T7-documentation-retex.md` sont écrits.

| Critère | Contrôle rejoué | Résultat | Limite |
| --- | --- | --- | --- |
| req-20 | `CHROME_BIN=/usr/bin/google-chrome python3 tools/verification/t7_prelaunch.py all`, code 0 : quatre assertions du résumé (limites par défaut, limites suivant les plafonds saisis, rôles/modèles résolus, prérequis vides après prévol) | PASS (fixtures locales) | Plafonds du plan et des rôles planification/revue seulement ; pas de fournisseur réel. |
| req-21 | Même exécution : diagnostic FR et EN copié sans URL ni chemin d'espace de travail ; refus du presse-papiers = message explicite. Relecture de `web/mission.js:37-52` : le texte copié contient tentative, version, cause, action ; les traces techniques sont exclues. | PASS (presse-papiers simulé) | Les champs `summary`/`cause`/`action` viennent du serveur et ne sont pas filtrés côté navigateur ; l'absence d'URL est prouvée pour les fixtures, pas pour toute cause future. |
| req-22 | Même exécution : résumé et prévol traduits EN, détails techniques dans une modale (Échap, focus restitué), `tests/i18n_test.cjs` parité du catalogue. | PARTIAL | Clarté rédactionnelle = jugement de revue indépendante, non rendu ici. |

Correction documentaire : les empreintes listées plus haut sont historiques. Les changements ultérieurs de mission.js et du catalogue les ont remplacées ; elles ne décrivent pas le candidat actuel. Les empreintes actuelles figurent dans la section de reprise en tête et dans le reçu moteur (14 captures présentes) ; `git diff --check` code 0. Aucune correction nécessaire, aucun défaut constaté, pas de relance identique.

État : candidat à examiner, non accepté. Revue indépendante et gate fraîche toujours requises. Prochaine action : responsable de plan — lancer la revue indépendante sur le même candidat (HEAD + diff sale), avec attention au libellé des textes (req-22).

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


## Reprise après expiration du quota — 30 septembre, après 15 h

`CHROME_BIN=/usr/bin/google-chrome python3 tools/verification/t7_prelaunch.py all`
a terminé avec le code 0 : prérequis, construction du binaire, recette navigateur
FR/EN dans les deux thèmes, parité i18n, empreintes fraîches et quatorze captures.
Le résultat local reste distinct de la revue indépendante et de l'acceptation.
La base livrée est maintenant 7110a0b805caf4be1bb09a57b776176b8dc52a68 ; les
fichiers contrôlés sont identifiés par test-results/t7-engine/sources.json.
Le catalogue de modèles Claude 5.5 comporte des modifications locales séparées.
Les tests utilisent des fournisseurs simulés et une frontière presse-papiers
simulée, comme décrit plus haut. Le livrable est soumis sans prétendre que le
processus Claude interrompu aurait repris ou terminé avec succès.
