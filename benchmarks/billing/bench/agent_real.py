"""Appel d'un agent réel (Claude Code, mode non interactif) pour les conditions à agent réel.

L'agent est lancé en mode restreint : réglages utilisateur et projet ignorés (aucun crochet,
aucune configuration de l'opérateur), sans serveur MCP, sans persistance de session. Deux profils
d'outils :
- PREPARER : écriture de fichiers dans le répertoire courant, rien d'autre. Le préparateur ne
  reçoit ni Bash ni lecture : la sonde d'isolement (isolation_probe.py) a montré que `curl file://`
  lit hors du répertoire malgré `--restricted`, ce qui exposerait le jeton de règlement (E1) ;
- PAYER (B0-réel seulement) : `curl` par Bash et écriture. L'agent y détient déjà le jeton ; il
  peut aussi lire des fichiers locaux par `curl file://`, propriété documentée de cette condition. Le modèle
est figé et déclaré dans chaque ligne de consommation.

Chaque appel ajoute une ligne JSON au fichier de consommation : modèle, coût déclaré par le
client, jetons, nombre de tours, durée, erreur éventuelle. Ce fichier est hors de l'espace de
travail de l'agent.
"""
import json
import os
import signal
import subprocess
import time
from pathlib import Path

from bench import config

PREPARER = ("Write", ("Write",))
PAYER = ("Bash,Write", ("Bash(curl:*)", "Write"))


def command(model, profile=PREPARER):
    tools, allowed = profile
    return ["claude", "-p", "--restricted", "--model", model, "--tools", tools,
            "--allowedTools", *allowed, "--output-format", "json",
            "--no-session-persistence", "--strict-mcp-config"]


def run(prompt, cwd, model, usage_path, label, kill_when=None, profile=PREPARER):
    """Lance l'agent ; renvoie (code, texte final). Une sortie illisible vaut échec de l'agent.

    `kill_when` : condition vérifiée pendant l'exécution ; vraie, l'agent et ses enfants sont tués
    (SIGKILL du groupe de processus, code 137) : arrêt brutal injecté (F3).
    """
    started = time.monotonic()
    timed_out = killed = False
    out_path, err_path = Path(usage_path).with_name(f"{label}.out"), Path(usage_path).with_name(f"{label}.err")
    with open(out_path, "w") as out, open(err_path, "w") as err:
        proc = subprocess.Popen(command(model, profile), stdin=subprocess.PIPE, stdout=out, stderr=err, text=True, cwd=cwd,
                                start_new_session=True)
        proc.stdin.write(prompt)
        proc.stdin.close()
        deadline = started + config.REAL_AGENT_TIMEOUT_S
        while proc.poll() is None:
            if kill_when is not None and kill_when():
                killed = True
            elif time.monotonic() > deadline:
                timed_out = True
            if killed or timed_out:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait()
                break
            time.sleep(0.1)
    code = 137 if killed else (None if timed_out else proc.returncode)
    stdout, stderr = out_path.read_text(), err_path.read_text()
    try:
        data = json.loads(stdout) if stdout.strip() else {}
    except ValueError:
        data = {}
    usage = data.get("usage") or {}
    line = {"label": label, "model": model, "tools": profile[0], "exit_code": code, "timed_out": timed_out, "killed": killed,
            "is_error": data.get("is_error"), "num_turns": data.get("num_turns"),
            "cost_usd": data.get("total_cost_usd"),
            "input_tokens": usage.get("input_tokens"), "output_tokens": usage.get("output_tokens"),
            "cache_creation_input_tokens": usage.get("cache_creation_input_tokens"),
            "cache_read_input_tokens": usage.get("cache_read_input_tokens"),
            "models_used": sorted(data.get("modelUsage") or {}),
            "duration_s": round(time.monotonic() - started, 1),
            "stderr_tail": stderr.strip()[-config.ERROR_TEXT_MAX:]}
    with open(usage_path, "a") as f:
        f.write(json.dumps(line, ensure_ascii=False) + "\n")
    ok = not timed_out and not killed and code == 0 and not data.get("is_error")
    return (0 if ok else 1), str(data.get("result") or "")


def read_usage(path):
    path = Path(path)
    return [json.loads(l) for l in path.read_text().splitlines() if l.strip()] if path.exists() else []
