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


## JS-UI-372-01/02 — reprise bornée du 9 novembre

Attribution du coordinateur : Swarm07 est le seul mutateur UI ; root consolide
les index partagés, Gi conserve la session humaine. Aucun Git, provider, mission,
branche, worktree ou sous-agent dans ce lot. Service courant372/PID1880842,
SHA4977c71b2f5da940cec84fcb45cd438d34b10034b5f2f4c08f6f08e7d05a0e99.
Les preuves356/368 ne sont pas réutilisées pour qualifier372.

01 : reçu Gi ui-003, Version contenu556/604 EN,556/601 FR à viewport624.
02 : observation EN sombre vers FR relisant état ; défaut encore à reproduire.
Sources examinées : swarm-design.css ligne114 et293–304 (retours de lignes,
listes) ; cockpit-native.js bouton thème (DOM/liens sans URL) ; i18n.js sélecteur
(recharge URL+lang) ; native_admin.rs (thème cockpit consommé depuis URL, défaut
état) ; runtime-health.js (notes en LI textContent). Contrats et état propriétaire :
CSS commune, langue/thème de présentation, renderer non modifié ; les autres dirty
moteur sont hors correction. Pas de build/redémarrage en parallèle autorisé ici.

Plan : témoin372 isolé, mêmes notes et transitions,4combinaisons ×624/390 ; corriger
seulement une cause reproduite, rejouer avec overlays explicitement marqués des
sources ciblées, puis recette permanente/profile et remise exacte au coordinateur
avant compilation/livraison mêlant d’autres dirty. Vérifier contenu intégral,
focus/Échap/réouverture, reload/langue/thème et entrée de thème invalide.
Preuves neuves locales `.cache/ui-version-20261109` ; Lexar480KiB libre, aucun
nettoyage ni écriture de nouvelles preuves sur ce support.

Premier banc interrompu : recette copiée du serveur historique Go attendait une
URLsession que Rust ne produit pas. Replan : lecture du vrai contrat Server dans
rust_native_http_cli.py ; banc corrigé sur web-bound JSON + session du seul Root
fixture. Aucun secret humain lu/transmis. L’essai initial reste incomplet, jamais
PASS. Le serveur372 actif n’a pas été arrêté.


### Diagnostic et correction source vérifiés — image non livrée

Témoin `.cache/ui-version-20261109/baseline372-protocol/result.json` : les huit
contextes (FR/EN × thèmes ×624/390) débordent dans Version, sans débordement du
document. BF16b LI long sans overflow-wrap. La transition EN sombre→FR et le
rechargement échouent aussi ; bouton sombre témoin passant. Cause02 : le bouton
modifiait le DOM, mais pas l’URL ; i18n recharge l’URL ; native_admin consomme son
thème et retombe sur état si absent. Le défaut est maintenant reproduit, pas inféré.

Correction01 : même règle partagée des paragraphes/listes/descriptions de dialogue,
`overflow-wrap:anywhere`, aucun texte coupé ou caché, aucune largeur augmentée.
Correction02 : toggleTheme documenté, enum existant vers DOM+URL replaceState,
conservation des autres paramètres et des liens Admin. Aucun localStorage de thème
ni règle moteur ajouté. Shell/i18n/renderer natif restent inchangés.

Replay final source, overlays explicitement marqués des seuls deux fichiers :
`source-final/result.json`,56/56 PASS. Comparaison intégrale des LI avec tous les
items FR/EN du producteur Version, non simplement recherche d’un token ; focus
au titre, Échap/opener, réouverture, reload, transition inverse et thème invalide.
Recette compilée maintenue : version-dialog-preferences.json (sans overlay).
Ces résultats sources ne prouvent pas l’image installée. Les fixtures sont privées,
aucun fournisseur/mission/POST ; processus de ces essais remis après fermeture.

Blocage de livraison explicite :8entrées effectives différentes de372, dont cinq
hors correction. pending-build-inputs.json et foreign-inputs.patch remis. Quatre
sources de l’inspecteur/placeholder sont dans3eeeaef8e ; message annonce14checks,
reçu original non établi par cette session. i18n test échoue : la clé générée
« Agent natif — exécution et mandat courants requis » est absente du catalogue
source. i18n-mismatch.json conserve le discriminant. Aucun retrait/reset de ce
travail ; root coordonne gel/qualification et génération avant décision de build.
Ne pas répéter le test inchangé ni installer un composite sur la base des overlays.

Revue politiques/fonctions/erreurs/doc liée aux SHA : HANDOFF.json. Source stable,
aucun build/redémarrage/Git ;372 reste active. Conditions de reprise : décision
root sur les cinq entrées étrangères et le catalogue, gel des sources, build
pris en charge/profil qualifié, recette sur l’image exacte, puis remise Gi pour
contrôle humain protégé et rapprochement des octets. Aucun login38434 demandé.
RETEX intermédiaire : suivre le producteur ET le consommateur des préférences ;
mesurer le contenu interne en plus du document ; préférer une vraie condition DOM
aux frames hors écran ; les doublons source n’établissent jamais une livraison.


