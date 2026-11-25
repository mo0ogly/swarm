# Contrôle obligatoire de l’organisation autonome

## Comportement

Avant `mission start`, un passage au mode autonome et chaque réservation
automatique, le moteur exige une organisation valide : responsable racine,
propriété cohérente des exigences, fournisseur de planification renseigné,
contrôles prévus par exigence, propriétaire de chaque tâche et politique de
validation explicitement autorisée. Les politiques humaines explicites restent
possibles ; elles demandent une intervention et ne sont pas une revue IA.

Un refus ne crée ni autorisation ni nouvelle tentative. Une ancienne autorisation
ne contourne pas le contrôle au moment de réserver l’agent. Les agents déjà
lancés ne sont pas interrompus par ce diagnostic. Les données historiques restent
conservées. L’admission manuelle et la préparation effective du processus
exigent la même organisation, y compris lors du démarrage d’une intention
historique encore en file.

Le contrat structurel est revérifié au cycle du conducteur et à la réservation.
Les contrôles existants de fournisseur, budget, espaces, dépendances, tentative,
révision et preuve restent nécessaires ; le diagnostic d’organisation ne les
remplace pas. Une organisation configurée n’est pas une garantie de résultat.

## Web et CLI

`mission status` expose `organization` et `evidence_stage` en JSON, et les mêmes
faits en texte. L’aperçu d’une organisation incomplète annonce zéro départ et ne
fournit pas de jeton de confirmation. Le serveur refuse aussi une demande directe.
Le web affiche « Organisation autonome non configurée », les manques et une aide.
La confirmation est désactivée quand l’aperçu indique cette absence.

L’état de preuve distingue le parcours encore non vérifié de la validation des
résultats et de la clôture du responsable racine dans la mission courante. Il ne
déduit jamais cette preuve d’un test isolé ou de l’installation d’un binaire.
Les détails des contrôles et leurs limites restent consultables. La revue IA
indépendante est décrite dans le [parcours de préparation](../../PREPARATION-UX.md).

## Reprendre une mission historique

Le périmètre racine est identifié par le couple (identifiant du travail,
`root`). Cette identité existe dès la création et reste observable sans
réécrire les documents historiques. La présence du rôle ne configure ni
fournisseur ni vérificateur et ne donne aucun droit de départ.

Utiliser `planning attach` pour rattacher un travail existant via son identifiant
et sa révision. Les sous-orchestrateurs s’inscrivent sous leur parent dans
`planning.scopes`. Les tâches ont leurs propres liens de prérequis. Le
rattachement conserve les tentatives et les résultats, et laisse les départs
soumis à une autorisation explicite. Voir le
[guide](../../GUIDE-UTILISATEUR.md#rattacher-une-équipe-à-un-projet-existant).

## Vérification / Verification

La [recette](../../tools/verification/recipes/default-orchestration.json)
prend le binaire natif candidat via `SWARM_ORGANIZATION_BINARY`. Elle observe
le refus sans réservation, le départ manuel avec un fournisseur inerte après
rattachement, la conservation des prérequis et le rendu du canevas. Le test
navigateur de base charge les sources du checkout. Le parcours cockpit protégé
contrôle séparément les scripts réellement servis et la racine affichée.

The [recipe](../../tools/verification/recipes/default-orchestration.json)
uses the candidate native binary selected by `SWARM_ORGANIZATION_BINARY`.
It checks refusal without reservation, a manual start with an inert provider
after attachment, preservation of prerequisites, and canvas rendering. The
basic browser test loads the checkout sources. The protected cockpit journey
separately checks the actual served scripts and the displayed root.
