# Reprendre un lancement et contrôler une livraison incomplète

[English](en/ENGINE-RECOVERY.md) · [Guide utilisateur](../GUIDE-UTILISATEUR.md)

## Ce que le moteur distingue

Une copie de travail préparée n’est pas encore un agent enregistré. Un processus
terminé n’est pas un résultat accepté. Une déclaration de couverture ne remplace
ni les contrôles exécutés ni l’avis indépendant.

### Lancement interrompu après la création de sa copie

Pour les nouveaux lancements dans un dépôt géré, le moteur conserve une opération
identifiée : tâche, copie, base Git, fournisseur, consigne et paramètres de lancement.
Si l’enregistrement de l’agent échoue, la transaction ne consomme pas de tentative.
La copie reste attribuée à l’opération initiale, même après redémarrage du serveur.
Le moteur retrouve aussi le fichier d’attribution lorsque l’interruption précède
son enregistrement SQLite.

Les erreurs SQLite BUSY/LOCKED peuvent entraîner deux nouvelles tentatives
**d’enregistrement**, avec la même identité (trois essais au total). Elles ne lancent
aucun fournisseur. Si la préparation reste en attente, le conducteur ne répète pas
le lancement à chaque passage : une reprise explicite est présentée.

Dans le **Pilotage des agents**, ouvrir la tâche puis choisir **Reprendre le
lancement préparé**. La modale expose la copie et les réglages conservés, une aide,
et une confirmation. Annuler ou Échap ne modifie pas la tâche. L’action existe
aussi dans le menu interactif du terminal.

Le CLI utilise la même opération et les mêmes gardes :

```sh
swarm --root /chemin/projet agent prepared IDENTIFIANT_TRAVAIL
swarm --root /chemin/projet agent resume-launch IDENTIFIANT_TRAVAIL --input reprise.json
```

```json
{
  "schema_version": 1,
  "prepared_id": "identifiant_retourne_par_agent_prepared",
  "expected_revision": 42
}
```

La révision doit être celle retournée par `agent prepared`. Une confirmation
répétée retrouve l’agent initial. Si son superviseur n’a pas encore pris en charge
la demande, la reprise peut le solliciter : une prise en charge atomique empêche
le démarrage de deux fournisseurs pour cet agent.

Les critères, consignes, budgets de tâche et base Git ne doivent pas avoir changé.
Les dépendances, budgets, organisation, fournisseur et disponibilité sont revérifiés
au départ. Une copie non attribuée, redirigée ou incohérente reste refusée ; elle
n’est jamais écrasée pour « débloquer ». Les anciennes préparations sans paramètres
sauvegardés nécessitent la demande initiale identifiée : le moteur n’invente pas
les réglages manquants.

## Bilan de livraison avant la revue

Un nouvel exécutant automatisé dans un dépôt géré reçoit un modèle de fichier
`docs/IDENTIFIANT_TACHE.delivery.json`, à produire avec son rapport Markdown.
Le fichier doit être inclus dans la révision Git remise. Pour un projet dans un
sous-dossier du dépôt, `docs/` se rapporte au dossier du projet ; les références
`evidence` se rapportent à la racine Git.

```json
{
  "version": 1,
  "task": "tache-exemple",
  "attempt": "identifiant_fourni_par_le_moteur",
  "contract": "empreinte_fournie_par_le_moteur",
  "outcome": "complete",
  "criteria": [
    {
      "index": 1,
      "status": "pass",
      "reason": "Comportement observé et limites de la vérification",
      "controls": ["controle_autorise_pour_ce_critere"],
      "evidence": ["tests/preuve_test.go", "docs/tache-exemple.md"]
    }
  ]
}
```

Un élément est requis pour chaque critère, sans doublon. La tentative et le
contrat doivent correspondre. Les contrôles référencés doivent être préautorisés
pour ce critère, et les fichiers de preuve doivent exister comme fichiers ordinaires
dans la révision examinée. Liens symboliques et chemins sortant du dépôt sont refusés.
Le bilan est limité à 32 Kio, sans troncature.

Si une obligation n’a pas été testée, déclarer `not_tested` ; si elle a échoué,
`fail`. `not_applicable` exige une justification et conserve ici un résultat à
examiner : cette déclaration ne supprime pas un critère. `outcome` vaut `partial`
ou `blocked` tant que toutes les obligations ne sont pas démontrées.

