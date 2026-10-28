# Accueil Projets et développement

Demande utilisateur du10octobre2026 : première UX développement, bilingue, inspirée du principe d’orientation de l’image fournie mais avec une composition propre à Swarm. Wattson et UX métier hors périmètre.

Livrer une page autonome projects.html, réutilisant les jetons/logo/i18n et les API publiques existantes. Composition : masthead horizontal, intention guidée avec trois besoins, parcours illustré jusqu’aux preuves, missions existantes filtrables, formation/contextes secondaires. Aucun chiffre inventé, aucun appel IA sur accueil, aucune mutation automatique. Les entrées mènent à la préparation réelle, aux modèles existants, au cockpit et à la formation. Préremplissage de besoin seulement dans une nouvelle préparation sans id ; aucun brouillon existant écrasé.

Vérifier FR/EN, sombre/etat, desktop/mobile/clavier, sélection/parcours, missions vides/chargement/erreur/reprise et absence de mutation ou appels fournisseurs. Utiliser serveur réel sur racine isolée et captures inspectées. Ne toucher ni candidat de campagne figé ni fichiers de formation en cours de modification. Ajouter accès explicite à l’accueil dans le cockpit, préserver les liens directs aux missions. Les deux applications séparables restent une cible ; ce lot ne prétend pas livrer l’UX métier ni la séparation complète des builds.

## Tranche adoptée — catalogue de sujets, accord utilisateur

Six modèles spécialisés : outil interne, portail client, API, stocks, traitement
de données et tableau de bord. Ils enrichissent le cadre commun sans remplacer
les quatre modèles historiques. Chaque sujet affiche son résultat, un exemple,
les livrables et les preuves attendues. Le choix d’intervention créer/faire
évoluer/corriger/migrer adapte le besoin et la méthode proposée ; aucun appel IA,
création d’équipe ou autorisation automatique. Préserver réponses et brouillons.

API/CLI partagent les mêmes données et le même contrôle déterministe des réponses.
Les cinq étapes communes puis six par fonctionnalité restent une méthode
proposée, pas un suivi automatique implémenté. Test : projection FR/EN sur toutes
les interventions, refus inconnus, gardes, navigation/modale/clavier/thèmes,
insertion et enregistrement explicites sur racine isolée. Aperçu18848 uniquement,
pas de redémarrage18792 ni changement au candidat de campagnes.

## Tranche suivante — rendre le parcours produit lisible

Reprise utilisateur « next » : aperçu accessible depuis les modèles de cadrage,
fermé par défaut. Montrer les cinq étapes communes et les six étapes répétées par
fonctionnalité avec leurs résultats attendus, selon PRODUCT-WORKFLOW.md. Une
modale native évite une longue explication dans l’accueil ; sélection d’une étape
sans mutation ni appel IA. Préserver navigation, traductions et états du catalogue.
Vérifier FR/EN, sombre/etat, cinq largeurs, clavier/Échap/restitution du focus,
les six résultats distincts et absence de missions créées sur racine isolée.

Clarification utilisateur : « modèle » doit à terme porter groupes d’actions,
dépendances, décisions et validations. Le catalogue actuel fournit des modèles
de cadrage ; il ne fournit pas encore ces graphes prédéfinis. Le libellé et
l’explication de l’accueil rendent cette limite explicite. Ce lot ne crée aucun
graphe réutilisable ni suivi persistant des étapes.
