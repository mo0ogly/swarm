# Prérequis métier et reprise des contrôles

Ces changements proviennent de l’audit du banc de facturation du 7 octobre 2026.
Le candidat est basé sur `main` (`cbf0075`), pas sur le binaire historique du banc
(`2ef5330`, sources modifiées). Ils ne changent pas les résultats déjà publiés.

## Garanties avant lancement

L’opérateur peut déclarer des prérequis entre les exigences du travail :

```json
{
  "schema_version": 1,
  "event_id": "billing-create-1",
  "expected_revision": 0,
  "title": "Facturation",
  "objective": "Préparer puis régler les factures autorisées",
  "scope": "Banc synthétique isolé",
  "criteria": ["Lot approuvé", "Règlement exact et unique"],
  "requirement_prerequisites": {"req-2": ["req-1"]}
}
```

Création : `swarm --root PROJET work create --input travail.json`.
Consultation : `swarm --root PROJET --json work show ID`.
Le même contrat JSON est exposé par les opérations publiques de travail.
Configurer les règles avant la création des tâches ; ensuite elles ne peuvent
pas être supprimées ou changées par `work update`. Les cycles, doublons et
exigences inconnues sont refusés. Les travaux sans règles conservent leur
comportement existant.

Dans une mission hiérarchique, une tâche affectée à `req-2` est retenue tant
qu’aucune **autre tâche acceptée avec preuve fraîche** ne couvre `req-1`, même
si le planificateur omet `depends`. Une dérogation `waived` ne suffit pas.
L’ordonnanceur, le lancement manuel d’un agent et la transition publique vers
`running` appliquent la règle avant de réserver une tentative. Une tâche sans
exigence déclarée est refusée lorsque ces règles sont configurées.

**Limite :** le moteur vérifie une classification déclarée ; il ne déduit pas
qu’une commande arbitraire effectue un paiement. L’opérateur doit contrôler
l’affectation des exigences, les outils et les autorisations du service métier.
Le service de paiement doit lui-même contrôler l’autorisation, l’empreinte du
lot, l’idempotence et les invariants comptables dans sa transaction. Un contrôle
avant lancement ne supprime pas un changement concurrent pendant l’exécution.
Ce correctif ne transforme pas Swarm en système financier qualifié.

## Preuve devenue périmée

Une dépendance acceptée dont les fichiers ont changé retient la branche. Le
responsable reçoit désormais un événement `dependency_stale`, y compris lorsque
son périmètre est déjà ouvert. Un état de preuve identique produit un seul
événement durable ; les contrôles suivants ne réécrivent pas continuellement la
révision du travail. La tâche acceptée et son historique sont conservés.
Revalider le résultat existant par les opérations publiques appropriées avant
reprise ; une notification ne constitue ni une nouvelle preuve ni une acceptation.

## Distinguer indisponibilité et résultat non conforme

Un contrôle peut déclarer ses codes d’indisponibilité :

```json
{
  "id": "check-lot",
  "command": ["python3", "check_lot.py"],
  "criteria": [1],
  "justification": "Vérifie le lot contre le grand livre disponible.",
  "timeout_seconds": 15,
  "environment_exit_codes": [3]
}
```

Ces codes doivent être uniques et compris entre 1 et 255. Le code zéro n’est
jamais un échec d’environnement. La politique reste autorisée par l’opérateur,
via `validation preview/apply` ou les contrôles de la planification.
Le reçu conserve `environment_failure: true`, le code, les empreintes et la
sortie. La tâche reste bloquée ; aucune correction automatique du producteur
n’est déclenchée par cette panne. Les codes non déclarés conservent leur
interprétation de résultat non conforme. Déclarer les codes correctement est
une responsabilité de l’auteur du contrôle.

Après réparation et vérification réelle du service, utiliser
`validation recheck-preview/recheck-apply` sur **la même tentative terminée**.
Pour conserver une politique identique après une panne identifiée, renseigner
`environment_recovery_reason` (8 à 500 caractères) dans la demande avec
`recheck_completed: true`, `intent: "replace"`, la politique courante et la
révision actuelle. Reprendre le `preview_token` exact dans l’application.
Le motif est une déclaration traçable de l’opérateur, pas une vérification
indépendante du rétablissement du service. Les contrôles doivent réussir à
nouveau et la revue/gate courantes restent requises lorsque la mission les exige.
Les anciens reçus d’échec restent conservés. Pas de boucle automatique identique.

## Contention SQLite

Les transactions de mutation réservent déjà le verrou d’écriture avant lecture
sur `main`. Une reprise bornée couvre désormais aussi les erreurs SQLite BUSY
(code de base 5, dont BUSY_SNAPSHOT), avec le même événement, la même demande et
la même révision attendue. La réservation d’une revue ou d’une décision n’est
pas répétée après un événement déjà enregistré. Les conflits de révision et les
autres erreurs ne sont pas transformés en succès. Un verrou durable finit par
produire une erreur réelle, pas une attente infinie.

Les valeurs par défaut sont dans `config/storage-retry.json`. La CLI et le
serveur lisent la surcharge locale `.swarm/storage-retry.json` :

```json
{"schema_version": 1, "busy_retries": 3, "busy_retry_delay_ms": 25}
```

`busy_retries` : 0 à 10 reprises supplémentaires ; délai : 0 à 1000 ms.
Le délai SQLite de connexion existant peut s’ajouter à chaque essai. Le même
réglage est utilisé pour la préparation d’un agent. Écrire la surcharge par
remplacement atomique ; fichier symbolique, configuration inconnue ou valeur
invalide sont refusés. Ce réglage se fait par fichier, **aucun nouveau formulaire
administrateur n’est livré ici**. Il ne relève ni les budgets fournisseurs ni
les tentatives de production.

## État de l’audit et du banc

| Point | Traitement |
| --- | --- |
| D1 : rapport dans l’espace de tentative | Protection déjà présente sur `main`, test sans copie à la racine |
| D2 : dépendance périmée silencieuse | Motif visible et événement au responsable, sans effacement de preuves |
| D3 : panne de contrôle classée métier | Codes déclarés, conservation du résultat et recontrôle explicite |
| D5 : contention de décision/revue | Verrou précoce existant plus reprise transactionnelle bornée et idempotente |
| D6 : dépendance omise par le planificateur | Règles d’exigences de l’opérateur imposées avant lancement |
| D4 : arrêt brutal, code 137 | Reprise explicite conservée ; pas de relance aveugle |

Le banc refuse désormais les sorties JSON absentes, non objets ou incomplètes du
client réel. Une réponse client valide n’est **pas** une preuve de succès métier.
Après erreur, les coûts déjà déclarés et les appels sans coût sont récupérés du
journal. Les lignes valides d’un journal partiellement corrompu sont conservées ;
la campagne s’arrête si la consommation ne peut pas être récupérée intégralement.
Les résultats historiques ne sont pas recalculés ni remplacés. Les tests avec
scripts et fixtures ne prouvent pas l’autonomie d’un fournisseur réel.
