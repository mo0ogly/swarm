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

## Réparer un résultat interrompu sans réécrire son historique

Une limite d’exécution domine les erreurs d’outils antérieures dans le diagnostic.
Le moteur transmet au responsable un bilan JSON d’arrêt dans `.swarm/interruptions/`,
avec l’identité, la copie, la cause, les compteurs et la dernière action observée.
Ce bilan est un constat du moteur, pas un rapport produit par l’agent ni une preuve
de réussite. Il ne fige pas les fichiers de la copie. En cas d’échec de stockage,
une alerte est conservée si le journal reste accessible ; aucun bilan n’est inventé.

Les consignes du nouvel exécutant désignent une seule racine de code et demandent
un rapport dès le début, actualisé régulièrement. C’est un cadrage explicite, pas
un confinement système du fournisseur ni une garantie qu’il suivra ces consignes.

Pour une réparation externe expressément demandée, un opérateur peut compléter
la copie arrêtée, vérifier toutes les obligations et remettre **une seule** révision
par `planning submit-recovered-result`. Cette opération ne lance aucun exécutant,
ne change aucun plafond et conserve le processus et la tentative interrompus.
Le reçu transmis au vérificateur identifie `external_repair` et son auteur.
Elle ne constitue donc pas une démonstration d’autonomie.

Après examen du diff et des preuves, indexer la copie avec `git add -A`, relever
`git write-tree`, puis fournir à la commande un JSON contenant `schema_version: 1`,
`event_id`, `expected_revision`, `task_id`, `agent_id`, `attempt_id`,
`confirm_recovery: true`, `result_tree` et un `reason` explicite. Une modification
des fichiers depuis cet examen, une tentative active ou remplacée, une déclaration
de livraison incomplète ou un rapport absent refuse la remise. La déclaration
`docs/TACHE.delivery.json` est obligatoire, y compris pour une ancienne tentative.

Les contrôles et la revue indépendante portent ensuite sur le même candidat.
Un refus reste un refus ; aucun cinquième essai n’est créé. Le rejeu du même
événement reprend la révision enregistrée sans nouvelle production ni nouvelle
revue implicite après refus. Une autre soumission de réparation pour cette tâche
est refusée ; les incidents de revue utilisent le parcours explicite déjà existant.

Un contexte de revue trop volumineux peut provenir des lignes inchangées du diff.
Au-delà de 160 Kio de contexte sérialisé, le moteur emploie trois lignes de contexte
Git au lieu de quarante. Il conserve toutes les lignes modifiées, les fichiers de
sources complets et les reçus. La limite finale de 192 Kio reste obligatoire.
Le rejeu explicite de la même remise externe peut reprendre cet échec préalable
si aucune revue de cette tentative n’a commencé ; un avis déjà rendu reste lié
à son candidat et ne déclenche aucun nouvel appel implicite.

### Corriger un résultat refusé sans relancer le producteur

`planning revise-recovered-result` remet une correction externe explicitement
examinée, liée au dernier `review_id` en `changes_requested`. Fournir les mêmes
champs que `submit-recovered-result`, plus cet identifiant et la révision courante.
L’arbre doit différer du résultat refusé et correspondre aux fichiers examinés.
Le moteur conserve l’ancien avis, son reçu et ses références Git ; le nouveau
candidat repasse les contrôles et une revue dans le budget existant.
Aucune tentative de production ni aucun compteur n’est réinitialisé. Un rejeu
de la même demande ne consomme pas une seconde revue. Une tâche déjà acceptée
ne peut pas être remplacée par cette opération. Cette réparation est une
intervention externe tracée, pas une production autonome.

### Erreur de format de l’avis indépendant

Une citation `pass` doit être un extrait exact et contigu des preuves fournies.
Une paraphrase ou plusieurs extraits assemblés ne sont pas acceptés. Le moteur
conserve désormais la réponse du vérificateur, même rejetée, dans un fichier
privé lié à l’avis (`reply_path`, `reply_sha256`) et précise la tâche/le critère
fautif. Ce fichier n’est pas une approbation. Les anciens avis produits avant
ce correctif peuvent ne pas disposer de cette réponse brute.

Après une correction démontrée du format ou de la précondition, `planning
retry-review` permet une reprise explicite dans le budget existant. Aucun rejeu
payant automatique n’est déclenché. Les reçus, contextes et rapports sont relus
pendant l’inférence et avant publication ; une modification rend l’avis périmé.

Après `retry-review`, le conducteur reprend aussi une réparation externe dont
le producteur reste interrompu : seul le résultat explicitement remis et en
attente d’intégration est repris. Le processus historique et ses tentatives
restent inchangés. Une remise refusée non réarmée ne déclenche aucun nouvel avis.

### Dossier cumulé trop volumineux

Le moteur garde la limite de contexte. Après réduction des seules lignes de
contexte inchangées du diff, il peut retirer une source complémentaire dupliquée
si le fichier est entièrement nouveau et si son contenu complet correspond
exactement au diff intégral, avec le même objet Git. Le diff, les rapports, les
reçus et toutes les lignes modifiées restent transmis. Aucun fichier modifié
préexistant n’est raccourci. Un contexte encore trop grand reste refusé avant
appel fournisseur ; cette déduplication n’autorise aucune troncature.
