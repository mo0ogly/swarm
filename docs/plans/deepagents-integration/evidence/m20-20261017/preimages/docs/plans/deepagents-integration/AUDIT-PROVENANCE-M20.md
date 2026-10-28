# M20 — producteurs, provenance et consommateurs du cadrage

16 octobre 2026. Codex seul, inspection du code du checkout direct. Inventaire
de contrat, distinct d'une recette de permissions ; les références ci-dessous
sont des chemins réellement lus. Le [plan M20](INCREMENT-MEMOIRE-16.md) reste
ouvert pour l'identité, l'attribution métier et les droits. La correction
[M21](RESULTATS-MEMOIRE-17.md) ne couvre que la liaison de contenu hors moteur.

## Ce que l'on peut effectivement attribuer aujourd'hui

| Élément | Producteur réel | Fait conservé et limite |
| --- | --- | --- |
| Question | `agents_store.go`, `prepareLaunch` | `Task.Question` reçoit `Launch.Instruction` lors de la création de la tâche Brainstorm. Le contenu ne porte pas une identité humaine authentifiée. |
| Réponse | `agents_store.go`, `settleAgentTask` | Préfère `Agent.Reply`, sinon relit le rapport de cadrage borné ; renseigne `Task.Response` et ajoute `BrainstormAnswer`. Une réponse conservée ne prouve ni succès actuel ni acceptation. |
| Référence de réponse | `model.go`, `BrainstormAnswer` ; producteur ci-dessus | `attempt` contient **Agent.ID**, pas `Agent.Attempt`. `At` date la conservation. Sans ligne d'agent cohérente et autorité attestée, ce lien ne devient pas une identité authentifiée. |
| Ancienne réponse | `dialogue_context.go`, `dialogueHits` | Sans `Answers`, utilise `Response` avec `attempt` vide. Ne pas inventer un auteur ni éliminer cette histoire par migration silencieuse. |
| Propriétaire courant de tâche | `agents_store.go`, `prepareLaunch` | `Task.Owner` est mis au fournisseur au lancement. Ce champ mutable ne représente pas la propriété stable du cadrage. |
| Brief adopté | `brainstorm.go`, `adoptBrief` | Gèle tâche, texte, empreinte et date après comparaison du texte relu. `PlanningBrief` ne porte ni auteur humain, ni propriétaire métier, ni référence exacte `DialogueRef` de la version adoptée. |
| Référence de contenu | `dialogue_context.go`, `DialogueRef` / `resolveDialogue` | Résout tâche, clé de réponse et SHA-256 de question + saut de ligne + réponse dans le même `Work`. Cette résolution prouve une concordance, pas un droit de lecture ou de partage. |
| Paquet de résumé | `memory_summary_source.go`, `memorySummarySourceForWork` | Lie un instantané opérateur, la révision et les échanges sélectionnés. `project_sha256` est le hash du chemin racine : liaison locale, pas identité globale du projet ni racine cryptographique de confiance. |

La provenance technique disponible doit rester distincte de la propriété, de
l'auteur attesté et des permissions. La session identité a confirmé dans IA1
que les contrôles Trust ne gouvernent pas encore l'admission générale des agents
et que le processus fournisseur scripté peut accéder au contrôle sous le même
compte système. Ne pas traiter une ligne d'agent actuelle comme preuve complète
d'authenticité. Aucun nouveau nom d'utilisateur, principal ou propriétaire n'a
été ajouté par cet audit.

## Où les textes sont consommés

