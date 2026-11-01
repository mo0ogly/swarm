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

## Quick wins proposés —1 novembre2026

Origine : demande utilisateur « tu as des quickwin ? », puis inspection des
sources actuelles. Propositions, pas décisions adoptées ni fonctions réalisées.
Périmètre : présentation du graphe ; propriétaire proposé : Codex/G1. Aucun
changement du moteur, des missions, des droits ou des politiques d’exécution.

| ID | Proposition et valeur | Preuve actuelle | Critère observable | État / reprise |
|---|---|---|---|---|
| QG1 | Recherche par titre/identifiant avec centrage, utile sur les plans denses | navigation de cockpit-native-graph.js sans recherche | une recherche révèle et centre la bonne carte ; effacement conserve le placement et le Work | proposé, priorité1 ; reprise : réalisation de ce lot UX |
| QG2 | Libellé « Enregistrement en cours… » jusqu’à la relecture, pour rendre l’attente explicite | editTask désactive les champs mais garde le libellé Enregistrer | état d’attente visible/accessible, retour correct après refus, un seul POST | proposé, priorité2 ; reprise : même lot UX |
| QG3 | Mini-carte navigable, clic et clavier pour atteindre une zone | SVG role=img et pointer-events:none dans swarm-design.css | navigation atteint la zone indiquée, focus visible et aucune modification du plan | proposé ; reprise : lot de navigation |
| QG4 | Annulation du dernier déplacement/rangement local des cartes | Ranger les cartes remet immédiatement les positions à zéro, sans historique local | restauration des positions précédentes, y compris après rangement, sans écrire de tâches/liens | proposé ; reprise : lot de placement, distinct de l’undo métier des brouillons |

Priorité conseillée : QG1 puis QG2. Les brouillons atomiques multi-tâches restent
un chantier moteur séparé, déjà conservé dans les critères ouverts précédents.

## Reprise QG1/QG2 —1 novembre2026

Utilisateur « reprends » : réalisation des deux priorités annoncées, Codex seul.
QG1/QG2 décidés et réalisés en source ; preuve produit et installation attendues.
QG3/QG4 restent proposés. Comparaison : filtrer/recalculer le graphe masquerait
des dépendances et modifierait le placement ; recherche/repères/centrage sur le
rendu existant retenus, sans requête réseau ni mutation de Work. La progression
de sauvegarde suit le pending existant jusqu’au consommateur de rafraîchissement.
Aucune nouvelle politique d’exécution, aucun délai, retry ou plafond de résultats.

Critères : titre/ID, casse/accents, plusieurs résultats et cycle Suivant, aucun
résultat, effacement, clavier/mobile/plein écran et géométrie/Work conservés ;
attente visible pendant envoi, relecture et refresh, un seul POST ; refus et
issue incertaine cessent de dire « en cours » sans réactiver un renvoi incertain.
Docs FR/EN, commentaires des fonctions et recette native actualisés ensemble.

Preuves originales `.cache/graph-quickwins-20261101/` : before-search échoue sur
search-control après témoin3cartes ; before-busy échoue sur saving-feedback-Ack
après un POST réel confirmé. Mode --skip-search-check uniquement pour isoler
ce second contre-exemple ; aucune qualification positive ne l’emploie.
Les travaux PAEA/BLP et les entrées d’index préexistantes sont conservés.

### QG1/QG2 livrés — Rust289

Recherche et progression de sauvegarde : réalisés, vérifiés et installés.
Image `510bf69d75b13ef7a7183983909b1511b52f01ae1e314fb1684104d8032099af`,
PID728136 au contrôle installé, v0.1.1-dev.289 active/prête.
`permanent289` :2steps PASS/verify VERIFIED, 395assertions navigateur
retenues (157navigation/238édition), diagnostics sans erreur inattendue.
FR/EN, clair/sombre,320/390px, recherche sans mutation, sauvegarde maintenue sur
les3phases avec un seul POST, refus et issue incertaine vérifiés. Premier
passage overlay PASS explicitement distinct du vrai binaire qualifié.
`installed` : identité du processus,7assets servis liés à l’inventaire compilateur
et14contrôles Version FR/EN PASS ; aucun POST dans ce contrôle du vrai Root.

