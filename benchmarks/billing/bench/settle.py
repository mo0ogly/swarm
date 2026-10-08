#!/usr/bin/env python3
"""Règlement déterministe d'un lot : une requête de paiement par ligne.

Clé d'idempotence selon le mode : aucune, par tentative, ou métier
(fournisseur, facture). La progression est enregistrée après chaque réponse,
avec l'empreinte du lot : une relance ne saute que les lignes enregistrées pour
le même lot. Un crash entre la réponse et l'enregistrement est injectable (F3) ;
il ne compte que les réponses payées ou rejouées.

Seuls les refus définitifs (400, 403, 409) sont consignés. Un 401 ou un 5xx
arrête le règlement sans consigner la ligne, retentée à la relance.

Codes : 0 tout réglé, 2 refus ou arrêt, 3 service injoignable après réessais,
4 lot modifié depuis la remise, 5 plantage, 137 crash injecté.
"""
import argparse
import hashlib
import json
import os
import sys
import traceback
import time
import urllib.error
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, markers

KEY_MODES = ("none", "attempt", "business")
DEFINITIVE_REFUSALS = (400, 403, 409)


class DigestMismatch(Exception):
    """Le lot sur disque n'est pas celui qui a été remis : règlement refusé."""


def key_for(mode, attempt, line):
    if mode == "none":
        return None
    if mode == "attempt":
        return f"{attempt}:{line['supplier']}:{line['number']}"
    if mode == "business":
        return f"{line['supplier']}:{line['number']}"
    raise ValueError(mode)


def error_text(raw):
    """Message d'un corps d'erreur : champ `error` du JSON, sinon texte brut ; toujours tronqué."""
    try:
        text = str(json.loads(raw).get("error", ""))
    except (ValueError, AttributeError):   # corps non JSON, ou JSON qui n'est pas un objet
        text = raw.decode("utf-8", "replace")
    return text[:config.ERROR_TEXT_MAX]


def pay_line(api, body, token):
    """Réessaie après une coupure ; un refus HTTP est définitif et n'est pas réessayé."""
    for retry in range(config.SETTLE_RETRIES + 1):
        try:
            return client.post(api, "/pay", body, token)
        except urllib.error.HTTPError as e:   # avant URLError : HTTPError en hérite
            return {"refused": e.code, "error": error_text(e.read())}
        except (ConnectionError, TimeoutError, urllib.error.URLError):
            if retry == config.SETTLE_RETRIES:
                raise
            time.sleep(config.SETTLE_RETRY_DELAY_S)


def settle(api, lot_path, key_mode, attempt, progress_path, *, expected_sha256=None, token=None,
           crash_after_pay=None, crash_marker=None):
    raw = Path(lot_path).read_bytes()
    sha = hashlib.sha256(raw).hexdigest()
    if expected_sha256 is not None and sha != expected_sha256:
        raise DigestMismatch(f"lot modifié depuis la remise : {sha} au lieu de {expected_sha256}")
    lines = json.loads(raw)["lines"]
    progress = Path(progress_path)
    rows = [json.loads(row) for row in progress.read_text().splitlines()] if progress.exists() else []
    done = {row["line"] for row in rows if row.get("lot_sha256") == sha}
    outcome = {"paid": 0, "replayed": 0, "refused": [], "stopped": None, "skipped": len(done)}
    effected = 0
    for index, line in enumerate(lines):
        if index in done:
            continue
        response = pay_line(api, {**line, "idem_key": key_for(key_mode, attempt, line), "attempt": attempt,
                                  "lot_sha256": sha}, token)
        if "refused" in response:
            if response["refused"] not in DEFINITIVE_REFUSALS:
                outcome["stopped"] = {"line": index, **response}
                break
            outcome["refused"].append({"line": index, **response})
        else:
            effected += 1
            outcome["replayed" if response["replay"] else "paid"] += 1
            if crash_after_pay == effected and not Path(crash_marker).exists():
                markers.mark(crash_marker, fault="crash-before-record", line=index)
                os._exit(config.EXIT_CRASH)
        with progress.open("a") as f:
            f.write(json.dumps({"line": index, "lot_sha256": sha, **response}) + "\n")
    return outcome


def _main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--lot", required=True)
    p.add_argument("--key-mode", choices=KEY_MODES, required=True)
    p.add_argument("--attempt", required=True)
    p.add_argument("--progress", required=True)
    p.add_argument("--expected-sha256")
    p.add_argument("--token-file")
    p.add_argument("--crash-after-pay", type=int)
    p.add_argument("--crash-marker")
    a = p.parse_args(argv)
    token = Path(a.token_file).read_text().strip() if a.token_file else None
    try:
        outcome = settle(a.api, a.lot, a.key_mode, a.attempt, a.progress, expected_sha256=a.expected_sha256,
                         token=token, crash_after_pay=a.crash_after_pay, crash_marker=a.crash_marker)
    except DigestMismatch as e:
        print(str(e), file=sys.stderr)
        return config.EXIT_DIGEST
    except (ConnectionError, TimeoutError, urllib.error.URLError) as e:   # réessais épuisés
        print(f"environnement : service de paiement injoignable ({e})", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    print(json.dumps(outcome))
    return config.EXIT_REFUSED if outcome["refused"] or outcome["stopped"] else 0


def main(argv=None):
    """Une exception inattendue écrit sa trace sur stderr et sort en EXIT_INTERNAL, distinct des codes métier."""
    try:
        return _main(argv)
    except Exception:   # SystemExit (argparse) et KeyboardInterrupt ne sont pas des Exception : inchangés
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
