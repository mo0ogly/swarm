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

Replan initial du banc : une recherche de ponctuation apparaissait deux fois et
l’édition a refusé ; la commande suivante avait continué. Cette tentative before
est arrêtée, pas comptée comme témoin. Édition et exécution sont ensuite séparées.
`before-zoom100` prouve le vrai contrôle absent après témoin de graphe valide.

Preview : nouveaux parcours PASS sur les premières variantes, puis attente sur
l’ancien point de molette après resize. La connexion CDP de diagnostic appliquait
son viewport par défaut et a perturbé l’observation (conservée dans preview/).
Aucune cause produit déduite de cet état ; arrêt propre. Le banc attend désormais
la disposition après resize, contrôle explicitement un point de fond visible
avant molette/pan et affirme le résultat après rendu au lieu d’attendre uniquement
le succès. Futurs diagnostics CDP : defaultViewport:null.

Revue de source avant build : les erreurs locales conservent une famille sûre
(SecurityError/QuotaExceededError/SyntaxError ou erreur de validation), sans valeur
ni message privés. Le record de scroll est de taille constante et ne clone pas
la carte des positions à chaque événement ; seul le snapshot complet de rerender
copie les coordonnées. Aucun nouveau timer ou plafond opérateur.

## QG13–QG17 livrés — Rust296

| Critère | Résultat local observable |
|---|---|
| QG13 Barre compacte | PASS : essentiels visibles, outils repliés sur mobile, ouverture/fermeture/Escape/focus, rendu320/390 inspecté clair/sombre |
| QG14 Zoom100% | PASS : centre conservé sous clamp, bornes15%/200% inchangées et boutons désactivés à leurs limites |
| QG15 Recherche arrière | PASS : Précédent et Maj+Entrée, premier résultat arrière =dernier, boucle/focus/effacement ; les deux sens désactivés sans résultat |
| QG16 Rétablir placement | PASS : déplacement et rangement rétablis, témoin consommé et invalidé par nouveau placement terminé, conservé sur annulation ; aucune mutation métier |
| QG17 Navigation persistante | PASS : orientation/zone/état des outils retrouvés après reload, pas de plein écran automatique, JSON invalide/offset négatif/champ fullscreen refusés visiblement ; get/set/remove refusés avec famille sûre, reprise et reset de la seule navigation |

Preuve native : `.cache/graph-qg13-17-20261101/embedded/`, deux étapes PASS,
`recipe.py verify` VERIFIED ; 1137 assertions (518 navigation Work strictement
identique +619 édition/refus/reprise), quatre variantes FR/EN×clair/sombre, mobile,
éditeur et pertes de réponse historiques. Source preview reste un overlay, distinct
de cette preuve. Syntaxe/i18n/diff vérifiés, captures mobile inspectées. Aucun
fournisseur/agent appelé, aucun Root de mission réelle utilisé comme fixture.

`build.stdout` : candidat déjà préparé296 réutilisé par le lanceur, sans rebuild
inutile ; empreintes des cinq entrées UI comparées à l’inventaire effectif et
identiques. `candidate.json` et `.cache/local-delivery/rust-0ukzkap_/build/profile/`
conservent profil natif normal, absence de features fixtures et archive source.
Image immuable SHA256
e4a802c4dc4d32d90d036169604168c64db15196695d692885369b748686ca72.
`restart.stdout` : sélection de cette empreinte exacte, zéro entrée compilée
modifiée en attente à cette activation. `installed/result.json` : PID1460574,
v0.1.1-dev.296, sept assets protégés identiques au build et 14 assertions Version
FR/EN (commit/lien réels, notes, état non commité). Aucun POST sur le Root réel
pendant cette vérification. Pas de pause Git ni d’interruption de processus tiers.

Revue liée aux octets : `review/source-review.json`, `review/result.json` et
archive des références ; cinq fichiers compilés du lot examinés sur politiques,
fonctions, erreurs, docs. Sur douze entrées modifiées depuis la baseline295, sept
producteurs Autorité/BLP parallèles restent ouverts sous leurs propriétaires :
le statut GLOBAL est BLOCKED, sans déduire leur qualification de cette livraison
UI locale. Pas de revue indépendante/acceptation moteur globale revendiquée.
Les commits parallèles ff2ca179a/b4cbfa225 ont intégré une partie des fichiers
pendant le travail ; historique conservé, aucune attribution ou réécriture ajoutée.

