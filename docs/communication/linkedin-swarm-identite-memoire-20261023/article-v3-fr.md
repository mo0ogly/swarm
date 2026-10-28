ARTICLE LINKEDIN | Swarm deuxième partie | 23 octobre 2026 | version 3

# Identité et mémoire des agents IA dans Swarm

Moteur déterministe Rust et contrôle humain

Un agent peut se tromper dans une commande. Que se passe-t-il si cette erreur devient une consigne permanente, puis se transmet à l’instance suivante ? C’est l’une des questions qui guident Swarm. Je veux relier ce qu’un agent peut faire à son identité, à une autorité explicite et au contexte qu’il reçoit. Le moteur déterministe Rust fait respecter le cadre et les validations définis par l’humain. Les superviseurs IA conduisent le travail ; leurs décisions restent soumises à ces contrôles. Cet article présente cette architecture, ses limites actuelles et cinq vues détaillées issues de la documentation interne.

Dans mon premier article sur Swarm, j'expliquais ce qui m'avait poussé à le développer : voir qui prend en charge une tâche, ce qu'il attend des autres et ce qu'il leur transmet.

## Une mauvaise consigne peut survivre à son auteur

Imaginons un agent de développement qui lit une instruction trompeuse dans une documentation, une issue ou la réponse d’un outil. Il l’applique, puis la reformule dans son compte rendu comme une procédure utile. Si cette procédure entre dans une mémoire permanente sans examen, une nouvelle instance peut la recevoir comme une pratique acquise. Le redémarrage du processus ne suffit alors plus à repartir d’un contexte sain. Ce scénario illustre un risque ; je ne le présente pas comme un incident constaté dans Swarm.

Une attaque n’est pas nécessaire pour créer une dérive. Une mauvaise commande proposée par l’agent lui-même, conservée comme solution puis réutilisée sans ses réserves, peut produire un effet semblable. J’emploie ici l’idée de contamination du contexte : une sortie non vérifiée devient une instruction de confiance. Cela ne signifie ni modification spontanée des poids du modèle, ni virus qui se réplique tout seul.

Deux publications aident à situer le problème. EchoLeak, identifié par Aim Labs dans Microsoft 365 Copilot en 2025, montre qu’un courriel malveillant pouvait détourner le traitement du contexte vers une fuite de données. La vulnérabilité a été corrigée ; la publication rapporte que Microsoft n’a constaté aucun client affecté [8]. MINJA montre expérimentalement qu’un attaquant peut faire entrer des traces malveillantes dans la mémoire d’agents par de simples interactions, sans écrire directement dans leur base, puis influer sur des requêtes ultérieures [7]. Ce second résultat dépend des agents et conditions étudiés ; ce n’est pas une compromission démontrée de tous les produits.

La question devient donc précise : qui peut faire entrer une instruction en mémoire, qui peut la transmettre, et quelle autorité lui donne-t-on quand elle revient ? Une identité authentifiée ne garantit pas qu’un contenu soit juste. L’OWASP distingue notamment l’abus d’identité et de privilèges, ASI03, de l’empoisonnement de la mémoire et du contexte, ASI06 [3].

## Le compte rendu comme point de départ

L'organisation de Swarm doit beaucoup aux travaux publiés par Cursor dans « Towards self-driving codebases » [1]. Leur conception finale confie l'objectif d'ensemble à un planificateur principal, délègue des périmètres à des sous-planificateurs et laisse chaque exécutant travailler sur sa propre copie du dépôt. À la fin, l'exécutant rédige un compte rendu unique : ce qu'il a fait, mais aussi ses doutes, ses écarts et ses constats.

Cursor a fait des choix assumés pour maximiser le débit, comme accepter un petit taux d'erreur stable ou retirer l'agent intégrateur qui devenait un goulot d'étranglement [1]. Pour leur objectif, c'est cohérent.

Le mien est un peu différent : je veux pouvoir expliquer pourquoi j'accepte un résultat. Dès que l'on cherche à expliquer, le compte rendu change de statut. Ce n'est plus seulement un message entre deux agents. Il porte une identité (qui l'a écrit, pour quelle tâche), une autorité (ce que cet agent avait le droit de faire) et une mémoire (ce qui sera réutilisé plus tard, et par qui).

## Le moteur déterministe Rust garde les transitions

Le cœur de Swarm est un moteur déterministe écrit en Rust, dans swarm-engine, avec WorkEngine et RootStore. Il relie l’objectif, le plan adopté, les tâches, les dépendances, les droits, les réservations, les budgets et les preuves. Les agents proposent et raisonnent ; le moteur vérifie les préconditions de l’opération publique concernée avant d’enregistrer la transition autorisée [9].

