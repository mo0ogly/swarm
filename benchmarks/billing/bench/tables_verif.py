#!/usr/bin/env python3
"""Verdicts de la campagne de vérification des correctifs, figés avant les données.

Usage : tables_verif.py VERIF.jsonl HISTORIQUE.jsonl

Critères (docs/benchmarks/billing/verification-correctifs.md), sur les exécutions OK sauf mention :
- D5 : aucune exclusion due à SQLITE_BUSY sous F4e (historique F4e : 26 ERREUR SQLite et 5 DÉLAI non attribués sur 300) ;
- D3 : sous F5, `prepare` lancée une fois, reçu d'échec d'environnement, aucun faux succès ni inexact ;
- D2 : sous F4e, au moins un événement `dependency_stale` et arrêt attribué au moteur ;
- D6 : variante nodeps, `settle` jamais lancée avant l'acceptation de `prepare` et règlement exact ;
  contrôle positif noguard : au moins une exécution où `settle` part trop tôt (ou ERREUR code 5) ;
- D1 : variante ownws, règlement exact ;
- non-régression : clé métier, mêmes nombres d'exécutions présentant chaque défaut que la campagne
  historique (S), case par case.
"""
import json
import argparse
import re
import math
import sys
from collections import defaultdict
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import tables

BUSY = ("SQLITE_BUSY", "database is locked")
MEASURES = ("double_runs", "wrong_runs", "unpaid_runs", "false_success_runs")
KEYS = ("none", "attempt", "business")
FAULTS = ("none", "F1", "F2", "F3", "F4", "F4e", "F5", "F6", "F7", "F8")
REGRESSION = ("none", "F1", "F2", "F3", "F4", "F6", "F7", "F8")