RETEX : stabiliser un changement responsive avant de mesurer sa géométrie,
vérifier un point de fond réellement visible avant un geste, et diagnostiquer
sans laisser CDP imposer son viewport par défaut. Une préférence invalide doit
rester visible après les deux rendus du chargement ; ne jamais persister le plein
écran ni assimiler erreur à cache vide valide. Garder les snapshots de scroll en
taille constante et les coordonnées provisoires hors stockage. Ces contrats
persistent dans helpers/commentaires, docs FR/EN et recette versionnée ; ils ne
modifient pas les instructions embarquées ou les contextes d’agents déjà lancés.
La page navigateur existante doit être rechargée pour recevoir les nouveaux JS.

Commit/push demandé : une écriture concurrente dans l’index partagé a ajouté le
lot BLP au premier commit local. Push suspendu ; objet original conservé dans
le reflog et son SHA dans `.cache/graph-qg13-17-20261101/git-recovery.json`.
Reconstruction du seul lot graphe avec un index temporaire privé et déplacement
atomique conditionnel de la branche ; fichiers et index partagés préservés.
Prévention : sélectionner les octets avec un index privé pour les commits dans
un checkout partagé ; ne pas déduire le contenu du commit d’un ancien statut.


## Ergonomie analyste — reprise APEX du1novembre2026

Demande : améliorer lecture et manipulation des graphes pour les analystes.
Premier incrément autorisé issu de la priorité proposée : A1 isoler une branche
(prérequis/dépendants transitifs, sans frères), A2 montrer tous les chemins
orientés entre deux tâches (absence explicite), A3 sélection multiple et
placement groupé pointeur/clavier avec annulation/rétablissement existants.
Codex seul ; aucune délégation, aucun fournisseur, aucune écriture métier pour
ces trois parcours. Duplication/aperçu/batch/undo métier restent des propositions
à qualifier séparément ; clarification asynchrone envoyée sur leur engagement.

Choix : masquer localement les éléments hors périmètre en conservant le placement,
plutôt que recalculer Dagre et perdre les repères spatiaux. Mini-carte/recherche
suivent la vue ; retour explicite au graphe complet. Une branche inclut sa racine,
tous ses ancêtres et descendants ; un chemin est orienté prérequis→tâche, union
des chemins, sans énumération exponentielle. Aucun résultat ne prétend un chemin.
Sélection explicite par bouton de carte, accessible au toucher et clavier ; les
poignées déplacent ensemble les cartes sélectionnées avec une translation commune
clampée à l’origine. Les gestes restent provisoires jusqu’à leur fin ; annulation
restaure tout le groupe, no-op conserve undo/redo. Sélection et filtres sont
éphémères au nouveau rendu ; zoom/coordonnées gardent leur contrat existant.

Livrables : JS/CSS, traductions/aide FR/EN, docs et recette navigateur permanente,
témoins normaux/refus/no-op/reprise, thèmes/mobile/console, installation locale par
lanceur après preuve sur l’image exacte. Revue locale politiques/fonctions/erreurs
liée aux octets ; aucune acceptation indépendante ni conformité globale déduite.
Preuves nouvelles sous `.cache/graph-analyst-20261101/` ; échecs préservés.

Témoin v296 : contrôle natif valide puis `fr-etat-analyst-control` FAIL attendu
(commandes absentes), originaux before/. Preview1 : filtre/compte corrects, mais
le banc supposait getClientRects vide pour un groupe SVG display:none. Un cas
Chromium minimal prouve un rectangle encore présent ; correction du banc vers
style calculé/aria-hidden et Tab excluant les cartes masquées. Aucun contournement
produit ni seuil ajouté. Reprise avec cette précondition de mesure corrigée.


Preview2 a ensuite révélé un défaut réel : Tab pouvait encore atteindre une
commande SVG masquée. setProjection retire explicitement tabindex des cartes et
liens masqués et restaure0 au retour complet ; preview3 ciblé PASS. Aucune carte
masquée ne doit rester dans le parcours clavier. Contrôle désormais permanent.
Inspection mobile : panneau initial trop haut ; trois sections exclusives natives,
fermeture après filtre/activation de sélection et statut hors panneau. Nouvelle
preuve compact/ :161assertions, zéro diagnostic inattendu, quatre variantes et
320/390px ; captures clair320/sombre390 inspectées. Test ajouté ensuite pour la
remise à zéro filtre/sélection au rerender, à vérifier sur le candidat embarqué.
Clarification optionnelle sans réponse : livraison du premier incrément priorisé
A1/A2/A3 ; aucune adoption implicite des quatre propositions métier restantes.

