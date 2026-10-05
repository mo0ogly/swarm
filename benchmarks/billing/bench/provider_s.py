#!/usr/bin/env python3
"""Fournisseur scripté pour la condition S : préparation ou règlement, jamais les deux.

Arguments : racine, binaire swarm, URL de l'API, mode de clé, faute, fichier de jeton ou '-'.
Le prompt Swarm (stdin) donne mission, tâche, agent et tentative
(agents_store.go : ligne « Coordination structurée »).

Seule la tâche `settle` lit le jeton de règlement : le préparateur n'a aucune capacité de paiement.

Codes : 0 livrable produit ; 1 échec simulé (F8) ou remise introuvable ; 2 règlement refusé ou arrêté ;
3 API injoignable ; 4 lot modifié depuis la remise ; 5 plantage.
"""
import json
import re
import subprocess
import sys
import time
import traceback
import urllib.error
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, markers, preparer, settle

PROMPT = re.compile(r"Coordination structurée : mission ([^,]+), tâche ([^,]+), agent ([^,]+), tentative ([^.]+)[.]")


def emit(event):
    print(json.dumps(event), flush=True)


def tool_call(i, command, output="", code=0):
    """Événement d'outil au format attendu par Swarm : il compte dans le budget d'appels."""
    emit({"type": "item.started", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                           "status": "in_progress"}})
    emit({"type": "item.completed", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                             "aggregated_output": output, "exit_code": code,
                                             "status": "completed" if code == 0 else "failed"}})


class Swarm:
    def __init__(self, root, binary):
        self.root, self.binary = root, binary

    def cli(self, args, data=None):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args]
                           + (["--input", "-"] if data is not None else []),
                           input=json.dumps(data) if data is not None else None, text=True, capture_output=True,
                           timeout=config.S_CLI_TIMEOUT_S)
        if p.returncode:
            raise RuntimeError(p.stdout + p.stderr)
        return json.loads(p.stdout) if p.stdout.strip() else None


def first_attempt(sw, work, task, attempt):
    tasks = sw.cli(["work", "show", work])["work"]["tasks"]
    ids = [a["id"] for a in next(t for t in tasks if t["id"] == task)["attempts"]]
    return ids.index(attempt) == 0


def send_handoff(sw, work, agent, attempt, sha):
    return sw.cli(["exchange", "send", work], dict(
        schema_version=1, event_id=uuid.uuid4().hex, kind="handoff", agent_id=agent, task_id="prepare",
        attempt_id=attempt, recipient_task_id="settle", recipient_role="worker", result_state="completed",
        artifacts=[dict(path="lot.json", sha256=sha)]))


def run_prepare(sw, root, api, fault, work, agent, attempt):
    faults = root / "faults"
    first = first_attempt(sw, work, "prepare", attempt)
    mode = "partial" if fault == "F6" else ("wrong-amount" if fault == "F8" and first else "nominal")
    tool_call(1, "préparer le lot depuis /due")
    sha = preparer.write_lot(root / "lot.json", preparer.build(api, mode))
    if fault == "F6":
        markers.mark(faults / "budget-loop.json", attempt=attempt)
        for i in range(2, 60):
            tool_call(i, "recompter les factures")
            time.sleep(0.1)
    send_handoff(sw, work, agent, attempt, sha)
    if fault == "F8" and first:
        markers.mark(faults / "stale-report.json", attempt=attempt)
        tool_call(99, "publier", "temporary failure: connection reset", 1)
        return config.EXIT_NONCONFORME
    emit({"type": "item.completed", "item": {"type": "agent_message", "text": "Lot proposé et remis."}})
    return 0


