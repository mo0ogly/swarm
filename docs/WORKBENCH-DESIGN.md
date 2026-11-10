# Design intégré du cockpit et des écrans de confiance

Référence visuelle adoptée : [maquettes](design/README.md). Le raccordement
utilise le cockpit Rust et les formulaires existants ; aucun adaptateur de
maquette, donnée de démonstration ou accès fournisseur n’est livré dans le produit.

## Conduite

Le sélecteur **Graphe / Couloirs** change uniquement la présentation. Sa préférence
est locale à l’appareil (`swarm-conduite:view:v1`, deux valeurs). Un stockage refusé
reste signalé et n’empêche pas la lecture. Aucun document ou identifiant de mission
n’est enregistré avec cette préférence.

Les Couloirs regroupent les tâches par `plan_role`, sinon `owner`, et par état
reçu. Les colonnes ne représentent pas des phases métier calculées à partir du
titre : ce sont les états effectivement disponibles. Une absence de rôle ou
d’observation demeure visible. Recherche, filtre de rôle et sélection sont locaux.
L’inspecteur lit les dépendances de Work et le motif de sa projection ; revenir au
graphe conserve les outils d’édition existants, avec leurs propres contrôles.

Changer de mission efface les anciennes fiches avant la lecture. Un chargement,
un refus ou une projection invalide ne vaut pas succès vide. Les cartes peuvent
montrer les données enregistrées avec une notice de suivi indisponible ; elles
n’accordent ni droit de lancement, ni validation, ni preuve d’activité actuelle.

## Préparation et Administration

Le dialogue et le document utilisent les contrôles réels de `/prepare.html`, avec
conversation plus large et document distinct sur ordinateur, empilement sur mobile.
Les étapes, les questions, les réponses et l’adoption restent les opérations du
moteur ; aucun état « prêt » n’est déduit des exemples des maquettes.

Les réglages Rust conservent leur catalogue, leurs portées, leurs valeurs exactes,
aperçus, historique et refus. La présentation des dialogues reprend les bordures,
la hiérarchie et les surfaces communes. Les valeurs et règles de l’exemple Admin
ne sont pas des defaults, limites ou politiques supplémentaires du produit.

## Sécurité et Audit

Le menu regroupe **Sécurité** avec Configuration et **Audit et preuves** avec Suivi.
Ce sont deux présentations de la page Trust existante :
`/trust-actions.html?view=security` et `?view=audit`. Les chemins et contrôles
historiques restent disponibles. Les paramètres ne transmettent que langue/thème ;
aucun jeton n’est ajouté à la navigation.

Sécurité conserve le dialogue réel et organise ses groupes en onglets : Biba,
Bell-LaPadula, conflits de rôles, identités et clés. Clavier : flèches, Début/Fin,
Tab et Échap natif du dialogue. Les champs, identifiants et écouteurs restent ceux
de `trust-native.js` ; aperçu, application et contrôles I3 restent côté moteur.

Audit présente les champs d’identité, la chaîne de transaction et le vérificateur
public existants. Les données et verdicts viennent des opérations réelles. Une
entrée de preuve vide ou refusée n’affiche pas un verdict favorable. Aucune clé,
identité ou transaction d’exemple n’est introduite. Les limites BLP et la portée
simulée des effets Trust restent celles indiquées par l’interface existante.

## Sources et vérification

- `web/swarm-workbench.js` : projection de lecture et préférence d’affichage.
- `web/trust-workbench.js` : composition du DOM existant, sans transport ou droit nouveau.
- `web/swarm-workbench.css` et les jetons communs : présentation et responsive.
- `rust/.../service/native_admin.rs` : assets privés embarqués et scripts natifs.
- `tests/swarm_workbench_ui.cjs` : contrôles sur HTTP Rust et fixtures CLI réelles.

Recette : `python3 tests/rust_native_cockpit_browser.py --binary BINAIRE --design
--output PREUVES_NEUVES`. Aucun fournisseur métier n’est lancé. Les événements et
faults client explicitement marqués restent distincts des réponses moteur réelles.
Les preuves de source, installation, refus et limites sont conservées dans le
[plan d’intégration](plans/DESIGN-INTEGRATION-20261108.md). Un contrôle local ne
constitue pas une revue indépendante, une qualification BLP globale ou une
acceptation du moteur. Les maquettes demeurent des références historiques.

En lecture partielle, les codes enregistrés `accepted`, `blocked` et `todo`
sont traduits en libellés FR/EN. Le refus de suivi et la notice restent visibles ;
ce rendu ne vaut pas acceptation indépendante ni rétablissement du suivi natif.

Les liens du cockpit vers Sécurité/Audit et les deux liens Admin conservent la
langue, le thème et le fragment local des limites. La page Trust applique le thème
entrant uniquement pour `etat` ou `sombre` ; ses liens relisent la préférence au
clic. Ces paramètres de présentation ne transportent aucun jeton.


## Conduite intégrée et défaut de présentation

Le cockpit démarre en sombre ; `?theme=etat` demande explicitement le clair. Le
choix au bouton reste porté par l’URL pour la langue et le rechargement. Les
commandes secondaires du graphe restent repliées au premier affichage ; leur choix
local est conservé, un élargissement seul ne les rouvre pas. Graphe et Couloirs
partagent un inspecteur : seul un identifiant du modèle reçu peut y être suivi.

La carte de mission du rail suit le Work reçu. État, boîte à traiter, assistant
et activité sont réunis sous « Détails de mission », sans clonage de contrôles.
Les compteurs sans producteur restent indisponibles avec explication ; les tâches
enregistrées ne deviennent pas une validation courante. La légende utilise les
badges d’état communs. Un nom identique au login n’est affiché qu’une fois, sans
modifier identité, habilitations ou authentification.

La recette Version compare aussi le thème invalide au défaut natif observé et
vérifie explicitement le défaut sombre de ce cockpit. Les contrôles du composant
(`workbench_summary_test.cjs`) restent des doubles DOM ; ils ne remplacent pas la
recette HTTP/browser ni la vérification de l’image réellement activée.
