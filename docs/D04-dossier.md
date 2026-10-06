# D04 — dossier de livraison et RETEX

## Décision de livraison

Le candidat est **prêt pour contrôle public D04 et revue indépendante**, mais pas prêt à être annoncé comme publié ou installé sur le serveur partagé. Les contrôles worker installation/version/actifs/rollback passent ; D03 dispose d’un reçu hôte complet, mais la preuve locale consultée ne démontre pas encore sa revue et son acceptation postérieures. D04 est nécessairement active pendant la rédaction de ce dossier : l’absence d’agents/contrôles actifs n’est donc pas satisfaite pour une installation serveur.

## Sources, tests, gardes et raccords

| Élément | Rôle | Preuve ou garde |
| --- | --- | --- |
| `install.sh` | installation native officielle | stage dans le préfixe cible, build réussi avant `mv`, mode `0755`, aucun serveur lancé |
| `build.sh` | identité du candidat | SemVer/commit/modified/date contrôlés et injectés ; release incomplète refusée |
| `version.go`, `version_diagnostic.go` | version publique | JSON stable et distinction CLI/serveur/candidat |
| `web_server.go`, `web/*` | actifs installés | actifs embarqués ; ETag lié aux octets et revalidation authentifiée |
| `graph_delivery_install_test.go` | recette D04 | tests `InstallVersion` et `RollbackSafety` sur `t.TempDir` |
| `INSTALL.md`, `docs/en/INSTALL.md` | exploitation bilingue | installation, sauvegarde complète, compatibilité et rollback |
| `docs/GRAPH-AUTOMATION-DELIVERY.md` et traduction anglaise | bilan livraison | même état prudent, mêmes préconditions et mêmes limites |
| reçu D03 `results/c84d84cada0c4b1786c907cffa0e4b91` | qualification hôte antérieure à D04 | découverte 1 045, 1 037 pass, 8 skips optionnels explicités, 19 shards complets/disjoints ; race/vet/npm/config/diff code 0 |

Les trois nouveaux inputs D04 (test et deux notes de livraison) doivent être inclus dans le contrôle/reviewer D04. Ils n’autorisent pas à réutiliser une identité D03 comme verdict D04.

## Recette D04 exécutée

Commande worker :

```sh
go test . -run '^TestGraphDeliveryD04(InstallVersion|RollbackSafety)$' -count=1 -json -timeout=140s
```

Résultat final après ajout du scénario négatif : exit 0, paquet 6,737 s ; `InstallVersion` 2,42 s et `RollbackSafety` 4,24 s. Le rejeu ciblé sous race passe aussi : exit 0, paquet 7,758 s ; tests 2,31 s et 4,13 s.

Assertions régressives :

1. l’installateur officiel accepte un préfixe temporaire avec espaces et produit un exécutable `0755` sans session serveur ;
2. `--json version` restitue exactement version, SHA, état modified, date UTC et provenance injectée ;
3. `graph.js` servi correspond aux octets embarqués, utilise l’ETag de contenu, `private, no-cache`, puis répond `304` ;
4. des métadonnées invalides font échouer l’upgrade sans remplacer l’ancien binaire ;
5. le nouveau binaire devient observable après upgrade ;
6. le binaire sauvegardé garde son SHA-256, est restauré par stage+rename et redevient observable ;
7. un fichier sentinelle hors cible reste intact ; aucun root projet, serveur ou SQLite n’est ouvert.

## Notes de livraison FR/EN

Les deux notes annoncent uniquement un candidat non publié. Elles listent le même contenu, les mêmes preuves, la même procédure de rollback et les mêmes six préconditions de supervision. Les termes diffèrent selon la langue mais aucun état n’est amélioré dans la traduction anglaise : fournisseur réel, coût, revue D03/D04, installation partagée et clôture restent explicitement non démontrés.

## RETEX attribué

