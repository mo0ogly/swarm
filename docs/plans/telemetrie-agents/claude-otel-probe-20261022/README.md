# Sonde OTLP de Claude Code, traces comprises (22 octobre 2026)

**But** : connaître la forme réelle des traces exportées par Claude Code avant
de les activer dans Swarm, sans appel payant.

**Méthode** (`probe.py`) : le vrai CLI `claude` (2.1.280) en mode `-p`, avec
`HOME` et `CLAUDE_CONFIG_DIR` temporaires (la configuration de l'utilisateur
n'est jamais lue), une clé factice et `ANTHROPIC_BASE_URL` pointant vers un
faux modèle local (API Messages en SSE). Le même serveur local reçoit l'export
OTLP/JSON. Le faux modèle demande un appel `Bash` (`echo swarm-probe`), puis
répond. Aucun compte, aucun réseau externe, aucune facturation.

**Résultat observé** (une exécution, 2 appels modèle, 1 export de traces) :

| Span | Parent | Attributs utiles |
|---|---|---|
| `claude_code.interaction` | racine | `user_prompt` = `<REDACTED>`, `user_prompt_length`, `interaction.duration_ms` |
| `claude_code.llm_request` | interaction | `model`, `input_tokens`, `output_tokens`, `cache_read_tokens`, `cache_creation_tokens`, `ttft_ms`, `stop_reason`, `success`, `attempt` |
| `claude_code.tool` | interaction | `tool_name`, `bash_command_class`, `bash_argv0` (nom du programme) |
| `claude_code.tool.execution` | tool | `success`, `duration_ms` |
| `claude_code.tool.blocked_on_user` | tool | `decision`, `source` |

Attributs communs : `user.id` (empreinte), `session.id`, `terminal.type`.
Ressource : `service.name`, `service.version`, `os.*`, `host.arch`. Le texte
du prompt et la commande (`echo swarm-probe`) n'apparaissent dans aucun export
(vérifié par recherche dans les corps bruts). Journaux et métriques sont émis
comme déjà connu (`api_request`, `tool_result`, `tool_decision`,
`user_prompt`, `assistant_response`, `managed_settings_resolved` ;
`session.count`, `cost.usage`, `token.usage`, `active_time.total`).

**Décision appliquée dans Swarm** : au lancement d'un agent `claude` instrumenté,
`OTEL_TRACES_EXPORTER=otlp`, `CLAUDE_CODE_ENHANCED_TELEMETRY_BETA=1` et
`OTEL_LOG_USER_PROMPTS=0` (explicite) s'ajoutent. Les cinq spans ci-dessus sont
gardés avec une liste blanche d'attributs ; `user_prompt` (même masqué) et
`bash_argv0` (peut être un chemin privé) sont retirés ; tout autre span est
compté « span (other) ».

**Limites** : une seule version de Claude Code observée ; fonction « beta »
dont la forme peut changer (les noms inconnus restent comptés, jamais
stockés) ; mode interactif non sondé.

Relancer : `python3 probe.py <dossier-neuf>` (nécessite `claude` sur le PATH).