Replan build298 : compilation moteur plus lente que la précédente observée ;
mesures build-pressure.json conservent CPU/threads et pression mémoire/IO système.
Aucun échec de compilation observé ; pas de relance identique, seuil ajouté ou
processus tiers arrêté. Serveur296 préservé pendant qualification du candidat.


Recette embarquée initiale : navigation PASS654assertions, Work strictement
inchangé ; édition fr-clair PASS puis fr-sombre FAIL sur compteur «2/3». Cause
banc : les langues suivantes observent les tâches réellement créées par la
première, donc le total n’est plus3. Capture des IDs courants avant filtrage et
comparaison au total réel ; retour complet/rerender alignés sur le même témoin.
Produit inchangé, image298 inchangée. L’échec original et son intégrité VERIFIED
sont conservés dans embedded/. Une première édition a refusé sur contexte ancien
(click devenu helper) ; aucune étape dépendante lancée avant relecture/correction.


## Ergonomie analyste A1–A3 livrée — Rust298

| Critère | Preuve locale |
|---|---|
| A1 Branche | PASS : racine/ancêtres/dépendants transitifs, frères exclus, retour complet, compteur réel, recherche/mini-carte/clavier filtrés |
| A2 Chemins | PASS : direction prérequis→tâche, union du diamant multi-sauts, zéro chemin conserve la vue, extrémités identiques et IDs prototype traités |
| A3 Groupe | PASS : clic/Entrée/Espace en mode sélection, texte/état accessible/motif, poignées clavier/pointeur, translation commune clampée, annulation complète, undo/redo, carte non sélectionnée indépendante |
| Espace de travail | PASS : trois sections exclusives, fermeture après application, Escape/focus ; retour filtre/sélection au rerender ; FR/EN×clair/sombre,1440/1024/760/390/320 |

`embedded2/` PASS, integrity VERIFIED :654 assertions navigation (Work inchangé),
755 édition réelle (création/liens/refus/perte de réponse/reprise persistée).
`responsive/` PASS, integrity VERIFIED :145 assertions, aucun POST et Work
inchangé. Total1554 assertions navigateur. Les recettes permanentes sont
`rust-graph-workbench.json` et `rust-graph-analyst-responsive.json` ; les sources
responsive sont `tests/rust_graph_analyst_responsive.py/.cjs`. Aucun watchdog,
fournisseur ou agent ; aucune donnée de mission réelle employée comme fixture.
Échecs embedded/ et previews conservés avec leur état d’origine, pas écrasés.

Build normal `.cache/local-delivery/rust-ch511cy4/build/profile/` :
VERIFIED_DEFAULT_NATIVE_PROFILE, sans features fixture, inventaire/archive des
entrées effectives. Candidat298 immuable SHA256
f5a154da4555962647106954274d09ce9160464383d849ba33782c700d033df9.
`candidate.json` et correspondance des cinq entrées UI conservés. `restart.stdout`
confirme la sélection de cette image ; il observait1 entrée postérieure non
installée. Observation suivante `pending-sources.json` :3 entrées de confidentialité
moteur parallèles plus récentes, non installées. Ne pas déduire la livraison de
ces sources du présent lot ; propriétaire : lot confidentialité/BLP. Reprise : leur
qualification et leur propre livraison par le parcours pris en charge.

`installed/result.json` PASS : PID1838935, version298, SHA exact du processus,
sept assets protégés correspondant au build et14 contrôles Version FR/EN
(notes, lien de commit réel, mention des modifications non commitées). La recette
n’effectue aucun POST sur le Root réel. La page existante doit être rechargée.

`review/source-review.json`, result et références originales archivées : couverture
PASS, cinq entrées UI examinées sur quatre axes sur neuf entrées compilées
modifiées depuis l’image296. Les quatre entrées Authority restent OPEN sous leur
propriétaire ; GLOBAL BLOCKED, aucune conformité ou acceptation indépendante
revendiquée. Les anciens problèmes ouverts ne sont pas clos par cette comparaison.
Aucun Rust modifié par le lot UI ; pas de compilation Rustdoc applicable.

