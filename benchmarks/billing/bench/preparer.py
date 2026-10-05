#!/usr/bin/env python3
"""Préparateur scripté (agent factice) : propose un lot, ne paie jamais.

Codes : 0 lot écrit, 1 échec simulé après écriture (F8), 5 plantage.
"""
import argparse
import hashlib
import json
import os
import sys
import traceback
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, markers

MODES = ("nominal", "wrong-amount", "partial")


def build(api, mode):
    if mode not in MODES:
        raise ValueError(mode)
    lines = [{k: inv[k] for k in config.FIELDS} for inv in client.get(api, "/due")]
    if mode == "wrong-amount" and lines:
        lines[0]["amount_cents"] += config.TAMPER_DELTA_CENTS
    elif mode == "partial":
        lines = lines[: len(lines) // 2]
    return {"schema_version": 1, "lines": lines}


def write_lot(path, lot):
    """Écriture atomique ; retourne l'empreinte SHA-256 du fichier écrit."""
    data = json.dumps(lot, indent=2, sort_keys=True).encode()
    path = Path(path)
    tmp = path.with_suffix(".tmp")
    tmp.write_bytes(data)
    os.replace(tmp, path)
    return hashlib.sha256(data).hexdigest()


def _main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--mode", choices=MODES, default="nominal")
    p.add_argument("--hang-after-write", action="store_true", help="F6 : boucle sans fin après un lot partiel")
    p.add_argument("--fail-after-write", action="store_true", help="F8 : tentative qui échoue après écriture")
    p.add_argument("--marker")
    a = p.parse_args(argv)
    print(write_lot(a.out, build(a.api, a.mode)))
    if a.hang_after_write:
        markers.mark(a.marker, fault="budget-loop")
        while True:
            time.sleep(1)
    return 1 if a.fail_after_write else 0


def main(argv=None):
    """Une exception inattendue écrit sa trace sur stderr et sort en EXIT_INTERNAL, distinct des codes métier."""
    try:
        return _main(argv)
    except Exception:   # SystemExit (argparse) et KeyboardInterrupt ne sont pas des Exception : inchangés
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
