---
name: swarm-model-design
description: Concevoir ou améliorer des modèles Swarm de cadrage ou de travail, avec graphe, décisions, validations et parcours FR/EN. À utiliser pour un catalogue ou un modèle réutilisable Swarm, pas pour lancer une campagne de tests de facturation.
---

# Concevoir un modèle Swarm

Transformer un sujet en une structure réutilisable que l’utilisateur comprend,
adapte et adopte avant exécution. Réutiliser la méthode et les contrôles réels de
Swarm. Une présentation de la méthode ne constitue pas un suivi d’exécution.

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

## Définir le modèle

Partir du résultat utilisateur et de ce qui doit être préservé. Choisir un sujet
et une intervention utiles (créer, faire évoluer, corriger ou migrer), sans
reproduire une quantité arbitraire de modèles ou un catalogue concurrent.

Pour un modèle de travail, préciser au minimum :

| Élément | Ce qu’il faut rendre observable |
| --- | --- |
| Entrées | Données exigées, paramètres, inconnues, préconditions et périmètre |
| Actions | Effet attendu, responsable, sorties et critères identifiables |
| Groupes | Regroupements lisibles ; conserver les IDs des actions partagées |
| Flèches | Prérequis réels et orientations de décision, avec leurs conditions |
| Validations | Contrôles déterministes, revue, décision humaine, effet d’un refus |
| Reprise | Erreur, arrêt, nouvelle précondition, idempotence si effets répétés |
| Fin | Critères du modèle remplis et effets externes confirmés si applicables |

Utiliser le format public existant, avec des identifiants stables. Garder les
paramètres opérationnels configurables dans les mécanismes du produit ; les
valeurs par défaut et les limites doivent rester visibles et explicables.

Pour le développement : réutiliser les cinq fondations, instancier le cycle de
six étapes par fonctionnalité, partager les tâches communes sans duplication.
Justifier les étapes non applicables. Les noms de groupes ou phases n’autorisent
pas à eux seuls une transition ou un départ d’agent.

Pour un processus métier : séparer définition versionnée, dossiers/exécutions et
supervision. Identifier les habilitations et effets externes avant de proposer
une exécution ; une fixture de facture ne constitue pas un paiement bancaire réel.

## Raccorder la préparation et l’interface

Si l’implémentation est demandée, conserver une projection cohérente CLI/API/UX.
Ne pas dupliquer dans le navigateur une garde qui doit appartenir au moteur.
Préserver les réponses, les choix d’intervention et les brouillons déjà saisis.
La sélection d’un modèle ne vaut ni adoption, ni autorisation de budget, ni départ.

Présenter résultat, exemple, livrables et preuves avant les détails. Garder les
explications longues fermées par défaut : bouton textuel et dialogue accessible
ou accordéon natif adapté au besoin. Un aperçu peut expliquer les étapes et leurs
résultats ; il ne doit afficher aucun état d’avancement fictif.

Composer une identité propre à Swarm avec ses jetons et son logo, sans recopier la
mise en page d’un site de référence. Traduire les nouveaux textes via le catalogue
existant ; préserver ses traductions partagées et les textes utilisateur. Contrôler
les deux thèmes, dont les surfaces, les champs et la sélection en thème clair.

## Vérifier et livrer

Choisir une racine isolée et un binaire construit depuis le candidat. Ne pas
modifier un candidat figé de campagne, une base de mission vivante ou les fichiers
d’un agent actif. Les budgets, tentatives, refus et historiques restent conservés.
Un aperçu isolé permet de présenter le résultat sans redémarrer le cockpit vivant.

Pour un modèle exécutable, vérifier via les opérations publiques : adaptation et
adoption, références et cycles, refus d’entrées invalides, prérequis non satisfaits,
chemin de refus, reprise après précondition corrigée et validations sur le même
candidat. Contrôler les branches en observant les actions réellement autorisées
ou refusées, pas seulement le dessin. Les fixtures et agents scriptés restent
identifiés ; ne pas prétendre avoir testé un LLM réel.

Pour l’UX, vérifier FR/EN, deux thèmes, mobile, Tab/Maj+Tab, Échap et retour du
focus, chargement, erreur/reprise et conservation des réponses. Collecter les
erreurs navigateur et les requêtes dès le départ. Distinguer les visites de la
fixture d’initialisation des mutations du parcours testé. Inspecter les captures,
scanner les jetons/couleurs et exécuter les contrôles frontend concernés.

Exécuter les tests moteur proportionnés si le moteur change, puis les contrôles
requis par le dépôt. Une modification d’explication frontend ne justifie pas de
rejouer une campagne globale de facturation. `git diff --check` et inspection du
diff restent nécessaires. Rafraîchir un manifeste courant uniquement selon la
procédure du dépôt ; cela ne renouvelle pas une acceptation historique.

Livrer le modèle et son mode réel (cadrage, travail exécutable ou proposition),
les critères vérifiés, le candidat, les résultats et les limites. Une auto-revue
n’est pas une revue indépendante. Commit, push et publication suivent les
instructions de l’utilisateur ; la skill n’accorde aucune autorisation supplémentaire.