RETEX vérifié : masquer un groupe SVG ne suffit pas pour conclure à son absence
au clavier, retirer ses tab stops explicitement. Ne pas utiliser getClientRects
comme témoin unique de masquage SVG. Les fixtures d’édition progressent réellement
entre variantes : comparer les compteurs au témoin observé, pas à une valeur
initiale. Dégager le graphe après un choix et limiter les panneaux ouverts à un
seul usage ; garde d’exclusivité et largeurs désormais permanente. Ces apprentissages
sont transmis par code/commentaires, docs FR/EN et recettes versionnées ; aucun
contexte d’agent déjà lancé n’est mis à jour ou assimilé à une réception.

Ce premier incrément termine A1/A2/A3. Duplication, aperçu métier, brouillons
multi-tâches et annulation métier restent des propositions distinctes à qualifier,
non réalisées et non présentées comme livrées. Aucun commit/push pour cet incrément.


## Suite métier B1–B4 adoptée — « fais la suite »

Reprise APEX : B1 duplication explicite d’une définition (sans preuves/statut/
réservations), B2 aperçu moteur des tâches et liens avant application, B3 brouillon
multi-tâches et application atomique, B4 annulation métier compensatrice du dernier
lot sans réécriture d’historique. Autorisation de développement/installation locale,
pas de commit/push. Codex seul, pas de délégation ni fournisseur. Risque A données :
preuves de refus/intégrité/reprise et revue nécessaires ; auto-revue distincte de
revue indépendante, aucune acceptation globale inventée.

Choix comparés : boucle de task.add/update (rejetée, risque de demi-application)
contre commande graph.batch dans la transaction WorkEngine existante (retenue).
DTO natif strict séparé des graph_drafts Go, tables/formats historiques intacts.
Réutiliser schéma/événement/revision/replay/Root reservation/contrôle d’activité et
validation de définition. Les autorisations graph_draft_authorizations existantes
restent applicables (prepare/apply/scopes), pas de contournement par nouvelle route.
Les Work hiérarchiques gardent leur refus de création manuelle ; pas de nouvelle
révision de contrat de mission dans ce lot. Aucune nouvelle politique de volume,
délai, tentative ou exécution. Taille d’entrée et critères viennent des sources
Admin actuelles. Aperçu sans mutation ; digest lie contenu, révision, candidat et
autorisation. Apply revalide dans la même transaction puis publie une seule révision.
Undo dérive l’inverse depuis l’événement exact du dernier lot enregistré ; aucun
snapshot arbitraire fourni par le navigateur n’est restauré. Les preuves invalidées
ne sont jamais réactivées par une annulation de définition.

Plan : contrat/guards/tests moteur → CLI/API → brouillon/duplication/aperçu/undo UX
FR/EN → tests publics isolés et refus/reprise → revue liée aux octets → build normal,
image exacte, installation/version/assets → RETEX. Preuves sous
`.cache/graph-business-20261101/`. Échecs conservés. Les fichiers Authority/BLP
parallèles et leurs preuves restent préservés ; rechercher les producteurs réels
avant lecture plutôt que supposer un nom de module (lectures initiales mal ciblées
corrigées via inventaire/symboles, aucune action dépendante lancée sur ces erreurs).

Suite métier : huit tests moteur ciblés PASS (profil test sans feature fixture).
Premier essai HTTP : trois échecs avant route, clé de fixture hors format de
LocalWebAuthority. Remplacement par la clé témoin déjà valide du banc service,
sans modifier la validation produit ; échec initial conservé sous http-tests.*.
Aucun résultat HTTP positif déduit de cette précondition refusée.


Suite métier, contrats12 tests PASS (8 moteur +3HTTP +1CLI). Le premier parcours
navigateur a franchi les quatre variantes complètes, puis FAIL sur diagnostics
inattendus ; pas déclaré PASS. L’écriture du rapport étant placée après cette
assertion, son contenu n’était pas conservé : le banc retient désormais les
objets diagnostics même en échec. Revue de code : risque de pending fantôme en
mémoire après quota local refusé avant POST ; ajout d’un contre-exemple ciblé
fr-clair (fermer/rouvrir, stockage encore refusé), avant correction générale.
Preuves initiales iteration/ conservées, binaire d’itération séparé du profil
livré. Captures de l’aperçu développées ajoutées pour lecture mobile réelle.


