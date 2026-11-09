# Intégration du design validé — 8 novembre 2026

Demande : intégrer les maquettes de docs/design, préserver les autres travaux,
installer puis vérifier dans l’instance locale. Propriétaire : Codex seul ; APEX,
replan, verify-fix et revue locale. Aucun droit de publication ajouté.

## État initial et décision

HEAD c927b9266 ; serveur Rust dev.349, image bf19eb56… port géré 38434.
Travaux concurrents dans le moteur, swarm-design.css, swarm-shell.js,
version_history.json et docs/UI-DESIGN.md : préserver les ajouts existants.
Preuves originales : /media/fpizzi/Lexar/swarm-evidence/design-integration-20261108/.

Choix : adapter les composants et opérations publics actuels ; éviter une nouvelle
application alimentée de faux snapshots ou la copie du moteur de démonstration.
La vue Couloirs consomme Work + projection d’état reçue, sans inférer une phase
métier d’un libellé ni attribuer une autorisation à un état. La sécurité conserve
ses contrôles/API, ses refus et ses limites BLP. Les formulaires réels priment sur
les champs d’exemple qui n’ont pas de producteur. Aucune donnée de mission utilisée
comme fixture, aucun fournisseur métier lancé.

## Critères et séquence

| ID | Résultat observable | Vérification | État |
|---|---|---|---|
| UX1 | Cockpit : hiérarchie, palette, titres, états et sélecteur Graphe/Couloirs | vrai HTTP Rust, FR/EN, thèmes et tailles | Implémenté, vérifié sur fixture |
| UX2 | Couloirs : tâches réelles, filtres, sélection, dépendances et erreur/vides | recette isolée ; changer de mission conserve l’isolation | Implémenté, vérifié sur fixture |
| UX3 | Préparation : conversation/document accessibles, composition cohérente | données réelles isolées et contrôle des actions existantes | Implémenté, vérifié sur fixture |
| UX4 | Admin : présentation des réglages et aperçu existants, sans politique parallèle | recette existante et contrôle du dialogue | Implémenté, vérifié sur fixture |
| UX5 | Administration/Sécurité et Suivi/Audit accessibles, contrôles réels conservés | onglets, lecture/refus et absence d’effet implicite | Implémenté, vérifié sur fixture |
| UX6 | Installation réelle : binaire, ressources servies, Version, notes FR/EN | launcher supporté ; hash, version et HTTP protégé | Installé ; vérification authentifiée restante |
| UX7 | Travaux concurrents conservés ; revue politiques/fonctions/erreurs liée aux octets | inventaire avant/après et revue locale | Revue locale et conservation vérifiées |

Ordre : inventorier les producteurs → composant/vue/styles → contrôles ciblés et
docs FR/EN → build supporté → qualification de l’image et installation → HTTP/Version
local → bilan/RETEX. Un échec requiert diagnostic et nouvelle précondition ; pas
de répétition inchangée ni de limite globale de test choisie par commodité.

La preuve navigateur intégrée file:// a été refusée dans le travail antérieur ;
ne pas contourner ce refus. Les recettes de produit HTTP préexistantes sur des
fixtures autorisées restent distinctes de l’ouverture de ces fichiers locaux.
Le contrôle visuel du service dans le navigateur outillé reste soumis à sa politique.

## Livraison et preuves finales

Image activée par `./swarm.sh restart --prepared-build` : **v0.1.1-dev.352**,
SHA-256 `a7fee3210bd38420ca0e346c45f45cc4f511538be44412e60caeb54fc3a23145`,
PID observé 1464773, commit embarqué c927b9266 et modifications non commitées.
Le lanceur a signalé zéro entrée compilée postérieure non installée. Aucun
commit/push ni redémarrage du démon d’autorité effectué dans cet incrément.