### Prérequis complémentaires autorisés par root

Le coordinateur autorise explicitement le contrôle existant de l’inspecteur et
le rétablissement canonique de la traduction d’Oryh, sans changer son consommateur.
`workbench-summary.stdout` :14/14 PASS nommés. Revue follow/mount, placeholder et
stageCSS de3eeea : id connu dans modèle validé requis, sélection de présentation
seule ; HTML textuel conservé, erreur DOM propagée ; dimensions/breakpoint1024 sont
un choix de géométrie UI, pas une politique d’exécution. Cette revue/test double
reste distincte d’une preuve de l’image composite ou d’une revue indépendante.

Catalogue : phrase utilisée trust-native.js:51, traduction anglaise déjà existante
dans i18n-en.js rétablie exactement dans locales/en.json, autres entrées dont
placeholder conservées. Générateur existant repassé ; i18n-after.stdout PASS.
Aucune nouvelle capacité native ni droit n’est déduite de cette phrase UI.

Root a relevé un cas d’erreur dans toggleTheme : le DOM pouvait changer avant un
refus de replaceState. Tests marqués d’injection locale dans history-refusal-before
et history-refusal-observed montrent cinq échecs (ancien DOM/URL, visibilité,
cause et reprise). Ces essais ne sont pas PASS. Nouvelle correction générale :
enregistrer URL d’abord ; en refus conserver ancien DOM/URL, afficher une phrase
FR/EN sûre, propager web-native-theme-update-refused avec cause originale. Le test
restaure sa méthode historique originale puis vérifie une nouvelle action +reload.
Un catalogue source neuf est nécessaire à ce diagnostic traduit ; overlays finaux
explicitement trois assets (CSS, cockpit-native.js, i18n-en.js), toujours non livrés.

HANDOFF.json initial complet est conservé ; sa duplication des notes a tronqué
une lecture du coordinateur. Replan de la transmission : un reçu compact séparé
avec chemins, SHA, comptes et noms de défauts, sans corps de notes. Ne pas recycler
le test source pour fabriquer une preuve runtime ; prochaine image unique après
inventaire final/gel coordonné et recette compilée, puis contrôle Gi.


### Gel source remis avant compilation

`READY.json` est le reçu compact (chemins/SHA/comptes, pas les corps des notes).
source-ready-load/result.json :63/63 PASS, overlays marqués trois assets seulement.
14/14 workbench-summary et i18n-after PASS ; cinq tests shell Échap conservés.
Les63 incluent cause/refus/reprise, console et requêtes échouées. Aucun échec
silencieusement filtré : source-refusal-fixed conserve4GETnative-version annulés
par navigation prématurée ; source-network-settled garde ReferenceError (lang
observé avant scripts chargés). Correction du banc : événement load de navigation
attendu avant composants, puis promesse réelle Version attendue avant départ.
Les diagnostics antérieurs ne prouvent pas une panne moteur. Le premier témoin de
refus avait effectivement terminé en FAIL ; ne pas qualifier sa simple progression
lente de blocage produit ou affirmer qu’une interruption l’a produit.

Les fichiers de production et tests du reçu sont maintenant stables ; aucun
processus des fixtures restantes (processes-final.json). Root décide gel et
créneau avant build unique ; pas de nouveau build, restart ou demande de login.
La phase installation/réexamen runtime/RETEX final reste ouverte.


### Candidat373 qualifié — redémarrage non exécuté

Créneau unique build attribué explicitement par root après revalidation de READY
sans écart et absence de cargo/rustc concurrents. Préflight : huit écarts connus
et aucun nouveau depuis372, inventaire complet dans build-preflight.json.
`./swarm.sh build` a produit une seule image373, sans redémarrage du serveur372.
Sources gelées durant compilation et tests ; aucun Git, fournisseur ou mission.

Image : v0.1.1-dev.373, SHA256
`9ba81aa1ca94baa743c063957a1e0cfe90ed744588385922b7589bde977330d4`.
Profil `.cache/local-delivery/rust-cpvqgcfk/build/profile/result.json` :
VERIFIED_DEFAULT_NATIVE_PROFILE, core features vides, core non-test, aucune feature
fixture, dépendances effectives stables. Archive effective SHA256
`5f716959f9f4112533b026ae31d7e1b4d2fc042aac3cbb4188546e848410780a`.
Le record candidat et tous les effective_inputs ont été rapprochés des sources
après les vérifications : zéro écart. Pas de nouvelle compilation des14 contrôles
inspecteur inchangés ; preuves exactes réutilisées dans leur portée source.