Contre-exemple before-ghost/ : refus quota n’envoie rien, puis fermeture/rouverture
révèle un pending mémoire fantôme — FAIL attendu. Correction : effacer ce marqueur
sur refus avant envoi, persister également avant reprise, lier le cache mémoire à
la même session opaque et toujours valider/déchiffrer le disque avant ce cache.
Une corruption ne peut plus être masquée par des propositions mémoire récentes.
Ghost-fix/ a ensuite échoué avant ce parcours sur l’ancien formulaire de duplication
encore présent : handoff désormais après l’événement close et assertion du banc
sur formulaire réellement ouvert ; observateur permanent interdit les identités
de champs dupliquées pendant cette transition. Pas de faux PASS de ces tentatives.


Targeted2/ : tous parcours fonctionnels fr-clair PASS, y compris contre-exemple
pending et corruption cachée par mémoire ; contrôle final diagnostics FAIL.
Détail conservé : requestfailed /api/v1/action, tandis que le serveur a262 réponses
complètes et zéro disconnected_replies/connection_failures. Hypothèse discriminante :
le POST concurrent du banc lisait seulement les headers puis abandonnait son corps.
La reprise attend maintenant son JSON et l’acquittement réel (id/révision), conserve
failure.errorText et phase pour toute nouvelle anomalie. Aucun allowlist ajouté.
Capture mobile : fermer se coupait en deux lignes ; bouton non rétrécissable et
aperçu scrollé dans le corps pour inspecter réellement les valeurs avant/après.


Targeted3/ PASS : parcours fr-clair complet, y compris60+ assertions métier,
refus/reprise/storage/compensation, aucun diagnostic inattendu. Le corps JSON du
POST concurrent est désormais consommé et son id/révision affirmés ; l’interruption
navigateur n’est plus observée, sans allowlist ni modification serveur. Cela étaye
la cause de mesure du banc. Source UI/contrats figés pour build et recette finale.
Les panneaux de comparaison avant/après sont inspectés sur captures mobiles ;
le bouton Fermer ne doit pas se couper, header reste visible et corps défilable.

Observation avant build final : une livraison parallèle a activé299 (PID1983729,
75ad879f…). Notre session n’a pas redémarré le service pour ce lot. Baseline exacte
299 retenue dans graph-business…/baseline.json, sans qualifier les autres travaux
par leur présence dans l’instance locale. Le journal298 précédent reste intact.


Image300 : recette métier13 contrats +206assertions navigateur PASS/VERIFIED,
recettes régression graphe et largeurs PASS. Ces preuves couvrent leurs scénarios
et n’incluent pas encore le nouveau garde d’intention de création trouvé en
revue : l’ajout d’un ID existant ne doit jamais devenir une modification.
Contre-exemple natif300 lancé avant correction ; originaux conservés sous
before-create-intent/. Le candidat n’a pas été activé par cette session.

Qualification métier : la baseline299 intermédiaire a déjà embarqué des modules
GraphBatch pendant le travail partagé. La comparer seule masquerait donc les
nouveaux producteurs métier non encore revus. Conserver le record299 comme état
observé, mais utiliser l’inventaire exact298 (dernier lot graphe propre vérifié)
pour la revue complète298→candidat. Conserver les findings historiques ; aucun
changement de baseline ne constitue une acceptation de299 ou des lots parallèles.
Amélioration de livraison à conserver : associer la préparation d’un build partagé
à l’état de qualification de ses propriétaires, pour éviter de co-livrer des
sources en cours. Propriétaire : parcours de livraison locale/coordination racine ;
reprise : contrat inter-lots de sources figées et témoin de refus/reprise avant
activation. Non réalisée ici : conserver le développement direct choisi et ne
pas modifier le lanceur parallèle ni fabriquer des droits ou une conformité.


Before-create-intent/ FAIL attendu sur l’image native300 après témoin valide :
un ID existant ferme le formulaire au lieu de refuser la création. Correction
au contrat frontend : transmettre explicitement creating, refuser tout ID déjà
présent dans le plan ou le brouillon, conserver le before d’une proposition éditée
jusqu’au rebasage explicite. La validation serveur continue de distinguer ajout
(before:null) et modification (before observé), sans nouveau bypass. Le formulaire
conserve la saisie et affiche le motif ; busy/feedback couvre aussi le staging.
Les preuves300 précédentes restent réelles mais ne suffisent pas à cette garde.


