"""Linux local server lifecycle with identity-checked process ownership."""
import argparse
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
    """Track the actual Go server, not a mount-wrapper that can leave it alive."""
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
    if not owned(state):
        return
    os.kill(state["pid"], signal.SIGTERM)
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
    try:
        key_path = root / ".swarm" / ("web-session-" + hashlib.sha256(address.encode()).hexdigest()[:16])
        token = key_path.read_text().strip()
        request = urllib.request.Request(f"http://{address}/", headers={
            "Cookie": "swarm_session_" + hashlib.sha256(address.encode()).hexdigest()[:12] + "=" + token})
        with urllib.request.urlopen(request, timeout=1) as response:
            return response.status == 200
    except (OSError, urllib.error.HTTPError):
        return False


def main():
    config_path = SOURCE / "deploy" / "local-web.json"
    settings = json.loads(config_path.read_text()) if config_path.exists() else {}
    parser = argparse.ArgumentParser(description="Swarm local : start, stop, restart, status, logs, open, build")
    parser.add_argument("action", nargs="?", default="start",
                        choices=["start", "stop", "restart", "status", "logs", "open", "build", "configure", "adopt"])
    parser.add_argument("--root", default=os.environ.get("SWARM_PROJECT_ROOT", settings.get("root", str(SOURCE))))
    parser.add_argument("--address", default=os.environ.get("SWARM_WEB_ADDRESS", settings.get("address", "127.0.0.1:18792")))
    parser.add_argument("--binary", default=os.environ.get("SWARM_BINARY", ""))
    parser.add_argument("--startup-timeout", type=float, default=30)
    parser.add_argument("--stop-timeout", type=float, default=15)
    parser.add_argument("--no-open", action="store_true")
    parser.add_argument("--mask-cifs", action="store_true", default=settings.get("mask_cifs",False), help="Masquer /cifs dans un espace de montages privé (bwrap requis)")
    parser.add_argument("--no-mask-cifs", dest="mask_cifs", action="store_false")
    parser.add_argument("--pid", type=int, help="PID exact de l’ancien serveur à adopter")
    args = parser.parse_args()
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
    with (runtime / "lifecycle.lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        state = json.loads(state_path.read_text()) if state_path.exists() else None
        running = owned(state)
        if args.action == "configure":
            config_path.parent.mkdir(parents=True, exist_ok=True)
            temporary = config_path.with_suffix(".tmp")
            temporary.write_text(json.dumps({"root":str(root), "address":args.address, "mask_cifs":args.mask_cifs}))
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
            print(json.dumps({"running": bool(running), "pid": state["pid"] if running else None,
                              "root": str(root), "url": f"http://{args.address}/", "ready": ready(args.address, root) if running else False}))
            return 0 if running else 1
        if args.action == "logs":
            # The underlying server prints a bearer URL. Never expose that credential.
            for line in log_path.read_text().splitlines()[-80:] if log_path.exists() else []:
                print(line.split("/session/")[0] + "/session/[private]" if "/session/" in line else line)
            return 0
        if args.action in ("stop", "restart"):
            stop(state, args.stop_timeout, args.address)
            state_path.unlink(missing_ok=True)
            if args.action == "stop":
                print("Serveur arrêté. Missions, données et agents conservés.")
                return 0
            running = False
        if args.action == "build":
            if running:
                raise RuntimeError("Arrêtez cette instance avant de remplacer son binaire.")
            subprocess.run(["sh", str(SOURCE / "build.sh"), str(binary)], cwd=SOURCE, check=True)
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
            if not args.binary:
                subprocess.run(["sh", str(SOURCE / "build.sh"), str(binary)], cwd=SOURCE, check=True)
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
                    os.kill(actual["pid"], signal.SIGTERM)
                process.terminate()
                process.wait(timeout=args.stop_timeout)
                raise RuntimeError("Délai de démarrage dépassé.")
            actual = server_identity(process.pid, server_command)
            if not actual:
                process.terminate()
                raise RuntimeError("Identité du serveur indisponible ; démarrage non adopté.")
            state = {"pid": actual["pid"], "identity": actual}
            temporary = state_path.with_suffix(".tmp")
            temporary.write_text(json.dumps(state))
            temporary.replace(state_path)
        print(f"Swarm : http://{args.address}/")
        if not args.no_open:
            key_path = root / ".swarm" / ("web-session-" + hashlib.sha256(args.address.encode()).hexdigest()[:16])
            info = key_path.lstat()
            if key_path.is_symlink() or info.st_mode & 0o077:
                raise RuntimeError("Clé de session non privée.")
            token = key_path.read_text().strip()
            # Authentication stays enabled. Only the local opener receives the private URL.
            opener = os.environ.get("SWARM_BROWSER", "xdg-open")
            subprocess.run([opener, f"http://{args.address}/session/{token}"], check=True,
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, RuntimeError, subprocess.CalledProcessError) as error:
        # Do not include subprocess command lines: an opener argument is a credential.
        print("Opération refusée : " + (str(error) if not isinstance(error, subprocess.CalledProcessError) else "commande externe en échec"), file=sys.stderr)
        sys.exit(2)
