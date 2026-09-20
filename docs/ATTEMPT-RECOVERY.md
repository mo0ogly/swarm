# Reprendre une tâche dont les tentatives sont épuisées

Dans **Pilotage des agents**, la carte bloquée et son détail proposent
**Autoriser une tentative supplémentaire** lorsque toutes les tentatives prévues
sont consommées. Indiquez le motif et la correction à apporter : sources manquantes,
précondition vérifiée ou travail partiel à reprendre. Une répétition à l’identique
ne résout pas le blocage.

La confirmation augmente le plafond d’une unité, sans dépasser trois tentatives.
Elle conserve les tentatives, rapports, contrôles, avis et budgets d’outils. La tâche
reste bloquée et non validée. Le formulaire de lancement s’ouvre ensuite : il
présente les conditions encore à résoudre avant de démarrer. Aucun fournisseur
n’est lancé par l’autorisation elle-même. Échap ou Fermer annule la saisie sans
modifier le travail. Un écran périmé est refusé : fermer et rouvrir la décision.

L’opération est refusée si un agent ou une revue travaille encore, si des tentatives
restent disponibles ou si trois ont déjà été autorisées. Au-delà, il faut revoir le
périmètre avec son responsable ; le bouton ne permet pas une boucle sans limite.

## Équivalent CLI

Lire d’abord la révision courante avec `swarm --json work show WORK`. Créer un fichier :

```json
{
  "schema_version": 1,
  "event_id": "identifiant-unique-de-la-decision",
  "expected_revision": 42,
  "task_id": "ma-tache",
  "reason": "Décision explicite après examen du blocage",
  "recovery_instruction": "Transmettre les sources manquantes du candidat puis rejouer les mêmes contrôles."
}
```

```sh
swarm planning extend-attempt WORK --input reprise.json
```

Cette décision opérateur est distincte d’une modification du contrat hiérarchique.
Elle ne change ni les critères, ni les dépendances, ni le responsable. Un même
`event_id` est rejouable sans accorder deux tentatives ; une révision périmée est refusée.
L’événement `task.attempt-extension` conserve le motif et la consigne.

## Donner au vérificateur les sources du candidat

Pour une intégration Git gérée, le producteur peut ajouter
`docs/ID-DE-TACHE.review-context.json` dans le sous-projet de la mission :

```json
{"version":1,"files":["managed_review.go","managed_review_test.go","tests/engine_acceptance.cjs"]}
```

Les chemins de `files` partent de la racine du dépôt. Le moteur lit les fichiers
complets dans le commit candidat contrôlé, jamais dans une copie modifiable. Il
joint leur contenu, chemin, objet Git et empreinte SHA-256 au contexte de revue.
Le choix des fichiers ne prouve pas que le contexte suffit : le vérificateur doit
maintenir un verdict inconnu si une preuve nécessaire manque.

Limites : manifeste de 8 Kio ; 24 fichiers distincts ; 96 Kio par fichier ; 128 Kio
pour les sources ; limite globale de revue de 192 Kio. Chemins invalides, fichiers
absents, liens symboliques, contenu binaire et dépassements sont refusés sans
troncature ni appel de revue. Un manifeste absent conserve le parcours existant
(diff, rapports et reçus des contrôles). Cette livraison est un contexte explicite,
pas une surveillance générale des fichiers ni un message direct entre agents.
