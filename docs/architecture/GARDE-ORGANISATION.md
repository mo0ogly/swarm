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
Les détails des contrôles et leurs limites restent consultables ; aucun
vérificateur IA indépendant n’est créé ou annoncé par cette correction.

## Reprendre une mission historique

Il n’existe pas encore de migration automatique de son organisation. Conserver
le travail et ses preuves. Créer une mission vide avec le besoin restant, puis
activer « Confier ce besoin à une équipe autonome » avec un fournisseur et des
contrôles adaptés. La reprise de résultats anciens exige leur vérification,
pas une acceptation rétroactive. Le mode manuel reste disponible.

## Recette visible exécutée et vérifiée

Mission : `w-440c1617beb6b56ecc835c7f`.
Ouvrir le lien de session fourni par votre propre serveur Swarm, puis sélectionner la mission de recette..

Objectif borné : un responsable racine délègue une exigence à un sous-responsable,
qui crée une tâche pour une fonction Python renvoyant 42. Le contrôle moteur
compare directement cette valeur. L’acceptation doit précéder la clôture du
sous-responsable puis de la racine. Cette preuve ne vaut que pour ce scénario.
Le verdict final de cette exécution est consigné séparément dans les preuves.

## Recettes de non-régression

Les anciennes recettes d’ordonnancement supposaient qu’un travail sans responsable
pouvait passer en autonome. Leurs fixtures déclarent désormais une organisation
et une politique humaine par défaut, sans appeler une IA. Les recettes de
validation conservent leurs contrôles propres. Ces fixtures ne sont pas des
preuves de délégation IA ; la mission visible fournit cette observation séparée.

Les tests de refus utilisent un travail réellement dépourvu d’organisation :
passage autonome refusé, démarrage refusé sans mutation, aperçu sans départ,
réservation refusée même après une ancienne autorisation et validation manquante.
Une correction supplémentaire conserve les champs d’autorisation après la
normalisation des contrôles hérités, qui les effaçait auparavant.


## Résultat constaté sur le serveur installé

Recette réussie : deux responsables (racine et calcul), une tâche acceptée,
quatre décisions IA et clôture des deux périmètres. Aucun verdict manuel ajouté.
Le bundle livré a été cloné ; la fonction exportée retourne 42 ; la source de
départ retourne toujours 0. Les 14 missions historiques sont inchangées.

Validation : suite Go complète réussie (47,384 s), tests ciblés avec détection
des courses, tests frontend et parcours navigateur dans les deux thèmes.
La recette visible a également été relue dans le DOM et en captures des deux
thèmes sur le serveur installé. Il ne s’agit pas d’une garantie pour toutes
les missions complexes ni d’une revue indépendante par un autre modèle.

Incident du script de collecte : après clôture réussie, son lecteur attendait
un JSON alors que la commande d’export n’émet pas de contenu. L’export existait.
Le clonage, le contrôle et la collecte ont été achevés séparément, sans relancer
la mission ni modifier son résultat ; cette erreur de collecte est conservée.

Les fichiers historiques `visible.json`, `live-ui.json` et `installation.json`
étaient conservés sous `docs/plans/swarm-architecture-implementation/evidence/organization/`
dans le dépôt d’origine. Ils ne sont pas distribués ici et ne constituent donc pas
une preuve consultable dans ce dépôt. Voir la [recette autonome publiée](../validation/standalone-docker-20260919.md)
pour les vérifications reproductibles et leurs limites actuelles.