| Observation | Attribution | Effet | Action / propriétaire |
| --- | --- | --- | --- |
| Deux expirations navigateur D01, puis correction de recette | recette et supervision ; sandbox pour EPERM | deux productions D01 consommées, aucun défaut produit déduit du timeout seul | conserver historique ; supervision a corrigé les assertions précises sans troisième production |
| La pause planning D01 n’arrêtait pas le dispatcher/conducteur | supervision et contrat moteur observé | seconde production D01 partie malgré l’intention annoncée | distinguer pause planning et arrêt d’exécution ; propriétaire moteur/docs |
| D02 avait réutilisé des captures alors que la revue exigeait des captures fraîches | préparation/supervision, pas fournisseur | première revue `changes_requested`, nouveau parcours hôte sans second producteur | aligner contrat et politique avant contrôle ; propriétaire planification/supervision |
| Sauvegardes migration v18–v27 en `0644` | moteur produit | confidentialité insuffisante confirmée puis corrigée en `0600` par D02 | test de régression D02 ; propriétaire moteur |
| Application de politique D01 a répondu `SQLITE_BUSY` alors que la lecture suivante montrait la mutation | moteur/stockage et supervision | résultat ambigu ; relance aveugle évitée | toujours relire l’état public avant retry ; propriétaire moteur/supervision |
| Suite D03 complète | recette hôte/supervision | une découverte, 19 shards complets, 1 037 pass, 8 skips optionnels visibles | revue indépendante et acceptation sur le même candidat ; propriétaire reviewer/moteur |
| Installation/rollback D04 isolés | worker D04 | preuves reproductibles sans toucher serveur/DB vivants | contrôle public puis reviewer ; propriétaire supervision/moteur |
| Aucun fournisseur réel lancé et aucun coût présent | fournisseur/environnement | autonomie et coût réel non mesurés | conserver `unknown`, ne pas extrapoler ; propriétaire d’une future recette autorisée |

### Mesures disponibles

- Productions observées : D01 2/2 ; D02 1 ; D03 1 ; D04 départ courant 1/2. Aucun plafond relevé.
- Suite D03 : 1 045 tests découverts, 1 037 passés, 8 skips optionnels, 19 shards ; commande de suite 169,415 s ; race 23,389 s ; vet 2,070 s ; npm 0,970 s.
- Contrôle hôte D01 rapporté : 37,921 s. Contrôle D02 rapporté : 8,953 s.
- Coûts fournisseur, requêtes et jetons : inconnus dans les preuves disponibles ; aucune valeur inventée.
- Délai jusqu’à acceptation D04 et temps d’attente consolidé par cause : non mesurables avant clôture moteur.
- Interventions humaines : autorisation de tranche D et supervision des contrôles ; aucun commit/push/release/install serveur autorisé dans cette tentative.

## Préconditions de supervision et rollback réel

La supervision doit vérifier, via l’état public du moteur et non par inférence documentaire : D03 acceptée après son reçu/reviewer ; D04 worker terminé ; aucun agent, contrôle ou revue actif ; contrôle D04 PASS ; revue D04 PASS ; sauvegardes/empreintes disponibles ; autorisation d’installation explicite. Ensuite seulement elle peut arrêter le serveur, installer, redémarrer, comparer CLI/serveur/candidat, contrôler les actifs et le parcours ciblé.

Si une divergence apparaît, arrêter le nouveau serveur, préserver l’état en échec, restaurer ensemble données/agents/binaire compatibles et contrôler les empreintes avant redémarrage. Aucun changement de `PRAGMA user_version` et aucun ancien binaire sur des données migrées.

## Clôture réelle

État actuel : **non clôturé**. Le rapport worker peut être terminé, mais la tâche ne devient close qu’après contrôle public, revue indépendante et acceptation moteur sur les inputs D04 finaux. L’installation supervisée est une action externe supplémentaire soumise aux six préconditions des notes de livraison ; elle n’est ni simulée ni revendiquée ici.

## Limites

- Aucun commit, push, tag, release, publication, Docker, fournisseur réel ou redémarrage du port 18792.
- Aucun SQLite vivant lu ou écrit ; tous les chemins mutés par les tests sont temporaires.
- Aucun navigateur D04 : la vérification des actifs utilise le vrai handler HTTP en mémoire, pas un parcours visuel. Le parcours ciblé post-installation reste propriété de l’hôte.
- Les rapports A/B/C, D01–D03 et le harness de supervision n’ont pas été modifiés.
- Cette auto-vérification worker ne remplace pas la revue indépendante.

## Complément de supervision avant revue D04

