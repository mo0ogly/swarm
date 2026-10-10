# Résultat — identité et ergonomie de Swarm

## Résultat et état de vérification

Le cockpit, la préparation, l’accès et les sessions d’agents partagent désormais une
identité issue du logo : rail marine, doré, cyan et surfaces claires couleur papier.
La navigation est regroupée, les lectures secondaires se déplient et la synthèse de
mission précède le graphe. Les parcours navigateur passent. La vérification de performance reste partielle :
le chargement à 200 tâches atteint 1 150 ms pour une cible de 1 000 ms ; les tailles
50 et 500 et la réponse clavier respectent leurs seuils. Suite Go exhaustive découpée : sortie 0, 1071 tests passés et 8 skips optionnels explicites sur 1079 découverts.

## Identité et périmètre

- Session native Codex, rôle d’implémentation autorisé par l’utilisateur. Aucun
  identifiant de mission Swarm ni verdict d’acceptation moteur n’est attribué à ce travail.
- Dépôt : `/home/fpizzi/workspace/swarm`, branche `codex/clear-product-method-names`.
- Base : `36f884b8131a7d8e1b3c1c270a35f3f5b7a5c873` ; modifications non commitées.
- Candidat UI : empreinte `e62a0cc4460b44bfcd1937484d30ee35f67d74ea5b225c7bbf930b55992bc83a`.
  [Manifeste des sources et du binaire](../screenshots/ui-redesign-20261010/candidate.json).
- Les changements préexistants de benchmark, facturation, publication et reprise
  sont conservés hors du périmètre. Aucun serveur utilisateur n’est arrêté ; aucune
  base de mission utilisateur n’est utilisée comme fixture.
- Binaire candidat : `/tmp/swarm-ui-20261010/swarm`. `bin/swarm` est aussi reconstruit.
  Les processus déjà lancés continuent à servir leur ancien binaire.

## Choix et livraison

La refonte conserve les composants métier et réaffecte leurs jetons sémantiques
avec une feuille partagée, chargée après les styles spécifiques. Cette solution
maintient les identifiants de thème et les préférences. Une réécriture complète des
vues aurait augmenté le risque sur les formulaires, le graphe et la préparation.
Les prochaines évolutions doivent modifier le système commun plutôt qu’empiler
une nouvelle feuille de surcharge.

La navigation contient cinq groupes natifs : Missions, Préparation, Conduite,
Suivi, Configuration. Les douze destinations restent accessibles en mode simplifié.
Le groupe actif s’ouvre sur navigation ou lien direct, les choix de lecture sont
mémorisés localement, et une décision en attente reste accessible quand son groupe
est fermé. Les commandes secondaires, consignes, contexte IA et détails de mission
sont dépliables. Les blocages et actions principales restent visibles.

Le graphe conserve le placement dagre et arrondit les connexions ; la mini-carte
utilise les mêmes chemins. Clavier, focus, menu mobile, lien d’évitement et réduction
des animations sont intégrés. Le logo reste le fichier original du projet.

Deux corrections fonctionnelles découvertes dans les parcours : l’ajout d’une
connexion attend ses données au lieu d’ignorer un clic précoce ; l’arrêt d’un agent
inclut la tâche attribuée dans le snapshot courant. L’arrêt confirmé ne vaut jamais
acceptation du résultat. Le logo et le CSS de connexion sont servis avant
l’authentification, avec les mêmes vérifications d’hôte ; les API et scripts métier
restent privés.

## Exigences et preuves

Chrome local, fixtures temporaires via CLI/API, binaire réellement servi. Les tests
avec fournisseurs utilisent uniquement des programmes locaux de recette ; aucun
appel IA payant ni autonomie réelle n’est revendiqué.