Create-intent-fix/ PASS sur fr-clair avec overlay explicite du correctif UI :
IDs du plan et IDs déjà proposés refusés, formulaire/saisie/propositions conservés,
aucune écriture métier lors du staging ; reste du parcours complet et diagnostics
PASS. Ce témoin ne qualifie pas des ressources embarquées. Le noyau Rust ne change
pas depuis300 ; préparer une nouvelle image contenant ces octets UI puis relancer
le corpus public sur cette image. Les succès et refus300 restent conservés.


Reprise suivante : image301 qualifiée localement par les trois recettes métier,
graphe et responsive, non activée par cette session. Deux contre-exemples
supplémentaires sont conservés : before-rebase-refusal/ montre un refus de
rebasage dont l’aperçu reste actif et un ID interprété comme code diagnostic ;
before-two-tabs/ montre deux ateliers écrivains simultanés. Correction générale :
invalider l’aperçu avant toute relecture, afficher les IDs comme texte littéral,
et acquérir un Web Lock par origine/Work/session jusqu’à fermeture. Les éditions
ne sont acceptées qu’après sauvegarde locale ; suppression du cache mémoire
qui pouvait diverger du disque. Ce verrou technique n’est pas une politique
de concurrence des agents ; aucun timer, plafond ou retry ajouté.

Local-writer-fix/ PASS en fr-clair avec overlay explicitement déclaré : deuxième
onglet refusé sans effacement, premier propriétaire conservé, rechargement et
reprise idempotente, refus de rebasage et corruption conservée au réaccès.
Aucune mutation native dans la projection synthétique de conflit (étiquetée
dans son artefact) ; les autres événements passent réellement par CLI/HTTP.
FR/EN et documentation actualisées. Préparation d’une nouvelle image embarquée ;
indépendance de revue et installation restent à établir. Propriétaire : Codex.


Candidat302 `02e4662938e85a50e631c9a1dfd3d27916d0357358da9eca370bb90234802a6e` :
profil natif par défaut qualifié, sans test-fixtures. Trois recettes finales PASS
et intégrité VERIFIED : métier13tests Rust +246assertions navigateur ; navigation/
édition1409assertions ; responsive145assertions. Total1800assertions navigateur,
FR/EN, clair/sombre, largeurs1440/1024/760/390/320. Captures avant/après fr-clair320
et en-sombre390 inspectées : titre/fermeture lisibles, valeurs en panneaux empilés,
texte hostile littéral, corps défilable. Aucune erreur navigateur inattendue.
Résumé exact : `.cache/graph-business-20261101/final302-summary.json`.

Revue locale des politiques/fonctions/erreurs/docs et empreintes :
[GRAPH-BUSINESS-REVIEW-20261101.md](GRAPH-BUSINESS-REVIEW-20261101.md).
Collecteur298→302 : couverture26/26 PASS, contrat BLOCKED attendu avec68axes
non attestés globalement (fichiers parallèles ou ajouts partagés d’un autre
propriétaire), pas68défauts Graph. Déclaration et résultat source-review*.json
conservés ; aucun changement de baseline ou succès Graph ne clôt ces limites.
Les onze entrées de livraison Graph contrôlées sont identiques au candidat302.
Revue indépendante risqueA toujours ouverte : le choix utilisateur Codex seul
n’accorde aucune délégation. Demander uniquement ce droit pour une revue
lecture seule du candidat figé et ses interactions ; aucune écriture ni fournisseur.
Activation locale/version/assets à vérifier ensuite par le parcours supporté.
Pas de commit/push, pas d’installation revendiquée.

RETEX intermédiaire vérifié : distinguer intention de création/modification,
invalider l’aperçu avant toute reprise (même refusée), garder les identifiants
hors du canal diagnostic et accepter un brouillon seulement après sauvegarde
sous exclusion entre onglets. Les quatre contre-exemples conservés deviennent
gardes permanentes du banc. Leçon pour livraison partagée : un build peut contenir
les ajouts d’un autre propriétaire avant sa qualification ; garder une baseline
propre et un registre par propriétaire, sans fabriquer une conformité globale.
B1–B4 implémentés et vérifiés localement ; acceptation indépendante et activation
restent ouvertes. Codex reste propriétaire de cette reprise.