Recette permanente version-dialog-preferences sur cette image, SANS OVERLAY :
63/63 PASS, recipe.py verify VERIFIED, originaux dans compiled-recipe373.
Une exception JS est intentionnellement injectée/typée/cause vérifiée pour le
refus historique ; aucune autre erreur JS, aucun warn/error console ni requête
échouée, aucun POST dans la fixture. Le résultat ne prouve pas une session humaine.
Six assets protégés servis par un autre Root privé du même binaire comparés aux
membres de l’archive : CSS partagé, cockpit-native.js, catalogueEN, workbenchJS/CSS
et graphJS. Tous identiques ; empreinte /proc de l’enfant égale à celle du candidat
avant/après lectures. Version/commit/modified et l’intégralité des notes FR/EN
servis correspondent au record et au JSON compilé. served373/result.json conserve
les chemins, statuts et empreintes ; pas de cookie humain exporté.

Remise compacte : `.cache/ui-version-20261109/READY-CANDIDATE373.json`.
372/PID1880842 et son SHA4977… sont encore actifs après le build. Redémarrage non
exécuté : root coordonne la suite et la recette Gi dans sa session normale.
Commande préparée (pas lancée) : swarm.sh restart --prepared-build SHA373 --no-open.
RETEX final après activation et observation humaine reste ouvert ; ne pas annoncer
« installé/corrigé sur instance » à partir de la recette de fixture.


### Activation373 — remise à root avant navigateur Gi

Créneau unique restart attribué explicitement après rapprochement par root des
hashes du candidat, profil, archive, recette et octets servis. Préflight renouvelé :
record373 identique, image candidate exacte, source sans entrée compilée nouvelle,
instance372/PID1880842 identifiée. Cinq Work lus via CLI publique avant arrêt,
corps non conservés ; seulement compte et empreintes. La commande préparée avec
le SHA complet a sélectionné373 sans nouvelle compilation et annoncé zéro entrée
plus récente non installée. Aucun autre service interrompu, aucune mission lancée.

Image active vérifiée par /proc de l’exécutable et API native-version :
v0.1.1-dev.373, PID2336522, SHA256
9ba81aa1ca94baa743c063957a1e0cfe90ed744588385922b7589bde977330d4,
commit3eeeaef8ea62e2165613d75b2c7714708885db80, modified=true,
builddate2026-11-09T21:57:09Z. Status running/ready true. Cinq Work après activation,
même SHA de leur liste publique et de engine-settings show ; aucun corps privé
écrit dans les preuves. ACTIVATION373.json est le reçu compact de cette remise ;
activation-before373 et activation-after373 gardent les snapshots originaux.

Défaut du collecteur de reçu conservé dans activation-receipt-contract-failure :
comparaison erronée de l’objet binary API à l’enveloppe build incluant schema_version.
Les champs communs étaient déjà exacts ; reprise par comparaison explicite des
quatre champs et de provenance injected. Aucun redémarrage/test répété pour cela.
Une namespace de métadonnées différente n’est pas une divergence d’exécutable.

RETEX local : UI01 venait d’une règle de prose qui oubliait les LI contenant des
identifiants longs ; UI02 du producteur de préférence DOM sans mise à jour de
l’URL que consommaient langue/reload. Correction aux contrats communs et preuve
avant/après conservée. En erreur, commettre l’URL avant affichage et conserver
l’ancien état/diagnostic/cause ; le test de refus/reprise est permanent. Attendre
les lectures Version avant navigation évite d’attribuer les annulations du banc
au produit. Le partage d’un workspace exige un gel explicite et le rapprochement
des octets étrangers, pas leur retrait ni une requalification implicite par un
commit. Reçus compacts et liens SHA transmettent les preuves sans tronquer les
notes ; les originaux restent intacts et locaux car Lexar presque pleine.

La vérification authentifiée n’est PAS close par cette activation. Prochaine remise
root→Gi : recharger373 dans sa session normale38433, vérifier Version FR/EN × thèmes,
624 et largeur mobile pertinente, contenu interne sans coupure/défilement injustifié,
Échap/focus/réouverture ; EN sombre→FR puis reload garde sombre ; restaurer les
préférences initiales, aucune mission. Rapprocher les ressources protégées observées
avec l’archive373 si demandé. Ce chat ne redemande pas de connexion/API et n’accède
pas à la session distincte de Gi. Les preuves fixture63/profil/exe/API/CLI sont
établies ; observation humaine, acceptation globale et conformité WCAG restent
hors de leur portée. Root coordonne ce dernier témoin avant clôture des deux constats.


## Clôture bornée JS-UI-372-01/02 — observation authentifiée373

