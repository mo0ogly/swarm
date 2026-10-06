#!/usr/bin/env python3
"""Verdicts mécaniques sur H1 à H5 (protocole 6.10), figés avant la fin de la campagne.

Règles, appliquées aux seules exécutions OK (6.7) :
- un taux « nul » est confirmé si aucune exécution ne présente le défaut (k = 0) ; la borne haute
  de Wilson est rapportée ;
- un taux « non nul » est confirmé si au moins une exécution le présente (k > 0) ;
- une case sans exécution valide rend l'hypothèse non concluante.
H5 compare les durées médianes de S et de B0 sans faute, clé par clé. H6 relève de l'étiquetage
(section 7) : non calculée ici. F4e est rapportée à part : elle n'existait pas lors de la
formulation de H2.
"""
import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import tables

KEYS = ("none", "attempt", "business")


def cell(summary, condition, key, fault):
    return summary.get((condition, key, fault))


def check(summary, expectations):
    """expectations : [(condition, clé, faute, mesure, 'nul' | 'non nul')] -> (verdict, détails)."""
    details, verdict = [], "confirmée"
    for condition, key, fault, measure, expected in expectations:
        s = cell(summary, condition, key, fault)
        if not s or s["ok"] == 0:
            details.append(f"{condition}/{key}/{fault} : aucune exécution valide")
            verdict = "non concluante" if verdict == "confirmée" else verdict
            continue
        k, n = s[measure], s["ok"]
        low, high = tables.wilson(k, n)
        holds = (k == 0) if expected == "nul" else (k > 0)
        details.append(f"{condition}/{key}/{fault} {measure} {k}/{n} [{low:.2f} ; {high:.2f}] attendu {expected}"
                       + ("" if holds else " : ÉCART"))
        if not holds:
            verdict = "infirmée"
    return verdict, details


def verdicts(records):
    summary = tables.summarize(records)
    out = {}
    out["H1"] = check(summary, [(c, "business", f, "double_runs", "nul")
                                for c in ("B0", "B1", "S") for f in ("F1", "F2", "F3", "F7")])
    out["H2"] = check(summary, [(c, k, f, "wrong_runs", "non nul") for c in ("B0", "B1") for k in KEYS
                                for f in ("F4", "F8")]
                      + [("S", k, f, "wrong_runs", "nul") for k in KEYS for f in ("F4", "F8")])
    out["H2 (F4e, à part)"] = check(summary, [(c, k, "F4e", "wrong_runs", "non nul") for c in ("B0", "B1") for k in KEYS]
                                    + [("S", k, "F4e", "wrong_runs", "nul") for k in KEYS])
    h3 = check(summary, [(c, k, "F6", "false_success_runs", "non nul") for c in ("B0", "B1") for k in KEYS]
               + [("S", k, "F6", "false_success_runs", "nul") for k in KEYS])
    partial = [r for r in records if r["condition"] == "S" and r["fault"] == "F6" and r["status"] == "OK"
               and r["metrics"]["payments"] > 0]
    if partial:
        h3 = ("infirmée", h3[1] + [f"S/F6 : {len(partial)} exécution(s) avec un règlement partiel"])
    else:
        h3 = (h3[0], h3[1] + ["S/F6 : aucun paiement dans les exécutions valides"])
    out["H3"] = h3
    out["H4"] = check(summary, [("S", "none", "F1", "double_runs", "non nul"),
                                ("S", "none", "F1", "false_success_runs", "nul")])
    details, verdict = [], "confirmée"
    for key in KEYS:
        s, b = cell(summary, "S", key, "none"), cell(summary, "B0", key, "none")
        if not s or not b or s["median_duration_s"] is None or b["median_duration_s"] is None:
            details.append(f"{key} : durées indisponibles")
            verdict = "non concluante"
            continue
        ratio = s["median_duration_s"] / b["median_duration_s"]
        details.append(f"{key} : S {s['median_duration_s']:.1f} s, B0 {b['median_duration_s']:.1f} s, rapport {ratio:.1f}")
        if s["median_duration_s"] <= b["median_duration_s"]:
            verdict = "infirmée"
    out["H5"] = (verdict, details)
    out["H6"] = ("à établir", ["étiquetage des blocages, section 7"])
    return out


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("jsonl")
    a = p.parse_args(argv)
    records, _ = tables.load(a.jsonl)
    for name, (verdict, details) in verdicts(records).items():
        print(f"{name} : {verdict}")
        for d in details:
            print(f"  - {d}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
