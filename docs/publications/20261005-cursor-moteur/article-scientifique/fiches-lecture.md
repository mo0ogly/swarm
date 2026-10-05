# Fiches de lecture — travaux liés

Relevé du 5 octobre 2026. Niveau de lecture indiqué pour chaque fiche :

- **résumé** : métadonnées et résumé lus sur la page primaire (arXiv, éditeur, blog) ;
- **texte** : passages du texte intégral consultés, citations reprises mot pour mot ;
- **indirect** : page primaire inaccessible, informations tirées d'un index (à revérifier avant citation).

Aucune fiche « résumé » ne suffit pour affirmer un détail de mécanisme ou un
chiffre hors résumé : lire le texte avant de l'écrire dans l'article.

---

## 1. Plus proche de notre contribution

### ACRFence — reprise après checkpoint et effets irréversibles
- Zheng, Yang, Zhang, Quinn. *ACRFence: Preventing Semantic Rollback Attacks in Agent Checkpoint-Restore*. arXiv:2603.20625, 21 mars 2026, atelier CoDAIM 2026. [arXiv](https://arxiv.org/abs/2603.20625) — **texte**.
- Constat : les cadres d'agents conseillent de rendre les appels d'outils « sûrs à réessayer », mais « All existing protection mechanisms...share the same assumption: *the caller will send identical requests on retry*. LLM agents violate this assumption. »
- Menaces : *Action Replay* (un crash provoqué après un paiement réussi fait réémettre le paiement « with a fresh reference ID ») et *Authority Resurrection* (réutilisation d'une approbation à usage unique après retour arrière).
- Mécanisme : proxy MCP + journal d'effets des appels irréversibles (eBPF) ; après restauration, « a lightweight *analyzer LLM* compares the new tool call against the logged entry » → rejeu de la réponse enregistrée si équivalent, blocage si différent.
- Évaluation : Claude Code CLI avec Qwen3-32B, services simulés ; Action Replay : 10/10 essais avec checkpoint produisent un doublon, 0/10 sans checkpoint ; 12 cadres recensés comme exposés.
- **Position par rapport à nous** : même problème que notre faute F3/F1 (crash ou relance après un paiement réussi). Différence de conception : ACRFence juge l'équivalence par un LLM, donc de façon non déterministe ; notre approche retire au LLM tout droit de paiement et confie l'effet à un règlement déterministe dont la clé dérive de l'identité métier (fournisseur, facture), pas de la requête générée. À citer en premier ; notre banc doit montrer ce que cette différence change, sans le présumer.

### Safe to Resume? — continuité d'exécution et retour arrière
- Wu, Li, Jiang, Niu, Wang, Zhang. *Safe to Resume? Breaking Execution Continuity of Agent Execution via Rollback*. arXiv:2608.29381, 29 août 2026, cs.CR. [arXiv](https://arxiv.org/abs/2608.29381) — **texte**.
- Cinq modes (SF1 état interne incomplet ; SF2 état incohérent ; SF3 décalage avec l'état externe ; SF4 rejeu non déterministe non lié ; SF5 « action produces effect outside recovery boundary, but rollback removes internal record that action occurred »).
- Mesures : 347 traces, 1 735 exécutions sur 5 cadres ; SF1 dans 67,2 % des exécutions, SF4 dans 67,7 %, SF5 dans 2,3 % ; micro-banc d'effets externes : 90/96 (93,8 %) workflows en échec dans l'ordre exécuter→enregistrer, 93/96 (96,9 %) dans l'ordre enregistrer→exécuter.
- Défenses : aucune proposée ; « we find no general mechanism » ; les services hétérogènes « may not expose such coordination interfaces » (transactions, idempotence).
- **Position** : SF5 est exactement notre F3 (crash entre paiement et enregistrement). Ils constatent l'absence de mécanisme général ; nous proposons et mesurons un mécanisme (règlement déterministe, clé métier, progression liée à l'empreinte). Leurs taux d'échec du micro-banc sont la référence à citer en motivation.

### Workflow fidelity dans les systèmes de paiement agentiques
- Huang, Chua, Wang. *Beyond Task Success: Measuring Workflow Fidelity in LLM-Based Agentic Payment Systems*. arXiv:2605.06457, 7 mai 2026, atelier AIDS4DF, PAKDD 2026. [arXiv](https://arxiv.org/abs/2605.06457) — **résumé**.
- Métrique ASR au niveau des transitions ; sur 18 LLM et 90 000 tâches, « 10 of 18 models systematically skip a confirmation checkpoint during payment checkout », invisible aux métriques de succès ; des « deterministic routing guards » améliorent le succès jusqu'à +93,8 points.
- **Position** : preuve empirique qu'un point de contrôle laissé au modèle est sauté ; argument direct pour un contrôle imposé par le moteur. Leur métrique de trajectoire peut compléter nos mesures sur le grand livre.

### RAILS — compensation vérifiable pour le commerce agentique
- de Valois-Franklin, Bogdan. *RAILS: Verification-Native Clearing For Agentic Commerce*. arXiv:2606.08790, 7 juin 2026. [arXiv](https://arxiv.org/abs/2606.08790) — **texte**.
- Sept primitives : Obligation Object, Evidence Envelope (« hash-anchored container of evidence items »), Verification Mesh, Clearing Decision, Settlement Instruction, Clearing Passport, Finality Rules (passage PROVISIONAL → FINAL).
- Seuil : classes de preuve ordonnées SELF ⪯ SIGN ⪯ {WIT, REC} ⪯ ATT ⪯ PROOF ; un règlement exige cls(B) ⪰ φ_O.
- Évaluation : analyse théorique seulement, aucune implémentation ni mesure. Idempotence, double paiement et reprise après crash : absents.
- **Position** : complémentaire. RAILS décide si une obligation est remplie et quel règlement suit ; nous garantissons qu'un règlement décidé s'exécute une seule fois et sur le lot validé. L'enveloppe de preuve ancrée par empreinte rejoint notre invariant I3.

### Governing Agentic AI in FinTech
- Han. *Governing Agentic AI in FinTech*. arXiv:2608.11344, 11 août 2026 (v3 15 sept. 2026), cs.CY. [arXiv](https://arxiv.org/abs/2608.11344) — **résumé**.
- « Verifiability Gap » ; les mises à jour de modèles fournisseurs modifient des actions financières passées ; propose une « evidence-contingent delegation ».
- **Position** : justifie le gel des versions de modèles et la conservation des preuves dans notre protocole ; cadre de discussion réglementaire.

---

## 2. Application des règles à l'exécution

### AgentSpec
- Wang, Poskitt, Sun. *AgentSpec: Customizable Runtime Enforcement for Safe and Reliable LLM Agents*. arXiv:2503.18666, Proc. ICSE'26, p. 2938-2950. [arXiv](https://arxiv.org/abs/2503.18666) — **résumé**.
- Langage dédié de contraintes d'exécution ; plus de 90 % d'exécutions dangereuses évitées pour les agents de code, surcoût de l'ordre de la milliseconde.
- **Position** : règle évaluée appel par appel ; correspond à notre condition B1 généralisée.

### AgenticRei — politiques déontiques
- Joshi, Finin, Joshi, Kagal. *Deontic Policies for Runtime Governance of Agentic AI Systems*. arXiv:2606.19464, 17 juin 2026, IEEE ICWS 2026. [arXiv](https://arxiv.org/abs/2606.19464) — **résumé**.
- Obligations, dispenses, résolution de conflits ; moteur logique externe au LLM, appliqué aux appels d'outils et aux échanges entre agents.
- **Position** : expressivité des règles ; ne traite pas, d'après le résumé, la reprise ni la fraîcheur des preuves.

### Point d'application pour agents MCP
- Wang (Shanshan). *Runtime Policy Enforcement for MCP-Based LLM Agents*. Electronics 15(13):2829, juin 2026. [MDPI](https://www.mdpi.com/2079-9292/15/13/2829) — **indirect** (page éditeur refusée en 403 ; informations issues de l'index de recherche, à revérifier).
- Interception à la frontière d'appel d'outil, étiquettes de flux entre étapes, journal d'audit chaîné SHA-256.
- **Position** : condition B1 + traçabilité ; le journal chaîné est une piste pour notre non-répudiation.

### Policies on Paths
- Kaptein, Khan, Podstavnychy. *Runtime Governance for AI Agents: Policies on Paths*. arXiv:2603.16586, 17 mars 2026. [arXiv](https://arxiv.org/abs/2603.16586) — **texte**.
- Politique πj(A, Pi, s*, Σ) → [0,1] : identité, chemin partiel, action proposée, état partagé ; sortie = probabilité de violation (texte). Exemples : intégrité d'agent, exfiltration, barrière informationnelle, seuil d'exécution.
- Implémentation de référence (Kyvvu) sans évaluation chiffrée : « policy scores [...] have not been empirically calibrated ». Défis ouverts : calibrage, contournement, complétude de l'interception, dérive, provenance de délégation, code généré.
- **Position** : cadre formel qui englobe notre graphe de dépendances ; leur défi « complétude de l'interception » est ce que notre séparation préparation / règlement traite par conception pour les paiements. Pas de mesure chez eux : notre banc apporte des chiffres.

### Compositional Policy Violations
- Kurady, Grandhi, Gupta, Kumar. arXiv:2609.18820, 16 sept. 2026. [arXiv](https://arxiv.org/abs/2609.18820) — **résumé**.
- Quatre catégories (Authority Creep, Threshold Laundering, Cumulative Sum Violation, Context Collapse) ; runtime tenant compte de la provenance sur toute la trace.
- **Position** : « Cumulative Sum Violation » et « Threshold Laundering » correspondent à des contrôles de lot (somme, plafond) que notre `check_lot` applique au candidat entier.

### What Can Be Enforced?
- Ray. *What Can Be Enforced? A Theory of Certified Runtime Safety for Tool-Using Agents*. arXiv:2607.22868, 24 juillet 2026. [arXiv](https://arxiv.org/abs/2607.22868) — **résumé**.
- « Runtime guardrails act before irreversible tool calls » ; garanties dépendant de l'état représentable, de l'observation du juge et de l'effet du blocage sur la suite.
- **Position** : cadre théorique pour la section modèle ; le point « blocking behavior affects closed-loop dynamics » rejoint notre mesure des blocages à tort.

### CaMeL
- Debenedetti, Shumailov, Fan, Hayes, Carlini, Fabian, Kern, Shi, Terzis, Tramèr. *Defeating Prompt Injections by Design*. arXiv:2503.18813, 2025. [arXiv](https://arxiv.org/abs/2503.18813) — **résumé**.
- Extraction des flux de contrôle et de données depuis la requête de confiance ; capacités appliquées à l'appel d'outil ; 77 % des tâches AgentDojo résolues avec sécurité prouvable contre 84 % sans défense.
- **Position** : même principe que notre invariant I7 (aucun texte de modèle interprété comme commande) et que la séparation préparation / règlement.

### Design Patterns for Securing LLM Agents
- Beurer-Kellner et al. arXiv:2506.08837, juin 2025. [arXiv](https://arxiv.org/abs/2506.08837) — **texte**.
- Six motifs (texte) : Action-Selector, Plan-Then-Execute (« formulate a plan to be executed (i.e., a fixed list of actions to take) »), LLM Map-Reduce, Dual LLM, Code-Then-Execute, Context-Minimization. Pas de traitement spécifique des actions irréversibles ou financières.
- **Position** (notre lecture) : le lot de paiements proposé par l'agent puis exécuté par un règlement déterministe est une instance de Plan-Then-Execute, appliquée à un effet financier. Notre apport : ce que ce motif ne dit pas, à savoir lier l'exécution au plan validé (empreinte) et la rendre sûre au rejeu (clé métier).

---

## 3. Orchestration et défaillances multi-agents

### MAST
- Cemri et al. *Why Do Multi-Agent LLM Systems Fail?* arXiv:2503.13657, 2025. [arXiv](https://arxiv.org/abs/2503.13657) — **texte**.
- 1 642 traces ; 14 modes (texte) : FC1 conception (FM-1.1 à 1.5, dont répétition d'étape 15,7 %), FC2 désalignement (FM-2.1 à 2.6, dont décalage raisonnement-action 13,2 %), FC3 vérification (FM-3.1 terminaison prématurée 6,2 %, FM-3.2 vérification absente ou incomplète 8,2 %, FM-3.3 vérification incorrecte 9,1 %).
- Annotation : trois annotateurs experts, κ = 0,88 ; juge LLM (o1) : exactitude 94 %, κ = 0,77.
- Aucune catégorie pour les défauts de l'orchestrateur lui-même (vérifié dans le texte).
- **Position** : base de notre taxonomie d'attribution (QR3). Notre extension, « faute du moteur » et « règle mal calibrée », comble une absence constatée. FM-3.2/3.3 correspondent à ce que nos contrôles liés au candidat doivent empêcher.

### Anthropic, système de recherche multi-agents
- Hadfield, Zhang, Lien, Scholz, Fox, Ford. *How we built our multi-agent research system*. Anthropic, 13 juin 2025. [Page](https://www.anthropic.com/engineering/multi-agent-research-system) — **texte**.
- « multi-agent systems use about 15× more tokens than chats » ; « deterministic safeguards like retry logic and regular checkpoints ».
- **Position** : retour d'industrie sur coût et reprise ; ne traite pas d'effets financiers.

### Cursor, février et juillet 2026
- Lin. *Towards self-driving codebases*. Cursor, 5 février 2026. [Page](https://cursor.com/blog/self-driving-codebases) — **texte** (intégrateur central retiré, 100 % de correction avant commit = sérialisation).
- Lin. *Agent swarms and the new model economics*. Cursor, 20 juillet 2026. [Page](https://cursor.com/blog/agent-swarm-model-economics) — **texte** : décisions consignées dans des documents de conception partagés, revues « decorrelated lenses stack », coût de 1 339 $ à 10 565 $ selon le mélange de modèles.
- **Position** : rapports d'ingénierie, non évalués par des pairs ; la tolérance d'erreur se discute pour le code réversible, pas pour un paiement.

### τ-bench
- Yao, Shinn, Razavi, Narasimhan. *τ-bench*. arXiv:2406.12045, 17 juin 2024. [arXiv](https://arxiv.org/abs/2406.12045) — **résumé**.
- Évaluation par état de base comparé à l'objectif ; métrique pass^k ; GPT-4o « succeed on <50% of the tasks ... pass^8 <25% in retail ».
- **Position** : nous reprenons pass^k et le principe « mesurer l'état final, pas le discours ».

---

## 4. Fondements cités sans relecture en ligne (classiques)

- Anderson, *Computer Security Technology Planning Study*, ESD-TR-73-51, 1972 — moniteur de référence. Référence exacte (volume, pagination) à vérifier dans une bibliothèque avant soumission.
- Garcia-Molina, Salem, *Sagas*, SIGMOD 1987 — compensation.
- Clés d'idempotence des API de paiement : citer la documentation d'un fournisseur, consultée et datée.

---

## Tableau de positionnement (provisoire)

Légende : **oui** = affirmé dans le résumé ou le texte lu ; **non** = hors du périmètre décrit ; **?** = non établi à ce niveau de lecture.

| Travail | Contrôle par appel | Contrôle du cycle de tentative (lancement, reprise) | Effets irréversibles / rejeu | Preuve liée au candidat | Mécanisme déterministe | Évaluation par fautes injectées |
| --- | --- | --- | --- | --- | --- | --- |
| ACRFence | oui (proxy MCP) | oui (restauration) | oui | ? | non (juge LLM) | oui |
| Safe to Resume? | ? | oui (attaque) | oui | ? | ? | oui (attaques) |
| Workflow fidelity | ? | non | ? | non | oui (garde de routage) | non |
| RAILS | ? | ? | oui (compensation) | oui (seuil de preuve) | ? | ? |
| AgentSpec | oui | ? | ? | ? | oui | ? |
| AgenticRei | oui | ? | ? | ? | oui | ? |
| PEP MCP | oui | ? | ? | ? | oui | ? |
| Policies on Paths | oui | oui (chemin) | ? | ? | oui | ? |
| CaMeL | oui | non | ? | non | oui | non (benchmark) |
| τ-bench | non | non | non | non | — | non (benchmark) |
| Ce travail (cible) | oui (B1 comme référence) | oui | oui | oui | oui | oui |

La ligne « Ce travail » est une cible de conception ; elle ne devient une
affirmation qu'après la campagne de mesures.

## Conséquences pour l'article

1. **ACRFence change le positionnement.** L'idée « une clé d'idempotence ne suffit pas pour un agent LLM » est déjà publiée. Notre apport ne peut pas être ce constat : c'est la réponse par conception (le LLM ne paie jamais ; clé métier dérivée de l'identité, pas de la requête générée ; acceptation liée à l'empreinte) et sa mesure sous huit fautes, comparée à une réponse par juge LLM.
2. **Workflow fidelity** fournit la meilleure motivation empirique : un point de confirmation laissé au modèle est sauté par 10 modèles sur 18.
3. **MAST** donne la base de la taxonomie d'attribution ; la classe « faute du moteur » est notre extension.
4. **Textes intégraux lus** : ACRFence, Safe to Resume?, RAILS, Policies on Paths, MAST, Design Patterns (passages ciblés). Restent au niveau du résumé : AgentSpec, AgenticRei, What Can Be Enforced?, CaMeL, Compositional Policy Violations, Workflow fidelity, Governing FinTech, τ-bench ; PEP MCP indirect.
5. **Motivation chiffrée disponible** : Safe to Resume? (93,8 % / 96,9 % d'échecs sur effets externes), ACRFence (10/10 doublons), Workflow fidelity (10 modèles sur 18 sautent la confirmation).