| Exigence | Effet observé | Vérification et entrées | Résultat | Preuve / limite |
| --- | --- | --- | --- | --- |
| UI-01 Identité | Palette du logo et composition commune ; clair/sombre | `node tests/design_system_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/final-design-v4`, sortie 0 | PASS | Captures de quatre surfaces ; inspection visuelle |
| UI-02 Navigation | 12 destinations, groupe actif, lien direct, décisions accessibles | Même recette, vrai serveur et snapshot de décision injecté au GET | PASS | `design-result.json` ; la décision injectée n’est pas un résultat moteur |
| UI-03 Lecture | Accordéons clavier, ouverture conservée, besoin enregistré conservé | Même recette ; création réelle d’une préparation sans demande IA | PASS | Persistance après rechargement, document et garde du brouillon |
| UI-04 Accessibilité et langues | FR/EN, 320–1440 px, focus rendu, pas de débordement global | Même recette + `node tests/i18n_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/final-i18n`, sortie 0 | PASS | Paires sémantiques texte/fond ≥ 4,5 ; aucune certification WCAG complète revendiquée |
| UI-05 Produit | Recherche, pagination, navigation, persistance et clavier | `node tests/product_views_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/final-product`, sortie 0 | PASS | FR/EN et les deux thèmes |
| UI-06 Chargement et erreurs | Action désactivée pendant lecture, formulaire ensuite actif ; rafraîchissement échoué récupérable | Recette design avec délai réseau et échec de fetch ; `node tests/ai_connections_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/final-connections`, sortie 0 | PASS | Aucun lancement IA ; les doubles vérifient le comportement réel des contrôles |
| UI-07 Session | Vrai PTY, saisie, pas de retransmission après réponse perdue, reprise, arrêt confirmé | `node tests/agent_terminal_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/terminal-v4`, sortie 0 | PASS | `terminal-result.json`, programme Python local ; aucune acceptation automatique |
| UI-08 Accès | GET/HEAD logo/CSS publics, mauvais hôte refusé, API privées | `go test ./internal/engine -run 'TestWeb(LoginDesignAssets|StableAddressLogin|AuthenticationOrigin|Prephase|Training)' -count=1`, sortie 0 ; recette login incognito | PASS | Régression 403 reproduite puis corrigée ; FR/EN, thèmes et 320 px |
| UI-09 Graphe | Édition, annulation, conflit, focus et sélection FR/EN/thèmes passent ; charge 200 hors cible | `node tests/graph_draft_ui.cjs /tmp/swarm-ui-20261010/swarm /tmp/swarm-ui-20261010/final-graph-v2`, sortie 1 | PARTIAL | `graph-result.json` conserve les quatre parcours et toutes les mesures ; seuil de 200 tâches dépassé |
| UI-10 Documentation | Direction durable et consigne pour futurs changements | `docs/UI-DESIGN.md`, référence dans `AGENTS.md` | PASS | Cette référence et les captures sont versionnables |

Vérifications statiques : `npm test`, `python3 tools/agent-workflows/check.py`,
`go vet ./...`, `git diff --check` : sortie 0. Compilation :
`npm run build:terminal --prefix frontend`,
`go build -o /tmp/swarm-ui-20261010/swarm ./cmd/swarm` : sortie 0.
Le binaire final est copié vers `bin/swarm` ; les deux fichiers ont la même empreinte
SHA-256 `a0a6fda97a64ccfa0c4e2c5020671dd7ca2d2e0f87e91e8c9a72398c8d2a5ff1`.
Le catalogue FR/EN généré reste synchronisé. `python3 tools/check_distribution.py`
sort également avec le code 0.

`go test ./...` a retourné 1 : ancien instantané périmé sur `AGENTS.md`, puis délai
global de dix minutes atteint. Le nouvel instantané `docs/ui-redesign-source-manifest.json`
conserve les anciens manifestes et leur statut de requalification en attente. Il
inclut les sources héritées observées, y compris les changements préexistants de
l’utilisateur, sans en revendiquer la production. Le test
`go test ./internal/engine -run 'Test(GraphDeliveryD03QualificationManifest|WebLoginDesignAssetsRemainPublicAndAPIPrivate)' -count=1`
sort ensuite avec le code 0 (0,244 s). La recette exhaustive de CI,
`python3 tests/supervision_go_suite.py`, est lancée avec ses limites et son
inventaire complet inchangés : sortie 0, 19 groupes, 1079 tests découverts,
1071 passés et 8 skips optionnels explicites. Le log local est
`/tmp/swarm-ui-20261010/go-suite.log`. Les skips n’incluent pas les critères
obligatoires de graphe, performance moteur ou cache de revue. Les recettes
navigateur exécutées séparément ne sont pas déduites de ces skips.

