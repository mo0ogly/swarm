#!/usr/bin/env python3
"""Conditions B0 (sans moteur) et B1 (contrôle par appel).

Chaîne naïve : préparer, régler, puis revue finale après l'effet. Chaque faute
suit le scénario de la spécification ; F2 et F7 ne diffèrent que par l'ordre,
faute de moteur pour les distinguer.

Un règlement n'est réussi que sur le code 0 de settle ; F3 ne relance que sur
le crash injecté (137). Un échec du banc lui-même (préparation ratée, script
planté, exception) lève harness.BenchError : c'est une ERREUR, jamais une
mesure. La racine d'exécution est conservée comme preuve.
"""
import json
import subprocess
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, markers

REVIEW = {0: "conforme", config.EXIT_NONCONFORME: "anomalie", config.EXIT_ENVIRONMENT: "indisponible"}
SETTLE_CODES = {0, config.EXIT_REFUSED, config.EXIT_ENVIRONMENT, config.EXIT_DIGEST, config.EXIT_CRASH}


def tail(data):
    """Fin d'un stderr pour diagnostic, tronquée."""
    if isinstance(data, bytes):
        data = data.decode("utf-8", "replace")
    return data.strip()[-config.ERROR_TEXT_MAX:]


def published(stdout):
    """Objet JSON publié sur stdout, ou None."""
    try:
        data = json.loads(stdout)
    except ValueError:
        return None
    return data if isinstance(data, dict) else None


def published_measure(stdout):
    """Un verdict « non conforme » publie sa mesure ; un plantage Python sort aussi en 1, mais sans elle."""
    m = published(stdout)
    return m is not None and all(isinstance(m.get(k), int) for k in ("doubles", "wrong", "unpaid"))


def published_refusal(stdout):
    """Un refus de settle (code 2) publie son résultat ; argparse sort aussi en 2, mais sans lui."""
    o = published(stdout)
    return (o is not None and all(isinstance(o.get(k), int) for k in ("paid", "replayed"))
            and isinstance(o.get("refused"), list) and (bool(o["refused"]) or o.get("stopped") is not None))


