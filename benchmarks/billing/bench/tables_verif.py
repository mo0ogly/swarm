#!/usr/bin/env python3
"""Verdicts de la campagne de vérification des correctifs, figés avant les données.

Usage : tables_verif.py VERIF.jsonl HISTORIQUE.jsonl

Critères (docs/benchmarks/billing/verification-correctifs.md), sur les exécutions OK sauf mention :
- D5 : aucune exclusion due à SQLITE_BUSY sous F4e (historique : 31 sur 300) ;
- D3 : sous F5, `prepare` lancée une fois, reçu d'échec d'environnement, aucun faux succès ni inexact ;
- D2 : sous F4e, au moins un événement `dependency_stale` et arrêt attribué au moteur ;
- D6 : variante nodeps, `settle` jamais lancée avant l'acceptation de `prepare` et règlement exact ;
  contrôle positif noguard : au moins une exécution où `settle` part trop tôt (ou ERREUR code 5) ;
- D1 : variante ownws, règlement exact ;
- non-régression : clé métier, mêmes nombres d'exécutions présentant chaque défaut que la campagne
  historique (S), case par case.
"""
import json
import sys
from collections import defaultdict
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import tables

BUSY = ("SQLITE_BUSY", "database is locked")
MEASURES = ("double_runs", "wrong_runs", "unpaid_runs", "false_success_runs")


def load(path):
    rows = [json.loads(l) for l in Path(path).read_text().splitlines() if l.strip()]
    return [r for r in rows if r.get("kind") != "campaign"]


def group(runs):
    g = defaultdict(list)
    for r in runs:
        g[(r["key_mode"], r["fault"], r.get("variant"))].append(r)
    return g


def ok(runs):
    return [r for r in runs if r["status"] == "OK"]


def verdict(passed, detail):
    return ("critère tenu" if passed else "CRITÈRE NON TENU"), detail


def verdicts(verif, historical):
    g = group(verif)
    out = {}
    f4e = [r for k in ("none", "attempt", "business") for r in g[(k, "F4e", None)]]
    busy = [r for r in f4e if r["status"] != "OK" and any(b in (r.get("error") or "") for b in BUSY)]
    out["D5"] = verdict(bool(f4e) and not busy, f"{len(busy)} exclusion(s) SQLITE_BUSY sur {len(f4e)} (historique : 31 sur 300) ; "
                        f"autres exclusions : {sum(r['status'] != 'OK' for r in f4e) - len(busy)}")
    f5 = ok([r for k in ("none", "attempt", "business") for r in g[(k, "F5", None)]])
    v = [r.get("verification") or {} for r in f5]
    good = [x for x, r in zip(v, f5) if x.get("prepare_attempts") == 1 and x.get("environment_failure")
            and not r["false_success"] and r["metrics"]["wrong"] == 0]
    out["D3"] = verdict(bool(f5) and len(good) == len(f5), f"{len(good)} sur {len(f5)} valides : une seule tentative de prepare, "
                        "reçu d'échec d'environnement, ni faux succès ni inexact")
    f4e_ok = ok(f4e)
    stale = [r for r in f4e_ok if (r.get("verification") or {}).get("dependency_stale_events", 0) >= 1
             and r.get("stopped_by") == "engine"]
    out["D2"] = verdict(bool(f4e_ok) and len(stale) == len(f4e_ok),
                        f"{len(stale)} sur {len(f4e_ok)} valides : événement dependency_stale et arrêt par le moteur")
    nodeps = ok(g[("business", "none", "nodeps")])
    early = [r for r in nodeps if (r.get("verification") or {}).get("settle_started_before_prepare_accepted")]
    exact = [r for r in nodeps if r["correct"]]
    noguard = g[("business", "none", "noguard")]
    positive = [r for r in noguard if (r.get("verification") or {}).get("settle_started_before_prepare_accepted")
                or (r["status"] == "ERREUR" and "code 5" in (r.get("error") or ""))]
    out["D6"] = verdict(bool(nodeps) and not early and len(exact) == len(nodeps) and bool(positive),
                        f"nodeps : {len(early)} départ(s) trop tôt, {len(exact)} exactes sur {len(nodeps)} valides ; "
                        f"contrôle positif noguard : {len(positive)} sur {len(noguard)}")
    own = g[("business", "none", "ownws")]
    out["D1"] = verdict(bool(own) and all(r["status"] == "OK" and r["correct"] for r in own),
                        f"{sum(r['status'] == 'OK' and r['correct'] for r in own)} exactes sur {len(own)}")
    now = tables.summarize([r for r in verif if r.get("variant") is None and r["key_mode"] == "business"])
    then = tables.summarize([r for r in historical if r["condition"] == "S" and r["key_mode"] == "business"])
    gaps = []
    for cell, s in sorted(now.items()):
        h = then.get(cell)
        if h is None:
            continue
        for m in MEASURES:
            if s["ok"] and h["ok"] and (s[m] > 0) != (h[m] > 0):
                gaps.append(f"{cell[2]} {m} : {s[m]}/{s['ok']} contre {h[m]}/{h['ok']}")
    out["Non-régression"] = (("conforme" if not gaps else "ÉCART"), "; ".join(gaps) or "mêmes défauts présents ou absents, case par case")
    return out


def main(argv=None):
    argv = sys.argv[1:] if argv is None else argv
    for name, (v, detail) in verdicts(load(argv[0]), load(argv[1])).items():
        print(f"{name} : {v} — {detail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
