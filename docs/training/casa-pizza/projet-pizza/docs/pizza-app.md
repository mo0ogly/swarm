# Task result — pizza-app

## Outcome in two sentences

Casa Pizza est implémentée avec son catalogue, panier, commande SQLite, suivi privé, espace restaurant et documentation. Les tests métier passent ; le transport TCP réel et le rendu navigateur restent non testés car la sandbox refuse l'ouverture d'un socket loopback et le formateur doit effectuer les captures.

## Identity and scope

- Work / task / attempt / producer : `w-9dec5c408415d85b6f332d2e` / `pizza-app` / `a-d61deab557f8ea13cff6b2d6` / `b62064a2-15c5-466c-a222-4546612d7ec4`
- Rôle et périmètre : worker ; application locale Casa Pizza, tests, README et handoff
- Révision de base : `700ec4598bf653570167a60202a44e04701d61b8`
- Candidat : révision de base avec changements non commités dans `app.py`, `static/`, `tests/`, `README.md`, `.gitignore`, `docs/HANDOFF.md` et ce rapport
- État : implémentation terminée et vérifiée dans les limites ci-dessous ; pas d'acceptation moteur

## Findings the responsible planner must know

Le dépôt ne contenait initialement que le besoin, les instructions et un `.gitignore`. Un premier harnais ouvrant un serveur TCP a échoué quatre fois dans une unique exécution de tests, chaque cas rencontrant `PermissionError: [Errno 1] Operation not permitted` lors de la création du socket. La précondition a changé : les tests appellent désormais directement les gestionnaires applicatifs réels et SQLite, sans substituer la logique métier ; ils ne prouvent toutefois pas le transport HTTP réel.

## Changes and verification

| Requirement | Change / file | Exact check and environment | Observed effect | Result | Evidence |
| --- | --- | --- | --- | --- | --- |
| C1 Catalogue, panier, confirmation | `app.py`, `static/index.html`, `static/app.js`, `static/style.css`, six SVG | `python3 -m unittest -v` (Python 3.13, gestionnaires sans socket) | Six pizzas et trois végétariennes ; câblage filtre/panier/formulaire et responsive contrôlé ; création valide testée | PARTIAL | Test `test_catalog_filter_basket_and_responsive_interface_are_wired` PASS ; rendu réel à 390 px/clavier non testé |
| C2 Persistance et montants serveur | `app.py`, `tests/test_app.py` | `python3 -m unittest -v` | Prix client falsifié ignoré ; sous-total 3450, livraison 290, total 3740 ; minimum, produit, quantité, téléphone et doublons excessifs rejetés ; relecture après recréation de l'application | PASS | Tests `test_invalid_orders_are_rejected_and_prices_are_server_owned` et `test_order_survives_application_recreation` PASS |
| C3 Suivi privé et restaurant protégé | `app.py`, `static/restaurant.*` | `python3 -m unittest -v` | Jeton absent/incorrect → 404 ; restaurant sans session → 401 ; mauvais mot de passe → 401 ; transitions hors ordre → 409 ; parcours reçue jusqu'à livrée visible avec le jeton | PASS | `test_private_tracking_restaurant_auth_and_strict_status_flow` PASS |
| C4 Tests métier et README | `tests/`, `README.md`, `docs/HANDOFF.md` | contrôle final `set -e; python3 -m unittest -v; python3 -m py_compile app.py tests/test_app.py; git diff --check` | 4 tests PASS, compilation sans diagnostic, diff sans erreur | PASS | Code de sortie final 0 |

## APEX / PDCA checkpoint

- Analysis / PLAN : besoin, contraintes de sécurité et livrables confrontés à l'état initial vide.
- Execution / DO : serveur standard library, schéma SQLite, interfaces client/restaurant, SVG, tests et documentation ajoutés.
- Verification / CHECK : quatre tests comportementaux PASS ; compilation et diff PASS. Le premier contrôle TCP a établi une interdiction de socket propre à la sandbox.
- Adjustment / ACT : harnais déplacé à la frontière des gestionnaires réels après l'échec socket ; validation des champs et agrégation des quantités durcies ; traversée de chemin statique empêchée par résolution canonique.
- Recovery limits : tentative courante, budget de 100 appels dont 20 réservés aux contrôles et au rapport.

## Next action and limits

Le responsable/formateur doit lancer `python3 app.py` dans un environnement autorisant `127.0.0.1`, vérifier à 390 px et au clavier une commande complète, l'avancement restaurant et l'actualisation client, puis arrêter avec `Ctrl+C`. Revue indépendante et acceptation moteur non réalisées ; aucun serveur durable, paquet, service externe, publication ou modification de `.swarm` n'a été effectué.

## Commandes réellement exécutées

1. Inventaire initial : `pwd`, `git rev-parse --show-toplevel`, `git status --short`, `git diff --stat`, lecture conditionnelle de `docs/pizza-app.md`, lecture de `docs/BESOIN.md`, puis `rg --files ...` — code 0.
2. Premier lot : `python3 -m unittest -v`, `python3 -m py_compile app.py tests/test_app.py`, `git diff --check`, `git status --short` — le premier sous-contrôle a produit quatre erreurs socket ; le code global 0 provenait de la dernière commande et n'a pas été utilisé comme preuve de tests.
3. `python3 -m unittest -v` après changement de harnais — 4 tests PASS, code 0.
4. `python3 -m py_compile app.py tests/test_app.py` — code 0.
5. `git diff --check` — code 0.
6. `python3 -m unittest -v` après durcissement des entrées — 4 tests PASS, code 0.
7. `exchange list w-9dec5c408415d85b6f332d2e --json` — liste vide, code 0.
8. État/révision, diff, `git diff --check` et `python3 app.py --help` — base confirmée, aide affichée, code 0.
9. Inspection ciblée de `.gitignore` après détection d'un écrasement involontaire — règles historiques restaurées, puis extensions `.db` ajoutées.
10. Contrôle final `set -e` : tests, compilation, diff — 4 tests PASS, code 0.