Reprise utilisateur « gfo » : délégation de revue indépendante lecture seule
explicitement accordée. Agent graph_independent_review via code-reviewer ;
candidat302 et sources conservés figés pendant son examen. Identité active
toujours299 ; record préparé302 et tous inputs compilés inchangés au réaccès.
Revue : défaut confirmé de contrat complet GraphTaskDefinition contre Request
partiel de task.update. title/deliverable vides signifient « absent » pour le
validateur hérité ; preview renvoie pourtant les vides demandés, journalise un
après qui ne correspond pas au Work, compromet la compensation suivante.
Reproduction sur image302 via CLI public en Root isolé :
`.cache/graph-business-20261101/before-full-definition/result.json` : titre vide
prévisualisé mais titre enregistré inchangé en révision3, puis même cas livrable
en révision4. Owner Codex ; installation suspendue, preuves302 non supprimées.
Options : changer globalement task.update (rejeté, casserait son contrat partiel)
ou valider les champs complets à la frontière Graph et vérifier leur projection
appliquée (retenu). Correction puis nouvelle revue sur nouveau candidat requises.


Revue indépendante302 : request changes, un finding P2, aucun autre défaut
confirmé. Original et périmètre dans GRAPH-BUSINESS-INDEPENDENT-20261101.md.
Nouveau test complete_definitions_refuse_blank_fields_and_match_recorded_compensation
FAIL avant correction (titre vide accepté), originaux before-full-definition-test.*
conservés ; PASS après correction dans full-definition-fix.*. Couverture : témoin
valide, titre/livrable vide ou blanc, refus preview/apply sans changement octets
SQL/events/Work, correction valide avec aperçu=projection, compensation cohérente.
HTTP existant étendu : mêmes refus422 et raisons statiques, aucune révision ajoutée.

Correction dans graph_batch.rs : nonempty Go partagé pour les deux champs
complets avant adaptateur partiel ; TaskDefinitionError typée existante pour
422/public reason ; garde de projection appliquée==after, invariant interne
refusé500 en cas de dérive future. Aucun nouveau réglage ou plafond ; aucune
modification de task.update. Documentation contrat complet FR/EN actualisée.
Une seule entrée compilée diffère de302 (graph_batch.rs), JS/CSS/locales/notes
et chemins historiques inchangés : réutiliser les1409+145assertions302 pour leur
périmètre inchangé, distinguer leurs binaires. Rejouer toute recette métier sur
303 car le validateur complet et la compensation sont ses consommateurs réels.
Build303 en cours ; revue indépendante ciblée à renouveler sur son SHA exact.


## Clôture B1–B4 — Rust303

Revue indépendante303 approve, finding302 corrigé et contre-exemple conservé.
[Verdicts et limites](GRAPH-BUSINESS-INDEPENDENT-20261101.md).
`.cache/graph-business-20261101/final303/business` PASS et intégrité VERIFIED :
14tests nommés (10moteur/3HTTP/1CLI) et246assertions navigateur sur image303
embarquée. Les1409graphe+145responsive302 restent réutilisées pour leurs
producteurs identiques ; elles n’ont pas tourné de nouveau sur303.
Revue corrective source-review303*.json : baseline302→303 effective, unique
entrée graph_batch.rs, quatre axes complets, aucun finding ouvert pour ce
correctif. Ce contrôle ne remplace ni la baseline298 d’introduction ni ses
findings autres propriétaires : source-review*.json original et68axes non
attestés globalement restent conservés. La revue indépendante couvre tous
les ajouts Graph B1–B4 ; aucune conformité globale parallèle n’est déduite.

Installation par restart --prepared-build SHA complet --no-open, aucun input
compilé plus récent en attente. Instance locale PID2298105, version303, binaire
exécuté SHA2565f22359a9b0acfed477b0e2e95b80d814a81b08d885362f6ac060be8164f518a.
Preuves install303.*, installed303/result.json, installed303-business.json :
7assets protégés identiques au profil, version réelle et fenêtre Version FR/EN
14assertions, commit réel/état modifié, notes local_changes complètes identiques
et capacités graph_batch preview/apply/undo-last-batch. Vérifications installées
en lecture seule, aucun POST ou Root de mission utilisé comme fixture.

