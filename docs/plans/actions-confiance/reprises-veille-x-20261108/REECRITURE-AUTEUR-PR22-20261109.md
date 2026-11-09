# Réécriture de l’auteur — PR #22, 9 novembre 2026

Décision de Fabrice : Swarm est commité avec `mo0ogly@proton.me` (et non
l’adresse de `~/.gitconfig`). Config locale du dépôt : `user.email mo0ogly@proton.me`.

Force-push avec lease (ancienne tête `477c26060`) de
`codex/delivery-swarm-20261109`, autorisé explicitement par Fabrice. Arbres et
messages identiques, dates conservées ; seuls auteur et committer changent.
`main` et la branche locale ont été déplacés par mise à jour conditionnelle.
Les anciens objets restent dans le dépôt local (reflog) ; les documents qui les
citent restent valides localement, pas sur GitHub.

| Ancien | Nouveau | Sujet |
| --- | --- | --- |
| d084c1487 | 86afd8c76 | feat(web): integrate Swarm workbench design |
| 5efab9fcb | 6da6df122 | docs(plan): consolidate Swarm recovery queue and bounded work cards |
| 7e8cca690 | fa4af08f3 | fix(web): preserve modal Escape focus ownership |
| 9d40b7e60 | b4f0eae3a | feat(web): share recorded task and agent status badges |
| 8549dcc42 | 07f536505 | fix(review): bind native reviews to initial candidate inputs |
| 1d69f956b | ae25b152b | fix(history): read retained missions through bound storage aliases |
| a547d8e05 | 482f9e381 | fix(review): retain task-local Go qualification and alias checks |
| 79b8d854a | 999b156c3 | docs: retain bounded integration365 evidence and open delivery gates |
| 254e1bc8b | ed2dc0be7 | docs: record authorized delivery PR and observed365 cockpit |
| fe49bd4e4 | 8c6a7b982 | test(history): isolate alias fixture from personal Git identity |
| 477c26060 | 5d34d499f | fix(i18n): reconcile historical English source catalogue |

Nouvelle tête PR #22 : `5d34d499fe38683c45a26a4b97a204aa631688df`. CI relancée
(en attente au moment du push). Les 498 commits de `origin/main` portent encore
l’ancienne adresse : non traités, décision séparée.