Revue QG1/QG2 liée aux octets : politiques UI, fonctions, erreurs et documentation
examinées (`review/owned-hashes.json` et `review/review.md`). La revue globale
du delta compilateur reste explicitement ouverte pour les producteurs BLP/PAEA
concurrents non modifiés par ce lot ; aucun verdict global, aucune acceptation
indépendante ni conformité de ces autres parcours n’est inférée du graphe.

RETEX borné : indicateur aligné sur la fin effective du consommateur, pas seulement
sur la réponse POST ; recherche sur le rendu courant plutôt que reconstruction
ou masquage du plan. Les recettes permanentes réutilisent ces témoins et gardes.
QG3/QG4 restent proposés : mini-carte navigable et historique de placement local,
avec leurs critères et conditions de reprise conservés dans le tableau initial.

## Quick wins complémentaires proposés —1 novembre2026

Nouvelle demande utilisateur : « tu as d’autres quickwin ? ». Source observée :
web/cockpit-native-graph.js et styles actuels ; aucune nouvelle réalisation
adoptée par cette question. QG3 mini-carte navigable et QG4 annulation du dernier
placement/rangement restent les propositions précédentes, sans doublon.

| ID | Origine / bénéfice | Périmètre et propriétaire | Critère observable | État / reprise |
|---|---|---|---|---|
| QG5 | Vue d’ensemble calcule son zoom sur base.width/base.height, tandis que update étend le SVG selon les cartes déplacées : améliorer le cadrage des placements manuels | affichage local, Codex/G1 | dans un cas compatible avec la plage de zoom, carte déplacée hors des dimensions initiales entièrement cadrée, positions et Work conservés | proposé, priorité conseillée ; hypothèse de défaut UI à reproduire avant correction |
| QG6 | L’identifiant est affiché dans la carte et le formulaire : copie directe pour support/échanges | carte/formulaire, Codex/G1 | copie le seul ID littéral ; succès annoncé seulement après confirmation, refus clipboard visible et texte sélectionnable | proposé ; reprise : prochain lot d’ergonomie |
| QG7 | Les gestes existent (Ctrl+molette, poignées, ports, Entrée, Échap) mais leur découverte dépend de la documentation | aide contextuelle courte, Codex/G1 | contrôle Aide nommé, gestes actuels FR/EN, ouverture/fermeture/focus clavier sans masquer les erreurs | proposé ; reprise : prochain lot de découverte de l’interface |

Ordre conseillé : QG5 puis QG4 pour retrouver le cadrage et récupérer un placement.
Ces propositions ne valent ni implémentation, ni vérification navigateur, ni
autorisation de modifier le moteur ou de revenir sur des tâches enregistrées.

## Lot QG3–QG7 adopté — demande « fais tout »

Codex prend les cinq propositions restantes : QG5 cadrage sur l’étendue réelle,
QG4 annuler le dernier placement/rangement, QG3 mini-carte souris/clavier et
repère de viewport, QG6 copie d’ID et refus récupérable, QG7 aide des gestes.
Scope : UI locale et préférences ; aucun changement de tâche, de politique
d’exécution, de droits ou de budget. Pas de délégation. Sources/état initial
conservés dans `.cache/graph-five-quickwins-20261101/`.

Choix : réutiliser le SVG/contrôles existants plutôt qu’un second moteur de
navigation. Un seul placement précédent en mémoire par rendu, plutôt qu’un
undo de données métier ou un historique persistant non demandé. La plage de
zoom existante est conservée : un cadrage empêché par le zoom minimum doit
être annoncé. La copie ne dit « copié » qu’après acknowledgement du navigateur ;
refus/indisponibilité donnent un ID lisible/sélectionnable et une cause sûre.

Critères observables :
- témoin Vue d’ensemble valide, déplacement hors dimensions initiales puis
  cadrage corrigé sans bouger les cartes (dans la plage de zoom) ;