Le superviseur racine garde l’objectif et arbitre les problèmes transversaux. Les sous-superviseurs possèdent leur périmètre. Le jugement global de cycle reste distinct de la revue indépendante d’un candidat. Une sortie de commande, un rapport d’agent ou un test vert ne suffit pas à accepter une livraison : il faut relier les preuves au candidat et vérifier leur fraîcheur. La vue 1 présente cette organisation complète.

Le contrôle humain fait partie du contrat du moteur. La politique de validation distingue human et automatic. En mode human, les commandes préautorisées produisent des preuves pour la revue, mais un succès conserve le résultat en attente de décision humaine. Il ne libère pas les tâches suivantes. Une revue IA favorable ne remplace pas cette validation. Le moteur doit faire respecter la politique enregistrée [9].

Déterministe décrit ici les contrôles et transitions du moteur. Cela ne rend pas le modèle déterministe et ne prouve pas l’innocuité d’une commande. La qualification Rust avance par parcours ; les schémas représentent les contrats d’architecture, avec les raccords encore ouverts explicitement indiqués [9].

## Un nom ne fait pas une identité

Appeler un agent « reviewer » ne dit pas grand-chose. Derrière ce nom, il faut distinguer la personne ou le service qui a lancé le travail, l'agent logique, l'instance qui s'exécute réellement et le fournisseur du modèle. Ce sont quatre réponses différentes à la question « qui ? ».

Le contrat d’identité de Swarm distingue le principal à l’origine de l’autorité, l’agent logique et l’instance réellement exécutée. Une autorisation porte sur une portée, un plan courant, une opération, une ressource et un contenu précis. Les octrois, les possibilités de délégation, les dates et les révocations doivent être vérifiés au moment de l’usage. L’historique conserve l’auteur de la décision, même après révocation [9].

La Secure Software Factory d’Agent Baseline sépare la planification de l’agent, l’autorisation et l’application par l’infrastructure [2]. C’est aussi le sens de mon architecture : le texte d’un plan ou une mémoire ne doit pas créer de droits. Leurs catalogues YAML aident à exprimer les contrôles et leurs objectifs ; ils ne constituent pas, à eux seuls, une politique de confinement exécutable.

Cette identité a déjà des composants et des protocoles matérialisés : enrôlement signé, journal, documents d’autorité, configuration et contrôles de données. Des preuves locales existent sur des périmètres isolés. Le compte humain de référence, le service protégé et le raccord de toutes les opérations publiques au dernier usage restent à intégrer ou à qualifier. La vue 2 montre la chaîne complète ; elle ne certifie pas un service d’identité déployé [9].

## Une mémoire qui garde ses sources

Une mémoire permanente d’agent est un stockage réinjecté dans de futurs contextes. Elle peut traverser les sessions et les instances qui y ont accès. Cette continuité est utile, mais elle prolonge aussi une erreur ou une injection si l’admission et la réutilisation ne sont pas maîtrisées. C’est le risque ASI06 décrit par l’OWASP [3].

Le parcours mémoire documenté de Swarm part d’un retour d’expérience vérifié. Une leçon est admise comme une version, avec ses sources et ses conditions d’usage, puis sélectionnée pour une tâche précise. L’assemblage relie les extraits au contexte préparé et au prompt final, avec versions et empreintes. Contexte préparé, envoi au fournisseur et réception effective restent des preuves différentes [9].

La mémoire Swarm n’est pas définie par le langage de son implémentation historique. Son contrat impose une sélection explicite, une provenance et des usages contrôlés. Une leçon ne doit élargir ni les droits ni le budget. Le contexte natif Rust est partiellement porté ; la mémoire gérée et l’application globale des droits d’usage restent des chantiers ouverts [9].

Un point m'a demandé du temps : lire, résumer et transmettre sont trois actes différents. Un agent peut avoir besoin de lire une note sans avoir le droit d'en faire un résumé pour un autre, et encore moins de la lui transmettre telle quelle. C'est la logique du besoin d'en connaître, que l'ANSSI recommande de prendre en compte dès la conception d'un système d'IA générative [4]. Relier ces trois décisions de bout en bout fait partie de mes prochaines étapes.

Retirer une leçon empêche sa sélection future dans le parcours concerné ; cela ne purge pas les prompts, historiques ou copies déjà reçus. Une autre instance ne doit donc pas hériter implicitement de tous les droits de la précédente. Les reprises doivent identifier les versions reçues et réconcilier les effets incertains avant un rejeu. La vue 3 relie connaissances de projet, consignes, contexte de tentative et reprise de conversation [9].

