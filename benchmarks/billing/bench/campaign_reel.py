#!/usr/bin/env python3
"""Lot à agents réels : B0-réel, W-réel et S-réel, clé métier, k essais par faute, en série.

Grille : harness.REAL_FAULTS × REAL_REPS graines (seed-base + rep). Chaque exécution est écrite
en une ligne JSONL dès sa fin ; une case déjà présente est sautée à la reprise. Avant chaque
case, le coût cumulé déclaré par l'agent (`real_usage[].cost_usd`) est comparé au plafond
REAL_COST_CEILING_USD : c'est un seuil d'arrêt sur consommation déclarée, pas une borne stricte
(la dernière case lancée peut le dépasser). Un appel tué (F3) ne déclare pas de coût : pour la
décision d'arrêt, chacun est compté au coût maximal déclaré par un appel du banc jusque-là ; la
ligne de fin publie séparément le coût déclaré et le nombre d'appels sans coût.
"""
import argparse
import json
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, run_real_b, run_real_w, run_s
from bench.campaign import append

KEY = "business"


def plan(reps, seed_base):
    return [(condition, fault, seed_base + rep) for condition, faults in harness.REAL_FAULTS.items()
            for fault in faults for rep in range(reps)]


def one(condition, fault, seed, model):
    try:
        if condition == "B0r":
            return run_real_b.run(KEY, fault, seed, model)
        if condition == "W":
            return run_real_w.run(KEY, fault, seed, model)
        return run_s.run(KEY, fault, seed, real_agent=[model])
    except Exception as e:   # publiée comme ERREUR ; les appels déjà engagés restent comptabilisés
        run_dir = getattr(e, "run_dir", None)
        usage_error = None
        try:
            usage = []
            if run_dir:
                usage_path = Path(run_dir) / "real-usage.jsonl"
                if not usage_path.exists():
                    usage_error = "journal de consommation absent"
                else:
                    for line in usage_path.read_text().splitlines():
                        if not line.strip():
                            continue
                        try:
                            entry = json.loads(line)
                            if not isinstance(entry, dict):
                                raise ValueError("entrée non objet")
                            usage.append(entry)
                        except ValueError:
                            usage.append({"cost_usd": None, "invalid_usage_entry": True})
                            usage_error = "journal de consommation partiellement illisible"
        except (OSError, ValueError) as read_error:
            usage_error = str(read_error)[:config.ERROR_TEXT_MAX]
        return {"condition": condition, "key_mode": KEY, "fault": fault, "seed": seed, "status": "ERREUR",
                "error": f"{type(e).__name__}: {e}"[:config.ERROR_TEXT_MAX * 3],
                "run_dir": run_dir, "real_usage": usage, "usage_recovery_error": usage_error, "usage_complete": run_dir is not None and usage_error is None, "provenance": dict(harness.provenance()), "real_agent": model}


def cost(record):
    return sum(u.get("cost_usd") or 0 for u in record.get("real_usage") or [])


def unpriced(record):
    return sum(1 for u in record.get("real_usage") or [] if u.get("cost_usd") is None)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", required=True)
    p.add_argument("--reps", type=int, default=config.REAL_REPS)
    p.add_argument("--seed-base", type=int, default=1000)
    p.add_argument("--model", default=config.REAL_MODEL)
    p.add_argument("--ceiling-usd", type=float, default=config.REAL_COST_CEILING_USD)
    p.add_argument("--only", action="append", default=[], metavar="CONDITION:FAUTE",
                   help="limiter la grille à ces scénarios (rejeu après correction du banc)")
    a = p.parse_args(argv)
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    previous = [json.loads(l) for l in out.read_text().splitlines() if l.strip()] if out.exists() else []
    runs = [r for r in previous if r.get("kind") != "campaign"]
    done = {(r["condition"], r["fault"], r["seed"]) for r in runs}
    spent, missing = sum(cost(r) for r in runs), sum(unpriced(r) for r in runs)
    worst = max([u.get("cost_usd") or 0 for r in runs for u in r.get("real_usage") or []] + [0])
    only = {tuple(x.split(":", 1)) for x in a.only}
    todo = [c for c in plan(a.reps, a.seed_base) if c not in done and (not only or (c[0], c[1]) in only)]
    append(out, {"kind": "campaign", "event": "start", "at": time.time(), "todo": len(todo), "spent_usd": spent,
                 "args": vars(a), **harness.provenance()})
    stopped = ("consommation historique non récupérable intégralement : reprise suspendue, coût inconnu"
               if any(r.get("usage_complete") is False for r in runs) else None)
    for condition, fault, seed in todo:
        if stopped:
            break
        charged = spent + missing * worst
        if charged >= a.ceiling_usd:
            stopped = (f"seuil d'arrêt atteint : {charged:.2f} $ >= {a.ceiling_usd:.2f} $ "
                       f"(déclaré {spent:.2f} $, {missing} appel(s) sans coût)")
            break
        record = one(condition, fault, seed, a.model)
        spent += cost(record)
        missing += unpriced(record)
        worst = max([worst] + [u.get("cost_usd") or 0 for u in record.get("real_usage") or []])
        append(out, record)
        print(f"{condition} {fault} {seed} {record['status']} cumul {spent:.2f} $", flush=True)
        if record.get("usage_complete") is False:
            stopped = "consommation non récupérable intégralement : arrêt conservateur, coût inconnu"
            break
    append(out, {"kind": "campaign", "event": "stopped" if stopped else "end", "at": time.time(),
                 "spent_usd": round(spent, 4), "unpriced_calls": missing, "reason": stopped})
    return 3 if stopped else 0


if __name__ == "__main__":
    sys.exit(main())
