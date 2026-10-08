#!/bin/sh
# Canonical version-aware build used by Make, native installation and Docker.
set -eu

test "$#" -eq 1 || { printf 'usage: %s OUTPUT\n' "$0" >&2; exit 2; }
output=$1

if test "${SWARM_VERSION+x}" = x; then
    version=$SWARM_VERSION
else
    tag=$(git describe --tags --exact-match 2>/dev/null || printf '')
    if printf '%s\n' "$tag" | grep -Eq '^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$'; then
        version=$tag
    else
        version=devel
    fi
fi
printf '%s\n' "$version" | grep -Eq '^(devel|v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?)$' || {
    printf 'invalid SWARM_VERSION\n' >&2
    exit 2
}
case "$version" in
    *-*)
        prerelease=${version#*-}
        prerelease=${prerelease%%+*}
        old_ifs=$IFS
        IFS=.
        set -- $prerelease
        IFS=$old_ifs
        for identifier do
            if printf '%s\n' "$identifier" | grep -Eq '^[0-9]+$' && ! printf '%s\n' "$identifier" | grep -Eq '^(0|[1-9][0-9]*)$'; then
                printf 'invalid SWARM_VERSION\n' >&2
                exit 2
            fi
        done
        ;;
esac

if test "${SWARM_COMMIT+x}" = x; then
    commit=$SWARM_COMMIT
else
    commit=$(git rev-parse --verify HEAD 2>/dev/null || printf '')
fi
if test -n "$commit" && ! printf '%s\n' "$commit" | grep -Eq '^[0-9a-f]{40}$'; then
    printf 'invalid SWARM_COMMIT\n' >&2
    exit 2
fi

if test "${SWARM_MODIFIED+x}" = x; then
    modified=$SWARM_MODIFIED
elif git rev-parse --verify HEAD >/dev/null 2>&1; then
    if test -n "$(git status --porcelain --untracked-files=normal 2>/dev/null)"; then modified=true; else modified=false; fi
else
    modified=unknown
fi
case "$modified" in true|false|unknown) ;; *) printf 'invalid SWARM_MODIFIED\n' >&2; exit 2;; esac

if test "${SWARM_BUILD_DATE+x}" = x; then
    build_date=$SWARM_BUILD_DATE
elif test -n "${SOURCE_DATE_EPOCH:-}"; then
    case "$SOURCE_DATE_EPOCH" in *[!0-9]*) printf 'invalid SOURCE_DATE_EPOCH\n' >&2; exit 2;; esac
    build_date=$(date -u -d "@$SOURCE_DATE_EPOCH" +%Y-%m-%dT%H:%M:%SZ)
else
    build_date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
fi
if test -n "$build_date" && ! printf '%s\n' "$build_date" | grep -Eq '^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$'; then
    printf 'invalid SWARM_BUILD_DATE\n' >&2
    exit 2
fi
if test -n "$build_date" && test "$(date -u -d "$build_date" +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || printf invalid)" != "$build_date"; then
    printf 'invalid SWARM_BUILD_DATE\n' >&2
    exit 2
fi

if test "$version" != devel; then
    test -n "$commit" || { printf 'SWARM_COMMIT is required for a release build\n' >&2; exit 2; }
    test "$modified" != unknown || { printf 'SWARM_MODIFIED is required for a release build\n' >&2; exit 2; }
    test -n "$build_date" || { printf 'SWARM_BUILD_DATE is required for a release build\n' >&2; exit 2; }
fi

mkdir -p "$(dirname -- "$output")"
ldflags="-X swarm.local/companion/internal/engine.buildVersion=$version -X swarm.local/companion/internal/engine.buildCommit=$commit -X swarm.local/companion/internal/engine.buildModified=$modified -X swarm.local/companion/internal/engine.buildDate=$build_date -X swarm.local/companion/internal/engine.buildProvenance=injected"
CGO_ENABLED=0 go build -buildvcs=false -trimpath -ldflags "$ldflags" -o "$output" ./cmd/swarm
