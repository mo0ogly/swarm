#!/usr/bin/env bash
# Install the Swarm cockpit authority daemon under its own system account.
#
# Run with sudo from a reviewed checkout, after building the binary as your
# normal user:   (cd rust && cargo build --release -p swarm-authority)
#
#   sudo deploy/authority/install.sh --admin LOGIN --display "Full Name" [--dry-run]
#   sudo deploy/authority/install.sh --uninstall [--purge]
#
# What it does (idempotent; accounts, key, pin and first user are never
# replaced once created; the binary and unit are refreshed and a running
# service is restarted):
#   1. system account `swarm-authority` (no login shell, no home creation)
#   2. root-owned binary /usr/local/lib/swarm-authority/swarm-authority
#   3. private state /var/lib/swarm-authority (0700, owned by the account)
#   4. `prepare` as the account: signing seed + declaration pinning the
#      daemon key and the first admin; the pin is stored root-owned in
#      /etc/swarm-authority/pin
#   5. hardened systemd unit (not started: see the printed next steps)
#   6. `init` as the account: creates the FIRST USER (admin) — the password
#      is asked twice on your terminal, never passed as an argument
# The engine must then switch to `authority` mode on an internal port before
# the service is enabled (printed at the end). No other system change.
set -euo pipefail
# Name the exact failing command (coreutils messages can omit the path).
trap 'printf "Erreur ligne %s : %s\n" "$LINENO" "$BASH_COMMAND" >&2' ERR

LIB_DIR=/usr/local/lib/swarm-authority
STATE=/var/lib/swarm-authority
ETC=/etc/swarm-authority
UNIT=/etc/systemd/system/swarm-authority.service
ACCOUNT=swarm-authority
repo=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)
binary="$repo/rust/target/release/swarm-authority"
listen=127.0.0.1:38433
engine=127.0.0.1:38434
admin='' display='' dry=no uninstall=no purge=no

fail() { printf 'Erreur : %s\n' "$*" >&2; exit 2; }
run() { if [ "$dry" = yes ]; then printf '[dry-run]'; printf ' %q' "$@"; printf '\n'; else "$@"; fi; }
need() { [ "$#" -ge 2 ] && [ -n "$2" ] || fail "valeur manquante pour $1"; }
# Write CONTENT (plus a newline) to a root-owned file with MODE, atomically:
# temporary file in the target directory, owner/mode set, then rename. No
# copy from /dev/stdin or /dev/null: coreutils `install` (uutils, under
# sudo-rs) refused /dev/stdin as a source ("No such file or directory").
write_root() {
    local mode=$1 target=$2 content=$3 tmp
    tmp=$(mktemp "$(dirname -- "$target")/.swarm-authority.XXXXXX")
    if printf '%s\n' "$content" >"$tmp" && chown root:root "$tmp" && chmod "$mode" "$tmp" && mv -f -- "$tmp" "$target"; then
        return 0
    fi
    rm -f -- "$tmp"
    fail "écriture refusée : $target"
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --admin) need "$@"; admin=$2; shift 2;;
        --display) need "$@"; display=$2; shift 2;;
        --listen) need "$@"; listen=$2; shift 2;;
        --engine) need "$@"; engine=$2; shift 2;;
        --binary) need "$@"; binary=$2; shift 2;;
        --dry-run) dry=yes; shift;;
        --uninstall) uninstall=yes; shift;;
        --purge) purge=yes; shift;;
        -h|--help) sed -n '2,22p' "$0"; exit 0;;
        *) fail "option inconnue : $1";;
    esac
done
[ "$dry" = yes ] || [ "$(id -u)" -eq 0 ] || fail "lancer avec sudo (ou --dry-run)"
cd /

if [ "$uninstall" = yes ]; then
    run systemctl disable --now swarm-authority.service || true
    run rm -f "$UNIT"
    run systemctl daemon-reload
    run rm -rf "$LIB_DIR"
    if [ "$purge" = yes ]; then
        printf 'Suppression des comptes, sessions et clés de %s (irréversible).\n' "$STATE"
        run rm -rf "$STATE" "$ETC"
        run userdel "$ACCOUNT" || true
    else
        printf 'Données conservées dans %s ; --purge pour les supprimer.\n' "$STATE"
    fi
    exit 0
fi

[[ "$admin" =~ ^[A-Za-z0-9_-]{1,100}$ ]] || fail "--admin LOGIN requis (lettres, chiffres, - ou _)"
[ -n "$display" ] || fail "--display requis"
[[ "$listen" =~ ^127\.0\.0\.1:[0-9]{1,5}$ ]] || fail "--listen doit être 127.0.0.1:PORT"
[[ "$engine" =~ ^127\.0\.0\.1:[0-9]{1,5}$ ]] || fail "--engine doit être 127.0.0.1:PORT"
[ "$listen" != "$engine" ] || fail "--listen et --engine doivent différer"
[ -x "$binary" ] || fail "binaire absent : $binary (cargo build --release -p swarm-authority)"
controller=${SUDO_UID:-}
[ -n "$controller" ] || [ "$dry" = yes ] || fail "SUDO_UID absent : lancer via sudo depuis votre compte"
controller=${controller:-$(id -u)}

