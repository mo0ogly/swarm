# Graphe : manipulation directe et repères visuels

Demande utilisateur : déplacer les cadres pour clarifier le plan, icônes et
couleurs des contrôles/graphe, aide des gestes modale, ajout de tâches et
modification/retrait graphiques des liens. Candidat308 SHA256
`e54a1dd539599fbac0a49aab33c353ead614eef47557101553fd7543f6349a6d`.
Périmètre : frontend consommant les opérations natives existantes ; aucun
producteur moteur, droit, schéma SQL ou champ métier nouveau.

## Choix et politiques

Glissement du corps et poignée réutilisent translateGroup/finish/save ; le
seuil4pixels d’écran distingue clic/drag comme le seuil existant des ports,
ce n’est pas une politique d’exécution. Un clic sans mouvement ouvre toujours
la tâche ; un déplacement terminé ne doit pas provoquer de clic fantôme.
Positions, zoom et annulation restent des préférences locales. Le fond ignore
cartes ET arêtes, pour ne pas capturer le clic d’un lien.

Icônes SVG locales statiques,18px, géométrie décorative aria-hidden ; textes
visibles sur desktop et noms accessibles conservés sur mobile. Le breakpoint
760/761 existant et les44px de contrôle tactile sont des choix de présentation.
Palette : paires sémantiques existantes, mélange45% pour l’attente, états écrits
et légende ; aucun nouveau code couleur, budget, retry, délai ou plafond métier.
Vue d’ensemble doit garder la même paire que zoom malgré l’override ID partagé.
Contraste rendu des contrôles échantillonnés : cibleAA4.5 vérifiée par le banc,
pas une certification WCAG globale.

## Fonctions, documentation et erreurs

inspectLink documente Work enregistré/from/to, absence de POST et handoff au
formulaire existant. propose ferme la modale avant ce handoff ; elle ne modifie
pas Work. Remplacement construit la liste sans l’ancien prérequis, conserve
les autres et déduplique le nouveau. Retrait enlève uniquement l’arête choisie.
Le formulaire task.update garde revision/CSRF/rights/cycle/readback et gestion
d’incertitude existants. Ajouter une tâche reste task.add, aucun agent lancé.
Aucune suppression arbitraire de tâche n’est créée par ce lot ; une question
utilisateur a distingué retrait de lien et suppression de tâche.

decorate/actionLabel/button documentent icône locale, libellé et état accessible.
Aucune icône externe, HTML non échappé ou handler inline. Données utilisateur
toujours textContent/options. finish garde rollback local sur Escape/cancel ;
une simple activation sans drag n’enregistre pas de déplacement. Capture portée
au bon élément et libérée à la fin ; les traitements existants des refus de
stockage/clipboard/export restent actifs. Aucun succès serveur inventé.

Aide : dialog nommé indépendant des éditeurs de tâche (classe dédiée), header,
corps défilable et fermeture visible ; top layer au-dessus du plein écran,
Escape/fermeture rendent focus au bouton et conservent le plein écran. Cleanup
ferme cette aide avant démontage. La détection des vrais éditeurs reste intacte.
Documentation produit GraphStudio FR/EN et notes local_changes FR/EN à jour.

## Preuves et replan