- annulation d’un déplacement ou rangement, geste annulé n’écrase pas le témoin,
  sauvegarde des coordonnées restaurées, aucune révision Work changée ;
- mini-carte clic/flèches/Début/Fin, focus visible et rectangle du viewport ;
- vrai clipboard headless après action utilisateur, puis refus injecté après
  témoin valide : pas de faux succès, ID sélectionné, aucun formulaire ouvert ;
- aide FR/EN repliée, ouverture/fermeture/Escape/focus, dans les deux thèmes et
  sur mobile ; recettes QG1/QG2 et édition/refus/reprise restent requises.

Documentation/fonctions/traductions/recette sont dans le même incrément.
Qualification sur image native immuable puis activation locale via le lanceur ;
review dans ce contexte est une auto-revue, jamais acceptation indépendante.

### Replan du lot QG3–QG7

- Défaut reproduit `noop-before/browser.stderr` : glisser une seconde carte puis
  revenir à son origine crée une entrée de coordonnées zéro et remplace undo
  malgré une géométrie identique. Critère concerné : annulation du dernier vrai
  placement. Correction décidée : comparer les coordonnées effectives avec zéro
  pour une entrée absente ; conserver le témoin navigateur avant/après.
- Lenteur de build : maintenance Git cruft concurrente ~7 Gio RSS, swap presque
  pleine, compilateur actif et compteurs IO/CPU progressants. Preuve
  `build-pressure.json` ; aucun processus tiers interrompu, aucune relance sans
  précondition changée. Le premier candidat ne sera pas livré avec le défaut.
- Amélioration de méthode retenue, propriétaire maintenance du dépôt : planifier
  les compilations et repacks lourds sans concurrence mémoire. État proposé hors
  lot UI ; reprise lors du prochain réglage explicite de maintenance, critère
  observable : progression du build sans saturation mémoire concurrente. Aucun
  réglage Git global ni politique machine changé ici.

### QG3–QG7 livrés — Rust290

Les cinq critères du lot sont réalisés et vérifiés localement : cadrage des cartes
manuelles, annulation d’un déplacement/rangement terminé (une fois), mini-carte
clic/clavier avec viewport, copie exacte d’ID avec refus/indisponibilité récupérables,
aide dépliable FR/EN. Les annulations et les glisser-retour sans changement gardent
le témoin précédent ; les coordonnées restaurées sont mémorisées localement.

Preuves sous `.cache/graph-five-quickwins-20261101/` :
- `before/` : témoin de cadrage valide puis défaut original reproduit ;
  `noop-before/` : glisser-retour écrasant undo avant correction ;
- `preview/`, `preview2/` : originaux des défauts de préconditions du harnais
  (permission clipboard distincte et ordre de nœuds), sans faux PASS ;
- `after-noop/` : reprise navigateur après correction, Work inchangé ;
- `embedded-recipe/` : deux étapes PASS, 613 assertions navigateur, FR/EN,
  clair/sombre, mobile 390/320, édition/refus/réponse perdue/reprise ; intégrité
  confirmée par `recipe.py verify`. Racines isolées, aucun fournisseur ;
- `candidate.json` : profil natif normal, quatre artefacts cœur sans feature
  fixture, 730 entrées effectives archivées. Le premier build est refusé pour
  modification concurrente de Cargo.lock et du manifeste swarm-authority ;
  originaux dans `.cache/local-delivery/rust-7z38_c2y/`. Second build qualifié dans
  `.cache/local-delivery/rust-gj64thyb/`, sans contournement du contrôle ;
- `review/source-review.json` et `review/result.json` : sept entrées de compilation
  modifiées examinées (politiques, fonctions, erreurs, documentation). Dépendances
  du workspace examinées pour leur effet sur ce binaire ; aucune qualification
  globale des nouveaux modules d’autorité non liés, ni revue indépendante ;
- `restart.stdout`, `installed/result.json` : service géré PID 933308,
  v0.1.1-dev.290, SHA256
  `2f104936daae347cbe46d21410a49c1485e135da9a2cfba15011e6d4eab39e2b` ;
  sept ressources protégées identiques à l’inventaire compilé, 14 assertions
  fenêtre Version FR/EN, commit réel `18bb6bb6b5ee987e12a1f51c5ccda70fcd9db508`,
  modifications non commitées explicites, aucun POST sur l’instance réelle.

