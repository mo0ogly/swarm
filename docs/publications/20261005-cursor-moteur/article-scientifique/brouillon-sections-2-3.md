# Brouillon — sections 2 et 3

Statut : brouillon du 5 octobre 2026. Chaque fait cité provient de
`fiches-lecture.md` (niveau de lecture indiqué). Les passages entre crochets
`[À AJUSTER]` dépendent de résultats qui n'existent pas encore et doivent être
réécrits après la campagne de mesures.

---

## 2. Introduction

Un règlement envoie un virement, la banque l'exécute, et la réponse se perd.
Le règlement réessaie. Selon la manière dont la demande est identifiée, la
banque reconnaît la répétition ou paie une seconde fois. Ce scénario est ancien
et les systèmes de paiement y répondent par des clés d'idempotence. Il change de
nature lorsque la demande n'est plus émise par un programme déterministe mais
par un agent fondé sur un grand modèle de langage (LLM).

Ces agents quittent les tâches de démonstration pour des processus où une
action a un effet irréversible. Des systèmes multi-agents sont évalués sur des
flux de paiement [Huang 2026], et des institutions financières confient des
décisions à des systèmes autonomes dont la vérifiabilité reste insuffisante
[Han 2026]. Dans le même temps, l'orchestration change d'échelle : Cursor décrit
des essaims où des centaines d'exécutants travaillent sous des planificateurs
hiérarchiques [Lin 2026a, 2026b], et Anthropic rapporte qu'un système
multi-agents consomme environ quinze fois plus de jetons qu'une conversation,
avec des reprises sur erreur et des points de sauvegarde réguliers [Hadfield 2025].

Deux résultats récents montrent que cette combinaison est fragile. Lorsqu'un
cadre d'agents restaure un point de sauvegarde après un incident, l'agent
réémet une requête légèrement différente de l'originale ; les protections qui
supposent des requêtes identiques au rejeu ne la reconnaissent plus, et un
paiement déjà exécuté est répété dans chacun des dix essais avec reprise
[Zheng 2026]. Une étude de la reprise sur cinq cadres trouve que la quasi-totalité
des workflows à effets externes échouent à reprendre correctement (93,8 % ou
96,9 % selon l'ordre entre exécution et enregistrement) et conclut à l'absence
de mécanisme général [Wu 2026]. Enfin, un point de confirmation laissé à la
discrétion du modèle n'est pas fiable : dans un système de paiement, dix
modèles sur dix-huit le sautent systématiquement [Huang 2026].

Les travaux d'application des règles à l'exécution contrôlent principalement
l'appel d'outil, au moment où l'agent le formule [Wang 2026a ; Joshi 2026 ;
Wang 2026b]. Or une suite d'étapes individuellement conformes peut violer une
règle portant sur l'exécution entière [Kurady 2026], et les défaillances
observées dans la reprise ne tiennent pas à un appel isolé mais au cycle de vie
de la tentative : lancement, interruption, reprise, acceptation du résultat.
La réponse publiée la plus proche, ACRFence, interpose un journal des effets et
confie à un second LLM le soin de juger si la requête rejouée équivaut à
l'originale [Zheng 2026]. La décision qui protège l'effet irréversible reste
donc probabiliste.

Nous étudions une réponse déterministe par conception. L'agent propose ; il ne
détient aucun droit sur l'effet. Un moteur déterministe médiatise le lancement,
la reprise et l'acceptation de chaque tentative, lie toute validation au
candidat exact qu'elle a examiné, et confie l'effet à un composant d'exécution
non LLM dont la clé d'idempotence dérive de l'identité métier (fournisseur,
facture) plutôt que de la requête générée. La question n'est pas seulement de
savoir si ces contrôles empêchent les effets doublés ou faux, mais aussi ce
qu'ils coûtent : Cursor rapporte qu'exiger une correction complète avant chaque
intégration sérialisait le travail [Lin 2026a], et un moteur trop strict peut
bloquer un travail valide.

Nous posons trois questions. **QR1** : quels contrôles d'exécution empêchent les
effets doublés, les acceptations sans preuve valide et les reprises sans cause
corrigée ? **QR2** : quel surcoût imposent-ils en durée, en consommation et en
blocages à tort ? **QR3** : peut-on attribuer automatiquement un échec à l'agent,
à l'environnement ou au moteur lui-même ?

Nos contributions sont les suivantes.

- **C1.** Un modèle de médiation du cycle de vie des tentatives d'agents,
  exprimé par huit invariants (au plus un effet par intention, absence de
  mutation sur état périmé, acceptation liée à une preuve fraîche du candidat,
  entre autres), chacun relié aux tests qui l'exercent.