## Des consignes conservées et des révisions contrôlées

Il serait inexact de dire que les consignes ne peuvent jamais changer. Le contrat documenté conserve la consigne initiale, instruction_base, et attribue les révisions suivantes. Le superviseur propriétaire peut proposer un ajout pour une tâche future, avec un motif fondé sur un événement reçu. Le moteur contrôle notamment le propriétaire, la révision, le bail et l’état de la tâche ; une tentative déjà réservée ne doit pas recevoir une réécriture silencieuse [9].

Le comportement documenté mérite d’être explicite : les ajouts du superviseur sont validés par défaut. L’opérateur peut les rejeter avant réservation ; après transmission, ce rejet n’efface pas le contexte reçu. Ce n’est donc pas une approbation humaine préalable de chaque apprentissage. La propriété recherchée est une évolution attribuée et contrôlée, dont on peut retrouver la version utilisée par chaque tentative [9].

Une modification opérateur peut aussi faire évoluer la base courante, en conservant l’original et l’historique des apprentissages. La mémoire n’autorise donc pas à changer librement le contrat de la tâche. Ces gardes sur les opérations du moteur ne prouvent toutefois ni que le modèle obéit toujours au prompt, ni qu’un agent est déjà confiné au niveau du système d’exploitation [9].

## Le contrôle humain dans la boucle de supervision

Dans la conduite documentée, un incident reste attribué à une tâche, un agent et une tentative. Le superviseur de périmètre reçoit le signal, établit le diagnostic et propose une décision ; le superviseur racine arbitre ce qui dépasse ce périmètre. Le moteur vérifie puis applique la décision autorisée. Il faut ensuite constater son effet avant de reprendre le besoin initial. Un log d’erreur ou une annonce de correction ne termine pas cette boucle. La vue 4 détaille ce circuit [9].

Les contrôles exécutables vérifient le candidat ; le vérificateur IA examine les pièces dans un contexte indépendant du producteur. Il doit rattacher ses observations aux sources reçues. Le moteur contrôle ensuite l’attribution, le candidat, les critères et la fraîcheur avant d’appliquer une acceptation. Il vérifie les conditions de la décision ; il ne certifie pas à lui seul la justesse sémantique de l’analyse [9].

L’humain fixe l’objectif, le périmètre, les droits et la politique de validation. Quand cette politique exige une revue humaine, il examine le résultat et ses preuves avant la décision de validation. Les contrôles automatiques et la revue indépendante l’informent ; ils ne prennent pas sa place. Cette séparation est un mécanisme de sécurité du moteur, pas seulement une possibilité d’interrompre un agent [9].

Le portage Rust matérialise déjà cette distinction dans le contrôleur natif : le mode human conserve les gates et checks manuels et signale human-validation-required. Sa qualification locale porte sur des fixtures et ne prouve pas l’authentification humaine complète du futur service d’identité. Les interventions de l’opérateur, dont l’arrêt et le rejet d’un apprentissage avant réservation, restent distinctes de la validation d’un résultat [9].

## Des logs pour reconstruire la décision

La vue 5 conserve la distinction entre sorties d’agents, terminal, événements moteur, registre des appels, mesures système et télémétrie. Ils servent à reconstruire qui a reçu quel contexte, quelle décision a été prise, ce qui a été appliqué et quel résultat a été constaté. Une dépense réservée n’est pas une facture ; un événement affiché n’est pas une acceptation. La future timeline de sécurité et la chaîne d’une connexion complèteront cette observabilité [9].

## La prochaine étape du contrôle des actions

Le plan AC02 est enregistré et n’est pas démarré. Après la migration et la qualification Rust, puis une décision explicite de reprise, il prévoit le confinement Linux des processus et des fichiers, le contrôle du réseau TCP et REST, l’usage encadré des secrets et une chaîne reliant tâche, outil, processus, connexion et décision. Une destination autorisée ne garantit pas à elle seule l’absence d’exfiltration ou d’effet métier indésirable [9].

Pour le format des règles, je m'appuie sur la politique YAML d'OpenShell publiée par NVIDIA [5]. Swarm aura son propre moteur, avec un sous-ensemble compatible clairement défini et testé.

