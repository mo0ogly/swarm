# Casa Pizza

Application locale de démonstration pour commander des pizzas, suivre une commande et gérer son avancement côté restaurant. Elle utilise uniquement Python 3, SQLite, HTML, CSS, JavaScript et des SVG locaux.

## Lancer l'application

Prérequis : Python 3.10 ou plus récent. Aucune installation de paquet n'est nécessaire.

```bash
export CASA_PIZZA_RESTAURANT_PASSWORD='choisir-un-mot-de-passe'
python3 app.py
```

Ouvrir ensuite <http://127.0.0.1:18841>. L'espace restaurant se trouve sur <http://127.0.0.1:18841/restaurant>.

Le serveur écoute uniquement sur `127.0.0.1`. Le port et le fichier SQLite sont configurables :

```bash
python3 app.py --port 18842 --db /tmp/casa-pizza-demo.db
```

Pour arrêter le serveur, revenir dans le terminal et appuyer sur `Ctrl+C`.

Si la variable `CASA_PIZZA_RESTAURANT_PASSWORD` est absente, le mot de passe de démonstration est `atelier`. Il faut définir la variable pour tout usage autre qu'un atelier local.

## Utilisation

- Le client filtre les recettes végétariennes, ajoute des pizzas, ajuste les quantités et renseigne des coordonnées fictives.
- Le minimum est de 10,00 € hors livraison. La livraison coûte 2,90 €.
- Après confirmation, le navigateur conserve temporairement le numéro et le jeton privé de suivi dans `sessionStorage`.
- Le restaurant se connecte puis fait avancer une commande dans l'ordre : reçue, préparation, livraison, livrée.
- Les prix, quantités, produits, totaux et transitions de statut sont toujours validés côté serveur. Tous les montants persistés sont des centimes entiers.

## Tests

```bash
python3 -m unittest -v
```

La suite contient aussi sept tests HTTP qui démarrent un vrai serveur sur un port jetable et le redémarrent sur la même base. Les tests comportementaux exercent les gestionnaires applicatifs réels et SQLite : calcul serveur malgré un prix falsifié, entrées invalides, minimum, suivi privé, accès restaurant, transitions de statut et persistance lors de la recréation de l'application. Dans un environnement autorisant les sockets loopback, un contrôle manuel final consiste à lancer le serveur, commander depuis `/`, changer le statut dans `/restaurant`, puis actualiser le suivi client.

## Portée de la démonstration

Il n'y a ni paiement, ni compte client, ni livreur, ni service externe. Les coordonnées saisies doivent rester fictives. L'application n'est pas conçue pour être publiée sur Internet : les sessions restaurant sont en mémoire, le mot de passe est unique et il n'existe aucun mécanisme de récupération de compte.

## Référence visuelle du guide

La référence a été retravaillée après le premier atelier. Les illustrations
restent des SVG locaux, sans photographies ni téléchargement externe. Le
rapport `docs/REFONTE.md` sépare cette recette de l'acceptation historique.
Les ajouts restent sur la carte : ouvrez « Mon panier » pour commander. Après
confirmation le panier est vidé, tandis que le suivi demeure dans l'onglet.
Le filtre par numéro de l'espace restaurant facilite le contrôle d'une commande.
Les statuts avancent uniquement à l'aide des boutons du restaurant.
