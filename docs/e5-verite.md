# E5 — Même vérité dans le web et le CLI

## Résultat en deux phrases
La projection commune expose les contrôles exécutés même lorsque la revue refuse le candidat, et le CLI comme la fenêtre de vérification montrent leurs identités, SHA, commandes, dates, codes et limites. Cette livraison est une réparation externe demandée par l’utilisateur après interruption à 100 outils ; elle ne démontre pas une exécution autonome réussie et attend la décision du moteur et la revue indépendante.

## Identité et périmètre
- Mission w-f923632399eea4d404bc94ac ; tâche e5-verite ; tentative a-c1faf94c71267b4728ab3c4b ; producteur initial auto-3e696db07838ce942d96, interrompu.
- Réparation externe Codex dans la copie attribuée e5-verite-4, sans cinquième tentative ni modification des plafonds.
- Base b9c85ac8a746e8bfa579a369084bbdd6c1e1ddf4. SHA candidat déterminé par le moteur après remise ; les reçus portent cette identité immuable.
- Fichiers : mission_cli.go, evidence_projection.go, web/planning.js, web/evidence-contract.js, web/index.html, web/mission.css, dictionnaires FR/EN, engine_contract_truth_test.go.
- Aucun fichier de R5 ni d’une autre copie n’a été modifié par cette réparation. Aucun import de l’historique d’une autre mission, aucune réécriture de sa base.

## Constat pour le responsable
L’exécutant avait confondu racine hôte et copie, puis atteint le budget avant les tests et le rapport. Les trois modifications qu’il avait conservées ont été complétées. Le moteur de maintenance reçoit séparément le cadrage d’une seule racine, le bilan d’arrêt et la remise externe explicite ; ce candidat E5 reste limité à la vérité des preuves CLI/web. Les rôles responsable/exécutants/vérificateur et la distinction processus terminé/accepté existaient déjà : ils sont vérifiés, sans inventer une modification pour satisfaire un libellé.

## Changements et vérification
| Obligation | Vérification / effet observé | Résultat |
|---|---|---|
| Responsable, exécutants, vérificateur réel | Cockpit isolé : organization-fixture, 2 tâches, managed-review-fixture distinct ; CLI conserve l’organisation et affiche producteur/reviewer pour la revue | PASS |
| SHA, commandes, dates, codes, décision, limites | TestEngineContractTruthReviewedReceiptWebCLI, vrai Git/contrôle git diff --exit-code et fournisseur fixture indépendant refusant ; mêmes valeurs CLI FR/EN et snapshot ; reçu altéré devient unknown | PASS |
| Terminé distinct d’accepté, rapport inconnu | TestEngineContractTruthProcessExitIsNotAcceptance : completed_unproven, absent_or_unattributed, pas accepted | PASS |
| Revue configurée non présentée absente | TestEngineContractTruthConfiguredReviewerIsPending : pending, identité connue ; limites corrigées pour les sources/contrôles du candidat géré | PASS |
| FR/EN, codes invariants | Libellés moteur et preuves traduits ; titres, prose fournisseur, identifiants et codes métier conservés ; lectures sans mutation du travail | PASS |
| Aide/reprise claire | Modale de revue avec aide existante et fermeture clavier ; refus reste corrections demandées ; aucune action de succès implicite | PASS |
| Navigateur clair/sombre, mobile | Recette CUA 390x844 et bureau FR/EN ; corrections de débordement, preuves dans docs/e5-verite.browser.json ; Échap et focus retour vérifiés | PASS |
| Jetons | Surface/encre/trait --wattson-carte-appuyee/texte/ligne ; aucune couleur littérale ajoutée ; rendu clair et sombre observé | PASS |
| R5 préservé | Éditions confinées à la copie E5 ; aucune commande de modification visant R5 | PASS |

Commandes observées, code 0 : `go test ./...` (53.735 s), `go test -count=1 -run '^TestEngineContractTruth' .`, `node tests/engine_acceptance.cjs --case truth` (3 tests comportementaux), `npm test`, `go vet ./...`, `git diff --check`. TMPDIR/GOTMPDIR=/dev/shm pour les commandes Go. Journaux locaux conservés dans e5-resolution-20260921 du répertoire de maintenance ; preuves source reproductibles dans le candidat. Le moteur doit rejouer le contrôle autorisé sur sa révision candidate.

## APEX / PDCA
PLAN : couvrir toutes les obligations U1, pas uniquement la traduction. DO : compléter la projection vérifiée, le CLI et la fenêtre de revue. CHECK : tests négatifs, reçus altérés, FR/EN et navigateur. ACT : corriger le débordement mobile et les libellés manquants détectés pendant la recette.

## Limites et prochaine action
Les fournisseurs de la recette sont des fixtures : aucune certification Cursor, aucun succès réel d’autonomie revendiqué. La prose originale du vérificateur et les titres utilisateur ne sont pas traduits. unknown/stale restent des codes explicites et ne prouvent aucun succès. Les captures sont conservées hors dépôt ; leur manifeste et leurs empreintes sont suivis. Le vérificateur réel examine cette réparation et les contrôles du même candidat ; son refus éventuel doit être conservé. Les autres tâches bloquées restent inchangées.
