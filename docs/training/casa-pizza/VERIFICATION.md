# Vérification de la publication Casa Pizza

Vérification du 8 octobre 2026, réalisée dans la session du formateur.
Base du dépôt Swarm : `7cbec49`. Périmètre : ce dossier de formation et les
liens ajoutés aux deux README principaux. Aucun changement du moteur Swarm.

| Contrôle | Exécution ou inspection | Résultat |
| --- | --- | --- |
| Guides français et anglais | Rendu avec le compilateur LibreOffice fourni par l'environnement, inspection des 36 pages de chaque version | PASS |
| Application contenue dans le kit français | ZIP extrait dans un dossier temporaire, `python3 -W error::ResourceWarning -m unittest -v` | PASS, 11 tests, 4,691 s |
| Application contenue dans le kit anglais | Même commande dans un autre dossier temporaire | PASS, 11 tests, 4,447 s |
| Création du projet et plan optionnel | Deux scripts exécutés dans un projet temporaire via le binaire Swarm installé | PASS, Git et besoin créés, aucun agent lancé |
| Confidentialité du paquet | Liste explicite, intégrité ZIP, inspection des textes et XML Word | PASS, aucune clé ou configuration interne publiée |
| Liens de documentation | Résolution des liens locaux des trois README | PASS |
| Contrat des méthodes | `python3 tools/agent-workflows/check.py` | PASS |
| Propreté du diff | `git diff --check` | PASS |

La recette navigateur de la référence courante est décrite dans
[REFONTE.md](projet-pizza/docs/REFONTE.md). L'ancienne recette et la remise de
l'exécutant restent conservées et sont explicitement historiques.

Cette vérification n'est pas une revue indépendante. Les formulaires de
validation automatique du guide ont été prévisualisés puis annulés. Le plan
à six tâches n'a pas été exécuté. Aucun résultat autonome, paiement réel,
livraison réelle ou déploiement du site n'est revendiqué.

## Complément du 9 octobre 2026 — tutoriels et accès UX

- Deux guides PDF de 38 pages, issus des Word étendus sans retrait des 36 chapitres initiaux. Pages rendues et inspectées ; les 38 flux de contenu de chaque PDF sont identiques au rendu inspecté. Cinq liens anglais corrigés après export ; téléchargements réels des deux PDF vérifiés. Les Word restent les sources éditables.
- 27 captures réelles, quatre modules bilingues, huit MP4 et huit GIF. MP4 décodés intégralement, durée 24/24/30/42 secondes et format 1280×900 vérifiés. Empreintes des captures et médias contrôlées. Ce sont des séquences de captures fixes légendées, pas des enregistrements continus.
- Commande fictive CP-75C24F0C : minimum refusé, quantité corrigée, accès restaurant refusé puis accepté, transitions manuelles jusqu'à livrée. Application isolée : 11 tests HTTP/reprise PASS en 6,777 s. Aucun appel fournisseur durant la démonstration.
- Besoin enregistré ; plan pédagogique créé séparément par script public avec six tâches non exécutées. Politique de contrôle prévisualisée puis annulée, aucune acceptation.
- Deux ZIP de 125 fichiers chacun, intégrité et présence des deux PDF vérifiées ; aucune base ou configuration de session embarquée.
- Lecteur : navigation, clavier, exercice corrigé, français/anglais, thèmes sombre/clair et lecture MP4 vérifiés dans le navigateur. Aucun autoplay ni dépendance distante.
- Accès embarqué dans Swarm via Aide du cockpit → Formation Casa Pizza, vérifié réellement sur le cockpit isolé 18844 ; ressources protégées par l'authentification habituelle et CSP inchangée. Test Go ciblé TestWebTrainingAssetsAndAuthentication PASS (1,219 s), couvre aussi PDF/ZIP/vidéo Range et fichiers exclus. npm test, go vet ./..., contrôle des méthodes et git diff --check PASS.
- Suite Go complète lancée : FAIL, dont TestEngineContractRevisionKillsInferenceOnContractMutation (sous-processus de revue non démarré), TestGraphDeliveryD03QualificationManifest (empreinte historique de web_server.go périmée), puis timeout global 600 s pendant TestRequalifyExplicitStalePassedReview/reboot. Aucun ancien manifeste accepté réécrit ni succès global revendiqué.
- Le candidat de test est installé sur 18844 seulement. Le cockpit principal 18792 et ses missions restent inchangés ; entrée de formation non encore installée sur ce serveur. Aucun commit/push.

Cette vérification est une inspection IA et des contrôles locaux, pas une revue humaine indépendante ni une preuve d'autonomie fournisseur.

### Sélection de langue, complément du 9 octobre

