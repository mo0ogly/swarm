# Badges KPI colorés — 8 novembre 2026

Demande utilisateur : badges des KPI en couleur. Codex seul, reprise replan et
vérification réelle ; aucun changement moteur, droit, budget ou consommateur
de données. Le témoin 348 montrait quatre valeurs transparentes et neutres.

Changement dans `web/swarm-design.css` : pastilles compactes sur les valeurs
strong, info bleu/cyan, success vert, attention ambre, alert rouge, neutre par
défaut. Paires fond/encre existantes des deux thèmes ; cartes discrètes conservées.
Classes de métriques et tonalités de KPI de mission déjà produites sont réutilisées.
Aucune fonction ni issue d’erreur modifiée, aucune nouvelle politique opérateur.
Les dimensions de pastille sont des choix de présentation, pas des limites moteur.

Build normal et installation locale par swarm.sh, version **v0.1.1-dev.349** ;
SHA-256 `bf19eb56a3794a5a32fbb375605563e1093f8064824e22a175f7f4cd16ce74c9`.
Profil sans feature de fixture, 779 entrées effectives archivées. Le contrôle final
compare aussi les 3 070 empreintes du manifeste avant build au checkout : inchangées.
Notes FR/EN rédigées avant build et vérifiées dans le dialogue Version réel, lien
Git de base et modifications non commitées visibles. Aucun commit/push par ce travail.

Vérification navigateur CUA de l’instance authentifiée : FR/EN × clair/sombre,
largeur réelle 609 px, quatre badges opaques selon leurs tonalités, aucun
débordement, valeurs conservées (8 tâches, autres indicateurs indisponibles « — »).
Contrastes calculés sur les styles réellement appliqués : clair 6,31–7,09:1 ;
sombre 8,21–8,63:1. Aucun avertissement ni erreur de console capturé. Après un
premier instant de chargement au rechargement, les mesures ont été reprises sur
l’état stabilisé ; aucun tiret transitoire assimilé à une valeur métier.

La feuille réellement observée par le navigateur a été exportée via CUA pageAssets
puis comparée au checkout et à l’archive effective du compilateur, empreinte
`15e0bec2004a05928d4b06b02b85ff9f6f95c9fbc8e42f904dc3788e24199934`.
L’export temporaire choisi par l’outil a été recopié dans la zone de preuves locale.
Pas de comparaison implicite à partir des seuls fichiers de travail.

Limites : surface legacy de KPI de mission non exercée ; pas de qualification de
toutes les largeurs ni de mutations de mission. Vérification visuelle réelle,
pas de test artificiel recopiant cette modification CSS réversible. Les labels
restent explicites et les tirets ne deviennent ni zéro ni validation. Préférences
initiales restaurées (FR, clair, aucune mission sélectionnée).

Documentation : docs/UI-DESIGN.md et docs/en/WINDOW-IDENTITY.md, notes embarquées
version_history.json. Preuves locales : `.cache/kpi-badges-20261108/`, index
LOCATION.json avec archive Lexar, empreinte, inventaire et restauration.
