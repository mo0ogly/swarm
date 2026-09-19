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
conservées et les lancements manuels conservent leurs contrôles habituels.

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

Il n’existe pas encore de migration automatique de son organisation. Conserver
le travail et ses preuves. Créer une mission vide avec le besoin restant, puis
activer « Confier ce besoin à une équipe autonome » avec un fournisseur et des
contrôles adaptés. La reprise de résultats anciens exige leur vérification,
pas une acceptation rétroactive. Le mode manuel reste disponible.
