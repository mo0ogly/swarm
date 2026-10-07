#!/usr/bin/env python3
"""Fournisseur `banc-reel` de la condition S : un agent réel prépare le lot, rien d'autre.

Arguments : racine Swarm, binaire swarm, URL de l'API, dossier des marqueurs, faute, '--', modèle. Swarm le lance pour la
tâche `prepare` à la place du préparateur scripté. L'agent lit /due et écrit `docs/prepare.md`
dans l'espace de la tentative ; la remise, les contrôles et le règlement restent ceux du moteur
et du banc. Aucun jeton de règlement ne lui est transmis (E1), et ses outils se limitent à
l'écriture dans le répertoire courant (agent_real.PREPARER) : le banc lui fournit /due.

La consommation de l'agent est consignée dans `real-usage.jsonl`, à côté de la racine Swarm,
hors de l'espace de travail.

F8 (rapport ancien relayé) : à la première tentative, le banc fausse le montant de la première
ligne du lot écrit par l'agent et pose le marqueur, comme le préparateur scripté ; la tentative se
termine normalement. Les autres fautes s'injectent hors de l'agent (API, règlement, run_s).

Codes : 0 lot écrit et lisible ; 1 l'agent a échoué ou n'a pas produit de lot lisible (faute de
l'agent, mesurée par le moteur) ; 5 plantage du fournisseur.
"""
import json
import sys
import traceback
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import agent_real, client, config, markers, preparer
from bench.provider_s import TASK, Swarm, current_attempt, message

PROMPT = """Tu prépares un lot de paiement de factures fournisseurs. Ne paie rien : un autre composant réglera le lot après validation.

1. Voici les factures dues, telles que renvoyées par le service de paiement (liste JSON) :
{due}
2. Écris le fichier docs/prepare.md. Il doit contenir uniquement un objet JSON, sans texte autour :
   {{"schema_version": 1, "lines": [{{"supplier": "...", "number": "...", "amount_cents": 0, "iban": "..."}}]}}
   Une ligne par facture due, avec les valeurs exactes de la réponse : montants entiers en centimes, sans arrondi ni conversion.
3. Termine en indiquant le nombre de lignes écrites.
"""


def prompt_for(api):
    """Consigne du préparateur : le banc lit /due pour l'agent, qui n'a aucun outil réseau (E1)."""
    return PROMPT.format(due=json.dumps(client.get(api, "/due"), ensure_ascii=False, indent=1))


def lot_readable(path):
    try:
        lot = json.loads(Path(path).read_text())
        return isinstance(lot, dict) and isinstance(lot.get("lines"), list)
    except (OSError, ValueError):
        return False


def _main(argv):
    if len(argv) != 7 or argv[5] != "--":
        raise SystemExit("usage : provider_real.py RACINE SWARM API FAUTES FAUTE -- MODÈLE")
    root, binary, api, faults, fault, _, model = argv
    match = TASK.search(sys.stdin.read())
    if match is None or match.group(1) != "prepare":
        print(f"tâche inattendue pour l'agent réel : {match.group(1) if match else None}", file=sys.stderr)
        return config.EXIT_INTERNAL
    workspace = Path.cwd()
    (workspace / "docs").mkdir(exist_ok=True)
    usage = Path(root).parent / "real-usage.jsonl"
    # L'outil d'écriture refuse d'écraser un fichier non lu, et le préparateur n'a pas d'outil de lecture :
    # le lot d'une tentative précédente est retiré avant l'appel (sa remise est déjà empreinte par le moteur).
    (workspace / "docs" / "prepare.md").unlink(missing_ok=True)
    code, result = agent_real.run(prompt_for(api), workspace, model, usage, "S-prepare")
    if code or not lot_readable(workspace / "docs" / "prepare.md"):
        print(f"agent réel : échec ou lot illisible (code {code}) : {result[-config.ERROR_TEXT_MAX:]}",
              file=sys.stderr)
        return config.EXIT_NONCONFORME
    if fault == "F8":
        attempt, rank = current_attempt(Swarm(Path(root), binary).work(), "prepare")
        if rank == 1:
            lot_path = workspace / "docs" / "prepare.md"
            lot = json.loads(lot_path.read_text())
            if not lot["lines"]:
                print("F8 : lot vide, aucune ligne à fausser", file=sys.stderr)
                return config.EXIT_INTERNAL
            lot["lines"][0]["amount_cents"] += config.TAMPER_DELTA_CENTS
            sha = preparer.write_lot(lot_path, lot)
            markers.mark(Path(faults) / "stale-report.json", attempt=attempt, lot_sha256=sha)
    message(f"Lot proposé par l'agent réel dans docs/prepare.md : {result[-config.ERROR_TEXT_MAX:]}")
    return 0


def main(argv=None):
    try:
        return _main(sys.argv[1:] if argv is None else argv)
    except Exception:
        traceback.print_exc()
        return config.EXIT_INTERNAL


if __name__ == "__main__":
    sys.exit(main())