Lecture publique r58 : D01/D02/D03 acceptées après contrôles et revues indépendantes fraîches ; D03 revue review-23bacd8fa37fd3e4e9f8b531 passée. Les mentions antérieures d’absence de preuve sont historiques et ne constituent plus le statut courant. D04 premier worker terminé, aucun second demandé. Contrôle public remplacé par verify_d04_host.py : tests isolés natifs/race plus garde liens/concepts bilingues ; suite globale D03 non répétée et ne couvre pas les nouveaux tests D04 livrés après. Installation sur18792 reste distincte des tests isolés ; autorisation de supervision déjà portée par la mission, sous préconditions absence agents/contrôles/revues et sauvegarde. Aucun commit/push/release autorisé.

### Source intégrale graph_delivery_install_test.go

```
package main

import (
        "bytes"
        "crypto/sha256"
        "encoding/json"
        "fmt"
        "net/http"
        "net/http/httptest"
        "os"
        "os/exec"
        "path/filepath"
        "strings"
        "testing"
)

type d04InstalledVersion struct {
        Schema int `json:"schema_version"`
        Binary struct {
                Version    string  `json:"version"`
                Commit     *string `json:"commit"`
                Modified   *bool   `json:"modified"`
                BuildDate  *string `json:"build_date"`
                Provenance string  `json:"provenance"`
        } `json:"binary"`
}

func d04Environment(overrides map[string]string) []string {
        blocked := make(map[string]bool, len(overrides))
        for key := range overrides {
                blocked[key] = true
        }
        env := make([]string, 0, len(os.Environ())+len(overrides))
        for _, entry := range os.Environ() {
                key, _, _ := strings.Cut(entry, "=")
                if !blocked[key] {
                        env = append(env, entry)
                }
        }
        for key, value := range overrides {
                env = append(env, key+"="+value)
        }
        return env
}

func installD04Binary(t *testing.T, binDir, version, commit string) string {
        t.Helper()
        cmd := exec.Command("./install.sh", "--mode", "native", "--bin-dir", binDir)
        cmd.Env = d04Environment(map[string]string{
                "SWARM_VERSION":    version,
                "SWARM_COMMIT":     commit,
                "SWARM_MODIFIED":   "false",
                "SWARM_BUILD_DATE": "2026-10-06T12:00:00Z",
        })
        output, err := cmd.CombinedOutput()
        if err != nil {
                t.Fatalf("native install %s failed: %v\n%s", version, err, output)
        }
        path := filepath.Join(binDir, "swarm")
        info, err := os.Stat(path)
        if err != nil || info.Mode().Perm() != 0755 {
                t.Fatalf("installed binary permissions: info=%v err=%v", info, err)
        }
        if !bytes.Contains(output, []byte("Binaire installé")) || bytes.Contains(output, []byte("session/")) {
                t.Fatalf("native install did not stay build-only: %s", output)
        }
        return path
}

func readD04Version(t *testing.T, binary string) d04InstalledVersion {
        t.Helper()
        cmd := exec.Command(binary, "--json", "version")
        cmd.Dir = t.TempDir()
        output, err := cmd.CombinedOutput()
        if err != nil {
                t.Fatalf("installed version command failed: %v\n%s", err, output)
        }
        var got d04InstalledVersion
        if err := json.Unmarshal(output, &got); err != nil {
                t.Fatalf("installed version is not JSON: %v\n%s", err, output)
        }
        return got
}

func assertD04Version(t *testing.T, got d04InstalledVersion, version, commit string) {
        t.Helper()
        if got.Schema != 1 || got.Binary.Version != version || got.Binary.Commit == nil || *got.Binary.Commit != commit || got.Binary.Modified == nil || *got.Binary.Modified || got.Binary.BuildDate == nil || *got.Binary.BuildDate != "2026-10-06T12:00:00Z" || got.Binary.Provenance != "injected" {
                t.Fatalf("installed identity mismatch: %+v", got)
        }
}

func TestGraphDeliveryD04InstallVersion(t *testing.T) {
        binDir := filepath.Join(t.TempDir(), "bin with spaces")
        commit := strings.Repeat("4", 40)
        binary := installD04Binary(t, binDir, "v0.0.0-d04", commit)
        assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04", commit)

        // Exercise the authenticated static-file boundary used by an installed
        // binary. The ETag must follow the embedded bytes so replacing the binary
        // cannot leave a fixed JS entry point silently stale.
        const host, token = "d04.local", "d04-session"
        handler := newWebHandler(nil, host, token)
        asset, err := cockpitWeb.ReadFile("web/graph.js")
        if err != nil || len(asset) == 0 {
                t.Fatalf("embedded graph asset unavailable: bytes=%d err=%v", len(asset), err)
        }
        req := httptest.NewRequest(http.MethodGet, "http://"+host+"/graph.js", nil)
        req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
        response := httptest.NewRecorder()
        handler.ServeHTTP(response, req)
        if response.Code != http.StatusOK || !bytes.Equal(response.Body.Bytes(), asset) {
                t.Fatalf("installed asset response: status=%d bytes=%d want=%d", response.Code, response.Body.Len(), len(asset))
        }
        etag := response.Header().Get("ETag")
        if etag != fmt.Sprintf("%q", hash(asset)) || response.Header().Get("Cache-Control") != "private, no-cache" {
                t.Fatalf("asset freshness headers: etag=%q cache=%q", etag, response.Header().Get("Cache-Control"))
        }
        req = httptest.NewRequest(http.MethodGet, "http://"+host+"/graph.js", nil)
        req.AddCookie(&http.Cookie{Name: "swarm_session_" + hash([]byte(host))[:12], Value: token})
        req.Header.Set("If-None-Match", etag)
        response = httptest.NewRecorder()
        handler.ServeHTTP(response, req)
        if response.Code != http.StatusNotModified || response.Body.Len() != 0 {
                t.Fatalf("asset revalidation: status=%d body=%q", response.Code, response.Body.String())
        }
}

func TestGraphDeliveryD04RollbackSafety(t *testing.T) {
        root := t.TempDir()
        binDir := filepath.Join(root, "bin")
        sentinel := filepath.Join(root, "project-state.must-not-change")
        if err := os.WriteFile(sentinel, []byte("preserved"), 0600); err != nil {
                t.Fatal(err)
        }

        oldCommit := strings.Repeat("1", 40)
        binary := installD04Binary(t, binDir, "v0.0.0-d04-old", oldCommit)
        oldBytes, err := os.ReadFile(binary)
        if err != nil {
                t.Fatal(err)
        }
        oldDigest := sha256.Sum256(oldBytes)
        backup := filepath.Join(root, "swarm.before-update")
        if err := os.WriteFile(backup, oldBytes, 0500); err != nil {
                t.Fatal(err)
        }

        failedUpdate := exec.Command("./install.sh", "--mode", "native", "--bin-dir", binDir)
        failedUpdate.Env = d04Environment(map[string]string{
                "SWARM_VERSION":    "not-a-release",
                "SWARM_COMMIT":     strings.Repeat("f", 40),
                "SWARM_MODIFIED":   "false",
                "SWARM_BUILD_DATE": "2026-10-06T12:00:00Z",
        })
        failedOutput, failedErr := failedUpdate.CombinedOutput()
        if failedErr == nil || !bytes.Contains(failedOutput, []byte("invalid SWARM_VERSION")) {
                t.Fatalf("invalid update was not rejected: err=%v output=%s", failedErr, failedOutput)
        }
        assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-old", oldCommit)

        newCommit := strings.Repeat("2", 40)
        installD04Binary(t, binDir, "v0.0.0-d04-new", newCommit)
        assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-new", newCommit)

        // Restore through a staged file and rename, mirroring the installer's
        // same-filesystem replacement. No project root or database is opened.
        staged := filepath.Join(binDir, ".swarm-rollback-stage")
        backupBytes, err := os.ReadFile(backup)
        if err != nil {
                t.Fatal(err)
        }
        if got := sha256.Sum256(backupBytes); got != oldDigest {
                t.Fatalf("backup digest changed: got=%x want=%x", got, oldDigest)
        }
        if err := os.WriteFile(staged, backupBytes, 0755); err != nil {
                t.Fatal(err)
        }
        if err := os.Rename(staged, binary); err != nil {
                t.Fatal(err)
        }
        assertD04Version(t, readD04Version(t, binary), "v0.0.0-d04-old", oldCommit)
        if raw, err := os.ReadFile(sentinel); err != nil || string(raw) != "preserved" {
                t.Fatalf("rollback touched unrelated state: %q err=%v", raw, err)
        }
}

```

