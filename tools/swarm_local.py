"""Linux local server lifecycle with identity-checked process ownership."""
import argparse
import contextlib
import fcntl
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from swarm_version import prepare as prepare_versioned_build, prepare_recorded, require_prepared_not_older, session_request

SOURCE = Path(__file__).resolve().parent.parent


def identity(pid):
    try:
        stat = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()
        if stat[0] == "Z":
            return None
        return {"pid": pid, "start": stat[19],
                "exe": str(Path(f"/proc/{pid}/exe").resolve()).removesuffix(" (deleted)"),
                "cmd": Path(f"/proc/{pid}/cmdline").read_bytes().hex()}
    except (OSError, ValueError):
        return None


def owned(state):
    return state and identity(state["pid"]) == state["identity"]


def server_identity(pid, expected):
    """Track the exact native server argv behind an optional mount wrapper.

    Kernel start time/executable/argv prove local ownership only, not confinement.
    Missing/exited process returns None; no process is signalled here.
    """
    actual = identity(pid)
    if actual and bytes.fromhex(actual["cmd"]).split(b"\0")[:-1] == [a.encode() for a in expected]:
        return actual
    try:
        children = Path(f"/proc/{pid}/task/{pid}/children").read_text().split()
    except OSError:
        return None
    for child in children:
        found = server_identity(int(child), expected)
        if found:
            return found
    return None


def stop(state, timeout, address):
    """Drain the owned Rust server with SIGINT; refuse a stale PID or stuck stop.

    Timeout is the caller-selected lifecycle policy, not a mission deadline.
    Never escalate to killing unknown processes or discard their state.
    """
    if not owned(state):
        return
    os.kill(state["pid"], signal.SIGINT)
    end = time.monotonic() + timeout
    while owned(state) and time.monotonic() < end:
        time.sleep(.1)
    if owned(state):
        raise RuntimeError("Le serveur ne s’arrête pas proprement ; aucun autre processus n’a été arrêté.")
    host, port = address.rsplit(":", 1)
    # Kernel socket cleanup can lag the process becoming a zombie. Do not
    # classify that short interval as an unrelated instance during restart.
    while True:
        with socket.socket() as sock:
            sock.settimeout(min(1, timeout))
            listening = sock.connect_ex((host, int(port))) == 0
        if not listening:
            break
        if time.monotonic() >= end:
            raise RuntimeError("Le port reste occupé après l’arrêt ; aucun autre processus n’a été arrêté.")
        time.sleep(.1)


def ready(address, root):
    """Require protected observed-native capabilities, never a public login 200.

    One existing local HTTP probe deadline bounds status latency, not test/work
    execution. Absent/private-key, HTTP, encoding and DTO failures mean unready;
    no credential is printed and no Root data/settings are changed.
    """
    try:
        request=session_request(address,root,'/api/v1/native-capabilities')
        with urllib.request.urlopen(request,timeout=1) as response:
            value=json.load(response)
        return isinstance(value,dict) and value.get('schema_version')==1 and value.get('profile')=='rust-native-http1-observed'
    except (OSError,ValueError,urllib.error.HTTPError):
        return False


def running_version(address,root,pid):
    """Read protected compiled declarations and hash the actual kernel executable.

    Caller verifies process ownership before/after the probe. This is local
    observation, not cryptographic server authentication. HTTP/JSON/IO errors
    refuse instead of returning a fabricated version or hashing a replacement.
    """
    request=session_request(address,root,'/api/v1/native-version')
    with urllib.request.urlopen(request,timeout=1) as response:value=json.load(response)
    if not isinstance(value,dict) or value.get('schema_version')!=1 or not isinstance(value.get('binary'),dict):
        raise ValueError('Native compiled version response invalid')
    with Path(f'/proc/{pid}/exe').open('rb') as stream:sha=hashlib.file_digest(stream,'sha256').hexdigest()
    return value,sha


