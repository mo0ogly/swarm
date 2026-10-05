#!/usr/bin/env python3
"""Contrôle du règlement (S) et revue finale (B0/B1), après l'effet.

Codes : 0 chaque facture payée une seule fois au montant exact, 1 sinon,
3 environnement indisponible, 5 plantage.
"""
import argparse
import json
import sys
import traceback
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, metrics


def _main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    a = p.parse_args(argv)
    try:
        snap = client.snapshot(a.api)
    except client.Unavailable as e:
        print(f"environnement : {e}", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    m = metrics.measure_rows(snap["invoices"], snap["payments"])
    print(json.dumps(m))
    return 0 if m["doubles"] == m["wrong"] == m["unpaid"] == 0 else config.EXIT_NONCONFORME


def main(argv=None):
    """Une exception inattendue écrit sa trace sur stderr et sort en EXIT_INTERNAL, distinct des codes métier."""
    try:
        return _main(argv)
    except Exception:   # SystemExit (argparse) et KeyboardInterrupt ne sont pas des Exception : inchangés
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