### Source intégrale install.sh

```
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
    (cd "$repo_dir" && sh ./build.sh "$stage_file")
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

```

### Source intégrale build.sh

```
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
ldflags="-X main.buildVersion=$version -X main.buildCommit=$commit -X main.buildModified=$modified -X main.buildDate=$build_date -X main.buildProvenance=injected"
CGO_ENABLED=0 go build -buildvcs=false -trimpath -ldflags "$ldflags" -o "$output" .

```

### Source intégrale docs/plans/graphe-automatisation-20261005/execution/d/verify_d04_host.py

```
"""D04 host install/rollback control with bilingual delivery document checks."""
import importlib.util,json,re,sys,hashlib
from pathlib import Path
D=Path(__file__).resolve().parent

def documents(text,path,concepts):
 assert all(word.lower() in text.lower() for word in concepts),'missing delivery concept: '+str(path)
 links=re.findall(r'\[[^\]]+\]\(([^)]+)\)',text)
 assert links,'documentation links absent'
 for target in links:
  if not target.startswith(('https://','http://','#')):
   assert (path.parent/target.split('#')[0]).is_file(),'broken delivery link: '+target

def main():
 fr=Path('docs/GRAPH-AUTOMATION-DELIVERY.md');en=Path('docs/en/GRAPH-AUTOMATION-DELIVERY.md')
 documents(fr.read_text(),fr,['non publiée','inconnu','rollback','304','0755','revue indépendante','sauvegarde','serveur'])
 documents(en.read_text(),en,['not published','unknown','rollback','304','0755','independent reviewer','backup','server'])
 for p,h in json.loads((D/'protected-d01.json').read_text()).items():
  assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,'accepted D01 changed'
 spec=importlib.util.spec_from_file_location('d_verify',D/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v);v.main('D04')
 print(json.dumps({'documentation':'PASS','languages':['fr','en'],'scope':'isolated official native installer, version, failed upgrade and binary rollback; in-memory actual handler assets. Live installation and provider autonomy not demonstrated.'}))
if __name__=='__main__':
 if '--self-test' in sys.argv:
  try:documents('[broken](does-not-exist-d04.md)',Path('docs/GRAPH-AUTOMATION-DELIVERY.md'),[])
  except AssertionError:print('PASS missing-link self-test; no install/functional control executed')
  else:raise AssertionError('broken link accepted')
 else:main()

```

