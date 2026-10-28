# IHM — logo et lisibilité des fenêtres

## Dernière livraison — détail de tâche simplifié

**Installé et observé sur le serveur réel en `v0.1.1-dev.5`.** Le panneau
demandé par l’utilisateur est rouvert : une zone d’action, blocage Git expliqué,
original repliable, libellés en gras et tâches liées nommées. Les quatre
combinaisons FR/EN et thèmes passent aux cinq largeurs prévues.
Preuves : [`task-reading/`](../screenshots/window-identity-20261017/task-reading/).
Le défaut Git de la tentative métier reste à diagnostiquer/réparer séparément ;
aucune relance métier ni acceptation n’est effectuée par cet incrément.

## Reprise demandée — numéro et redémarrage

**État final : installé et vérifié sur le serveur géré en `v0.1.1-dev.3`,
`http://127.0.0.1:38433/`.** Le binaire canonique et le processus actif utilisent
le même artefact. [Modale réelle](../screenshots/window-identity-20261017/local-delivery/version-live.png)
et [preuve de conservation](../screenshots/window-identity-20261017/local-delivery/installation-result.json).

Après la livraison locale, l’utilisateur autorise explicitement la relance et
le changement de numéro. Le serveur géré observé est le PID 2186583 sur
`127.0.0.1:38433`, binaire `bin/swarm`, version `devel` construite le 15 octobre.
Le lanceur confirme son identité et son état prêt. Les cinq travaux contiennent
trente agents, tous terminés, interrompus ou échoués ; aucun agent actif.
Le watcher de mission indépendant et le serveur de preuves sur un autre port
restent hors de cette relance.

Décision : construire `bin/swarm-ui-v0.1.1-ui.1` avec `SWARM_VERSION` par le
build canonique, vérifier sur racine isolée, puis remplacer uniquement le
serveur géré en conservant adresse, données et authentification. Cette
préversion locale distingue la livraison sans inventer de release publiée.
Le mécanisme existant ne fait pas d’incrément automatique : sans version explicite,
il prend le tag SemVer exact de HEAD ou `devel`. La procédure FR/EN précise la
numérotation des livraisons, la vérification après relance et le choix persistant
du binaire. État au démarrage de cette reprise : compilation et effet réel à vérifier.

Preuves de reprise conservées dans `/tmp/swarm-ui-restart-20261017`.

Précision suivante de l’utilisateur : ce parcours appartient à `swarm.sh` et doit
être une règle, avec commits et évolutions dans la modale. La construction manuelle
`v0.1.1-ui.1` est donc un témoin, pas la livraison finale. Aucun serveur utilisateur
n’a encore été arrêté à ce stade. Plan révisé :

| Exigence | Observable | État final |
| --- | --- | --- |
| REL-1 | Le lanceur prépare le binaire source avant arrêt, numérote les entrées modifiées et réutilise le numéro si elles sont identiques ; un échec garde l’ancien serveur | PASS : cinq tests ciblés, quatre contrôles de cycle de vie, builds réels identiques puis modifiés |
| REL-2 | Modale FR/EN : évolutions du binaire local, commit réel cliquable et historique publié distinct | PASS : dix contrôles navigateur, neuf tests Go et modale du serveur réel |
| REL-3 | Règle commune persistante et transmise aux futurs contextes ; documentation FR/EN | PASS : AGENTS + contrat embarqué ; douze méthodes et cinq tests/sous-tests de rôles |
| REL-4 | Serveur géré relancé à la même adresse, numéro/ressources vérifiés par HTTP et navigateur, données conservées | PASS : cinq missions, trente-cinq tâches, trente agents et deux pauses conservés |

Le premier parcours Version est incomplet : le bouton se trouvait dans le groupe
Apparence replié, et un focus programmatique sur un contrôle masqué n’ouvre pas la
modale. La recette ouvre maintenant le groupe réel avant l’action clavier. Elle
n’attend plus un silence réseau global sur un cockpit à rafraîchissement périodique.
Les premières preuves sont conservées, sans les qualifier de PASS.

REL-1 : le premier `swarm.sh build` prépare `v0.1.1-dev.1` et vérifie les notes
par le vrai serveur sur racine isolée ; le PID utilisateur reste 2186583.
Un second build, avec la même source et une concurrence de compilation différente,
réutilise exactement la version et l’artefact. Les contrôles source couvrent aussi
les fichiers Go à la racine, les embeds et les octets des overlays, avec témoin
valide avant chaque substitution. Quatre tests de cycle de vie existants passent.

Replan de contraste REL-2 : le nouveau lien du commit apparaît dans une section
transparente. La recette héritée mesurait cette transparence comme du noir au lieu
du fond effectif du dialogue. Elle compose maintenant les fonds des ancêtres et
conserve les couleurs et le ratio calculés. Aucun jeton de palette ni seuil AA
n’est assoupli ; l’échec original reste dans `automatic-version-ui/failure.txt`.

