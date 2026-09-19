#!/usr/bin/env bash
# Installer from a reviewed checkout. No sudo, shell-profile edits or remote pipe.
set -euo pipefail
repo_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
mode=docker
project_dir=
port=18787
bin_dir=${SWARM_BIN_DIR:-${HOME}/.local/bin}
agent_dir=${SWARM_AGENT_HOME:-${HOME}/.local/share/swarm/agents}
start=yes
usage() {
    cat <<'HELP'
Installation de Swarm depuis ce dépôt (Linux).

  ./install.sh --project /chemin/projet [--port 18787] [--no-start]
  ./install.sh --mode native [--bin-dir ~/.local/bin] [--project /chemin/projet]

Docker est le mode par défaut. --no-start construit sans démarrer le cockpit.
Le mode native compile avec Go 1.24+ et installe le binaire sans lancer de serveur.
--agent-home DOSSIER choisit le stockage persistant des outils et connexions agents.
Les fournisseurs IA s'installent et s'authentifient séparément. Voir INSTALL.md.
HELP
}
fail() { printf 'Erreur : %s\n' "$*" >&2; exit 2; }
need_value() { [ "$#" -ge 2 ] && [ -n "$2" ] || fail "Valeur manquante pour $1"; }
while [ "$#" -gt 0 ]; do
    case "$1" in
        --mode) need_value "$@"; mode=$2; shift 2;;
        --project) need_value "$@"; project_dir=$2; shift 2;;
        --port) need_value "$@"; port=$2; shift 2;;
        --bin-dir) need_value "$@"; bin_dir=$2; shift 2;;
        --agent-home) need_value "$@"; agent_dir=$2; shift 2;;
        --no-start) start=no; shift;;
        --help|-h) usage; exit 0;;
        *) fail "Option inconnue : $1";;
    esac
done
case "$mode" in docker|native) ;; *) fail '--mode attend docker ou native';; esac
[[ "$port" =~ ^[0-9]{1,5}$ ]] || fail 'Port invalide (1 à 65535).'
port=$((10#$port))
[ "$port" -ge 1 ] && [ "$port" -le 65535 ] || fail 'Port invalide (1 à 65535).'
[ "$(uname -s)" = Linux ] || fail 'Cet installateur est validé sur Linux uniquement. Voir INSTALL.md.'
if [ -n "$project_dir" ]; then
    [ -d "$project_dir" ] || fail "Créez d’abord le dossier du projet : $project_dir"
    project_dir=$(cd -- "$project_dir" && pwd -P)
    [ -w "$project_dir" ] || fail "Projet non accessible en écriture : $project_dir"
fi
if [ "$mode" = native ]; then
    command -v go >/dev/null || fail 'Go 1.24+ est requis pour le mode native.'
    mkdir -p -- "$bin_dir"
    bin_dir=$(cd -- "$bin_dir" && pwd -P)
    stage_file=$(mktemp "$bin_dir/.swarm-install.XXXXXXXX")
    trap 'rm -f -- "${stage_file:-}"' EXIT
    (cd "$repo_dir" && CGO_ENABLED=0 go build -trimpath -o "$stage_file" .)
    chmod 755 "$stage_file"
    mv -f -- "$stage_file" "$bin_dir/swarm"
    if [ -n "$project_dir" ]; then "$bin_dir/swarm" --root "$project_dir" init; fi
    printf 'Binaire installé : %s/swarm\n' "$bin_dir"
    printf 'Lancer : %q --root %q web 127.0.0.1:%s\n' "$bin_dir/swarm" "${project_dir:-/chemin/du/projet}" "$port"
    printf 'Si nécessaire, ajoutez %s au PATH. Aucun profil shell modifié.\n' "$bin_dir"
    exit 0
fi
[ -n "$project_dir" ] || fail 'Le mode Docker demande --project /chemin/projet.'
command -v docker >/dev/null || fail 'Installez Docker Engine et Docker Compose v2. Voir INSTALL.md.'
docker info >/dev/null 2>&1 || fail 'Docker est inaccessible. Vérifiez le moteur et vos permissions.'
docker compose version >/dev/null 2>&1 || fail 'Docker Compose v2 est requis.'
mkdir -p -- "$agent_dir"
agent_dir=$(cd -- "$agent_dir" && pwd -P)
[ -w "$agent_dir" ] || fail "Dossier agents non accessible en écriture : $agent_dir"
config_file="$repo_dir/deploy/install.env"
compose=(docker compose --project-directory "$repo_dir" --env-file "$config_file" -f "$repo_dir/compose.yaml")
if [ -f "$config_file" ] && [ -n "$("${compose[@]}" ps --status running -q swarm)" ]; then
    fail 'Swarm Docker fonctionne déjà. Arrêtez les missions et le conteneur avant une mise à jour. Voir INSTALL.md.'
fi
# Single-quoted Compose values preserve spaces and dollars. Reject ambiguous input.
for value in "$project_dir" "$agent_dir"; do
    case "$value" in *"'"*|*\\*|*$'\n'*|*$'\r'*) fail 'Chemin incompatible avec le fichier Compose (apostrophe, antislash ou retour à la ligne).';; esac
done
umask 077
config_tmp=$(mktemp "$repo_dir/deploy/.install-env.XXXXXXXX")
trap 'rm -f -- "${config_tmp:-}"' EXIT
printf "SWARM_PROJECT='%s'\nSWARM_AGENT_HOME='%s'\nSWARM_PORT=%s\nSWARM_UID=%s\nSWARM_GID=%s\n" "$project_dir" "$agent_dir" "$port" "$(id -u)" "$(id -g)" > "$config_tmp"
mv -- "$config_tmp" "$config_file"
"${compose[@]}" config --quiet
"${compose[@]}" build
if [ "$start" = yes ]; then
    if ! "${compose[@]}" up -d --wait --wait-timeout 60; then
        "${compose[@]}" logs --no-color --tail 20 swarm >&2
        fail "Le cockpit n’a pas démarré. Vérifiez le port, les droits et INSTALL.md."
    fi
    printf '\nLien de session (les journaux suivants restent privés) :\n'
    "${compose[@]}" logs --no-color --tail 15 swarm
else
    printf 'Image construite. Démarrer depuis ce dépôt :\n  docker compose --env-file deploy/install.env up -d\n'
fi
printf '\nGuide : %s/INSTALL.md\n' "$repo_dir"