def run(condition, key_mode, fault, seed):
    if condition not in ("B0", "B1") or key_mode not in harness.KEY_MODES or fault not in harness.FAULTS:
        raise ValueError((condition, key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir(f"banc-{condition.lower()}-")
    try:
        return _run(run_dir, condition, key_mode, fault, seed, started)
    except harness.BenchError:
        raise
    except Exception as e:   # toute autre exception porte aussi la racine conservée
        raise harness.BenchError(f"exécution interrompue : {type(e).__name__}: {e}", run_dir) from e


def _run(run_dir, condition, key_mode, fault, seed, started):
    faults = run_dir / "faults"
    harness.init_ledger(run_dir / "ledger.db", seed)
    launches, ok, review, timed_out = 0, [], None, False
    api_options = {"policy": "b1" if condition == "B1" else "none",
                   "lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "fail_snapshot": faults / "snapshot-503.json" if fault == "F5" else None}
    with harness.Api(run_dir, **api_options) as api:

        def prepare(name, mode="nominal", hang=False, fail=False):
            """Lance le préparateur ; seule une préparation conforme au scénario renvoie un lot."""
            nonlocal launches
            launches += 1
            lot = run_dir / f"lot-{name}.json"
            err_path = run_dir / f"preparer-{name}.err"
            args = [sys.executable, harness.SCRIPTS["preparer"], "--api", api.url, "--out", str(lot), "--mode", mode]
            if hang:
                args += ["--hang-after-write", "--marker", str(faults / "budget-loop.json")]
            if fail:
                args.append("--fail-after-write")
            with open(err_path, "wb") as err:   # fichier, pas un tube : un préparateur bloqué ne peut pas s'y bloquer
                proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=err)
                expired = False
                try:
                    proc.wait(timeout=config.B_PREPARE_TIMEOUT_S)
                except subprocess.TimeoutExpired:   # garde-fou externe : le lot partiel reste sur disque
                    expired = True
                finally:
                    harness.stop(proc)   # F6 : le préparateur bloqué est tué, jamais laissé derrière
            if hang:   # F6 : le délai est attendu, l'arrêt forcé est normal
                expected = expired
            else:      # F8 a1 : échec simulé après écriture, code 1 attendu
                expected = not expired and proc.returncode == (config.EXIT_NONCONFORME if fail else 0)
            if not expected:
                outcome = "délai dépassé" if expired else f"code {proc.returncode}"
                raise harness.BenchError(f"préparation {name} ratée ({outcome}) : {tail(err_path.read_bytes())}",
                                         run_dir)
            try:
                if not isinstance(json.loads(lot.read_text())["lines"], list):
                    raise TypeError("lines n'est pas une liste")
            except (OSError, ValueError, KeyError, TypeError) as e:
                raise harness.BenchError(f"préparation {name} : lot absent ou illisible ({e})", run_dir) from e
            return lot

        def settle(lot, attempt, progress, crash=False):
            nonlocal launches
            launches += 1
            args = [sys.executable, harness.SCRIPTS["settle"], "--api", api.url, "--lot", str(lot),
                    "--key-mode", key_mode, "--attempt", attempt,
                    "--progress", str(run_dir / f"progress-{progress}.jsonl")]
            if crash:
                args += ["--crash-after-pay", "1", "--crash-marker", str(faults / "crash-before-record.json")]
            done = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S)
            if done.returncode not in SETTLE_CODES or (done.returncode == config.EXIT_REFUSED
                                                       and not published_refusal(done.stdout)):
                raise harness.BenchError(f"règlement {attempt} planté (code {done.returncode}) : {tail(done.stderr)}",
                                         run_dir)
            return done.returncode

        def final_review():
            args = [sys.executable, harness.SCRIPTS["verify_settlement"], "--api", api.url]
            done = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S)
            if done.returncode == config.EXIT_ENVIRONMENT:   # relance naïve, une fois
                done = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S)
            if done.returncode not in REVIEW or (done.returncode == config.EXIT_NONCONFORME
                                                 and not published_measure(done.stdout)):
                raise harness.BenchError(f"revue finale plantée (code {done.returncode}) : {tail(done.stderr)}",
                                         run_dir)
            return REVIEW[done.returncode]

        try:
            if fault in ("none", "F1", "F5"):
                ok.append(settle(prepare("a1"), "a1", "a1") == 0)
            elif fault == "F2":
                first, second = prepare("a1"), prepare("a2")
                markers.mark(faults / "dual-launch.json", launchers=2,
                             lot_sha256={"a1": harness.file_sha256(first), "a2": harness.file_sha256(second)})
                ok += [settle(first, "a1", "a1") == 0, settle(second, "a2", "a2") == 0]
            elif fault == "F3":
                lot = prepare("a1")
                code = settle(lot, "a1", "a1", crash=True)
                if code == config.EXIT_CRASH:   # relance naïve : même progression, nouvelle exécution
                    code = settle(lot, "a1-relance", "a1")
                ok.append(code == 0)
            elif fault in ("F4", "F4e"):   # sans moteur, F4e n'a pas de « départ » distinct : même injection
                lot = prepare("a1")
                name = harness.EXPECTED_MARKER[fault]
                harness.tamper(lot, faults / f"{name}.json", fault=name)
                ok.append(settle(lot, "a1", "a1") == 0)
            elif fault == "F6":
                ok.append(settle(prepare("a1", mode="partial", hang=True), "a1", "a1") == 0)
            elif fault == "F7":
                stalled = prepare("a1")
                markers.mark(faults / "owner-stalled.json", owner="a1", lot_sha256=harness.file_sha256(stalled))
                ok.append(settle(prepare("a2"), "a2", "a2") == 0)
                ok.append(settle(stalled, "a1", "a1") == 0)
            elif fault == "F8":
                stale = prepare("a1", mode="wrong-amount", fail=True)
                prepare("a2")   # le bon lot est annoncé, puis supplanté par l'annonce tardive de a1
                deltas = harness.amount_deltas(run_dir / "ledger.db", stale)
                if deltas:   # sans écart observé, pas de marqueur : l'exécution sera INVALIDE
                    markers.mark(faults / "stale-report.json", announced="lot-a1.json",
                                 lot_sha256=harness.file_sha256(stale), amount_delta_cents=sum(deltas),
                                 lines_with_delta=len(deltas))
                ok.append(settle(stale, "a1", "a1") == 0)
            review = final_review()
        except subprocess.TimeoutExpired:   # subprocess.run a déjà tué l'enfant
            timed_out = True
    return harness.finish(run_dir, condition=condition, key_mode=key_mode, fault=fault, seed=seed, started=started,
                          declared_success=bool(ok) and all(ok) and not timed_out, timed_out=timed_out,
                          extra={"launches": launches, "final_review": review})