Replan REL-3 : le contrôle des contextes embarqués refuse planner/subplanner
(32453/32456 octets pour une borne technique de 32000). La règle nouvelle était
dupliquée en détail dans AGENTS et le contrat commun, déjà proche de sa capacité.
La règle complète reste dans AGENTS ; son contenu utile embarqué et les formulations
redondantes du contrat sont compactés sans supprimer les droits, preuves, rôles
ou obligations. La borne n’est ni relevée ni contournée, aucun contexte tronqué.
Le témoin d’échec reste dans `workflow-controls.log`. Cette correction modifie
une ressource embarquée : elle exige une nouvelle version et une nouvelle relance.

Les contextes passent après compaction, sans changement de la borne de 32000
octets. Le lanceur détecte la modification du contrat embarqué et prépare
`v0.1.1-dev.2`. Un contrôle supplémentaire des types des notes locales ajoute un
refus explicite pour les objets/listes/langues mal formés ; cinq tests passent,
dont chaque injection est précédée d’un témoin valide. Le lanceur prépare alors
`v0.1.1-dev.3`. Ces trois numéros locaux suivent des entrées effectivement
différentes ; les reprises à entrées identiques ne consomment pas un numéro.

### Preuves de relance finale

Le serveur initial 2186583 a été remplacé par le parcours géré ; le processus
final est 2108275. Le SHA-256 installé est
`7b72f4ab3ae44bef730098aa9a99a00d50f84dbaada05164bf610d7d784a1455`.
Le [reçu de build](../screenshots/window-identity-20261017/local-delivery/build-receipt.json)
conserve les entrées réelles, y compris ressources.go, embeds, contrat et scripts
du lanceur. Aucun fichier de ces entrées n’a changé depuis la compilation finale.
Les processus distincts de watcher et de serveur de preuves restent présents.

Les tests Version (dix contrôles) et lecture dense (quatre combinaisons) utilisent
le premier candidat automatique `dev.1`. Le code web est identique dans `dev.3` ;
les différences ultérieures sont le contrat embarqué et le traitement des notes
mal formées par le lanceur. Le parcours final sur le vrai serveur vérifie
`dev.3`, les quatre évolutions, le commit cliquable, la consigne réelle inchangée,
les contrôles du journal et l’absence de requête de mutation, requête extérieure,
exception ou requête échouée. La capture durable ne contient que la modale,
aucun texte de mission ni identifiant de session.

Un contrôle d’inventaire via une nouvelle connexion CLI a rencontré
`SQLITE_BUSY`. L’opération n’a pas été répétée à l’identique ni sa protection
modifiée : le navigateur de vérification a été fermé et la lecture a utilisé
l’API authentifiée du Store déjà ouvert par le serveur. Les inventaires et les
trois indicateurs authorized/enabled/paused sont strictement conservés. Cette
lecture réussie ne prouve pas la correction générale de la contention CLI.

RETEX et suites conservées :

- REL-F1, propriétaire moteur/contextes : le contexte des planificateurs reste
  proche de la borne technique. Toute nouvelle consigne doit passer le contrôle
  des rôles ; reprendre une projection par responsabilité si la croissance
  continue, sans tronquer les droits/preuves ni relever la borne par commodité.
- REL-F2, propriétaire moteur/stockage : origine exacte du refus CLI concurrent
  non établie ; conserver le [refus original](../screenshots/window-identity-20261017/local-delivery/post-cli-original-failure.json).
  Reprise : reproduire avec deux Store et un writer sur une racine isolée,
  identifier le premier appel refusé, puis vérifier lecture/reprise sans perte
  et sans utiliser la base de mission comme fixture. L’inventaire post-relance
  requis est vérifié par l’API ; cet incident distinct reste ouvert.

La règle complète est persistante dans AGENTS ; son rappel est compilé dans le
contrat de workflow et les futurs contextes. Les processus déjà démarrés ne
reçoivent pas rétroactivement une nouvelle consigne. Aucun commit, push, tag,
publication ni appel fournisseur n’a été effectué pour cette livraison.

## Extension demandée — lecture des textes denses

Le 17 octobre, l’utilisateur précise que les instructions et les logs sont le
besoin principal, puis demande un traitement détaillé et sophistiqué. La première
livraison améliorait surtout les cadres : elle ne prouvait pas cette lecture.
Les résultats historiques ci-dessous restent conservés et distincts.

Responsable : Codex seul, même checkout ; aucune orchestration ni fournisseur IA.
Choix : composant de lecture partagé avec construction DOM sûre plutôt qu’une
injection HTML ou une copie de parseur par écran. Le moteur et les textes
enregistrés ne changent pas. La console Monaco existante conserve recherche,
sources et export ; sa présentation reçoit navigation et repères supplémentaires.

