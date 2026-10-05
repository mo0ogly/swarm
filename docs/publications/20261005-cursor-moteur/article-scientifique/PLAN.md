# Plan détaillé — article scientifique

Statut : plan de travail, 5 octobre 2026. Aucun résultat chiffré n'existe encore ;
les sections marquées « À PRODUIRE » conditionnent la rédaction.

---

## 0. Positionnement

### Titre (options)

1. *Proposer n'est pas agir : un moniteur déterministe pour agents LLM sur processus à effets irréversibles*
2. *Runtime Mediation for LLM Agent Swarms: Idempotent Launches, Candidate-Bound Evidence and Fault Attribution*
3. *Du code à la facture : contrôles d'exécution pour l'orchestration d'agents sur processus critiques*

Recommandation : titre 2 (anglais) si soumission internationale ; titre 1 pour une
version française.

### Thèse en une phrase

Pour des processus où une action a un effet irréversible (paiement, facturation,
livraison), la sûreté d'un essaim d'agents LLM ne peut pas reposer sur une revue
finale : un moniteur déterministe doit médiatiser chaque lancement, chaque reprise
et chaque acceptation, et ce coût de contrôle doit être mesuré, pas supposé.

### Question de recherche

- **QR1 (sûreté)** — Quels contrôles d'exécution empêchent les doubles effets,
  les acceptations sans preuve valide et les reprises sans cause corrigée ?
- **QR2 (coût)** — Quel surcoût ces contrôles imposent-ils (débit, blocages à tort,
  consommation) ? C'est la contrepartie directe du « goulot » observé par Cursor.
- **QR3 (attribution)** — Peut-on distinguer automatiquement une faute de l'agent,
  de l'environnement et du moteur lui-même ?

### Contributions revendiquées (à n'affirmer qu'une fois prouvées)

- C1. Un modèle d'états et six invariants de médiation pour agents (section 4).
- C2. Une implémentation ouverte (Swarm, Go, SQLite embarqué) et la correspondance
  invariant → test.
- C3. Un protocole d'injection de fautes et ses résultats, avec et sans moteur.
- C4. Une taxonomie d'attribution des échecs (agent / environnement / moteur /
  règle mal calibrée) et sa précision mesurée.
- C5. Une transposition sur un scénario de facturation simulé.

### Venues possibles (à vérifier : dates, format, appel)

- Préprint arXiv (cs.SE ou cs.MA) en premier, pour dater la contribution.
- Track industrie/pratique d'une conférence génie logiciel (type ICSE SEIP, FSE Industry).
- Ateliers sur agents LLM, sûreté et gouvernance de l'IA.
- Francophone : C&ESAR (cybersécurité appliquée) — angle « moniteur de référence ».

---

## 1. Résumé (≈ 200 mots, à écrire en dernier)

Structure imposée : contexte (agents LLM sur processus critiques) → problème
(la revue finale arrive après l'effet) → approche (moniteur déterministe, six
invariants) → évaluation (N scénarios de faute, M missions) → résultats
(chiffres QR1-QR3) → limite principale.

---

## 2. Introduction (≈ 1,5 page)

1. **Accroche concrète** — une réponse perdue, l'agent rejoue sa demande : deux
   agents démarrent, ou deux virements partent. Même problème, deux mondes.
2. **Pourquoi maintenant** — agents déployés dans la finance (paiement, gestion
   de patrimoine, commerce initié par agent) ; essaims de centaines d'agents
   (Cursor, février et juillet 2026).
3. **Lacune** — les travaux d'application des règles à l'exécution contrôlent
   surtout l'appel d'outil isolé ; la composition d'étapes conformes peut rester
   non conforme (Kurady et al., 2026). Un point de confirmation laissé au modèle
   est sauté par 10 modèles sur 18 dans un système de paiement (Huang et al., 2026).
   ACRFence (Zheng et al., 2026) a montré qu'une clé d'idempotence ne suffit pas
   quand un LLM réécrit sa requête au rejeu, et y répond par un juge LLM. Reste
   ouvert : une réponse déterministe par conception (le LLM n'émet jamais l'effet),
   la *fraîcheur des preuves* et *l'attribution des fautes au moteur lui-même*,
   mesurées sous fautes injectées.
4. **Contributions** — liste C1-C5.
5. **Ce que l'article ne revendique pas** — pas de gain de coût général, pas de
   preuve formelle complète, pas de déploiement bancaire réel.

---

## 3. Contexte et travaux liés (≈ 2 pages)

Statut des références : voir `fiches-lecture.md` (résumé lu / texte lu / indirect). Ne rien citer en « à lire » dans la
version soumise.

### 3.1 Orchestration d'essaims d'agents