RETEX du lot : une entrée de position absente et une entrée zéro ont la même
signification visuelle ; la comparaison de représentations JSON ne suffisait pas
pour protéger undo. Le contre-exemple est désormais dans la recette permanente.
Le témoin clipboard réel précède chaque refus injecté ; l’attente de test observe
aussi l’issue non confirmée au lieu d’attendre uniquement un succès. Les tests
mini-carte choisissent une position géométrique, sans supposer l’ordre du moteur
de placement. Cargo « Compiling » n’établit pas un artefact terminé : lire les
événements JSON. Ces apprentissages persistent dans tests/recette/commentaires et
ce suivi ; aucune consigne native embarquée n’a été modifiée dans ce lot.

Limites maintenues : undo porte seulement sur le dernier placement local et sa
mémoire disparaît au rerender ; limite de zoom 15 % annoncée si insuffisante.
Brouillons de liens atomiques et historique métier restent le chantier de migration
précédemment ouvert, hors des cinq quick wins. Aucun push ni publication ici ;
les commits concurrents observés sont conservés, sans réécriture d’historique.

## Nouvelles pistes QG8–QG12 — demande de propositions

Origine : inspection du cockpit natif après le lot Rust290, nouvelle question
« tu pas plus de quickwin? ». État : propositions, sans réalisation autorisée
par cette seule question. Propriétaire proposé : Codex, périmètre graphe local.
Les limites ci-dessous sont établies par le code ; leur impact utilisateur reste
à reproduire avant une correction. Priorité proposée : QG8 puis QG9.

| ID | Origine et valeur attendue | Critère observable proposé | État et reprise |
|---|---|---|---|
| QG8 | Le handler Ctrl+molette change zoom sans compenser scrollLeft/scrollTop : zoomer autour du pointeur pour garder son repère | le point du graphe sous le curseur reste au même endroit à l’écran hors bornes de défilement ; Work inchangé | proposé ; reproduire puis adopter le lot navigation |
| QG9 | save ne mémorise que zoom/positions ; orientation est dans dataset et cleanup ferme la modale au rerender : conserver la vue après édition | après sauvegarde confirmée, orientation, zone visible et plein écran sont restaurés sur le même Work ; changement de Work indépendant | proposé ; définir précisément les restaurations lors du prochain lot de navigation |
| QG10 | La recherche souligne les correspondances ; pas de repérage local des voisins d’une carte : rendre prérequis et dépendants immédiatement lisibles | une commande nommée ou le focus souligne la carte, ses liens entrants/sortants et leurs voisins ; légende et effacement, aucun lien masqué ni modifié | proposé ; prochain lot de lecture des dépendances |
| QG11 | Le formulaire liste tous les prérequis dans un select multiple sans recherche : faciliter les plans denses | filtre par titre/ID, choix sélectionnés toujours visibles et conservés en changeant le filtre ; refus moteur inchangés | proposé ; prochain lot du formulaire |
| QG12 | Le toolbar natif n’a pas de téléchargement du SVG courant : partager le graphe sans capture d’écran | export SVG local fidèle au placement, liens et libellés, ouvrable hors application, sans contrôle interactif ni credential ; aucun POST | proposé ; prochain lot de partage local, vérifier la sérialisation et le rendu exporté |

Aucune modification de comportement, de fonction ou de politique : cette mise à
jour conserve les pistes et leurs critères. Pas de nouvelle livraison requise
pour ce seul suivi ; les versions du produit restent celles réellement installées.

## Lot QG8–QG12 adopté — « ok lances tout »

Propriétaire : Codex seul, sans délégation. Parcours : manipuler/lire le graphe,
éditer des prérequis, enregistrer puis exporter la vue. Risque C pour présentation ;
les écritures de tâches gardent les contrats moteur existants. Les travaux BLP et
Autorité parallèles sont préservés. Baseline installée Rust290, originaux dans
`.cache/graph-qg8-12-20261101/`. Pas de commit/push implicite.