| Exigence | Observable et vérification | État |
| --- | --- | --- |
| DEN-1 | Instructions : titres, paragraphes, listes, citations/alertes explicites, tableaux et code ; texte source exact | PASS local : 8 cas parseur et 4 parcours navigateur |
| DEN-2 | Même lecteur dans consigne de tâche, révisions, profil commun et aperçu des saisies ; pas de modification envoyée par la lecture | PASS local : parcours réel tâche/révision/saisie ; projections de présentation commune/préparée |
| DEN-3 | Sommaire clavier, alternance lecture/source, copie avec retour explicite, protection HTML et URL | PASS local, succès/refus/reprise et source longue de 86 lignes |
| DEN-4 | Logs : navigation par événement, JSON mis en valeur, espacement confortable, retour à la ligne réglable ; originaux et recherche conservés | PASS local : 4 parcours denses et 6 contrôles Journaux |
| DEN-5 | FR/EN, deux thèmes, mobile et bureau, états vide/chargement/erreur, console masquée puis visible | PASS local : 5 largeurs, design, visibilité et instructions/RETEX |
| DEN-6 | Documentation FR/EN, preuves et manifeste courant ; limites d’installation et RETEX | PASS local : documentation, RETEX, 13 assets servis et manifeste sélectionné |

Séquence : tests du contrat et cas public initial → lecteur sûr et intégrations →
console et contrôles → recettes réelles isolées → documentation, intégrité et
RETEX. Aucune limite d’exécution ni politique opérateur n’est ajoutée.

### Livraison locale et preuves finales DEN

Preuves durables : [dossier dense-reading](../screenshots/window-identity-20261017/dense-reading/).
Le candidat immuable est `/tmp/swarm-dense-reading-20261017-release` ; son SHA-256
et les 13 assets servis comparés aux sources figurent dans
[source-integrity.json](../screenshots/window-identity-20261017/dense-reading/source-integrity.json).
Douze réponses correspondent octet pour octet ; pour `log-console.html`, seul
le remplacement du nonce CSP prévu par `web_terminal.go` est appliqué et contrôlé
contre l’en-tête CSP. La première comparaison naïve avait refusé ce remplacement ;
ce n’était ni une divergence d’asset ni une raison de supprimer la protection.

Les recettes suivantes se terminent avec un code 0 sur ce même candidat :

- `tests/text_reader_test.cjs` : huit cas nommés ; `npm run test:i18n` : catalogue,
  interpolation et refus d’un artefact généré périmé sans mutation.
- `tests/dense_text_ui.cjs` : quatre combinaisons FR/EN × etat/sombre ; 1440, 1024,
  760, 390 et 320 px. Source exacte, HTML/URL inactifs, sommaire et focus,
  défilement conservé après rafraîchissement, copie/refus/reprise, aperçu éditable,
  instruction commune/préparée, recherche réelle, navigation des logs, refus d’un
  message provenant de la mauvaise fenêtre, pause conservée après handshake et
  préférences conservées après rechargement réel du cadre. Aucun enregistrement
  ni départ d’agent ; révision lue par CLI inchangée.
- `tests/instruction_retex_ui.cjs` : quatre combinaisons ; rejet public, refus de
  révision périmée puis reprise, persistance, mobile et préparation RETEX sourcée.
- `tests/log_console_ui.cjs` : six groupes ; recherche, export original multiligne,
  refus réseau conservant la lecture, réponse tardive d’une autre mission écartée,
  source superviseur, vide et repli texte lorsque Monaco est indisponible.
- `tests/log_console_visibility_ui.cjs` : journal moteur réel, masquage, resize
  mobile/bureau et retour lisible sans erreur de mesure.
- `tests/design_system_ui.cjs` : dix groupes dont les douze destinations,
  treize paires de contraste par combinaison, chargement/refus/reprise,
  clavier, fermeture, focus et conservation des préférences.
- `tools/agent-workflows/check.py` : douze méthodes communes et cinq tests/sous-tests
  des contextes de rôles construits par le moteur.

Les diagnostics denses finaux n’observent aucune exception, réponse HTTP en erreur,
requête extérieure ou requête échouée inattendue. Les annulations `ERR_ABORTED`
strictement liées aux navigations complètes commandées restent enregistrées
séparément, avec leur phase ; aucune exception globale par endpoint n’est ajoutée.
Le premier collecteur ne couvrait pas les requêtes démarrées pendant la navigation,
ce qui avait classé une annulation normale de snapshot comme un refus inattendu.
L’original reste conservé ; le collecteur final distingue ces deux situations.

Intégrité : les deux générateurs ont actualisé le manifeste UI puis le manifeste
moteur sélectionné. `GOMAXPROCS=2 GOFLAGS=-p=1 go test ./internal/engine -run
'^TestGraphDeliveryD03QualificationManifest$' -count=1 -v -timeout=0` exécute bien
le test nommé et passe (code 0) ; sortie conservée dans
[manifest-verification.txt](../screenshots/window-identity-20261017/dense-reading/manifest-verification.txt).
Les seize fichiers JS vérifiés sont syntaxiquement valides ; `git diff --check`
passe. Les sources UI et recettes n’ont pas changé depuis les vérifications
navigateur. Les snapshots sont une preuve d’intégrité courante, jamais une
renouvellement d’acceptation historique.

Limites : vérification par l’auteur, pas de revue indépendante ni d’acceptation
native Swarm. Logs API, presse-papiers et présentation commune/préparée utilisent
des doubles explicitement étiquetés ; ils ne prouvent pas un lancement ni un appel
IA. Les tests réels de tâche, rejet, export et journal moteur sont distincts.
Le serveur utilisateur n’a été ni remplacé ni redémarré. Les autres changements
moteur du checkout restent des entrées observées, sans attribution à cet incrément.

