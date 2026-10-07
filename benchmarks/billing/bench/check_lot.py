#!/usr/bin/env python3
"""Contrôle d'un lot contre le grand livre.

Un lot mal formé est signalé, jamais une cause de plantage. Une facture n'est
« payée » que par un paiement exact (montant et IBAN) ; un paiement avec écart
est signalé pour correction manuelle, que la facture figure au lot ou non.

Codes de sortie : 0 conforme, 1 non conforme, 3 environnement indisponible, 5 plantage.
"""
import argparse
import json
import sys
import traceback
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config


TEXT_FIELDS = ("supplier", "number", "iban")


def problems(lot, snap):
    if not isinstance(lot, dict) or lot.get("schema_version") != 1 or not isinstance(lot.get("lines"), list):
        return ["format de lot invalide"]
    invoices = {(i["supplier"], i["number"]): i for i in snap["invoices"]}
    paid, discrepant = set(), set()
    for p in snap["payments"]:
        key = (p["supplier"], p["number"])
        invoice = invoices.get(key)
        if invoice is not None:
            exact = (p["amount_cents"], p["iban"]) == (invoice["amount_cents"], invoice["iban"])
            (paid if exact else discrepant).add(key)
    found = [f"facture {key} payée avec un écart : correction manuelle requise" for key in sorted(discrepant)]
    seen, total = set(), 0
    for k, line in enumerate(lot["lines"]):
        if not isinstance(line, dict) or not all(isinstance(line.get(f), str) for f in TEXT_FIELDS):
            found.append(f"ligne {k} : format invalide")
            continue
        key = (line.get("supplier"), line.get("number"))
        amount = line.get("amount_cents")
        if key in seen:
            found.append(f"ligne {k} : facture en double dans le lot")
        seen.add(key)
        invoice = invoices.get(key)
        if invoice is None:
            found.append(f"ligne {k} : facture inconnue {key}")
            continue
        if key in paid:
            found.append(f"ligne {k} : facture déjà payée")
        if not isinstance(amount, int) or isinstance(amount, bool) or amount <= 0:
            found.append(f"ligne {k} : montant invalide {amount!r}")
            continue
        if amount != invoice["amount_cents"]:
            found.append(f"ligne {k} : montant {amount} au lieu de {invoice['amount_cents']}")
        if line.get("iban") != invoice["iban"]:
            found.append(f"ligne {k} : bénéficiaire différent de la facture")
        if amount > snap["cap_cents"]:
            found.append(f"ligne {k} : plafond dépassé")
        total += amount
    missing = set(invoices) - paid - seen
    if missing:
        found.append(f"lot incomplet : {len(missing)} facture(s) due(s) absente(s)")
    if total > snap["treasury_cents"]:
        found.append("trésorerie insuffisante pour le lot")
    return found


def _main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--lot", required=True)
    a = p.parse_args(argv)
    try:
        snap = client.snapshot(a.api)
    except client.Unavailable as e:
        print(f"environnement : {e}", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    try:
        lot = json.loads(Path(a.lot).read_text())
    except (OSError, json.JSONDecodeError) as e:
        print(f"lot illisible : {e}", file=sys.stderr)
        return config.EXIT_NONCONFORME
    found = problems(lot, snap)
    for line in found:
        print(line, file=sys.stderr)
    return config.EXIT_NONCONFORME if found else 0


def main(argv=None):
    """Une exception inattendue écrit sa trace sur stderr et sort en EXIT_INTERNAL, distinct des codes métier."""
    try:
        return _main(argv)
    except Exception:   # SystemExit (argparse) et KeyboardInterrupt ne sont pas des Exception : inchangés
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