Ordre et critères : QG8 conserve le point sous Ctrl+molette hors clamp natif ;
QG9 conserve orientation/zoom/scroll/plein écran pendant les rerenders du même Work,
sans imposer sa vue à un autre Work ni ouvrir le graphe par-dessus un formulaire ;
QG10 commande explicite de repérage d’une carte et de ses voisins directs, tous
les nœuds/liens restent visibles, effacement nommé ; QG11 filtre littéral titre/ID
sans casse/accents, sélection toujours visible et conservée, même API/refus ;
QG12 vrai téléchargement SVG autonome reflétant géométrie/couleurs/libellés,
sans contrôles interactifs, credential ni ressource externe, et sans POST.

Choix : réutiliser le renderer et ses préférences plutôt qu’un second moteur.
Snapshot en mémoire de la vue pendant les rafraîchissements du même Work, sans
persister le plein écran ou la recherche ; zéro changement de politique moteur.
Le plein écran est différé jusqu’à la fermeture de l’éditeur pour respecter les
couches modales. La recherche des prérequis conserve le select natif et expose
les choix sélectionnés même hors filtre. Export : sérialisation d’un clone épuré
avec les couleurs calculées, plutôt qu’une capture ou un endpoint serveur.

Vérifications : contre-exemple sur image immuable, contrôles ciblés pendant les
édits, puis recette native consolidée FR/EN, deux thèmes, mobile, erreurs/reprise,
Work inchangé pour présentation. Docs, commentaires et traductions dans le même
incrément, revue politiques/fonctions/erreurs liée aux empreintes. Installation
via candidat qualifié, identité réelle/assets protégés et dialogue Version vérifiés.
Clôture : cinq critères vérifiés localement et RETEX ; auto-revue distincte d’une
acceptation indépendante, aucune qualification globale de travaux tiers.

Replan QG11 : `preview/escape-diagnostic.json` établit qu’Échap dans une saisie
search non vide efface le texte et conserve le formulaire ; le harnais attendait
sa fermeture. La tentative est arrêtée proprement avec originaux conservés.
Choix : contrat explicite cohérent avec la recherche du graphe, Échap efface le
filtre puis un second ferme le dialogue. Handler et assertion de recalcul des
options ajoutés ; ce n’est pas une preuve d’export ou de restauration de vue.

Replan QG9 : `preview3/browser/fr-etat-view-control.json` montre TB/zoom2/plein
écran valides, largeur 1410 > SVG1196, défilement vertical réel 58 mais horizontal
nul. Le témoin exigeait à tort un défilement horizontal. L’assertion couvre
maintenant tout axe réellement défilable ; aucune limite produit n’est modifiée.
Pour l’export, ajout d’une validation XML avant téléchargement afin qu’un texte
non représentable en XML soit refusé visiblement plutôt que livré illisible ;
le cas invalide suit le téléchargement valide dans le garde permanent.

Replan QG12 : `preview4` atteint 450 assertions d’interaction puis refuse au
contrôle console : trois violations CSP par langue/thème lors du XML invalide.
`xml-csp-diagnostic.json` confirme exactement les trois empreintes des styles de
la page parsererror générée par Chrome. Le parseur de contrôle a été remplacé
par la validation des caractères XML 1.0 avant sérialisation du SVG construit par
le renderer. Aucun unsafe-inline ni exception console ajouté. Les plages Unicode
sont un invariant du format, pas une politique opérateur. Original XML invalide,
refus sûr et récupération restent dans le garde ; recette native finale attendue.
Le premier affichage du fichier diagnostic était trop précoce et la commande
suivante avait continué : la vérification du diagnostic a été terminée séparément,
avec correspondance exacte. Les étapes dépendantes restantes sont séparées.

