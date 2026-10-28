# Correspondance avec les contrats internes / Internal contract map

Les vues décrivent les contrats de l’architecture Swarm. Elles conservent les objets et les frontières internes ; la matrice de maturité Rust reste dans la vue technique. / These views describe Swarm architectural contracts; Rust implementation maturity remains in the technical overview.

Chaque identifiant ci-dessous existe dans les sources Mermaid FR et EN. Les liens pointent vers les sections réellement lues ; leurs empreintes et lignes sont conservées dans [architecture-contracts.json](architecture-contracts.json). Un lien source établit la provenance du dessin, pas l’exécution ou la conformité du code. / Every node ID exists in both Mermaid languages. Source links establish provenance, not execution or code conformance.

## Architecture interne Swarm : moteur, responsabilités et preuves / Swarm internal architecture: engine, responsibilities and evidence

[SVG FR](architecture-overview-fr.svg) · [SVG EN](architecture-overview-en.svg) · [Mermaid FR](architecture-overview-fr.mmd) · [Mermaid EN](architecture-overview-en.mmd)

| Composants / Nodes | Contrat interne / Internal contract |
| --- | --- |
| `A_INPUT`, `A_ENGINE`, `A_SCHEDULE` | [Protocole entre les IA et le moteur déterministe](../architecture/AGENT-ENGINE-PROTOCOL.md#protocole-entre-les-ia-et-le-moteur-déterministe) |
| `A_PLAN` | [Liaison au contrat courant du plan — IA3a](../architecture/AGENT-IDENTITY-CONTRACT.md#liaison-au-contrat-courant-du-plan--ia3a) |
| `A_ROOT`, `A_SCOPE` | [Répartition des responsabilités](../architecture/RUN-SUPERVISION.md#répartition-des-responsabilités) |
| `A_CYCLE` | [Fin et refus](../architecture/JUDGMENT-CYCLES.md#fin-et-refus) |
| `A_ID` | [Données et sources d'autorité](../architecture/AGENT-IDENTITY-CONTRACT.md#données-et-sources-dautorité) |
| `A_POLICY` | [Configuration persistée et opérations publiques](../architecture/PROVIDER-LIVENESS.md#configuration-persistée-et-opérations-publiques) |
| `A_CONTEXT` | [Contexte préparé et contexte de départ](../PROJECT-MEMORY.md#contexte-préparé-et-contexte-de-départ) |
| `A_WORK` | [Contrat réellement implémenté](../architecture/RUN-SUPERVISION.md#contrat-réellement-implémenté) |
| `A_DELIVERY`, `A_REVIEW` | [Comportement global du moteur](../architecture/REVIEW-EVIDENCE.md#comportement-global-du-moteur) |
| `A_GATE`, `A_ACCEPT`, `A_STATE` | [La chaîne de preuve](../architecture/TRACEABILITY.md#la-chaîne-de-preuve) |
| `A_MEMORY` | [Utilisation](../PROJECT-MEMORY.md#utilisation) |
| `A_LOG` | [Console des journaux](../architecture/LOG-CONSOLE.md#console-des-journaux) |
| `A_AC02` | [AC-02 — Plan APEX consolidé avec spécification des menus de sécurité](../plans/actions-confiance/securite-execution-apex.md#ac-02--plan-apex-consolidé-avec-spécification-des-menus-de-sécurité) |
| `A_HUMAN` | [Contrôles comme preuves d’une revue humaine](../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine) |
| Limite Rust / Rust status | [Tâches ordinaires et validation humaine — raccord139 en qualification](../architecture/RUST-MIGRATION.md#tâches-ordinaires-et-validation-humaine--raccord139-en-qualification) |

## Identité Swarm : source d’autorité, registre et usage exact / Swarm identity: authority source, registry and exact use

[SVG FR](identity-authority-fr.svg) · [SVG EN](identity-authority-en.svg) · [Mermaid FR](identity-authority-fr.mmd) · [Mermaid EN](identity-authority-en.mmd)

| Composants / Nodes | Contrat interne / Internal contract |
| --- | --- |
| `I_HUMAN` | [Authentification locale par défaut et réglage UX](../architecture/AGENT-IDENTITY-CONTRACT.md#authentification-locale-par-défaut-et-réglage-ux) |
| `I_PINS`, `I_ENROLL` | [Authentifier une réponse du futur service d’enrôlement — IA3c.2a](../architecture/AGENT-IDENTITY-CONTRACT.md#authentifier-une-réponse-du-futur-service-denrôlement--ia3c2a) |
| `I_JOURNAL` | [Journal durable de l’enrôlement — IA3c.2b](../architecture/AGENT-IDENTITY-CONTRACT.md#journal-durable-de-lenrôlement--ia3c2b) |
| `I_AUTH`, `I_PERMIT` | [Émettre un permis depuis une autorité humaine explicite — IA3b](../architecture/AGENT-IDENTITY-CONTRACT.md#émettre-un-permis-depuis-une-autorité-humaine-explicite--ia3b) |
| `I_CONFIG` | [Témoin local et reprise coordonnée — IA3c.2f-02b](../architecture/AGENT-IDENTITY-CONTRACT.md#témoin-local-et-reprise-coordonnée--ia3c2f-02b) |
| `I_CHANNEL` | [Remise privée IA3c.2f-03c — preuve avant demande métier](../architecture/AGENT-IDENTITY-CONTRACT.md#remise-privée-ia3c2f-03c--preuve-avant-demande-métier) |
| `I_PLAN` | [Liaison au contrat courant du plan — IA3a](../architecture/AGENT-IDENTITY-CONTRACT.md#liaison-au-contrat-courant-du-plan--ia3a) |
| `I_REGISTRY`, `I_HEAD` | [Source durable et publication récupérable — IA2.3b](../architecture/AGENT-IDENTITY-CONTRACT.md#source-durable-et-publication-récupérable--ia23b) |
| `I_ACTOR`, `I_GRANT`, `I_RECEIPT`, `I_COMMIT`, `I_OPS` | [Données et sources d'autorité](../architecture/AGENT-IDENTITY-CONTRACT.md#données-et-sources-dautorité) |
| `I_REQUEST`, `I_ENGINE`, `I_PROOF` | [Contrat du consommateur à réaliser](../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md#contrat-du-consommateur-à-réaliser) |
| `I_PREPLAN` | [Le cadrage précède l'adoption du plan](../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md#le-cadrage-précède-ladoption-du-plan) |

## Mémoire Swarm : connaissances, contexte et reprise de conversation / Swarm memory: knowledge, context and conversation recovery

[SVG FR](memory-context-fr.svg) · [SVG EN](memory-context-en.svg) · [Mermaid FR](memory-context-fr.mmd) · [Mermaid EN](memory-context-en.mmd)

| Composants / Nodes | Contrat interne / Internal contract |
| --- | --- |
| `M_RETEX`, `M_ADMIT`, `M_VERSION`, `M_SELECT` | [Utilisation](../PROJECT-MEMORY.md#utilisation) |
| `M_REPLACE` | [Stockage, reprise et limites](../PROJECT-MEMORY.md#stockage-reprise-et-limites) |
| `M_RIGHTS`, `M_ENGINE` | [Contrat du consommateur à réaliser](../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md#contrat-du-consommateur-à-réaliser) |
| `M_ATTRIB` | [Attribution, propriété et permission](../plans/deepagents-integration/CONTRAT-USAGES-MEMOIRE.md#attribution-propriété-et-permission) |
| `M_EXTRACT`, `M_CHECK` | [Sélection extractive pour un lancement explicite](../PROJECT-MEMORY.md#sélection-extractive-pour-un-lancement-explicite) |
| `M_HISTORY`, `M_METHODS`, `M_MANIFEST`, `M_PROVIDER` | [Contexte préparé et contexte de départ](../PROJECT-MEMORY.md#contexte-préparé-et-contexte-de-départ) |
| `M_INSTRUCT` | [Amélioration récursive des consignes](../architecture/INSTRUCTION-LEARNING.md#amélioration-récursive-des-consignes) |
| `M_BOUNDARY` | [État durable aux frontières du dialogue](../PROJECT-MEMORY.md#état-durable-aux-frontières-du-dialogue) |
| `M_RECOVERY`, `M_RESUME` | [Reprise explicite d'une conversation arrêtée](../PROJECT-MEMORY.md#reprise-explicite-dune-conversation-arrêtée) |
| `M_RECONCILE` | [Fin confirmée d’un tour de dialogue](../PROJECT-MEMORY.md#fin-confirmée-dun-tour-de-dialogue) |

Complément `M_INSTRUCT` : [historique et transmission](../architecture/INSTRUCTION-LEARNING.md#historique-et-transmission).

## Supervision Swarm : observation, décision, application et reprise / Swarm supervision: observation, decision, application and recovery

[SVG FR](run-supervision-fr.svg) · [SVG EN](run-supervision-en.svg) · [Mermaid FR](run-supervision-fr.mmd) · [Mermaid EN](run-supervision-en.mmd)

| Composants / Nodes | Contrat interne / Internal contract |
| --- | --- |
| `S_WORKER`, `S_OBSERVE`, `S_EVENT`, `S_DECIDE`, `S_ENGINE`, `S_RECORD`, `S_CONTINUE`, `S_STOP`, `S_END`, `S_POLICY` | [Contrat réellement implémenté](../architecture/RUN-SUPERVISION.md#contrat-réellement-implémenté) |
| `S_INBOX` | [Protocole entre les IA et le moteur déterministe](../architecture/AGENT-ENGINE-PROTOCOL.md#protocole-entre-les-ia-et-le-moteur-déterministe) |
| `S_SCOPE`, `S_ROOT` | [Répartition des responsabilités](../architecture/RUN-SUPERVISION.md#répartition-des-responsabilités) |
| `S_WAIT` | [Plusieurs agents, propriétaires et indisponibilité](../architecture/RUN-SUPERVISION.md#plusieurs-agents-propriétaires-et-indisponibilité) |
| `S_EFFECT` | [La chaîne de preuve](../architecture/TRACEABILITY.md#la-chaîne-de-preuve) |
| `S_REVIEW` | [Comportement global du moteur](../architecture/REVIEW-EVIDENCE.md#comportement-global-du-moteur) |
| `S_LEARN` | [Historique et transmission](../architecture/INSTRUCTION-LEARNING.md#historique-et-transmission) |
| `S_CALL` | [Configuration persistée et opérations publiques](../architecture/PROVIDER-LIVENESS.md#configuration-persistée-et-opérations-publiques) |
| `S_JUDGE` | [Fin et refus](../architecture/JUDGMENT-CYCLES.md#fin-et-refus) |
| `S_HUMAN`, `S_ACCEPT` | [Contrôles comme preuves d’une revue humaine](../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine) |
| Limite Rust / Rust status | [Tâches ordinaires et validation humaine — raccord139 en qualification](../architecture/RUST-MIGRATION.md#tâches-ordinaires-et-validation-humaine--raccord139-en-qualification) |

Complément `S_ACCEPT` : [preuves et candidat courant](../architecture/REVIEW-EVIDENCE.md#comportement-global-du-moteur).

## Observabilité Swarm : logs, direct, télémétrie et preuves / Swarm observability: logs, live views, telemetry and evidence

[SVG FR](logs-observability-fr.svg) · [SVG EN](logs-observability-en.svg) · [Mermaid FR](logs-observability-fr.mmd) · [Mermaid EN](logs-observability-en.mmd)

| Composants / Nodes | Contrat interne / Internal contract |
| --- | --- |
| `L_AGENT`, `L_PROC`, `L_PUSH`, `L_LIVE` | [Observation en direct des agents — LIVE-UX-01 (lecture seule)](../architecture/RUST-MIGRATION.md#observation-en-direct-des-agents--live-ux-01-lecture-seule) |
| `L_ENGINE`, `L_CHAIN` | [La chaîne de preuve](../architecture/TRACEABILITY.md#la-chaîne-de-preuve) |
| `L_CALLS` | [Contrat et provenance](../architecture/CALL-ACCOUNTING.md#contrat-et-provenance) |
| `L_OTLP`, `L_NORMAL`, `L_TELEMETRY` | [Télémétrie fournisseur — LIVE-UX-03, incrément E2E-1 (local, vérifié)](../architecture/RUST-MIGRATION.md#télémétrie-fournisseur--live-ux-03-incrément-e2e-1-local-vérifié) |
| `L_ORIGINAL`, `L_API`, `L_CONSOLE`, `L_EXPORT` | [Console des journaux](../architecture/LOG-CONSOLE.md#console-des-journaux) |
| `L_ACCEPT` | [Contrôles comme preuves d’une revue humaine](../ENGINE-ACCEPTANCE.md#contrôles-comme-preuves-dune-revue-humaine) |
| `L_SECURITY` | [AC-02 — Plan APEX consolidé avec spécification des menus de sécurité](../plans/actions-confiance/securite-execution-apex.md#ac-02--plan-apex-consolidé-avec-spécification-des-menus-de-sécurité) |
| Limite Rust / Rust status | [Tâches ordinaires et validation humaine — raccord139 en qualification](../architecture/RUST-MIGRATION.md#tâches-ordinaires-et-validation-humaine--raccord139-en-qualification) |

Complément `L_ACCEPT` : [preuves et candidat courant](../architecture/REVIEW-EVIDENCE.md#comportement-global-du-moteur).

## Contrôles sémantiques / Semantic review

- Le moteur déterministe Rust demeure le cœur de toutes les vues ; les agents proposent et le moteur contrôle/applique. / The deterministic Rust engine stays central; agents propose and the engine checks/applies.
- Identité humaine, principal, agent logique, instance, propriétaire et auteur restent distincts. Registre/tête Trust et configuration/témoin sont des états différents. / Human identity, principal, logical agent, instance, owner and author remain distinct; registry/Trust head and configuration/witness are different state.
- Signature, décision et engagement au dernier usage ne sont pas interchangeables ; l’autorité avant plan n’est pas déduite d’un cookie. / Signature, decision and final-use commit are distinct; pre-plan authority is not inferred from a cookie.
- La mémoire conserve connaissances, contexte de tentative, historique, consignes et frontières de conversation. Lire, résumer et transmettre sont des usages distincts. / Memory retains knowledge, attempt context, history, instructions and conversation boundaries; read, summarize and transmit are separate uses.
- Réception, frontière de tour, effets incertains, nouvelle tentative et même conversation restent visibles dans la reprise. / Reception, turn boundaries, uncertain effects, new attempt and same conversation remain visible in recovery.
- run_observation, boîte du propriétaire, événement reçu, décision attribuée, bail/révision/budgets et confirmation d’arrêt sont conservés. / run_observation, owner inbox, received event, attributed decision, lease/revision/budgets and stop confirmation remain explicit.
- Journaux, direct, OTLP, mesures Linux et registre d’appels ne sont ni fusionnés en une vérité unique ni confondus avec une acceptation. / Logs, live views, OTLP, Linux metrics and call records retain their sources and do not become acceptance.
- Validation humaine du résultat, revue IA et révision des consignes restent distinctes : en mode `human`, ni un PASS ni un avis IA ne remplacent la décision humaine ; la source humaine protégée est un raccord séparé. / Human result validation, AI review and instruction revision remain separate; in `human` mode, neither PASS nor AI review replaces the human decision; protected human identity is separate.
- Candidat, originaux, contrôles, revue, jugement global et acceptation conservent leurs frontières ; AC-02 reste futur et non démarré. / Candidate, originals, checks, review, global judgment and acceptance retain their boundaries; AC-02 remains future and not started.

Cette relecture est celle de Codex dans la même session. Elle ne constitue pas une revue indépendante ni une qualification du moteur. / This is same-session Codex review, not independent review or engine qualification.

## Précisions éditoriales du film / Film editorial limits

`I_JOURNAL` rappelle que l’authenticité des octets ne certifie pas la vérité de leur
contenu ; `M_VERSION` rappelle qu’une mémoire ne crée pas de droit. Ces limites
n’ajoutent ni nœud ni capacité. Le raisonnement et les sources sont conservés dans
le [chapitre d’introduction](../communication/linkedin-swarm-identite-memoire-20261023/article-v4-fr.md)
et son [contrôle des sources](../communication/linkedin-swarm-identite-memoire-20261023/sources-v4-review.json).
Les empreintes historiques du catalogue restent attribuées à leurs observations ;
cette correction de libellés ne requalifie pas tous les fichiers Rust en évolution.

`I_JOURNAL` separates authenticated bytes from truthful content; `M_VERSION`
separates memory from authority. These editorial clarifications add no nodes or
capability and do not renew historical runtime qualification claims.
