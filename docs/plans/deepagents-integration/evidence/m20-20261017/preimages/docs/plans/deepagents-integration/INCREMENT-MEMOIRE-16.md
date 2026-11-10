# M20 — attribuer le cadrage avant sa transmission hiérarchique

16 octobre 2026. Reprise « la suite ? », puis précision utilisateur :
« les échanges de cadrage devraient avoir un propriétaire ? ».
Codex seul, méthode `replan` ; aucun appel de modèle ou lancement natif.

## Observation et correction du cadrage

M19 reste vérifié dans son périmètre. L'extension immédiate aux workers
hiérarchiques annoncée à la reprise est prématurée : le propriétaire métier du
cadrage n'est pas représenté distinctement dans le contrat actuel.

Sources inspectées :

- `model.go`, `BrainstormAnswer` : clé de réponse, texte et date.
- `agents_store.go`, `settleAgentTask` : la clé `attempt` d'une réponse reçoit
  `Agent.ID`. La ligne d'agent conserve sa mission, sa tâche et sa tentative.
- `agents_store.go`, `prepareLaunch` : `Task.Owner` reçoit le fournisseur lors
  du passage à `running`. Ce champ ne constitue pas une propriété stable du cadrage.
- `dialogue_context.go`, `DialogueRef` / `dialogueHits` : identité task/attempt/hash,
  résolution dans un travail ; aucune autorisation de partage déduite de ces clés.
- `worker_context.go` : le contexte délégué exclut le brief racine et les tâches
  voisines. Une référence technique ne doit pas contourner cette frontière.
- `planning.go`, `applyPlanning(enable)` : activation sur mission neuve sans
  tâches ; `prepareLaunch` refuse le rôle planner d'un brainstorming dans une
  mission hiérarchique. Ne pas fabriquer un historique de cadrage hiérarchique
  par mutation privée pour rendre une recette possible.

Le constat « sans propriétaire de périmètre » est précisé : attribution technique
partielle existante, responsabilité et autorité de diffusion non représentées.
La question de l'utilisateur corrige une hypothèse du lot, sans annuler le besoin
mémoire ni autoriser une nouvelle identité humaine ou un partage automatique.

## Proposition de contrat à instruire

Distinguer trois dimensions :

1. Responsable logique du cadrage : mission pour le cadrage autonome ; périmètre
   racine/enfant lorsqu'un producteur hiérarchique autorisé existe. La responsabilité
   reste stable à travers le remplacement d'un agent ou d'un titulaire de bail.
2. Origine de chaque intervention : acteur effectivement attesté par le parcours,
   agent, tentative, date et événement. Ne pas inventer un utilisateur identifié
   lorsque le parcours n'atteste que « opérateur local ».
3. Usage autorisé : consultation, sélection/résumé et transmission explicite à une
   tâche ou un périmètre ; vérifier l'autorité courante indépendamment de l'auteur.
   Une propriété ne crée pas à elle seule tous ces droits.

Comparaison : réutiliser `Task.Owner` mélangerait fournisseur mutable et
responsabilité ; attribuer seulement un agent rendrait la propriété périmée à
chaque reprise. Une attribution métier distincte, versionnée et reliée aux
événements est la piste retenue pour le prochain contrat, avant implémentation.

Les anciens échanges restent des originaux historiques. Une origine retrouvée
par référence vérifiable n'établit pas une propriété métier. L'état inconnu doit
rester explicite ; une attribution ultérieure nécessite une décision tracée,
sans réécrire auteur, texte ou date des interventions passées.

## Ordre révisé et critères

| Lot | Propriétaire / dépendance | Critère observable |
| --- | --- | --- |
| M20.1 | Codex ; M19 | Inventaire de code consolidé dans `AUDIT-PROVENANCE-M20.md` : producteurs, treize voies de consommation, import/export et limites d'attribution. Recette de droits non réalisée ; dépend du contrat IA2/IA3. |
| M20.2 | Codex ; M20.1 | Contrat testable de création, attribution/réattribution, portée et historique ; compatibilité/migration décidée depuis le stockage réel. Non réalisé. |
| M20.3 | Codex ; M20.2 | Opérations publiques communes, refus sans mutation et visibilité CLI/API/UI pertinente, tests de non-régression avant correction. Non réalisé. |
| M20.4 | Codex ; M20.3 | Création → attribution → autorisation de transmission → réception vérifiée, docs FR/EN et RETEX. Non réalisé. |
| Extension hiérarchique | Codex ; M20.4 et producteur autorisé | Définir le transfert entre cadrage et tâche destinataire, sans inventer de propriétaire ni élargir le contexte. Reste fermée. |

L'audit initial et cette révision de plan ne constituent ni une attribution
effective, ni une migration, ni une nouvelle permission. Aucun code applicatif
n'a été modifié pour M20 à ce stade. M19 et les preuves antérieures restent intacts.

## Extension explicite : identité et création des agents

L'utilisateur demande ensuite l'application effective du propriétaire, de
l'auteur et du droit d'usage, puis la protection contre l'usurpation d'agent et
la création d'un enfant non légitime par un agent légitime. Il confirme que les
scénarios complémentaires appartiennent à la gestion des identités de l'application.

Ces exigences sont rattachées à R01/R13 dans le
[plan Identité des agents](../actions-confiance/identites-agents-replan.md), avec
18 scénarios et les lots IA1–IA5. La création hors plan doit rester bloquée,
expliquée et soumise à une décision humaine authentifiée sur le contenu exact.
Un nom, un fournisseur ou `event_id` ne suffit pas à attester une identité.

Ordre révisé : IA1/IA2 établissent les contre-exemples et l'autorité ; IA3 fournit
le contrôle commun de création et de décision humaine ; IA4 reprend M20.2/3 pour
appliquer propriété et usages dans tous les consommateurs ; IA5 et M20.4 vérifient
la chaîne complète et le RETEX. Les échanges existants ne reçoivent aucun droit
implicite et l'extension hiérarchique reste fermée. L'audit initial est réutilisé,
mais les 18 nouveaux scénarios ne sont pas déclarés testés ni implémentés.

## Reprise après ouverture de la session identité

La conversation dédiée a livré l'audit IA1 et sept contre-exemples, sans
modifier les sources applicatives. Son rapport a été consulté en lecture seule ;
le chantier mémoire conserve ses sources et son suivi. L'état IA2/IA3 et les
droits effectifs restent ouverts. Aucun message inter-chat n'a été envoyé.

Le travail indépendant [M21](INCREMENT-MEMOIRE-17.md) vérifie la liaison de
contenu des citations dans les outils de préparation ; il ne remplace ni
l'attribution métier ni le contrôle d'usage de M20. Les producteurs et
consommateurs à raccorder sont consolidés dans
[l'inventaire M20](AUDIT-PROVENANCE-M20.md), sans inventer de nouvelle identité
ou de permission.