### RETEX DEN et transmission

Le besoin initial exige deux niveaux : identité des fenêtres et lecture de leur
contenu. Le lecteur partagé traite la seconde couche sans réinterpréter du HTML
ni réécrire la source. Les intégrations à boutons demandent des sélecteurs d’action
explicites : sélectionner « le premier bouton » aurait cassé le rejet de consigne.
Une source longue révèle la perte de focus/défilement que les textes courts ne
montrent pas ; ces propriétés sont maintenant conservées et vérifiées.

Les recettes attendent le montage réel du graphe, les cycles de rendu après
resize et la nouvelle navigation du cadre. Une projection API stable évite de
faire concourir doubles et données réelles. Les navigateurs sont exécutés en
série après l’incident mémoire ; les watchdogs généraux hérités sans contrat ont
été désactivés pour ces vérifications, avec suivi de progression. Les délais de
la fixture de réponse tardive restent des injections contrôlées, distinctes des
politiques produit. Aucun plafond d’exécution n’a été modifié.

La documentation [FR](../DENSE-TEXT.md)/[EN](../en/DENSE-TEXT.md) contient les
contrôles, la syntaxe reconnue et un exemple réutilisable. Les prochains travaux
les retrouvent depuis UI-DESIGN, WINDOW-IDENTITY et LOG-CONSOLE ; les recettes et
le manifeste conservent les consommateurs et les preuves. Cette transmission
persistante ne modifie pas les contextes d’agents ou processus déjà démarrés.

Premières preuves DEN : les huit cas du parseur passent ; la nouvelle recette
publique échoue sur le binaire précédent avec « task instructions have a
structured reader ». La recette Journaux existante (six contrôles) et celle de
console masquée passent sur le premier candidat. La recette dense a ensuite
révélé une précondition de test : snapshot reçu avant montage du graphe. Elle
attend maintenant le nœud réellement affiché et ouvre explicitement Conduite,
sans dépendre de la destination mémorisée. Les sources ne changent pas pendant
une exécution de vérification.

Replan DEN, inspection visuelle : le compteur de logs affichait les placeholders
`{current}` et `{total}`. Le relais local de traduction ne transmettait pas les
valeurs d’interpolation. Correction du relais, assertion sur « 1 / 4 » FR/EN et
absence de réannonce du compteur inchangé. Les liens du lecteur utilisent aussi
le jeton partagé plutôt que le violet visité du navigateur. Les premières
captures restent dans `candidate2`, sans requalification rétroactive.

Replan environnement : pendant trois recettes navigateur simultanées, la machine
n’avait plus qu’environ 325 Mo libres, la swap n’avait plus que 103 Mo et des
compilations externes restaient actives. Les trois navigateurs de nos seules
fixtures ont été interrompus ; les recettes dense, instruction/RETEX et design
sont INCOMPLÈTES, même si des combinaisons FR passaient. Aucun autre processus
n’a été arrêté. Reprise conditionnée à la mémoire récupérée, un seul navigateur
à la fois et compilation locale à concurrence réduite ; aucune limite produit
ni protection d’exécution n’est changée. Les sources testées restent immuables
pendant chaque nouvelle exécution.

Replan de mesure responsive : une mesure du cadre à 320 px a encore observé la
largeur Monaco du palier précédent (356 px pour un viewport de 286 px). La recette
attend désormais le cycle de rendu du parent et du cadre après chaque changement
de largeur, comme la recette console existante. Ce changement de précondition ne
vaut pas correction CSS ; le débordement reste à revérifier sur le rendu stabilisé.

La mesure stabilisée passe aux cinq largeurs dans les quatre combinaisons. La
recette dense complète et la recette instructions/RETEX passent alors. L’examen
des captures EN révèle deux libellés manquants (Previous/Next), ajoutés au catalogue
plutôt que codés dans la vue. Une nouvelle fixture avec 60 lignes d’annexe reproduit
aussi la perte du focus de la source lors d’un rafraîchissement. Le lecteur attribue
maintenant des repères à tous ses éléments focalisables et capture le défilement
avant remplacement du DOM ; la restauration s’effectue sur le rendu monté.
La recette vérifie aussi les préférences de logs après rechargement du cadre.

La source longue conserve ensuite focus, position et alternance de modes dans les
cas FR. Le contrôle de persistance a exposé une course de recette : l’attente de
quatre options pouvait encore lire l’ancien document avant la navigation du
cadre. La recette attend maintenant l’événement de navigation réelle, puis les
options et la police dans le nouveau document. Aucun délai arbitraire ni retry
sur contexte détruit n’est ajouté ; l’original reste dans `release-dense/failure.json`.

## Besoin et plan

Demande du 17 octobre 2026 : logo sur toutes les fenêtres, esthétique et mise en
valeur des textes. Responsable : Codex, développement direct dans le checkout
canonique ; changements moteur et documentation préexistants conservés.

