# Reprise après contention SQLite

## Ce qui change

Les décisions de planification utilisent la reprise SQLite bornée existante. Pour la revue indépendante, un résultat fournisseur déjà obtenu est désormais journalisé de façon attribuable avant sa persistance SQLite : après une contention, le moteur reprend ce même verdict sans rappeler le fournisseur, sans augmenter le compteur de revue et sans créer de tentative agent.

La politique opérationnelle est commune au moteur, à la CLI et à l’administration HTML : `busy_retries` (0–10), `busy_retry_delay_ms` (0–1 000) et `busy_timeout_ms` (1–60 000). `swarm storage-retry show` affiche les valeurs configurées/effectives, leur source, la persistance, la cause `sqlite_busy` et la borne totale `(retries + 1) × busy_timeout + retries × delay`. `swarm storage-retry apply --input <fichier>` valide puis enregistre atomiquement la politique dans `.swarm/storage-retry.json`.

## Mesure reproductible

Commande worker réellement exécutée :

```sh
python3 tests/sqlite_contention_final.py --timeout 300 --measure-only
```

La recette extrait `HEAD` dans une racine temporaire puis exécute exactement le même test et les mêmes paramètres sur cette baseline et sur le candidat courant. Chaque variante comporte neuf revues indépendantes avec fournisseur local déterministe, un verrou d’écriture, `busy_timeout_ms=1`, zéro reprise automatique et une reprise causale.

| Variante | n | p50 | p95 | max | erreurs initiales SQLite | effets métier durables | erreurs métier | appels revue | tentatives |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Avant (`HEAD` 8923ba5) | 9 | 60,843 ms | 71,537 ms | 71,537 ms | 9 | 0 | 9 | 9 | 9 |
| Après (candidat sale) | 9 | 59,286 ms | 67,748 ms | 67,748 ms | 9 | 9 | 0 | 9 | 9 |

Empreinte du résultat normalisé de la reprise : `4ed05b1bcd9cab73f7194ca82da02eb72cf489a42f09e62af239904a151143a1`.

La correction élimine l’échec métier dans cette charge scriptée. La p50 baisse légèrement tandis que p95/max augmentent légèrement ; neuf échantillons ne justifient aucune conclusion générale de performance. Ces nombres ne sont ni un benchmark fournisseur réel ni une preuve d’autonomie : le fournisseur est un double local et les racines sont isolées.

## Qualification finale à lancer sur l’hôte

Le conducteur hôte, et lui seul, doit exécuter :

```sh
python3 tests/sqlite_contention_final.py --timeout 290 --baseline-ref <revision-avant-correction>
```

La recette `tests/sqlite_contention_final.py` est portable depuis la racine du dépôt et compare la référence fournie avec une enveloppe globale explicite. La recette exécute la mesure avant/après, la race ciblée sans skip ni zéro test, `go vet ./...`, `npm test`, le contrôle de configuration, `git diff --check`, puis une seule découverte/exécution de la suite Go via le harness préservé `tests/supervision_go_suite.py`. Les skips éventuels sont inventoriés ; les tests requis ne peuvent pas être skippés.

Le contrôle final, le manifeste candidat, le rapport et le dossier doivent être liés aux mêmes octets avant la revue indépendante. La revue technique précède l’acceptation administrative ; la clôture globale n’est jamais une précondition de la revue S04.

## RETEX attributif

- Moteur : l’ancienne reprise de revue transformait un verdict calculé mais non persisté en erreur d’interruption. Le candidat journalise le verdict attribué et le reprend sans nouvel appel.
- Banc : les 27 erreurs historiques sont réparties entre 14 décisions et 13 revues ; les cinq délais fournisseur restent hors attribution SQLite. La mesure S04 est un cas scripté distinct et ne réécrit pas ces archives.
- Fournisseur : aucune autonomie réelle n’est démontrée. Les appels du banc sont locaux et déterministes ; coûts, quota et disponibilité d’un fournisseur réel restent inconnus.
- Supervision : le premier cadrage IA n’a pas respecté son contrat JSON ; la supervision a adopté le brief et le plan. Pour S03, un premier wrapper hôte omettait les arguments navigateur puis une assertion a classé à tort `ERR_ABORTED` de rechargement ; les préconditions ont été corrigées sans effacer les échecs historiques ni augmenter le budget.
- Worker S04 : le premier contrôle race a révélé que le test de mesure exigeait une sortie même lors de la suite découverte. Le test a été corrigé : sans sortie il vérifie 9/9 verdicts candidats, avec sortie la recette compare baseline/candidat. Les deux chemins ciblés passent.

## Limites actuelles

Au moment de la remise worker, la suite Go globale, le contrôle fonctionnel S04 hôte et la revue indépendante réelle ne sont pas exécutés. Les preuves ciblées ne valent donc ni acceptation moteur, ni installation sur un cockpit vivant, ni clôture de mission.


## Intégration sur main — 10 octobre 2026

La mission w-82cb75995c4b5ee02130797b est clôturée publiquement à la révision 98 : quatre tâches acceptées fraîches et revue indépendante passée. Sa qualification isolée a découvert 1 072 tests ; les skips optionnels sont explicitement exclus des réussites. Ces preuves concernent le candidat isolé, pas automatiquement la fusion. Le portage conserve les modules `internal/engine`, la connexion stable et les tutoriels de main. La recette exige désormais une référence Git avant correction explicite, et les tests historiques lisent les archives depuis la racine du dépôt. La fusion doit être contrôlée puis installée séparément. Les données ci-dessus sont historiques ; neuf échantillons ne prouvent pas un gain général de latence.
