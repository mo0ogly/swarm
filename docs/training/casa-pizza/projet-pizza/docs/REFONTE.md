# Recette de la refonte visuelle Casa Pizza

La référence de formation a été retravaillée dans la session du formateur le
8 octobre 2026, après la recette historique décrite dans FORMATEUR.md.
Cette révision n'est pas attribuée à l'exécutant Swarm initial. La capture
d'acceptation historique ne constitue pas une gate pour ces nouveaux fichiers.

## Direction graphique

Identité de trattoria illustrée : papier crème, encre vert sombre, accent
terre cuite, titres en serif et texte utilitaire en sans serif. L'accueil
associe un message court, un bouton vers la carte et une composition de pizza.
Les six illustrations SVG ont été redessinées avec ingrédients distincts,
gradients et textures locales. Il ne s'agit pas de photographies.
Aucune note client, promesse de délai ou provenance fictive n'est affichée.

Le panier utilise des miniatures et sépare sélection, montants et coordonnées.
Le bouton d'envoi est désactivé pendant la requête, puis réactivé. Une commande
réussie vide le panier pour limiter une nouvelle soumission par inadvertance.
Les ajouts restent sur la carte et le compteur permet d'ouvrir le panier.
Les erreurs réseau donnent une action utile au lieu de « Failed to fetch ».
Le restaurant peut filtrer les commandes par numéro après authentification.

## Contrôles réellement effectués

- Suite Python : onze tests réussis en 3,258 s, sans ResourceWarning.
- Catalogue navigateur : six recettes puis trois après filtre végétarien.
- Minimum : une Margherita, 9,50 euros hors livraison, refusée avec le message.
- Quantité au clavier : flèche haute sur Margherita recalcule le total.
- Commande desktop CP-75D66373 : deux Margherita et une Reine, 34,40 euros.
- Accès restaurant : mauvais mot de passe refusé, démonstration autorisée.
- Restaurant : filtre par numéro, préparation, livraison puis livrée.
- Client : même numéro et statut livré après actualisation du suivi.
- Mobile : viewport de 390 pixels, contenu disponible de 375 pixels hors
  barre de défilement, largeur de contenu 375 pixels, sans débordement.
- Commande mobile CP-59EEF5C3 : une Reine, 15,40 euros, statut reçue.
- Captures : accueil, carte, erreur, panier, confirmation, restaurant,
  livraison finale et confirmation mobile dans le vrai navigateur.

La revue visuelle est celle du formateur, pas une revue indépendante. Les
onze tests ne sont pas un audit de design ou d'accessibilité. Les règles métier
et la confidentialité sont couvertes par les tests précédents conservés.
L'avancement reste manuel et aucune commande, livraison ou paiement réel n'est
réalisé. Les captures montrent uniquement des coordonnées de démonstration.