Méthodes : `replan`, puis incrément APEX borné. Aucune délégation ni appel IA.

| Exigence | Résultat observable | Vérification | État |
| --- | --- | --- | --- |
| FEN-1 | Logo canonique dans les dialogues statiques et créés à la demande | 48 groupes, 208 ouvertures ; aide et session supplémentaires | PASS local |
| FEN-2 | Logo dans les fenêtres secondaires, sur mobile et menu replié ; même icône d’onglet | Pages et largeurs 1440/1024/760/390/320 ; écran OIDC sans exécuter sa redirection | PASS local |
| FEN-3 | Titres, sous-titres, emphases et notices lisibles dans les deux thèmes | FR/EN ; 13 paires sémantiques par combinaison ; captures inspectées | PASS local |
| FEN-4 | Fermeture, clavier, focus, formulaires et erreurs préservés | Design, 15 aides contextuelles, méthode produit, terminal et journaux | PASS local |

Séquence : reproduire l’absence du logo → modifier le système graphique commun
et les points d’entrée → vérifier les parcours → actualiser la documentation et
le manifeste d’intégrité → examiner le diff et capitaliser les limites.

## Diagnostic et décisions

Le logo existe déjà dans le rail, la préparation, l’accès, les projets et la
mémoire. Les dialogues n’ont aucun repère de marque commun ; le rail est masqué
sur mobile et peut être replié sur ordinateur. Le terminal, les processus et les
actions de confiance manquent de logo. Plusieurs pages déclarent encore une
icône SVG de graphe après le logo PNG, donc l’icône réellement choisie diverge.

Deux options : ajouter du balisage dans chaque dialogue et chaque constructeur
JS, ou utiliser leurs titres d’en-tête communs. La seconde limite les oublis et
couvre les dialogues dynamiques sans observation DOM supplémentaire. Le logo
décoratif du titre reste séparé de son nom accessible. Les fenêtres principales
utilisent une vraie image avec lien et nom accessible. Aucun contenu métier ni
état n’est déduit de la couleur.

Les recherches initiales ont produit des sorties trop volumineuses et révélé
deux chemins supposés inexistants (`version.js`, `prepare.css`). Reprise : inventaire
réel des fichiers, lecture des producteurs `runtime-health.js` et `prephase.css`,
sorties ciblées. Aucun changement moteur nécessaire.

## Preuves et limites

Avant modification : les deux exécutions de la nouvelle recette sur le binaire
construit depuis les sources initiales échouent sur `/ favicon` (deux icônes), puis
sur `#modal-title has the canonical logo` après déplacement du contrôle de titre
en amont. Originaux : `/tmp/swarm-windows-20261017/before/failure.txt` et
`/tmp/swarm-windows-20261017/before-logo/failure.txt`.

L’ajout documentaire a d’abord échoué sur une ancre sans son tiret de liste ;
aucune modification partielle appliquée. L’inventaire a aussi établi l’absence
d’une page anglaise UI-DESIGN : le contrat de cet incrément est fourni en anglais
dans `docs/en/WINDOW-IDENTITY.md`, lié depuis la référence française.

Première vérification : 48 groupes page/langue/thème/largeur, 208 ouvertures de
dialogues, aucun échec HTTP ni erreur JavaScript. Méthode produit et terminal
interactif passent également. Le terminal utilise un programme Python local,
un vrai PTY et les opérations publiques ; aucun fournisseur IA n’est appelé.

Replan FEN-4 : la recette design échoue à sa ligne 100 sur le candidat et le
binaire initial. Cause établie dans `GraphStudio.curve/update` : les routes sont
des courbes cubiques `C`, alors que le test exige exclusivement `Q`. Correction
de la recette : vérifier les trois dépendances, les courbes `C` ou `Q`, la
longueur positive et les coordonnées finies, sans toucher au graphe. Originaux
dans `/tmp/swarm-windows-20261017/design/failure.json` et `design-baseline/failure.json`.

Replan FEN-2/FEN-4 : l’inspection du PNG d’aide mobile révèle que le focus natif
sur la fermeture en pied fait défiler l’introduction hors écran. Deux nouvelles
assertions échouent avant correction sur la préparation et l’aide contextuelle.
Choix : focus initial du titre pour les fenêtres d’information, plutôt qu’un
en-tête collant qui masquerait du contenu. Le helper de présentation partagé
remet aussi le défilement au début. Les formulaires gardent leur focus de saisie.
Originaux : `before-help/failure.txt`, sortie de `before-context-help` et capture
`windows/help-mobile-fr-sombre.png` sous le même répertoire de vérification.

Les résultats locaux ne prouvent pas le remplacement d’un serveur déjà
lancé. Les vidéos et captures pédagogiques enregistrées restent historiques :
cet incrément ne change pas les gestes enseignés et ne les recapture pas.

## Replan — console masquée et qualité du diagnostic

La recette design atteint ensuite sa vérification finale et conserve 18 erreurs
`JSHandle@error` sur le candidat comme sur le binaire initial. Le collecteur
conserve maintenant le nom, le message, la pile et l’emplacement de l’erreur.
La pile originale est `TypeError: Cannot read properties of undefined (reading
'top')`, dans la mesure DOM des retours à la ligne Monaco. Preuves :
`design-diagnostic/failure.json` et `design-baseline-diagnostic/failure.json`.

