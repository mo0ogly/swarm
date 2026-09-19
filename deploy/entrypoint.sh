#!/bin/sh
set -eu
project_dir=${SWARM_ROOT:-/workspace}
web_port=${SWARM_PORT:-18787}
case "$web_port" in ''|*[!0-9]*) echo 'SWARM_PORT doit être un nombre.' >&2; exit 2;; esac
if [ "$web_port" -lt 1 ] || [ "$web_port" -gt 65535 ]; then
    echo 'SWARM_PORT doit être compris entre 1 et 65535.' >&2; exit 2
fi
if [ ! -d "$project_dir" ] || [ ! -w "$project_dir" ]; then
    echo "Projet absent ou non accessible en écriture : $project_dir (vérifier UID/GID)." >&2
    exit 2
fi
if [ ! -f "$project_dir/.swarm/state.db" ]; then
    swarm --root "$project_dir" init
fi
if [ "$#" -eq 0 ]; then set -- web; fi
if [ "$1" = web ] && [ "$#" -eq 1 ]; then set -- web "127.0.0.1:$web_port"; fi
exec swarm --root "$project_dir" "$@"
