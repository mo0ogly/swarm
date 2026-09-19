# Swarm — rôles, responsabilités et limites réelles

État vérifié le 18 septembre 2026. Ce document corrige les présentations trop
larges de la livraison v19. Il distingue le code disponible, les missions
réellement configurées et les obligations qui restent à implémenter.

> Évolution après cet audit : voir le [contrôle obligatoire de l’organisation](GARDE-ORGANISATION.md). Le constat ci-dessous décrit l’état antérieur à cette correction.

## 1. Verdict de l’audit

L’organisation agentique demandée n’est pas déployée dans les missions du serveur
local. La présence du code de planification ne signifie pas son activation.
Lecture SQLite de `.swarm/state.db` : 14 travaux, aucun objet `planning` actif
ou configuré dans ces travaux. Cela comprend « APEX — Audit Swarm : architecture
Cursor et autonomie réelle » et « Swarm — autonomie et coordination du lancement
au résultat ». Les agents persistés comprennent 72 `worker` et un `planner`.
Ce dernier compte historique ne démontre pas une organisation hiérarchique.

Les missions de recette hiérarchiques ont utilisé des racines temporaires.
Elles prouvent une capacité sur leurs scénarios, pas la conversion des missions
utilisateur. Installer le binaire n’a pas effectué cette conversion.

**Conclusion : capacité hiérarchique partielle disponible et testée ; organisation
hiérarchique absente des 14 missions observées ; vérificateur IA dédié absent.**

## 2. Vocabulaire et pouvoirs réels

| Élément | Nature | Responsabilité actuelle | Limite |
|---|---|---|---|
| Planificateur racine | Appel IA structuré associé au périmètre `root` | Décider des tâches, délégations, reprises et clôtures | Seulement si `Work.Planning` existe ; pas une carte worker |
| Responsable de branche | Appel IA structuré associé à un périmètre enfant | Décider dans les exigences déléguées | Créé par une décision `delegate`, pas systématiquement |
| Exécutant | Processus agent `worker` | Réaliser la tâche et remettre son résultat | Sa déclaration de réussite ne vaut pas acceptation |
| Conducteur | Moteur local | Organiser les départs autorisés et faire progresser la mission | Ce n’est pas un coordinateur IA qui comprend le besoin |
| Contrôleur d’intégration | Code du moteur et commandes de contrôle | Fusionner en copie temporaire, contrôler et publier le résultat vérifié | Ne produit pas une expertise qualitative indépendante |
| Vérificateur IA dédié | Non implémenté comme rôle de lancement | Aucun contrat exécutable actuellement | `reviewer` est refusé au lancement |
| Relecteur humain | Utilisateur | Examiner et décider selon les parcours de validation | Intervention humaine, pas autonomie démontrée |

Un rôle de lancement `planner` ou `subplanner` n’établit pas à lui seul le lien
avec le protocole de planification durable. Celui-ci utilise des périmètres,
des décisions, des baux et des retours propres à `Work.Planning`.
Une tâche intitulée « revue » et exécutée par un worker reste un worker.

## 3. Ce que le code impose effectivement

### Activation

`planning.go`, action `enable`, initialise notamment le périmètre `root`.
Le parcours livré demande une activation explicite sur une mission vide.
Les travaux historiques ne reçoivent aucun responsable racine automatiquement.
Ne pas transformer leur historique en inventant des décisions de planification.

### Planification et délégation

`planning_runner.go` demande une réponse structurée et applique les opérations
`task`, `delegate`, `retry`, `close`. Le planificateur ne code pas dans cet appel.
`planning.go` transfère les exigences déléguées au périmètre enfant. La profondeur
est bornée à trois niveaux et le nombre de périmètres à vingt.
La création de tâche fixe explicitement `task.PlanRole = "worker"`.
Il est donc normal que le graphe des tâches de ce protocole contienne des workers ;
ce graphe seul est insuffisant pour représenter l’organisation complète.

