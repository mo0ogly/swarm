#!/usr/bin/env python3
"""Condition S : préparation et règlement conduits par Swarm, fautes injectées de l'extérieur.

Chaque exécution a sa propre racine Swarm (`swarm init` dans la racine d'exécution), son API, et un
conducteur `web` sans adresse : il écoute sur 127.0.0.1:0 (web_server.go, serveWeb), donc aucun
conflit de port entre conducteurs ni entre exécutions.

Un échec du banc lui-même (CLI swarm en échec pendant l'installation, API non démarrée, conducteur
mort) lève harness.BenchError : c'est une ERREUR, jamais une mesure. Un dépassement de
S_RUN_TIMEOUT_S donne DÉLAI. Le nettoyage arrête mission, agents, conducteurs et API dans tous les
cas, puis tue tout processus restant dont la ligne de commande désigne la racine d'exécution.
"""
import json
import os
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, markers

SWARM = harness.SWARM_BIN
ACTIVE = {"queued", "starting", "running", "stopping"}   # agents_store.go : activeAgent
LISTENING = "Cockpit local : http://127.0.0.1:"          # web_server.go : annonce d'écoute de serveWeb


class Swarm:
    def __init__(self, binary, root):
        self.binary, self.root = str(binary), root

    def cli(self, args, data=None):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args]
                           + (["--input", "-"] if data is not None else []),
                           input=json.dumps(data) if data is not None else None,
                           text=True, capture_output=True, timeout=config.S_CLI_TIMEOUT_S)
        if p.returncode:
            raise RuntimeError(f"swarm {' '.join(args[:2])} : code {p.returncode} : {(p.stdout + p.stderr).strip()}")
        return json.loads(p.stdout) if p.stdout.strip() else None

    def mutate(self, args, work, **fields):
        out = self.cli(args, dict(schema_version=1, event_id=uuid.uuid4().hex,
                                  expected_revision=work.get("revision", 0), **fields))
        return out.get("work", out)

    def work(self, wid):
        out = self.cli(["work", "show", wid])
        return out.get("work", out)

    def agents(self, wid):
        return [x.get("agent", x) for x in self.cli(["agent", "list", wid])["agents"]]

    def conductor(self, log):
        return subprocess.Popen([self.binary, "--root", str(self.root), "web"], stdout=log, stderr=subprocess.STDOUT)


def profile(root, provider="banc"):
    return {"provider": provider, "role": "worker", "workspace": str(root), "capture_output": True,
            "timeout_seconds": 120}


def setup_work(sw, api_url, fault, prepare_provider):
    w = sw.mutate(["work", "create"], {}, title="Banc de facturation",
                  objective="Payer chaque facture due une seule fois, au montant exact",
                  scope="banc synthétique isolé", criteria=["grand livre conforme"], next="lancer")

    def control(task, script, *extra, why):
        # Programme `python3` (liste blanche validationPrograms) ; répertoire : la racine du travail.
        return {"mode": "automatic", "controls": [{
            "id": task + "-controle", "command": ["python3", harness.SCRIPTS[script], "--api", api_url, *extra],
            "criteria": [1], "justification": why, "timeout_seconds": 15}]}

    specs = [
        ("prepare", [], "lot.json", "lot conforme au grand livre",
         control("prepare", "check_lot", "--lot", "lot.json",
                 why="Compare chaque ligne au grand livre : facture, montant, bénéficiaire, plafond, complétude."),
         prepare_provider),
        ("settle", ["prepare"], "settlement.json", "chaque facture payée une fois au montant exact",
         control("settle", "verify_settlement",
                 why="Relit les paiements du grand livre : aucun doublon, aucun écart, aucune facture impayée."),
         "banc"),
    ]
    for tid, deps, deliverable, criterion, policy, provider in specs:
        w = sw.mutate(["task", "add", w["id"]], w, id=tid, title=tid, deliverable=deliverable, criteria=[criterion],
                      depends=deps, next="Produire uniquement le livrable demandé")
        update = {"id": tid, "max_attempts": 2, "validation_policy": policy}
        if fault == "F6" and tid == "prepare":
            update["max_tool_calls"] = config.S_BUDGET_TOOL_CALLS
        w = sw.mutate(["task", "update", w["id"]], w, **update)
        sw.cli(["profile", w["id"], tid], profile(sw.root, provider))
        w = sw.work(w["id"])
    return w


def log_tail(path):
    try:
        return Path(path).read_text(errors="replace").strip()[-config.ERROR_TEXT_MAX:]
    except OSError as e:
        return f"journal illisible : {e}"


