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
