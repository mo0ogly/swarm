# T1 — correctif du superviseur, 3 octobre 2026

Le résumé avant lancement est maintenant implémenté dans le moteur, le CLI et le cockpit. Il sépare la configuration demandée/résolue de l’observation réelle, toujours inconnue avant le départ ; l’acceptation indépendante de T1 reste à réaliser.

## Identité

- Mission w-115c11e8f802a4f98c3def32 ; tâche plan-115c11e8f8-T1.
- Auteur du correctif : superviseur Codex dans cette conversation ; aucune tentative Swarm fictive créée pour ces modifications.
- Base : 1d9570bd4ef617130c6be96b7ec88844fdbcd00e ; candidat non commité. Empreintes exactes : T1-candidate.json.
- Ancien rapport T1-handoff.md conservé comme diagnostic de la première tentative ; ses constats d’absence ne décrivent plus ce candidat.

## Changements

MissionLaunchPreview transporte une identité commune et une identité par départ. Chaque départ utilise son LaunchProfile effectif, avec priorité au profil propre de la tâche. Les identités font partie du payload complet déjà signé par verdict_token : un aperçu devenu différent nécessite une nouvelle confirmation. L’aperçu reste sans mutation de révision.

Les deux interfaces affichent objectif, rôle, fournisseur, niveau demandé, modèle résolu, modèle rapporté, profil du projet, skills sélectionnés, périmètre et limites. Le champ du modèle réel n’est jamais déduit du modèle configuré : « Inconnu avant l’exécution ». Un fournisseur personnalisé sans résolution connue affiche « Inconnu ». Aucun profil/skill absent n’est inventé.

## Matrice

| Critère T1 | Preuve du candidat | Résultat |
|---|---|---|
| req-4 : résumé complet avant confirmation, web et CLI | mission_status.go : missionLaunchIdentity ; mission_cli.go : printMissionLaunchIdentity ; web/mission.js : launchIdentityView. Tests Go : configuration codex standard → gpt-5.6-sol, observation vide, skills sélectionnés ; départ override distinct du fournisseur commun fixture et révision inchangée. | PASS sur les cas contrôlés |
| req-5 : donnée inconnue explicite | Test fournisseur legacy sans modèle, CLI « Inconnu » ; cockpit legacy « Unknown », observation « Unknown before execution ». Aucun modèle réel inféré. | PASS |
| req-6 : recette FR/EN, thèmes, clavier, demandé différent de résolu | Go standard distinct de gpt-5.6-sol ; navigateur natif : auto distinct de sonnet. Cockpit sur racine /tmp/swarm-qw1-visual-joy403z4, aucun agent lancé : FR/EN, etat/sombre, sélection au clavier, fermeture Échap restituant le focus au bouton de lancement. Captures launch-fr-dark.png, launch-en-dark.png, launch-en-light-unknown.png, launch-fr-light.png dans test-results/clear-launch-recovery/. | PASS sur ce périmètre ; profils non vides contrôlés par le moteur, pas une campagne fournisseurs réelle |

## Contrôles exécutés

- go test ./... -timeout 40m : exit 0, 476,065 s, premier candidat Go avant dernière correction des traductions CLI.
- go test ./... -run 'TestMissionLaunchIdentity|TestMissionLaunchPreview|TestMissionPreview|TestI18n' -count=1 -timeout 120s : exit 0, 1,579 s, après dernière correction CLI.
- npm test : exit 0, graphes, aperçu, rafraîchissement, audit et catalogue i18n.
- go vet ./... : exit 0.
- python3 tools/agent-workflows/check.py : exit 0.
- git diff --check : exit 0.
- Console capturée du navigateur de recette : zéro erreur/avertissement. Pas de capture réseau exhaustive ; aucun succès d’appel réel Claude/Codex revendiqué.

## APEX / PDCA

Analyser : manque confirmé du transport d’identité. Planifier : compléter le contrat existant sans nouveau parcours. Exécuter : moteur + rendus + catalogue. Vérifier : tests comportementaux et DOM réel. Ajuster : textes moteur anglais repérés puis corrigés pendant la recette.

## Suite

Faire examiner ce candidat et ses empreintes. Ne pas accepter les anciennes preuves FAIL ni créer une exécution fictive. L’intervention directe est distincte de l’autonomie observée ; T2–T5 ne sont pas terminés.

## Correction après la revue indépendante à cinq appels
Le réviseur a confirmé req-5 et req-6 sur les captures et les reçus du moteur, puis refusé req-4 : objectif absent. Correction réelle : champ objective alimenté depuis Work.Objective dans les identités commune et par départ, rendus CLI et web, test moteur et test de rendu DOM. Le texte de mission saisi par l’utilisateur reste tel quel ; son contenu n’est pas traduit automatiquement.

Recette reprise sur le serveur isolé : objectif « Comprendre les agents avant de lancer » effectivement visible dans FR/EN, etat/sombre. Captures v2 sous docs/screenshots/clear-launch-recovery. Échap rend le focus à mission-primary, constat DOM réel. Aucun fournisseur exécuté pendant la recette. Go ciblé, npm test et go vet PASS après cette correction ; nouvelle suite Go complète en cours. L’ancienne revue reste dans l’historique ; aucune acceptation forcée.
