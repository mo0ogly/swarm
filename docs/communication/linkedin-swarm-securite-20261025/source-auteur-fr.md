# IA agentique : jusqu’où peut-on déléguer sans perdre le contrôle ?

### Pour un cadre d'assurance de l'exécution des processus critiques

*Fabrice Pizzi — Swarm | Octobre 2026*

## L'autonomie progresse. Mais qu'en est-il de notre capacité à la maîtriser ?

Les progrès de l'intelligence artificielle agentique changent profondément notre manière de concevoir les systèmes informatiques.

Les modèles gagnent en capacité, les outils évoluent rapidement et les architectures multi-agents permettent d'automatiser des processus toujours plus complexes.

Une personne seule peut désormais développer des systèmes dont la réalisation aurait auparavant mobilisé plusieurs spécialistes pendant plusieurs mois.

Cette démocratisation est extraordinaire. Mais elle crée un déséquilibre.

**Notre capacité à construire des systèmes autonomes progresse plus vite que notre capacité à démontrer que nous en maîtrisons l'exécution.**

Un agent ne se contente plus de fournir une réponse.

Il peut décomposer un objectif, créer des sous-agents, appeler des outils, modifier des données, accéder à des services externes, conserver des expériences et poursuivre une mission pendant une période prolongée.

Cette autonomie introduit une nouvelle difficulté : nous ne pouvons plus supposer que chaque décision intermédiaire sera examinée par un humain.

La question n'est donc plus seulement de savoir si le modèle produit de bonnes réponses.

Elle devient : **dans quelles conditions peut-on lui permettre de produire des conséquences réelles ?**

## Le problème n'est pas simplement la fiabilité du modèle

Les systèmes agentiques associent des mécanismes de raisonnement probabilistes à des environnements informatiques dont les règles de fonctionnement sont souvent déterministes.

Cette association est précisément ce qui les rend puissants.

Le modèle interprète, explore et propose. Les systèmes qu'il utilise exécutent des opérations concrètes.

Mais cette architecture crée une asymétrie fondamentale.

Une erreur de raisonnement peut rester bénigne tant qu'elle demeure une proposition. Elle change de nature lorsqu'elle devient un virement, une modification de droits, une suppression de données ou une commande adressée à une infrastructure.

Les mécanismes traditionnels de sécurité ne disparaissent pas. Ils deviennent au contraire indispensables.

Authentification, autorisation, séparation des privilèges, isolation des processus, vérification des transactions et journalisation doivent être appliquées à une nouvelle catégorie d'acteurs : des programmes capables de déterminer dynamiquement leurs prochaines actions.

À cela s'ajoute une difficulté particulière.

Dans un système agentique, les informations qui influencent une décision peuvent évoluer pendant l'exécution.

Un document externe peut contenir une injection de prompt. Un résultat d'outil peut être trompeur. Un résumé transmis entre agents peut perdre sa provenance. Une mémoire persistante peut conserver une procédure erronée ou malveillante et la réintroduire plusieurs sessions plus tard.

L'OWASP décrit ces risques dans son Top 10 pour les applications agentiques, notamment à travers le détournement d'objectif, les abus de privilèges, l'empoisonnement de mémoire et les défaillances en cascade.

Ces catégories mettent en évidence une propriété essentielle : **une information n'acquiert pas une autorité du seul fait qu'un agent l'a mémorisée, répétée ou transmise.**

## De la sécurité des agents à l'assurance de leurs effets

La recherche et les initiatives de standardisation commencent à apporter des réponses importantes.

Le projet CaMeL, de Debenedetti et ses coauteurs, montre l'intérêt d'une séparation explicite entre flux de contrôle et flux de données, associée à des capacités restreignant les appels d'outils.

AgentSpec explore l'application de contraintes déclaratives pendant l'exécution.

L'OWASP Agent Control Standard, publié en septembre 2026, définit un protocole d'interception et de décision permettant à un Guardian de contrôler les opérations d'un agent.

SPIFFE apporte des identités vérifiables pour les workloads. Les standards OAuth permettent d'exprimer des autorisations structurées et des délégations.

NVIDIA OpenShell propose des politiques de confinement des fichiers, processus et accès réseau.

Enfin, un Internet-Draft individuel déposé par Sangam Das en septembre 2026 propose une architecture d'autorisation graduée et de contrôle de l'engagement final des actions critiques.

