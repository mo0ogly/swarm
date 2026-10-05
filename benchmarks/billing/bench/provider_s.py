#!/usr/bin/env python3
"""Fournisseur scripté de la condition S hiérarchique : préparation ou règlement, jamais les deux.

Arguments : racine Swarm, dossier des marqueurs de fautes, binaire swarm, URL de l'API, mode de
clé, faute, puis le fichier de jeton pour le seul fournisseur `banc-settle`. E1 par construction :
le fournisseur `banc-prepare` est déclaré sans ce dernier argument, et le jeton vit dans le dossier
de la banque, hors de la racine Swarm qui sert d'espace de travail aux agents.

Le prompt (stdin) d'une mission hiérarchique désigne la tâche par « Tâche <id> : » et
demande d'écrire `docs/<id>.md` ; il ne donne ni le travail ni la tentative. Le
fournisseur les relit avec la CLI : l'unique travail de la racine, et la dernière
tentative, encore ouverte, de sa tâche (docs/benchmarks/billing/observations.md, § 2).

Le livrable est écrit dans `docs/<id>.md` du répertoire courant, l'espace de la
tentative. Le banc lance les deux tâches dans la racine (espace partagé) : le moteur
remet alors `docs/<id>.md`, chemin relatif à la racine, et les contrôles le lisent au
même endroit (observations.md, § 1 et § 4).

- `prepare` : construit le lot depuis /due et l'écrit en JSON dans `docs/prepare.md`.
- `settle` : retrouve la remise de `prepare` produite par sa tentative acceptée
  (`automatic_validation.attempt_id`), vérifie que l'empreinte remise est celle que les
  contrôles ont validée (`automatic_validation.artifacts[path]`), relit le fichier remis,
  exige cette empreinte (E3), paie avec le jeton du fichier passé en argument (E1, seule cette tâche le lit),
  puis écrit le bilan JSON dans `docs/settle.md`. Un refus ou un arrêt ne produit pas
  de livrable et sort en code non nul : le moteur ne remet alors rien.

Sortie standard au format du harnais de référence (`item.completed` / `agent_message`).
Crochets de fautes conservés, non exercés au cas nominal : F3 (plantage après un
paiement), F4 (attente de la modification du lot par run_s), F6 (lot partiel puis
boucle d'appels d'outils), F8 (tentative 1 de `prepare` terminée normalement avec un
lot faux).

Codes : 0 livrable produit ; 2 règlement refusé ou arrêté ; 3 API injoignable ; 4 lot
modifié depuis la remise ; 5 plantage ou contrat du moteur différent de celui observé
(tentative en cours ou remise acceptée introuvable, remise différente du livrable validé) : run_s en fait une ERREUR du banc ;
137 plantage injecté (F3).
"""
import json
import re
import subprocess
import sys
import time
import traceback
import urllib.error
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, markers, preparer, settle

TASK = re.compile(r"Tâche ([A-Za-z0-9_-]+) :")


class ValidationMismatch(Exception):
    """L'empreinte de la remise diffère de celle que la validation automatique a acceptée (E3)."""


class Unexpected(Exception):
    """État du moteur contraire au contrat observé (tentative, remise) : défaut du banc, jamais une mesure."""


def emit(event):
    print(json.dumps(event, ensure_ascii=False), flush=True)


def message(text):
    emit({"type": "item.completed", "item": {"type": "agent_message", "text": text}})


def tool_call(i, command, output="", code=0):
    """Appel d'outil au format reconnu par Swarm : il compte dans le budget d'appels (F6)."""
    emit({"type": "item.started", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                           "status": "in_progress"}})
    emit({"type": "item.completed", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                             "aggregated_output": output, "exit_code": code,
                                             "status": "completed" if code == 0 else "failed"}})


class Swarm:
    def __init__(self, root, binary):
        self.root, self.binary = root, binary

    def cli(self, args):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args], text=True,
                           capture_output=True, timeout=config.S_CLI_TIMEOUT_S)
        if p.returncode:
            raise RuntimeError(f"swarm {' '.join(args[:2])} : code {p.returncode} : {(p.stdout + p.stderr).strip()}")
        return json.loads(p.stdout) if p.stdout.strip() else None

    def work(self):
        works = self.cli(["work", "list"])
        if len(works) != 1:
            raise Unexpected(f"{len(works)} travaux dans la racine, un seul attendu")
        return self.cli(["work", "show", works[0]["id"]])["work"]


def task_of(work, tid):
    return next(t for t in work["tasks"] if t["id"] == tid)


def current_attempt(work, tid):
    """Tentative en cours : la dernière, encore sans `ended`, tâche `running`. Renvoie (identifiant, rang).

    Une tentative naît au statut `recorded` et ne reçoit `ended` qu'à sa fin (store.go : task.update).
    """
    task = task_of(work, tid)
    attempts = task.get("attempts") or []
    if task["status"] != "running" or not attempts or attempts[-1].get("ended"):
        raise Unexpected(f"aucune tentative en cours pour {tid} : statut {task['status']}, {attempts}")
    return attempts[-1]["id"], len(attempts)