### Source intégrale docs/GRAPH-AUTOMATION-DELIVERY.md

```
# Livraison candidate — graphe et automatisation

État au 6 octobre 2026 : **candidate à qualifier, non publiée**. Cette note décrit le contenu vérifié dans la copie D01–D04 ; elle ne constitue ni une release, ni une acceptation moteur, ni la preuve que le serveur partagé exécute ce candidat.

## Contenu du candidat

- préparation et édition de graphe avec aperçu, détection des cycles, révisions et conflits ;
- journal d’agents et restitution des preuves liées au graphe ;
- programmes d’automatisation, demandes idempotentes, planification, pause et reprise ;
- migration/archives et programmes importés désactivés ;
- diagnostic de version séparant CLI installée, serveur actif et sources candidates ;
- documentation d’installation, sauvegarde et rollback en français et en anglais.

Les parcours D01/D02 utilisent des racines et navigateurs isolés sans autonomie fournisseur réelle. Le coût fournisseur reste inconnu.

## Installation et version vérifiées

Le test D04 appelle l’installateur natif officiel dans un préfixe temporaire contenant des espaces. Il vérifie le mode `0755`, l’absence de démarrage, puis l’identité stable renvoyée par :

```sh
swarm --json version
```

Il vérifie aussi les actifs web embarqués, leur ETag fondé sur le contenu et la revalidation `304`, afin qu’un remplacement de binaire ne conserve pas silencieusement un point d’entrée JavaScript périmé. Ce contrôle isolé ne remplace pas la comparaison, après supervision autorisée, des trois identités CLI/serveur/candidat.

## Mise à jour et rollback

Avant toute mise à jour réelle : attendre l’absence d’agent et de contrôle actifs, arrêter le serveur, sauvegarder ensemble `.swarm/`, le dossier des agents et le binaire correspondant, puis contrôler leurs empreintes. Ne jamais utiliser un ancien binaire sur des données déjà migrées ; restaurer le couple sauvegarde/binaire compatible.

La recette D04 vérifie qu’une mise à jour aux métadonnées invalides échoue sans remplacer le binaire installé. Elle vérifie ensuite un passage ancien→nouveau, puis la restauration atomique du binaire sauvegardé avec SHA-256 identique et sans altérer un état sentinelle hors cible. Les commandes opérationnelles complètes restent dans [INSTALL.md](../INSTALL.md).

## Autorisation et limites de livraison

Aucun commit, push, tag, paquet, image publique, release ou redémarrage du serveur partagé n’est autorisé par cette note. La supervision ne peut installer le candidat qu’après :

1. contrôle D04 public sur les inputs finaux ;
2. revue indépendante du même candidat ;
3. acceptation D03 puis D04 enregistrée par le moteur ;
4. absence constatée d’agents et de contrôles actifs ;
5. sauvegarde et plan de rollback prêts ;
6. autorisation explicite de l’installation du serveur.

Après installation, relire la version CLI et la santé/version du serveur, puis effectuer le parcours de navigation ciblé. Toute identité divergente, actif périmé ou régression du graphe impose l’arrêt et le rollback documenté.

## État des preuves

- D01 et D02 : contrôles/revues historiques rapportés comme acceptés, avec limites fixture explicites.
- D03 : reçu hôte présent (suite Go découverte une fois, race, vet, npm et gardes à code 0), mais la preuve locale consultée ne démontre pas encore une revue/acceptation postérieure à ce reçu.
- D04 : contrôles worker installation/version/actifs/rollback à code 0 ; contrôle public, revue indépendante, installation supervisée et clôture moteur encore attendus.

Voir [le rapport D04](D04.md) et [son dossier](D04-dossier.md).

```