L’hypothèse initiale attribuait l’erreur à l’éditeur de préparation. Une recette
isolée de création, édition, masquage, thèmes et redimensionnement ne la reproduit
ni avec du texte long ni avec du texte court. Ces essais ont été interrompus sur
des préconditions de recette : contrôle dans un accordéon fermé, puis `select`+synthétique ignoré par la garde `isTrusted`. Ils ne sont pas des PASS. La recette
exploratoire a été retirée, les échecs originaux restent dans
`before-editor-visibility/failure.json` et `before-editor-short/failure.json`.
Aucun changement de l’éditeur de préparation n’a été appliqué.

La recette discriminante `tests/log_console_visibility_ui.cjs` reproduit la même
erreur en naviguant hors de Journaux, en redimensionnant la fenêtre, puis en
revenant : six erreurs, sans agent ni IA. La garde de `render()` existait déjà,
mais l’observateur interne `automaticLayout` la contournait. Décision : conserver
le ResizeObserver applicatif et son layout gardé, désactiver uniquement
l’observateur interne concurrent. L’accessibilité et le calcul des retours à la
ligne restent actifs ; la bibliothèque embarquée n’est pas modifiée.
Original : `before-log-visibility-ready/failure.json`.

Un premier essai de cette recette avait capturé le message vide avant les données ;
un lien direct Journaux avant le premier snapshot laissait aussi ce message vide.
La précondition retenue attend le snapshot attribué avant d’ouvrir Journaux puis
attend l’événement `work.create`, pas seulement un texte non vide. Le lien direct
initial vide reste une amélioration distincte : propriétaire frontend Journaux,
reprise par reproduction du lien direct sur une mission avec activité et assertion
de chargement des données sans changer de destination. Ce point ne conditionne
pas le logo ni le rendu demandé, et aucune correction n’est revendiquée ici.

Apprentissages : attribuer une erreur à sa fenêtre avant de modifier un consommateur ;
une garde de rendu doit couvrir chaque propriétaire du layout ; ouvrir les
accordéons et utiliser des événements clavier réels pour les contrôles protégés.

## Résultat vérifié et RETEX

Les quatre exigences de cet incrément sont vérifiées localement. Les sources
partagées ajoutent le logo aux dialogues sans copier leurs constructeurs ; les
fenêtres secondaires, le mobile, le rail replié et les icônes d’onglet conservent
l’identité. Les titres à empattements, sous-titres cyan, interlignes, notices,
citations et code en ligne emploient les jetons existants. Le contraste minimal
des 13 paires vérifiées dans les quatre combinaisons langue/thème est de **5,23:1**.
Ce contrôle n’est pas une certification WCAG complète.

Preuves pérennes : [`docs/screenshots/window-identity-20261017/`](../screenshots/window-identity-20261017/).
`verification.json` conserve les commandes, les codes de sortie, les empreintes
des sources UI et des quatre binaires, et les dates de production des résultats.
Les originaux des échecs, y compris les diagnostics de préparation interrompus,
y sont conservés. Les données proviennent uniquement de fixtures temporaires.

| Contrôle | Résultat | Preuve |
| --- | --- | --- |
| Fenêtres : FR/EN, deux thèmes, cinq largeurs, 208 ouvertures, logo, icône, focus | PASS | `windows-result.json` |
| Design : 12 destinations, clavier, menu, accordéons, formulaires, persistance, erreur et reprise, accès public | PASS | `design-result.json` |
| Aides contextuelles : 15 sujets, fermeture, focus, thèmes et mobile | PASS | `context-help-result.json` |
| Session : vrai PTY local, saisie, reprise, arrêt, logo et absence d’acceptation implicite | PASS | `terminal-result.json` |
| Aperçu méthode produit : FR/EN, thèmes, clavier, cinq largeurs, aucune mutation | PASS | `method-result.json` |
| Console masquée : absence d’erreur après navigation/redimensionnement et conservation des originaux | PASS | `log-visibility-result.json` |
| Console complète : six contrôles incluant filtres, export, sources, refus, mobile et repli | PASS | `logs-result.json` |
| Go : accès aux assets de connexion, garde d’origine et reconnexion après redémarrage | PASS, 3 tests nommés | `go test ./internal/engine -run 'TestWeb(LoginDesignAssetsRemainPublicAndAPIPrivate\|AuthenticationOriginAndSharedGuard\|StableAddressLoginAndRestart)$' -count=1 -v -timeout=0` |
| Traductions, syntaxe JS, workflow et espaces du diff | PASS | `npm run test:i18n`, `node --check`, `python3 tools/agent-workflows/check.py`, `git diff --check` |

Le binaire final de vérification est `/tmp/swarm-windows-20261017-verified`.
Les recettes contextuelles et terminal ont été exécutées sur le binaire `final`,
avant la correction indépendante de la console masquée ; la recette méthode
sur `candidate`. Leurs sources pertinentes de marque, aide ou méthode n’ont pas
changé ; la console commune a ensuite sa régression ciblée et sa recette complète.
Aucune revue indépendante, acceptation moteur, installation ou interruption
d’un serveur utilisateur n’est revendiquée.

