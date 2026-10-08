# Remise — Casa Pizza

## Résultat

Casa Pizza est implémentée avec la bibliothèque standard Python, SQLite et des ressources web locales. Les tests métier passent ; le parcours sur un vrai socket et le contrôle visuel dans un navigateur restent à exécuter par le formateur, car la sandbox a refusé l'ouverture d'un socket loopback.

## Périmètre livré

- Catalogue de six pizzas, ingrédients, prix et filtre végétarien.
- Panier avec ajout, modification de quantité et totaux visibles.
- Commande validée et recalculée côté serveur, montants entiers en centimes, minimum de 10,00 € et livraison à 2,90 €.
- Persistance SQLite et suivi individuel protégé par un jeton aléatoire dont seul le condensat est stocké.
- Espace restaurant protégé par mot de passe et cookie de session `HttpOnly`/`SameSite=Strict`.
- Transitions strictes : reçue → préparation → livraison → livrée.
- Interface française responsive, navigation clavier native, messages d'erreur visibles et six SVG locaux.

## Vérifications exécutées

| Critère | Commande / environnement | Résultat | Preuve ou limite |
| --- | --- | --- | --- |
| Tests métier | `python3 -m unittest -v` | PASS — 4 tests | Calcul serveur, rejets, confidentialité, authentification, statut et persistance |
| Syntaxe Python | `python3 -m py_compile app.py tests/test_app.py` | PASS | Aucun diagnostic, code 0 dans le contrôle final |
| Propreté du diff | `git diff --check` | PASS | Aucun diagnostic, code 0 dans le contrôle final |
| Transport HTTP loopback | premier `python3 -m unittest -v` avec serveur TCP | NOT TESTED | `PermissionError: [Errno 1] Operation not permitted` à la création du socket ; harnais remplacé par appel direct des gestionnaires réels |
| Rendu navigateur à 390 px et clavier | Non exécuté | NOT TESTED | Aucun serveur durable lancé ; captures et parcours manuel laissés au formateur |

## Commandes de lancement

```bash
export CASA_PIZZA_RESTAURANT_PASSWORD='choisir-un-mot-de-passe'
python3 app.py
```

Arrêt : `Ctrl+C`. Tests : `python3 -m unittest -v`.

## Limites et prochaine action

Le formateur doit lancer le candidat dans un environnement autorisant `127.0.0.1`, réaliser une commande complète à 390 px et au clavier, avancer son statut dans l'espace restaurant, puis vérifier l'actualisation du suivi après redémarrage. La revue indépendante et l'acceptation restent du ressort du moteur Swarm ; cette remise ne les remplace pas.
