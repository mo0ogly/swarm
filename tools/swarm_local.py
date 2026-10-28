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
    parser = argparse.ArgumentParser(description="Swarm local : start, stop, restart, status, logs, open, build")
    parser.add_argument("action", nargs="?", default="start",
                        choices=["start", "stop", "restart", "status", "logs", "open", "build", "configure", "adopt"])
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
        if args.action in ("stop", "restart"):
            stop(state, args.stop_timeout, args.address)
            state_path.unlink(missing_ok=True)
            if args.action == "stop":
                print("Serveur arrêté. Missions, données et agents conservés.")
                return 0
            running = False
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
        print(f"Swarm : http://{args.address}/")
        if not args.no_open:
            # Native login accepts a POSTed key; no credential-bearing URL exists.
            opener = os.environ.get("SWARM_BROWSER", "xdg-open")
            subprocess.run([opener, f"http://{args.address}/login?next=/live.html"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        # Do not include subprocess command lines: an opener argument is a credential.
        print("Opération refusée : " + (str(error) if not isinstance(error, subprocess.CalledProcessError) else "commande externe en échec"), file=sys.stderr)
        sys.exit(2)
