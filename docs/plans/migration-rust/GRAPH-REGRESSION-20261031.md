# Régression de l’atelier graphe — 31 octobre 2026

Besoin : retrouver zoom, navigation, déplacement des cartes, plein écran et
création/édition des objets du graphe après la migration. Propriétaire : Codex.
État initial : `cockpit-native-graph.js` déclare un rendu readonly « no buttons » ;
`native_admin::cockpit_page` retire tous les scripts historiques, dont
`pilotage.js`, `graph-draft.js` et `graph-studio.js`. Les sources historiques
restent présentes. Le lanceur local annonce `running:false` à cette observation.
Les nombreux changements préexistants du checkout sont conservés.

Choix : restaurer les interactions sur les vrais Work natifs. Recharger les
scripts historiques sans adaptation exigerait leur snapshot Go complet et des
routes de brouillon absentes ; fabriquer ce snapshot masquerait des contrats
non portés. La présentation locale reste séparée des écritures contrôlées par
le moteur, sans lancement d’agent.

Plan et critères :
1. G1 : témoin navigateur natif puis contrôle manquant reproduit avant correction.
2. G2 : zoom, vue d’ensemble, panoramique, déplacement clavier/souris et annulation,
   positions mémorisées par mission/orientation, plein écran accessible.
3. G3 : identifier les objets attendus ; raccorder création et liens aux opérations
   publiques réelles, avec refus/révision périmée sans mutation et relecture.
4. G4 : recette permanente FR/EN, thèmes, mobile, console/réseau, aucune mutation
   pour la présentation ; documentation et revue politiques/fonctions/erreurs.
5. G5 : build normal, empreintes des ressources servies, livraison locale et
   dialogue Version. Un candidat ou une doc ne prouve pas l’installation.

État initial de cette étape : diagnostic source établi ; critères fonctionnels alors non vérifiés.
Preuves originales : `.cache/graph-regression-20261031/` ; échecs conservés.
La parité des brouillons atomiques historiques, annuler/rétablir et inspection
reste un critère distinct à examiner, jamais déduite de simples contrôles visuels.

## APEX — reprise demandée et preuves de navigation

Classement adopté : G1/G11 présentation réécrivable, G4 données/écritures en
parité stricte. Témoin navigateur avant correctif : trois cartes présentes puis
FAIL `fr-etat-zoom-control-present` (`before-browser`). Premier lancement refusé
par le sandbox (`web-listener-bind`, errno1), puis serveur autorisé sans changer
le produit. Deux défauts du banc isolés : focus SVG Puppeteer et événement close
asynchrone. `navigation-preview3` : 69 contrôles navigateur PASS + Work inchangé,
FR/EN, clair/sombre, 390/320px ; overlay workspace explicite, aucune preuve
embarquée ou installation déduite. Inspection visuelle du 390px clair effectuée.

Prochain incrément réservé : ajout borné de `task-add` à la route `/api/v1/action`
(service.rs, déjà modifié avant ce travail ; original/empreinte conservés),
formulaire natif de tâche/liens et rafraîchissement cockpit. Même enveloppe,
CSRF, Root lié, révision et validation moteur que `task.update` ; aucun nouveau
policy, modèle ou lancement. Les brouillons batch atomiques historiques restent
une capacité distincte à porter ; ne pas les simuler avec des écritures partielles.

## Vérification du premier candidat et replan

Profil normal277 `5f827830…` vérifié et archive conservée ; première qualification
HTTP refusée par sandbox, puis `build-authorized` qualifie le même candidat sans
installation. Recette `native277-editing2` : création/liens/cycles/persistance
fonctionnels dans4combinaisons, mais FAIL diagnostic :4lectures status500
`root-cleanup` et requêtes interrompues. Les tests enchaînaient sur le rendu
provisoire après sauvegarde. Remplacement de l’événement non attendu par un
consommateur asynchrone : le formulaire attend la relecture du cockpit ; une
erreur de rafraîchissement conserve le résultat non confirmé et interdit le
renvoi. Aucun refus500 n’est autorisé dans la recette.

Overlay `refresh-pointer-preview2` : les4parcours clavier/souris et cycles
passent sans500 ; la perte d’accusé est correctement bloquée sans renvoi et
la CLI montre UN objet `uncertain` réellement enregistré (révision17). Le banc
échoue ensuite car il observait main-content libre avant le début de select ;
l’attente corrigée suit refresh.disabled et l’objet effectivement relu.
Un précédent banc de fermeture a été arrêté via SIGINT sur son seul Node
identifié (même parent que la fixture) ; aucune borne arbitraire ou relance
inchangée. Attendre l’événement close effectif, pas seulement dialog.open=false.

