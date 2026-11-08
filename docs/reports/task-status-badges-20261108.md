# Badges des statuts de tâches — 8 novembre 2026

Demande : mêmes familles de couleur en clair/sombre, comme pour les KPI.
Replan/verify-fix, Codex seul. Onglet initial ancien 349 ; rechargé contre le
service réellement actif 356 avant diagnostic. Le tableau affichait accepted,
blocked, todo sans badge ; les couloirs avaient déjà leurs couleurs.

Correction : producteur commun SwarmWorkbench.statusBadge pour tableau natif,
couloirs et inspecteur. Vert accepté, bleu en cours, rouge bloqué/échec, ambre
soumis/périmé/dérogation, gris à faire/inconnu. Même famille dans les deux thèmes,
paires fond/encre adaptées au contraste ; pas d’égalité arbitraire des hexadécimaux.
Le ton vient du code reçu, jamais du libellé. Codes conservés en data-task-state,
libellés traduits via le catalogue existant, texte uniquement. Les couleurs ne
prouvent ni activité réelle ni acceptation indépendante ; la notice reste visible.

Preuve avant correction : test réel du consommateur échoue « missing badge
accepted », source antérieure conservée ; témoin navigateur356 huit lignes, zéro
badge de tableau. Après : cinq assertions nommées passent (codes, inconnu sûr et
neutre, libellé sans autorité, état manquant explicite, remplacement et vide).
Recette permanente task-status-badges.json PASS, integrity VERIFIED, sans watchdog.
Double DOM explicitement distinct de l’interface réelle.

Version **v0.1.1-dev.357** construite et installée via swarm.sh build puis
restart --prepared-build, SHA-256
`962fff329f5f6d6875b5d51b59ce558d3e73deed56419aca9efd8c1d7246062e`.
Profil normal sans feature fixture ; manifeste de sources resté identique.
Dialogue réel Version : notes FR/EN, commit de base
`b4e07cb26066cffdb20b36ac2fc9c60f758cb23e`, modifications non commitées divulguées.
Aucun commit/push par ce travail ; sources préexistantes du workbench préservées
par snapshots, seules les parties statut concernées sont reprises ici.

Parcours réel FR/EN × clair/sombre, huit badges visibles, pas de débordement à
609 px. Acceptée/Accepted vert, Bloquée/Blocked rouge, À faire/To do gris,
contrastes minimums mesurés 6,69:1 clair et 8,06:1 sombre. Couloirs contrôlés,
sélection de tâche bloquée : inspecteur rouge correspondant. Console capturée
sans erreur/avertissement. Pas d’injection de faux états dans une mission :
en cours, soumis et autres variantes restent prouvés au niveau composant.

Deux feuilles réellement observées par le navigateur exportées via CUA pageAssets,
copiées depuis le temporaire choisi par l’outil vers les preuves gouvernées,
comparées au checkout et à l’archive effective du compilateur. Source-review.json
lie politiques, fonctions et erreurs aux empreintes ; DOM/translation échouent
vers le refus visible du chargeur existant, sans succès vide ajouté.

Pas de nouvelle politique, délai, budget ni droit. Aucun état de tâche modifié ni
appel fournisseur. La sélection des tâches et des vues est de présentation.
Documentation FR/EN dans UI-DESIGN.md et en/WINDOW-IDENTITY.md ; notes embarquées
version_history.json. Preuves locales .cache/task-status-badges-20261108, index
LOCATION.json pour archive Lexar, empreinte et restauration. Limites : pas de
matrice de toutes les largeurs ni de capture réseau exhaustive via CUA.

Conservation achevée : 53 fichiers comparés aux sources ; archive USB
relue, SHA-256 `fb633534330fe3cd5dbd1fbe89f6444b1a42041681e15a542a4b937e072c6075`. Recette restaurée
et revérifiée VERIFIED/PASS. Aucun original supprimé. Rapport final postérieur
à l’archive conservé séparément ; préférences initiales FR/clair/aucune mission restaurées.