- **C2.** Une implémentation ouverte, Swarm, moteur déterministe écrit en Go
  sur une base SQLite embarquée.
- **C3.** Un banc de facturation simulé, sur données synthétiques, qui injecte
  huit fautes reproductibles et compare trois conditions : chaîne sans moteur
  avec revue finale, contrôle par appel, moteur complet ; chaque faute est
  croisée avec trois choix de clé d'idempotence, et chaque injection est
  d'abord validée par un contrôle positif dans la condition non protégée.
  [À AJUSTER : résumé des résultats QR1 et QR2.]
- **C4.** Une extension de la taxonomie MAST [Cemri 2025] aux défaillances du
  moteur et des règles, et une mesure de la précision de l'attribution
  automatique. [À AJUSTER : résultats QR3.]

Cet article ne revendique ni un gain général de coût, ni une preuve formelle
complète des invariants, ni une validation sur un système bancaire réel.

---

## 3. Contexte et travaux liés

### 3.1 Orchestration multi-agents et ses défaillances

Les rapports d'ingénierie de Cursor décrivent une organisation en planificateurs,
sous-planificateurs et exécutants à périmètre limité, reliés par des passations
structurées [Lin 2026a]. Deux enseignements nous concernent directement : un
intégrateur central a été retiré parce qu'il devenait un goulot pour des
centaines d'exécutants, et exiger une correction complète avant chaque commit
sérialisait le travail ; le système accepte donc un taux d'erreur faible mais
non nul, en comptant sur des corrections ultérieures [Lin 2026a]. La version
suivante consigne les décisions dans des documents de conception partagés et
superpose des revues décorrélées [Lin 2026b]. Ces choix conviennent à du code,
dont l'erreur se corrige au commit suivant ; ils ne se transposent pas tels quels
à un effet financier. Ce sont par ailleurs des rapports d'ingénierie, non
évalués par des pairs.

Cemri et al. analysent 1 642 traces de systèmes multi-agents et proposent la
taxonomie MAST : quatorze modes de défaillance répartis entre conception du
système, désalignement entre agents et vérification des tâches, avec un accord
inter-annotateurs κ = 0,88 [Cemri 2025]. La vérification absente, incomplète
ou incorrecte y figure parmi les modes de défaillance identifiés. MAST ne comporte aucune catégorie pour les défauts de
l'orchestrateur lui-même : un moteur qui applique une règle erronée, ou une
règle correcte mais mal calibrée, n'y a pas de place. Notre question QR3 part
de ce manque.

### 3.2 Application des règles à l'exécution

Une première famille de travaux interpose un contrôle entre l'agent et ses
outils. AgentSpec fournit un langage dédié de règles à déclencheurs et
prédicats, appliquées à l'exécution avec un surcoût de l'ordre de la
milliseconde [Wang 2026a]. AgenticRei exprime des politiques déontiques
(obligations, dispenses, résolution de conflits) évaluées par un moteur logique
externe au LLM, pour les appels d'outils comme pour les échanges entre agents
[Joshi 2026]. Un point d'application pour agents MCP combine règles
déclaratives, étiquettes de flux propagées entre étapes et journal d'audit
chaîné [Wang 2026b]. Ces approches décident à la frontière de l'appel d'outil ;
notre condition de comparaison B1 en est une instance minimale.

Plusieurs travaux montrent les limites d'une décision appel par appel. Kaptein
et al. formalisent une politique comme une fonction de l'identité de l'agent,
du chemin d'exécution partiel, de l'action proposée et d'un état partagé, et
montrent que l'évaluation à l'exécution est nécessaire dès qu'une politique
dépend du chemin [Kaptein 2026] ; leur implémentation n'est pas évaluée
empiriquement, et ils citent parmi les défis ouverts la complétude de
l'interception. Kurady et al. décrivent des violations compositionnelles, dont
le contournement de seuil par fractionnement et le dépassement de somme
cumulée, invisibles à un contrôle par étape [Kurady 2026]. Ray étudie ce qu'une
garde peut garantir avant un appel irréversible et souligne que le blocage
modifie lui-même la suite de l'exécution [Ray 2026] ; nous mesurons ce coût sous
la forme des blocages à tort.

Une seconde famille sépare par conception ce que le modèle décide de ce qu'il
exécute. CaMeL extrait les flux de contrôle et de données de la requête de
confiance, de sorte que les données non fiables ne modifient jamais le flux du
programme, et applique des capacités lors des appels d'outils ; il résout 77 %
des tâches d'AgentDojo avec une sécurité prouvable, contre 84 % sans défense
[Debenedetti 2025]. Beurer-Kellner et al. recensent six motifs de conception,
dont Plan-Then-Execute, où le modèle formule une liste fixe d'actions exécutée
ensuite [Beurer-Kellner 2025]. Notre séparation entre préparation d'un lot et
règlement déterministe relève de ce motif. Ces travaux visent l'injection de
prompt ; ils ne traitent ni la reprise après incident, ni le lien entre une
validation et le candidat exécuté.

