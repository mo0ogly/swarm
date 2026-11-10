ARTICLE LINKEDIN | Swarm deuxième partie | 23 octobre 2026 | version 5

# Identité et mémoire des agents IA dans Swarm

Moteur déterministe Rust et contrôle humain

La mémoire d’un agent influence ce qu’il fera demain. Une mauvaise commande, conservée comme solution puis transmise à une autre instance, peut survivre à la session qui l’a produite. C’est une question centrale pour Swarm : relier chaque contexte à sa provenance, chaque agent à une identité et chaque action à une autorité explicite. Le moteur déterministe Rust porte les contrôles et les validations définis par l’humain. Cet article présente cette architecture, les recherches qui motivent ces choix et cinq vues détaillées issues de la documentation interne.

Dans mon premier article sur Swarm, j'expliquais ce qui m'avait poussé à le développer : voir qui prend en charge une tâche, ce qu'il attend des autres et ce qu'il leur transmet.

## La mémoire participe à l’identité fonctionnelle

Un agent IA n’est pas seulement son modèle. Les préférences qu’il conserve, les objectifs qu’il reprend et les expériences qu’il retrouve contribuent à son comportement. Cette continuité forme une partie de son identité fonctionnelle : la manière dont il se situe dans le travail. Elle reste différente de son identité authentifiée, de ses permissions et de la personne ou du service pour lequel il agit.

Dès qu’un souvenir est relu comme une consigne, la mémoire devient une surface de contrôle du comportement. C’est précisément le passage à surveiller : une information conservée ne doit pas acquérir une autorité simplement parce qu’elle est ancienne, formulée avec assurance ou présentée comme une réussite passée. Le risque d’empoisonnement de la mémoire et du contexte figure dans ASI06 de l’OWASP [3].

Imaginons un agent de développement qui lit une instruction trompeuse dans une documentation, un courriel ou une réponse d’outil. Il l’applique, puis la reformule dans son compte rendu comme une procédure utile. Si ce résumé entre en mémoire sans ses réserves ni ses sources, une nouvelle instance peut le recevoir comme une pratique acquise. Le redémarrage du processus ne suffit alors plus à repartir d’un contexte sain. Ce scénario est illustratif ; je ne le présente pas comme un incident observé dans Swarm.

## Quand une erreur circule entre agents

Une attaque n’est pas nécessaire pour amorcer la dérive. L’agent peut lui-même proposer une mauvaise commande, la conserver comme solution puis la réutiliser. Dans une équipe d’agents, un résumé, un plan ou un résultat partiel peut ensuite transporter l’erreur vers un autre contexte. Plusieurs reprises du même contenu peuvent donner une apparence de confirmation sans apporter de preuve indépendante. J’emploie ici contamination du contexte pour désigner ce mécanisme ; il ne s’agit ni d’une modification des poids du modèle, ni d’un virus qui se réplique seul.

Des recherches ont éprouvé différents maillons de cette chaîne. MINJA étudie l’injection de traces par des interactions avec l’agent, sans écriture directe de sa base [7]. MemoryGraft examine la réutilisation d’expériences présentées comme des réussites [10]. MemGhost évalue un courriel qui fait écrire une mémoire empoisonnée, masque cette écriture dans la réponse et influe ensuite sur le comportement [13]. Ces résultats décrivent leurs environnements expérimentaux ; ils ne démontrent pas une compromission de tous les agents.

## L’origine d’un souvenir ne lui donne pas tous les droits

Pour chaque message ou souvenir, il faut pouvoir retrouver qui l’a produit, dans quelle tâche, pour quel destinataire et avec quelle autorisation. Une identité authentifiée ne rend pas un contenu juste. Un agent autorisé peut se tromper ou travailler à partir d’une source hostile ; un contenu signé peut donc rester dangereux.

Dans un billet du 27 août 2026, le NIST appelle à donner aux agents leurs propres identifiants et permissions, rattachés à l’utilisateur ou au système qui les mandate. Il recommande des autorisations dynamiques, étroitement limitées et destinées au bon service, en s’appuyant notamment sur OAuth 2.0 et SPIFFE [15]. Les auteurs soulignent aussi le risque de fatigue du consentement lorsque les demandes d’approbation deviennent trop fréquentes.