def main():
    """Manage one configured local Rust service through locked public operations.

    Build and isolated qualification precede any owned stop. Existing Roots are
    opened by Rust without implicit init; new Roots use public init. Failures keep
    data/history and never select Go. Browser opening uses the native login page,
    without a secret URL. Source binaries are built; external binaries stay external.
    """
    config_path = SOURCE / "deploy" / "local-web.json"
    settings = json.loads(config_path.read_text()) if config_path.exists() else {}
    parser = argparse.ArgumentParser(description="Swarm local : start, stop, restart, status, logs, open, build, rotate")
    parser.add_argument("action", nargs="?", default="start",
                        choices=["start", "stop", "restart", "status", "logs", "open", "build", "configure", "adopt", "rotate"])
    parser.add_argument("--root", default=os.environ.get("SWARM_PROJECT_ROOT", settings.get("root", str(SOURCE))))
    parser.add_argument("--address", default=os.environ.get("SWARM_WEB_ADDRESS", settings.get("address", "127.0.0.1:18792")))
    parser.add_argument("--binary", default=os.environ.get("SWARM_BINARY", settings.get("binary", "")))
    parser.add_argument("--prepared-build", help="SHA256 exact du dernier candidat source qualifié à installer sans reconstruire les sources courantes")
    parser.add_argument("--startup-timeout", type=float, default=30)
    parser.add_argument("--stop-timeout", type=float, default=15)
    parser.add_argument("--no-open", action="store_true")
    parser.add_argument("--mask-cifs", action="store_true", default=settings.get("mask_cifs",False), help="Masquer /cifs dans un espace de montages privé (bwrap requis)")
    parser.add_argument("--no-mask-cifs", dest="mask_cifs", action="store_false")
    parser.add_argument("--pid", type=int, help="PID exact de l’ancien serveur à adopter")
    args = parser.parse_args()
    if args.prepared_build and args.action not in ("start","restart"):parser.error("--prepared-build exige start ou restart")
    root = Path(args.root).resolve()
    host, sep, port = args.address.rpartition(":")
    if not sep or host != "127.0.0.1" or not port.isdigit() or not 0 < int(port) < 65536:
        parser.error("--address : 127.0.0.1 et port fixe 1..65535 requis")
    if args.startup_timeout <= 0 or args.stop_timeout <= 0:
        parser.error("Délais strictement positifs requis")
    os.umask(0o077)
    root.mkdir(parents=True, exist_ok=True)
    runtime = root / ".swarm" / "local-web" / hashlib.sha256(args.address.encode()).hexdigest()[:16]
    runtime.mkdir(parents=True, exist_ok=True)
    binary = Path(args.binary).resolve() if args.binary else SOURCE / "bin" / "swarm-local"
    state_path = runtime / "server.json"
    log_path = runtime / "server.log"
    with contextlib.ExitStack() as stack, (runtime / "lifecycle.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = json.loads(state_path.read_text()) if state_path.exists() else None
        running = owned(state)
        if args.action == "configure":
            config_path.parent.mkdir(parents=True, exist_ok=True)
            temporary = config_path.with_suffix(".tmp")
            temporary.write_text(json.dumps({**settings, "root":str(root), "address":args.address, "mask_cifs":args.mask_cifs,
                                             "binary":str(binary) if args.binary else ""}))
            temporary.replace(config_path)
            print("Configuration locale enregistrée (projet et adresse stables).")
            return 0
        if args.action == "adopt":
            expected = [str(binary), "--root", str(root), "web", args.address]
            actual = identity(args.pid) if args.pid and args.pid > 1 else None
            if running or not actual or bytes.fromhex(actual["cmd"]).split(b"\0")[:-1] != [a.encode() for a in expected]:
                raise RuntimeError("Adoption refusée : PID, binaire, projet et adresse doivent correspondre exactement.")
            state_path.write_text(json.dumps({"pid":args.pid, "identity":actual}))
            print("Ancienne instance identifiée et adoptée ; aucune donnée modifiée.")
            return 0
        if args.action == "status":
            version,sha=running_version(args.address,root,state['pid']) if running else (None,None)
            if running and not owned(state):raise RuntimeError('Server identity changed during version observation')
            print(json.dumps({"running": bool(running), "pid": state["pid"] if running else None,
                              "root": str(root), "url": f"http://{args.address}/", "ready": ready(args.address, root) if running else False,
                              "engine":"rust", "binary":version['binary'] if version else None,"binary_sha256":sha}))
            return 0 if running else 1
        if args.action == "logs":
            # The underlying server prints a bearer URL. Never expose that credential.
            for line in log_path.read_text().splitlines()[-80:] if log_path.exists() else []:
                print(line.split("/session/")[0] + "/session/[private]" if "/session/" in line else line)
            return 0
        source_binary = binary in (SOURCE / "bin/swarm", SOURCE / "bin/swarm-local")
        if args.prepared_build and not source_binary:parser.error("--prepared-build exige le chemin binaire source géré")
        prepared = None
        if args.action == "build" or source_binary and (args.action == "restart" or args.action == "start" and (not running or args.prepared_build)):
            (SOURCE / "deploy").mkdir(parents=True, exist_ok=True)
            build_lock = stack.enter_context((SOURCE / "deploy/.local-build.lock").open("a"))
            fcntl.flock(build_lock, fcntl.LOCK_EX)
            if args.prepared_build:
                prepared,selected = prepare_recorded(SOURCE,binary,args.prepared_build)
                try:
                    if running:
                        live,live_sha=running_version(args.address,root,state['pid'])
                        if not owned(state):raise RuntimeError('Server identity changed during prepared selection')
                        require_prepared_not_older(live['binary']['version'],selected['version'],live_sha,args.prepared_build)
                        if args.action=="start":
                            if live_sha!=args.prepared_build:raise RuntimeError('Prepared start requires restart to replace an active server')
                            prepared.unlink();prepared=None
                except BaseException as primary:
                    try:
                        if prepared is not None:prepared.unlink(missing_ok=True)
                    except BaseException as cleanup:raise BaseExceptionGroup('Prepared refusal and candidate cleanup failed',[primary,cleanup])
                    raise
            else:prepared, _ = prepare_versioned_build(SOURCE, binary)
        # Pair guard: before stopping anything, refuse an engine replacement
        # that would desynchronize the authority assertion pair (2026-10-31
        # incident prevention). The currently running binary, not the
        # candidate, is the one whose settings decide — `candidate` only
        # carries bytes the live server has not loaded yet.
        if args.action in ("restart", "start"):
            pair_candidate = str(prepared) if prepared is not None else str(binary)
            require_matching_assertion_pair(str(binary), pair_candidate, str(root))
        if args.action in ("stop", "restart"):
            stop(state, args.stop_timeout, args.address)
            state_path.unlink(missing_ok=True)
            if args.action == "stop":
                print("Serveur arrêté. Missions, données et agents conservés.")
                return 0
            running = False
        if args.action == "rotate":
            # Owner key rotation: the server keeps its key in memory, so stop the
            # owned server, replace the key with the native command (it refuses
            # if anything still listens), then start again below without a build.
            if running:
                stop(state, args.stop_timeout, args.address)
                state_path.unlink(missing_ok=True)
                running = False
            subprocess.run([str(binary), "--root", str(root), "web-rotate", args.address, "--json"],
                           cwd=SOURCE, check=True, stdout=subprocess.DEVNULL)
            print("Clé locale du cockpit remplacée : anciens cookies et liens de connexion refusés.")
        if args.action == "build":
            if not running and prepared != binary:
                prepared.replace(binary)
            elif running:
                print("Candidat prêt ; restart l’installera après arrêt du seul serveur géré.")
            return 0
        if args.action == "open" and not running:
            raise RuntimeError("Serveur absent : ./swarm.sh start")
        if not running:
            # Never kill an unrelated listener, even when its name contains swarm.
            with socket.socket() as sock:
                sock.settimeout(1)
                # Bind probes can reject TIME_WAIT sockets after a clean stop.
                # A live listener is refused here; the real server's bind remains
                # authoritative and also rejects a listener appearing afterwards.
                if sock.connect_ex((host, int(port))) == 0:
                    raise RuntimeError("Port occupé par une instance non gérée ; aucun processus arrêté. Utilisez --address ou arrêtez explicitement cette instance.")
            if prepared and prepared != binary:
                prepared.replace(binary)
            if not (root / ".swarm/state.db").exists():
                subprocess.run([str(binary), "--root", str(root), "init"], cwd=SOURCE,
                               check=True, stdout=subprocess.DEVNULL)
            server_command = [str(binary), "--root", str(root), "web", args.address]
            command = server_command
            if args.mask_cifs:
                command = ["bwrap", "--bind", "/", "/", "--tmpfs", "/cifs",
                           "--dev-bind", "/dev", "/dev", "--proc", "/proc", "--", *command]
            with log_path.open("ab") as log:
                process = subprocess.Popen(command, cwd=root, stdin=subprocess.DEVNULL,
                                           stdout=log, stderr=log, start_new_session=True)
            end = time.monotonic() + args.startup_timeout
            while time.monotonic() < end:
                if process.poll() is not None:
                    raise RuntimeError("Démarrage refusé ; ./swarm.sh logs pour le diagnostic sans clé privée.")
                if ready(args.address, root):
                    break
                time.sleep(.1)
            else:
                actual = server_identity(process.pid, server_command)
                if actual:
                    os.kill(actual["pid"], signal.SIGINT)
                process.send_signal(signal.SIGINT)
                process.wait(timeout=args.stop_timeout)
                raise RuntimeError("Délai de démarrage dépassé.")
            actual = server_identity(process.pid, server_command)
            if not actual:
                process.send_signal(signal.SIGINT)
                raise RuntimeError("Identité du serveur indisponible ; démarrage non adopté.")
            try:
                version,sha=running_version(args.address,root,actual['pid'])
                with binary.open('rb') as stream:expected_sha=hashlib.file_digest(stream,'sha256').hexdigest()
                if sha!=expected_sha or identity(actual['pid'])!=actual:
                    raise RuntimeError('Started native executable differs from the prepared candidate')
            except BaseException:
                # Signal only the process still matching our exact owned identity.
                if identity(actual['pid'])==actual:os.kill(actual['pid'],signal.SIGINT)
                raise
            state = {"pid": actual["pid"], "identity": actual,"binary_sha256":sha,"version":version['binary']}
            temporary = state_path.with_suffix(".tmp")
            temporary.write_text(json.dumps(state))
            temporary.replace(state_path)
        try:
            front = authority_front(binary, root)
        except RuntimeError as error:
            # The server itself is up; only the browser hand-off is impossible.
            if not args.no_open:
                raise
            print("Avertissement : " + str(error), file=sys.stderr)
            front = None
        print(f"Swarm : http://{front or args.address}/" + (" (démon d’autorité ; moteur interne " + args.address + ")" if front else ""))
        if not args.no_open:
            opener = os.environ.get("SWARM_BROWSER", "xdg-open")
            url = f"http://{front}/login" if front else sign_in_url(binary, root, args.address)
            subprocess.run([opener, url], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return 0


def authority_front(binary, root):
    """Return the authority daemon listen address when the Root is in
    `authority` access mode, else None (owner-key mode, pre-v52 Root).

    In authority mode the engine refuses the owner cookie on every route
    except the launcher probes, so `open` must send the browser to the daemon
    sign-in page instead of an owner ticket. The daemon address has one
    source of truth: the `--listen` argument of the installed systemd unit
    written by `deploy/authority/install.sh` (path overridable with
    SWARM_AUTHORITY_UNIT for isolated tests). Errors: RuntimeError when the
    settings cannot be read or the unit is absent/unreadable/malformed; the
    owner fallback is never used silently in authority mode. No state change.
    """
    shown = subprocess.run([str(binary), "--root", str(root), "engine-settings", "show", "--json"],
                           cwd=SOURCE, capture_output=True, text=True)
    if shown.returncode != 0:
        raise RuntimeError("Réglages du moteur illisibles ; mode d’accès inconnu.")
    try:
        mode = json.loads(shown.stdout)["values"].get("web_access_mode", "owner-key")
    except (ValueError, KeyError, TypeError, AttributeError):
        raise RuntimeError("Réglages du moteur illisibles ; mode d’accès inconnu.") from None
    if mode != "authority":
        return None
    unit = Path(os.environ.get("SWARM_AUTHORITY_UNIT", "/etc/systemd/system/swarm-authority.service"))
    try:
        lines = unit.read_text().splitlines()
    except OSError:
        raise RuntimeError(f"Mode authority sans démon installé ({unit}) : sudo deploy/authority/install.sh") from None
    for line in lines:
        if line.startswith("ExecStart="):
            words = line.split()
            if "--listen" in words[:-1]:
                listen = words[words.index("--listen") + 1]
                host, _, port = listen.partition(":")
                if host == "127.0.0.1" and port.isdigit():
                    return listen
    raise RuntimeError(f"Adresse d’écoute du démon introuvable dans {unit}.")


def sign_in_url(binary, root, address):
    """Return the browser URL for `open`: a single-use ticket link when the
    native `web-ticket` command issues one, else the plain login page.

    The ticket URL is a short-lived secret (saved v50 lifetime, consumed on
    first use): it is passed only to the opener, never printed or logged.
    A refusal (older binary, Root that has not adopted the v50 setting, read
    failure) is reported on stderr with its stable code and falls back to the
    key form, which keeps working; no state is changed by the fallback.
    """
    plain = f"http://{address}/login?next=/live.html"
    result = subprocess.run([str(binary), "--root", str(root), "web-ticket", address, "/live.html", "--json"],
                            cwd=SOURCE, capture_output=True, text=True)
    if result.returncode == 0:
        try:
            url = json.loads(result.stdout)["url"]
        except (ValueError, KeyError, TypeError):
            url = ""
        if isinstance(url, str) and url.startswith(f"http://{address}/login?ticket="):
            return url
        print("Lien de connexion à usage unique illisible ; page de connexion classique ouverte.", file=sys.stderr)
        return plain
    try:
        code = json.loads(result.stderr or "{}").get("failure", {}).get("code", "inconnu")
    except (ValueError, AttributeError):
        code = "inconnu"
    hint = " Enregistrez une fois les réglages dans Admin pour adopter la durée du ticket (v50)." if code == "web-ticket-settings" else ""
    print(f"Lien de connexion à usage unique indisponible ({code}) ; page de connexion classique ouverte.{hint}", file=sys.stderr)
    return plain


AUTHORITY_ETC = Path(os.environ.get("SWARM_AUTHORITY_ETC", "/etc/swarm-authority"))
AUTHORITY_INSTALLED = Path(os.environ.get("SWARM_AUTHORITY_INSTALLED", "/usr/local/lib/swarm-authority/swarm-authority"))

# Assertion protocols the launcher can recognize in a binary. Each protocol era
# hardcodes exactly one domain byte string (`rust/crates/swarm-authority/src/
# assertion.rs` signs and verifies with it), so byte presence is the observable
# that tells which protocol a daemon or engine speaks. v2 first: verify()
# refuses every other claim version, so a binary carrying both speaks v2.
ASSERTION_PROTOCOL_DOMAINS = (
    ("v2", b"swarm-authority-assertion-v2\0"),
    ("v1", b"swarm-authority-assertion-v1\0"),
)


def assertion_protocol(path):
    """Return the assertion protocol a binary speaks, from its embedded domain
    marker ("v2"/"v1"), or None when the file is missing/unreadable or carries
    no known marker. The 2026-10-31 incident (engine v2 running against an
    installed v1 daemon) refused every relayed request with `assertion-format`;
    this byte-presence observable was verified on the installed daemon, the
    release build and the engine. Reads the file whole (the launcher already
    hashes binaries whole); executes nothing, changes nothing.
    """
    try:
        data = Path(path).read_bytes()
    except OSError:
        return None
    return next((name for name, marker in ASSERTION_PROTOCOL_DOMAINS if marker in data), None)


def require_matching_assertion_pair(current, candidate, root):
    """Refuse an engine replacement that would desynchronize the authority pair,
    before the launcher stops anything.

    On a Root whose saved access mode is `authority` (read through the public
    read-only `engine-settings show` run with the binary in place), the
    installed daemon (SWARM_AUTHORITY_INSTALLED, default
    /usr/local/lib/swarm-authority/swarm-authority) and the candidate engine
    binary must speak the same assertion protocol. Errors: RuntimeError naming
    both binaries, their protocols and the supported recovery (`./swarm.sh
    authority install`) on protocol mismatch or on an installed daemon file
    that carries no recognized protocol marker; a mode that cannot be read
    from an initialized Root is also refused (never silently treated as
    owner-key). Passes: owner-key Roots, a Root never initialized (no
    .swarm/state.db — the saved mode is then the owner-key default, nothing
    was ever switchable), and an absent daemon file (nothing on disk to
    desynchronize with; refusing would lock the operator out of a Root whose
    daemon was uninstalled, and the start path already warns about the
    missing front). Limits: the guard observes AUTHORITY_INSTALLED only — a
    daemon served from another path is out of its sight, and a copied
    authority Root whose daemon is elsewhere still fails closed at runtime
    with `assertion-format` on every relayed request; `authority install`
    replaces the daemon before the engine restart this guard observes, so the
    supported recovery stays a same-tree rebuild of both sides. Reads
    settings and binary bytes; starts, stops and reconfigures nothing.
    """
    if not (Path(root) / ".swarm" / "state.db").is_file():
        return
    if not Path(current).is_file():
        return
    try:
        shown = subprocess.run([str(current), "--root", str(root), "engine-settings", "show", "--json"],
                               cwd=SOURCE, capture_output=True, text=True)
    except OSError:
        shown = None
    if shown is None or shown.returncode != 0:
        raise RuntimeError("Réglages du moteur illisibles ; compatibilité d’assertion inconnaissable, moteur non arrêté.")
    try:
        mode = json.loads(shown.stdout)["values"].get("web_access_mode", "owner-key")
    except (ValueError, KeyError, TypeError, AttributeError):
        raise RuntimeError("Réglages du moteur illisibles ; compatibilité d’assertion inconnaissable, moteur non arrêté.") from None
    if mode != "authority":
        return
    installed_path = Path(AUTHORITY_INSTALLED)
    if not installed_path.is_file():
        return
    installed = assertion_protocol(installed_path)
    candidate_protocol = assertion_protocol(candidate)
    if installed is not None and installed == candidate_protocol:
        return
    def label(protocol, path):
        if protocol is not None:
            return protocol
        return "absent" if not Path(path).is_file() else "sans-protocole-reconnu"
    raise RuntimeError(
        f"Paire d’assertion incompatible : démon installé ({installed_path})={label(installed, installed_path)}, "
        f"moteur candidat ({candidate})={label(candidate_protocol, candidate)}. Moteur non arrêté ; "
        f"rétablissez la paire avec ./swarm.sh authority install puis relancez le redémarrage.")


def unit_addresses():
    """Return (listen, engine) from the installed daemon unit `ExecStart`
    (single source of truth written by install.sh). RuntimeError when the
    unit is missing or lacks either address."""
    unit = Path(os.environ.get("SWARM_AUTHORITY_UNIT", "/etc/systemd/system/swarm-authority.service"))
    try:
        words = next(l for l in unit.read_text().splitlines() if l.startswith("ExecStart=")).split()
    except (OSError, StopIteration):
        raise RuntimeError(f"Démon d’autorité non installé ({unit}) : ./swarm.sh authority install --admin LOGIN --display NOM") from None
    found = {}
    for flag in ("--listen", "--engine"):
        if flag in words[:-1]:
            found[flag] = words[words.index(flag) + 1]
    if set(found) != {"--listen", "--engine"}:
        raise RuntimeError(f"Adresses du démon introuvables dans {unit}.")
    return found["--listen"], found["--engine"]


def set_access_mode(binary, root, mode, key):
    """Switch the Root's saved `web_access_mode` (and daemon public key) with
    the public engine-settings preview → apply flow; no-op when already set.
    A Root saved under an older document version is adopted to the current
    one (`describe` defaults fill the newer fields, saved values win), as the
    Admin save does; the change envelope must carry that version.
    Takes effect at the next engine start. Errors: RuntimeError with the
    engine's refusal code."""
    def cli(*args):
        r = subprocess.run([str(binary), "--root", str(root), *args], cwd=SOURCE, capture_output=True, text=True)
        if r.returncode != 0:
            try:
                code = json.loads(r.stderr)["failure"]["code"]
            except (ValueError, KeyError, TypeError):
                code = "inconnu"
            raise RuntimeError(f"Réglages du moteur refusés ({code}).")
        return json.loads(r.stdout)
    shown = cli("engine-settings", "show", "--json")
    values = dict(shown["values"])
    if values.get("web_access_mode") == mode and (key is None or values.get("web_authority_public_key") == key):
        return False
    schema = shown["schema_version"]
    described = cli("engine-settings", "describe", "--json")
    if schema < described["document_schema_version"]:
        # Older Root (e.g. v49): adopt the current document like the Admin
        # does — saved values kept, newer fields from the versioned defaults.
        values = {**described["defaults"], **values}
        schema = described["document_schema_version"]
        print(f"  (réglages adoptés en v{schema} : nouveaux champs aux valeurs d’installation)", flush=True)
    values["web_access_mode"] = mode
    if key is not None:
        values["web_authority_public_key"] = key
    change = {"schema_version": schema, "event_id": "swarm-sh-authority-" + str(int(time.time())),
              "expected_revision": shown["revision"], "values": values,
              "reason": f"./swarm.sh authority : mode d’accès {mode}", "preview_token": ""}
    request = root / ".swarm" / "local-web" / "access-mode-change.json"
    request.parent.mkdir(parents=True, exist_ok=True)
    try:
        request.write_text(json.dumps(change))
        change["preview_token"] = cli("engine-settings", "preview", "--input", str(request), "--json")["preview_token"]
        request.write_text(json.dumps(change))
        cli("engine-settings", "apply", "--input", str(request), "--json")
    finally:
        request.unlink(missing_ok=True)
    return True


def authority(argv):
    """`./swarm.sh authority install|enable|disable|status`: the one entry
    point for the cockpit authority daemon (phase B).

    - install --admin LOGIN --display NOM : build the release daemon, then run
      `sudo deploy/authority/install.sh` (sudo password and the first user's
      password are asked on this terminal; idempotent, restarts a running
      service).
    - enable : stop the engine, save `authority` mode + the installed daemon
      key, move the engine to the unit's internal `--engine` address, start
      it, `sudo systemctl enable --now swarm-authority`, open the sign-in page.
      Refuses while the installed daemon differs from the last release build
      (an outdated daemon would be activated).
    - disable : `sudo systemctl disable --now`, back to `owner-key` mode, the
      engine on the daemon's former public address.
    - status : unit, service, installed/build identity, engine mode.
    - accept-restore : after a deliberate restore of the daemon state (the
      engine then refuses assertions with `web-assertion-rollback`/`-fork`),
      stop the engine, reset the Root rollback witness (recorded in the
      Root attribution ledger) and start it again.
    - reset-password LOGIN : break-glass password reset through sudo, as the
      daemon account (`swarm-authority set-password`); the new password is
      asked twice on this terminal. For when no administrator can sign in;
      otherwise an administrator resets it from the Admin.
    Every step prints what it does; a failure stops at that step (rerun after
    fixing: completed steps are no-ops). Uses only public commands.
    """
    parser = argparse.ArgumentParser(prog="swarm.sh authority", description="Démon d’autorité du cockpit (comptes, connexion)")
    parser.add_argument("step", choices=["install", "enable", "disable", "status", "reset-password", "accept-restore"])
    parser.add_argument("login", nargs="?", help="compte (reset-password)")
    parser.add_argument("--admin")
    parser.add_argument("--display")
    parser.add_argument("--no-open", action="store_true")
    args = parser.parse_args(argv)
    me = [sys.executable, str(Path(__file__).resolve())]
    built = SOURCE / "rust" / "target" / "release" / "swarm-authority"
    sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest() if p.exists() else None
    def step(text):
        print("→ " + text, flush=True)
    if args.step == "install":
        if not args.admin or not args.display:
            parser.error("install exige --admin LOGIN --display NOM")
        step("Compilation du démon (release)")
        subprocess.run(["cargo", "build", "--release", "-p", "swarm-authority"], cwd=SOURCE / "rust", check=True)
        step("Installation système (sudo)")
        subprocess.run(["sudo", str(SOURCE / "deploy/authority/install.sh"), "--admin", args.admin, "--display", args.display], check=True)
        settings = json.loads((SOURCE / "deploy" / "local-web.json").read_text()) if (SOURCE / "deploy" / "local-web.json").exists() else {}
        root = Path(os.environ.get("SWARM_PROJECT_ROOT", settings.get("root", str(SOURCE)))).resolve()
        binary = Path(settings["binary"]).resolve() if settings.get("binary") else SOURCE / "bin" / "swarm-local"
        shown = subprocess.run([str(binary), "--root", str(root), "engine-settings", "show", "--json"], cwd=SOURCE, capture_output=True, text=True)
        mode = None
        with contextlib.suppress(ValueError, KeyError):
            mode = json.loads(shown.stdout)["values"].get("web_access_mode")
        if mode == "authority":
            # Daemon and engine share the assertion format: refresh both.
            step("Moteur reconstruit et redémarré (même format d’assertion que le démon)")
            subprocess.run([*me, "restart", "--no-open"], check=True)
            print("Démon et moteur à jour.")
        else:
            print("Démon installé. Suite : ./swarm.sh authority enable")
        return 0
    settings = json.loads((SOURCE / "deploy" / "local-web.json").read_text()) if (SOURCE / "deploy" / "local-web.json").exists() else {}
    root = Path(os.environ.get("SWARM_PROJECT_ROOT", settings.get("root", str(SOURCE)))).resolve()
    binary = Path(settings["binary"]).resolve() if settings.get("binary") else SOURCE / "bin" / "swarm-local"
    service = lambda: subprocess.run(["systemctl", "is-active", "swarm-authority"], capture_output=True, text=True).stdout.strip()
    if args.step == "status":
        try:
            listen, engine = unit_addresses()
        except RuntimeError:
            listen = engine = None
        mode = None
        if binary.exists():
            shown = subprocess.run([str(binary), "--root", str(root), "engine-settings", "show", "--json"], cwd=SOURCE, capture_output=True, text=True)
            with contextlib.suppress(ValueError, KeyError):
                mode = json.loads(shown.stdout)["values"].get("web_access_mode", "owner-key")
        print(json.dumps({"installed": listen is not None, "service": service(), "listen": listen, "engine": engine,
                          "engine_mode": mode, "engine_address": settings.get("address"),
                          "installed_matches_build": sha(AUTHORITY_INSTALLED) is not None and sha(AUTHORITY_INSTALLED) == sha(built)}, indent=1))
        return 0
    if args.step == "accept-restore":
        step("Arrêt du moteur")
        subprocess.run([*me, "stop"], check=True)
        step("Témoin de retour arrière remis à zéro (noté dans le registre d’attribution)")
        try:
            subprocess.run([str(binary), "--root", str(root), "web-authority-witness", "reset", "--json"], cwd=SOURCE, check=True)
        finally:
            subprocess.run([*me, "start"] + (["--no-open"] if args.no_open else []), check=True)
        print("Restauration acceptée : le prochain accès rétablit le témoin.")
        return 0
    listen, engine = unit_addresses()
    if args.step == "reset-password":
        if not args.login:
            parser.error("reset-password exige LOGIN")
        unit = Path(os.environ.get("SWARM_AUTHORITY_UNIT", "/etc/systemd/system/swarm-authority.service"))
        words = next(l for l in unit.read_text().splitlines() if l.startswith("ExecStart=")).split()
        declaration, pin = words[words.index("--declaration") + 1], words[words.index("--pin") + 1]
        step(f"Nouveau mot de passe pour « {args.login} » (sudo)")
        subprocess.run(["sudo", "runuser", "-u", "swarm-authority", "--", str(AUTHORITY_INSTALLED), "set-password", args.login,
                        "--declaration", declaration, "--pin", pin, "--json"], check=True)
        print("Mot de passe remplacé ; les sessions ouvertes de ce compte prennent fin.")
        return 0
    if args.step == "enable":
        if sha(AUTHORITY_INSTALLED) != sha(built):
            raise RuntimeError("Le démon installé diffère du dernier build : ./swarm.sh authority install --admin LOGIN --display NOM")
        try:
            key = (AUTHORITY_ETC / "public_key").read_text().strip()
        except OSError:
            raise RuntimeError(f"Clé publique du démon illisible ({AUTHORITY_ETC}/public_key).") from None
        step("Arrêt du moteur")
        subprocess.run([*me, "stop"], check=True)
        step("Réglage : mode d’accès authority + clé du démon")
        try:
            set_access_mode(binary, root, "authority", key)
        except RuntimeError:
            # Nothing switched: give the engine back where it was.
            subprocess.run([*me, "start", "--no-open"], check=False)
            raise
        step(f"Moteur déplacé sur l’adresse interne {engine}")
        subprocess.run([*me, "configure", "--address", engine], check=True)
        subprocess.run([*me, "start", "--no-open"], check=True)
        step("Démarrage du démon (sudo)")
        subprocess.run(["sudo", "systemctl", "enable", "--now", "swarm-authority"], check=True)
        print(f"Bascule faite. Connexion : http://{listen}/login")
        if not args.no_open:
            subprocess.run([os.environ.get("SWARM_BROWSER", "xdg-open"), f"http://{listen}/login"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return 0
    # disable
    step("Arrêt du démon (sudo)")
    subprocess.run(["sudo", "systemctl", "disable", "--now", "swarm-authority"], check=True)
    step("Arrêt du moteur")
    subprocess.run([*me, "stop"], check=True)
    step("Réglage : mode d’accès owner-key")
    try:
        set_access_mode(binary, root, "owner-key", "")  # owner-key requires an empty key
    except RuntimeError:
        subprocess.run([*me, "start", "--no-open"], check=False)
        raise
    step(f"Moteur remis sur l’adresse publique {listen}")
    subprocess.run([*me, "configure", "--address", listen], check=True)
    subprocess.run([*me, "start"] + (["--no-open"] if args.no_open else []), check=True)
    print("Retour au mode clé locale fait.")
    return 0


if __name__ == "__main__":
    try:
        if sys.argv[1:2] == ["authority"]:
            sys.exit(authority(sys.argv[2:]))
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        # Do not include subprocess command lines: an opener argument is a credential.
        print("Opération refusée : " + (str(error) if not isinstance(error, subprocess.CalledProcessError) else "commande externe en échec"), file=sys.stderr)
        sys.exit(2)