### 3.3 Reprise, rejeu et effets irréversibles

Les techniques classiques de durabilité reposent sur la compensation [Garcia-Molina
1987] et, pour les paiements, sur des clés d'idempotence fournies par l'appelant.
Zheng et al. montrent que ces protections supposent que l'appelant envoie une
requête identique au rejeu, hypothèse que les agents LLM violent ; ils
identifient deux attaques, le rejeu d'action après un crash provoqué et la
résurrection d'une autorisation à usage unique, et proposent ACRFence, un proxy
qui journalise les effets irréversibles et fait juger l'équivalence des
requêtes rejouées par un LLM [Zheng 2026]. Wu et al. caractérisent cinq modes de
défaillance de la reprise par point de sauvegarde, dont des effets produits
hors de la frontière de reprise et dont la trace disparaît au retour arrière ;
ils les observent sur cinq cadres et ne trouvent pas de mécanisme général pour
y remédier [Wu 2026].

Notre travail se place dans cette lignée avec une hypothèse différente : plutôt
que de reconnaître après coup qu'une requête générée équivaut à une autre, nous
retirons au modèle l'émission de l'effet. La clé d'idempotence est alors
calculée par un composant déterministe à partir de l'identité métier, et
l'exécution n'est autorisée que sur le candidat dont l'empreinte a été validée.

### 3.4 Agents et processus financiers

Huang et al. mesurent la fidélité de trajectoire de systèmes multi-agents de
paiement sur 18 modèles et 90 000 tâches : dix modèles sautent
systématiquement un point de confirmation, défaut invisible aux métriques de
succès, et des gardes de routage déterministes améliorent fortement les
résultats [Huang 2026]. Han identifie un écart de vérifiabilité dans la
gouvernance des agents financiers, montre que les mises à jour de modèles par
les fournisseurs modifient des décisions passées, et propose une délégation
conditionnée à la conservation des preuves [Han 2026]. RAILS formalise le
problème de la compensation agentique par sept primitives et un seuil minimal
d'admissibilité des preuves avant tout règlement [de Valois-Franklin 2026] ;
le travail est théorique et ne traite ni l'idempotence ni la reprise. Il est
complémentaire du nôtre : RAILS décide quel règlement doit suivre une
obligation, nous garantissons qu'un règlement décidé s'exécute une seule fois
et sur le lot validé.

Pour l'évaluation, τ-bench compare l'état final d'une base à l'état attendu et
introduit la métrique pass^k, qui mesure la régularité d'un agent sur k essais
[Yao 2024]. Nous reprenons ces deux principes : mesurer dans le grand livre et
non dans le discours des agents, et rapporter la régularité des agents réels.

### 3.5 Positionnement

Le tableau 1 résume le périmètre des travaux les plus proches, d'après les
textes lus. Il ne qualifie pas leur efficacité, qui relève de leurs propres
évaluations.

**Tableau 1 — Périmètre des travaux les plus proches.**

| Travail | Ce qui est contrôlé | Nature de la décision | Reprise et effets irréversibles | Évaluation |
| --- | --- | --- | --- | --- |
| AgentSpec [Wang 2026a] | Appel d'outil | Règles déterministes | Non traité dans le résumé | Agents de code, incarnés, conduite |
| Policies on Paths [Kaptein 2026] | Action proposée au vu du chemin | Fonction de politique, scores non étalonnés | Non traité | Aucune mesure |
| CaMeL [Debenedetti 2025] | Flux de contrôle et de données | Interpréteur et capacités | Non traité | AgentDojo |
| ACRFence [Zheng 2026] | Appels rejoués après restauration | Juge LLM d'équivalence | Oui | 10 essais, services simulés |
| Safe to Resume? [Wu 2026] | Aucun (analyse et attaques) | — | Oui | 5 cadres, 1 735 exécutions |
| RAILS [de Valois-Franklin 2026] | Admissibilité des preuves avant règlement | Modèle formel | Non (hors périmètre) | Analyse théorique |
| Workflow fidelity [Huang 2026] | Trajectoire d'exécution | Métrique ; gardes déterministes | Non | 18 modèles, 90 000 tâches |
| **Ce travail** | Cycle de vie de la tentative et exécution de l'effet | Moteur et règlement déterministes | Oui | Banc synthétique, 8 fautes [À AJUSTER] |