**Les deux constats sont clos dans le périmètre ci-dessous.** Reçu final de Gi
relu le9novembre : `.cache/ui-version-20261109/gi-final373/result.json`, statut
PASS_BOUNDED_AUTHENTICATED_373. SHA256
6dba6b0d0db97bdadb880ceb76a49452b46ec718ce256c0f9a8e51c3e48500a5.
Reçu de comparaison lié : comparison.json, SHA256
377a12e4024dd4248290c09e4e734978f539029efb8df982702f9a847fbdb52c.
Liaison image373/SHA9ba81aa1… vers activation, profil/archive et ressources conservée.

Gi est l’exécutant navigateur ; root compare les corps téléchargés et les octets
de l’archive ; Swarm07 finalise ici documentation/RETEX, sans nouvelle exécution.
Huit cas uniques FR/EN × état/sombre ×624/390 : version373, commit/date et disclosure,
modal/titre et huit fermetures Échap avec retour au déclencheur visible. À624,
contenu556/556 ; à390,322/322. Le LI long BF16b est intégral,434/434 puis200/200,
hauteur client égale à scrollHeight, overflow-wrap:anywhere et aucun line-clamp.
Égalité du texte complet par rapport au DOM initial de chaque langue conservée ;
ce n’est pas une nouvelle revue sémantique de tout le catalogue historique.

**UI01 : corrigé et vérifié sur l’instance.** Le débordement interne mesuré372
556/604 EN et556/601 FR ne se reproduit plus dans ces huit cas, sans masquer ni
couper le texte ni élargir artificiellement le dialogue. **UI02 : corrigé et vérifié
sur l’instance.** Deux transitions EN sombre→FR avec vrai reload restent sombres
(624 et390) ; les transitions claires et reload gardent état. Préférences initiales
FR/état/624×684 restaurées et overrideviewport réinitialisé ; aucune mission choisie.

Six ressources authentifiées ciblées comparées par root à l’archive373 : six
concordances exactes, hash de comparaison revérifié ici. L’API d’export ne fournit
ni statutHTTP ni en-têtes : aucun200 n’est déduit de cette remise. Ce contrôle
n’est pas un rejeu des22assets de372 ni une requalification368/372. Console warn/error
avant/après vide dans la collecte limitée100 ; univers réseau non exhaustivement
mesuré. Captures dans le transcript Gi, aucun fichierimage nouveau annoncé.

RETEX final du lot : deux causes différentes (retour à la ligne des LI ; préférence
DOM non reportée dans l’URL consommée par langue/reload) ont eu chacune un témoin
négatif puis la même vérification après correction. Le cas d’erreur replaceState
conserve ancien état et cause, avec diagnostic FR/EN et reprise ; il reste une
injection de fixture, jamais présentée comme incident de la session humaine.
63assertions compilées sansoverlay,14du composant inspecteur et catalogue cohérent,
un seul build373 et une activation du prepared-build exact précèdent la remise
humaine. Les cinq Work et réglages publics sont conservés selon leurs empreintes.
Les originaux des échecs du banc, du catalogue et du collecteur restent distincts
des preuves produit. Les règles/contrats FR/EN et la recette permanente transmettent
le retour d’expérience ; cela ne garantit pas l’absence d’un autre défaut futur.

Observation séparée à conserver : à390, le libellé français de fermeture apparaît
« Ferme/r » sur deux lignes ; texte accessible et bouton fonctionnel. **Cause et
régression non établies**, aucun nouveau correctif ni test lancé ici. Propriétaire
Swarm00/UI ; condition future : triage explicitement attribué, comparaison à une
baseline390 pertinente et lecture des règles de bouton. Cette observation ne
rouvre pas implicitement les deux contrats vérifiés et ne vaut pas acceptation
globale ni conformité WCAG/BLP. Root maintient le suivi commun.

Reçu compact de clôture : `.cache/ui-version-20261109/CLOSURE373.json`. Cette
finalisation modifie seulement ce plan et produit le reçu ; aucun test/build/
restart/Git supplémentaire, aucune action métier, aucun transfert de secret.


### Publication isolée des corrections UI01/02

La demande de commit/push porte sur ce seul lot. Commit préparé sur la base publique
origin/main, sans les commits Conduite locaux ultérieurs ni les dirty des autres
agents. Aucun checkout, reset ou réécriture de main. Le rétablissement du libellé
natif d’Oryh est conservé localement avec son consommateur non publié : le commit
retient uniquement le nouveau diagnostic de refus de thème dans les catalogues.
Les preuves373 incluent le contexte local qualifié de cette date ; elles ne
requalifient pas implicitement une future image du commit isolé. GitHub exige PR
et checks/install ; la publication reste conditionnée au parcours autorisé,
sans contourner la protection ni créer une branche sans autorité applicable.
