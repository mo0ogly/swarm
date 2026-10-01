# T7 — Livrable courant

Le rapport courant est `docs/plan-843bb3ce22-T7.md` : observations du résumé,
transcription FR/EN des textes, revue rédactionnelle motivée et code exact de
construction du diagnostic copié. Les anciens états sont conservés dans
`docs/history/T7-documentation-retex-before-final-review.md`.

La copie utilise des formulations contrôlées plutôt que les champs libres du
fournisseur ; une recette navigateur injecte secret et URL pour vérifier leur
exclusion. Les détails techniques restent locaux, dans la modale de prévol ou
les traces repliées. La tentative initiale et la reprise du contrôle sont conservées dans l’historique.

La recette locale n'est pas l'avis indépendant. Ce livrable reste à examiner.

## Compte rendu de reprise du 1er octobre 2026

Tentative a-d38aa7df53b896925c0c4858 (départ 5/5, révision 311), base Git
`7fba73aef48073c35b5d21f6921331e34321f6b2`, modifications non commitées. Les
sections précédentes sont historiques ; leurs empreintes ne sont pas celles de
cette reprise (voir les empreintes actuelles dans `docs/plan-843bb3ce22-T7.md`).

Commandes rejouées le 1er octobre 2026, hors base réelle :

- `python3 tools/verification/t7_prelaunch.py all` : code 0, `PASS T7 all`, onze constats, quatorze captures, parité i18n.
- `node tests/planning_metrics_ui.cjs /tmp/swarm-next /tmp/swarm-t7-final-metrics` : code 0, `PASS: truthful planning metrics, FR/EN, both themes and help`.

Le compteur `Retours à traiter` mesure les événements de planification sans
décision. Les `activations de planification` ne sont pas un nombre prouvé
d'appels fournisseur.

Limites : recette navigateur locale FR/EN, deux thèmes ; presse-papiers système
simulé ; jugement de clarté réservé à la revue indépendante ; aucune preuve de
fonctionnement d'un fournisseur réel. Aucune acceptation par le producteur.
