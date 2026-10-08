# Webapp de livraison de pizzas pour un atelier débutant

Créer « Casa Pizza », une application locale française permettant à un client
de consulter six pizzas, filtrer les végétariennes, ajouter des produits au
panier, modifier les quantités, saisir une adresse fictive et confirmer une
commande. Le paiement est à la livraison ; aucun paiement ou livreur réel.

Le serveur calcule les montants en centimes, conserve les commandes en SQLite
et refuse les produits ou quantités invalides. La livraison coûte 2,90 euros,
avec un minimum de 10 euros hors livraison. L'utilisateur obtient un numéro
et un suivi. L'espace restaurant protégé permet de passer de reçue à préparation,
puis en livraison et livrée. Les changements doivent être visibles côté client.

Stack pour l'atelier : Python 3 standard library, SQLite, HTML CSS JS natif,
sans CDN ni dépendance à installer. Écoute 127.0.0.1, port configurable 18841.
Interface responsive, labels français accessibles, clavier et erreurs visibles.
Design : crème, encre sombre, rouge tomate, vert basilic ; hero court,
cartes pizzas illustrées par SVG local, panier clair, étapes de suivi lisibles.

Critères observables
1. Le catalogue présente six pizzas avec ingrédients et prix.
2. Le filtre végétarien affiche uniquement les produits concernés.
3. Ajouter deux pizzas et modifier une quantité recalcule le panier.
4. Le minimum et les champs invalides bloquent une commande avec un message.
5. Les montants et produits sont contrôlés par le serveur.
6. Une commande valide est sauvegardée et possède un suivi individuel.
7. La commande reste consultable après redémarrage du serveur.
8. L'espace restaurant refuse l'accès non authentifié et fait avancer le statut.
9. Le client retrouve le statut actualisé ; les données d'autres clients sont privées.
10. Le parcours fonctionne à 390 pixels de largeur et au clavier.
11. Les tests démontrent commande, erreurs, calcul serveur, accès et persistance.
12. Le README explique lancement, tests, portée de démonstration et arrêt.

Livrables : code, tests, README, docs/HANDOFF.md avec vérifications exécutées.
Aucune intégration externe, aucune donnée réelle et aucune publication Internet.

## Complément de conception pour la référence de formation

Le premier jet fonctionnel doit être repris si son rendu reste générique.
Direction choisie pour la référence : trattoria illustrée, papier crème, encre
vert sombre, terre cuite, titres serif. Accueil avec composition originale,
six illustrations locales distinctes, panier à miniatures et suivi cohérent.
Pas de fausses évaluations clients, de faux délais ni de promesses non testées.

La personne chargée de la recette doit examiner le vrai rendu desktop et mobile,
le focus clavier, les erreurs, les montants et la continuité visuelle du parcours.
Les tests métier ne valident pas la qualité graphique. Consigner dans
docs/HANDOFF.md les écrans réellement examinés et les contrôles non réalisés.
