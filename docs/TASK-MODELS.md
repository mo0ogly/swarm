# Un modèle différent pour chaque tâche

Dans la liste des tâches, ouvrir **Modèle de la tâche**. Le bouton est aussi
accessible depuis le détail du graphe et la liste des agents.
Choisir **Choisir pour cette tâche**, le fournisseur et le niveau. Le modèle
résolu et l’effort éventuel s’affichent avant **Prévisualiser le modèle**, puis
**Enregistrer le modèle**. Avec la politique Claude actuelle, Standard correspond
à Sonnet et Exigeant à Opus ; la politique du fournisseur reste la référence.

Le choix est persistant et utilisé pour les prochains départs manuels et
automatiques. Un lancement manuel demandant un fournisseur ou un niveau
contradictoire est refusé. Choisir **Hériter du profil de lancement** retire
uniquement cette surcharge : le profil propre à la tâche, sinon celui de la
mission, s’applique à nouveau. Le reste du profil (répertoire, durée, outils,
consigne) reste inchangé. Sans profil utilisable, il faut encore configurer le lancement.

Les cartes distinguent **Prévu**, **Hérité** et **Utilisé**. Pour une tentative
existante, Utilisé désigne la sélection enregistrée au départ, pas une preuve
indépendante de la version interne du fournisseur. Un alias `opus` ou `sonnet`
ne permet pas d’inventer un numéro de version. Le détail conserve le modèle
prévu même lorsque la carte présente la dernière tentative.

## CLI commun au moteur web

```sh
swarm task-model show WORK --json
swarm task-model preview WORK --input model.json --json
swarm task-model apply WORK --input model.json --json
```

`show` renvoie le travail, sa révision et `available_models`, avec les modèles,
efforts et empreintes de politiques disponibles, sans appel IA. Copier
`available_models.claude.exigeant.policy_hash` dans la demande :

```json
{"schema_version":1,"event_id":"model-choice-001","expected_revision":12,"task_id":"t1","provider":"claude","level":"exigeant","model_policy_hash":"EMPREINTE_RENVOYEE_PAR_SHOW","inherit":false}
```

Pour revenir au profil : même enveloppe avec `inherit:true`. Une page ou révision
périmée est refusée. L’événement rejoué à l’identique ne crée pas une seconde
modification. Une politique fournisseur modifiée après sauvegarde bloque le
départ : examiner puis enregistrer à nouveau le choix, sans substitution silencieuse.
Une tentative active ou une revue en cours interdit la modification.

## Portée et limites

Ce réglage ne donne pas de tentative supplémentaire, n’augmente aucun budget,
ne valide aucun résultat et ne modifie pas les tentatives précédentes.
Il ne change pas les modèles du planificateur, des sous-planificateurs ou du
vérificateur : ces rôles ont un contrat distinct. Leurs modèles se règlent dans le panneau des responsables décrit ci-dessous.
Les propositions automatiques du planificateur ne sont pas livrées. Le choix d’un modèle exact ou de son effort se règle dans
**IA et connexions → Configurer les niveaux** ; la tâche choisit ensuite un niveau.


## Planificateur, sous-planificateurs et vérificateur

Mettre la mission en pause. Dans **L’équipe : qui décide, qui réalise, qui vérifie**,
chaque responsable possède **Modèle du responsable**, et le vérificateur possède
**Modèle du vérificateur**. Choisir fournisseur et niveau, prévisualiser puis
enregistrer. Réactiver ensuite la mission uniquement si ses départs restent autorisés.
Sauvegarder un modèle ne reprend pas la mission et n’efface aucun échec.

Chaque responsable conserve son propre choix. En l’absence de surcharge, il
hérite du modèle de planification enregistré à la création de la mission,
**pas du choix personnalisé de son parent**. Modifier la racine ne change donc
pas silencieusement ses enfants. Le vérificateur exige toujours un choix explicite.
Les cartes de rôles montrent le modèle configuré. `last_model` du périmètre
conserve la sélection de sa dernière réservation ; les nouveaux avis conservent
`model_route`. Les données anciennes sans ce champ restent inconnues.

```sh
swarm role-model show WORK --json
swarm role-model preview WORK --input role-model.json --json
swarm role-model apply WORK --input role-model.json --json
```

Exemple pour un sous-planificateur existant :

```json
{"schema_version":1,"event_id":"role-model-001","expected_revision":12,"scope":"validation","reviewer":false,"provider":"claude","level":"exigeant","model_policy_hash":"EMPREINTE_RENVOYEE_PAR_SHOW","inherit":false}
```

Pour le vérificateur : `scope:""`, `reviewer:true`. Pour revenir au modèle de
planification d’un responsable : son `scope`, `reviewer:false`, `inherit:true`.
La révision doit être celle observée après la pause. Un formulaire périmé est refusé.

Une session de planification détenue, une revue en cours ou une reprise de revue
par lots non terminée interdit le changement. Aucun rôle ni vérificateur absent
n’est créé par cette action. Quotas, consommations, résultats acceptés, traces
et avis précédents sont conservés ; une nouvelle sélection ne constitue pas
une reprise ni une validation.
