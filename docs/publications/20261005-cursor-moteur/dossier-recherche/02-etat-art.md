# 2. État de l'art

Quatre familles de travaux encadrent la question : l'orchestration d'agents et l'analyse de ses
défaillances, le contrôle des actions à l'exécution, la reprise et les effets irréversibles, et
les agents appliqués aux processus financiers. Pour chacune, nous retenons ce qu'elle apporte et
ce qu'elle laisse ouvert. Les travaux cités ont été lus en texte intégral ou au niveau du résumé ;
cette distinction est consignée dans les fiches de lecture du dépôt.

## 2.1 Orchestration d'agents et défaillances

Les rapports de Cursor décrivent l'organisation la plus aboutie publiquement : des planificateurs
et sous-planificateurs répartissent le travail entre des exécutants à périmètre limité, reliés
par des passations structurées [Lin 2026a]. Le retrait de l'intégrateur central et l'acceptation
d'un taux d'erreur non nul y sont des choix d'ingénierie explicites. La version suivante consigne
les décisions dans des documents de conception partagés et superpose des revues décorrélées, en
discutant le coût selon le mélange de modèles [Lin 2026b]. Anthropic rapporte, pour un système
de recherche multi-agents, une consommation de l'ordre de quinze fois celle d'une conversation et
recommande des protections déterministes comme la reprise et les points de sauvegarde
[Hadfield 2025]. Ces textes sont des rapports d'ingénierie, non évalués par des pairs ; ils
documentent surtout des travaux réversibles.

Du côté de l'analyse, Cemri et al. étudient 1 642 traces de systèmes multi-agents et proposent la
taxonomie MAST : quatorze modes de défaillance répartis entre conception du système, désalignement
entre agents et vérification des tâches, avec un accord inter-annotateurs de κ = 0,88
[Cemri 2025]. La vérification absente, incomplète ou incorrecte y figure en bonne place. MAST ne
contient en revanche aucune catégorie pour les défauts de l'orchestrateur lui-même : un moteur qui
applique une règle erronée, ou une règle juste mais mal calibrée, n'y a pas de place. La question
d'attribution QR3 part de ce manque.

## 2.2 Contrôle des actions à l'exécution

Une première famille interpose une décision entre l'agent et ses outils. AgentSpec fournit un
langage de règles à déclencheurs et prédicats, appliquées à l'exécution avec un surcoût de l'ordre
de la milliseconde [Wang 2026a]. AgenticRei exprime des politiques déontiques (obligations,
dispenses, résolution de conflits) évaluées par un moteur logique extérieur au modèle
[Joshi 2026]. Un point d'application pour agents MCP combine règles déclaratives, étiquettes de
flux propagées entre étapes et journal chaîné [Wang 2026b]. Toutes ces approches décident à la
frontière de l'appel d'outil ; notre condition de comparaison B1 en est une instance minimale.

Plusieurs travaux montrent les limites d'une décision appel par appel. Kaptein et al. formalisent
une politique comme fonction de l'identité de l'agent, du chemin d'exécution partiel, de l'action
proposée et d'un état partagé, et montrent que l'évaluation à l'exécution devient nécessaire dès
qu'une politique dépend du chemin [Kaptein 2026]. Kurady et al. décrivent des violations
compositionnelles, comme le fractionnement d'un montant sous un seuil, invisibles à un contrôle
étape par étape [Kurady 2026]. Ray étudie ce qu'une garde peut garantir avant un appel
irréversible et souligne que le blocage modifie lui-même la suite de l'exécution [Ray 2026].

Une seconde famille sépare par conception ce que le modèle décide de ce qu'il exécute. CaMeL
extrait les flux de contrôle et de données de la requête de confiance, de sorte que des données
non fiables ne modifient jamais le flux du programme ; il résout 77 % des tâches d'AgentDojo avec
une sécurité prouvable, contre 84 % sans défense [Debenedetti 2025]. Beurer-Kellner et al.
recensent six motifs de conception, dont « planifier puis exécuter », où le modèle formule une
liste fixe d'actions exécutée ensuite [Beurer-Kellner 2025]. Notre séparation entre la préparation
d'un lot et son règlement déterministe relève de ce motif. Ces travaux visent l'injection de
consignes ; ils ne traitent ni la reprise après incident, ni le lien entre une validation et le
candidat effectivement exécuté.