Recettes complémentaires exécutées pendant l’implémentation :
`tests/mission_guidance_ui.cjs` et `tests/modes_ui.cjs` : sortie 0.
Les résumés web/CLI restent égaux, les explications de changements, attentes,
dépenses et reprise restent accessibles ; le mode conserve les douze destinations.
La fixture de guidance lance un travailleur déterministe local, sans modèle payant.

## Échecs diagnostiqués et corrections

- Le passage de lignes SVG à des chemins nécessitait la même adaptation de la
  mini-carte ; l’erreur a été corrigée et les recettes réexécutées.
- Les recettes ouvraient auparavant des contrôles toujours visibles. Elles ouvrent
  désormais leurs accordéons avant les clics, sans retirer les assertions métier.
- Un tableau de connexions héritait de la largeur minimale des tableaux de tâches :
  règle corrigée sur cette surface, vérification à 320 px.
- Avant authentification, le CSS et le logo étaient refusés ; test Go de régression
  avant/après. Une icône explicite utilise ce logo public.
- La recette de terminal reproduisait un refus réel « tâche et identifiant de
  commande requis ». La requête d’arrêt est corrigée ; même parcours complet vert.
- Les doubles réseau de la recette design conservent les écouteurs de suivi des
  requêtes. Seuls les abandons déjà en vol lors d’une navigation sont classés attendus.

La première charge de 200 tâches dépassait le seuil historique de 1 000 ms
(p95 1 046 ms), avec d’autres vérifications actives. Le binaire de référence avant
refonte dépassait aussi ce seuil dans cet environnement (p95 1 094 ms).
Un autre essai a dépassé le seuil à 50 tâches (1 045 ms). Le suivi des accordéons
parcourait tout le document à chaque mutation : il est limité aux éléments HTML
ajoutés. Les parcours de conception et de terminal repassent après cette correction.
La recette de charge conserve toutes les tailles avant de retourner son échec,
sans changer les assertions ni les seuils. Mesure finale sans nos autres suites :

| Tâches | Chargement p95 | Cible | Clavier p95 | Cible clavier | Résultat |
| --- | --- | --- | --- | --- | --- |
| 50 | 805 ms | ≤ 1 000 ms | 32 ms | ≤ 100 ms | PASS |
| 200 | 1 150 ms | ≤ 1 000 ms | 32,7 ms | ≤ 100 ms | FAIL chargement |
| 500 | 1 909 ms | ≤ 2 000 ms | 32 ms | ≤ 100 ms | PASS |

Cinq échantillons par taille : le « p95 » de cette recette est le maximum des cinq.
La machine exécute aussi d’autres processus utilisateur, qui ne sont pas arrêtés.
Cette variation n’est pas démontrée comme une régression ou une cause exclusivement
externe. Une qualification sur machine contrôlée et le profilage du chargement à
200 tâches restent nécessaires. Les seuils et limites moteur ne sont pas modifiés.

## APEX / PDCA et revue

Analyse : partir du logo réel, conserver les parcours métier et établir le rapport
entre checkout, binaire et serveur. Plan : système commun, navigation, lecture,
parcours, vérification et documentation. Exécution : modifications bornées au web,
aux ressources de connexion et aux recettes correspondantes. Vérification : preuves
ci-dessus. Ajustements : corriger les erreurs observées, puis vérifier leur effet.

Auto-relecture de l’auteur avec `code-reviewer`, dans la même session. Aucune revue
indépendante ni acceptation par le moteur n’est revendiquée. Aucune synchronisation
moteur n’est modifiée ; test de race non applicable à ces changements.

## Suite et limites

La [référence de conception](../UI-DESIGN.md) est à lire avant toute évolution
visuelle. Les [captures](../screenshots/ui-redesign-20261010/README.md) sont issues du
produit servi, sur une mission de démonstration. L’aperçu local utilise cette fixture.
Le serveur utilisateur doit être relancé avec le binaire reconstruit pour appliquer
les ressources embarquées ; cette session n’a pas interrompu ses processus.

La refonte est implémentée ; la qualification de performance à 200 tâches reste
partielle. Pas de commit, push ou déploiement. Les appels réels aux fournisseurs, la réception
par des utilisateurs, Safari et Firefox ne sont pas démontrés par les tests Chrome.