### Exécution

`agents_store.go` accepte seulement `worker`, `planner`, `subplanner` et donne
`worker` par défaut en l’absence de rôle. Il n’accepte pas `reviewer`.
L’icône correspondante ajoutée au navigateur était une possibilité d’affichage,
pas une fonction de lancement. Elle ne doit pas être présentée comme livrée.

### Validation et intégration

`planning_validation.go` reprend les contrôles autorisés par l’opérateur pour
les exigences. Les suggestions de l’IA ne lui donnent pas le pouvoir de choisir
elle-même les preuves qui autorisent son résultat.
`managed_integration.go` traite le résultat de la tentative exacte, intègre en
copie temporaire et vérifie aussi les résultats précédemment acceptés.
Ces contrôles sont utiles mais ne prouvent que les propriétés qu’ils couvrent.
Un contrôle superficiel peut réussir sans démontrer la satisfaction du besoin.

### Clôture : une garantie déjà présente à ne pas nier

L’opération `close` refuse les retours non traités, les enfants non clos, les
tâches sans périmètre, les résultats descendants non acceptés ou périmés et
les exigences du périmètre sans preuve. Une fin de processus ne valide pas une
tâche. Ces conditions existent dans le mode hiérarchique ; il serait faux
d’affirmer qu’aucun contrôle de clôture n’a été implémenté.
En revanche, elles n’exigent pas une revue indépendante par un agent spécialisé.

## 4. Cursor : référence exacte et décisions propres à Swarm

