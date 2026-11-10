# Observation du 10 octobre 2026 : vivacité et supervision du moteur

Statut : diagnostic et correction après observation, hors des campagnes de
facturation pré-enregistrées. Ce cas ne modifie aucune donnée historique ni les
verdicts D1 à D6. Il porte sur la mission d’évolution Cursor
`w-ca937edf4e0abd85f8d3d67b`, tâche `plan-ca937edf4e-T1`.

## Symptôme, cause et attribution

Le producteur avait terminé une première tentative et livré le candidat
`38345bb1805ef56137087b3326d19f2c4d8cf204`. Le vérificateur indépendant a été
interrompu après 90 secondes malgré 70 événements `thinking_tokens`. Le responsable
racine a ensuite reçu les retours `independent_review` et `integration_failed`,
mais son diagnostic a été interrompu par le même plafond total de 90 secondes.

Le routage vers un responsable existait donc. Le superviseur était appelé,
puis le moteur l'arrêtait avant sa décision. Dans la taxonomie QR3, la règle
de durée mal calibrée est la cause établie de ces interruptions. Son inscription
dans le code empêchait le réglage par l'opérateur. L'ancien bail de propriété non
renouvelé constituait une autre limite du mécanisme ; nous ne lui attribuons pas
ces deux interruptions observées à 90 secondes.

Seul le responsable racine est configuré à ce stade de la mission. Les
sous-planificateurs interviennent quand des périmètres leur ont réellement été
délégués. Leur présence dans un schéma ou un plan ne prouve pas leur activation.

## Correction du mécanisme

Les politiques viennent de `config/provider-wait.json`, puis de révisions
persistées par les opérations publiques du projet. Elles sont visibles et
modifiables dans Admin FR/EN, avec aperçu, contrôle de révision, motif et historique.
Les mêmes valeurs sont lues par le moteur, l'API et la CLI.

| Politique active après correction | Valeur | Interprétation |
| --- | ---: | --- |
| Silence du fournisseur | 300 s | Attente sans événement productif reconnu ; réglable, 0 désactive cette surveillance |
| Durée totale maximale | 0 | Aucun arrêt fondé uniquement sur la durée écoulée ; réglable |
| Bail de propriété du responsable | 120 s | Réservation renouvelée pendant l'appel vivant ; ce n'est pas une durée maximale d'exécution |

Le raisonnement et le contenu reconnus réarment le délai de silence.
Initialisation, erreurs, demandes de nouvel essai API et événements inconnus ne
le réarment pas. Cette activité ne vaut ni preuve de progression sémantique ni
avis favorable. Un fournisseur peut rester actif tout en tournant en rond ; les
budgets, l'arrêt opérateur et la validation indépendante restent nécessaires.

Le bail renouvelé conserve titulaire et génération sans nouvel appel de modèle
ni nouvelle activation de planification. Une propriété expirée ou remplacée ne
peut pas être ressuscitée. Une durée totale définie explicitement pour une revue
reste applicable ; la valeur 0 hérite de la politique du projet.

Le [contrat d'architecture](../../../../architecture/PROVIDER-LIVENESS.md) décrit
les événements, transitions, opérations publiques et conditions de reprise.

## Observation après installation sur le serveur réel

Le binaire local a été reconstruit depuis le HEAD
`d4539e0041000a38acb162a17f053e318fb3db5d` avec modifications non commitées.
La revue a été reprise par l'opération publique, sur le même candidat et la même
tentative du producteur. L'ancien échec est conservé ; les plafonds n'ont pas été
augmentés et les appels déjà dépensés n'ont pas été remboursés.

| Événement public, heures UTC | Observation |
| --- | --- |
| 17:29:47.604934808 — début de `review-849606d4679df263de116295` | Deuxième appel de revue, même candidat |
| 17:32:34.000739990 — fin de la revue | 166,40 s ; `changes_requested`, critères 1, 2 et 4 favorables, critère 3 `unknown` |
| 17:32:35.442174384 — activation du responsable | Traitement des retours par le périmètre `root` |
| 17:34:44.831099302 — `planning-claim-d61f03603da48d818b2ba11e-decision` | Décision enregistrée : correction ciblée de T1, après environ 129 s |
| 17:34:48.886596130 — seconde tentative de T1 | Départ automatique du worker `auto-ed3d6ce2b54e02740a68` |

La revue demandait une preuve lisible sur les sources complémentaires : un fichier
cité était absent du contexte soumis au vérificateur. Le responsable a demandé
d'inclure cette preuve dans le nouveau diff, en conservant les éléments déjà
favorables. Ce résultat montre une boucle effective de retour et correction,
pas une acceptation de T1 ni la conformité du moteur à tout le contrat Cursor.

L'[observation structurée](20261010-vivacite-observation.json) conserve les
identités, paramètres, décisions et empreintes utiles sans recopier les prompts
ou la base privée. Les événements sont lus par `swarm --json work show` ; aucune
modification directe de la base n'a servi à la reprise.

La correction installée est postérieure à l'instantané de départ de la mission.
Un [patch contre cet instantané](../../../../plans/engine-cursor-contract/recovery/provider-wait-20261010.patch)
et son [manifeste](../../../../plans/engine-cursor-contract/recovery/provider-wait-20261010.json)
sont conservés pour sa reproduction. Son application a été contrôlée en copie
isolée, et la consigne d'intégration a été enregistrée par `task update` sur T5,
sans modifier ses critères ni ses budgets. L'intégration de ce correctif dans le
résultat géré doit encore être vérifiée et revue ; l'installation sur le serveur
ne vaut pas cette acceptation.

## Vérification et limites

Tests avec fournisseurs simulés : activité au-delà du délai de silence, silence
réel, sorties improductives, durée totale explicite, révocation, persistance,
CAS/rejeu et renouvellement du bail sans double activation. Tests ciblés et
`go test -race` réussis ; `go vet ./...` réussi. Les recettes navigateur isolées
vérifient Admin et les réglages de revue en FR/EN, clair/sombre, clavier et mobile.

La suite globale `go test ./... -timeout 5m` n'est pas qualifiée : manifeste
historique périmé sur `PREPARATION-UX.md`, ancienne assertion exigeant à tort le
refus de 901 secondes, puis dépassement de la durée de la suite. L'assertion a été
corrigée pour refuser une durée négative et son test ciblé passe. Aucun manifeste
de preuve n'a été réécrit pour annoncer artificiellement un succès global.

Il s'agit d'un seul incident de terrain et de sa reprise, observés après le
défaut. Aucune réduction statistique du taux de blocage, économie de coût,
performance des sous-planificateurs ou propriété générale de vivacité n'est
démontrée. Les cinq délais historiques de la section 6.4 gardent leur cause
initiale inconnue : ce nouveau cas ne permet pas de les réattribuer.

## Essai comparatif à pré-enregistrer

Comparer durée totale fixe, surveillance du silence et diagnostic supervisé,
à tâches, fournisseurs, budgets et critères d'acceptation identiques. Inclure :
raisonnement long utile, fournisseur silencieux, activité répétitive sans progrès,
quota, pause opérateur, perte de bail, concurrence et preuve insuffisante.
Mesurer séparément interruptions à tort, temps sans progrès, coût, décisions
appliquées, reprises et acceptations fraîches. Une décision proposée ou une
activité détectée ne doit jamais être comptée comme une livraison acceptée.