# 1. account
if ! id "$ACCOUNT" >/dev/null 2>&1; then
    run useradd --system --home-dir "$STATE" --no-create-home --shell /usr/sbin/nologin "$ACCOUNT"
fi
# 2. root-owned binary (the account cannot replace it)
run install -d -o root -g root -m 0755 "$LIB_DIR"
run install -o root -g root -m 0755 "$binary" "$LIB_DIR/swarm-authority"
# 3. private state
run install -d -o "$ACCOUNT" -g "$ACCOUNT" -m 0700 "$STATE"
run install -d -o root -g root -m 0755 "$ETC"
# 4. prepare (once)
as_account() { run runuser -u "$ACCOUNT" -- "$LIB_DIR/swarm-authority" "$@"; }
declaration="$STATE/bootstrap/declaration.json"
if [ "$dry" = yes ]; then
    as_account prepare --dir "$STATE" --controller-uid "$controller" --admin "$admin" --json
    pin=DRY-RUN-PIN public_key=DRY-RUN-KEY
elif [ -f "$ETC/pin" ]; then
    pin=$(cat "$ETC/pin")
    public_key=$(cat "$ETC/public_key")
else
    out=$(runuser -u "$ACCOUNT" -- "$LIB_DIR/swarm-authority" prepare --dir "$STATE" --controller-uid "$controller" --admin "$admin" --json)
    pin=$(printf '%s' "$out" | python3 -c 'import json,sys;print(json.load(sys.stdin)["pin"])')
    public_key=$(printf '%s' "$out" | python3 -c 'import json,sys;print(json.load(sys.stdin)["public_key"])')
    write_root 0644 "$ETC/pin" "$pin"
    write_root 0644 "$ETC/public_key" "$public_key"
fi
# 5. hardened unit (root-owned; carries the pin)
unit_text="[Unit]
Description=Swarm cockpit authority (sign-in, sessions, signed relay)
After=network.target

[Service]
User=$ACCOUNT
Group=$ACCOUNT
ExecStart=$LIB_DIR/swarm-authority serve --declaration $declaration --pin $pin --listen $listen --engine $engine
Restart=on-failure
UMask=0077
NoNewPrivileges=yes
ProtectSystem=strict
ReadWritePaths=$STATE
ProtectHome=yes
PrivateTmp=yes
PrivateDevices=yes
ProtectKernelTunables=yes
ProtectKernelModules=yes
ProtectControlGroups=yes
RestrictAddressFamilies=AF_UNIX AF_INET
IPAddressDeny=any
IPAddressAllow=localhost
CapabilityBoundingSet=
LockPersonality=yes
MemoryDenyWriteExecute=yes
RestrictNamespaces=yes
SystemCallArchitectures=native

[Install]
WantedBy=multi-user.target
"
if [ "$dry" = yes ]; then
    printf '[dry-run] write %s:\n%s\n' "$UNIT" "$unit_text"
else
    write_root 0644 "$UNIT" "$unit_text"
fi
run systemctl daemon-reload
# Reinstall: a running service picks up the new binary and policy now
# (try-restart does nothing when the service is stopped).
run systemctl try-restart swarm-authority.service
# 6. first user (password asked on the terminal)
if [ "$dry" = yes ]; then
    as_account init --declaration "$declaration" --pin "$pin" --admin "$admin" --display "$display" --json
else
    if [ -f "$ETC/initialized" ]; then
        printf 'Déjà initialisé : premier utilisateur existant conservé.\n'
    else
        # Prompts go to the terminal (stderr); the password never hits argv.
        printf 'Création du premier utilisateur « %s » :\n' "$admin"
        runuser -u "$ACCOUNT" -- "$LIB_DIR/swarm-authority" init --declaration "$declaration" --pin "$pin" --admin "$admin" --display "$display" --json \
            || fail "init refusé (voir le code ci-dessus)"
        write_root 0644 "$ETC/initialized" ""
    fi
fi

cat <<EOF

Installé (service NON démarré). Suite, depuis votre compte (pas sudo) :
  ./swarm.sh authority enable
  (règle le moteur en mode authority avec la clé $public_key,
   le déplace sur $engine, démarre le démon via sudo et ouvre
   http://$listen/login ; connexion avec « $admin »).
Retour arrière : ./swarm.sh authority disable
EOF
