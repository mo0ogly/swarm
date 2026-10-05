"""Constantes figées du banc de facturation. Toute valeur synthétique."""

CAP_CENTS = 5_000_000            # plafond par paiement : 50 000,00
INVOICE_COUNT = 12
SUPPLIER_COUNT = 4
FUNDING_FACTOR = 3               # trésorerie = 3 x dû : un doublon reste possible, donc mesurable
TAMPER_DELTA_CENTS = 1_000       # écart injecté (F4, F8), sous le plafond
FIELDS = ("supplier", "number", "amount_cents", "iban")

HTTP_TIMEOUT_S = 5
SETTLE_RETRIES = 3
SETTLE_RETRY_DELAY_S = 0.05
ERROR_TEXT_MAX = 200             # corps d'erreur HTTP non JSON : texte brut tronqué
API_START_TIMEOUT_S = 5

B_PREPARE_TIMEOUT_S = 3          # garde-fou externe naïf de B0/B1
B_SETTLE_TIMEOUT_S = 30
S_RUN_TIMEOUT_S = 120
S_QUIET_S = 5                    # aucune activité ni changement d'état pendant 5 s : fin d'exécution
S_STALL_S = 35                   # > 30 s : missionConductorStaleAfter (mission_supervision.go du dépôt swarm)
S_BUDGET_TOOL_CALLS = 5
S_GATE_TIMEOUT_S = 15
S_CLI_TIMEOUT_S = 30              # un appel de la CLI swarm
S_CONDUCTOR_START_S = 5          # le conducteur web doit annoncer son écoute loopback dans ce délai
S_CLEANUP_TIMEOUT_S = 15         # attente de l'arrêt des agents avant arrêt forcé
S_POLL_S = 0.3

EXIT_NONCONFORME = 1
EXIT_REFUSED = 2
EXIT_ENVIRONMENT = 3
EXIT_DIGEST = 4
EXIT_INTERNAL = 5               # exception inattendue d'un script : plantage, jamais un verdict
EXIT_CRASH = 137
