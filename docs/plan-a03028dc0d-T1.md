# Task result — plan-a03028dc0d-T1

## Outcome in two sentences

R1 est implémenté sur le candidat non commité et les quatre critères locaux sont `PASS` d’après les contrôles ciblés et les recettes CLI/web conservées. Les contrôles ont été exécutés par le superviseur, pas par l’agent de cette reprise ; la revue indépendante et l’acceptation moteur restent requises, et la commande/code de sortie de la fixture finale ne sont pas consignés séparément.

## Identity and scope

- Work / task / attempt / producer : `w-a03028dc0d3f69d1f52f0ee6` / `plan-a03028dc0d-T1` / `a-116e8bdca5f90da16b18d2cd` / `4f20368f-4fec-4d56-a68c-eedf702fb3c0` (départ `2/2`, révision de travail fournie `6`).
- Rôle et périmètre : worker, remise R1 uniquement ; aucune nouvelle implémentation ni exécution de test pendant cette reprise.
- Base Git observée : `cc3069dc7bb61b90168d21f945cb2eb5e27578ed` ; candidat non commité et checkout global déjà sale avec des changements hors T1 préservés.
- Diff R1 observé : `main.go`, `version.go`, `version_diagnostic.go` (non suivi), `runtime_health.go`, `runtime_health_test.go`, `version_test.go`, `web/runtime-health.js`, `tests/version_ui.cjs`, `locales/en.json`, `web/i18n-en.js`.
- État : implémentation et vérification personnelle documentées ; ni commit, ni push, ni acceptation revendiquée. Le statut `5/6` de `w-01567e073c1ed2f3d4c71c9e` n’a pas été modifié.

## Findings the responsible planner must know

Les sources canoniques retenues sont : binaire appelant pour `installed_cli`, `/api/v1/runtime-health` du serveur local explicitement fourni par `--server` pour `active_server`, et `HEAD` Git de `--root` pour `candidate`. Le même objet structuré (`installed_cli`, `active_server`, `candidate`, `state`, `differences`, `next_step`) alimente le JSON CLI et `version_diagnostic` de runtime-health ; le navigateur déclare le CLI local inaccessible au lieu de l’inventer.

La tentative précédente a été interrompue sur la suite globale non bornée après 300 s. Le superviseur a ensuite reconstruit le même candidat et exécuté les contrôles ciblés ainsi que le serveur TCP isolé ; les preuves ci-dessous lui sont attribuées. Les deux premiers essais navigateur réels, conservés en échec, concernaient des attentes erronées de la recette ; la recette finale corrigée est celle citée ici.

## Changes and verification

| Requirement | Expected observable | Check / environment / attribution | Result | Evidence / limit |
| --- | --- | --- | --- | --- |
| `req-1` | CLI installée, serveur actif et candidat ont des libellés distincts. | Superviseur : `go test ./... -run 'TestVersion|TestServerVersion|TestRuntimeHealthEndpoint' -count=1` (code `0`) ; recette réelle `node tests/version_ui.cjs /tmp/swarm-r1-recovery-candidate docs/plans/supervision-fiable/T1-version-ui-real-final` (code `0`). | `PASS` | Test ciblé des trois observations ; recette réelle : 10 contrôles, FR/EN, deux thèmes, cockpit/préparation, `errors=[]`, `failed=[]`. |
| `req-2` | État `identical`, `divergent` ou `unknown`; une absence reste inconnue. | Même test Go ciblé (code `0`) ; fixture finale avec trois commits distincts puis `installed_cli` absent. | `PASS` | `TestVersionDiagnosticDistinguishesThreeValuesAndMissingValue`; fixture : `PARTIAL`, 11 contrôles, dont « missing installed CLI remains unknown », sans erreur. Commande exacte et code de processus de cette fixture finale non consignés séparément. |
| `req-3` | Une divergence nomme les paires et l’inconnu fournit une action. | Même test Go ciblé (code `0`) ; artefact CLI public sans serveur. | `PASS` | Le test exige `installed_cli / active_server`; `T1-cli-version-real.json` contient `state: unknown`, `differences: []` et demande `--server` pour rendre `active_server` accessible. Code de sortie CLI non consigné séparément. |
| `req-4` | CLI et web consomment les sources et formats canoniques décidés. | Même test Go ciblé (code `0`) ; recette serveur réelle (code `0`) ; fixture déclarée séparément `PARTIAL`. | `PASS` | JSON CLI et runtime-health exposent le contrat commun ; la recette réelle a effectivement démarré le serveur TCP isolé du candidat. La fixture ne démontre ni serveur réel ni autonomie fournisseur. |

