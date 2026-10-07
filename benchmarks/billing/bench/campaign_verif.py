#!/usr/bin/env python3
"""Campagne de vérification des correctifs du moteur (D1, D2, D3, D5, D6), agents scriptés, condition S.

Grille (docs/benchmarks/billing/verification-correctifs.md) : F4e et F5 sous les trois clés ;
variantes nodeps, noguard et ownws sans faute, clé métier ; non-régression clé métier sur les
autres cas. Une exécution par case et par graine, en série. Reprise : une case présente (clé,
faute, variante, graine) est sautée. Une exception devient une ligne ERREUR, conservée.
Empreintes du banc et du binaire en début et en fin, comme campaign.py.
"""
import argparse
import json
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, run_s
from bench.campaign import append, publish

KEYS = ("none", "attempt", "business")
REGRESSION = ("none", "F1", "F2", "F3", "F4", "F6", "F7", "F8")


def plan(reps, seed_base):
    cases = [(k, f, None) for f in ("F4e", "F5") for k in KEYS]
    cases += [("business", "none", v) for v in ("nodeps", "noguard", "ownws")]
    cases += [("business", f, None) for f in REGRESSION]
    return [(k, f, v, seed_base + r) for k, f, v in cases for r in range(reps)]


def one(key, fault, variant, seed):
    try:
        return {**run_s.run(key, fault, seed, variant=variant), "variant": variant}
    except Exception as e:   # publiée comme ERREUR, jamais comme mesure
        return {"condition": "S", "key_mode": key, "fault": fault, "variant": variant, "seed": seed,
                "status": "ERREUR", "error": f"{type(e).__name__}: {e}"[:config.ERROR_TEXT_MAX * 3],
                "run_dir": getattr(e, "run_dir", None), "provenance": dict(harness.provenance())}


def case_of(r):
    return (r["key_mode"], r["fault"], r.get("variant"), r["seed"])


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", required=True)
    p.add_argument("--reps", type=int, default=100)
    p.add_argument("--seed-base", type=int, default=1000)
    p.add_argument("--purge-ok", action="store_true")
    a = p.parse_args(argv)
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    done = set()
    if out.exists():
        for line in out.read_text().splitlines():
            r = json.loads(line)
            if r.get("kind") != "campaign":
                done.add(case_of(r))
    todo = [c for c in plan(a.reps, a.seed_base) if c not in done]
    start = dict(harness.provenance())
    begin = time.time()
    append(out, {"kind": "campaign", "event": "start", "at": begin, "todo": len(todo), "args": vars(a), **start})
    interrupted = False
    try:
        for key, fault, variant, seed in todo:
            publish(out, one(key, fault, variant, seed), a.purge_ok)
    except KeyboardInterrupt:
        interrupted = True
    bench_end = harness.bench_digest(harness.BENCH_DIR)
    swarm_end = harness.file_sha256(harness.SWARM_BIN) if harness.SWARM_BIN.is_file() else None
    end = {"kind": "campaign", "event": "interrupted" if interrupted else "end", "at": time.time(),
           "duration_s": round(time.time() - begin, 1), "bench_sha256": bench_end, "swarm_sha256": swarm_end,
           "bench_changed": bench_end != start["bench_sha256"], "swarm_changed": swarm_end != start["swarm_sha256"]}
    append(out, end)
    return 130 if interrupted else 0


if __name__ == "__main__":
    sys.exit(main())