## Suite : rail repliable et chargement des résultats

Demande suivante du 10 octobre : replier toute la navigation de gauche pour
donner davantage de place au graphe. Cette suite conserve les preuves et mesures
précédentes ; son candidat est distinct (`candidate-menu.json`). Base Git identique,
sources non commitées. Binaire de cette étape `/tmp/swarm-ui-20261010/swarm-next`,
copié alors dans `bin/swarm`, SHA-256
`241e4754cb993cf3901be2750ef4f3301f5a35f1c60d3da42ad31270cc497fe7`.
Empreinte des 19 sources UI :
`cca7cea896320b8314ce535dfbb9d4893672558893a7060491a7e7d7a70bfbf7`.

Le bouton dans le contenu replie le rail complet sur ordinateur. Le contenu
récupère sa largeur (248 px à 1440 px), sans remonter le graphe ni changer son
zoom, sa sélection ou la révision du travail. Le bouton de réouverture reste
accessible au défilement ; le choix survit au rechargement. Le menu mobile est
indépendant de ce choix. Les groupes internes conservent leurs accordéons.

Diagnostic du chargement à 200 tâches : le profil instrumenté initial montre
50,7 ms dans `Mission.render`, 130,8 ms dans le placement dagre, 276,8 ms dans
le dessin du graphe et 340,6 ms dans le rendu total du cockpit. La liste complète
des résultats était construite dans un accordéon fermé. Elle est maintenant
construite à l’ouverture ; les interventions prioritaires restent hors de cette
liste. Une actualisation ouverte conserve les sous-accordéons et le focus ;
une actualisation fermée diffère la construction et garde les choix pour la
réouverture. Les deux lectures initiales GET session/missions sont parallèles ;
les commandes continuent à attendre le jeton CSRF. Aucun traitement moteur ni
seuil de qualification n’est modifié.

Le profil instrumenté après correction observe 19,9 ms dans `Mission.render`.
Son temps total est supérieur au premier profil, et ce profil chevauche la
recette de conception : il sert au diagnostic, pas à une qualification ou à
une affirmation de gain global. Les mesures de la recette B03 normale sont
rapportées séparément ci-dessous.

| Critère | Observable | Vérification | Résultat |
| --- | --- | --- | --- |
| UI-11 | Rail masqué, graphe élargi de plus de 200 px, réouverture clavier, choix mémorisé, mobile accessible | `next-design-v2` : FR/EN, clair/sombre, 320 à 1440 px | PASS |
| UI-12 | Résultats absents du DOM fermé, complets à l’ouverture, sous-accordéons et focus conservés après actualisation | `next-design-v2` : quatre tâches, ouvert/fermé/réouvert, deux actualisations fermées | PASS |
| UI-13 | Parcours métier, diagnostic et actions restent disponibles | `next-guidance` : web/CLI, clavier, thèmes, mobile, fournisseur Python déterministe local | PASS |

Commandes sur ce candidat, toutes sorties 0 :

```sh
go build -o /tmp/swarm-ui-20261010/swarm-next ./cmd/swarm
node tests/design_system_ui.cjs /tmp/swarm-ui-20261010/swarm-next /tmp/swarm-ui-20261010/next-design-v2
node tests/mission_guidance_ui.cjs /tmp/swarm-ui-20261010/swarm-next /tmp/swarm-ui-20261010/next-guidance
npm test
python3 tools/agent-workflows/check.py
git diff --check
```

Les changements de cette suite sont JavaScript/CSS/HTML, catalogue et documentation.
La suite Go complète de la refonte ci-dessus n’est pas revendiquée comme rejouée
sur cette suite. Auto-relecture de l’auteur ; aucune revue indépendante, acceptation
moteur, publication ou commande IA payante. Seul le serveur de démonstration
possédé par cette session a été relancé, sur le même port 44977 et la même fixture.
Les autres serveurs utilisateur restent intacts.

### Placement mémorisé dans l’onglet

La recette sans cache (`next-graph`) a encore retourné 1 : p95 de rechargement
740 / 1 093 / 1 734 ms pour 50 / 200 / 500 tâches ; clavier 32,3 / 35,1 / 48,6 ms.
Les quatre variantes fonctionnelles passent, mais le seuil à 200 reste dépassé.
Cette preuve est conservée dans `next-graph-before-cache.json`.