Replan export : `download-refusal-before` reproduit un lien de téléchargement
resté dans le DOM après SecurityError du clic navigateur. Le lien est désormais
retiré dans finally, le refus conserve sa famille sûre et la récupération exige
un nouveau vrai téléchargement. Le candidat291, techniquement préparé avant cette
correction, n’est pas installé ; son record est conservé dans
`candidate-before-download-cleanup.json`. Nouveau candidat requis pour la source
corrigée et la documentation de nettoyage de sélection dans le cockpit.

Replan QG8/QG9 : `unfinished-before` prouve qu’un rerender capturait un glisser
inachevé ; `unfinished-zoom-before` prouve que le zoom pouvait aussi mémoriser ses
coordonnées provisoires. Le snapshot commun `committedPositions` dessert désormais
stockage et rerender ; le snapshot du défilement exclut de même un panoramique
inachevé. Annulation, zoom, placement terminé et restauration doivent rester
compatibles dans la recette finale. Le candidat292 est conservé mais non installé
(`candidate-before-gesture-snapshot.json`) ; nouvelle image requise. Hypothèse
hors critères courants conservée : vérifier séparément le défilement extérieur du
studio quand ses commandes débordent sur mobile ; propriétaire UX, reprise si un
parcours réel démontre une zone visible perdue malgré le canevas restauré.

Recette embarquée293 initiale : échec du témoin de pan en anglais clair, après
PASS des gardes glisser/zoom/rerender. La hauteur du toolbar anglais laissait le
SVG entièrement visible verticalement après zoom : aucun pan n’était possible.
Fixture corrigée sur largeur390 avec étendue horizontale constatée avant geste,
sans changement produit ni nouveau build. Original `embedded-recipe/` conservé ;
nouvelle preuve requise sur la même image293 et le harnais corrigé.

QG9, hypothèse mobile reprise et confirmée : `outer-scroll-before` sur image293
constate une fenêtre défilée à largeur390/hauteur600 puis un retour à zéro au
rerender. Le snapshot conserve maintenant aussi le scroll extérieur du studio,
y compris durant le double rendu sous l’éditeur. Le garde de sauvegarde réelle
comprend un témoin positif de scroll extérieur ; son clamp natif reste applicable.
La recette293 `embedded-recipe-2/` demeure PASS/verify VERIFIED pour son ancienne
portée, sans prétendre vérifier ce critère ajouté ni la prochaine image.

QG11 : `filter-enter-before` prouve qu’Entrée dans la recherche déclenchait une
soumission native avec formulaire valide. Le garde capture cette tentative avant
le gestionnaire applicatif, sans POST ni création de tâche. La saisie intercepte
maintenant Entrée pour rester dans le filtre ; l’enregistrement explicite reste
inchangé. Candidats291–293 non installés ; candidat final requis pour ces octets.

Attente de livraison finale : `final-preview/result.json` PASS sur les sources
corrigées, quatre variantes et éditions réelles, mais overlay explicite et donc
pas preuve d’embarquement. `build4` attend le verrou du lanceur derrière une autre
compilation native ; /proc confirme locks_lock_inode_wait. La maintenance Git
concurrente (PID1114540, identité conservée dans maintenance-observation.json)
consomme environ7Go et crée une forte pression mémoire/swap. Aucun processus
tiers n’est arrêté. Autorisation de pause temporaire avec reprise automatique
sollicitée à l’utilisateur, sans considérer le silence comme un accord. Propriété
et reprise : maintenance/détenteur du verrou ; fin naturelle ou accord explicite,
puis qualification/recette de l’image finale et activation. Le serveur290 reste
actif à cette observation. Les progrès de source ne valent pas installation.

## QG8–QG12 livrés localement — Rust294

Les cinq critères adoptés sont réalisés et vérifiés sur l’image native finale.