Référence : [Towards self-driving codebases — The final system design](https://cursor.com/blog/self-driving-codebases#the-final-system-design), consultée le 18 septembre 2026.

Le modèle final décrit un planificateur racine, des sous-planificateurs récursifs
et des exécutants dans leurs copies du dépôt. Les retours remontent au responsable
qui peut reprendre ses décisions. Le juge indépendant apparaît dans une étape
antérieure puis est retiré ; l’intégrateur central est également retiré plus loin.
Ce texte est un retour d’expérience, pas une norme de conformité.

Swarm en reprend certains mécanismes mais ajoute une intégration contrôlée et
des barrières de validation. Imposer une revue indépendante pour certains
livrables serait une politique propre à Swarm, pas une exigence littérale du
modèle final de Cursor. La comparaison doit porter sur des comportements prouvés,
pas sur le nombre de rôles affichés ni sur une revendication de conformité.

## 5. Organisation cible — NON ENCORE LIVRÉE de bout en bout

### Contrat avant lancement autonome

Le moteur doit vérifier un responsable racine identifié, la propriété des
exigences, la responsabilité de chaque branche, les moyens d’exécution et une
politique de validation explicite pour chaque résultat. Un prévol incomplet
ne doit pas être présenté comme une mission autonome prête.
Le diagnostic doit expliquer le manque et proposer une réparation concrète.
Il ne faut pas imposer une branche artificielle à une mission simple : un
responsable racine et un exécutant peuvent suffire selon le besoin.

### Politique de vérification

Distinguer trois moyens : contrôles objectifs, revue indépendante et décision
humaine. Leur combinaison doit être définie à partir des critères de réussite.
Pour une revue indépendante requise, enregistrer au minimum : auteur de la
tentative, réviseur distinct, version exacte des artefacts examinés, critères,
constats, verdict et date. Un changement de contenu invalide cette revue.
Le réviseur ne doit pas approuver sa propre production ni modifier silencieusement
le résultat qu’il approuve. Une correction doit repasser par la vérification.
Une simple tâche de revue exécutée par un worker ne garantit pas ces propriétés.

### Reprises et clôture

Conserver les contrôles de clôture existants. Ajouter les obligations de revue
là où la politique les impose. Un refus renvoie au responsable avec les constats
utiles ; celui-ci peut préparer une correction bornée. L’absence de réponse ne
constitue jamais une approbation. Une indisponibilité de fournisseur doit être
expliquée sans déclencher une boucle de relance ou supprimer une exigence.

## 6. Représentation web attendue

Deux relations distinctes : « est responsable de » pour l’organisation et
« doit être terminé avant » pour les dépendances. Une flèche ne doit pas confondre
ces sens. Montrer le responsable racine même entre deux appels IA : un rôle
persistant n’est pas un processus constamment actif.

Chaque élément affiche sa nature (agent IA, moteur, contrôle ou humain), son
rôle, son état, sa dernière décision et sa prochaine action. Une carte worker
sans vérificateur ne doit pas recevoir une coche de revue indépendante.
Le panneau de vérification indique la politique choisie, les contrôles exécutés,
les preuves, les critères non couverts et l’identité du relecteur si applicable.
Les manques doivent se résoudre depuis une action visible ; pas depuis un
simple badge ou une consigne demandant à l’utilisateur de chercher ailleurs.

## 7. CLI et API

Commandes existantes : `swarm planning show TRAVAIL`, `swarm planning history
TRAVAIL`, `swarm mission status TRAVAIL`, `swarm help planification` et
`swarm help pilotage`. Elles ne créent pas un vérificateur indépendant.

À implémenter : un diagnostic d’organisation partagé avec le web, un aperçu
de réparation lié à la révision et une application transactionnelle. Les futurs
champs doivent distinguer acteur, responsabilité, politique de validation,
preuve et motif de refus. Une sortie JSON doit fournir les mêmes verdicts que
l’interface. La couleur ne doit jamais porter seule l’information dans le terminal.
Aucune nouvelle syntaxe CLI n’est annoncée comme disponible par ce document.

## 8. Reprise des missions historiques

Inventorier leur état et conserver leurs preuves. Proposer explicitement soit
leur maintien dans le parcours historique, soit une migration vérifiée, soit
une nouvelle mission liée à la précédente. Prévisualiser les exigences reprises,
les tâches sans propriétaire, les preuves réutilisables et les revues manquantes.
Ne pas interrompre ni réaffecter silencieusement une tentative active. Ne pas
attribuer rétroactivement au planificateur des décisions humaines anciennes.

## 9. Recette nécessaire avant d’annoncer la correction complète

| Scénario | Résultat exigé |
|---|---|
| Mission autonome sans responsable | Refus explicite et réparation proposée |
| Mission simple sans sous-planificateur | Acceptable si responsabilité et validation complètes |
| Délégation | Exigences possédées, responsable et retour visibles |
| Revue indépendante obligatoire mais absente | Départ ou acceptation bloqué selon le contrat explicite |
| Auteur choisi comme réviseur | Refus de l’auto-approbation |
| Artefact modifié après revue | Verdict périmé, nouvelle revue nécessaire |
| Processus terminé avec test réussi mais critère non couvert | Pas de validation globale abusive |
| Redémarrage après verdict | Pas de double acceptation ni perte de responsabilité |
| Modification concurrente web/CLI | Refus de la révision périmée |
| Mission historique | Mode et manques affichés honnêtement ; historique conservé |
| Parcours utilisateur réel | Responsable, exécutant et vérification observables dans la mission du serveur |

Tests existants utiles, mais insuffisants pour cette cible :
`TestPlanningClosureRequiresFreshEvidenceAndReopens`,
`TestPlanningValidationReactivatesAfterResultConsumed`,
`TestPlanningCLIAndAuthenticatedHTTPShareContract` et recettes IA conservées
dans `docs/plans/swarm-architecture-implementation/evidence/`.

## 10. Conditions d’une annonce de livraison honnête

Annoncer séparément : code implémenté, tests isolés réussis, binaire installé,
mission réellement configurée et scénario utilisateur exécuté. La correction
sera complète quand ces cinq niveaux auront été démontrés sur le parcours visé.
La présente mise à jour est un audit documentaire ; elle n’installe pas les
contrats manquants et ne modifie aucune mission.