### Source intégrale docs/en/GRAPH-AUTOMATION-DELIVERY.md

```
# Candidate delivery — graph and automation

Status on October 6, 2026: **candidate awaiting qualification, not published**. This note describes content verified in the D01–D04 checkout; it is not a release, engine acceptance, or proof that the shared server is running this candidate.

## Candidate contents

- graph preparation and editing with preview, cycle detection, revisions and conflicts;
- agent journal and graph-linked evidence rendering;
- automation programs, idempotent requests, scheduling, pause and recovery;
- migration/archives and disabled imported programs;
- version diagnostics separating installed CLI, active server and candidate sources;
- French and English installation, backup and rollback documentation.

D01/D02 journeys use isolated roots and browsers without real provider autonomy. Provider cost remains unknown.

## Verified installation and version

The D04 test invokes the official native installer in a temporary prefix containing spaces. It checks mode `0755`, confirms that no server starts, then reads the stable identity through:

```sh
swarm --json version
```

It also checks embedded web assets, their content-based ETag and `304` revalidation so replacing the binary cannot silently retain a stale JavaScript entry point. This isolated check does not replace the supervised post-install comparison of installed CLI, active server and candidate identities.

## Upgrade and rollback

Before a real upgrade: wait until no agent or check is active, stop the server, back up `.swarm/`, the agent directory and the matching binary together, then verify their hashes. Never run an old binary against already migrated data; restore the compatible backup/binary pair.

The D04 recipe verifies that an update with invalid metadata fails without replacing the installed binary. It then checks an old→new upgrade and atomic restoration of the saved binary with the same SHA-256, without changing unrelated sentinel state. The complete operational commands remain in [INSTALL.md](INSTALL.md).

## Delivery authorization and limits

This note authorizes no commit, push, tag, package, public image, release or shared-server restart. Supervision may install the candidate only after:

1. the public D04 check runs against the final inputs;
2. an independent reviewer examines the same candidate;
3. the engine records D03 and then D04 acceptance;
4. no active agent or check remains;
5. the backup and rollback plan are ready; and
6. shared-server installation is explicitly authorized.

After installation, read the CLI identity and server health/version again, then run the targeted navigation journey. Any divergent identity, stale asset or graph regression requires stopping and following the documented rollback.

## Evidence status

- D01 and D02: historical checks/reviews are reported accepted, with fixture limits explicit.
- D03: a host receipt is present (single discovery of the Go suite, race, vet, npm and guards all exit 0), but the inspected local evidence does not yet establish a later review/acceptance of that receipt.
- D04: worker installation/version/assets/rollback checks exit 0; public check, independent review, supervised installation and engine closure remain pending.

See the [D04 report](../D04.md) and [dossier](../D04-dossier.md).

```