Cockpit anglais → lien de formation ?lang=en vérifié dans la vraie modale. Lecteur : vidéo/GIF/transcription anglais, PDF anglais et kit anglais sélectionnés ; téléchargement PDF réel PASS. Passage au français puis rechargement : ?lang=fr conservé et tous les liens français corrects. Indication explicite « Langue des ressources » avec aria-live. Captures du panneau anglais clair et français sombre inspectées ; aucune erreur console. npm test et git diff --check PASS. Deux kits régénérés (125 fichiers chacun) ; candidat embarqué reconstruit et installé uniquement sur le cockpit isolé 18844. Les échecs globaux précédents et le statut inchangé du serveur principal 18792 restent valables.

### Logo et favicon, complément du 9 octobre

Nouvelle marque PNG transparente inspirée de docs/swarm.jpeg (original conservé), générée via ImageGen : lampe, engrenage et graphe. Intégrée au cockpit, à la préparation et au lecteur ; favicon PNG déclarée dans leurs en-têtes. Asset chargé réellement dans le navigateur (1254 pixels), thème sombre/clair et parcours FR/EN inspectés ; navigation vers formation conservée, aucune erreur console de préparation. CSS nouveau utilise seulement des jetons existants définis dans les deux thèmes, sans couleur littérale. Kits régénérés à 126 fichiers et intégrité/logo exact vérifiés. npm test, test ciblé TestWebTrainingAssetsAndAuthentication (0,532 s), contrôle méthodes, build candidat et diff check PASS. Première commande de test/build interrompue par SIGTERM (143), cause non établie ; reprise limitée avec GOMAXPROCS=2, GOGC=50 et répertoire temporaire disque, puis succès. Aucun test global relancé, anciens échecs conservés. Logo installé dans le candidat isolé 18844 seulement, pas serveur principal 18792. Les PDF et les médias de démonstration préexistants ne sont pas régénérés par cette modification de marque UX.

## README, installation et visibilité du graphe — 9 octobre 2026

README FR/EN : logo, aperçu GIF du graphe dès le début avec MP4 et image fixe, guides PDF38 pages, lecteur et ressources selon la langue. INSTALL FR/EN documentent les ressources embarquées et la reconstruction du binaire. Le graphe et ses commandes précèdent maintenant le résumé détaillé, via l’ordre DOM de pilotage.js ; aucune information de validation n’est supprimée. Build canonique et npm test PASS ; contrôle navigateur FR/EN, sombre/État, navigation clavier du graphe et ordre DOM vérifiés sur18844. Pas de nouvelle qualification globale ni de remplacement du serveur18792 : les limites Go précédemment consignées restent ouvertes.

## Intégration de la présentation V2 — 9 octobre 2026

- Présentation française ajoutée au lecteur, masquée et mise en pause lors du passage à l’anglais ; les liens vidéo/PDF/ZIP des modules suivent toujours la langue.
- Lecture au clavier réelle dans le navigateur : durée 63 s, readyState 4, aucun message d’erreur média, une piste de sous-titres. Contrôles DOM FR/EN et captures des deux thèmes dans `/home/fpizzi/workspace/rapports-banc/swarm-training-v2-integration`. Aucun autoplay.
- `go test ./internal/engine -run '^TestWebTrainingAssetsAndAuthentication$' -count=1` PASS, 0.159 s. Les nouveaux médias sont présents dans l’embed authentifié.
- Kits reconstruits : 129 fichiers chacun, contrôle ZIP intègre. `node --check` et `git diff --check` PASS.
- Recette du lecteur menée sur serveur statique loopback isolé ; service des ressources authentifiées couvert séparément par le test Go. Le serveur principal 18792 n’a pas été reconstruit ni redémarré : intégration source validée, déploiement non effectué.

## Actualisation graphique du 10 octobre 2026

Les tutoriels Swarm sont recapturés avec la nouvelle interface, puis les quatre
MP4 FR/EN et la présentation du graphe sont reconstruits avec leurs posters et GIF.
Les médias client/restaurant du 9 octobre restent inchangés. Les dates sont
explicites ; les illustrations PDF/DOCX restent historiques. Les deux kits
ZIP contiennent les nouveaux médias et 133 fichiers chacun.

La recette `tests/training_design_ui.cjs` passe sur le binaire SHA-256
`35dc4e13b70f8eda87eb175451c53e1e40b8737b775c19f68450fc7ebee0055a`
et les deux kits extraits : FR/EN, lecture/seek, durées, aucune lecture automatique,
thèmes et mobile. Les neuf vidéos passent le décodage intégral ffmpeg.
Preuves dans `docs/screenshots/ui-redesign-20261010/training-*-result.json` ;
méthode et limites dans `docs/reports/ui-redesign-20261010.md`.
