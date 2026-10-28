# M23 — cachets dans la vue mémoire

Reprise du 17 octobre 2026 après « merci la suite ». Propriétaire : Codex,
développement direct, sans délégation ni orchestration native.

## État et décision

M22 a vérifié le composant isolé ; `project-memory.html` ne le charge pas encore.
Le producteur `projectMemoryCatalog` fournit type, version, état et action de
retrait, sans propriétaire ni auteur attestés ou décision de droit d'usage.
Le champ `MemoryVersion.Actor` reste une attribution historique affichée dans
les sources, sans être converti en identité certifiée.

Deux options : recopier des pictogrammes propres à la page, ou livrer les deux
assets canoniques et consommer leur API commune. Choix : seconde option, par
génération explicite de `web/signals.js` et `web/signals.css` depuis
`frontend/signals/`. Le catalogue de démonstration n'est pas embarqué.
Les contrôles et actions mémoire existants sont conservés ; le cachet est au
pied de la fiche. Une version active n'induit aucun droit d'usage.

La compilation Go observée dans la session identité interdit une mutation
concurrente des ressources embarquées. Préparer les fichiers hors de ces
ressources, puis contrôler la fin du processus avant leur application. Ne pas
modifier les sources identité ni arrêter son processus. Le serveur utilisateur
reste inchangé ; aucun déploiement implicite.

## Critères et séquence

| ID | Observable | Vérification prévue |
| --- | --- | --- |
| M23-01 | Cachet au pied de chaque fiche réelle, type expérience/procédure provenant de `version.kind`. | Admission et changement de type par le parcours CLI/API/navigateur isolé. |
| M23-02 | Active, historique, retirée, indisponible distincts ; type inconnu sans fausse expérience. | Remplacement, retrait et projection synthétique explicitement séparée des réponses réelles. |
| M23-03 | Identité/droits inconnus et aucune attribution certifiée inventée. | Vérifier les badges, textes et l'absence de succès vert ; `actor` reste dans les sources. |
| M23-04 | Sélection, aperçu, clavier, retrait, contexte reçu et refus périmé conservés. | Recette existante enrichie ; fournisseur local scripté, zéro appel modèle. |
| M23-05 | FR/EN, deux thèmes, cinq largeurs, contraste et erreurs navigateur. | Mesures et captures des fiches réelles. |
| M23-06 | Assets générés identiques aux sources et documentation FR/EN courante. | Génération et `--check`, lecture du diff et `git diff --check`. |

Préserver les préimages et empreintes dans `evidence/m23-20261017/`, ajouter les
assertions avant l'intégration, vérifier le contre-exemple sur les anciens
assets puis le candidat. Choisir le plus petit environnement démontrant le
parcours : binaire isolé actuel si ses entrées peuvent être figées ; sinon API
M19 conservée et ressources frontend candidates explicitement servies dans
la recette, sans déclarer une compilation Go actuelle vérifiée.

## Limites maintenues

M20/IA3 restent ouverts : aucune protection réelle ni nouvelle autorité n'est
activée ici. La revue locale n'est pas une revue indépendante. Le résultat et
le RETEX seront conservés dans `RESULTATS-MEMOIRE-19.md` avec les contrôles
effectivement exécutés et les critères non vérifiés.

Résultat : l'intégration frontend et les parcours sur API M19 conservée passent
dans onze groupes, dont vingt configurations de rendu. L'embarquement Go est
contrôlé séparément ; aucune validation du moteur identité courant ou du serveur
utilisateur n'est déduite. Voir [résultats et RETEX](RESULTATS-MEMOIRE-19.md).
