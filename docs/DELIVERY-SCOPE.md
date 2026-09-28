# Réduire une remise trop large

Le moteur retient une nouvelle revue lorsque le delta depuis le dernier candidat
accepté dépasse **100 fichiers**. C’est une limite de remise Swarm, pas une règle
Cursor ni une limite de contexte du modèle. Augmenter le budget ou découper les
appels de revue ne contourne pas cette protection. Les avis déjà obtenus ne sont
pas annulés rétroactivement.

Le résultat original reste conservé dans Git. Aucun fichier n’est supprimé,
aucune tentative remboursée et aucune acceptation accordée. Le moteur ne peut pas
déduire automatiquement quels fichiers sont nécessaires : cette sélection relève
du responsable, qui doit examiner les dépendances et les critères de la tâche.

## CLI et API

Après un refus de précontrôle de revue, exporter le périmètre :

```sh
swarm --json planning scope-preview WORK --input task.json
```

`task.json` : `{"task_id":"TASK"}`. La réponse contient `revision`,
`accepted_base`, `candidate_commit`, `changed_files` et `file_limit`.

Préparer une sélection explicite, puis exporter son patch :

```json
{
  "schema_version": 1,
  "task_id": "TASK",
  "expected_revision": 123,
  "expected_candidate": "SHA_DU_CANDIDAT",
  "scope_files": ["fichier.go", "fichier_test.go", "docs/TASK.md", "docs/TASK.delivery.json"]
}
```

```sh
swarm --json planning scope-patch WORK --input selection.json > selection-patch.json
```

La réponse expose `patch`, `selected_files` et `deferred_files`. Elle ne modifie
ni copie de travail, ni index, ni preuve, ni budget. Le patch s’applique sur une
copie **exacte de `accepted_base`**, pas sur une branche arbitraire. Les chemins
sont littéraux ; les renommages sont représentés par suppression et ajout, dont
les deux chemins doivent être sélectionnés si le renommage est voulu.

API équivalente :

- `GET /api/v1/planning?work=WORK&task=TASK&action=scope-preview`
- `POST /api/v1/planning?work=WORK&action=scope-patch` avec la sélection JSON.

L’authentification locale habituelle s’applique. Aucun nouveau bouton web n’est
introduit par ce changement.

## Remise et limites

Appliquer le patch dans une copie séparée de la base, vérifier les dépendances,
compiler, tester et mettre à jour le rapport et le bilan de livraison. Préserver
le résultat original avant de reporter la sélection dans la copie attribuée.
`planning revise-recovered-result` accepte une correction explicitement attribuée
d’une livraison complète mais trop large, arrêtée, sans revue engagée. Elle exige
la révision et l’arbre Git examinés, puis les contrôles et une revue indépendante.
Le patch exporté seul n’est jamais une preuve d’acceptation.

Les fichiers différés doivent devenir des remises séparées avec leurs propres
contrôles. Un delta de 100 fichiers ou moins peut encore être trop volumineux ou
incohérent : les contrôles de contenu, transport, budget et métier restent actifs.
Les contrôles locaux peuvent avoir été exécutés avant ce précontrôle de revue.

## Diagnostic de découpage

`scope-preview` expose désormais `decomposition`. Il détecte aussi un diff unique
trop volumineux et les remises mêlant plusieurs responsabilités. Il décrit cinq
options : exigences, composants, dépendances, spécialités et volume. Les lots par
chemin sont des suggestions : `state=proposal_only`, dépendances non examinées.
Ce diagnostic ne lance pas un sous-planificateur et n’autorise aucune revue.
La limite actuelle reste active jusqu’à une remise réduite ; un futur plan
sémantique validé pourra offrir une autre voie que la réduction.