La transmission est persistante : la référence UI lue depuis `AGENTS.md` décrit
les composants et le focus initial ; son contrat anglais et les deux pages
Journaux expliquent le même comportement. Les commentaires des producteurs et
les recettes portent aussi les gardes. Les contextes d’agents moteur et les
processus déjà démarrés ne sont pas reconstruits par une modification documentaire.
Le seul suivi hors périmètre retenu est le chargement du lien direct Journaux
avant le premier snapshot, avec propriétaire et critère de reprise ci-dessus.

## Replan — chaîne du manifeste d’intégrité

Le contrôle d’intégrité a d’abord refusé `.claude/skills/replan/SKILL.md` comme
entrée périmée. Le manifeste UI fraîchement produit ne contenait pas cette
entrée. Lecture du producteur : `graphDeliveryD03Load` choisit le manifeste moteur
quand il existe, après le manifeste UI. La recette documentée UI s’arrêtait donc
un générateur trop tôt. Le générateur moteur existant reprend le parent UI et
les entrées moteur actuelles ; il ne renouvelle aucune acceptation historique.

Décision : conserver cette chaîne et ses protections, corriger la documentation
FR/EN, puis régénérer les deux instantanés dans l’ordre avant de vérifier celui
réellement sélectionné. Aucun test, ancienne preuve ou hash n’est neutralisé.
Les modifications préexistantes du moteur restent des entrées observées, sans
revendication d’auteur ni de vérification métier pour cet incrément UI.

Replan de fixture DEN : le journal réel de la mission (trois événements) pouvait
remplacer les quatre événements projetés après leur injection DOM. Le témoin
valide existe dans les parcours précédents ; la nouvelle preuve conserve le
contenu et les coordonnées reçus dans `verified-dense/interrupted.json`.
La recette projette maintenant la réponse API activité de façon constante, comme
la recette Journaux existante, au lieu de concurrencer un chargement réel. Aucun
code produit ni délai n’est changé ; le parcours public et les contrôles de
sécurité de la fenêtre émettrice restent exercés.


## Publication Git demandée

La livraison est répartie en deux commits : lecture et identité des fenêtres,
puis lanceur de versions et modale des évolutions. Les sources, recettes,
contrats FR/EN et preuves de chaque capacité restent associés. Les travaux
moteur, mémoire et identité des autres chantiers sont conservés hors index.

Les instantanés livrés sont recalculés depuis les octets de l’index Git, avec
résolution des liens symboliques et exclusion des nouvelles entrées non indexées.
Leurs empreintes ne décrivent donc pas les modifications étrangères encore
présentes dans le checkout. Les instantanés du répertoire de travail restent
ceux de l’arbre partagé. Cette préparation ne renouvelle aucune acceptation
historique et ne transforme pas les recettes locales en revue indépendante.

Replan Git : deux écritures ont été refusées par `Unable to create
'.git/index.lock': File exists`. Le verrou était récent et réapparaissait ; son
propriétaire précis n’est pas établi. Aucun verrou n’a été supprimé ni processus
interrompu. Après une écriture groupée réussie de l’index, le commit a utilisé
un index temporaire reprenant exactement les mêmes entrées ; la création du
premier commit prouve cette reprise. Ce détour ne change aucun fichier source
et ne constitue pas un correctif général à la concurrence des clients Git.


Contrôle du second index : les sources étrangères sont remplacées par leurs
versions Git via un overlay Go temporaire, sans nouvelle copie complète du
projet. Le contrôle des rôles et les tests Version s’exécutent sur ces entrées
indexées ; tous passent. Le reçu `local-delivery/staged-version-checks.txt`
conserve les noms et sorties, y compris un test historique Version supplémentaire
sélectionné par le préfixe. Cette preuve du candidat Git reste distincte des
contrôles antérieurs du binaire installé construit dans l’arbre partagé.


## Replan — rendre le détail d’une tâche compréhensible

L’utilisateur observe le panneau réel et relève deux défauts : répétition des
mêmes informations et vocabulaire technique sans geste compréhensible. Le panneau
réel montre deux fois un refus de `git cat-file`, deux groupes d’actions, des
noms de rôle internes et une liste de dépendances réduite à ses identifiants.

| Exigence | Observable attendu | Vérification prévue |
| --- | --- | --- |
| CLR-1 | Libellés et titres en gras ; IA et consignes secondaires repliables | Navigateur FR/EN, deux thèmes, cinq largeurs |
| CLR-2 | Une seule action principale, en premier ; fin d’exécution distincte de validation | Compter les groupes, boutons et messages ; actualisation et clavier |
| CLR-3 | Tâches préalables et tâches concernées par leur titre, accessibles au clic | Navigation sans mutation, données `waiting_on` du moteur |
| CLR-4 | Échec de lecture Git expliqué ; original exact accessible une seule fois | Variantes de lecture Git et blocage métier inconnu, HTML littéral |