def checked_load(path, *, historical=False, reps=100, seed_base=1000):
    """Validate archived inputs, not current sources, before computing any verdict.

    The defaults describe the registered campaigns. Other grids require explicit
    CLI arguments; a shortened file cannot silently redefine the expected grid.
    """
    records = []
    for number, line in enumerate(Path(path).read_text().splitlines(), 1):
        if not line.strip():
            continue
        try:
            r = json.loads(line)
            if not isinstance(r, dict):
                raise ValueError("objet JSON attendu")
            records.append(r)
        except ValueError as e:
            raise ValueError(f"{path}:{number}: {e}") from e
    if not records or records[0].get("event") != "start" or records[-1].get("event") != "end":
        raise ValueError("campagne incomplète : début et fin requis")
    headers = [r for r in records if r.get("kind") == "campaign"]
    if [r.get("event") for r in headers] != ["start", "end"]:
        raise ValueError("une campagne terminée unique est requise ; segmenter les reprises explicitement")
    start, end = headers
    for field in ("bench_sha256", "swarm_sha256"):
        if not re.fullmatch(r"[0-9a-f]{64}", str(start.get(field, ""))) or end.get(field) != start[field]:
            raise ValueError(f"empreinte absente ou incohérente : {field}")
    if start.get("git_dirty") is not False or not re.fullmatch(r"[0-9a-f]{40}", str(start.get("git_commit", ""))):
        raise ValueError("provenance Git propre et commit complet requis")
    if end.get("bench_changed") is not False or end.get("swarm_changed") is not False:
        raise ValueError("banc ou binaire modifié pendant la campagne")
    if historical:
        cells = [(c, k, f, None) for c in ("B0", "B1", "S") for k in KEYS for f in FAULTS]
    else:
        cells = [("S", k, f, None) for f in ("F4e", "F5") for k in KEYS]
        cells += [("S", "business", "none", v) for v in ("nodeps", "noguard", "ownws")]
        cells += [("S", "business", f, None) for f in REGRESSION]
    expected = {(*cell, seed) for cell in cells for seed in range(seed_base, seed_base + reps)}
    seen, runs = set(), []
    for r in records:
        if r.get("kind") == "campaign":
            continue
        case = (r.get("condition"), r.get("key_mode"), r.get("fault"), r.get("variant"), r.get("seed"))
        if type(r.get("seed")) is not int or case not in expected or case in seen:
            raise ValueError(f"case inattendue ou dupliquée : {case}")
        seen.add(case)
        provenance = r.get("provenance") or {}
        if not isinstance(provenance, dict):
            raise ValueError(f"provenance non structurée : {case}")
        if any(provenance.get(f) != start.get(f) for f in ("git_commit", "git_dirty", "bench_sha256", "swarm_sha256")):
            raise ValueError(f"provenance incohérente : {case}")
        if r.get("status") not in ("OK", "ERREUR", "DÉLAI", "INVALIDE"):
            raise ValueError(f"statut absent ou inconnu : {case}")
        if r["status"] == "OK":
            if type(r.get("correct")) is not bool or type(r.get("false_success")) is not bool:
                raise ValueError(f"mesures booléennes manquantes : {case}")
            duration = r.get("duration_s")
            if type(duration) not in (int, float) or not math.isfinite(duration) or duration < 0:
                raise ValueError(f"durée absente ou invalide : {case}")
            metrics = r.get("metrics") or {}
            if not isinstance(metrics, dict):
                raise ValueError(f"mesures non structurées : {case}")
            if any(type(metrics.get(f)) is not int or metrics[f] < 0 for f in ("doubles", "wrong", "unpaid")):
                raise ValueError(f"mesures de paiement manquantes : {case}")
            if not historical and (r.get("fault") in ("F4e", "F5") or r.get("variant") == "nodeps"):
                v = r.get("verification") or {}
                if not isinstance(v, dict):
                    raise ValueError(f"preuves non structurées : {case}")
                if (type(v.get("prepare_attempts")) is not int or type(v.get("dependency_stale_events")) is not int
                        or type(v.get("environment_failure")) is not bool
                        or type(v.get("settle_started_before_prepare_accepted")) is not bool):
                    raise ValueError(f"preuves de vérification manquantes : {case}")
        runs.append(r)
    if seen != expected:
        raise ValueError(f"grille incomplète : {len(expected - seen)} case(s) manquante(s)")
    return runs


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
    out["D5"] = verdict(bool(f4e) and not busy, f"{len(busy)} exclusion(s) SQLITE_BUSY sur {len(f4e)} (historique F4e : 26 ERREUR SQLite et 5 DÉLAI non attribués sur 300) ; "
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
    now = tables.summarize([r for r in verif if r.get("variant") is None and r["key_mode"] == "business" and r["fault"] in REGRESSION])
    then = tables.summarize([r for r in historical if r["condition"] == "S" and r["key_mode"] == "business"])
    gaps = []
    for fault in REGRESSION:
        cell = ("S", "business", fault)
        s = now.get(cell)
        h = then.get(cell)
        if s is None or h is None or not s["ok"] or not h["ok"]:
            gaps.append(f"{cell[2]} : comparaison absente ou aucune mesure valide")
            continue
        for m in MEASURES:
            if s["ok"] and h["ok"] and (s[m] > 0) != (h[m] > 0):
                gaps.append(f"{cell[2]} {m} : {s[m]}/{s['ok']} contre {h[m]}/{h['ok']}")
    out["Non-régression"] = (("conforme" if not gaps else "ÉCART"), "; ".join(gaps) or "mêmes défauts présents ou absents, case par case")
    return out


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("verification")
    parser.add_argument("historique")
    parser.add_argument("--reps", type=int, default=100)
    parser.add_argument("--seed-base", type=int, default=1000)
    args = parser.parse_args(argv)
    if args.reps < 1:
        parser.error("--reps doit être positif")
    try:
        verif = checked_load(args.verification, reps=args.reps, seed_base=args.seed_base)
        history = checked_load(args.historique, historical=True, reps=args.reps, seed_base=args.seed_base)
        results = verdicts(verif, history)
    except (ValueError, KeyError, TypeError, OSError) as e:
        print(f"DONNÉES NON VALIDÉES : {e}", file=sys.stderr)
        return 2
    for name, (v, detail) in results.items():
        print(f"{name} : {v} — {detail}")
    return int(any(v not in ("critère tenu", "conforme") for v, _ in results.values()))


if __name__ == "__main__":
    sys.exit(main())