| Voie | Source inspectée | Contrôle existant | Raccord d'usage à prévoir |
| --- | --- | --- | --- |
| Lecture opérateur, recherche | `console_cli.go`, affichage d'agent ; `web_server.go`, `dialogue-search` ; `dialogue_context.go`, `searchDialogue` ; `web/brainstorm.js` | Travail courant, recherche bornée, session web/origine/CSRF selon parcours. | Distinguer une inspection administrateur autorisée d'une lecture par un agent ; la session partagée n'identifie pas une personne. |
| Historique automatique de cadrage/plan | `brainstorm.go`, `brainstormHistory` ; `agents_store.go`, assemblage du prompt | Quatre derniers échanges hors échange courant. | Contrôler l'usage de chaque question/réponse ; une absence de sélection explicite n'autorise pas leur diffusion automatique. |
| Brief commun hors hiérarchie | `agents_store.go`, assemblage du prompt | Copie `PlanningBrief.Text` quand `Planning == nil`. | Droits sur la version adoptée et sur la tâche destinataire, indépendants de l'auteur. |
| Brief pour worker racine | `worker_context.go`, `workerExecutionContext` | En hiérarchie, le brief racine reste réservé au périmètre racine ; les enfants gardent leur contrat propre. | Ne pas contourner cette séparation avec une référence ou un résumé ; vérifier aussi le droit d'usage dans le périmètre racine. |
| Citations explicites intégrales | `dialogue_context.go`, `dialogueAttachments` ; `agents_store.go` | Références résolues dans le travail, dédoublonnage et capacité des pièces jointes. | Autorisation de transmission à la tâche/instance destinataire ; hash correct insuffisant. |
| Assistant de page | `assist_context.go`, `brainstormContext` | Échanges sélectionnés, sinon six derniers ; question/réponse et brief marqués comme texte non fiable. | C'est une transmission à une IA, même si l'écran est consulté par un opérateur ; contrôler destination et usage. |
| Export opérateur pour résumé | `memory_summary_source.go`, `memorySummarySource` | Même lecture de ligne/révision, sélection explicite, champs et sources vérifiés. | Le contrat `local_operator` n'est pas un contexte délégué autorisé ; vérifier l'autorité de l'exporteur et les usages des sources. |
| Préparation locale de résumé/choix | `tools/memory-evaluation/swarm_source.py`, `admission.py` | Liaison des octets, des citations, des annotations et du reçu ; M21 refuse alias et répétition. | Outils hors moteur : aucun droit créé, aucune identité authentifiée par une empreinte. |
| Résumé transmis au worker | `memory_summary_selection.go`, `memorySummaryAttachment` et contrôle avant départ | M19 reconstruit les textes et exclut l'instantané opérateur ; sélection explicite, confirmation et fraîcheur avant processus. | Contrôler le droit sur les sources lors de l'aperçu, de la réservation et du départ, avec révocation et effet observé. |
| Plan structuré depuis brief/cadrage | `plan_store.go`, `planSource`, `readPlan`, `commitPlan` | Lien au brief courant, réponse finale et empreinte, décisions conservées, validation du plan. | Le plan proposé ne crée ni droits de création d'agent ni permission implicite d'utiliser les textes sources. |
| RETEX et export de leçon | `retex.go`, `retexAction`, `retexMarkdown`, `retexGuide` | Source dialogue ou recette résolue ; contrôles de qualification et de preuve pour les étapes concernées. | Distinguer citation d'origine, reformulation, publication et droits de réutilisation. |
| Mémoire de projet réutilisée | `project_memory.go`, `verifiedMemorySource`, `validateMemoryVersion` | RETEX vérifié et source courante, version/hash/projet, retrait et import contrôlés. | Un RETEX vérifié n'établit pas à lui seul l'autorité de diffuser l'échange d'origine. |
| Archives et contextes reçus | `archive.go`, `export` / `importBundle` ; `dialogue_context.go`, `SavedContext` | État/historique exportés ensemble ; projet mémoire importé et sélections marqués non admis localement. | Garder les textes et décisions historiques sans activer de nouveaux droits. Définir l'origine importée du cadrage/brief dans le futur contrat, sans réécriture des auteurs passés. |

Cet inventaire vient de lectures de code. Le parcours M21 observe réellement
export CLI/API, préparation et réception de citations par un worker scripté ; il
ne teste pas l'autorisation des treize voies. L'audit ne déclare donc pas
IDA-13 conforme, ni M20.3/4 réalisés.

## Raccordement à IA2/IA3 et critères de reprise

Le chantier mémoire doit recevoir un contrat d'autorité utilisable pour :

1. identifier le demandeur et l'instance destinataire depuis une preuve courante,
   sans accepter ces qualités sur simple déclaration du texte ou du fournisseur ;
2. désigner le propriétaire logique du cadrage et les versions de contenu, tout
   en gardant séparés auteur original et responsable actuel ;
3. décider lecture, résumé et transmission vers un destinataire précis, y compris
   les voies automatiques et l'assistant ; ne pas faire d'un droit de lecture un
   droit général de rediffusion ;
4. revalider cette décision lors de l'application, avec révocation, changements de
   périmètre et reprise après panne ; enregistrer refus, décision et effet réel ;
5. traiter anciens échanges, réattribution explicite, import et contexte déjà reçu
   sans inventer une identité ou effacer l'historique.

Ce sont des besoins d'interface, pas des champs JSON, endpoints ou rôles nouveaux
déclarés disponibles. Propriétaire du contrat d'identité : conversation dédiée,
IA2/IA3 ; propriétaire des consommateurs et des tests d'usage : chantier mémoire,
M20. La reprise des contrôles effectifs exige ce contrat puis sa preuve publique
de refus/application. La documentation seule ne satisfait pas cette dépendance.

Améliorations retenues : valider les fixtures contre le producteur réel (M21,
vérifié) ; ne pas confondre hash de racine et identité de projet (IA2/M20,
contrat à définir) ; couvrir les copies indirectes et les archives (M20.3/4,
à réaliser après IA2/IA3). Leur valeur est d'éviter qu'une bonne garde du résumé
laisse subsister un autre chemin de diffusion non contrôlé.