Revue lexical/sémantique : trois libellésEN absents ajoutés ; placementTB des
tâches indépendantes corrigé ; lecture de révision exige un entier sûr ; handlers
de rafraîchissement et commentaires de fonctions actualisés. Sources du banc
désormais archivées avant exécution et comparées après, exit/argv/dates retenus.
Le code public de cycle reste `format` ; diagnostic détaillé côté UX est une
amélioration à étudier par G4, sans changer implicitement le wire historique.
Brouillons atomiques multi-destinations, aperçu moteur et undo/redo : toujours
ouverts hors restauration du formulaire atomique d’une tâche, propriétaire G4.

## Graphe278 vérifié ; précondition de lancement découverte

Recette permanente278 :2steps PASS, verify VERIFIED, navigation sans mutation
et édition complète FR/EN/thèmes, souris/clavier, perte d’accusé sans renvoi et
création depuis zéro. Profil normal archive exact46b57657… ; installation pas
encore faite. Compte Biba parallèle : delta FD observé et conservé dans
`biba-concurrent.diff`, aucun fichier d’autrui modifié ni conformité déduite.
Le témoin public Biba278 refuse AVANT la frontière avec
`agent-workflow-body-limit-exceeded` ; cas2/3 non exécutés, originaux conservés.

Préflight : planner/worker33022, subplanner33025, reviewer18851, cap initial32000.
Replan : APEX grossi avec le tableau de bord ; réduire répétitions plutôt que
relever le cap (option rejetée car masquerait le défaut). Canonique APEX4653→3584
octets, mêmes4étapes, champs du plan, replan/autorités, revue liée au SHA,
critères de clôture, toutes les campagnes/compteurs PAEA et référence Rust.
CONTRACT reste inclus, aucune obligation n’est déplacée dans une référence
non transmise. Préflight après :31953/31956/18851 ; fichiers de politique intacts.
Propriétaire Codex ; effet attendu : vrai contexte Rust, puis témoin public
démarrant le stub inerte avec même politique. Ancienne skill conservée dans
`apex-before.md` ; écriture canonique propagée à l’alias .agents, pas aux processus
déjà lancés. Tests natifs/rebuild/réexécution et installation restent à vérifier.

## Livraison intermédiaire280 et RETEX borné (historique)

**Instance locale active : Rust280**, image
`c7fe8753a82d972dbd9476ac8ebd6f4c76f803e96023af364bc2e6c6fcd204d1`,
PID1726106 au contrôle final. Sélection préparée et redémarrage pris en charge ;
GET protégés, `/proc/PID/exe`, version,7assets compilés, capacité task/task-add et
dialogue Version FR/EN14assertions vérifiés dans `installed280/`. Aucun POST
dans ce contrôle installé, aucun commit/push/publication. Les missions restent
les données du Root ; les écritures de recette ne touchent que les fixtures.

| Critère | Résultat et portée | Preuve |
|---|---|---|
| G1 Contre-exemple | PASS : image274 immuable,3cartes puis absence du zoom ; FAIL original attendu, pas une recette positive | frozen274-reproducer, image06170fdb… |
| G2 Navigation | PASS : zoom boutons/molette, fond, cartes souris/clavier, annulation, persistance, orientation, plein écran/focus, mini-carte,320/390px | permanent279-frozen,105contrôles ; Work identique |
| G3 Création/liens | PASS : tâches réelles, ports souris/clavier, proposition sans écriture, retrait, cycle refusé sans mutation, reprise, perte d’accusé sans renvoi, première tâche | permanent279-frozen,157contrôles, CLI et relecture ; aucune IA |
| G4 Documentation/revue | PASS local : FR/EN, fonctions/erreurs/politiques relues avec empreintes ; revue déclarative, pas indépendante | review279 ; recettes verify VERIFIED |
| W1 Consignes | PASS : politique32000 intacte,4rôles et9tests Rust ; vrai témoin agent/stub et gardes atteints | workflow-check et biba279-frozen |
| G5 Activation | PASS :280 actif/prêt, même graphe/API et mêmes consignes que279 ;7assets réellement servis identiques au compilateur | install280, installed280 |

