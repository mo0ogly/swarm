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
