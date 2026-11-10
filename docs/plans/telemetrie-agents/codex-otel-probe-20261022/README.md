# Sonde OTLP de Codex — 22 octobre 2026

Objet : établir, avant d'instrumenter les lancements Codex (LIVE-UX-03, A.2),
comment `codex-cli 0.163.0` exporte sa télémétrie. Aucun appel payant : faux
fournisseur de modèle local (`fake-model-and-otlp-sink.py`, port 47311, réponses
SSE `responses` simulées), `CODEX_HOME` temporaire (`config.toml` ci-contre), le
`~/.codex` de l'utilisateur n'est pas lu. Le faux modèle demande une fois
`exec_command` avec `echo swarm-probe` pour observer `tool_result`.

## Faits observés (reproductibles avec les fichiers de ce dossier)

| # | Essai | Résultat |
|---|---|---|
| A | en-tête `Authorization = "Bearer ${SWARM_OTLP_TOKEN}"` dans `[otel]` | reçu **littéralement** `Bearer ${SWARM_OTLP_TOKEN}` : aucune expansion |
| B | `[otel]` sans `headers` + `OTEL_EXPORTER_OTLP_HEADERS=Authorization=Bearer …` | en-tête **repris de l'environnement** |
| C | aucune section `[otel]`, seulement `OTEL_*` (endpoint, protocole, exporteurs) | **aucun export** |
| D | `codex -c 'otel.exporter={otlp-http={endpoint=".../v1/logs",protocol="json"}}' -c 'otel.metrics_exporter=…' -c 'otel.log_user_prompt=false' exec -` (prompt sur stdin) + en-tête par l'environnement | logs et métriques exportés en JSON avec le jeton ; `-c` accepté **avant** la sous-commande |
| E | session trop courte (erreur immédiate du modèle) | export par lots parfois absent à la sortie : il faut une session réelle pour l'observer |

Forme des journaux : nom dans l'attribut `event.name` (`codex.conversation_starts`,
`codex.startup_phase`, `codex.user_prompt`, `codex.api_request`,
`codex.sse_event`, `codex.tool_decision`, `codex.tool_result`,
`codex.turn_ttft`) ; `timeUnixNano = "0"`, heure dans `observedTimeUnixNano` ;
valeurs en chaînes. `codex.api_request` = tentative HTTP (durée, statut,
`attempt`), **sans jetons** ; les jetons d'un appel sont sur `codex.sse_event`
`event.kind = response.completed` (`input_token_count`, `cached_token_count`,
`cache_write_token_count`, `output_token_count`, `reasoning_token_count`).
`user_prompt.prompt` vaut `[REDACTED]`, mais **`tool_result` transporte
`arguments` et `output` en clair** (contrairement à la documentation) : la
liste blanche du normaliseur les écarte (test
`codex_completed_response_is_the_call_and_content_never_kept`). Métriques :
nombreuses séries internes (`codex.sqlite.*`, `codex.skills.*`…), plus
`codex.api_request`, `codex.turn.token_usage` (histogramme par `token_type`),
`codex.conversation.turn.count`.

## Conséquences pour Swarm

Jeton par `OTEL_EXPORTER_OTLP_HEADERS` (environnement, jamais `argv`) ;
exporteurs par arguments globaux `-c otel.…` insérés après le programme ;
`log_user_prompt=false` imposé ; réponse `response.completed` → appel
`api_request` (entrée non cachée = entrée − cache, la convention OpenAI incluant
le cache dans l'entrée — hypothèse documentaire, non mesurée par la sonde) ;
`codex.api_request` → `api_attempt` ; autres `sse_event` comptés, non stockés.
Limites : une seule version de Codex observée ; histogrammes non stockés
(rejetés et comptés) ; aucune exécution réelle facturée.