## 2.3 Reprise, rejeu et effets irréversibles

Les techniques classiques de durabilité reposent sur la compensation [Garcia-Molina 1987] et,
pour les paiements, sur des clés d'idempotence fournies par l'appelant. Zheng et al. montrent que
ces protections supposent que l'appelant renvoie une requête identique lors d'un rejeu, hypothèse
que les agents violent : un agent relancé reformule. Ils proposent ACRFence, qui journalise les
effets irréversibles et fait juger par un modèle l'équivalence des requêtes rejouées
[Zheng 2026]. Wu et al. caractérisent cinq modes de défaillance de la reprise par point de
sauvegarde, dont des effets produits hors de la frontière de reprise et dont la trace disparaît au
retour arrière, sans trouver de mécanisme général pour y remédier [Wu 2026].

Notre travail se place dans cette lignée avec une hypothèse différente. Plutôt que de reconnaître
après coup qu'une requête générée équivaut à une autre, nous retirons au modèle l'émission de
l'effet. La clé d'idempotence est calculée par un composant déterministe à partir de l'identité
métier de l'intention, et l'exécution n'est autorisée que sur le candidat dont l'empreinte a été
validée.

## 2.4 Agents et processus financiers

Huang et al. mesurent la fidélité de trajectoire de systèmes de paiement multi-agents sur 18
modèles et 90 000 tâches : dix modèles sautent systématiquement un point de confirmation, défaut
invisible aux métriques de succès, et des gardes de routage déterministes améliorent fortement les
résultats [Huang 2026]. Han met en évidence un écart de vérifiabilité dans la gouvernance des
agents financiers et propose une délégation conditionnée à la conservation des preuves [Han 2026].
RAILS formalise la compensation entre agents par sept primitives et un seuil minimal
d'admissibilité des preuves avant tout règlement [de Valois-Franklin 2026] ; le travail est
théorique et ne traite ni l'idempotence ni la reprise. Il est complémentaire du nôtre : RAILS
décide quel règlement doit suivre une obligation ; nous garantissons qu'un règlement décidé
s'exécute une seule fois et sur le lot validé.

Pour l'évaluation, τ-bench compare l'état final d'une base à l'état attendu et introduit la
métrique pass^k, qui mesure la régularité d'un agent sur k essais [Yao 2024]. Nous en reprenons
les deux principes : mesurer dans le grand livre et non dans le discours des agents, et rapporter
la régularité des agents réels.

## 2.5 Positionnement

Le tableau 1 résume le périmètre des travaux les plus proches, d'après les textes lus ; il ne
qualifie pas leur efficacité, qui relève de leurs propres évaluations.

**Tableau 1 — Périmètre des travaux les plus proches.**

| Travail | Ce qui est contrôlé | Nature de la décision | Reprise et effets irréversibles | Évaluation |
|---|---|---|---|---|
| AgentSpec [Wang 2026a] | Appel d'outil | Règles déterministes | Non traité dans le résumé | Agents de code, incarnés, conduite |
| Policies on Paths [Kaptein 2026] | Action au vu du chemin | Fonction de politique | Non traité | Aucune mesure |
| CaMeL [Debenedetti 2025] | Flux de contrôle et de données | Interpréteur et capacités | Non traité | AgentDojo |
| ACRFence [Zheng 2026] | Appels rejoués après restauration | Juge LLM d'équivalence | Oui | 10 essais, services simulés |
| Safe to Resume? [Wu 2026] | Aucun (analyse et attaques) | — | Oui | 5 cadres, 1 735 exécutions |
| RAILS [de Valois-Franklin 2026] | Admissibilité des preuves avant règlement | Modèle formel | Hors périmètre | Analyse théorique |
| Workflow fidelity [Huang 2026] | Trajectoire d'exécution | Métrique ; gardes déterministes | Non | 18 modèles, 90 000 tâches |
| **Ce travail** | Cycle de vie de la tentative et exécution de l'effet | Moteur et exécutant déterministes | Oui | Banc synthétique, 8 types de fautes, 9 000 exécutions scriptées et 90 avec agent réel |

Le manque que ce travail vise est donc précis : aucun des travaux recensés ne combine la
médiation du cycle de vie des tentatives (lancement, reprise, acceptation), le lien entre la
preuve et le candidat exécuté, et une évaluation sous fautes ordinaires avec et sans médiation.