def await_listening(conductors, log_path, run_dir):
    """Chaque conducteur lancé doit annoncer son écoute loopback ; sinon le banc a échoué."""
    deadline = time.monotonic() + config.S_CONDUCTOR_START_S
    while True:
        dead = [c.pid for c in conductors if c.poll() is not None]
        if dead:
            raise harness.BenchError(f"conducteur arrêté au démarrage {dead} : {log_tail(log_path)}", run_dir)
        if Path(log_path).read_text(errors="replace").count(LISTENING) >= len(conductors):
            return
        if time.monotonic() > deadline:
            raise harness.BenchError(f"conducteur muet après {config.S_CONDUCTOR_START_S} s : {log_tail(log_path)}",
                                     run_dir)
        time.sleep(0.05)


def stall_and_take_over(sw, faults, conductors, log):
    """F7 : le premier conducteur est figé au-delà du seuil de péremption, un second prend la main."""
    first = conductors[0]
    os.kill(first.pid, signal.SIGSTOP)
    markers.mark(faults / "owner-stalled.json", pid=first.pid, stalled_for_s=config.S_STALL_S)
    time.sleep(config.S_STALL_S)
    conductors.append(sw.conductor(log))
    time.sleep(2)
    os.kill(first.pid, signal.SIGCONT)


def watch(sw, wid, run_dir, fault, conductors, log):
    faults = run_dir / "faults"
    deadline = time.monotonic() + config.S_RUN_TIMEOUT_S
    tasks, last, quiet_since, injected = {}, None, None, False
    while time.monotonic() < deadline:
        if all(c.poll() is not None for c in conductors):
            raise harness.BenchError(f"plus aucun conducteur actif : {log_tail(run_dir / 'conductor.log')}", run_dir)
        tasks = {t["id"]: t["status"] for t in sw.work(wid)["tasks"]}
        if not injected and tasks.get("prepare") == "accepted":
            if fault == "F4" and (run_dir / "lot.json").exists():
                harness.tamper(run_dir / "lot.json", faults / "lot-tampered.json")
                (faults / "tamper-done").write_text("1")
                injected = True
            elif fault == "F7":
                stall_and_take_over(sw, faults, conductors, log)
                injected = True
        if tasks.get("settle") == "accepted":
            return tasks, False
        busy = any(a["status"] in ACTIVE for a in sw.agents(wid))
        if not busy and tasks == last:
            quiet_since = quiet_since or time.monotonic()
            if time.monotonic() - quiet_since >= config.S_QUIET_S:
                return tasks, False
        else:
            quiet_since = None
        last = tasks
        time.sleep(config.S_POLL_S)
    return tasks, True


def stop(sw, wid, conductors):
    """Arrêt ordonné : mission en pause, agents arrêtés et attendus, puis conducteurs. Renvoie les erreurs."""
    errors = []
    if wid is not None:
        try:
            sw.cli(["mission", "stop", wid])
        except (RuntimeError, subprocess.TimeoutExpired) as e:
            errors.append(f"mission stop : {e}")
        deadline = time.monotonic() + config.S_CLEANUP_TIMEOUT_S
        while True:
            try:
                active = [a["id"] for a in sw.agents(wid) if a["status"] in ACTIVE]
            except (RuntimeError, subprocess.TimeoutExpired, KeyError, ValueError) as e:
                errors.append(f"agent list : {e}")
                break
            if not active or time.monotonic() > deadline:
                if active:
                    errors.append(f"agents encore actifs après {config.S_CLEANUP_TIMEOUT_S} s : {active}")
                break
            for aid in active:
                try:
                    sw.cli(["agent", "stop", aid])
                except (RuntimeError, subprocess.TimeoutExpired) as e:
                    if "déjà terminé" not in str(e):
                        errors.append(f"agent stop {aid} : {e}")
            time.sleep(config.S_POLL_S)
    for c in conductors:
        try:
            os.kill(c.pid, signal.SIGCONT)   # F7 : un conducteur figé ne traite pas SIGTERM
        except ProcessLookupError:
            pass
        harness.stop(c)
    return errors