L’article de Renaud Deraison sur Bromure [6] montre comment rendre une protection compréhensible : remonter d’une connexion refusée à la commande et au contexte qui l’ont provoquée. Il inspire quatre vues de sécurité prévues dans AC02 : éditeur de politique, panneau des refus, chaîne d’une connexion et timeline de sécurité. Elles sont distinctes des cinq vues d’architecture intégrées à ce document. Une comparaison avec une référence OpenShell figée devra éprouver la compatibilité annoncée ; elle ne prouvera pas, à elle seule, le confinement réel.

## État actuel

Le moteur Rust existe et reste le cœur du projet. Les preuves disponibles couvrent notamment des parcours worker, supervision et reprise sur fixtures CLI. Elles ne qualifient pas encore l’ensemble des rôles, de l’identité, de la mémoire et de l’interface web. La publication de la version issue de cette migration reste une étape à venir ; les cinq planches rendent visibles l’architecture et ses limites actuelles [9].

Mon objectif n'a pas changé depuis le premier article : faire de l'ingénierie logicielle avec des IA, avec des décisions de validation que l'on peut expliquer. Pour chaque résultat, je veux pouvoir répondre à cinq questions. Quelle tâche ? Quel agent ? Quelle autorité ? Quelle version de mémoire ? Quelles preuves ?

Avez-vous déjà vu une consigne douteuse revenir dans une nouvelle session, parce qu’elle avait été conservée comme mémoire ou transmise par un autre agent ? Comment reliez-vous son origine, les droits de l’agent et les preuves soumises à la validation humaine ? Ce sont ces situations que je veux rendre examinables dans Swarm.

#AgentsIA #SecuriteIA #IngenierieLogicielle #IntelligenceArtificielle #Rust

## Références

[1] Lin, W. Towards self-driving codebases. Cursor, 5 février 2026.

[2] Agent Baseline. The Secure Software Factory: A Reference Architecture. Version 1, 13 octobre 2026, révision d842a1b.

Catalogue YAML associé.

[3] OWASP Gen AI Security Project. Top 10 for Agentic Applications 2026. ASI03 et ASI06.

[4] ANSSI. Recommandations de sécurité pour un système d’IA générative. 29 avril 2024, R8 sur le besoin d’en connaître.

[5] NVIDIA. OpenShell Policy Schema Reference, version 0.1.5.

[6] Deraison, R. OpenShell policies in Bromure, and the reasoning behind every connection. 21 octobre 2026.

[7] Dong, S. et al. Memory Injection Attacks on LLM Agents via Query-Only Interaction. MINJA, arXiv:2503.03704v5, 12 février 2026.

[8] Ravia, I., Aim Labs. Breaking down EchoLeak. CVE-2025-32711, recherche divulguée en 2025, reprise chez Cato Networks.

[9] Documentation interne Swarm relue le 23 octobre 2026. Architecture : docs/architecture/OVERVIEW.md. Validation humaine : docs/ENGINE-ACCEPTANCE.md, section Contrôles comme preuves d’une revue humaine ; docs/architecture/RUST-MIGRATION.md, raccord139 ; rust/crates/swarm-engine/src/root_store/task_validation/gate.rs. Consignes : INSTRUCTION-LEARNING.md ; identité : AGENT-IDENTITY-CONTRACT.md ; supervision : RUN-SUPERVISION.md ; preuves : REVIEW-EVIDENCE.md ; mémoire : docs/PROJECT-MEMORY.md. Correspondances dans docs/diagrams/CONTRACT-MAP.md. Les schémas décrivent des contrats, pas une qualification globale Rust.

Les cinq vues complètes qui suivent sont intégrées au Word. Le grand format conserve les détails des sources Mermaid ; l’article reste en A4. Les traits et légendes signalent les raccords ouverts. Les couleurs servent à distinguer les responsabilités, sans établir un niveau de qualification.

## Vue 1 Moteur Rust et responsabilités

Le moteur déterministe garde les transitions et les validations humaines

Objectif et plan adopté, propriétaire de périmètre, jugement global de cycle, réservations et contexte figé, conducteur, candidat Git, contrôles, revue indépendante et fraîcheur restent reliés. La validation humaine exigée par la politique reste une condition d’acceptation ; le superviseur IA ne peut pas la remplacer par son avis.

Limite de lecture. Qualification native par parcours. En mode human, un PASS de contrôle conserve l’attente de décision humaine. AC02 reste futur.

Sources internes. AGENT-ENGINE-PROTOCOL.md · ENGINE-ACCEPTANCE.md · REVIEW-EVIDENCE.md · TRACEABILITY.md · JUDGMENT-CYCLES.md · Correspondances détaillées dans CONTRACT-MAP.md.

Source Mermaid conservée dans docs/diagrams/architecture-overview-fr.mmd. Figure complète issue de la documentation interne du 23 octobre 2026.