| Preuve | Résultat et portée |
|---|---|
| Ancien binaire349 sur fixture CLI réelle | Échec attendu : sélecteur Couloirs absent ; original conservé |
| `recipe352` | PASS et intégrité VERIFIED :205 assertions navigateur, vrai HTTP Rust, même image qualifiée que la livraison |
| `existing352` | PASS :416 assertions des parcours existants, assets natifs et commandes publiques de fixture |
| `preparation351` | PASS :105 assertions, dont CAS, refus, réponses longues et reprise ; producteurs de préparation/CSS inchangés en352 |
| `limits351` | PASS et intégrité VERIFIED :4 contextes langue/thème, valeurs exactes, aperçu/application/historique ; mêmes producteurs en352 |
| i18n et diff | Contrôle du catalogue/généré PASS ; syntaxe JS/Python et diff --check PASS |
| Image réellement active | SHA attendu, profil sans fixtures, version et notes compilées/servies vérifiés |
| Données de l’instance | Empreintes des lectures CLI publiques work list et engine-settings show identiques avant/après ; aucun contenu privé publié |

Emplacement de collecte Unix : `.cache/design-integration-20261108/`. Les recettes
locales ont été utilisées après le refus de remplacement atomique sur la clé FAT.
Les fichiers originaux et le premier refus sont sous
`/media/fpizzi/Lexar/swarm-evidence/design-integration-20261108/` ; l’archive finale
et son index conservent les droits/métadonnées nécessaires sans supprimer ces
originaux. Revue sémantique locale liée aux23 empreintes dans source-review.json.
Elle ne constitue pas une revue indépendante ou une conformité BLP globale.

### Limite exacte de vérification sur l’instance réelle

`swarm.sh authority status` a établi la façade utilisateur **38433**, le moteur
interne **38434** et le mode authority. Le cookie de livraison est autorisé pour
les seules sondes native-version/native-capabilities ; les lectures métier et
les nouveaux fichiers privés refusent 403 comme prévu par authorize_request et
launcher_probe (`service.rs`). La feuille partagée de connexion est publique,
ce qui expliquait son 200 initial sans prouver une session métier.

Une tentative de collecteur live s’est arrêtée au premier asset privé refusé :
ce n’est pas une preuve de fichier mal servi ni une vérification complète de
l’interface. La façade a été ouverte dans l’onglet3 du navigateur intégré ; elle
montre le vrai formulaire Compte/Mot de passe, sans saisie ni changement d’accès.
La fenêtre Version FR/EN et le readback des nouveaux assets sur ce Root ne sont
**pas déclarés vérifiés**. Le même binaire sert les bons octets en fixture ; cela
ne remplace pas cette observation sur la session réelle.

Reprise : Fabrice se connecte normalement sur http://127.0.0.1:38433/ ; Codex
vérifie alors le DOM et les ressources observées de cette session, la fenêtre
Version, les deux vues, les liens Sécurité/Audit et les diagnostics, en lecture
seule. Ne pas élargir launcher_probe, désactiver authority, transférer le mot de
passe ou extraire une session d’un autre navigateur pour rendre le contrôle vert.

### Conservation et RETEX

Les56 fichiers initialement modifiés/non suivis ont été indexés et archivés avant
édition. Les ajouts de l’autre agent dans UI-DESIGN, swarm-design.css et le registre
de versions restent présents octet pour octet (ou structure JSON identique après
retrait de notre seule entrée). swarm-shell.js et les sources Rust préexistantes
sont conservés. Deux documents de l’autre agent ont continué d’évoluer ; ils n’ont
été ni remis à l’état initial ni écrits par cet incrément. Les sources Rust
préexistantes examinées correspondent aussi à celles du binaire349, sans prétendre
requalifier leur travail métier.

Échecs conservés : absence attendue349 ; CSS de préparation manquante en350
(en-tête reconstruit séparément) ; interruption de lectures par le banc351
(corrigée en attendant la fin du réseau, sans allowlist de diagnostics) ; collecte
FAT refusée ; mauvais argv initial de verify (corrigé avec --evidence) ; refus403
du collecteur sans session métier. Chaque reprise a modifié une précondition.

Apprentissages : suivre les en-têtes réellement produits et tous les assets
embarqués ; séparer code/fixture/image installée/session utilisateur ; faire une
recette sur un filesystem possédant les opérations Unix requises puis archiver ;
attendre les lectures avant navigation ; afficher les libellés reçus plutôt que
les codes d’état techniques. Les maquettes restent des références, leurs exemples
n’accordent aucune capacité. Le besoin de vérification authentifiée UX6 reste ouvert.