- Lin, *Towards self-driving codebases*, Cursor, 5 février 2026 — **lue**.
  Planificateurs, sous-planificateurs, workers, handoff ; intégrateur central retiré
  (goulot) ; 100 % de correction avant commit = sérialisation. Rapport d'ingénierie,
  non évalué par des pairs.
- Lin, *Agent swarms and the new model economics*, Cursor, 20 juillet 2026 — texte lu (voir fiches).
- Hadfield et al., *How we built our multi-agent research system*, Anthropic, 13 juin 2025 — texte lu.
- Cemri et al., *Why Do Multi-Agent LLM Systems Fail?* (MAST), arXiv:2503.13657, 2025 — résumé lu.
  Sert de base à la taxonomie de la section 7.

### 3.2 Application des règles à l'exécution

- Wang, Poskitt, Sun, *AgentSpec*, ICSE 2026 — résumé lu.
- Joshi et al., *Deontic Policies for Runtime Governance of Agentic AI Systems* (AgenticRei), ICWS 2026 — résumé lu.
- Wang, *Runtime Policy Enforcement for MCP-Based LLM Agents*, Electronics 15(13):2829, 2026 — indirect (page éditeur en 403), à revérifier.
- Ray, *What Can Be Enforced?* (arXiv 2607.22868) — résumé lu.
- Kaptein et al., *Policies on Paths* (arXiv 2603.16586) — résumé lu.
- Kurady et al., *Compositional Policy Violations* (arXiv 2609.18820) — résumé lu.
- Debenedetti et al., *Defeating Prompt Injections by Design* (CaMeL), arXiv:2503.18813 — résumé lu.
- Beurer-Kellner et al., *Design Patterns for Securing LLM Agents*, arXiv:2506.08837 — résumé lu.

**Différenciation à démontrer** : ces travaux médiatisent l'*appel d'outil* ;
nous médiatisons le *cycle de vie de la tentative* (lancement, reprise, preuve,
acceptation) et nous mesurons les fautes du moniteur lui-même.

### 3.3 Exécution durable et transactions

- Garcia-Molina, Salem, *Sagas*, SIGMOD 1987 — compensation.
- Clés d'idempotence des API de paiement (documentation Stripe).
- Exécution durable : Temporal, Restate — positionnement « workflow figé ».
- **ACRFence** (arXiv 2603.20625, CoDAIM 2026) — texte lu en partie : **travail le plus proche**. Montre que les clés d'idempotence supposent des requêtes identiques au rejeu, hypothèse violée par les LLM ; répond par un juge LLM d'équivalence. Notre différence : LLM sans droit de paiement, règlement déterministe, clé métier.
- *Safe to Resume?* (arXiv 2608.29381) — texte lu : SF5 = notre F3 ; 93,8 % / 96,9 % d’échecs sur effets externes ; aucun mécanisme général proposé.

### 3.4 Agents et finance

- Han, *Governing Agentic AI in FinTech* (arXiv 2608.11344) — résumé lu.
- Huang et al., *Measuring Workflow Fidelity in LLM-Based Agentic Payment Systems* (arXiv 2605.06457, PAKDD 2026 AIDS4DF) — résumé lu : 10 modèles sur 18 sautent un point de confirmation ; motivation empirique principale.
- *RAILS* (arXiv 2606.08790) — texte lu : modèle formel sans mesure ; décide quel règlement suit, pas son exécution unique ; complémentaire.
- Yao et al., *τ-bench*, arXiv:2406.12045, 2024 — résumé lu ; métrique pass^k.

### 3.5 Fondements

- Anderson, *Computer Security Technology Planning Study*, 1972 — moniteur de
  référence : médiation complète, inviolable, vérifiable. Cadre conceptuel de l'article.
- Séparation des tâches / principe des quatre yeux (maker-checker).
- Cadre réglementaire (contexte, pas contribution) : AI Act art. 14 (supervision
  humaine), DORA (résilience opérationnelle numérique, secteur financier).

### 3.6 Tableau de positionnement (Figure/Tableau 1)

Colonnes : médiation de l'appel / du lancement / de la reprise ; preuve liée au
candidat ; idempotence des mutations ; budgets fail-closed ; attribution des fautes ;
évaluation par injection de fautes. Lignes : AgentSpec, CaMeL, Temporal+LLM,
Cursor, τ-bench, ce travail. Version provisoire dans `fiches-lecture.md` (à compléter après lecture des textes intégraux).

---

## 4. Modèle (≈ 2 pages) — contribution C1

### 4.1 Acteurs et séparation des rôles

- **Agents** : proposent plans, produits, explications. Aucune autorité.
- **Moteur** : seul à transformer une proposition en action et un processus
  terminé en résultat accepté.
- **Humain** : fixe objectif, périmètre, politique de validation ; arbitre les exceptions.

### 4.2 État