Ces travaux montrent que le problème est identifié et que les réponses techniques progressent.

Il serait donc inexact d'affirmer que les standards sont inexistants.

En revanche, **l'existence de mécanismes de contrôle ne démontre pas encore leur continuité sur l'ensemble du cycle de vie d'un processus agentique**.

Une identité peut être authentifiée sans que l'action soit autorisée.

Une action peut avoir été approuvée puis modifiée avant son exécution.

Une mémoire peut conserver des données intègres mais mensongères.

Une décision de refus peut être contournée par un autre connecteur.

Une transaction peut avoir été exécutée alors que l'agent, après une panne, croit devoir la recommencer.

C'est cette continuité qu'il faut désormais considérer comme une propriété de sécurité à part entière.

## Qu'est-ce qu'un processus critique dans un système agentique ?

Je propose de ne pas limiter cette notion aux seules infrastructures officiellement classées critiques.

Un processus métier peut présenter une criticité élevée sans appartenir à un secteur réglementé particulier.

Je retiendrais la définition suivante :

**Un processus agentique critique est un ensemble d'activités dans lequel une action incorrecte, non autorisée, répétée ou insuffisamment contrôlée peut provoquer une conséquence grave pour les personnes, les droits, les finances, la confidentialité, la sûreté ou la continuité d'un service essentiel.**

Cette définition repose sur la conséquence possible, et non sur la technologie employée.

Elle concerne notamment :

- Les transferts financiers, règlements et modifications de bénéficiaires.
- Les décisions ou opérations affectant directement la santé et la sécurité des personnes.
- Les commandes industrielles et les modifications de systèmes assurant des fonctions de sûreté.
- Les changements de privilèges dans des infrastructures sensibles.
- La divulgation ou la destruction de données dont la compromission aurait des conséquences graves.
- Les décisions administratives ou contractuelles affectant significativement les droits d'une personne.
- Les modifications de services essentiels, de leurs contrôles de sécurité ou de leurs mécanismes de restauration.

La criticité ne dépend pas uniquement du montant d'une transaction ou de la sensibilité d'un outil.

Un ensemble d'opérations apparemment limitées peut produire un effet critique par accumulation.

Dix opérations individuellement admissibles ne constituent pas nécessairement une opération globale autorisée.

Le contrôle doit donc porter sur la trajectoire complète du processus, ses dépendances, ses délégations et ses effets cumulés.

Autre point essentiel : **l'assistance agentique et l'engagement autonome doivent être distingués**.

Un agent peut analyser une situation critique sans avoir le droit de modifier le système concerné.

Il peut préparer une proposition d'action sans disposer des moyens techniques nécessaires pour l'exécuter.

Cette séparation permet de bénéficier des capacités de l'IA sans lui transférer automatiquement l'autorité associée aux systèmes métier.

## Trois responsabilités : les règles, les clés et les barrières

Dans les deux précédents articles consacrés à Swarm, j'ai présenté le rôle de son moteur déterministe Rust, puis les questions d'identité et de mémoire persistante.

Je voudrais maintenant élargir cette réflexion.

Une architecture de contrôle agentique devrait distinguer trois responsabilités fondamentales.

### Le gardien des règles

Il vérifie les conditions dans lesquelles une action peut être autorisée.

Il applique des politiques explicites, versionnées et indépendantes du raisonnement du modèle.

Il contrôle l'identité de l'acteur, sa délégation, les ressources concernées, les dépendances, les restrictions métier et les validations exigées.

Son fonctionnement déterministe ne garantit pas que ses règles sont correctes. Il permet en revanche de vérifier que la même règle est appliquée selon les mêmes conditions.

### Le gardien des clés

Il conserve les capacités techniques permettant de produire des effets sensibles.

Un agent chargé de préparer un paiement n'a aucune raison de posséder directement les identifiants permettant de déclencher ce paiement.

Un agent de développement n'a pas nécessairement besoin de disposer des secrets de production.

Ces capacités doivent être protégées et délivrées uniquement pour des opérations précises, sous des conditions définies.

Le contrôle des secrets n'est donc pas une simple mesure de confidentialité.

Il constitue une séparation des pouvoirs entre l'agent qui propose et le composant qui peut agir.

### Les garde-barrières