---

## Références (à mettre au format de la venue)

- [Beurer-Kellner 2025] L. Beurer-Kellner et al. *Design Patterns for Securing LLM Agents against Prompt Injections*. arXiv:2506.08837, 2025.
- [Cemri 2025] M. Cemri et al. *Why Do Multi-Agent LLM Systems Fail?* arXiv:2503.13657, 2025.
- [Debenedetti 2025] E. Debenedetti et al. *Defeating Prompt Injections by Design*. arXiv:2503.18813, 2025.
- [de Valois-Franklin 2026] A. de Valois-Franklin, A. Bogdan. *RAILS: Verification-Native Clearing For Agentic Commerce*. arXiv:2606.08790, 2026.
- [Garcia-Molina 1987] H. Garcia-Molina, K. Salem. *Sagas*. SIGMOD 1987. (Référence complète à vérifier.)
- [Hadfield 2025] J. Hadfield et al. *How we built our multi-agent research system*. Anthropic, 13 juin 2025.
- [Han 2026] H. Han. *Governing Agentic AI in FinTech*. arXiv:2608.11344, 2026.
- [Huang 2026] D. Huang, J. K. Chua, Z. Wang. *Beyond Task Success: Measuring Workflow Fidelity in LLM-Based Agentic Payment Systems*. AIDS4DF, PAKDD 2026. arXiv:2605.06457.
- [Joshi 2026] A. Joshi, T. Finin, K. P. Joshi, L. Kagal. *Deontic Policies for Runtime Governance of Agentic AI Systems*. IEEE ICWS 2026. arXiv:2606.19464.
- [Kaptein 2026] M. Kaptein, V.-J. Khan, A. Podstavnychy. *Runtime Governance for AI Agents: Policies on Paths*. arXiv:2603.16586, 2026.
- [Kurady 2026] A. Kurady, S. S. C. Grandhi, R. Gupta, S. Kumar. *Compositional Policy Violations: When Step-Level Compliance Fails In Agentic AI Workflows*. arXiv:2609.18820, 2026.
- [Lin 2026a] W. Lin. *Towards self-driving codebases*. Cursor, 5 février 2026.
- [Lin 2026b] W. Lin. *Agent swarms and the new model economics*. Cursor, 20 juillet 2026.
- [Ray 2026] S. Ray. *What Can Be Enforced? A Theory of Certified Runtime Safety for Tool-Using Agents*. arXiv:2607.22868, 2026.
- [Wang 2026a] H. Wang, C. M. Poskitt, J. Sun. *AgentSpec: Customizable Runtime Enforcement for Safe and Reliable LLM Agents*. ICSE 2026, p. 2938-2950. arXiv:2503.18666.
- [Wang 2026b] S. Wang. *Runtime Policy Enforcement for MCP-Based LLM Agents*. Electronics 15(13):2829, 2026. (Métadonnées issues d'un index ; à vérifier sur la page de l'éditeur.)
- [Wu 2026] G. Wu, D. Li, K. Jiang, J. Niu, C. Wang, Y. Zhang. *Safe to Resume? Breaking Execution Continuity of Agent Execution via Rollback*. arXiv:2608.29381, 2026.
- [Yao 2024] S. Yao, N. Shinn, P. Razavi, K. Narasimhan. *τ-bench: A Benchmark for Tool-Agent-User Interaction in Real-World Domains*. arXiv:2406.12045, 2024.
- [Zheng 2026] Y. Zheng, Y. Yang, W. Zhang, A. Quinn. *ACRFence: Preventing Semantic Rollback Attacks in Agent Checkpoint-Restore*. CoDAIM 2026. arXiv:2603.20625.

## Points à vérifier avant soumission

1. MAST : les pourcentages par mode (FM-3.2 8,2 %, FM-3.3 9,1 %) n'ont pas été repris dans le texte, car leur base (part des traces ou des défaillances) reste à vérifier. Les ajouter seulement après vérification.
2. « des centaines d'exécutants » : citation de Lin 2026a (« hundreds of workers »), relue.
3. AgentSpec, AgenticRei, CaMeL, Kurady, Ray, Han, Huang, τ-bench : lus au niveau du résumé ; aucune phrase ci-dessus ne va au-delà du résumé, à confirmer par lecture du texte.
4. Wang 2026b : page éditeur inaccessible (403) ; vérifier auteurs et contenu.
5. Swarm comme « implémentation ouverte » : vérifier que le dépôt public correspond au code mesuré.
6. Le nombre d'invariants (huit) suit le plan actuel ; à figer avec la section 4.
