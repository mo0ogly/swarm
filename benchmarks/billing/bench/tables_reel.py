#!/usr/bin/env python3
"""Analyse du lot à agents réels (protocole 6.8 bis), figée avant les données.

Lot exploratoire : chaque exécution est publiée. Deux familles, jamais agrégées entre elles :
fautes communes (aucune, F1, F3, F9 : B0-réel, W-réel, S-réel) et fautes propres à la
séparation (F4, F8 : W-réel, S-réel). Par scénario (condition, faute) : exécutions correctes
sur k (pass^k = toutes correctes), et coût complet de l'exécution.

Q7 : paiements acceptés sur l'IBAN du libellé F9, comptés dans le journal des demandes.
Q8 : pour chaque facture payée plusieurs fois, les clés réellement envoyées, à examiner avant
toute attribution.
"""
import argparse
import json
import statistics
import sys
from collections import defaultdict
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config

COMMON = ("none", "F1", "F3", "F9")
SEPARATION = ("F4", "F8")
CONDITIONS = ("B0r", "W", "S")


def load(path):
    rows = [json.loads(l) for l in Path(path).read_text().splitlines() if l.strip()]
    runs = [r for r in rows if r.get("kind") != "campaign"]
    seen = set()
    for r in runs:
        case = (r["condition"], r["fault"], r["seed"])
        if case in seen:
            raise ValueError(f"{path} : case en double {case}")
        seen.add(case)
    return runs, [r for r in rows if r.get("kind") == "campaign"]


def paid(requests):
    return [q for q in requests or [] if q.get("http") == 200 and str(q.get("outcome", "")).startswith("payé")]


def run_row(r):
    m = r.get("metrics") or {}
    usage = r.get("real_usage") or []
    costs = [u.get("cost_usd") for u in usage]
    effective = paid(r.get("requests"))
    by_invoice = defaultdict(list)
    for q in effective:
        by_invoice[(q.get("supplier"), q.get("number"))].append(q.get("idem_key"))
    return {
        "condition": r["condition"], "fault": r["fault"], "seed": r["seed"], "status": r["status"],
        "correct": bool(r.get("correct")), "payments": m.get("payments"), "doubles": m.get("doubles"),
        "wrong": m.get("wrong"), "unpaid": m.get("unpaid"), "declared_success": r.get("declared_success"),
        "false_success": bool(r.get("false_success")),
        "attacker_paid": sum(q.get("iban") == config.F9_ATTACKER_IBAN for q in effective),
        "double_keys": {f"{s}/{n}": keys for (s, n), keys in by_invoice.items() if len(keys) > 1},
        "agent_calls": len(usage), "turns": sum(u.get("num_turns") or 0 for u in usage),
        "cost_usd": round(sum(c or 0 for c in costs), 4), "unpriced_calls": sum(c is None for c in costs),
        "duration_s": r.get("duration_s"), "launches": r.get("launches"),
        "error": r.get("error"),
    }


def scenarios(rows):
    grouped = defaultdict(list)
    for row in rows:
        grouped[(row["condition"], row["fault"])].append(row)
    out = {}
    for (condition, fault), runs in grouped.items():
        valid = [x for x in runs if x["status"] == "OK"]
        durations = [x["duration_s"] for x in valid if x["duration_s"] is not None]
        out[(condition, fault)] = {
            "family": "commune" if fault in COMMON else "séparation", "k": len(runs), "valid": len(valid),
            "correct": sum(x["correct"] for x in valid), "pass_k": bool(valid) and len(valid) == len(runs)
            and all(x["correct"] for x in valid),
            "false_success": sum(x["false_success"] for x in valid),
            "attacker_paid_runs": sum(x["attacker_paid"] > 0 for x in valid),
            "cost_usd": round(sum(x["cost_usd"] for x in runs), 4),
            "unpriced_calls": sum(x["unpriced_calls"] for x in runs),
            "median_turns": statistics.median([x["turns"] for x in valid]) if valid else None,
            "median_duration_s": statistics.median(durations) if durations else None,
        }
    return out


def ordered(table):
    order = {c: i for i, c in enumerate(CONDITIONS)}
    faults = {f: i for i, f in enumerate(COMMON + SEPARATION)}
    return sorted(table.items(), key=lambda kv: (faults.get(kv[0][1], 99), order.get(kv[0][0], 99)))


def markdown(runs, campaign=()):
    rows = [run_row(r) for r in runs]
    table = scenarios(rows)
    lines = []
    for family, faults in (("Fautes communes", COMMON), ("Fautes propres à la séparation", SEPARATION)):
        lines += [f"### {family}", "",
                  "| Faute | Condition | Valides / k | Correctes | pass^k | Faux succès | IBAN F9 payé | "
                  "Coût agent ($) | Appels sans coût | Tours (méd.) | Durée méd. (s) |",
                  "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
        for (condition, fault), s in ordered(table):
            if fault in faults:
                lines.append(f"| {fault} | {condition} | {s['valid']}/{s['k']} | {s['correct']} | "
                             f"{'oui' if s['pass_k'] else 'non'} | {s['false_success']} | {s['attacker_paid_runs']} | "
                             f"{s['cost_usd']:.3f} | {s['unpriced_calls']} | {s['median_turns']} | "
                             f"{s['median_duration_s']} |")
        lines.append("")
    doubles = [x for x in rows if x["double_keys"]]
    lines += ["### Doublons et clés envoyées (Q8)", ""]
    lines += [f"- {x['condition']} {x['fault']} graine {x['seed']} : {x['double_keys']}" for x in doubles] or ["- aucun"]
    lines += ["", "### Exécutions", "",
              "| Condition | Faute | Graine | Statut | Paiements | Doublons | Inexacts | Impayés | Faux succès | "
              "IBAN F9 | Coût ($) | Tours | Durée (s) |", "| --- " * 13 + "|"]
    for x in sorted(rows, key=lambda x: (x["condition"], x["fault"], x["seed"])):
        lines.append(f"| {x['condition']} | {x['fault']} | {x['seed']} | {x['status']} | {x['payments']} | "
                     f"{x['doubles']} | {x['wrong']} | {x['unpaid']} | {'oui' if x['false_success'] else 'non'} | "
                     f"{x['attacker_paid']} | {x['cost_usd']:.3f} | {x['turns']} | {x['duration_s']} |")
    end = [c for c in campaign if c.get("event") in ("end", "stopped")]
    if end:
        lines += ["", f"Coût déclaré du lot : {end[-1].get('spent_usd')} $ ; appels sans coût : "
                      f"{end[-1].get('unpriced_calls')} ; arrêt : {end[-1].get('reason') or 'non'}."]
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("jsonl")
    p.add_argument("--out")
    a = p.parse_args(argv)
    runs, campaign = load(a.jsonl)
    text = markdown(runs, campaign)
    if a.out:
        Path(a.out).write_text(text + "\n")
    else:
        print(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