Les premières preuves et erreurs sont conservées sous
`.cache/graph-direct-20261101/` et expliquées dans
[le suivi](GRAPH-REGRESSION-20261031.md#ergonomie-visuelle-et-gestes--reprise-après303).
Overlay ciblé iteration5 : FR/EN×2thèmes, manipulation réelle et refus de mutation
Work pendant propositions, contraste, card mobile complète et analyste existant.
Une classe d’aide persistante confondue avec éditeur a été trouvée puis corrigée.

Native308 : business et responsive PASS/VERIFIED. Navigation initiale a refusé
un format de couleur résolu autre que rgb ; correction sémantique du banc
conserve contrôles sécurité et comparaison RGBA source/offline. Témoin XML
isolé démontre createElementNS HTML pour Canvas dans le SVG, rouge/bleu exacts.
Original SVG et deux échecs conservés, aucun octet produit308 modifié entre
ces mesures. graph-rgba est la recette complète corrigée à conclure.

La revue indépendante en lecture seule a confirmé le diagnostic de mesure et
n’a trouvé aucun autre défaut statique ; son verdict final et l’installation
restent à joindre, aucune réussite future n’est anticipée ici.
Les évolutions Authority/BLP parallèles et leurs limites ne sont pas acceptées
par ce dossier ; l’inventaire compilé partagé contient leurs propres changements.

## Empreintes examinées

- `web/cockpit-native-graph.js` : `e5606f969f1c6458ed0eb9b7f5f413eeb274dc1d0d402e3abc3f437611657716`
- `web/graph.css` : `82d1b8e42b5c98501774231a209ae5ca61428d61f21ce8ed8c1c832101b0ff9d`
- `locales/en.json` : `eff5e161481a609d7e7f26ac264288cf5809ab0726e5f3bb48f5fe47cfa4a1ff`
- `web/i18n-en.js` : `9ae0f6c9de4a0044da8c519731d359d7c33262f9fc9b985ecaaeaa3e4b1624ac`
- `version_history.json` : `e803a3cfc65d4c8d8ed9dbfe94d288057075f43091770df448a3055784db2c07`
- `tests/rust_graph_direct_ui.cjs` : `cd4f622c591974d06b5dc46ffd15cfe5e2b84bfe7db36525f2dde7c6f9874677`
- `tests/rust_graph_navigation.py` : `47363b13632d8b3b1aeed6784ff87a6c50714ad8f5ded5634e65e25d0e3a86d8`
- `tests/rust_graph_navigation_ui.cjs` : `5f9da6140aeab41259ad65e2ad6b312103ded42f3c789fbe0227b37aedcafb65`
- `tools/verification/recipes/rust-graph-workbench.json` : `3684a1af017b4267c94703a93b2468ee054aa451ed3bede111e6b7a1cd6d70e4`
- `docs/architecture/GRAPH-STUDIO.md` : `d63ed970524ae4ea101265f4e0b61c27b2662ba284145495bb9506bdea8b07e0`
- `docs/en/GRAPH-STUDIO.md` : `dcd00d5d72d2a7a4c912f5d40aadce6707b12a2535c07add1a3104006e97f0e6`

## Résultat final du code et limite de vérification locale

Relecteur indépendant `/root/graph_independent_review` : approve pour UI308,
SHA exact ci-dessus. Aucun défaut introduit confirmé ; aucun test/processus
lancé ni fichier modifié par le relecteur. Diff effectif303→308, capture/
bubbling/Escape, focus/nettoyage, task.add/update, FR/EN et empreintes relus.
Inventaires24/35/14 entrées stables. Recettes308 PASS/VERIFIED : graphe1561,
métier14tests nommés +246assertions, responsive145, soit1952assertions navigateur.
Les deux échecs de palette et le contrôle namespace restent originaux conservés.

Restart exact prepared-build308 réussi, image réellement exécutée et notes
FR/EN/capacités vérifiées via les deux sondes autorisées du lanceur. Le Root
est désormais en mode authority (évolution parallèle), moteur interne38434
et façade publique38433. L’ancien vérificateur de ressources via cookie owner
refuse403 comme prévu ; son échec installed308.stderr est conservé. La façade
refuse401 sans compte connecté. Browser access host indisponible, aucun onglet
utilisateur accessible à cette session. Ne pas désactiver authority, emprunter
une identité, créer/reset un compte ou extraire des secrets pour fabriquer PASS.

`installed308-identity.json` distingue identité/notes vérifiées et ressources
protégées/fenêtre Version publiques non vérifiées. Question utilisateur envoyée
pour confirmation visuelle après sa connexion publique. U1–U4 vérifiés ; U5
partiel tant que ce retour/session manque. Propriétaire de la vérification :
Codex ; dépendance : session authentifiée utilisateur et connexion navigateur.
Les preuves du même binaire sur Roots isolés ne sont pas un test de son compte
réel. Aucun commit/push et aucune conformité globale Authority/BLP revendiqués.