| Critère | Résultat local et preuve observable |
|---|---|
| QG8 Zoom autour du curseur | PASS : point conservé hors clamp ; zoom durant un glisser ne mémorise pas les coordonnées provisoires |
| QG9 Conservation de la vue | PASS : même travail conserve orientation/zoom/positions/scroll du canevas ET du studio ; sauvegarde réelle à hauteur600, éditeur au-dessus durant Ack/Readback/Refresh ; contexte différent/sélection absente indépendants ; gestes inachevés annulés |
| QG10 Repérage des voisins | PASS : carte, prérequis, dépendants et liens concernés distingués ; aucun élément masqué, effacement/focus |
| QG11 Recherche des prérequis | PASS : titre/ID sans casse/accents, sélection visible et conservée hors filtre, absence de résultat et Échap ; Entrée ne soumet pas, sauvegarde explicite et refus/reprise conservés |
| QG12 Export SVG | PASS : téléchargement réel, géométrie et palette conservées, ouverture hors application sans contrôle ni ressource externe ni credential ; XML invalide/génération/clic refusés sûrement, nettoyage et vrai téléchargement de reprise, zéro POST d’export |

Preuves finales : `.cache/graph-qg8-12-20261101/embedded-final/`, deux étapes
PASS, intégrité `recipe.py verify` VERIFIED. 905 assertions nommées (402
navigation avec Work strictement inchangé +503 édition/refus/récupération),
FR/EN, clair/sombre, largeurs320/390, écran bas et lecture SVG hors application.
Les 14 contrôles Version après installation s’ajoutent à ces 905. Contrôles de
syntaxe et i18n réussis, catalogue pertinent inchangé depuis le contrôle, diff
sans erreur de whitespace. Captures visuelles contrôlées : mobile, prérequis et
export, clair et sombre. Aucun fournisseur ou agent appelé ; racines isolées.

`candidate-final.json` désigne v0.1.1-dev.294, empreinte complète
`84a62c6d8f18387a5198db6bc2a80d6b075bb11c202176029b63fdca9f28c537`,
profil natif normal sans feature fixture et sources effectives archivées dans
`.cache/local-delivery/rust-byrhlo7_/build/profile/`.
`restart.stdout` prouve la sélection exacte préparée, zéro entrée compilée en
attente au redémarrage. `installed/result.json` confirme PID1200221, sept assets
protégés conformes au build et dialogue Version FR/EN : commit compilé réel,
lien Git, notes et modifications non commitées affichées, aucun POST sur le Root
réel. Aucun serveur tiers, mission ou maintenance Git interrompu ; la pause
sollicitée n’a pas été exécutée et n’était plus nécessaire après le build.

Revue : `review-final/source-review.json`, `result.json` et archive des références
lient les six entrées compilées de ce lot aux octets réellement livrés. La revue
incrémentale complète couvre 30 entrées mais son contrat GLOBAL reste BLOCKED :
24 producteurs Autorité/BLP parallèles restent sous leurs propriétaires et ne
sont pas qualifiés par cette auto-revue. La livraison UI locale ne constitue ni
leur acceptation, ni une conformité globale de la migration, ni une revue
indépendante. Les limites historiques batch/undo métier restent inchangées.

RETEX vérifié : les captures/mémorisations doivent utiliser le placement terminé,
pas l’état transitoire d’un pointeur. Une vue plein écran contient deux niveaux
de défilement, à conserver sous l’éditeur. Un champ search dans un formulaire
métier doit distinguer Entrée de l’enregistrement ; le garde avant gestionnaire
permet de reproduire sans mutation. Le parsererror de Chrome peut produire du
style bloqué par CSP : conserver la CSP et éviter cette génération pour refuser
les caractères XML. Un lien de téléchargement temporaire doit être nettoyé aussi
sur exception. Ces règles sont dans les commentaires et gardes permanents ; les
consignes natives embarquées et processus déjà lancés n’ont pas été modifiés.
Les preuves des échecs et candidats291–293 restent intactes. Source preview et
ancienne recette293 demeurent des preuves limitées à leurs propres octets.

Amélioration de méthode conservée : deux contentions Git/compilation ont allongé
la livraison ; le verrou a effectivement protégé les builds concurrents. Les
métadonnées natives compilées dans swarm-engine (`version_identity.rs`) changent
lors des livraisons UI ; mesurer leur effet sur la réutilisation des artefacts
avant toute optimisation. Propriétaire : outillage/maintenance ; état à étudier,
reprise sur chantier explicite de performance des builds avec témoin mesuré,
identité/version/forgery guards préservés. Aucun réglage Git, priorité machine,
politique de build ou contexte d’agent n’a été modifié pour fabriquer un PASS.

