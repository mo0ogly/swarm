# T4 — Vérifier le dossier avant une revue payante

Le premier agent a été interrompu après 600 secondes et 41 appels sans rapport.
Le conducteur a lancé automatiquement la deuxième tentative autorisée à 08:59 UTC.
La supervision a constaté cette reprise pendant ses propres contrôles : elle n'a
pas démarré une troisième tentative ni augmenté un plafond.

## Correction attribuée au superviseur

Le moteur indépendant refusait déjà les preuves de contrôles manquantes ou
périmées avant de réserver un appel, mais sa projection ne montrait pas le motif.
result_presentation.go projette maintenant ce refus, uniquement pour une tâche
soumise, un producteur terminé et une revue indépendante requise sans avis.
Le diagnostic ne crée aucun avis ni événement, ne modifie pas le statut,
les tentatives, les limites ou les consommations. Les preuves redevenues
admissibles permettent la revue normale. Une revue favorable ne valide pas la tâche.

## Preuves et limites

TestIndependentReviewStepGatesDossierBeforeProviderCall utilise un Store temporaire,
un fournisseur shell local contrôlé et une marque de lancement du processus.
Les métadonnées initiales du reçu sont synthétiques : ce test ne démontre pas
l'exécution réelle de go version ni un jugement autonome d'une IA.
Il vérifie la porte d'entrée réelle independentReviewStep, utilisée par le conducteur
web et planning review-step, les lectures CLI mission status et HTTP snapshot.
Cas absents, chemin devenu dossier et empreinte périmée : zéro processus lancé,
zéro appel consommé, raison et prochaine action identiques web/CLI, historique
et révision inchangés. Preuves restaurées : un processus, un appel et état submitted.
Les sorties détaillées du contrôle moteur sont les observations exécutées.
Aucune capture ni recette de rendu visuel n'est affirmée ici : la modification
concerne la projection Go existante, pas le HTML, le CSS ou le JavaScript.

La supervision et le second agent ont lancé une suite Go en parallèle après
un état de reprise mal observé : ce doublon est une erreur de supervision,
à mesurer au RETEX ; il ne doit pas être attribué au fournisseur.