Le journal ciblé se termine par `PASS` et `ok swarm.local/companion 0.440s`; la note superviseur indique code `0` en `0,383s`. Cette différence de durée rapportée ne change pas le statut, mais aucune durée unique n’est revendiquée.

## APEX / PDCA checkpoint

- Analysis / PLAN : conserver l’implémentation R1 existante et remplacer le handoff interrompu par les preuves finales attribuées.
- Execution / DO : rapport uniquement ; aucune source modifiée pendant cette reprise.
- Verification / CHECK : lecture du diff, de l’interruption, du journal Go, des résultats navigateur réel/fixture et de l’artefact CLI ; aucun contrôle relancé par cet agent.
- Adjustment / ACT : recette réelle `PASS` séparée de la fixture `PARTIAL`; métadonnées absentes signalées au lieu d’être inventées.
- Recovery limits : seconde et dernière tentative ; suite globale et serveur TCP interdits dans ce sandbox ; aucune hausse de budget.

## Evidence and next action

- `docs/plans/supervision-fiable/T1-go-targeted.log`
- `docs/plans/supervision-fiable/T1-version-ui-real-final/result.json`
- `docs/plans/supervision-fiable/T1-version-ui-fixture-final/result.json`
- `docs/plans/supervision-fiable/T1-cli-version-real.json`
- `docs/plans/supervision-fiable/T1-reprise-superviseur.md`

Le moteur doit faire examiner ce même candidat et ces preuves dans un contexte indépendant avant toute acceptation publique. Le checkout étant non commité et partagé avec des changements hors T1, toute modification ultérieure du candidat invalide la fraîcheur de cette vérification ; la présence de ce rapport ne prouve ni lecture par le responsable ni acceptation.

## Complément du superviseur — dossier de revue corrigé

La première revue a demandé des observations corroborées : les chemins de journaux ne suffisaient pas. Le superviseur a ajouté le contrôle public `r1-version-recipe`, commande `python3 tests/version_acceptance.py`, avec sorties autorisées dans le dossier de revue. Cette recette reconstruit le candidat, exécute les tests Go ciblés, démarre un serveur isolé pour la recette web réelle, distingue la fixture et relève le code de sortie du CLI. Son exécution moteur et sa nouvelle revue restent à constater ; cette modification documentaire ne vaut pas validation. Attribution : superviseur, après la remise du producteur, sans troisième tentative ni changement des budgets.

## Sortie réellement capturée par le moteur — complément superviseur

Contrôle `r1-version-recipe`, code 0, du 2026-10-04T16:48:41.267511124Z au 2026-10-04T16:48:59.906256824Z. Reçu conservé : `.swarm/validation/w-a03028dc0d3f69d1f52f0ee6/plan-a03028dc0d-T1/a-116e8bdca5f90da16b18d2cd-receipt-5dd78d0965924fde810fd984.json`. Les lignes suivantes reproduisent exactement la sortie capturée, sans reformattage ni invention. Le serveur de recette est isolé ; les identités de la fixture sont simulées.