def run_settle(sw, root, api, key_mode, fault, token_file, work, agent, attempt):
    faults = root / "faults"
    if fault == "F4":
        deadline = time.monotonic() + config.S_GATE_TIMEOUT_S
        while not (faults / "tamper-done").exists() and time.monotonic() < deadline:
            time.sleep(0.1)
    consumed, refusals = None, []
    for x in sw.cli(["exchange", "list", work]):   # consommateur naïf : la plus ancienne d'abord
        if x["source_task_id"] != "prepare" or x["state"] == "consumed":
            continue
        try:
            sw.cli(["exchange", "consume", work], dict(
                schema_version=1, event_id=uuid.uuid4().hex, exchange_id=x["id"], agent_id=agent,
                task_id="settle", attempt_id=attempt, acknowledgement="Empreinte contrôlée par le moteur"))
        except RuntimeError as e:
            refusals.append(str(e)[:300])
            continue
        consumed = x
        break
    (root / f"settle-{attempt}-refusals.json").write_text(json.dumps(refusals, ensure_ascii=False))
    if consumed is None:
        tool_call(1, "consommer la remise", "aucune remise valide", 1)
        print("aucune remise consommable", file=sys.stderr)
        return config.EXIT_NONCONFORME
    token = Path(token_file).read_text().strip() if token_file != "-" else None
    tool_call(2, "régler le lot")
    try:
        outcome = settle.settle(api, root / "lot.json", key_mode, attempt, root / "settle-progress.jsonl",
                                expected_sha256=consumed["artifacts"][0]["sha256"], token=token,
                                crash_after_pay=1 if fault == "F3" else None,
                                crash_marker=faults / "crash-before-record.json")
    except settle.DigestMismatch as e:
        tool_call(3, "contrôler l'empreinte du lot", str(e), config.EXIT_DIGEST)
        print(str(e), file=sys.stderr)
        return config.EXIT_DIGEST
    # Le livrable n'est écrit que pour un règlement complet : un refus ou un arrêt n'en produit pas.
    if outcome["refused"] or outcome["stopped"]:
        (root / f"settle-{attempt}-outcome.json").write_text(json.dumps(outcome))
        tool_call(3, "bilan du règlement", json.dumps(outcome)[:config.ERROR_TEXT_MAX], config.EXIT_REFUSED)
        print(f"règlement refusé ou arrêté : {json.dumps(outcome)}", file=sys.stderr)
        return config.EXIT_REFUSED
    (root / "settlement.json").write_text(json.dumps(outcome))
    emit({"type": "item.completed", "item": {"type": "agent_message", "text": "Règlement exécuté."}})
    return 0


def _main(argv):
    if len(argv) != 6:
        raise SystemExit("usage : provider_s.py RACINE SWARM API MODE_CLE FAUTE JETON|-")
    root, binary, api, key_mode, fault, token_file = Path(argv[0]), *argv[1:6]
    match = PROMPT.search(sys.stdin.read())
    if match is None:
        print("prompt sans ligne de coordination structurée", file=sys.stderr)
        return config.EXIT_INTERNAL
    work, task, agent, attempt = match.groups()
    sw = Swarm(root, binary)
    if task == "prepare":
        return run_prepare(sw, root, api, fault, work, agent, attempt)
    if task == "settle":
        return run_settle(sw, root, api, key_mode, fault, token_file, work, agent, attempt)
    print(f"tâche inattendue : {task}", file=sys.stderr)
    return config.EXIT_INTERNAL


def main(argv=None):
    """Exception inattendue : trace sur stderr et EXIT_INTERNAL ; API injoignable : EXIT_ENVIRONMENT."""
    try:
        return _main(sys.argv[1:] if argv is None else argv)
    except urllib.error.HTTPError as e:   # avant URLError : HTTPError en hérite (lecture de /due)
        print(f"API : HTTP {e.code} ({settle.error_text(e.read())})", file=sys.stderr)
        return config.EXIT_ENVIRONMENT if e.code >= 500 else config.EXIT_INTERNAL
    except (ConnectionError, TimeoutError, urllib.error.URLError) as e:
        print(f"environnement : service de paiement injoignable ({e})", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    except Exception:
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