Décision : corriger le composant de détail, plutôt qu’ajouter des surcharges ou
réécrire les états métier. L’explication de lecture Git se fonde sur le préfixe
de commande produit par `managedGitText`, pas sur un identifiant, un hash ou une
phrase anglaise d’erreur. Elle décrit une lecture impossible, sans prétendre
diagnostiquer ou réparer automatiquement le dépôt. Les blocages inconnus gardent
leur motif visible. Les droits et commandes disponibles viennent toujours du
snapshot du moteur ; aucun lancement ou essai supplémentaire n’est demandé.

Anomalie de recette initiale : une écriture directe de `works` par SQLite a été
refusée avec `no such function: swarm_identity_plan_digest_v1`. Le binaire inclut
un contrat de stockage supplémentaire ; le témoin n’atteignait donc pas l’IHM.
La fixture emploie désormais `task update` public pour le statut et le blocage.
Seule une ligne d’agent historique fictive reste injectée dans la racine de test
isolée, sans processus ni fournisseur. Aucune protection n’est désactivée.

Suite CLR-F1, propriétaire moteur/dépôts : le message observé dans la mission
réelle signale un objet Git vide et un commit illisible. L’amélioration de lecture
ne répare pas cette donnée. Reprise : identifier le dépôt effectivement employé
par cette tentative, établir les sauvegardes et la source de récupération, puis
vérifier le commit par l’opération de lecture Git avant toute nouvelle tentative.
Ne pas supprimer d’objets ni autoriser une relance pour masquer cette erreur.


Premier passage CLR : gras, lecture Git, original exact, tâches nommées et
navigation sont vérifiés en FR/État. La fermeture Échap échoue après un clic vers
une autre tâche puis redimensionnement : le bouton déclencheur a été remplacé,
le focus revient au document et l’écouteur du panneau ne reçoit plus la touche.
Le panneau place maintenant le focus sur son titre à chaque ouverture/navigation,
par le helper commun, sans délai ajouté. Les prérequis remontent juste après
l’action principale ; leur avertissement ne duplique plus un refus brut contenant
les identifiants. Les refus complets restent dans leur accordéon dédié.


Second passage CLR : les quatre combinaisons langue/thème et les cinq largeurs
passent, puis la garde de recette refuse trois POST `/api/v1/visit` à la sortie
des pages. Lecture du producteur et du consommateur : ce sont les reçus de lecture
normaux (`markVisit`), distincts d’une mutation de tâche. La garde contrôle
maintenant leur méthode, coordonnées de fixture et corps vide, les compte
séparément et refuse toujours toute commande métier. La révision de la mission
reste contrôlée inchangée. Aucun code produit ni délai n’est modifié pour ce refus.


### Vérification finale CLR et RETEX

CLR-1 à CLR-4 : PASS dans la recette isolée, FR/EN × État/sombre, cinq largeurs,
avec originaux exacts, code littéral, navigation par titre, maintien des accordéons,
focus et Échap. Les huit cas du lecteur et la parité i18n restent verts.
Le contrôle ajouté du rendu complet puis de l’actualisation explicite passe.
Une capture intermédiaire affichait un avertissement d’actualisation à la
navigation ; la cause n’est pas établie et aucune réparation générale du
rafraîchissement n’est revendiquée. Suite CLR-F2, propriétaire frontend : reprendre
avec une trace du premier échec de refresh lors d’une navigation répétée ; garder
les captures `focus-failure.png` et `receipt-failure.png` comme observations.

Le lanceur prépare `dev.4`, puis `dev.5` après la correction de focus et d’ordre
de lecture. La relance réutilise le candidat vérifié `dev.5`. HTTP confirme le
numéro et l’empreinte du binaire installé ; cinq missions, trente-cinq tâches,
trente agents, leurs révisions/états et les politiques de mission sont conservés.
Les deux processus distincts observés sont encore présents. L’onglet navigateur
initial étant devenu indisponible, un nouvel onglet de la même interface est
ouvert ; il montre le panneau demandé et aucune erreur console reçue.
La première mesure DOM utilisait `parseInt`, absent du contexte de lecture de
l’outil ; la mesure corrigée reçoit les chaînes de poids CSS : treize valeurs
`750`. Cela corrige la mesure, sans changement du produit.

Limite CLR-F3, propriétaire frontend : les cartes du graphe conservent encore
leur résumé technique historique. Une prochaine harmonisation doit partager
le vocabulaire des diagnostics entre cartes et détail, avec une garde sur les
libellés de carte et la conservation de leurs originaux dans le détail. Cet
incrément vérifie le panneau explicitement signalé ; il ne prétend pas réécrire
tout le vocabulaire des autres écrans.

Apprentissage : hiérarchiser l’action avant les métadonnées ; décrire la commande
qui a échoué sans prétendre connaître sa cause exacte ; conserver les originaux
et faire venir les prérequis et les droits du moteur. Une ouverture depuis un
élément remplacé doit replacer le focus. Les reçus de lecture ne sont pas des
modifications métier, et doivent être comptés séparément dans les recettes.
