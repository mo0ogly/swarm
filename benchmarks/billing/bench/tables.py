#!/usr/bin/env python3
"""Tableaux de l'article à partir d'un JSONL de campagne (protocole, sections 6.6 à 6.11).

Tableau principal : une ligne par case (condition × clé × faute). Seules les exécutions OK entrent
dans les taux ; les nombres d'INVALIDE, DÉLAI et ERREUR sont publiés, et une case dont plus de
10 % des exécutions sont exclues est signalée (6.7). Taux avec intervalle de Wilson à 95 % (6.11) ;
doublons, paiements inexacts et impayés sont comptés séparément, jamais additionnés (6.6). Durées :
médiane et intervalle interquartile des exécutions OK.

Tableaux annexes : répartition de `stopped_by` sous F4 et F4e (moteur, règlement, aucun), et
nombre d'exécutions F5 où la faute a été absorbée (`f5_absorbed`).

Les lignes `kind: "campaign"` (empreintes, avertissements, interruption) ne sont pas des exécutions :
leurs avertissements sont repris en tête du tableau.
"""
import argparse
import json
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

EXCLUSION_LIMIT = 0.10        # protocole 6.7 : au-delà, la case est signalée et discutée
STOPPERS = ("engine", "settlement", "none")


def wilson(k, n, z=1.96):
    """Intervalle de score de Wilson pour k succès sur n ; (0, 1) sans exécution."""
    if n == 0:
        return 0.0, 1.0
    p = k / n
    d = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / d
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return max(0.0, centre - half), min(1.0, centre + half)


def load(path):
    """(exécutions, lignes de campagne) d'un JSONL ; une ligne illisible est une erreur, jamais ignorée."""
    runs, campaign = [], []
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except ValueError as e:
            raise ValueError(f"{path}:{number} : ligne JSON illisible ({e})") from e
        (campaign if record.get("kind") == "campaign" else runs).append(record)
    seen = set()
    for r in runs:   # une case comptée deux fois fausserait tous les taux
        case = (r["condition"], r["key_mode"], r["fault"], r["seed"])
        if case in seen:
            raise ValueError(f"{path} : case en double {case}")
        seen.add(case)
    return runs, campaign


def cells(records):
    grouped = defaultdict(list)
    for r in records:
        grouped[(r["condition"], r["key_mode"], r["fault"])].append(r)
    return dict(sorted(grouped.items()))


def interquartile(values):
    if len(values) == 1:
        return values[0], values[0]
    q1, _, q3 = statistics.quantiles(values, n=4, method="inclusive")
    return q1, q3


def summarize(records):
    out = {}
    for cell, runs in cells(records).items():
        ok = [r for r in runs if r["status"] == "OK"]
        durations = sorted(r["duration_s"] for r in ok)
        excluded = len(runs) - len(ok)
        out[cell] = {
            "total": len(runs), "ok": len(ok), "excluded": excluded,
            "invalid": sum(r["status"] == "INVALIDE" for r in runs),
            "timeout": sum(r["status"] == "DÉLAI" for r in runs),
            "error": sum(r["status"] == "ERREUR" for r in runs),
            "flagged": excluded > EXCLUSION_LIMIT * len(runs),
            "double_runs": sum(r["metrics"]["doubles"] > 0 for r in ok),
            "wrong_runs": sum(r["metrics"]["wrong"] > 0 for r in ok),
            "unpaid_runs": sum(r["metrics"]["unpaid"] > 0 for r in ok),
            "false_success_runs": sum(bool(r["false_success"]) for r in ok),
            "median_duration_s": statistics.median(durations) if ok else None,
            "iqr_duration_s": interquartile(durations) if ok else None,
        }
    return out


def stopped_by(records):
    """Sous F4 et F4e, exécutions OK par auteur de l'arrêt du paiement (S seulement : B n'a pas de moteur)."""
    out = {}
    for cell, runs in cells(r for r in records if r["fault"] in ("F4", "F4e") and "stopped_by" in r).items():
        counts = dict.fromkeys(STOPPERS, 0)
        for r in runs:
            if r["status"] == "OK" and r.get("stopped_by") in counts:
                counts[r["stopped_by"]] += 1
        out[cell] = counts
    return out


def f5_absorbed(records):
    """Sous F5, exécutions OK et, parmi elles, celles où l'instantané a fini par être servi."""
    return {cell: {"ok": sum(r["status"] == "OK" for r in runs),
                   "absorbed": sum(r["status"] == "OK" and bool(r.get("f5_absorbed")) for r in runs)}
            for cell, runs in cells(r for r in records if r["fault"] == "F5").items()}


def rate(k, n):
    low, high = wilson(k, n)
    return f"{k}/{n} [{low:.2f} ; {high:.2f}]"


def seconds(value):
    return "—" if value is None else f"{value:.1f}"


def markdown(summary, campaign=(), records=()):
    lines = []
    warnings = [c["warning"] for c in campaign if c.get("warning")]
    if warnings:
        lines += ["**Avertissements de campagne :**", ""] + [f"- {w}" for w in warnings] + [""]
    lines += ["| Condition | Clé | Faute | Valides / total | Doublons | Inexacts | Impayés | Faux succès |"
              " Invalides | Délais | Erreurs | Durée médiane (s) [IQR] | Signalement |",
              "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for (condition, key, fault), s in summary.items():
        n = s["ok"]
        iqr = s["iqr_duration_s"]
        duration = "—" if iqr is None else f"{seconds(s['median_duration_s'])} [{seconds(iqr[0])} ; {seconds(iqr[1])}]"
        flag = f"signalée : {s['excluded']}/{s['total']} exclues" if s["flagged"] else ""
        lines.append(f"| {condition} | {key} | {fault} | {n}/{s['total']} | {rate(s['double_runs'], n)} | "
                     f"{rate(s['wrong_runs'], n)} | {rate(s['unpaid_runs'], n)} | "
                     f"{rate(s['false_success_runs'], n)} | {s['invalid']} | {s['timeout']} | {s['error']} | "
                     f"{duration} | {flag} |")
    stops = stopped_by(records)
    if stops:
        lines += ["", "**F4 et F4e — auteur de l'arrêt du paiement (exécutions OK)**", "",
                  "| Condition | Clé | Faute | Moteur | Règlement | Aucun |", "| --- | --- | --- | --- | --- | --- |"]
        lines += [f"| {c} | {k} | {f} | {v['engine']} | {v['settlement']} | {v['none']} |"
                  for (c, k, f), v in stops.items()]
    absorbed = f5_absorbed(records)
    if absorbed:
        lines += ["", "**F5 — faute absorbée (instantané servi pendant l'exécution)**", "",
                  "| Condition | Clé | Valides | Absorbées |", "| --- | --- | --- | --- |"]
        lines += [f"| {c} | {k} | {v['ok']} | {v['absorbed']} |" for (c, k, _), v in absorbed.items()]
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("jsonl")
    p.add_argument("--out", help="fichier Markdown ; sinon sortie standard")
    a = p.parse_args(argv)
    runs, campaign = load(a.jsonl)
    text = markdown(summarize(runs), campaign, runs) + "\n"
    if a.out:
        Path(a.out).write_text(text)
    else:
        sys.stdout.write(text)
    return 0


if __name__ == "__main__":
    sys.exit(main())