def accepted_handoff(work, tid):
    """Remise produite par la tentative acceptée de `tid` : (tentative, artefact unique)."""
    task = task_of(work, tid)
    validation = task.get("automatic_validation") or {}
    if task["status"] != "accepted" or validation.get("state") != "accepted" or not validation.get("attempt_id"):
        raise Unexpected(f"{tid} non accepté : statut {task['status']}, validation {validation.get('state')}")
    attempt = validation["attempt_id"]
    found = [e for e in work["planning"]["inbox"]
             if e.get("kind") == "handoff" and e.get("task") == tid and e.get("attempt") == attempt]
    if len(found) != 1 or len(found[0].get("artifacts") or []) != 1:
        raise Unexpected(f"remise de {tid} pour la tentative acceptée {attempt} introuvable ou ambiguë : {found}")
    artifact = found[0]["artifacts"][0]
    validated = (validation.get("artifacts") or {}).get(artifact["path"])
    if validated != artifact["sha256"]:   # automatic_validation.go : record.Artifacts[report] = hash du livrable
        raise ValidationMismatch(f"remise {artifact['sha256']} pour {artifact['path']}, validé : {validated}")
    return attempt, artifact


def run_prepare(sw, faults, api, fault, workspace):
    attempt, rank = current_attempt(sw.work(), "prepare")
    first = rank == 1
    mode = "partial" if fault == "F6" else ("wrong-amount" if fault == "F8" and first else "nominal")
    sha = preparer.write_lot(workspace / "docs" / "prepare.md", preparer.build(api, mode))
    if fault == "F6":
        markers.mark(faults / "budget-loop.json", attempt=attempt)
        for i in range(1, 60):
            tool_call(i, "recompter les factures")
            time.sleep(0.1)
    if fault == "F8" and first:
        markers.mark(faults / "stale-report.json", attempt=attempt, lot_sha256=sha)
    message(f"Lot proposé dans docs/prepare.md (sha256 {sha}).")
    return 0


def run_settle(sw, root, faults, api, key_mode, fault, token_file, workspace):
    if fault == "F4":
        deadline = time.monotonic() + config.S_GATE_TIMEOUT_S
        while not (faults / "tamper-done").exists() and time.monotonic() < deadline:
            time.sleep(0.1)
    work = sw.work()
    attempt, _ = current_attempt(work, "settle")
    try:
        source, artifact = accepted_handoff(work, "prepare")
    except ValidationMismatch as e:   # incohérence du moteur, pas une mesure : 4 reste réservé au lot modifié (F4)
        print(f"contrat du moteur : {e}", file=sys.stderr)
        return config.EXIT_INTERNAL
    token = Path(token_file).read_text().strip()
    try:
        outcome = settle.settle(api, root / artifact["path"], key_mode, attempt, root / "settle-progress.jsonl",
                                expected_sha256=artifact["sha256"], token=token,
                                crash_after_pay=1 if fault == "F3" else None,
                                crash_marker=faults / "crash-before-record.json")
    except settle.DigestMismatch as e:
        print(str(e), file=sys.stderr)
        return config.EXIT_DIGEST
    summary = {"prepare_attempt": source, "lot_path": artifact["path"], "lot_sha256": artifact["sha256"],
               "settle_attempt": attempt, **outcome}
    if outcome["refused"] or outcome["stopped"]:   # aucun livrable : rien à remettre
        (root / f"settle-{attempt}-outcome.json").write_text(json.dumps(summary, ensure_ascii=False))
        print(f"règlement refusé ou arrêté : {json.dumps(outcome)}", file=sys.stderr)
        return config.EXIT_REFUSED
    (workspace / "docs" / "settle.md").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n")
    message(f"Règlement exécuté : {outcome['paid']} payée(s), {outcome['replayed']} rejouée(s).")
    return 0


def _main(argv):
    if len(argv) not in (6, 7):
        raise SystemExit("usage : provider_s.py RACINE FAUTES SWARM API MODE_CLE FAUTE [JETON]")
    root, faults, binary, api, key_mode, fault = Path(argv[0]), Path(argv[1]), *argv[2:6]
    token_file = argv[6] if len(argv) == 7 else None
    match = TASK.search(sys.stdin.read())
    if match is None:
        print("prompt sans « Tâche <id> : »", file=sys.stderr)
        return config.EXIT_INTERNAL
    task = match.group(1)
    sw, workspace = Swarm(root, binary), Path.cwd()
    (workspace / "docs").mkdir(exist_ok=True)
    if (task == "settle") != (token_file is not None):   # E1 : seul le règlement reçoit le jeton
        print(f"contrat du banc : jeton {'absent' if token_file is None else 'fourni'} pour {task}", file=sys.stderr)
        return config.EXIT_INTERNAL
    try:
        if task == "prepare":
            return run_prepare(sw, faults, api, fault, workspace)
        if task == "settle":
            return run_settle(sw, root, faults, api, key_mode, fault, token_file, workspace)
    except Unexpected as e:
        print(f"contrat du moteur : {e}", file=sys.stderr)
        return config.EXIT_INTERNAL
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