Ils appliquent effectivement les décisions à la frontière des ressources protégées.

Une politique de refus n'offre aucune garantie si l'agent peut emprunter un autre chemin d'exécution.

Les accès aux fichiers, au réseau, aux outils, aux services métier et aux opérations privilégiées doivent être médiatisés par des mécanismes que l'agent ne peut pas modifier ou désactiver avec ses propres permissions.

Ce principe rejoint celui du moniteur de référence et des points d'application des politiques du Zero Trust.

La distinction entre les trois fonctions est logique : elles peuvent partager des composants physiques si leur architecture préserve réellement leurs propriétés de sécurité.

**L'important n'est pas le nombre de services. C'est l'impossibilité de contourner les fonctions de contrôle.**

## La mémoire doit entrer dans le périmètre de sécurité

C'est probablement l'un des aspects que les architectures agentiques devront approfondir.

Une mémoire persistante n'est pas seulement un mécanisme d'optimisation du contexte.

Elle participe au comportement futur de l'agent.

Elle peut conserver des préférences, des décisions, des procédures, des résultats antérieurs ou des expériences présentées comme réussies.

Dès qu'une mémoire influence une action future, son intégrité, sa provenance et ses droits d'usage deviennent des propriétés du processus.

Dans Swarm, je cherche notamment à distinguer :

- L'identité de l'agent et les droits qui lui sont accordés.
- L'origine et la version d'une information mémorisée.
- Les conditions de lecture, de résumé et de transmission.
- Le contexte effectivement préparé pour une tentative.
- Les règles qui gouvernent l'action résultante.

Le modèle de Biba fournit ici une référence historique utile pour raisonner sur les niveaux d'intégrité.

Mais Biba, une signature cryptographique ou un contrôle de provenance ne prouvent pas qu'un souvenir est sémantiquement correct.

L'architecture doit donc préserver une frontière : une information, même authentifiée, ne peut pas créer d'autorisation par elle-même.

Le contenu peut contribuer au raisonnement.

**Il ne doit pas modifier seul les règles qui gouvernent les conséquences de ce raisonnement.**

## Proposition : PAEA, un profil d'assurance de l'exécution agentique

À partir de ces constats, je propose un premier cadre de travail :

**PAEA — Profil d'Assurance de l'Exécution Agentique.**

Son objectif n'est pas de remplacer les standards existants.

Il consiste à les articuler autour d'une propriété centrale :

**Toute action critique produite par un système agentique doit rester sans effet tant que son identité, son autorité, son contenu exact et ses conditions d'engagement n'ont pas été vérifiés par une chaîne de contrôle non contournable.**

Le framework couvre également ce qui précède l'action et ce qui lui succède.

Avant l'action : classification de criticité, identité, mandat, provenance et contexte.

Pendant l'action : contrôles d'autorisation, approbations, capacité d'exécution limitée et vérification au dernier point permettant d'empêcher l'effet.

Après l'action : résultat observé, preuve, réconciliation, supervision et reprise.

La proposition s'appuie sur des invariants vérifiables plutôt que sur une confiance déclarée envers les modèles.

Elle distingue plusieurs niveaux d'assurance proportionnés aux conséquences possibles.

Elle impose enfin une qualification fondée sur des scénarios adversariaux, des tests reproductibles et l'identification exacte de la version évaluée.

Le modèle lui-même peut changer. Ses performances peuvent progresser.

Les garanties d'autorité et d'exécution doivent rester indépendantes de ces évolutions.

## Pourquoi Swarm ?

Je positionne Swarm autour d'une idée :

**Organiser le travail des agents et rendre vérifiables les conditions dans lesquelles ils peuvent agir.**

Swarm est un moteur d'orchestration multi-agents à contrôle déterministe.

Il vise à relier chaque action à une identité, une autorité, un contenu approuvé et un résultat vérifiable, avec des mécanismes de supervision et de reprise.

Son moteur Rust porte des contrôles et des transitions qui restent distincts des propositions des modèles.

Les travaux sur la mémoire, les contrats d'identité et les actions de confiance cherchent à étendre cette séparation à des processus complets.

Swarm est toutefois un projet en évolution.

Les garanties actuellement disponibles sont établies sur des parcours précis et des versions identifiées. Les mécanismes d'identité, de mémoire et de confinement ne sont pas encore tous qualifiés de bout en bout.

