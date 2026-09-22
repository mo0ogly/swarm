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

### Doublons exacts entre diff et sources

Si même un lot individuel dépasse la limite, le moteur peut transmettre les
sources par segments. Les portions déjà présentes dans un hunk du diff sont
référencées par fichier et numéro de hunk ; les autres portions restent littérales.
Chaque segment porte sa taille et son empreinte, ainsi que le fichier reconstitué.
Le diff complet, les suppressions, rapports, critères et reçus restent présents.
Il ne s'agit ni d'un résumé ni d'une réduction des preuves : la concaténation
restitue exactement le fichier candidat. Une correspondance partielle, ambiguë
ou un patch non pris en charge conserve le texte littéral. Un dossier encore
trop grand reste refusé avant tout appel.

Ce transport est essayé seulement après l'échec du découpage antérieur complet.
Les plans de revue déjà valides conservent donc exactement leurs lots et prompts ;
les avis acquis ne sont pas invalidés par une optimisation de transport inutile.
Le contexte canonique enregistré reste inchangé. Les tests reconstruisent les
octets dans un processus distinct et rejettent une altération de leur empreinte.

### Reprendre un refus avant tout appel

`planning retry-review WORK --input requête.json` couvre aussi un résultat terminé
retenu par le précontrôle de taille, sans avis indépendant enregistré. La requête
contient les champs habituels `schema_version`, `event_id`, `expected_revision`,
`task_id` et `reason`. Conserver le même `event_id` pour rejouer la même demande.

Avant de réarmer l'intégration, le moteur vérifie la dernière tentative, la fin
du producteur, le résultat Git conservé, le candidat et le contrat, les reçus,
la taille de tous les lots et le budget nécessaire. Le motif initial doit être
un refus de taille ; un contrôle échoué ou un avis défavorable ne passe pas par
cette reprise. Si la précondition n'est pas corrigée, aucune reprise n'est inscrite.
Le conducteur réexamine ensuite le même résultat. Aucun nouveau producteur,
remboursement ou hausse de plafond ; l'acceptation attend toujours la revue réelle.

## Reprendre une intégration après un contrôle en échec

`planning retry-integration WORK --input request.json` reprend le **résultat Git
conservé**, sans nouvel exécutant. Cette opération est distincte de `retry-review` :
elle rejoue les contrôles cumulés avant de demander une nouvelle revue indépendante.
Elle exige une base validée différente, descendante de la base du contrôle échoué,
et un producteur terminé dont la fin du processus est confirmée.

La demande JSON contient `schema_version: 1`, `event_id`, `expected_revision`,
`task_id`, `agent_id`, `attempt_id`, `result_commit`, `expected_candidate` et
`reason` (8 à 2000 caractères). Les identités proviennent de l’état public ; elles
ne doivent pas être devinées. CLI et API planning appliquent le même contrat.

Une seule reprise est permise par résultat et base. Réenvoyer le même événement
est sans effet supplémentaire. La réservation, le motif, l’échec et sa provenance
restent dans `integration_retries`. Les tentatives, budgets et preuves ne sont pas
remis à zéro. Un dossier de revue déjà constitué doit suivre sa propre récupération :
il n’est jamais supprimé par cette opération. Une modification de base ou de contrat
après réservation arrête la reprise.

Pour les anciens échecs sans diagnostic, le moteur exige l’événement d’échec exact
et une publication antérieure démontrant la base. `legacy_missing_output: true`
signale que la sortie initiale manque ; aucune sortie ni cause n’est reconstruite.
Sans cette provenance, la reprise est refusée. Une réservation réussie ne valide
pas la tâche : seuls les nouveaux contrôles et l’avis indépendant permettent
la publication du nouveau candidat.

## Dossiers cumulatifs : base acceptée et revue incrémentale

Renvoyer tout le diff depuis le début de la mission à chaque nouvelle revue fait
croître le dossier avec l'historique, même si le nouveau changement est petit.
Le moteur conserve d'abord le parcours historique lorsqu'il tient dans la limite.
Si même les lots unitaires débordent, il peut construire une revue incrémentale :

1. vérifier les avis favorables, tentatives, contrats et politiques de chaque
   tâche déjà acceptée sur la base publiée ; une étiquette « acceptée » ne suffit pas ;
2. joindre ces avis comme **preuves de la base précédente**, avec leurs références
   et empreintes, et conserver leurs contextes et réponses originaux ;
3. fournir tout le diff entre cette base et le nouveau candidat, tous les rapports,
   tous les critères et tous les contrôles cumulés exécutés sur le nouveau candidat ;
4. relire les sources déclarées depuis le nouveau candidat, y compris celles qui
   n'apparaissent plus dans le diff incrémental ;
5. obtenir de nouveaux avis sur chaque critère et les régressions possibles. Un avis
   antérieur ne constitue jamais une acceptation du nouveau candidat.

`accepted_baseline` rend ce protocole explicite dans le contexte conservé et les
consignes du vérificateur. Le moteur contrôle récursivement les empreintes des
preuves historiques utilisées, pendant la revue et avant publication. Une preuve
manquante ou modifiée invalide la chaîne. Une reprise ne modifie pas les dossiers
antérieurs ni leurs plans de lots. Les dossiers historiques déjà admissibles
conservent exactement leur transport.

Ce protocole remplace la réinspection intégrale de tout l'historique par un examen
indépendant des changements et de leurs effets à partir d'une base vérifiée. Le
vérificateur doit demander une preuve (`unknown`) si les sources actuelles et les
avis de base ne suffisent pas. Le plafond reste 192 Kio par appel, les budgets et
l'exigence d'un avis indépendant sur le même candidat sont inchangés. Un changement
individuel ou un ensemble de sources encore trop grand reste refusé avant tout
appel : cette limite n'autorise jamais une troncature ou une validation partielle.