## Reprise authentifiée — 8 novembre, session utilisateur

Méthode replan : session normale disponible sur la façade38433, sans extraction
de cookie ni extension du probe. Observation réelle352 : Graphe/Couloirs,
recherche R5, dépendances, onglets Sécurité, clavier/Échap et vérificateur vide
consultés. Le Work historique refuse son suivi détaillé avec
`mission-status-planning-unported` ; la notice de lecture enregistrée demeure.
Ce refus antérieur ne fait pas partie du raccordement de présentation et aucune
mission n’est lancée pour le contourner.

Défauts observés : les états enregistrés restaient en anglais technique ; le lien
Limites perdait son fragment puis rencontrait le garde des destinations non
raccordées ; Sécurité ne consommait pas le thème entrant ; Graph n’était pas traduit.
Corrections générales dans les consommateurs existants : traduction des trois
états canoniques, préservation du fragment et des seuls paramètres langue/thème,
exclusion des deux vrais liens Admin du garde, lecture du thème enum côté Trust.
Aucun droit, jeton, politique ou budget ajouté. Les reproductions sont conservées
dans `.cache/design-integration-20261108/authenticated-ui/`.

Les candidats353–355 ne sont pas déclarés installés. Le candidat356 comprend ces
corrections et la révision source effective `b4e07cb26066cffdb20b36ac2fc9c60f758cb23e`
(dépôt avancé concurremment ; aucune réécriture de son historique par cet incrément).
La non-régression existante356 a exécuté416 assertions : PASS.

Diagnostic du banc : `recipe356`, `recipe356-observable`, `recipe356-dom` et
`recipe356-requests` gardent leurs interruptions originales, sans PASS.
La destination Limites et sa lecture projet ont été observées correctes ;
l’attente réseau générique pouvait rester sans progression. La recette observe
maintenant les fins de requêtes et les états DOM explicites, sans watchdog global.
Un relevé périodique de diagnostic n’est ni délai produit ni condition d’arrêt.
La cause complète de chaque attente reste soumise aux nouveaux relevés ; ne pas
attribuer ces interruptions à une panne moteur sans preuve.


### Vérification finale authentifiée et installation356

**UX6 : vérifié dans la session normale de Fabrice sur38433.** Le besoin convenu
est clos pour l’intégration visuelle et les parcours examinés, avec les limites
métier ci-dessous. Le refus historique352 reste conservé, pas effacé par ce résultat.

- Image active : `v0.1.1-dev.356`, PID1589809, SHA256
  `ef14867a6c91393ae1c75c08d2602d481bcf250fc80cbd143f7efeca75cc1220`.
  Activation par `./swarm.sh restart --prepared-build SHA256 --no-open` ; zéro
  entrée compilée différente du candidat examiné lors de l’activation.
- Profil : `VERIFIED_DEFAULT_NATIVE_PROFILE`, core features vides, sans fixture,
  dépendances effectives stables. Cela ne requalifie pas les changements métier
  préexistants de l’autre agent.
- `recipe356-mutation` :233 assertions design PASS, `recipe.py verify` VERIFIED.
  `existing356` :416 assertions du cockpit existant PASS. i18n PASS, syntaxe JS
  contrôlée et `git diff --check` sans erreur. Les contrôles de préparation105 et
  limites FR/EN des351 restent historiques ; les nouveaux233 contrôles couvrent
  aussi les surfaces de préparation et le vrai dialogue de limites en356.
- Session réelle : Version FR/EN, numéro356, lien du commit exact `b4e07cb26066…`,
  notice de modifications non commitées ; aucun commit/tag publié par cet incrément.
  Limites : navigation réelle, cinq champs, Enregistrer désactivé sans aperçu,
  Échap/retour du focus après relecture. Aucun réglage changé.
- Couloirs : sept tâches enregistrées, états Acceptée/Bloquée/À faire traduits,
  aucun débordement observé à la largeur effective du navigateur. Le refus
  `mission-status-planning-unported` demeure visible pour ce Work ancien.
