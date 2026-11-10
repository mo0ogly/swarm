# LIVE-UX-03 — Télémétrie native des agents (récepteur OTLP) — spécification

## Raccord documentaire APEX — 24 octobre 2026

La [file APEX FR](../FILE-APEX.md#vague-2--rapprochement-des-suivis-antérieurs-24-octobre-2026)
et [EN](../../en/APEX-QUEUE.md#wave-2--reconciling-earlier-tracking-october-24-2026)
relie LIVE-UX-03 à la feuille de route LangWatch : L1 désigne le même raccord,
pas une seconde réalisation. Le statut ci-dessous conserve la chronologie
E2E-1 puis E2E-2 : la mention « non réalisé » de T2/T5 appartient à l’état
antérieur à E2E-2, et ne doit pas effacer les résultats locaux qui suivent.
Reviewer/planification et la revue indépendante restent ouverts selon ce suivi.
Codex racine conserve la coordination documentaire ; cette précision ne lance
aucun travail, ne requalifie aucun consommateur et n’autorise pas le juge L4.

Statut : **proposé ; incrément E2E-1 implémenté et vérifié localement le 21
octobre 2026** (demande utilisateur APEX) avec D2 = (a) base séparée et D3 = (a)
listener existant. Réalisé : TEL-T1 (sauf rétention appliquée), TEL-T3, TEL-T4,
TEL-T6 partiel, TEL-T7 (étapes de `live-monitor-native`), TEL-T8. Non réalisé :
TEL-T2 et TEL-T5 réalisés en E2E-2 (même jour) pour Claude Code et les workers :
réglage v30 `agent_telemetry_providers` (liste au lieu du booléen R1 : aucune unité
booléenne au catalogue), endpoint publié par le serveur web. Restent ouverts :
reviewer/planification. Codex instrumenté le 22 octobre (sonde `codex-otel-probe-20261022/`, arguments `-c otel.…` + jeton par l'environnement). R6 (plafond par tentative) et R11 (rétention appliquée) réalisés le 22 octobre (réglages v33, store console v4). Écart R3 : jeton vérifié avant **décodage** ;
avant **lecture** seuls les en-têtes le sont (corps borné). Détails :
`docs/architecture/RUST-MIGRATION.md` § LIVE-UX-03.
Auteur : Claude (session directe, 21 octobre 2026). Revue : auto-revue seulement.
Dépend de : LIVE-UX-01/02 (console live, `docs/plans/migration-rust/PLAN.md`).
Feuille de route associée : [langwatch-feuille-de-route](../langwatch-feuille-de-route/SPEC.md).

## 1. Problème, résultat attendu, exclusions

**Problème.** La console live ne connaît que l'usage **cumulé** déclaré à la fin
d'un tour (`agent_provider_facts.rs:161-251` : « dernier événement terminal
valide… sans somme ni cumul »). Elle ne sait pas répondre à « où sont partis les
jetons ? », « quel outil a duré ou échoué ? », « combien de lignes modifiées,
quelles éditions refusées ? », « temps actif vs attente ».

**Résultat.** Claude Code et Codex exportent nativement ces faits en
OpenTelemetry. Swarm reçoit cette télémétrie **localement**, la rattache sans
confiance implicite à la tentative Swarm qui l'a produite, et l'affiche : jetons
et coût **par appel**, cascade d'outils (durée, échec), lignes modifiées,
décisions d'édition, temps actif.

**Exclusions v1.** Traces/spans (Claude Code ne les émet qu'avec un drapeau bêta),
protobuf, contenu des prompts/réponses/outils, envoi vers un service tiers,
alertes, évaluations, toute modification du schéma de la base mission partagée
avec Go, tout changement du chemin critique de la migration.

## 2. Faits établis (sources)

Swarm (exploration du 21 octobre) :
- Aucun code OTel/OTLP dans `rust/crates`.
- Lancement fournisseur : `PreparedExec::prepare`
  (`swarm-engine/src/native_provider_exec.rs:42-107`) ; env = **liste de noms**
  autorisés copiés depuis l'env du moteur (:61-73), valeurs jamais injectées ;
  `execve` sans shell ni PATH (:123-136).
- Liste autorisée = réglage `agent_environment_allow` (`engine_settings.rs:376`,
  validé :1394-1440) + `env_allow` par fournisseur (`agent_models.rs:538`),
  fusionnés dans `agent_launch_preflight.rs:103-139` ; seuls des digests SHA-256
  des valeurs sont persistés ; figée dans `agent.env_allow`
  (`agent_launch.rs:829`) ; refus à l'exécution si dérive
  (`agent_execution_preflight.rs:352`).
- Web : cookie de session + Host exact, et Origin + `X-Swarm-CSRF` hors GET
  (`swarm-web/src/access.rs` ~:139-171) ; **aucune voie d'authentification
  machine**. Corps borné par `web_http_body_max_bytes`
  (`service/transport.rs:276-300`), authentification sur l'en-tête avant lecture
  du corps (:224-244).
- Réglages : valeurs `config/engine-settings.json` (chaînes), catalogue
  `config/engine-settings-catalog.json` (nom, libellé FR, unité, nullable, aide FR,
  `introduced_schema_version`), validateur Rust, anglais dans `locales/en.json`,
  version de document 29 actuellement (`engine_settings.rs:1843`).
- La base mission est au schéma 37, commun avec Go (`initial_schema_37.sql`) ;
  la migration interdit le double écrivain et vise une bascule unique
  (`PLAN.md:5-7, 820-822`).

Fournisseurs (lecture LangWatch, Apache-2.0, `docs/coding-agents/*.mdx`,
`modules/coding-agent/contract/src/telemetry/*.ts`) :
- **Claude Code** : `CLAUDE_CODE_ENABLE_TELEMETRY=1`,
  `OTEL_{METRICS,LOGS}_EXPORTER=otlp`, `OTEL_EXPORTER_OTLP_PROTOCOL=http/json`,
  `OTEL_EXPORTER_OTLP_ENDPOINT`, `OTEL_EXPORTER_OTLP_HEADERS="Authorization=Bearer …"`,
  `OTEL_RESOURCE_ATTRIBUTES`. Spans seulement avec
  `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1`. Contenu opt-in :
  `OTEL_LOG_USER_PROMPTS`, `OTEL_LOG_TOOL_DETAILS`, `OTEL_LOG_TOOL_CONTENT`,
  `OTEL_LOG_RAW_API_BODIES`, `OTEL_LOG_ASSISTANT_RESPONSES`.
- **Codex** : section `[otel]` de `config.toml`, exporteurs
  `otel.trace_exporter.otlp-http` et `otel.exporter.otlp-http`, protocole `json`,
  **chemins complets** `/v1/traces`, `/v1/logs` (sans quoi 404) ; aucun contenu
  (longueurs seulement).
- Noms normalisés (préfixes `claude_code.`, `codex.` retirés) — métriques :
  `token.usage`, `cost.usage`, `lines_of_code.count`, `commit.count`,
  `pull_request.count`, `code_edit_tool.decision`, `active_time.total`,
  `tool.call.count`, Codex `turn.token_usage` ; événements : `user_prompt`,
  `api_request`, `api_response`, `api_error`, `api_refusal`, `rate_limit*`,
  `tool_result`, `tool_decision`, `compaction`, `session_created`,
  `session_idle`, `turn_ttft`… ; seaux de jetons : input, output, cache_read,
  cache_creation, reasoning ; clé de session : `session.id`, `conversation.id`,
  `gen_ai.conversation.id`, `thread.id` (Codex : `thread.id` = session).
- Ingestion LangWatch : `POST /api/otel/v1/{traces,logs,metrics}`, JSON et
  protobuf, `partialSuccess` pour les rejets permanents, 503 pour les échecs
  internes, points immuables identifiés par sha256 (ADR-055).

Inconnues : liste exhaustive des événements `codex.*` ; quels modèles alimentent
le seau `reasoning` ; stabilité des noms entre versions fournisseurs ; présence
d'une dépendance de décompression déjà verrouillée dans `rust/Cargo.lock`.

## 3. Analyse et décisions

**D1 — Injection de la configuration télémétrie dans le processus fournisseur.**
- (a) Nouveau champ « env injecté par le moteur » (nom → valeur calculée par
  tentative : endpoint, en-tête, attributs). Impacte le contrat digest/fraîcheur
  (`agent_execution_preflight.rs:352`) : les valeurs injectées doivent être
  exclues du digest d'env hérité et tracées à part (digest du jeton seulement).
- (b) Fichier de configuration fournisseur généré par tentative (Codex
  `CODEX_HOME` dédié contenant `config.toml` ; Claude Code `settings.json`
  `env`). Évite de toucher l'env du moteur mais duplique l'état de configuration
  du fournisseur et dépend de son format.
- **Proposition : (a)**, avec une liste fermée de noms injectables (`OTEL_*`,
  `CLAUDE_CODE_ENABLE_TELEMETRY`) ; pour Codex, (a) ne suffit pas (config
  fichier) → (b) limité à Codex, dans le workspace de tentative, supprimé à la fin.
- Menace nommée : le jeton est lisible dans `/proc/PID/environ` par le même
  utilisateur Unix. Acceptable car il n'autorise que l'écriture de télémétrie
  pour **cette** tentative et expire avec elle ; jamais réutilisé pour lire.

**D2 — Stockage.**
- (a) Base séparée `.swarm/telemetry.db` (schéma propre versionné).
- (b) Nouvelles tables dans la base mission (schéma 38).
- **Proposition : (a)** : ne modifie pas le schéma 37 commun avec Go, compatible
  bascule unique et retour arrière ; rétention indépendante ; corruption isolée.

**D3 — Authentification machine.**
- (a) Jeton d'ingestion **par tentative** sur le listener existant, route
  dédiée exemptée de cookie/CSRF mais exigeant `Authorization: Bearer`.
- (b) Second listener loopback dédié, port éphémère, même jeton.
- **Proposition : (a)** si l'exemption est strictement limitée aux routes
  `/api/otel/v1/*` et vérifiée avant lecture du corps ; (b) en repli si la revue
  juge l'exemption trop risquée pour le listener humain.

**D4 — Ordre.** Lot UX indépendant (n'altère ni schéma mission ni chemin
critique) **ou** après la bascule R7. Décision utilisateur ouverte.

## 4. Exigences

| ID | Quand / étant donné | Le système doit | Vérification | Reprise |
|---|---|---|---|---|
| TEL-R1 | réglage `agent_telemetry_enabled` faux (défaut) | ne rien injecter, ne pas ouvrir la route, afficher « télémétrie désactivée » | test engine + UI | activer via Admin, prise en compte au prochain lancement |
| TEL-R2 | lancement d'une tentative avec télémétrie active | générer un jeton aléatoire par tentative, persister son seul digest, injecter endpoint/en-tête/attribut `swarm.agent_id` | test preflight : jeton absent de toute persistance/journal | jeton invalide à la fin de la tentative |
| TEL-R3 | `POST /api/otel/v1/metrics` ou `/logs` JSON | authentifier par jeton avant lecture du corps ; rattacher à l'agent **du jeton**, jamais aux attributs déclarés | test TCP : jeton d'un agent A + attribut agent B → rattaché à A | rejet 401 code `telemetry-token-invalid` |
| TEL-R4 | jeton absent, expiré, d'une tentative terminée ou révoquée | refuser 401 sans lire le corps ni divulguer l'existence de l'agent | test TCP | aucun |
| TEL-R5 | corps protobuf ou `Content-Encoding` non pris en charge | refuser 415 explicite (`telemetry-format-unsupported`) | test | décision D-proto en v2 |
| TEL-R6 | corps au-delà de `web_http_body_max_bytes` ou plafond d'événements par tentative atteint | refuser 413 / `partialSuccess` avec compte rejeté ; ne jamais tronquer silencieusement | test | plafond réglable Admin |
| TEL-R7 | point déjà reçu (même empreinte sha256) | idempotence : aucun doublon | test rejeu | — |
| TEL-R8 | échec interne d'écriture | 503 (le client OTel réessaie) ; aucune perte silencieuse ; erreur journalisée sans contenu | injection de faute | rejeu client |
| TEL-R9 | attributs ou corps contenant prompt/réponse/contenu d'outil | les ignorer par défaut ; ne stocker que les champs de la liste blanche (noms, compteurs, durées, codes) | test avec contenu factice | réglage explicite pour l'opt-in, hors v1 |
| TEL-R10 | lecture console d'un agent | exposer jetons/coût **par appel depuis les événements `api_request`/`api_response`** (un événement = un appel), totaux depuis les métriques `token.usage`/`cost.usage` selon leur temporalité déclarée (cumulative : dernière valeur ; delta : somme), jamais un mélange des deux pour un même chiffre ; outils (nom, durée, échec), lignes modifiées, décisions d'édition, temps actif, **étiquetés « déclaration fournisseur non authentifiée »** | test intégration + Chrome | — |
| TEL-R11 | rétention | purger au-delà de `agent_telemetry_retention_days` ; purge observable | test horloge injectée | réglage Admin |
| TEL-R12 | version fournisseur émettant des noms inconnus | conserver le compteur « non reconnu » par nom, sans les afficher comme connus | test | mise à jour de la table de normalisation |
| TEL-R13 | redémarrage du moteur, annulation ou arrêt d'une tentative | jeton révoqué dès que la tentative n'est plus active (fin, arrêt, abandon) ; après redémarrage, seuls les jetons de tentatives encore actives et réconciliées restent valides ; points reçus avant la révocation conservés | test : arrêt pendant l'envoi, redémarrage moteur | l'exporteur fournisseur reçoit 401 et abandonne ; aucune réouverture implicite |

## 5. Contrats

- **Routes** : `POST /api/otel/v1/metrics`, `POST /api/otel/v1/logs` (JSON OTLP),
  réponses OTLP standard (`partialSuccess`) ; `GET /api/v1/live/agent` étendu
  (`telemetry` : séries par appel, outils, compteurs, état) — lecture seule.
- **Réglages Admin nouveaux** (catalogue + validateur + `en.json`, schéma 30) :
  `agent_telemetry_enabled` (booléen, défaut faux), `agent_telemetry_retention_days`
  (count), `agent_telemetry_max_events_per_attempt` (count ; null = aucun plafond
  opérateur, affiché comme tel dans l'Admin, la limite de corps HTTP restant
  appliquée). Aucune constante cachée.
- **Version de schéma des réglages** : la prochaine version libre au moment de
  l'implémentation (Codex fait évoluer la série en parallèle, v29 au 21 octobre) ;
  allocation à convenir avec Codex avant tout changement de catalogue.
- **Persistance** : `.swarm/telemetry.db` — tables `points(digest PK, agent_id,
  at_ms, name, unit, value, attrs_json_whitelist)`, `events(digest PK, agent_id,
  at_ms, name, fields_json_whitelist)`, `unknown_names(name, count)`.
- **Erreurs** : codes stables `telemetry-disabled`, `telemetry-token-invalid`,
  `telemetry-format-unsupported`, `telemetry-quota-reached`,
  `telemetry-store-unavailable`.
- **Droits** : écriture seule ; aucune lecture via le jeton ; aucune donnée
  envoyée hors loopback.

## 6. Rôles et gardes

- **Planificateur** (utilisateur, ou Codex sur délégation) : tranche D1–D4,
  ordonne le lot par rapport au chemin critique.
- **Worker** (Claude ou Codex) : TEL-T1..T8 dans son workspace ; ne s'accepte pas.
- **Relecteur indépendant** (autre contexte) : revue du code et des refus TEL-R3/R4/R9/R13
  avant acceptation.
- **Gardes moteur** : jeton et rattachement (TEL-R2/R3) = préparation du
  lancement + vérification HTTP avant lecture du corps ; liste blanche (TEL-R9) =
  normaliseur ; plafonds (TEL-R6) = validateur des réglages + compteur du store ;
  révocation (TEL-R13) = cycle de vie de la tentative dans le store agents.

## 7. États visibles (console)

Désactivé · En attente (activé, rien reçu) · Reçu (horodatage dernier point) ·
Refusé (code) · Partiel (rejets comptés) · Noms non reconnus (compte).

## 8. Tâches

| ID | Exigences | Fichiers | Livrable | Acceptation | Reprise |
|---|---|---|---|---|---|
| TEL-T1 | R7,R8,R11 | `swarm-engine/src/telemetry_store.rs` | base séparée + migrations + purge | tests unitaires (idempotence, faute disque, purge) | refus explicite si base illisible |
| TEL-T2 | R1,R2,R13 | `agent_launch_preflight.rs`, `native_provider_exec.rs` | jeton + env injecté (D1) | tests : digest seul, dérive refusée, désactivé = rien | rollback = réglage faux |
| TEL-T3 | R3–R6 | `swarm-web/src/service/otel.rs`, `access.rs`, `transport.rs` | routes + auth jeton | tests TCP réels (valide, A/B, absent, révoqué, protobuf, taille) | — |
| TEL-T4 | R9,R12 | `swarm-engine/src/telemetry_normalize.rs` | table de noms + liste blanche | tests sur échantillons JSON Claude/Codex | mise à jour table |
| TEL-T5 | R1,R6,R11 | config + catalogue + validateur + `en.json` | 3 réglages Admin | parcours Admin FR/EN | — |
| TEL-T6 | R10 | `service/live.rs`, `web/live-session.js` | vues par appel/outils/lignes | Chrome : fixture OTLP rejouée | — |
| TEL-T7 | toutes | `tools/verification/recipes/telemetry-native.json` | recette | PASS sur instantané figé | — |
| TEL-T8 | — | `RUST-MIGRATION.md` FR/EN | contrat documenté | relecture | — |

Dépendances : T1 → T3 → T6 ; T2 ∥ T4 ; T5 avant T2 ; T7 après T1–T6.

## 9. Matrice exigences → tests

R1: T2 test désactivé, T6 UI · R2: T2 digest seul · R3: T3 A/B · R4: T3 révoqué ·
R5: T3 protobuf · R6: T3 taille + plafond · R7: T1 rejeu · R8: T1 faute disque ·
R9: T4 contenu factice · R10: T6 Chrome · R11: T1 purge horloge · R12: T4 nom inconnu ·
R13: T2/T3 arrêt pendant l'envoi + redémarrage moteur.

## 10. Risques et décisions ouvertes

- D1–D4 ci-dessus (utilisateur ; revue indépendante pour D3).
- Les noms fournisseurs changent entre versions (atténuation : R12).
- Des attributs de ressource peuvent contenir des chemins ou identifiants : liste
  blanche stricte (R9).
- La télémétrie reste une **déclaration** du fournisseur : jamais utilisée pour
  des décisions de budget ou d'acceptation sans contrôle séparé.
