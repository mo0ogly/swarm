#!/usr/bin/env python3
"""Responsable de mission scripté de la condition S : aucun modèle appelé.

Adapté de `tests/organized_fixture.py` du dépôt swarm, réécrit ici pour que le banc ne
dépende pas des tests du moteur. Le moteur appelle le même exécutable pour deux rôles,
reconnus au contexte JSON contenu dans le prompt (stdin) :

- revue indépendante (contexte avec `report`) : verdict « pass » pour chaque critère si
  le rapport n'est pas vide, « unknown » sinon. Les contrôles automatiques restent seuls
  juges du fond (check_lot, verify_settlement).
- passe de planification (sinon) : prend acte de tous les événements reçus, et clôt le
  périmètre quand toutes les tâches sont acceptées. Jamais de reprise (`retry`) : une
  tentative en échec reste bloquée, ce que le banc mesure.

Nom imposé : le moteur n'accepte comme responsable ou vérificateur qu'un exécutable
désigné par un chemin absolu dont le nom de base est `claude`, `codex` ou
`skynet_harness` (assist_provider.go : assistantProvider, appelé par planning.go à
`planning enable`, par planning_runner.go à chaque passe et par independent_review.go).
Pour `claude`, il remplace les arguments par ceux de l'adaptateur sans outils
(`-p --output-format stream-json …`) et lit la réponse au format stream-json :
`{"type": "result", "result": "<JSON>"}`. `install` écrit donc dans la racine
d'exécution un lanceur nommé `claude` qui exécute `main` de ce module ; les arguments
reçus sont ignorés.
"""
import json
import sys
from pathlib import Path

PROVIDER = "fixture-planner"
BENCH_ROOT = Path(__file__).resolve().parents[1]   # benchmarks/billing : rend `bench` importable


def context_of(prompt):
    """Contexte JSON qui termine le prompt : la première accolade dont tout le reste se lit en JSON.

    Comme la référence, on exige que la suite entière soit du JSON : un exemple JSON cité plus tôt
    dans les consignes n'est pas pris pour le contexte.
    """
    for i, c in enumerate(prompt):
        if c == "{":
            try:
                value = json.loads(prompt[i:])
            except json.JSONDecodeError:
                continue
            if isinstance(value, dict):
                return value
    raise ValueError("prompt sans contexte JSON")


def respond(context):
    if "report" in context:
        report = str(context["report"]).strip()
        return {"reason": "Revue déterministe : rapport présent ; le fond relève des contrôles automatiques.",
                "criteria": [{"index": i + 1, "verdict": "pass" if report else "unknown",
                              "evidence": report[:500] if report else "Rapport vide"}
                             for i in range(len(context["criteria"]))]}
    tasks = context.get("tasks", [])
    response = {"input_events": [e["id"] for e in context["events"]],
                "reason": "Retours lus par le responsable scripté du banc.", "operations": []}
    if tasks and all(t["status"] == "accepted" for t in tasks):
        response["operations"] = [{"kind": "close"}]
    return response


def main():
    response = respond(context_of(sys.stdin.read()))
    print(json.dumps({"type": "result", "result": json.dumps(response, ensure_ascii=False)}), flush=True)
    return 0


def install(run_dir):
    """Écrit le lanceur `claude` dans la racine d'exécution ; renvoie la déclaration du fournisseur."""
    launcher = Path(run_dir) / "claude"
    launcher.write_text(f"#!{sys.executable}\n"
                        "import sys\n"
                        f"sys.path.insert(0, {str(BENCH_ROOT)!r})\n"
                        "from bench import planner_fixture\n"
                        "sys.exit(planner_fixture.main())\n")
    launcher.chmod(0o700)
    return {"command": str(launcher), "args": [], "env_allow": []}


if __name__ == "__main__":
    sys.exit(main())