279→280 : seul `swarm-biba-linux/src/kernel.rs` diffère dans l’inventaire
compilateur ; FD de ruleset dupliqué au-dessus des descripteurs standard0–2.
Aucun masque de droits, cap, défaut, fallback ou contrat de tâche ne change.
Diff original `kernel280-concurrent.diff` conservé ; témoin et refus/reprise
Biba280 PASS (`biba280-frozen`), sans verdict Biba global/identité authentifiée.
Les262contrôles du graphe279 sont réutilisés seulement pour leurs entrées
pertinentes inchangées ; le binaire installé280 a sa propre identité, son profil
normal et les ressources protégées vérifiées. La sélection279 a d’abord refusé
car280était devenu le dernier candidat : garde respectée, jamais contournée.

Deux erreurs de méthode supplémentaires conservées : un chemin de staging
`bin/swarm.candidate-*` est consommé par build lorsque le serveur est absent ;
les reprises utilisent l’archive immuable hashée. Le statut exécuté dans le
sandbox ne voyait pas le PID hôte : état final vérifié hors du même sandbox que
le serveur, et corroboré par HTTP/proc, sans faux diagnostic d’arrêt.

Cause initiale : remplacement de l’atelier interactif par une projection readonly
sans recette de gestes équivalente. Prévention : recette permanente native
`rust-graph-workbench.json` sur image explicite ; contrôles positifs puis refus,
entrées/flux/exits conservés, modal close attendu réellement et rafraîchissement
attendu jusqu’au consommateur. Résultat mesuré :262contrôles navigateur,9tests
de contexte,3cas ciblés Biba,14contrôles Version ; aucun pourcentage ni gain
autonomie/coût inféré. L’apprentissage est dans les sources/recettes et les
consignes compactées embarquées280 ; aucun contexte déjà démarré n’est réécrit.

Restent ouverts hors ce parcours restauré : brouillons atomiques multi-tâches,
aperçu de lot et undo/redo historiques (G4, reprise : porter API/stockage et
recette atomique, sans simuler par écritures partielles). Le message public de
cycle `format` reste peu détaillé (G4, reprise : contrat de diagnostic sûr compatible
avec le wire). Aucune clôture de migration globale ni acceptation indépendante
n’est déduite de cette livraison.

## Clôture locale courante —281

La dernière relecture a corrigé la notice « autres commandes à raccorder » de
Conduite et le commentaire Rust du service d’assets : le graphe possède désormais
des écritures de tâches/liens explicitement portées. Nouveau témoin de notice
FR/EN ajouté à la recette. Premier build281 refusé car un éditeur concurrent
modifiait locales/en.json, i18n-en.js et paea-approval-form.js pendant compilation ;
originaux conservés,280resté actif. Contrôle de cohérence et observations
d’empreintes stabilisées, puis nouveau build sur ces nouveaux octets accepté.

**Courant : v0.1.1-dev.281, PID1866377**, image
`5ae9857108767ecd305687cb7879206c489172a1a94b7b40ee0fc6f09e17b0d3`.
`permanent281` :2steps PASS, verify VERIFIED,270contrôles fonctionnels navigateur
(109navigation/161édition), plus diagnostics sans erreur inattendue.
`biba281-frozen` :3cas ciblés PASS, limites de sécurité inchangées.
`installed281` : processus hashé,7assets protégés conformes à l’inventaire
compilateur, capacité d’édition et14assertions Version FR/EN PASS, sans POST.
Sources compilées plus récentes en attente à l’activation :0.

`review281` complète la revue liée aux octets de la notice, traduction, commentaire
et delta de présentation PAEA conservé séparément ; ce dernier n’est pas chargé
par le graphe et aucune acceptation globale PAEA n’est inférée. Les preuves274,
277/278/279/280 et les échecs intermédiaires restent intacts. Aucun commit/push exécuté par cette session.
Les limites des brouillons par lots/aperçu/undo restent celles indiquées ci-dessus.

Dernière observation Git : HEAD a évolué pendant ce travail via des actions
concurrentes (final observé f9ff5821…, build281 enregistré8d9eb325…). Cette
session n’a exécuté aucune commande commit/push et conserve ces évolutions.
Comparaison finale avec l’inventaire effectif281 :0entrée compilée différente ;
les nouveaux changements documentaires/tests ne remplacent pas la provenance
du binaire. Le dialogue conserve le vrai commit de build et sa déclaration dirty.

## Publication Git demandée ensuite

L’utilisateur a ensuite autorisé « commit push ». Les mentions précédentes
d’absence de commit/push décrivent l’étape de développement avant cette demande.
Le code du graphe est déjà dans l’historique partagé ; cette étape conserve le
bilan final et pousse la branche courante sans inclure les modifications non
commitées des autres chantiers ni réécrire les commits existants.