C’est la séparation que je veux rendre explicite dans Swarm : la mémoire informe le raisonnement ; les contrats du moteur établissent les droits et les conditions de validation. Une leçon ne doit ni élargir un périmètre, ni créer un budget, ni remplacer la validation humaine exigée par la politique. Les tests et la revue IA apportent des preuves à cette décision.

## Ce qu’une mémoire contrôlée doit permettre

Il faut examiner l’admission d’un souvenir puis chacun de ses usages : écriture, lecture, résumé et transmission. La provenance doit relier auteur, tâche, tentative, version et source ; une frontière de projet ou de destinataire ne doit pas disparaître au passage. Les critères d’admission, les droits et les schémas sont des contrôles distincts de l’analyse du texte. Une empreinte ou une signature peut établir une intégrité et une origine vérifiables sous ses hypothèses de confiance ; elle ne prouve ni la vérité du souvenir ni l’absence d’injection.

La détection d’instructions suspectes ou de secrets, la mise à l’écart d’une entrée et la préparation d’une restauration sont des pistes complémentaires à spécifier et à éprouver. Leur efficacité ne se déduit pas de leur nom. Le projet de benchmark déposé le 17 octobre 2026 dans le Datatracker IETF propose, par exemple, de tester les altérations, suppressions, réordonnancements, rejouements et retours à un ancien stockage. Il s’agit d’une contribution individuelle, sans statut de norme IETF ; elle mesure une intégrité au repos, pas l’innocuité sémantique des souvenirs [17].

Dans Swarm, les contrats de provenance, de versions et d’autorité guident déjà l’architecture ; leur raccord Rust reste à qualifier par parcours. Je ne présente pas un filtre universel des lectures et écritures, une détection d’injection ou une quarantaine mémoire comme des protections déjà disponibles. Retirer une entrée doit aussi rester distinct de l’effacement des contextes qui l’ont reçue.

L’enjeu est concret : lorsqu’un agent reprend le travail, peut-on retrouver ce qu’il croit avoir appris, qui le lui a transmis, ce qu’il a le droit d’en faire et les preuves que l’humain devra examiner ? C’est cette chaîne que les cinq vues de Swarm rendent visible.

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

## Repères publics sur la persistance des attaques

Ces publications situent le risque ; elles ne sont ni des tests de Swarm ni une mesure de la fréquence des attaques en production. Les dates distinguent parution initiale, révision et divulgation.

### Mars et décembre 2025 puis février 2026

MINJA paraît le 5 mars 2025 ; la version citée est révisée le 12 février 2026 [7]. MemoryGraft est déposé le 18 décembre 2025 et évalué sur DataInterpreter de MetaGPT avec GPT 4o [10]. EchoLeak, divulgué en 2025 par Aim Labs, concerne une injection indirecte via courriel et une fuite de données dans Microsoft 365 Copilot ; cette vulnérabilité corrigée ne constitue pas une preuve d’empoisonnement mémoire dans Swarm. L’analyse rapporte que Microsoft n’a constaté aucun client affecté [8].

### Février 2026 et les recommandations commerciales

Le 10 février, Microsoft décrit des tentatives d’AI Recommendation Poisoning dans des boutons Summarize with AI : les liens transmettent des instructions destinées à faire mémoriser une marque comme source fiable ou à la recommander en priorité. L’efficacité varie selon l’assistant et dans le temps ; Microsoft indique avoir déployé des atténuations. Une tentative observée n’équivaut donc pas à une mémoire effectivement modifiée [11].

### 2026 et les trajectoires simulées

Yazdinejad et Karimipour étudient 2 614 trajectoires simulées dans Temporal Dynamics of Memory Poisoning in Web3 Style LLM Agents, publié dans IEEE Access en 2026. La version d’auteur consultée est référencée en mai. Les scénarios et outils sont abstraits, sans transaction réelle : leur intérêt est d’examiner l’activation différée et les limites d’une évaluation étape par étape, sans mesurer des incidents en production [12].

### Juillet 2026 et le souvenir écrit depuis un courriel

MemGhost, déposé le 6 juillet dans When Claws Remember but Do Not Tell, évalue toute la séquence : courriel externe, écriture persistante, discrétion de la réponse et effet ultérieur. Il s’agit d’une prépublication et d’un benchmark sur des configurations précises, pas d’un taux de compromission applicable à tous les assistants [13].

### Septembre 2026 et la propagation entre agents