Travail, tâches (graphe de dépendances), tentatives, espaces réservés, preuves
(liées à une empreinte de candidat), révision globale, journal d'événements.

### 4.3 Invariants (à formuler précisément, puis relier aux tests)

| Id | Invariant | Mécanisme Swarm | Tests existants (exemples) |
|----|-----------|-----------------|----------------------------|
| I1 | Au plus un effet par intention (`event_id`) | idempotence des mutations | `TestIdempotencyAndRevision`, `TestConductorReplayHasNoDoubleEffect`, `TestExchangeHostConcurrentSendHasOneEffect` |
| I2 | Aucune mutation sur un état périmé (`expected_revision`) | concurrence optimiste | `TestConcurrentWriters`, `TestGatePreviewRevisionConflictVisible` |
| I3 | Aucune acceptation sans preuve fraîche liée au candidat | empreinte de preuve | `TestAcceptanceFreshnessAndAttempts`, `TestAutomaticValidationStaleEvidenceBlocksDependency`, `TestExchangeChangedArtifactBecomesStale` |
| I4 | Un lancement est atomique et exclusif sur son espace | réservation par arbre de chemins | `TestLaunchAtomicIdempotentAndWorkspaceExclusive` |
| I5 | Limite atteinte = blocage, jamais succès | budgets fail-closed | `TestBudgetReservationReplayAndExhaustion`, `TestCostReadFailureStopsDispatchAndDecisions` |
| I6 | Une reprise exige une nouvelle preuve de cause corrigée | diagnostic de tentative | `TestEnvironmentRetryRequiresNewEvidenceAndKeepsLimits` |
| I7 | Aucun texte issu d'un modèle n'est interprété comme commande | liste blanche, sans shell | à identifier |
| I8 | Une confirmation porte exactement sur l'aperçu montré | jeton d'aperçu | `TestOperatorPreview`, `TestConductorExactPreviewAndIdempotencyIsolation` |

Correspondance finance (Tableau 2) : I1 ↔ clé d'idempotence de paiement ;
I2 ↔ double débit concurrent ; I3 ↔ validation liée à la version de l'ordre ;
I8 ↔ maker-checker ; I5 ↔ plafonds ; I6 ↔ rejeu après incident.

### 4.4 Ce que le modèle ne garantit pas

Vérité métier des preuves ; exhaustivité des critères saisis ; justesse des règles
(« déterministe ≠ correct ») ; sûreté compositionnelle au-delà des dépendances déclarées.

### 4.5 Option : spécification formelle

Modéliser I1, I2, I4 en TLA+ (ou Alloy) et vérifier par model checking sur
petites instances. Renforce fortement l'article ; coût estimé non négligeable.
**À décider.**

---

## 5. Implémentation (≈ 1,5 page) — contribution C2

- Go, binaire statique, SQLite embarqué (`modernc.org/sqlite`), hors ligne.
- Requêtes versionnées : `schema_version`, `event_id`, `expected_revision`.
- Coordinateur durable : déduplication des boucles concurrentes, clôture du
  propriétaire expiré, reprise de la même tentative après redémarrage.
- Politique de validation : `human` ou `automatic` (1-8 contrôles, commandes en
  liste blanche, budget cumulé ≤ 300 s, couverture de chaque critère obligatoire).
- Comptabilité de consommation : appels d'outils, requêtes, jetons, durée, coût
  rapporté ; coût absent = inconnu, jamais estimé.
- Interfaces : CLI et cockpit web en loopback, même décision métier.
- **Figure 2** : architecture (reprendre `orchestration.svg`, retirer la partie
  « évolutions prévues » ou la placer en travaux futurs).
- Taille : ≈ 30 000 lignes Go hors tests, 478 fonctions de test (mesure du
  5 octobre 2026, à refaire au gel de version). Citer le commit exact.

---

## 6. Évaluation de la sûreté et du coût — QR1, QR2 (≈ 3 pages) — C3

### 6.1 Conditions comparées

- **B0** : orchestration sans moteur (agents + revue finale seule).
- **B1** : moteur avec contrôles d'appel seulement (approximation des approches
  « policy enforcement point »).
- **S** : Swarm complet.
- Option **B2** : workflow figé (exécution durable), pour situer le compromis
  sûreté / liberté d'exploration.

### 6.2 Campagne d'injection de fautes (déterministe, reproductible)

| Faute injectée | Mesure |
|----------------|--------|
| Réponse perdue puis rejeu | nombre d'effets dupliqués |
| Deux lanceurs concurrents | lancements doubles, espaces partagés |
| Crash avant / après commit | état incohérent, tentative perdue ou doublée |
| Candidat modifié après preuve | acceptations sur preuve périmée |
| Environnement indisponible (navigateur, réseau) | reprises inutiles du code |
| Budget épuisé | succès déclarés à tort |
| Propriétaire expiré | double reprise |
| Rapport ancien relayé | acceptation sur ancienne tentative |

