"""Outils communs aux exécutions : racine isolée, API, mesure finale, enregistrement."""
import functools
import hashlib
import json
import os
import platform
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from bench import config, invoices, ledger, markers, metrics, preparer

BENCH_DIR = Path(__file__).resolve().parent
SCRIPTS = {name: str(BENCH_DIR / f"{name}.py") for name in (
    "payment_api", "preparer", "settle", "check_lot", "verify_settlement", "provider_s", "provider_real")}
EXPECTED_MARKER = {"F1": "response-lost", "F2": "dual-launch", "F3": "crash-before-record", "F4": "lot-tampered",
                   "F5": "snapshot-503", "F6": "budget-loop", "F7": "owner-stalled", "F8": "stale-report"}
FAULTS = ("none",) + tuple(EXPECTED_MARKER)
KEY_MODES = ("none", "attempt", "business")
REPO_ROOT = BENCH_DIR.parents[2]          # racine du dépôt swarm : benchmarks/billing/bench -> racine
# Binaire mesuré : `make build` produit bin/swarm ; BANC_SWARM_BIN permet de pointer un binaire construit ailleurs.
SWARM_BIN = Path(os.environ.get("BANC_SWARM_BIN") or REPO_ROOT / "bin" / "swarm")


class BenchError(Exception):
    """Le banc lui-même a échoué (préparation ratée, script planté, exception) : ERREUR, jamais une mesure.

    `run_dir` désigne la racine conservée comme preuve.
    """

    def __init__(self, message, run_dir):
        super().__init__(message)
        self.run_dir = str(run_dir)


def file_sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def bench_digest(directory):
    """SHA-256 de la concaténation triée (nom puis contenu) des `*.py` du dossier."""
    digest = hashlib.sha256()
    for path in sorted(Path(directory).glob("*.py")):
        digest.update(path.name.encode() + b"\0" + path.read_bytes() + b"\0")
    return digest.hexdigest()


def init_ledger(db, seed):
    conn = ledger.connect(str(db))
    invoices.generate(conn, seed)
    conn.close()


def new_run_dir(prefix):
    run_dir = Path(tempfile.mkdtemp(prefix=prefix))
    (run_dir / "faults").mkdir()
    return run_dir


def stop(proc):
    """Arrête un sous-processus : terminate, puis kill s'il ne sort pas."""
    if proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()


class Api:
    """API de paiement en sous-processus, arrêtée à la sortie du bloc `with`, même sur exception."""

    def __init__(self, run_dir, policy="none", lose_response_once=None, fail_snapshot_once=None, token_file=None):
        self.port_file = run_dir / "api.port"
        self.log_path = run_dir / "api.log"
        self.args = [sys.executable, SCRIPTS["payment_api"], "--db", str(run_dir / "ledger.db"),
                     "--port-file", str(self.port_file), "--policy", policy]
        for flag, value in (("--lose-response-once", lose_response_once),
                            ("--fail-snapshot-once", fail_snapshot_once), ("--token-file", token_file)):
            if value:
                self.args += [flag, str(value)]

    def __enter__(self):
        self.log = open(self.log_path, "w")
        try:
            self.proc = subprocess.Popen(self.args, stdout=self.log, stderr=subprocess.STDOUT)
        except BaseException:
            self.log.close()
            raise
        try:
            deadline = time.monotonic() + config.API_START_TIMEOUT_S
            while not self.port_file.exists():
                if self.proc.poll() is not None or time.monotonic() > deadline:
                    raise RuntimeError(f"API de paiement non démarrée, voir {self.log_path}")
                time.sleep(0.05)
            self.url = f"http://127.0.0.1:{self.port_file.read_text()}"
        except BaseException:   # __exit__ n'est pas appelé si __enter__ échoue
            self.__exit__()
            raise
        return self

    def __exit__(self, *exc):
        try:
            stop(self.proc)
        finally:
            self.log.close()