| Critère | Résultat observable | Preuve |
| --- | --- | --- |
| B1 | Clone avec nouvelle identité, définition seule, original conservé | business303 + CLI readback |
| B2 | Aperçu sans mutation, champs/liens complets, cycles et vides refusés | 10tests moteur/3HTTP + navigateur303 |
| B3 | Un lot, un événement/révision ; quota refuse avant POST, réponse perdue reprise sans doublon | business303 + reprise après redémarrage des fixtures |
| B4 | Compensation dérivée du dernier lot, historique conservé, refus après autre événement | moteur/CLI/HTTP/navigateur303 |
| UX | FR/EN,2thèmes,5largeurs,focus/fermeture,aucune erreur inattendue | business303, producteurs302 réutilisés |
| Livraison | Binaire303 réel,7assets,Version et notes/capacités concordants | installed303* |

RETEX final : la revue indépendante a trouvé un défaut hors du corpus initial,
malgré1800assertions navigateur vertes302. Une définition complète ne doit pas
consommer sans adaptation un validateur de patch ; valider les champs requis
et vérifier la projection réellement produite évite faux aperçu, événement
mensonger et compensation impossible. Le garde permanent échoue avantfix et
passe après ; les refus422 restent exploitables sans affaiblir task.update.
Les leçons précédentes (intention créer/modifier, aperçu invalidé au refus,
ID littéral, sauvegarde sous verrou) restent des assertions du banc maintenu.

B1–B4 terminés pour les plans manuels et installés localement. Les missions
hiérarchiques gardent leur parcours de révision de contrat, aucun undo général
ni restauration de preuves n’est ajouté. RETEX et transmission au prochain
contexte par PLAN.md + docs produit FR/EN + tests/recette versionnés ; aucun
contexte d’agent natif en cours n’est prétendu réécrit. Commit/push non réalisés.
## Prévention paire-protocole livrée (clôture de l'item ouvert)

L'item « prévention paire-protocole avant remplacement moteur » est livré et
vérifié localement le 31 octobre 2026. Garde dans `tools/swarm_local.py`
(`require_matching_assertion_pair`, branchée dans `main()` pour `start` et
`restart`) : avant tout `stop()`, refus (exit 2, « Opération refusée : Paire
d’assertion incompatible : … ») quand le mode d'accès sauvegardé du Root est
`authority` et que le démon installé et le moteur candidat ne parlent pas le
même protocole d'assertion. Discriminant observé : octets du domaine embarqué
(`swarm-authority-assertion-v2\0`/`v1`, `rust/crates/swarm-authority/src/assertion.rs`) ;
vérifié sur les binaires réels du jour — `bin/swarm` v2 et démon installé
`/usr/local/lib/swarm-authority/swarm-authority` v2 (paire courante compatible,
activation sans effet pour l'opérateur). Passages : owner-key, Root jamais
initialisé, démon absent, protocoles égaux ; refus fail-closed : paire
divergente (dans les deux sens), démon sans marqueur reconnu, réglages
illisibles. Le refus ne nomme jamais la ligne de commande (argument opener =
credential) et nomme la reprise `./swarm.sh authority install`.

Preuve : reproduiseur `tests/test_local_pair_guard.py` (12 tests ; échec
initial FAIL avant fix conservé — Root fixture créé hors umask 0o077 refusé
« path » par le moteur, correction fidèle au parcours réel) + recette
`tools/verification/recipes/local-pair-guard.json` PASS (12 tests, épinglée
empreinte `41f00bf9…` v312, preuve `.swarm-fixtures/recipe-pair-guard-result.json/`).
Suites voisines inchangées : `test_swarm_local.py` 14/14 (un échec intermédiaire
FAIL avant fix conservé — fixture `built` sans marqueur rendait la comparaison
d'empreintes d'`enable` légitimement refusante), `test_authority_refresh.py`
8/8. Aucune écriture /tmp. Docs lanceur FR/EN mises à jour dans le même
incrément (contrat de la garde, passages, limites : démon servi d'ailleurs hors
de la garde, échec fermé à l'exécution).

Reste ouvert (inchangé) : la preuve du parcours authentifié public (session
humaine du démon) et l'inspection visuelle graphe restent bloquées sans session
opérateur ; la garde observe le chemin d'installation par défaut uniquement.