Emergence World, déposé le 15 septembre, suit huit mondes simulés. Les auteurs distinguent détection, retenue, propagation et reprise. Dans un cas, un agent tente de rouvrir un lien hostile environ 46 heures après la fin de la campagne, depuis un message conservé ; le service renvoie alors une réponse sûre. L’étude ne dit pas que ce même agent avait détecté ce lien exactement 46 heures auparavant. D’autres agents ont stocké la charge comme documentation. Ce résultat montre une possibilité dans l’expérience, sans établir sa fréquence dans un système réel [14].

### Octobre 2026 et les permissions cloud

Le 8 octobre, Zenity publie AgentCorruption : dans sa chaîne de recherche sur AgentCore, l’accès aux identifiants du rôle d’exécution, puis des permissions trop larges, permet d’atteindre conversations et mémoires d’autres agents du même compte AWS et de la même région. Le rapport signale un durcissement important du rôle par défaut observé dès le 29 septembre. AWS décrit l’accès aux identifiants du propre rôle comme attendu et recommande le moindre privilège. Ce cas relie contexte, identité et permissions ; il ne prouve pas que tous les déploiements actuels restent exposés [16, 18].

Le dépôt individuel IETF du 17 octobre complète ces travaux par une proposition de mesure de l’intégrité du stockage [17]. Détecter une altération des octets, reconnaître une instruction hostile et autoriser une action restent trois propriétés à vérifier séparément.

## Références

[1] Lin, W. Towards self-driving codebases. Cursor, 5 février 2026. Article Cursor

[2] Agent Baseline. The Secure Software Factory: A Reference Architecture. Version 1, 13 octobre 2026, révision d842a1b. Architecture de référence

Catalogue YAML associé. Contrôles Agent Baseline à la même révision

[3] OWASP Gen AI Security Project. Top 10 for Agentic Applications 2026. ASI03 et ASI06. Publication OWASP du 9 décembre 2025

[4] ANSSI. Recommandations de sécurité pour un système d’IA générative. 29 avril 2024, R8 sur le besoin d’en connaître. Guide ANSSI

[5] NVIDIA. OpenShell Policy Schema Reference, version 0.1.5. Schéma versionné OpenShell

[6] Deraison, R. OpenShell policies in Bromure, and the reasoning behind every connection. 21 octobre 2026. Article Bromure

[7] Dong, S. et al. Memory Injection Attacks on LLM Agents via Query-Only Interaction. MINJA, arXiv:2503.03704v5, 12 février 2026. Étude et conditions expérimentales

[8] Ravia, I., Aim Labs. Breaking down EchoLeak. CVE-2025-32711, recherche divulguée en 2025, reprise chez Cato Networks. Analyse des chercheurs et limites de l’impact constaté

[9] Documentation interne Swarm relue le 23 octobre 2026. Architecture : docs/architecture/OVERVIEW.md. Validation humaine : docs/ENGINE-ACCEPTANCE.md, section Contrôles comme preuves d’une revue humaine ; docs/architecture/RUST-MIGRATION.md, raccord139 ; rust/crates/swarm-engine/src/root_store/task_validation/gate.rs. Consignes : INSTRUCTION-LEARNING.md ; identité : AGENT-IDENTITY-CONTRACT.md ; supervision : RUN-SUPERVISION.md ; preuves : REVIEW-EVIDENCE.md ; mémoire : docs/PROJECT-MEMORY.md. Correspondances dans docs/diagrams/CONTRACT-MAP.md. Les schémas décrivent des contrats, pas une qualification globale Rust.

[10] Srivastava, S. S., et He, H. MemoryGraft. Prépublication arXiv 2512.16962v1, 18 décembre 2025. Étude et périmètre expérimental

[11] Microsoft Defender Security Research Team et Kochavi, N. Manipulating AI memory for profit. 10 février 2026. Analyse Microsoft et atténuations

[12] Yazdinejad, A., et Karimipour, H. Temporal Dynamics of Memory Poisoning in Web3-Style LLM Agents. IEEE Access 14, 76200–76221, 2026. DOI 10.1109/ACCESS.2026.3693560. Version d’auteur consultée sur ResearchGate

[13] Zhang, Y., et al. When Claws Remember but Do Not Tell. MemGhost, prépublication arXiv 2607.05189v1, 6 juillet 2026. Étude et conditions du benchmark

[14] Akkil, D., et al. Emergence World: Adversarial Stress-Testing of Long-Horizon Multi-Agent Systems. Prépublication arXiv 2609.17320v1, 15 septembre 2026. Texte intégral et limites de la simulation