## Propositions QG13–QG17 — après Rust294

Demande : « tu as encore des quickwin ? ». Inspection du renderer et du contrat
Graph Studio actuels ; propositions uniquement, pas de nouveau lot exécuté.
Propriétaire proposé : Codex/UX graphe. Aucune modification produit, fonction,
API ou politique ; pas de rebuild/version nouvelle pour ce seul suivi.

| ID | Origine observée et bénéfice attendu | Critère observable proposé | État / condition de reprise |
|---|---|---|---|
| QG13 | Toolbar contient toutes les actions, avec plusieurs rangées sur largeur390 ; dégager davantage de surface de graphe | outils secondaires dans un contrôle nommé dépliable ; zoom/recherche/plein écran restent accessibles ; clavier, focus, FR/EN et mobile sans perte de commande | proposé ; prochain lot d’ergonomie, priorité1 |
| QG14 | Zoom affiche un pourcentage et deux boutons, sans retour direct à100% ; faciliter le retour à une échelle lisible | commande100% conserve le centre de la vue sous clamp ; boutons +/− désactivés aux bornes existantes, aucune borne modifiée | proposé ; même lot, priorité2 |
| QG15 | centerMatch avance uniquement et Entrée/Maj+Entrée suivent actuellement la même branche | Précédent et Maj+Entrée parcourent les correspondances à rebours avec boucle/focus/centrage ; recherche et placement conservés | proposé ; prochain lot de recherche |
| QG16 | undo consomme previousPositions puis le remet ànull, sans rétablissement | rétablir une fois le placement annulé, consommé/invalide après nouveau placement ; aucune donnée métier ni révision changée | proposé ; prochain lot de placement, distinct de l’historique métier G4 |
| QG17 | save persiste zoom/positions ; vue/orientation/scroll restent en mémoire de rendu et disparaissent au rechargement | restaurer orientation et zone du même travail après reload ; état invalide/refus storage explicites, autre travail indépendant, remise à zéro accessible ; plein écran reste une action explicite | proposé ; prochain lot de préférences locales |

Priorité proposée : QG13 puis QG14. Les constats source ne prouvent pas encore
le gain utilisateur ; chaque adoption nécessitera son témoin et sa recette.

## Lot QG13–QG17 adopté — « tu peux les mettre en place »

Codex seul, risque C présentation ; mêmes API/droits/données. QG13 outils
secondaires repliables (défaut mobile), commandes essentielles visibles et focus
restauré ; QG14 retour100% autour du centre et bornes existantes reflétées par les
boutons ; QG15 recherche précédente/Maj+Entrée ; QG16 rétablissement d’un seul
placement annulé, invalidé au prochain déplacement terminé ; QG17 orientation,
scroll canevas/studio et état des outils locaux par Work après rechargement,
sans plein écran automatique. Navigation réinitialisable, zoom/positions gardent
leurs commandes dédiées. Sources/propriétaires parallèles préservés.

Choix : details natif et mêmes contrôles plutôt qu’un second toolbar mobile ;
un seul undo/redo en mémoire plutôt qu’un historique métier. Préférence locale
versionnée distincte des coordonnées existantes : validation stricte, clamp
natif, diagnostic sûr sur JSON invalide ou stockage refusé. Les snapshots de
placement restent terminés ; un geste annulé ne détruit pas redo. Aucune politique
moteur, limite d’exécution, dépendance distante ou délégation ajoutée.

Preuves dans `.cache/graph-qg13-17-20261101/` : contre-exemple avant modification,
checks ciblés puis recette consolidée sur image immuable, documentation/commentaires
et traductions FR/EN, revue politiques/fonctions/erreurs liée aux octets, activation
via lanceur, assets protégés/Version et RETEX. Aucune autorisation de commit/push
implicite dans cette demande. Temps de build et contentions observés, sans arrêter
les autres processus. Ne pas réutiliser une ancienne demande de pause Git.