```text
COMMAND ["sh", "build.sh", "/tmp/swarm-r1-controls-cv0sw6y5/swarm"]
exit_code=0
COMMAND ["go", "test", "-v", "./...", "-run", "TestVersion|TestServerVersion|TestRuntimeHealthEndpoint", "-count=1"]
exit_code=0
=== RUN   TestVersionTwoBackupRestoresContextBeforeTrackingMigration
--- PASS: TestVersionTwoBackupRestoresContextBeforeTrackingMigration (0.28s)
=== RUN   TestServerVersionReflectsBuildAndLocalGitWithoutNetwork
--- PASS: TestServerVersionReflectsBuildAndLocalGitWithoutNetwork (0.04s)
=== RUN   TestRuntimeHealthEndpointSurvivesDatabaseFailure
--- PASS: TestRuntimeHealthEndpointSurvivesDatabaseFailure (0.05s)
=== RUN   TestVersionCLIIsStableAndDoesNotOpenStorage
--- PASS: TestVersionCLIIsStableAndDoesNotOpenStorage (0.03s)
=== RUN   TestVersionDiagnosticDistinguishesThreeValuesAndMissingValue
--- PASS: TestVersionDiagnosticDistinguishesThreeValuesAndMissingValue (0.00s)
=== RUN   TestVersionRejectsExtraArgumentsBeforeStorage
--- PASS: TestVersionRejectsExtraArgumentsBeforeStorage (0.00s)
=== RUN   TestVersionHistoryAcceptsOnlyBoundedCoherentGitHubData
--- PASS: TestVersionHistoryAcceptsOnlyBoundedCoherentGitHubData (0.00s)
=== RUN   TestServerVersionJSONSeparatesBinarySourceAndHistoryWithoutGit
--- PASS: TestServerVersionJSONSeparatesBinarySourceAndHistoryWithoutGit (0.00s)
PASS
ok      swarm.local/companion   0.456s

COMMAND ["node", "tests/version_ui.cjs", "/tmp/swarm-r1-controls-cv0sw6y5/swarm", "/tmp/swarm-r1-controls-cv0sw6y5/real"]
exit_code=0
{"status": "PASS", "mode": "real candidate server", "checks": ["cockpit fr/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit fr/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit en/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit en/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare fr/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare fr/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare en/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare en/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "loading state before delayed real runtime-health response", "explicit unavailable state on HTTP 503"], "errors": [], "failed": []}
COMMAND ["node", "tests/version_ui.cjs", "/tmp/swarm-r1-controls-cv0sw6y5/swarm", "/tmp/swarm-r1-controls-cv0sw6y5/fixture", "--fixture"]
exit_code=0
{"status": "PARTIAL", "mode": "browser fixture; server socket unavailable", "checks": ["cockpit fr/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit fr/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit en/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "cockpit en/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare fr/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare fr/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare en/etat: binary version+commit, empty history, safe link, keyboard/Escape/focus", "prepare en/sombre: binary version+commit, empty history, safe link, keyboard/Escape/focus", "loading state before delayed fixture runtime-health response", "explicit unavailable state on HTTP 503", "missing installed CLI remains unknown with actionable guidance"], "errors": [], "failed": []}
COMMAND ["/tmp/swarm-r1-controls-cv0sw6y5/swarm", "--json", "version"]
exit_code=0
{"public_cli_version": {"schema_version": 1, "binary": {"version": "devel", "commit": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed", "modified": true, "build_date": "2026-10-04T16:48:41Z", "provenance": "injected"}, "diagnostic": {"installed_cli": {"available": true, "version": "devel", "commit": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed", "modified": true, "provenance": "injected"}, "active_server": {"available": false, "version": null, "commit": null, "modified": null, "provenance": "runtime_health", "hint": "Fournissez l’URL de session du serveur actif avec --server."}, "candidate": {"available": true, "version": null, "commit": "cc3069dc7bb61b90168d21f945cb2eb5e27578ed", "modified": true, "provenance": "git"}, "state": "unknown", "differences": [], "next_step": "Rendez accessibles les identités manquantes (active_server) puis relancez le diagnostic. Pour le serveur actif, fournissez son URL de session avec --server."}}}
PASS R1 targeted controls. Fixture identities are simulated; real server was started separately. No claim of live mission autonomy.
```
