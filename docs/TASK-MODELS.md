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
vérificateur : ces rôles ont un contrat distinct. L’édition individuelle de leurs
modèles et les propositions automatiques du planificateur ne sont pas livrées
par cet écran. Le choix d’un modèle exact ou de son effort se règle dans
**IA et connexions → Configurer les niveaux** ; la tâche choisit ensuite un niveau.