[15] Fisher, B., et Galluzzo, R. Back to the Future: Why Agentic AI Needs a Strong Identity Foundation. Blog NIST, 27 août 2026. Identité et autorisations des agents

[16] Sharbat, T. I., et Salameh, L. AgentCorruption. Zenity Labs, 8 octobre 2026. Recherche, chronologie et réponse AWS

[17] Khandelwal, Y. A Benchmarking Method for the Integrity of AI Agent Memory at Rest. Internet Draft individuel 02, 17 octobre 2026. Sans statut de norme IETF. Version figée et statut du document

[18] AWS. Understanding Credentials Management in Amazon Bedrock AgentCore. Documentation consultée le 23 octobre 2026. Gestion du rôle et moindre privilège

## Présentation vidéo

### Swarm identité mémoire et contrôle humain

Le film de 1 min 40 suit une mauvaise consigne conservée puis transmise, avant de présenter l’identité, le moteur déterministe Rust, la validation humaine, la supervision et les preuves. La mise en scène reste explicative ; elle ne représente pas une session de l’application filmée.

Le cœur du moteur et ses contrats dans le film.

Ouvrir le film MP4 placé à côté de ce document

Français à l’écran, musique instrumentale originale, sans voix. Sous-titres FR et EN disponibles avec le média. Les textes structurants restent lisibles sans son.

00:00 Mémoire et transmission   ·   00:22 Identité   ·   00:34 Rust
00:46 Validation humaine   ·   00:58 Supervision   ·   01:10 Preuves
01:20 AC02 futur   ·   01:30 Architecture complète

Les cinq vues techniques qui suivent conservent tous leurs composants. Les schémas d’identité et de mémoire rappellent désormais qu’une signature ne prouve pas la vérité du contenu et qu’un souvenir ne crée aucun droit. Les 18 références de l’article restent conservées. AC02 n’est pas démarré.

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

## Liens des nouvelles références

[10] [Étude et périmètre expérimental](https://arxiv.org/abs/2512.16962v1)

[11] [Analyse Microsoft et atténuations](https://www.microsoft.com/en-us/security/blog/2026/02/10/ai-recommendation-poisoning/)

[12] [Version d’auteur consultée sur ResearchGate](https://www.researchgate.net/publication/404886610_Temporal_Dynamics_of_Memory_Poisoning_in_Web3-Style_LLM_Agents)

[13] [Étude et conditions du benchmark](https://arxiv.org/abs/2607.05189v1)

[14] [Texte intégral et limites de la simulation](https://arxiv.org/html/2609.17320v1)

[15] [Identité et autorisations des agents](https://www.nist.gov/blogs/cybersecurity-insights/back-future-why-agentic-ai-needs-strong-identity-foundation)

[16] [Recherche, chronologie et réponse AWS](https://labs.zenity.io/post/agentcorruption-how-a-single-prompt-collapsed-the-entire-cloud-security-model)

[17] [Version figée et statut du document](https://datatracker.ietf.org/doc/draft-khandelwal-bmwg-agent-memory-integrity/02/)

[18] [Gestion du rôle et moindre privilège](https://docs.aws.amazon.com/bedrock-agentcore/latest/devguide/security-credentials-management.html)

## Liens des références

[1] Article Cursor — https://cursor.com/blog/self-driving-codebases

[2] Architecture de référence — https://github.com/agentbaseline/agentbaseline/blob/d842a1bb34a42e9d7c0d54a1b12ead9e0e59e283/software-factory/software-factory.md

[3] Publication OWASP du 9 décembre 2025 — https://genai.owasp.org/resource/owasp-top-10-for-agentic-applications-for-2026/

[4] Guide ANSSI — https://messervices.cyber.gouv.fr/guides/recommandations-de-securite-pour-un-systeme-dia-generative

[5] Schéma versionné OpenShell — https://docs.nvidia.com/openshell/v0.1.5/how-it-works/policies/schema

[6] Article Bromure — https://bromure.io/en/blog/openshell-policies-and-network-lineage

[7] Étude et conditions expérimentales — https://arxiv.org/abs/2503.03704v5

[8] Analyse des chercheurs et limites de l’impact constaté — https://www.catonetworks.com/blog/breaking-down-echoleak/



[Film MP4](Swarm_Architecture_Film_v2.mp4)