def amount_deltas(db, lot_path):
    """Écarts (montant du lot − montant de la facture) des lignes d'un lot qui diffèrent de leur facture."""
    conn = ledger.connect(str(db))
    try:
        due = {(r[0], r[1]): r[2] for r in conn.execute("SELECT supplier, number, amount_cents FROM invoices")}
    finally:
        conn.close()
    lines = json.loads(Path(lot_path).read_text())["lines"]
    return [line["amount_cents"] - due[(line["supplier"], line["number"])] for line in lines
            if (line["supplier"], line["number"]) in due and line["amount_cents"] != due[(line["supplier"], line["number"])]]


def tamper(lot_path, marker):
    """F4 : modifie le montant de la première ligne après validation."""
    before = hashlib.sha256(Path(lot_path).read_bytes()).hexdigest()
    lot = json.loads(Path(lot_path).read_text())
    lot["lines"][0]["amount_cents"] += config.TAMPER_DELTA_CENTS
    after = preparer.write_lot(lot_path, lot)
    markers.mark(marker, fault="lot-tampered", before=before, after=after)


@functools.lru_cache(maxsize=None)
def provenance():
    """Code mesuré : commit, état du dépôt, empreintes du banc et du binaire swarm. Calculée une fois par processus.

    Un échec de git est dit (git_dirty à None, git_error), jamais pris pour propre. L'objet renvoyé est
    partagé : finish en insère une copie.

    Les empreintes sont figées au premier appel : ne pas modifier le banc pendant une campagne. La
    campagne recalcule l'empreinte à la fin et la compare.
    """
    git = ["git", "-C", str(BENCH_DIR)]
    env = {**os.environ, "GIT_OPTIONAL_LOCKS": "0"}   # lecture seule : ne pas rafraîchir l'index du dépôt
    record = {"git_commit": "", "git_dirty": None, "python": platform.python_version(),
              "bench_sha256": bench_digest(BENCH_DIR),
              "swarm_sha256": file_sha256(SWARM_BIN) if SWARM_BIN.is_file() else None}
    try:
        head = subprocess.run(git + ["rev-parse", "HEAD"], capture_output=True, text=True, timeout=30, env=env)
        status = subprocess.run(git + ["status", "--porcelain", "--", str(REPO_ROOT)],
                                capture_output=True, text=True, timeout=30, env=env)
    except (OSError, subprocess.TimeoutExpired) as e:
        return {**record, "git_error": f"{type(e).__name__}: {e}"}
    errors = [f"{name} : code {p.returncode} : {p.stderr.strip()[:config.ERROR_TEXT_MAX]}"
              for name, p in (("rev-parse", head), ("status", status)) if p.returncode != 0]
    if head.returncode == 0:
        record["git_commit"] = head.stdout.strip()
    if status.returncode == 0:
        record["git_dirty"] = bool(status.stdout.strip())
    if errors:
        record["git_error"] = " ; ".join(errors)
    return record


def finish(run_dir, *, condition, key_mode, fault, seed, started, declared_success, timed_out, extra, bank_dir=None):
    """Mesure après arrêt de l'API : le grand livre ne bouge plus.

    `bank_dir` : dossier du grand livre quand il est séparé de la racine d'exécution (condition S) ;
    par défaut la racine elle-même (B0, B1).
    """
    conn = ledger.connect(str(Path(bank_dir or run_dir) / "ledger.db"))
    try:
        m = metrics.measure(conn, config.CAP_CENTS)
    finally:
        conn.close()
    seen = markers.read_all(run_dir / "faults")
    expected = EXPECTED_MARKER.get(fault)
    status = "DÉLAI" if timed_out else ("INVALIDE" if expected and expected not in seen else "OK")
    return {"condition": condition, "key_mode": key_mode, "fault": fault, "seed": seed, "status": status,
            "markers": seen, "metrics": m, "correct": metrics.correct(m), "declared_success": declared_success,
            "false_success": bool(declared_success) and not metrics.correct(m),
            "duration_s": round(time.monotonic() - started, 3), "run_dir": str(run_dir),
            "provenance": dict(provenance()), **extra}