## Analyse du refus de revue — phases de clôture

Le refus review-a0dfca5ec4eeb520d69b1782 est conservé. Il confirme les contrôles D04 mais demande une clôture déjà acquise avant sa propre revue. Le code moteur fourni ci-dessous refuse effectivement close tant que toute tâche descendante, dont D04, n’est pas acceptedFresh. Le contrat de supervision exige quatre acceptations fraîches puis enfant/root close publics, et installation vivante ensuite. Ce séquencement rend pending_review normal ici et n’autorise aucune déclaration de clôture anticipée.

Aucune clôture ni installation18792 n'est revendiquée. Pour rendre vérifiable cette frontière, le contrôle D04 ajoute deux tests moteur existants isolés : clôture après avis frais, refus sans avis/avec preuve périmée. Les preuves de clôture de cette mission et d’installation sont des opérations de supervision à réaliser après acceptation, pas des succès inventés dans ce dossier. L’exigence de clôture réelle demeure à remplir avant livraison finale et désactivation du suivi. L'installation/version/rollback isolés est déjà prouvée ; « puis supervision autorisée » est couverte par l'autorisation de mission sous les préconditions ci-dessus, sans prétendre déploiement accompli.

### Garde moteur planning.go (extrait exact)
```go
        case "close":
                for _, event := range p.Inbox {
                        if event.Scope == id && event.Decision == "" && !containsString(r.Inputs, event.ID) {
                                return fmt.Errorf("retour non traité : %s", event.ID)
                        }
                }
                for _, child := range p.Scopes {
                        if child.Parent == id && child.State != "closed" {
                                return fmt.Errorf("périmètre enfant non terminé")
                        }
                }
                for _, task := range w.Tasks {
                        if task.ScopeID == "" {
                                return fmt.Errorf("tâche sans périmètre")
                        }
                        owner, err := p.scope(task.ScopeID)
                        if err != nil {
                                return err
                        }
                        for owner.ID != id && owner.Parent != "" {
                                owner, _ = p.scope(owner.Parent)
                        }
                        if owner.ID == id && (task.Status != "accepted" || !s.acceptedFresh(w, &task, map[string]bool{})) {
                                return fmt.Errorf("preuve descendante non valide : %s", task.ID)
                        }
                }

```

### Tests moteur existants intégraux
```go
//go:build linux

package main

import (
        "encoding/json"
        "os"
        "testing"
)

func TestAutomaticValidationClosesRootAfterFreshReviewDespiteOldFailure(t *testing.T) {
        s, w, a, _ := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
        current, err := s.get(w.ID)
        if err != nil {
                t.Fatal(err)
        }
        current.Planning.Failure = "old reviewer timeout"
        raw, _ := json.Marshal(current)
        if _, err = s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, current.ID); err != nil {
                t.Fatal(err)
        }
        s.conduct(a, "completed")
        got, err := s.get(w.ID)
        if err != nil {
                t.Fatal(err)
        }
        root, _ := got.Planning.scope("root")
        if got.Tasks[0].Status != "accepted" || root.State != "closed" {
                t.Fatalf("fresh accepted result did not close root: task=%s root=%s", got.Tasks[0].Status, root.State)
        }
        if got.Planning.Failure != "old reviewer timeout" {
                t.Fatal("historical failure was erased")
        }
}

func TestAutomaticValidationDoesNotCloseRootWithoutReviewOrWithStaleProof(t *testing.T) {
        t.Run("missing-review", func(t *testing.T) {
                s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
                current, _ := s.get(w.ID)
                current.Tasks[0].IndependentReview = nil
                raw, _ := json.Marshal(current)
                if _, err := s.db.Exec("UPDATE works SET body=? WHERE id=?", raw, current.ID); err != nil {
                        t.Fatal(err)
                }
                accepted, _ := s.runAutomaticValidation(a, report)
                if accepted {
                        t.Fatal("result accepted without independent review")
                }
                got, _ := s.get(w.ID)
                root, _ := got.Planning.scope("root")
                if root.State == "closed" {
                        t.Fatal("root closed without independent review")
                }
        })

        t.Run("stale-proof", func(t *testing.T) {
                s, w, a, report := automaticValidationFixture(t, automaticPolicy("go", "version"), false)
                s.conduct(a, "completed")
                if err := os.WriteFile(filepathJoin(s.root, report), []byte("changed after acceptance\n"), 0600); err != nil {
                        t.Fatal(err)
                }
                got, err := s.reconcilePlanningProofs(mustGetWork(t, s, w.ID))
                if err != nil {
                        t.Fatal(err)
                }
                root, _ := got.Planning.scope("root")
                if root.State == "closed" {
                        t.Fatal("root remained closed with stale proof")
                }
        })
}

func filepathJoin(root, path string) string { return root + string(os.PathSeparator) + path }

func mustGetWork(t *testing.T, s *Store, id string) Work {
        t.Helper()
        w, err := s.get(id)
        if err != nil {
                t.Fatal(err)
        }
        return w
}

```