Un bilan absent, invalide, partiel ou mal attribué produit **Résultat à compléter**.
La copie et le rapport sont conservés, le candidat publié reste inchangé, et aucun
appel de revue n’est consommé. Le responsable reçoit un événement d’intégration
refusée avec son motif. Toute correction reste soumise aux tentatives autorisées.

Dans le détail de la tâche, **Rapports et preuves de la tâche** permet d’ouvrir
le rapport conservé, y compris après une revue refusée. Le CLI interactif propose
le même rapport par l’action de lecture. Le lecteur web exige l’attribution au
travail et à la tâche pour les fichiers internes ; il ne donne pas accès aux autres
fichiers du moteur. Un rapport de revue modifié depuis son enregistrement n’est
pas présenté comme la preuve courante.

La synthèse IA reçoit aussi l’état actuel de la revue et le nombre de tentatives
consommées. Ces faits du moteur priment sur les annonces historiques du rapport.
Son schéma impose deux lignes courtes ; une réponse non conforme reste refusée,
sans masquer le rapport ni produire une validation.

Un bilan complet autorise la suite des contrôles ; il n’accepte jamais la tâche.
Le moteur exécute les commandes autorisées, puis transmet au vérificateur le
candidat, les reçus, le rapport, les sources disponibles et le bilan. Le vérificateur
doit respecter les critères exacts : un audit peut prouver qu’un code préexistant
convient ; l’absence de modification n’est pas, à elle seule, un défaut. Une preuve
manquante ou un contrat ambigu doit être distingué d’un défaut démontré.

## Limites et compatibilité

- Le contrôle de structure ne juge pas la suffisance sémantique d’une preuve.
  Un critère qui regroupe dix obligations peut toujours être mal couvert par un
  seul test. La qualité du plan et de la revue reste déterminante.
- La consigne au vérificateur améliore son cadrage ; elle ne garantit pas chaque
  jugement d’un modèle. Les tests déterministes vérifient le protocole, pas cette qualité.
- L’obligation du bilan concerne les nouveaux exécutants automatisés gérés.
  Les tentatives historiques et les modes interactifs ne deviennent pas rétroactivement
  non conformes. Un bilan présent est néanmoins contrôlé.
- Aucun budget n’est remboursé, aucun plafond augmenté, aucune ancienne décision
  de revue changée par ces mécanismes. Une mission au plafond reste bloquée.

## Décisions et vérification

La reprise conserve une identité et exige une confirmation après l’échec persistant.
L’adoption automatique d’une copie par une nouvelle demande a été écartée : elle
mélangerait consignes, attribution et fichiers. Le contrôle utilise un bilan structuré
au lieu de rechercher des mots comme « partiel » dans un rapport libre ; il conserve
la revue indépendante pour l’analyse du fond.

Tests de référence : `managed_preparation_test.go`, `managed_delivery_test.go`,
`result_presentation_test.go`, `tests/managed_recovery_ui.cjs`. La recette navigateur
utilise un fournisseur déterministe réellement lancé, dans une racine temporaire :
elle ne démontre pas une mission autonome réussie avec une IA réelle.

```sh
go test ./...
go vet ./...
go test -race -run '^TestManaged(PreparedLaunch|Delivery|CompleteDelivery)' .
npm test
# Après construction du binaire ; Puppeteer et Chrome doivent être disponibles.
SWARM_RECOVERY_UI_BINARY=/chemin/absolu/swarm \
SWARM_RECOVERY_UI_OUT=/chemin/absolu/recette \
go test -run '^TestManagedRecoveryBrowserRecipe$' -count=1 -v .
```

### Avis favorable enregistré, publication interrompue

Une réparation externe explicitement soumise peut avoir passé la revue après un
ancien refus de taille, puis rencontrer une erreur de stockage lors de la
publication. Rejouer exactement la même demande de réparation permet de reprendre
cette publication : le moteur réutilise l'avis enregistré après revalidation du
candidat, du contrat, du contexte et des reçus. Aucun nouvel appel IA ni producteur
n'est créé. Un avis défavorable, une preuve modifiée ou une autre tentative ne
bénéficie pas de cette reprise. Un avis favorable seul reste distinct d'une tâche
acceptée ; vérifier l'état public après l'opération.