Il serait prématuré de présenter Swarm comme un système certifié pour des processus critiques.

Son intérêt, dans le cadre de cette recherche, est de constituer un terrain d'expérimentation sur lequel les propriétés proposées pourront être implémentées, attaquées, mesurées et comparées à d'autres approches.

## Faut-il suspendre l'exécution autonome des processus critiques ?

Ma position est la suivante.

**Nous ne devrions pas autoriser un système agentique à engager directement des actions critiques lorsque les garanties nécessaires à leur contrôle n'ont pas été démontrées.**

Cela ne signifie pas interdire l'usage de l'IA dans les domaines sensibles.

Cela signifie distinguer ce que le système peut analyser, ce qu'il peut préparer, ce qu'il peut recommander et ce qu'il peut effectivement exécuter.

L'autonomie d'engagement devrait dépendre d'un niveau d'assurance explicite.

Pour certaines opérations, une autorisation préalable limitée et un contrôle déterministe suffiront peut-être.

Pour d'autres, il faudra une validation humaine indépendante.

Pour les fonctions de sûreté les plus sensibles, le framework ne pourra pas remplacer les exigences de certification et les dispositifs de protection propres au domaine.

Cette approche suppose d'accepter un coût supplémentaire de contrôle.

Mais l'optimisation de la vitesse d'exécution n'est pas le seul objectif d'un processus critique.

L'enjeu est de rendre l'autonomie compatible avec la responsabilité.

## De l'expérimentation à une proposition vérifiable

Le prochain travail consiste à transformer PAEA en spécification technique.

Il faudra définir les invariants, les frontières de confiance, les conditions de déploiement et les scénarios de qualification.

Puis comparer plusieurs architectures : agents disposant directement des outils, contrôle des appels d'outils, workflow déterministe et contrôle complet du cycle de vie.

Les mesures devront porter sur les effets indésirables réellement observés, les blocages injustifiés, les coûts, la latence et la capacité à reprendre correctement après incident.

Une série de tests réussis ne constituera pas une preuve universelle de sécurité.

Elle permettra néanmoins de formuler des garanties bornées, reproductibles et réfutables.

C'est ce type de démonstration qui doit accompagner le développement de l'autonomie agentique.

Nous avons consacré beaucoup d'efforts à apprendre aux agents à raisonner et à agir.

Il devient nécessaire de consacrer autant d'efforts à **définir les conditions vérifiables dans lesquelles nous acceptons de leur laisser produire des conséquences**.

La vraie question n'est pas de savoir si l'IA agentique deviendra suffisamment capable pour piloter nos processus critiques.

Elle est de savoir si nous construirons, avant de lui en confier les clés, les mécanismes qui garantissent que ces clés ne pourront pas être utilisées hors de l'autorité accordée.

*Swarm — orchestrer les agents, contrôler leur autorité, vérifier leurs effets.*

---

**Références principales**

[1] OWASP GenAI Security Project, *Top 10 for Agentic Applications*, 2026.

[2] OWASP, *Agent Control Standard*, spécification v0.1.0, 2026.

[3] NIST, *AI Risk Management Framework*, 2023, et travaux sur l'identité agentique et les infrastructures critiques, 2026.

[4] Debenedetti et al., *Defeating Prompt Injections by Design*, CaMeL, arXiv:2503.18813.

[5] Wang et al., *AgentSpec: Customizable Runtime Enforcement for Safe and Reliable LLM Agents*, arXiv:2503.18666.

[6] Das, *Adaptive Authorization for Agentic AI: Graduated and Escalated Execution Control for Critical Infrastructure*, Internet-Draft individuel, septembre 2026.

[7] Zheng et al., *ACRFence: Preventing Semantic Rollback Attacks in Agent Checkpoint-Restore*, arXiv:2603.20625.

[8] Agent Baseline, *The Secure Software Factory: A Reference Architecture*, version 1, octobre 2026.

[9] ISO/IEC 42001:2023, ISO/IEC 23894:2023 ; NIST SP 800-207 ; SPIFFE ; RFC 8693 et RFC 9396.

[10] ANSSI, *Recommandations de sécurité pour un système d'IA générative*, avril 2024.

[11] Documentation d'architecture et contrats de Swarm, état d'octobre 2026.