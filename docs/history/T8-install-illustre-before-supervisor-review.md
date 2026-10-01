# T8 — Installation illustrée (INSTALL.md FR/EN)

Tâche `plan-843bb3ce22-T8`, tentative `a-0978abf2bb9d55b0bb8c9fa6`. Base : `7110a0b805ca` + arbre de travail modifié (lots T3–T7 non commités, préservés). Rapport détaillé : `docs/plan-843bb3ce22-T8.md`.

## Ce qui a changé

- `INSTALL.md` et `docs/en/INSTALL.md` : **+37 lignes chacun, 0 suppression** (`git diff --numstat`). Diff horodaté : `docs/history/T8-install-diff-20260930T151527Z.patch`.
- Ajouts : capture du formulaire « Ajouter une IA » (rempli, non enregistré) ; tableau « trois états » (formulaire rempli / configuration enregistrée / lancement effectif) ; captures de configuration enregistrée FR/EN, thèmes clair et sombre ; section version serveur et mission supprimée ; section « Erreurs rencontrées et résolution » tirée du RETEX réel.
- Nouvelles captures réelles : `docs/screenshots/installation/admin-saved-{fr,en}-{etat,sombre}.png`, produites par `tools/verification/t8_saved_config_shots.cjs` (parcours web réel, racine temporaire, binaire compilé pour la tentative).
- Captures existantes réutilisées : `admin-*-etat/sombre`, `mission-lancee-fr`, `connexion` (FR/EN), récapitulatif, diagnostic.

## Contenu déjà présent conservé

Prérequis, Docker, réseau local/tunnel SSH, connexion des IA, CLI, mise à jour, installation native, dépannage, première mission, limites, récapitulatif/diagnostic : inchangés.

## Limites

- Le seul lancement réel illustré est `mission-lancee-fr.png` (interface française, 29/09/2026) ; aucun lancement inventé, pas de version EN de cette capture (légende EN précise « French UI »).
- Aucune capture de « mission supprimée » : décrit en texte uniquement.
- Les captures d'Administration proviennent de racines isolées, pas de la mission réelle.
- Revue documentaire indépendante non effectuée ici (revue humaine des captures faite par lecture visuelle du producteur : pas de secret visible, ce n'est pas une revue indépendante).
