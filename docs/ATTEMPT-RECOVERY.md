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
restent disponibles ou si trois ont déjà été autorisées. Après trois essais, le parcours exceptionnel ci-dessous est distinct ; aucune
augmentation automatique du plafond n’est possible.

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

## Après trois essais : autoriser une correction précise

**Préparer un essai correctif** apparaît dans la carte, le diagnostic de reprise
et le détail de la tâche. Le formulaire reprend le dernier refus indépendant et
propose une consigne modifiable. Lire le diagnostic ne consomme aucun appel IA et
ne change aucun plafond. La proposition reprend les constats ; elle ne garantit
pas que le diagnostic du vérificateur soit complet ou exact.

**Autoriser cet essai correctif** accorde exactement un quatrième essai à cette
tâche. La mission active le prend en charge quand les conditions de départ sont
réunies ; la pause, les dépendances, les réservations, les contrôles de stockage,
les budgets et la disponibilité du vérificateur restent applicables. Le résultat
corrigé doit obtenir des contrôles et un nouvel avis indépendant avant acceptation.

Le moteur exige une tâche bloquée avec trois essais consommés, un refus de revue
lié à la dernière tentative, un vérificateur configuré avec un budget disponible,
une nouvelle consigne, un motif et une confirmation explicite. Aucun agent ne doit
encore travailler sur cette tâche. L’opération est atomique et rejouable : deux
clics n’accordent qu’un essai ; un formulaire périmé est refusé. L’événement
`task.corrective-recovery` et `task.corrective_recovery` conservent l’auteur, la
date, l’avis, la tentative, la révision, le motif et la consigne.

Il n’existe pas de cinquième essai dans ce parcours, même après modification de
la tâche. Ni le planificateur ni l’assistant de page ne peuvent accorder cette
exception par une proposition. Un opérateur local doit l’autoriser ; l’API/CLI
locale demeure dans le périmètre de confiance de cet opérateur.

```json
{
  "schema_version": 1,
  "event_id": "reprise-corrective-unique",
  "expected_revision": 42,
  "task_id": "ma-tache",
  "review_id": "identifiant-du-dernier-avis",
  "attempt_id": "identifiant-de-la-derniere-tentative",
  "confirm_recovery": true,
  "reason": "Correction ciblée après examen du refus indépendant",
  "recovery_instruction": "Corriger les preuves manquantes identifiées dans cet avis et rejouer tous les contrôles du candidat corrigé."
}
```

```sh
swarm planning authorize-recovery WORK --input reprise-corrective.json
```

Cette commande autorise une reprise susceptible de démarrer avec le conducteur
actif. Examiner le JSON avant de l’envoyer. Elle ne relève aucun budget d’outils,
de coût ou de revue et n’efface aucun essai. Les étapes non concernées gardent
leur propre plafond. Un environnement en erreur exige toujours ses vérifications.

## Ce que l’exécutant reçoit lors d’une reprise

Le moteur prépare automatiquement un dossier dans `.git/swarm-recovery.json`
de la **nouvelle copie isolée**. La consigne de l’agent indique son emplacement
et son empreinte SHA-256. Le CLI, le web et le conducteur passent par cette même
préparation ; aucun transfert manuel de correctif n’est nécessaire lorsqu’un
résultat Git a déjà été enregistré par le moteur.

Le dossier lie la mission, la tâche, l’agent et la tentative précédents à la
correction du responsable, au motif d’arrêt et, s’ils existent, à l’avis indépendant
complet et au reçu correspondant. Le reçu est relu et son empreinte vérifiée.
Le résultat Git attribué à cette tentative est importé dans la nouvelle copie
sous `refs/swarm/recovery/previous`. Son rapport et ses changements sont donc
consultables localement avec `git show` et `git diff`, sans lire ni modifier les
fichiers de l’ancien agent. Les identifiants de base et de résultat sont dans le JSON.

La nouvelle copie démarre toujours sur le candidat validé courant. L’exécutant
examine, réutilise et corrige les changements nécessaires dans sa propre copie ;
il doit résoudre les éventuels conflits. Le résultat refusé n’est pas publié ni
appliqué automatiquement. Le dossier reste dans les métadonnées Git : il ne
pollue pas les fichiers du livrable. Les nouveaux contrôles, le bilan par critère
et la revue indépendante restent obligatoires sur le nouveau candidat.

Si aucun résultat Git immuable n’a été enregistré, le dossier l’indique : il ne
prétend pas récupérer les fichiers non remis. Une attribution incohérente, un reçu
altéré ou un dossier préparé modifié empêche le départ et conserve les copies.
Le dossier est limité à 512 Kio, sans troncature. Cette passation ne consomme aucun
appel IA et n’accorde aucune tentative supplémentaire. Elle facilite une reprise
déjà autorisée ; elle ne remplace pas la décision sur un plafond épuisé.

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
