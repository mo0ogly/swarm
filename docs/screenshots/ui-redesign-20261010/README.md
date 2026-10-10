# Refonte du 10 octobre 2026

Captures de l’application réellement servie par le binaire candidat, dans une
mission de fixture Casa Pizza isolée. Aucun résultat métier ou appel IA réel
n’est revendiqué. Voir `../../UI-DESIGN.md` pour les règles de conception et
`../../reports/ui-redesign-20261010.md` pour la vérification.

- `cockpit-fr-etat.png` / `cockpit-fr-sombre.png` : navigation, synthèse et graphe.
- `prepare-fr-etat.png` / `prepare-fr-sombre.png` : rédaction du besoin.
- `preparation-dialogue-fr.png` : document enregistré, dialogue et contexte IA.
- `connection-en-sombre.png` : formulaire bilingue et contraste des actions.
- `login-fr-etat.png` / `login-fr-sombre.png` : accès avant authentification.
- `terminal-mobile.png` : session interactive locale sur écran étroit.
- `design-result.json` : assertions et mesures de contraste de la recette navigateur.
- `terminal-result.json` : vraie session PTY, reprise et arrêt, sans modèle IA.
- `graph-result.json` : parcours du graphe et mesures de charge, dont un seuil dépassé à 200 tâches.
- `graph-task-inspector.png` : sélection d’une tâche et inspection de sa tentative.
- `candidate.json` : empreintes des sources UI et du binaire vérifié.
- `cockpit-collapsed-fr-etat.png` / `cockpit-collapsed-fr-sombre.png` : rail replié,
  graphe élargi, bouton de réouverture visible.
- `next-design-result.json` / `next-guidance-result.json` : repli du rail, lecture
  différée des résultats, conservation du focus et parcours métier de la suite.
- `candidate-menu.json` : étape rail + résultats différés.
- `next-graph-before-cache.json` : recette de cette étape, dont le seuil à 200
  tâches reste dépassé (1 093 ms).
- `candidate-next.json` : candidat avec cache géométrique ; `candidate.json`
  reste le candidat initial de la refonte.
- `next-design-final-result.json` / `cockpit-collapsed-final-fr-*.png` : recette
  complète et captures du candidat avec cache, dont le recalcul après corruption.
- `next-graph-result.json` : quatre parcours du graphe et budgets de rechargement
  PASS ; premier affichage à froid mesuré séparément, sans qualification p95 à froid.
- `load-profile-before.json` / `load-profile-after.json` : diagnostic instrumenté,
  distinct de la recette de qualification. Le second profil chevauche une autre
  recette UI : ses durées totales ne prouvent pas un gain comparatif.

Ces exemples montrent une mission non configurée : ses avertissements sont réels
pour cette fixture et ne constituent pas l’état du projet de l’utilisateur.

Formation : `training-capture-result.json` (captures réelles et atelier inchangé),
`training-capture-v2-failure.json` et `training-capture-v3-failure.json` (échecs
du harnais conservés), `training-media-result.json` (empreintes et décodage
intégral), `training-reader-result.json` (binaire et deux kits hors ligne).
`candidate-graph-cache.json` conserve le binaire des recettes graphe ;
`candidate-next.json` désigne le binaire livré avec les médias actualisés.
`preview-final-collapsed.png` et `preview-final-training.png` sont les vues
vérifiées dans le navigateur de l’application.