## Vue 2 Identité et autorité des agents

Une autorisation lie un acteur à un usage exact

Source humaine, enrôlement et preuve signée, octrois, instances et permis de publication sont séparés. Le plan courant et le contenu de la demande participent au contrôle. Trust, SQLite et runtime constituent des vérifications distinctes.

Limite de lecture. Noyaux présents et preuves bornées. Le compte humain, le service protégé et le raccord public de toutes les opérations au dernier usage restent ouverts.

Sources internes. AGENT-IDENTITY-CONTRACT.md · CONTRAT-USAGES-MEMOIRE.md · Correspondances détaillées dans CONTRACT-MAP.md.

Source Mermaid conservée dans docs/diagrams/identity-authority-fr.mmd. Figure complète issue de la documentation interne du 23 octobre 2026.

## Vue 3 Mémoire persistante et contexte

Une nouvelle instance doit connaître la provenance de ce qu’elle reçoit

Une leçon admise, une révision de consigne, un contexte préparé et une conversation reprise ne sont pas le même objet. La sélection attribuée, les empreintes et les reçus permettent de retrouver la version transmise. Lire, résumer et transmettre restent trois usages distincts.

Limite de lecture. Contexte Rust partiellement porté et droits mémoire globaux ouverts. Un retrait ne purge pas les contextes déjà reçus. Les ajouts de consigne sont validés par défaut puis rejetables avant réservation.

Sources internes. PROJECT-MEMORY.md · INSTRUCTION-LEARNING.md · CONTRAT-USAGES-MEMOIRE.md · Correspondances détaillées dans CONTRACT-MAP.md.

Source Mermaid conservée dans docs/diagrams/memory-context-fr.mmd. Figure complète issue de la documentation interne du 23 octobre 2026.

## Vue 4 Supervision et traitement des incidents

La conduite IA reste soumise aux contrôles et validations du moteur

Durée, attente d’outil, silence et échecs restent des observations attribuées. Le superviseur IA propriétaire reçoit le signal et motive sa décision. Le moteur vérifie puis applique ; la confirmation d’arrêt précède la libération. Au stade du résultat, la politique human exige la validation humaine et garde les tâches suivantes bloquées jusque-là.

Limite de lecture. Les preuves sur fixtures ne valent pas conduite native complète. Conduite du processus, revue IA et validation humaine sont distinctes. Continuer ne réinitialise pas les compteurs.

Sources internes. RUN-SUPERVISION.md · ENGINE-ACCEPTANCE.md · AGENT-ENGINE-PROTOCOL.md · PROVIDER-LIVENESS.md · Correspondances détaillées dans CONTRACT-MAP.md.

Source Mermaid conservée dans docs/diagrams/run-supervision-fr.mmd. Figure complète issue de la documentation interne du 23 octobre 2026.

## Vue 5 Logs et preuves de supervision

Reconstituer la chaîne sans confondre observation et acceptation

Sorties, événements, comptabilité des appels, ressources Linux et télémétrie gardent leurs sources. La consultation, les curseurs et l’export rendent le parcours examinable. Une acceptation demande aussi le candidat, les contrôles, la revue et une vérification de fraîcheur.

Limite de lecture. Surfaces live et OTLP vérifiées localement dans leur périmètre. La timeline de sécurité et la filiation réseau AC02 restent futures.

Sources internes. LOG-CONSOLE.md · CALL-ACCOUNTING.md · TRACEABILITY.md · RUST-MIGRATION.md · Correspondances détaillées dans CONTRACT-MAP.md.

Source Mermaid conservée dans docs/diagrams/logs-observability-fr.mmd. Figure complète issue de la documentation interne du 23 octobre 2026.

## Liens des références

[1] Article Cursor — https://cursor.com/blog/self-driving-codebases

[2] Architecture de référence — https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/software-factory/software-factory.md

[3] Publication OWASP du 9 décembre 2025 — https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/

[4] Guide ANSSI — https://messervices.cyber.gouv.fr/guides/recommandations-de-securite-pour-un-systeme-dia-generative

[5] Schéma versionné OpenShell — https://docs.nvidia.com/openshell/v0.1.5/how-it-works/policies/schema

[6] Article Bromure — https://bromure.io/en/blog/openshell-policies-and-network-lineage

[7] Étude et conditions expérimentales — https://arxiv.org/abs/2503.03704v5

[8] Analyse des chercheurs et limites de l’impact constaté — https://www.catonetworks.com/blog/breaking-down-echoleak/