Pour chaque faute : 100+ répétitions, agents factices puis agents réels.
Rapporter taux, intervalles de confiance, pass^k pour les agents réels.

### 6.3 Missions réelles (QR2)

- N missions comparables (même périmètre, même modèle), en B0 et S.
- Mesures : durée, appels d'outils, requêtes, jetons, coût rapporté, nombre de
  blocages, **blocages à tort** (étiquetés manuellement), reprises, rework.
- **À PRODUIRE** : choisir N (≥ 20 pour un premier signal), figer le modèle et
  la version, pré-enregistrer les métriques avant exécution.

### 6.4 Résultats attendus (hypothèses, pas résultats)

- H1 : S élimine les doubles effets que B0 produit sous rejeu.
- H2 : S a un surcoût de durée mesurable, concentré sur les attentes de dépendance.
- H3 : une partie des blocages de S sont à tort, dus à des règles mal calibrées
  (fraîcheur trop large, plafond trop bas) — à quantifier, pas à cacher.

---

## 7. Attribution des fautes — QR3 (≈ 1,5 page) — C4

- Taxonomie : faute de l'agent / de l'environnement / du moteur (bug) / de la
  règle (règle juste mais mal calibrée) / inconnue. Partir de MAST et l'étendre.
- Existant : diagnostic de tentative qui sépare erreurs observées, limite
  consécutive, échecs d'outils et environnement
  (`TestAttemptDiagnostic*`, `TestAttemptDiagnosticBwrapMountFailureIsEnvironment`).
- Protocole : étiqueter à la main un corpus de tentatives échouées (deux
  annotateurs, accord inter-annotateurs), comparer au diagnostic automatique.
- Métriques : précision / rappel par classe, taux « inconnu » assumé.
- Point clé : montrer des cas où **le moteur était fautif** — c'est ce qui rend
  l'article crédible.

---

## 8. Transposition : facturation simulée (≈ 1,5 page) — C5

- Banc minimal : grand livre simulé, API de paiement fictive avec clé
  d'idempotence, invariants en partie double, plafond par montant, rapprochement.
  Données entièrement synthétiques, présentées comme telles.
- Rejouer les fautes de 6.2 sur ce banc.
- Montrer la correspondance I1-I8 → contrôles financiers (Tableau 2).
- **À PRODUIRE** : ce banc n'existe pas aujourd'hui ; Swarm coordonne des agents
  de développement. Sans ce banc, l'article reste valide mais la finance devient
  motivation et discussion, pas résultat.

---

## 9. Discussion (≈ 1 page)

- Compromis avec Cursor : leur tolérance d'erreur pour le débit vs notre
  fermeture en cas d'échec. Thèse : la tolérance convient au code réversible,
  pas aux effets irréversibles. Où placer la frontière.
- Contrôle proportionné : ce qui doit être bloquant pendant l'exécution, ce qui
  peut attendre la validation d'ensemble.
- Supervision humaine : quand le moteur doit rendre la main, et ce que l'humain
  voit (cause, qui peut agir, condition de reprise).

## 10. Menaces sur la validité (≈ 0,5 page)

- Interne : agents factices dans une partie des tests ; règles écrites par l'auteur
  du moteur.
- Externe : un seul projet, un seul opérateur ; domaine logiciel ≠ banque réelle.
- Construction : « blocage à tort » dépend d'un étiquetage humain.
- Conclusion : non-déterminisme des modèles → répétitions, pass^k, intervalles.
- Version des modèles et fournisseurs figée et déclarée.

## 11. Conclusion et travaux futurs (≈ 0,5 page)

Édition du graphe par brouillon, programmation durable, espaces parallèles
administrés (aujourd'hui non activés), spécification formelle complète.

## Annexes

- A. Disponibilité : dépôt, commit, commandes de reproduction (`go test -race ./...`).
- B. Liste complète invariant → tests.
- C. Grille d'étiquetage des fautes.

---

## Chemin critique avant rédaction

1. Lire et classer les références « à lire » ; remplir le Tableau 1.
2. Figer une version de Swarm (commit) et isoler les tests par invariant (Annexe B).
3. Écrire le harnais d'injection de fautes B0 / S (réutiliser les tests existants).
4. Lancer la campagne de missions réelles (6.3), métriques pré-enregistrées.
5. Constituer et étiqueter le corpus d'échecs (7).
6. Décider : banc de facturation (8) oui/non ; TLA+ (4.5) oui/non.
7. Vérifier publication : dépôt public conforme, autorisation de diffusion
   (code développé en contexte ANSSI).
8. Rédiger, résumé en dernier.
