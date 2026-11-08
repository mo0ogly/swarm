# Reference guide — Pour choisir le type de modèle et vérifier les capacités réelles

Les chemins et commandes conservent leur contexte de travail d’origine.

## Déterminer ce qui est demandé

Distinguer explicitement :

- **Modèle de cadrage** : questions, contexte, livrables, risques, preuves attendues,
  méthode proposée et brouillon du besoin.
- **Modèle de travail** : groupes d’actions, dépendances, décisions, contrôles,
  validations humaines ou revues, conditions de reprise et données à paramétrer.
- **Exécution** : instance issue du modèle, plan adapté et adopté, tâches réelles,
  autorisations et preuves observées par le moteur.

Un questionnaire ou une liste de rôles ne suffit pas à livrer un modèle de travail.
Ne pas annoncer que les flèches de décision ou validations sont déjà définies
sans les trouver dans un artefact réellement consommé et contrôlé par le moteur.
Une demande de conception seule produit une proposition, pas un lancement.

## Examiner les capacités et le contexte

Localiser le dépôt Swarm demandé ; lire son `AGENTS.md` et
`tools/agent-workflows/CONTRACT.md`. Observer la branche, les modifications
préexistantes et les processus actifs avant de choisir les fichiers à toucher.

Lire selon le périmètre :

- `docs/PRODUCT-ARCHITECTURE.md` : deux UX séparables, développement et processus
  métier ; Wattson est une application distincte.
- `docs/PRODUCT-WORKFLOW.md` : cinq étapes communes au produit, puis six étapes
  pour **chaque** fonctionnalité. Ce parcours développement ne doit pas être
  imposé à un processus métier comme la facturation.
- `docs/PREPARATION-TEMPLATES.md`, `tools/agent-workflows/templates/preparations.json`,
  `internal/engine/preparation_templates.go` : catalogue, questions et projection
  publique existants. Rechercher leurs tests avant d’en modifier les contrats.
- Pour un graphe exécutable : documentation et source actuelles des plans, tâches,
  politiques de validation, décisions et opérations CLI/API concernées.
- Pour l’interface : `docs/UI-DESIGN.md`, feuilles de jetons et traduction FR/EN.

Vérifier les champs et opérations acceptés dans cette version. Une dépendance de
prérequis n’est pas automatiquement une branche conditionnelle. Pour chaque
orientation de décision, déterminer le mécanisme moteur qui l’applique. Si la
capacité manque, nommer l’écart et proposer une extension séparée ; ne pas inventer
un champ JSON ou faire passer une flèche dessinée pour une garde exécutable.