Le candidat suivant (`candidate-next.json`) ajoute un cache de géométrie par
onglet, limité à une entrée. La signature couvre dagre, ses paramètres, les
dimensions et la forme affichée. Le cache ne contient ni états ni autorisations :
les textes, couleurs, coûts et commandes restent issus du snapshot courant.
Une entrée malformée ou un stockage refusé/plein provoque un placement normal.
La recette unitaire utilise le vrai dagre : mêmes coordonnées et mêmes routes,
invalidation des dépendances/orientation/dimensions, corruption et stockage refusé.
Les rechargements mesurés peuvent ainsi réutiliser la géométrie ; la recette
enregistre désormais séparément le premier affichage sans cache, sans changer
les cinq échantillons historiques ni leurs seuils.

La première recette de conception avec cache (`next-design-v3`) s’est interrompue
sur une erreur du harnais Puppeteer : son intercepteur appelait `continue` sur
une requête reçue alors que l’interception était désactivée. Le harnais attend
maintenant les continuations, retire son écouteur avant de désactiver l’interception
et ne traite que les requêtes interceptables. Aucun diagnostic du produit n’est
masqué. La recette corrigée est relancée sur le même binaire.

`next-design-v4` retourne 0 : repli/réouverture, largeur réelle du graphe,
clavier, préférence mémorisée, mobile indépendant, résultats différés,
sous-accordéons/focus après actualisation, routes identiques après rechargement,
cache corrompu recalculé, FR/EN, clair/sombre, erreurs réseau et reprise.
Preuve `next-design-final-result.json`. `npm test` retourne 0 et inclut maintenant
`tests/graph_layout_cache_test.cjs`.
Le candidat final possède l’empreinte UI
`6864ed0afbaaa71bb3bfc56f2b98262fe4905be40dccf1049027502f77e3f52e`
et le binaire SHA-256
`14e3e24825334673c868371a8abf94911332bb870a98a5ffcf6228908ae6a75d`.

La recette complète `next-graph-v2` retourne 0, avec les quatre variantes
FR/EN et sombre/clair. Édition clavier/souris, prévisualisation, application,
conflit concurrent, undo/redo, focus, flèches, viewport et fraîcheur des états
passent. Aucun départ implicite. Preuve `next-graph-result.json`.

| Tâches | p95 rechargement, cinq mesures | Seuil conservé | p95 clavier | Premier affichage, une observation |
| --- | ---: | ---: | ---: | ---: |
| 50 | 628 ms | 1 000 ms | 33,7 ms | 750 ms |
| 200 | 874 ms | 1 000 ms | 32,5 ms | 1 230 ms |
| 500 | 1 262 ms | 2 000 ms | 32,0 ms | 2 216 ms |

Les seuils historiques de la recette passent maintenant sur les rechargements
du même onglet. Le premier affichage inclut la redirection du lien de session
et un placement sans cache. Ses observations à 200/500 sont supérieures aux
seuils de rechargement ; elles ne constituent pas une qualification p95 à froid.
Cette limite reste explicite : le cache améliore les rechargements et ne démontre
pas un premier affichage sous une seconde à 200 tâches.

Commande finale :
`node tests/graph_draft_ui.cjs /tmp/swarm-ui-20261010/swarm-next /tmp/swarm-ui-20261010/next-graph-v2`.

### Formation Casa Pizza : remplacement des anciennes captures bleues

Le bleu vif subsistait dans les fichiers de captures et vidéos enregistrées, qui
n’héritent pas de la nouvelle CSS. Le 10 octobre, les quinze captures Swarm FR/EN
ont été refaites par les interactions réelles dans un atelier CLI/API jetable,
avec quatre vues supplémentaires pour la présentation du graphe. Le besoin
enregistré reste séparé du plan pédagogique ; six tâches non exécutées, aucune
politique appliquée, aucun agent lancé. `training-capture-result.json` porte
l’empreinte du binaire de capture (candidat avec cache).