### Wrapper actuel après diagnostic (remplace sa version historique ci-dessus)
```python
"""D04 host install/rollback control with bilingual delivery document checks."""
import importlib.util,json,re,sys,hashlib
from pathlib import Path
D=Path(__file__).resolve().parent

def documents(text,path,concepts):
 assert all(word.lower() in text.lower() for word in concepts),'missing delivery concept: '+str(path)
 links=re.findall(r'\[[^\]]+\]\(([^)]+)\)',text)
 assert links,'documentation links absent'
 for target in links:
  if not target.startswith(('https://','http://','#')):
   assert (path.parent/target.split('#')[0]).is_file(),'broken delivery link: '+target

def main():
 fr=Path('docs/GRAPH-AUTOMATION-DELIVERY.md');en=Path('docs/en/GRAPH-AUTOMATION-DELIVERY.md')
 documents(fr.read_text(),fr,['non publiée','inconnu','rollback','304','0755','revue indépendante','sauvegarde','serveur'])
 documents(en.read_text(),en,['not published','unknown','rollback','304','0755','independent reviewer','backup','server'])
 for p,h in json.loads((D/'protected-d01.json').read_text()).items():
  assert hashlib.sha256(Path(p).read_bytes()).hexdigest()==h,'accepted D01 changed'
 spec=importlib.util.spec_from_file_location('d_verify',D/'verify.py');v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v);v.main('D04')
 import subprocess
 command=['go','test','-race','.','-run','^TestAutomaticValidation(ClosesRootAfterFreshReviewDespiteOldFailure|DoesNotCloseRootWithoutReviewOrWithStaleProof)$','-count=1','-json','-timeout=60s']
 result=subprocess.run(command,capture_output=True,text=True,timeout=75)
 assert result.returncode==0,result.stdout[-4000:]+result.stderr
 events=[json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
 expected={'TestAutomaticValidationClosesRootAfterFreshReviewDespiteOldFailure','TestAutomaticValidationDoesNotCloseRootWithoutReviewOrWithStaleProof'}
 assert expected<={e.get('Test') for e in events if e.get('Action')=='pass'}
 assert not any(e.get('Action') in ['skip','fail'] for e in events)
 print(json.dumps({'closure_transition_tests':'PASS','command':command,'exit_code':result.returncode,'log_sha256':hashlib.sha256((result.stdout+result.stderr).encode()).hexdigest(),'scope':'isolated engine fixtures; live parent/child closure remains post-review public operation'}))
 print(json.dumps({'documentation':'PASS','languages':['fr','en'],'scope':'isolated official native installer, version, failed upgrade and binary rollback; in-memory actual handler assets. Live installation and provider autonomy not demonstrated.'}))
if __name__=='__main__':
 if '--self-test' in sys.argv:
  try:documents('[broken](does-not-exist-d04.md)',Path('docs/GRAPH-AUTOMATION-DELIVERY.md'),[])
  except AssertionError:print('PASS missing-link self-test; no install/functional control executed')
  else:raise AssertionError('broken link accepted')
 else:main()

```
