# Délai de la revue indépendante

[English](en/REVIEW-TIMEOUT.md)

Une production terminée et des tests réussis peuvent rester bloqués si le
vérificateur ne répond pas à temps. Le moteur conserve le candidat, le rapport,
le reçu des contrôles et l'appel consommé ; il ne fabrique pas un avis favorable.

Le délai historique est de **90 secondes**. Il reste inchangé pour les missions
existantes tant qu'une décision explicite ne le modifie pas. La configuration
`planning.reviewer.timeout_seconds` autorise 1 à 900 secondes. Chaque nouvelle
revue conserve son propre `timeout_seconds` dans son reçu d'état.

Le CLI et l'API locale utilisent la même transaction. Exemple de demande :

```json
{
  "schema_version": 1,
  "event_id": "review-timeout-decision-unique",
  "expected_revision": 50,
  "review_timeout_seconds": 300,
  "reason": "Deux revues ont atteint 90 secondes ; décision bornée à cinq minutes, sans modifier les appels autorisés ni le candidat."
}
```

Relire la révision de la mission avant d'envoyer cette demande :

```sh
swarm planning show WORK
swarm planning review-timeout WORK --input delai.json
```

API : `POST /api/v1/planning?work=WORK&action=review-timeout`, avec la même
demande et l'authentification locale habituelle. Aucun bouton de réglage dédié
n'est ajouté au web par cette évolution ; le champ est visible dans ses données.

Le changement est refusé pendant une revue active. Une révision périmée est
refusée ; le rejeu de la même décision ne la duplique pas. Aucun budget d'appels,
nombre de tentatives, verdict, modèle, contrôle ou preuve n'est modifié.

**Modifier le délai ne relance rien.** Après examen de la cause, une reprise
séparée passe par `planning retry-review`, avec tâche, motif et révision courante.
Elle consomme un nouvel appel du budget existant, garde la tentative de production
et le candidat déjà contrôlé. Un avis défavorable ne peut être relancé comme
une panne. Un budget épuisé reste bloqué.

Un délai plus long augmente le temps disponible pour un appel ; il ne démontre
pas que la lenteur est résolue. Les tests avec fournisseur déterministe vérifient
expiration, reprise sur le même SHA, persistance et conservation des compteurs.
Seule la revue réelle peut produire un avis utilisable par cette mission.

## Signal du conducteur pendant la revue

Lorsqu'une production terminée d'un dépôt géré est réconciliée, les contrôles
et la revue se déroulent hors de la boucle du conducteur. Celui-ci continue
d'actualiser son signal et d'examiner les autres missions. Le verrou Git entre
processus et la réservation transactionnelle de revue empêchent un second appel
pour le même traitement. Un contrôle encore en cours n'est pas un résultat accepté.

Un avis `unknown` correspond à des preuves insuffisantes. L'état
`changes_requested` reste non accepté et ne se relance pas comme une panne :
compléter le livrable ou ses preuves dans les limites autorisées, puis faire
vérifier le nouveau candidat. Un délai supplémentaire ne suffit pas à résoudre
un manque de contexte ou de reçus de tests.

## Dossiers cumulatifs et revues par lots

Le moteur essaie d'abord un appel unique. Si le dossier dépasse 192 Kio après
encodage sans perte, il prépare des lots déterministes de tâches. Chaque lot
conserve le même candidat Git, le reçu des contrôles, le diff complet et tous les
rapports et critères cumulatifs. Les sources annexes sont réparties selon leurs
manifestes ; chaque source reste entière. Les limites cumulées de 24 fichiers,
128 Kio de sources et 96 Kio par fichier restent applicables. Si une tâche ne
tient pas dans un lot, le moteur refuse avant tout appel, sans tronquer le dossier.

Le moteur vérifie le budget nécessaire à tous les lots restants avant le premier
appel, puis réserve et compte chaque appel séparément. Il conserve par lot son
identifiant, les tâches examinées, le contexte et son empreinte, la réponse brute,
son empreinte et son état. Un avis inconnu ou défavorable empêche la publication.
L'acceptation exige une couverture exacte de tous les critères et la relecture
des preuves du même candidat ; réussir le premier lot ne valide pas la tâche.

Une interruption exige la reprise explicite existante `planning retry-review`.
Seuls les lots favorables enregistrés durablement peuvent être réutilisés, après
vérification du candidat, du contrat, du fournisseur, de la politique du modèle,
de la méthode et de toutes les empreintes. Un appel payé sans verdict durable
reste consommé et son lot doit être examiné de nouveau dans le budget restant.
Si tous les avis favorables sont déjà durables, terminer leur agrégation ne
consomme pas d'appel supplémentaire, même au plafond. Aucun remboursement ni
nouvelle tentative de production n'est créé par cette reprise.

Ces garanties sont couvertes par des tests isolés avec processus fournisseur
simulé (`managed_review_batch_runtime_test.go`). Ces tests ne démontrent ni la
qualité d'une revue par un modèle réel, ni la réussite d'une mission en cours.