def reap(run_dir):
    """Tue tout processus restant dont la ligne de commande désigne la racine (superviseurs, fournisseurs).

    Les superviseurs `_supervise` sont détachés (Setsid, agents_process.go : spawnAgent) : ni le conducteur
    ni ce processus ne les attendent. Renvoie les lignes de commande des processus tués.
    """
    needle, me, found = str(run_dir).encode(), os.getpid(), {}
    for entry in Path("/proc").iterdir():
        if not entry.name.isdigit() or int(entry.name) == me:
            continue
        try:
            cmdline = (entry / "cmdline").read_bytes()
        except OSError:
            continue
        if any(arg == needle or arg.startswith(needle + b"/") for arg in cmdline.split(b"\0")):
            found[int(entry.name)] = cmdline.replace(b"\0", b" ").decode("utf-8", "replace").strip()
    for sig in (signal.SIGCONT, signal.SIGTERM):
        for pid in found:
            try:
                os.kill(pid, sig)
            except ProcessLookupError:
                pass
    deadline = time.monotonic() + 5
    while time.monotonic() < deadline and any(Path(f"/proc/{pid}").exists() for pid in found):
        time.sleep(0.05)
    for pid in found:
        try:
            os.kill(pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    return sorted(found.values())


def run(key_mode, fault, seed, binary=SWARM, real_agent=None):
    """`real_agent` : commande d'un agent réel pour la préparation (tâche 11), sinon préparateur scripté."""
    if key_mode not in harness.KEY_MODES or fault not in harness.FAULTS:
        raise ValueError((key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir("banc-s-")
    try:
        return _run(run_dir, key_mode, fault, seed, started, Path(binary), real_agent)
    except harness.BenchError:
        raise
    except Exception as e:   # toute autre exception porte aussi la racine conservée
        raise harness.BenchError(f"exécution interrompue : {type(e).__name__}: {e}", run_dir) from e


def _run(run_dir, key_mode, fault, seed, started, binary, real_agent):
    if not binary.is_file():
        raise harness.BenchError(f"binaire swarm absent : {binary}", run_dir)
    faults = run_dir / "faults"
    harness.init_ledger(run_dir / "ledger.db", seed)
    token_file = run_dir / "settle.token"
    token_file.write_text(uuid.uuid4().hex)
    sw = Swarm(binary, run_dir)
    conductors, tasks, timed_out, agents, wid = [], {}, False, [], None
    cleanup = {"errors": [], "reaped": []}
    api_options = {"lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "fail_snapshot_once": faults / "snapshot-503.json" if fault == "F5" else None,
                   "token_file": token_file}
    log_path = run_dir / "conductor.log"
    try:
        with harness.Api(run_dir, **api_options) as api, open(log_path, "w") as log:
            try:
                sw.cli(["init"])
                providers = {"banc": {"command": sys.executable, "env_allow": [],
                                      "args": [harness.SCRIPTS["provider_s"], str(run_dir), str(binary), api.url,
                                               key_mode, fault, str(token_file)]}}
                if real_agent:
                    providers["banc-reel"] = {"command": sys.executable, "env_allow": ["HOME", "PATH"],
                                              "args": [harness.SCRIPTS["provider_real"], str(run_dir), str(binary),
                                                       api.url, "--", *real_agent]}
                (run_dir / ".swarm" / "providers.json").write_text(
                    json.dumps({"schema_version": 1, "providers": providers}))
                w = setup_work(sw, api.url, fault, "banc-reel" if real_agent else "banc")
                wid = w["id"]
                sw.cli(["autonomy", wid, "autonome", "2"])
                sw.cli(["mission", "start", wid], profile(run_dir))
                conductors.append(sw.conductor(log))
                if fault == "F2":
                    conductors.append(sw.conductor(log))
                await_listening(conductors, log_path, run_dir)
                if fault == "F2":
                    markers.mark(faults / "dual-launch.json", conductors=[c.pid for c in conductors])
                tasks, timed_out = watch(sw, wid, run_dir, fault, conductors, log)
                agents = sw.agents(wid)
            finally:
                cleanup["errors"] = stop(sw, wid, conductors)
    finally:   # API arrêtée par le bloc with ; tout reste lié à la racine est tué
        cleanup["reaped"] = reap(run_dir)
        (run_dir / "cleanup.json").write_text(json.dumps(cleanup, ensure_ascii=False, indent=2))
    return harness.finish(run_dir, condition="S", key_mode=key_mode, fault=fault, seed=seed, started=started,
                          declared_success=tasks.get("settle") == "accepted", timed_out=timed_out,
                          extra={"launches": len(agents), "task_status": tasks,
                                 "agent_status": [a["status"] for a in agents], "conductors": len(conductors),
                                 "cleanup": cleanup, "real_agent": real_agent[0] if real_agent else None})