- Sécurité : thème clair transmis après bascule, cinq onglets, avertissement BLP
  intact, flèche clavier et Échap/focus vérifiés. Audit : contrôles hors dialogue,
  preuve vide donnant `Unexpected end of JSON input — Vérifier l’état avant reprise`,
  verdict vide. Aucun jeton saisi ni aperçu/application de sécurité déclenché.
-43 assets effectivement servis à cette session (23 scripts,19 styles et logo)
  comparés aux octets de l’archive de sources effectives du build356 : identiques.
  Les téléchargements se font par le navigateur connecté ; aucune extraction de
  cookie ou extension d’autorité. Aucun warn/error console capturé sur les contrôles.
- Empreintes des lectures publiques CLI Work/réglages identiques avant/après356.
  Les10 fichiers Rust/shell préexistants inventoriés sont inchangés. Les deux jetons
  CSS et les trois entrées de nouveautés sont nos seuls ajouts dans ces fichiers
  partagés ; retrait de ces seuls ajouts restitue leurs originaux (JSON équivalent).
  UI-DESIGN conserve son préfixe original. Quatre documents d’autrui ont évolué
  concurremment, dont les deux plans AGENT-CONTROL-POCS ; aucun n’a été réécrit par nous.

Preuves locales : `.cache/design-integration-20261108/authenticated-ui/final356.json`,
`assets356.json`, capture `securite356.jpg`, `source-review356.json`, recettes et
snapshots avant/après. Copie USB et commande de restauration : index local
`usb-index356.json`. L’onglet final est laissé au cockpit FR/clair, sans mission
sélectionnée ; la sélection locale ne constitue aucune mutation de Work.

RETEX : les attentes fondées sur les frames de rendu ne constituent pas une bonne
preuve de disponibilité d’un contrôle hors écran après navigation vers un fragment.
Observer les mutations du DOM pour l’activation des contrôles, conserver les refus
visibles et attendre la relecture avant départ. Le banc garde ses diagnostics sans
watchdog ; le timer de diagnostic ne maintient plus à lui seul un échec en vie.
La collecte `recipe356-readiness` avait vérifié trop tôt une pause entre requêtes
sérialisées : assertion négative conservée, remplacée par une vraie condition de
chargement, puis vérifiée sur les quatre contextes. Les tentatives précédentes
restent des preuves incomplètes, jamais comptées comme PASS.

Limites de clôture : aucune revue indépendante ni qualification BLP globale.
Le suivi détaillé historique non porté appartient à la migration moteur ; reprise
par son propriétaire avec un cas de migration dédié, sans lancer ni modifier cette
mission pour les besoins de la présentation. Amélioration à conserver : lorsque
aucun Work n’est sélectionné, la notice générique de lecture partielle reste
visible en Couloirs ; elle n’affiche aucune ancienne fiche. Priorité UX faible,
propriétaire frontend, critère futur : notice spécifique à l’absence de sélection.
Ce point ne bloque pas les parcours observés et n’est pas déclaré corrigé ici.


### Publication Git demandée par Fabrice

Commit du seul incrément design sur `main`, sans nouvelle branche, parent
`b4e07cb26066cffdb20b36ac2fc9c60f758cb23e`. Index Git isolé ; checkout courant,
index partagé conservé pour tous les autres fichiers, changements d’autrui inchangés. Les versions vérifiées356 de
SwarmWorkbench et cockpit-native-read sont retenues : les ajouts de badges
ultérieurs de l’autre agent restent locaux. Dans les trois fichiers initialement
partagés, seuls nos deux jetons CSS, trois notes FR/EN et lien documentaire sont
inclus ; les anciens ajouts locaux de l’autre agent restent exclus. Les preuves
233/416 portent sur le build356 incluant le contexte local de cette date ; elles
ne requalifient pas ce contexte exclu ni l’image d’un futur build de cette branche.
La publication ne change pas le binaire local déjà vérifié.

Consigne utilisateur relue dans le chat de coordination : séquencer sur main sans
nouvelle branche. Protection GitHub relue : PR obligatoire, checks/install exigés,
enforce_admins actif. Aucun contournement ; le push peut être refusé, auquel cas
ce commit reste local et une décision sur le parcours PR sera nécessaire.
