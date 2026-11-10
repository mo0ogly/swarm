# Feuille de route inspirée de LangWatch — spécification de préparation

## Raccord à la file APEX — 24 octobre 2026

La [file APEX FR](../FILE-APEX.md#vague-2--rapprochement-des-suivis-antérieurs-24-octobre-2026)
et [EN](../../en/APEX-QUEUE.md#wave-2--reconciling-earlier-tracking-october-24-2026)
conserve cette feuille de route, ses acquis locaux et ses limites. Codex racine
coordonne ce rapprochement documentaire. La répartition Claude/Codex proposée
plus bas reste historique : elle ne lance ni délégation ni implémentation ; le
propriétaire d’une nouvelle tranche doit être confirmé avant son adoption.

L1 partage son périmètre avec [LIVE-UX-03](../telemetrie-agents/SPEC.md) ; sa
réalisation et ses preuves ne sont pas comptées deux fois. L4 distingue les
évaluateurs déterministes décrits comme livrés et le
[juge par modèle proposé](../telemetrie-agents/JUGE-L4-SPEC.md), toujours non
adopté. La priorisation et les inconnues initiales ne remplacent pas les résultats
plus récents du statut ci-dessous ; L2 reste dérivé en fin de tentative et la
liaison L9 au commit reste à prouver de bout en bout. Une prochaine tranche
inventorie chaque preuve, son candidat, sa fraîcheur et ses critères de revue.
Aucun lot L1–L11 n’est qualifié à nouveau ni déclaré accepté par cette inscription.

Statut : **proposé ; lots réalisés localement le 22 octobre 2026** : L1 en grande partie (OTLP logs/métriques, Claude, lancement instrumenté), L3 (santé du cache, réglages v31), L2 (transcript typé dérivé à la lecture du flux retenu en fin de tentative ; pas de transcript en direct), L5 (alertes superviseur : six règles sur marques moteur, historique durable, notifications ; évaluation seulement console ouverte), L9 (registre des livraisons : tâche → candidat revu + effort/coût observés ; commit non prouvé de bout en bout faute de revue Rust publique). Le 22 octobre (suite) : L7 (alertes seuil de budget et dépassement constaté, v35), L11 (regroupement déterministe des causes d’échec), L10 (annotations opérateur, v36), L8 (export JSONL du jeu de régression, faits enregistrés seulement), L4 (évaluateurs déterministes asynchrones, v37 ; **juge par modèle non réalisé** : il doit passer par les portes de lancement et les réservations de budget du moteur), L6 (scénarios d’attentes par tâche jugés contre les faits enregistrés), plus l’évaluation des alertes par le serveur et les traces Claude Code. Preuves locales seulement ; aucune revue indépendante. Détails : `docs/architecture/RUST-MIGRATION.md` § LIVE-UX-03. Auteur : Claude (21 octobre
2026), auto-revue seulement. Source : lecture du dépôt
<https://github.com/langwatch/langwatch> (branche `main`, 21 octobre 2026) et de
sa documentation <https://langwatch.ai/docs/llms.txt>.

## 1. Objet et règles

Objet : retenir de LangWatch (observabilité des agents LLM, module
`modules/coding-agent` très proche de Swarm) ce qui peut améliorer Swarm, en lots
priorisés, sans détourner la migration Go→Rust.

Règles :
- **Licences** (`NOTICE`, `LICENSE.md`, `enterprise/LICENSE.md`) : Apache-2.0
  hors `enterprise/` (réutilisation avec attribution et conservation du NOTICE) ;
  MIT pour `sdks/typescript`, `sdks/python`, `mcp/typescript`,
  `services/langevals` (autres `LICENSE` par dossier non vérifiés).
  **`enterprise/` est propriétaire : aucune copie de code ni de conception
  détaillée** (gouvernance, anomalies de dépense, attribution des coûts, routage
  avancé, SSO/SCIM).
- On reprend des **idées et contrats**, pas du code TypeScript ; toute
  implémentation Swarm est native Rust.
- Toute politique (seuil, fenêtre, rétention) devient un réglage Admin persistant
  et modifiable (AGENTS.md : une seule source de vérité, aucune constante cachée).
- Aucune donnée de prompt/contenu stockée par défaut.

## 2. Inventaire

| # | Idée LangWatch (source) | Équivalent Swarm actuel | Valeur pour Swarm | Coût | Prérequis |
|---|---|---|---|---|---|
| L1 | Ingestion OTLP des agents de code (`modules/trace/process/src/transport/otlp-ingest.rest.ts`, `modules/log`, `modules/metric`, ADR-055) | aucun ; usage = dernier événement terminal (`agent_provider_facts.rs`) | jetons/coût par appel, outils, lignes, décisions d'édition | élevé | [spec télémétrie](../telemetrie-agents/SPEC.md) |
| L2 | Transcript typé (`coding-agent-transcript.ts` : `system_prompt`, `user_prompt`, `assistant_message`, `model_call`, `tool`, `tool_rejected`, `note`) | journal plat `agent_logs` + parse `agent_native_stream.rs:159-296` | lecture structurée du travail, filtres par type, base de L3/L4 | moyen | aucun (flux déjà parsé) |
| L3 | Santé du cache / reconstruction (`token-timeline.ts` : seuils 0,5 / 1000 jetons ; page session « Cache health ») | aucun | repérer les tentatives qui paient deux fois le contexte | moyen | L1 ou usage par tour ; seuils en réglages Admin |
| L4 | Moniteurs : évaluations asynchrones non bloquantes, échantillonnage, préconditions (`modules/evaluator`, `modules/monitor`) | revues bloquantes (reviewer) | score qualité continu sans ralentir ; tri des tentatives à revoir | élevé | L2 ; budget d'appels séparé |
| L5 | Automatisations filtre→action, digest 5 min, alertes sur séries avec reprise, historique des déclenchements, test d'envoi (`modules/automation`, `docs/features/automations.mdx`) | notifications ad hoc | alertes superviseur fiables et traçables | moyen | événements moteur stables |
| L6 | Scénarios : critères jugés contre les traces, pas contre les dires de l'agent (`modules/scenario`, `docs/agent-testing/scenarios.mdx`) | critères de tâche, preuves, revues | acceptation fondée sur preuves observables | élevé | L2 ; recettes existantes |
| L7 | Budgets : portée hiérarchique, fenêtres, avertissement à 80 %, dépense comptée après réponse (`docs/ai-gateway/budgets.mdx`, partie Apache) | budgets mission/admin, réservations | avertissement précoce, lisibilité de l'écart après coup | moyen | chemin critique budgets (Codex) |
| L8 | Jeux de données depuis traces (`modules/dataset`) | aucun | jeux de régression depuis tentatives échouées | moyen | L2 |
| L9 | Lien session→PR, usage cumulé par PR (`coding-agent-trace-pull-request.ts`) | livraisons locales numérotées | coût et effort par livraison | faible | L1 ou usage cumulé existant |
| L10 | Files d'annotation (`modules/annotation`) | revues | revue humaine outillée | moyen | L2 |
| L11 | Regroupement thématique (`modules/topic`) | aucun | regrouper les causes d'échec | élevé | volume suffisant |

Propriétaire par lot : non attribué ; à désigner par l'utilisateur au moment de
l'adoption (par défaut : Claude pour les lots UX/observation, Codex pour ce qui
touche budgets, revues et chemin critique).

## 3. Priorisation proposée

Critères : valeur directe pour l'opérateur, coût, dépendances, interférence avec
la migration.

1. **L2 transcript typé** — valeur élevée, coût moyen, aucune dépendance,
   réutilise le parseur natif ; prépare L3/L4/L6/L8/L10.
2. **L1 télémétrie** — valeur très élevée, mais touche lancement et auth ; voir
   la spec dédiée (décision d'ordre ouverte).
3. **L9 coût par livraison** — faible coût, lisibilité budget.
4. **L5 alertes superviseur** — fiabilité opérationnelle.
5. **L3 santé du cache** — après L1 (ou usage par tour).
6. **L7 avertissements budgets** — à coordonner avec le travail budgets de Codex.
7. **L6, L4, L8, L10** — après stabilisation de L2 et de la revue native.
8. **L11** — différé (faible valeur à petit volume).

## 4. Critères de sortie par lot (communs)

- Contrat écrit (routes/CLI, erreurs, persistance, idempotence, droits).
- Réglages Admin pour toute politique ; FR/EN ; deux thèmes ; mobile ; clavier.
- Tests natifs collectés sans filtre inattendu ; recette permanente PASS sur
  instantané figé ; inspection visuelle ; documentation FR/EN.
- Revue indépendante (autre contexte) avant acceptation ; aucune acceptation
  revendiquée par l'auteur.

## 5. Spécification succincte du lot prioritaire L2 (transcript typé)

| ID | Quand / étant donné | Le système doit | Vérification |
|---|---|---|---|
| TR-R1 | flux fournisseur parsé (`agent_native_stream.rs`) | produire des entrées typées (`model_call`, `tool` avec durée/échec, `tool_rejected`, `assistant_message`, `note` info/warning/error) | tests sur flux Claude et Codex enregistrés |
| TR-R2 | contenu de prompt/réponse | ne stocker que longueurs et métadonnées par défaut | test |
| TR-R3 | console harnais | frise et liste par type, filtre par type, durée par outil | Chrome |
| TR-R4 | entrée inconnue | entrée `note` « type non reconnu » comptée, sans perte du journal brut | test |

Stockage : à décider (table dédiée dans `.swarm/telemetry.db` ou dérivation à la
lecture depuis `agent_logs`, recommandée en premier pour ne rien persister de plus).

## 6. Inconnues

Événements `codex.*` exhaustifs ; seau `reasoning` ; licences exactes de
`sdks/go`, `pkg/*` ; comportement des fournisseurs à travers les versions.
