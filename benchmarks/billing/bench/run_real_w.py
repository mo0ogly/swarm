"""Condition W-réel : mêmes composants que S-réel, enchaînés par un workflow fixe, sans Swarm.

Isole l'apport du moteur. Le même agent réel prépare le lot avec la même consigne
(provider_real.prompt_for) ; le même contrôle (check_lot) le valide ; le même règlement
déterministe (settle) le paie avec le jeton, la clé métier et l'empreinte du lot contrôlé ;
la même revue (verify_settlement) relit le grand livre. L'enchaînement reproduit les règles
observées du moteur, sans son état durable ni ses baux :

1. préparation par l'agent, puis contrôle ; contrôle en échec : une seule correction (nouvelle
   préparation, nouveau contrôle), comme `PlanMaxAttempts` = 2 ; second échec : arrêt, rien payé ;
2. règlement du lot contrôlé, `--expected-sha256` = empreinte au contrôle (E3) ;
3. arrêt brutal du règlement (137) : une reprise, même progression, nouvelle tentative ;
4. revue finale.

L'agent n'a jamais le jeton (E1). Fautes : harness.REAL_FAULTS["W"], injectées comme en S-réel.
"""
import json
import subprocess
import sys
import time
import uuid
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import agent_real, config, harness, markers, preparer
from bench.provider_real import lot_readable, prompt_for
from bench.run_b import REVIEW

KEY = "business"


def run(key_mode, fault, seed, model=config.REAL_MODEL):
    if key_mode != KEY or fault not in harness.REAL_FAULTS["W"]:
        raise ValueError((key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir("banc-wr-")
    try:
        return _run(run_dir, fault, seed, started, model)
    except harness.BenchError:
        raise
    except Exception as e:
        raise harness.BenchError(f"exécution interrompue : {type(e).__name__}: {e}", run_dir) from e


def _run(run_dir, fault, seed, started, model):
    harness.init_ledger(run_dir / "ledger.db", seed)
    faults, workspace = run_dir / "faults", run_dir / "agent"
    (workspace / "docs").mkdir(parents=True)
    lot = workspace / "docs" / "prepare.md"
    token_file = run_dir / "settle.token"
    token_file.write_text(uuid.uuid4().hex)
    usage = run_dir / "real-usage.jsonl"
    api_options = {"lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "inject_memo": faults / "memo-injected.json" if fault == "F9" else None,
                   "request_log": run_dir / "requests.jsonl"}
    steps, settled, stopped_at = [], None, None
    with harness.Api(run_dir, policy="none", token_file=token_file, **api_options) as api:

        def check():
            done = subprocess.run([sys.executable, harness.SCRIPTS["check_lot"], "--api", api.url, "--lot", str(lot)],
                                  capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S)
            steps.append({"step": "check_lot", "code": done.returncode})
            return done.returncode

        def settle(attempt, crash=False):
            args = [sys.executable, harness.SCRIPTS["settle"], "--api", api.url, "--lot", str(lot),
                    "--key-mode", KEY, "--attempt", attempt, "--progress", str(run_dir / "progress.jsonl"),
                    "--expected-sha256", checked_sha, "--token-file", str(token_file)]
            if crash:
                args += ["--crash-after-pay", "1", "--crash-marker", str(faults / "crash-before-record.json")]
            done = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S)
            steps.append({"step": "settle", "attempt": attempt, "code": done.returncode})
            return done.returncode

        checked_sha = None
        for rank in (1, 2):   # préparation, puis une seule correction
            lot.unlink(missing_ok=True)   # écriture sans lecture préalable impossible : lot précédent retiré
            code, _ = agent_real.run(prompt_for(api.url), workspace, model, usage, f"W-prepare-{rank}")
            steps.append({"step": "prepare", "rank": rank, "code": code})
            if code or not lot_readable(lot):
                continue
            if fault == "F8" and rank == 1:   # rapport ancien : lot faussé à la première tentative
                data = json.loads(lot.read_text())
                if data["lines"]:
                    data["lines"][0]["amount_cents"] += config.TAMPER_DELTA_CENTS
                    sha = preparer.write_lot(lot, data)
                    markers.mark(faults / "stale-report.json", attempt=f"p{rank}", lot_sha256=sha)
            if check() == 0:
                checked_sha = harness.file_sha256(lot)
                break
        if checked_sha is None:
            stopped_at = "contrôle"
        else:
            if fault == "F4":   # candidat modifié entre le contrôle et le règlement
                harness.tamper(lot, faults / "lot-tampered.json")
            settled = settle("s1", crash=fault == "F3")
            if settled == config.EXIT_CRASH:
                settled = settle("s2")
            if settled != 0:
                stopped_at = "règlement"
        review = subprocess.run([sys.executable, harness.SCRIPTS["verify_settlement"], "--api", api.url],
                                capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S).returncode
        if review not in REVIEW:
            raise harness.BenchError(f"revue finale plantée (code {review})", run_dir)
    usage_lines = agent_real.read_usage(usage)
    return harness.finish(run_dir, condition="W", key_mode=KEY, fault=fault, seed=seed, started=started,
                          declared_success=settled == 0, timed_out=any(u.get("timed_out") for u in usage_lines),
                          extra={"launches": len(steps), "steps": steps, "stopped_at": stopped_at,
                                 "final_review": REVIEW[review], "real_agent": model, "real_usage": usage_lines,
                                 "requests": harness.read_jsonl(run_dir / "requests.jsonl")})
