# Comprendre une attente de quota IA

[English](en/PROVIDER-QUOTAS.md) · [Guide utilisateur](../GUIDE-UTILISATEUR.md)

Un refus de quota signifie que le fournisseur n’accepte plus cet appel. Il ne
prouve ni un défaut du code produit, ni la réussite de la tâche. Une tentative
interrompue reste non validée et ses fichiers sont conservés pour examen.

## Ce que fait le moteur

Le moteur reconnaît les événements structurés de refus du fournisseur Claude :
`rate_limit_event` avec `status=rejected`, ou un résultat avec `is_error=true`
et `api_error_status=429`. Un avertissement de consommation, un nombre 429 dans
une sortie d’outil ou une phrase dans un rapport ne déclenchent pas cette attente.

L’heure de reprise vient exclusivement de `resetsAt`, affichée en UTC. Le moteur
ne devine pas une date à partir d’une phrase telle que « resets 10:50pm ».
L’attente est conservée dans l’état local du projet, y compris après redémarrage.
Pendant cette attente, les nouveaux appels des exécutants, des responsables et
du vérificateur utilisant ce fournisseur sont retenus avant leur réservation.

Un appel déjà réservé quand le refus devient connu peut être arrêté avant son
exécution. Les compteurs de tentatives et d’appels restent consommés. Une réserve
financière estimative pour un processus jamais démarré peut être libérée selon
les règles habituelles ; cette libération ne redonne aucune tentative et ne
relève aucun plafond.

Une production achevée qui attend seulement une revue conserve son candidat et
ses contrôles : le quota ne lui impose pas de refaire son travail. Une revue
effectivement appelée puis rejetée conserve son échec et son appel consommé ;
sa reprise doit être explicite et rester dans le budget.

## Voir la cause exacte

Le statut partagé par le cockpit et le CLI expose `provider_cooldowns`, le refus,
l’échéance connue et la prochaine action. Les compteurs d’agents et de résultats
continuent d’indiquer leurs faits propres : attente de quota ne signifie pas
mission terminée.

```sh
swarm --root /chemin/du/projet --json providers cooldown show claude
swarm --root /chemin/du/projet --json mission status ID_TRAVAIL
```

L’expiration d’un délai autorise une nouvelle évaluation des conditions ; elle
ne garantit pas que le fournisseur sera disponible. Une mission en pause reste
en pause. Une tâche ayant atteint son plafond reste bloquée après l’échéance.

## Si aucune heure n’est fournie

Vérifier le compte et l’accès au fournisseur. La levée explicite de cette attente
exige le `digest` courant renvoyé par `show`, un identifiant d’événement unique
et un motif décrivant cette vérification :

```json
{
  "schema_version": 1,
  "event_id": "verification-compte-001",
  "expected_digest": "DIGEST_RENVOYE_PAR_SHOW",
  "reason": "Accès au compte vérifié ; autoriser une nouvelle vérification du fournisseur."
}
```

```sh
swarm --root /chemin/du/projet --json providers cooldown clear claude --input verification.json
```

Cette commande enregistre le motif et l’opérateur local. Elle refuse de lever un
délai connu encore futur, ne lance aucun agent, ne valide aucun résultat et ne
prouve pas que le quota est rétabli. Un état modifié depuis `show` doit être relu.

## Périmètre actuel

La protection concerne les processus du moteur : exécutants, planificateurs et
vérificateur indépendant. Les assistants de page et de préparation ne sont pas
encore reliés à cette attente. Elle est partagée par identifiant de fournisseur
dans le même projet ; elle ne synchronise pas automatiquement des comptes communs
utilisés sous plusieurs identifiants ou dans des projets distincts.

Les tests à réponses simulées vérifient les refus, la persistance, les budgets et
les reprises. Ils ne démontrent ni la disponibilité d’un compte réel, ni une
mission autonome achevée.
