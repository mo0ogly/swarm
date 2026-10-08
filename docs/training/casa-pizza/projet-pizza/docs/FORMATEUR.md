# Recette finale Casa Pizza

Le code produit par l'exécutant Swarm a été contrôlé et complété dans la session
du formateur. L'application permet un parcours de commande réel sur le serveur
local, avec persistance SQLite, suivi individuel et avancement restaurant.

## Historique et portée

La première tentative Codex a refusé le projet non initialisé en Git. Après
initialisation, la deuxième tentative a livré le code et quatre tests métier.
Son échange de remise a échoué et le plafond d'erreurs d'outils l'a interrompue.
L'historique Swarm conserve ces deux états ; ils ne deviennent pas des succès.
Le rapport de l'exécutant signale correctement les contrôles HTTP et navigateur
non réalisés dans sa sandbox. La présente recette apporte ces contrôles dans
l'environnement du formateur, après inspection du code.

Cette vérification est faite dans la session de réalisation. Elle n'est pas une
revue indépendante et ne démontre pas une mission hiérarchique autonome.

## Corrections finales

- Ajout de sept tests HTTP avec vrais serveurs et bases temporaires.
- Fermeture explicite des connexions SQLite après transaction.
- Effacement de l'erreur de minimum lorsque le panier est corrigé.

## Résultats observés

| Critère | Vérification réelle | Résultat |
| --- | --- | --- |
| Catalogue, filtre et panier | Navigateur : six recettes, filtre de trois végétariennes, quantités au clavier | PASS |
| Commande et montants | 2 Margherita et 1 Reine : 31,50 euros + 2,90 euros = 34,40 euros ; prix client falsifié ignoré en HTTP | PASS |
| Erreurs | Commande sous le minimum rejetée, champs/produits/quantités invalides rejetés, erreur effacée après correction | PASS |
| Restaurant et suivi | Mauvais mot de passe rejeté ; progression reçue, préparation, livraison, livrée visible côté client | PASS |
| Confidentialité | Suivi sans jeton ou jeton faux : 404 ; accès restaurant sans session : 401 | PASS |
| Persistance | Vrai processus arrêté puis relancé sur la même base, commande relue et accès restaurant à reconnecter | PASS |
| Mobile | Viewport réellement vérifié de 390 px, pas de débordement horizontal, commande Reine 15,40 euros confirmée | PASS |
| Tests et lancement | 11 tests, compilation Python, diffcheck et kit de création d'un projet neuf | PASS |

Commandes finales :

```sh
python3 -W error::ResourceWarning -m unittest -v
python3 -m py_compile app.py tests/test_app.py tests/test_http.py
git diff --check
```

Les onze tests passent (1,525 s), sans avertissement de ressource. Les captures
ont été réalisées sur le vrai cockpit Swarm et la vraie application locale.
Les coordonnées et commandes sont fictives. Le mot de passe de démonstration
n'est pas destiné à un déploiement : définir la variable décrite dans le README.
Aucun paiement, envoi de message ou livraison extérieure n'a été effectué.

## Limites du livrable

Application d'atelier sur loopback uniquement, sans paiement en ligne, comptes
clients ou exploitation logistique réelle. Les sessions restaurant sont en
mémoire ; le suivi est mémorisé dans l'onglet client. L'accessibilité vérifiée
couvre les labels et quelques actions clavier ; ce n'est pas un audit complet.
La soumission du rapport, la gate et l'acceptation se font par les opérations
publiques Swarm après cette recette, sans modifier sa base directement.