Les premiers essais ont révélé deux défauts du harnais : l’inspecteur encore
ouvert interceptait l’accès au formulaire, puis les coordonnées du cadrage de
dialogue ignoraient le défilement de la page. Les essais échoués v2/v3 sont
conservés ; fermeture explicite de l’inspecteur et cadrage tenant compte du
défilement corrigent les causes. La capture v4 passe, avec vérification de la
visibilité réelle du programme python3. Les images ont été inspectées.

Quatre MP4 Swarm (deux modules × deux langues), leurs GIF/posters, et la
présentation française de 63 secondes sont reconstruits. La présentation est
maintenant en 1280 × 900 ; ses légendes décrivent les captures actuelles. Les
médias client et restaurant sont identiques octet pour octet à leur enregistrement
du 9 octobre. Le lecteur et le storyboard indiquent les dates distinctes. Les
captures PDF/DOCX historiques ne sont pas prétendues recapturées.

Les deux kits contiennent 133 fichiers et passent leur contrôle ZIP. Toutes les
empreintes du manifeste correspondent aux ressources ; neuf MP4 sont décodés
intégralement avec ffmpeg (`training-media-result.json`). La recette navigateur
`tests/training_design_ui.cjs` passe sur le binaire final et les deux kits extraits :
FR/EN, quatre modules, dimensions/durées, lecture et seek réels, aucun autoplay,
introduction française, thèmes clair/sombre et largeur 320 px. Aucun diagnostic
JS/HTTP inattendu ; seules les annulations de lecture média par changement de
module sont reconnues comme attendues. Preuve `training-reader-result.json`.

Binaire livré : `/tmp/swarm-ui-20261010/swarm-training`, SHA-256
`35dc4e13b70f8eda87eb175451c53e1e40b8737b775c19f68450fc7ebee0055a`,
copié dans `bin/swarm`. Les dix-neuf sources UI conservent leur empreinte
`6864ed0afbaaa71bb3bfc56f2b98262fe4905be40dccf1049027502f77e3f52e`.
`candidate-graph-cache.json` conserve le candidat des recettes graphe ;
`candidate-next.json` ajoute les empreintes des nouveaux médias au binaire livré.
La recette graphe précédente porte sur le binaire avec cache avant changement
des médias ; elle n’est pas présentée comme exécutée sur ce nouveau binaire.

Seul le serveur d’aperçu de cette tâche (atelier `/tmp/swarm-design-fo2UD5`,
port 44977) a été renouvelé. La vue du menu replié et le lecteur avec le nouveau
graphe sont vérifiés dans le navigateur de l’application et enregistrés dans
`preview-final-collapsed.png` et `preview-final-training.png`. Les autres
serveurs et missions restent hors de cette intervention.

Commandes :
- `node tools/verification/training_ui_capture.cjs /tmp/swarm-ui-20261010/swarm-next /tmp/swarm-ui-20261010/training-captures-v4`
- `python3 docs/training/casa-pizza/tutoriels/build_graph_overview.py`
- `python3 docs/training/casa-pizza/tutoriels/build_media.py --modules 01-plan 02-controles --swarm-recorded 2026-10-10`
- `python3 docs/training/casa-pizza/tutoriels/build_player.py`
- `python3 docs/training/casa-pizza/build_kits.py`
- `node tests/training_design_ui.cjs /tmp/swarm-ui-20261010/swarm-training /tmp/swarm-ui-20261010/training-reader`

Le premier contrôle d’intégrité final a correctement rejeté le snapshot :
`preview-final-training.png` venait d’être recapturé après sa génération.
`final-source-check-before-capture-failure.json` conserve cet échec.
Le contrôle est relancé après stabilisation des captures et rafraîchissement
du manifeste, sans modification des seuils ou de l’acceptation historique.

Contrôle final corrigé : `TestWebTrainingAssetsAndAuthentication` et
`TestGraphDeliveryD03QualificationManifest` passent (0,367 s). `npm test`,
`tools/check_distribution.py`, `tools/agent-workflows/check.py` et
`git diff --check` passent. L’empreinte de `/proc/1525155/exe` correspond
à celle de `bin/swarm` et du binaire de la recette du lecteur.
Les tests Go complets et `go vet` du précédent jalon restent leurs preuves
datées ; ils ne sont pas présentés comme relancés pour cette actualisation
de ressources et scripts frontend.
