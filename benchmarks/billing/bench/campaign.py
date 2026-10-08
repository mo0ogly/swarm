#!/usr/bin/env python3
"""Campagne : grille condition × clé × faute × répétition, dans un JSONL reprenable (protocole 6.8).

- Grille : conditions B0, B1, S ; clés de harness.KEY_MODES ; cas de harness.FAULTS (sans faute,
  F1 à F8 et F4e) ; graines seed_base + répétition, identiques d'une condition à l'autre.
- Reprise : une case déjà présente dans le JSONL (même condition, clé, faute et graine), quel que
  soit son statut, est sautée. Une ERREUR est un résultat publié (6.7), pas une case à rejouer.
- Erreurs : une exception devient une ligne `ERREUR`, avec la racine conservée quand elle est
  connue (`harness.BenchError.run_dir`).
- Parallélisme par condition : les exécutions B (`--jobs-b`) puis S (`--jobs-s`), jamais en même
  temps. Une exécution S parallèle charge la machine et fausse les durées, donc la mesure de H5 :
  `--jobs-s 1` par défaut.
- Empreinte du banc : lignes `kind: "campaign"` au début (`provenance()`) et à la fin (empreinte
  recalculée, harness.bench_digest). Si elle a changé, la ligne de fin porte un avertissement.
- Purge (`--purge-ok`, désactivée par défaut) : après l'écriture de la ligne d'une exécution OK,
  sa racine `run_dir` est supprimée ; la ligne garde `run_dir` exact et porte `run_dir_purged: true`.
  Les racines INVALIDE, DÉLAI et ERREUR sont conservées pour diagnostic. Si la suppression échoue,
  une ligne `kind: "campaign"`, `event: "purge-failed"` le dit.
- Interruption (Ctrl-C) : plus aucune case n'est lancée, les exécutions en cours se terminent et
  sont écrites, puis une ligne `interrupted` est ajoutée ; code de sortie 130. Chaque ligne est
  écrite d'un seul appel système en ajout (O_APPEND) : le fichier ne contient que des lignes complètes.
"""
import argparse
import itertools
import json
import os
import shutil
import sys
import threading
import time
from concurrent.futures import FIRST_COMPLETED, ThreadPoolExecutor, wait
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, run_b, run_s

CONDITIONS = ("B0", "B1", "S")
_WRITE = threading.Lock()


def one(condition, key, fault, seed):
    """Une exécution ; toute exception devient une ligne ERREUR, jamais un succès silencieux."""
    try:
        if condition == "S":
            return run_s.run(key, fault, seed)
        return run_b.run(condition, key, fault, seed)
    except Exception as e:   # une case en erreur ne doit pas arrêter la campagne ; elle est publiée
        return {"condition": condition, "key_mode": key, "fault": fault, "seed": seed, "status": "ERREUR",
                "error": f"{type(e).__name__}: {e}"[:config.ERROR_TEXT_MAX * 3],
                "run_dir": getattr(e, "run_dir", None), "provenance": dict(harness.provenance())}


def plan(conditions, keys, faults, reps, seed_base):
    return [(c, k, f, seed_base + rep) for c, k, f, rep in itertools.product(conditions, keys, faults, range(reps))]


def append(out, record):
    """Ajoute une ligne complète en un seul appel système (O_APPEND) : jamais de ligne partielle entrelacée."""
    data = (json.dumps(record, ensure_ascii=False) + "\n").encode()
    with _WRITE:
        fd = os.open(out, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
        try:
            os.write(fd, data)
        finally:
            os.close(fd)


def publish(out, record, purge):
    """Écrit la ligne, puis supprime la racine d'une exécution OK si la purge est demandée."""
    root = record.get("run_dir")
    purge = purge and record.get("status") == "OK" and bool(root)
    if purge:
        record = {**record, "run_dir_purged": True}
    append(out, record)
    if purge:
        try:
            shutil.rmtree(root)
        except OSError as e:   # la ligne annonçait la purge : la corriger, ne jamais laisser mentir le JSONL
            append(out, {"kind": "campaign", "event": "purge-failed", "run_dir": root, "error": str(e),
                         "case": [record.get(k) for k in ("condition", "key_mode", "fault", "seed")]})


def already_done(out):
    done = set()
    if out.exists():
        for number, line in enumerate(out.read_text().splitlines(), 1):
            if not line.strip():
                continue
            try:
                r = json.loads(line)
            except ValueError as e:
                raise SystemExit(f"{out}:{number} : ligne illisible, reprise refusée ({e})") from e
            if r.get("kind") != "campaign":
                done.add((r["condition"], r["key_mode"], r["fault"], r["seed"]))
    return done


def run_group(out, todo, jobs, purge=False):
    """Exécute un groupe de cases ; True si interrompu. Aucune case n'est lancée après l'interruption."""
    pending = iter(todo)
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        running = set()
        try:
            for args in itertools.islice(pending, jobs):
                running.add(pool.submit(one, *args))
            while running:
                finished, running = wait(running, return_when=FIRST_COMPLETED)
                for future in finished:
                    publish(out, future.result(), purge)   # KeyboardInterrupt d'un exécutant remonte ici
                    nxt = next(pending, None)
                    if nxt is not None:
                        running.add(pool.submit(one, *nxt))
        except KeyboardInterrupt:
            for future in running:   # les exécutions en cours se terminent et sont écrites
                try:
                    result = future.result()
                except BaseException:   # une exécution elle-même interrompue n'a pas de résultat
                    continue
                if result.get("status") == "ERREUR":
                    # Ctrl-C atteint aussi les sous-processus : cette ERREUR peut venir de l'interruption.
                    # Elle n'est pas publiée comme résultat ; la case reste à faire à la reprise.
                    append(out, {"kind": "campaign", "event": "discarded-after-interrupt",
                                 "case": [result.get(k) for k in ("condition", "key_mode", "fault", "seed")],
                                 "error": result.get("error"), "run_dir": result.get("run_dir")})
                    continue
                publish(out, result, purge)
            return True
    return False


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", required=True)
    p.add_argument("--reps", type=int, required=True)
    p.add_argument("--seed-base", type=int, default=1000)
    p.add_argument("--conditions", default=",".join(CONDITIONS))
    p.add_argument("--keys", default=",".join(harness.KEY_MODES))
    p.add_argument("--faults", default=",".join(harness.FAULTS))
    p.add_argument("--jobs-b", type=int, default=4)
    p.add_argument("--jobs-s", type=int, default=1)
    p.add_argument("--purge-ok", action="store_true",
                   help="supprimer la racine d'une exécution OK après écriture de sa ligne")
    a = p.parse_args(argv)
    out = Path(a.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    conditions, keys, faults = a.conditions.split(","), a.keys.split(","), a.faults.split(",")
    for value, allowed in ((conditions, CONDITIONS), (keys, harness.KEY_MODES), (faults, harness.FAULTS)):
        unknown = set(value) - set(allowed)
        if unknown:
            raise SystemExit(f"valeur inconnue : {sorted(unknown)}")
    done = already_done(out)
    grid = plan(conditions, keys, faults, a.reps, a.seed_base)
    todo = [case for case in grid if case not in done]
    start = dict(harness.provenance())
    begin = time.time()
    append(out, {"kind": "campaign", "event": "start", "at": begin, "grid": len(grid), "todo": len(todo),
                 "args": vars(a), **start})
    interrupted = False
    for group, jobs in ((("B0", "B1"), a.jobs_b), (("S",), a.jobs_s)):
        cases = [case for case in todo if case[0] in group]
        if cases and run_group(out, cases, jobs, a.purge_ok):
            interrupted = True
            break
    bench_end = harness.bench_digest(harness.BENCH_DIR)
    swarm_end = harness.file_sha256(harness.SWARM_BIN) if harness.SWARM_BIN.is_file() else None
    end = {"kind": "campaign", "event": "interrupted" if interrupted else "end", "at": time.time(),
           "duration_s": round(time.time() - begin, 1), "bench_sha256": bench_end, "swarm_sha256": swarm_end,
           "bench_changed": bench_end != start["bench_sha256"], "swarm_changed": swarm_end != start["swarm_sha256"]}
    if end["bench_changed"] or end["swarm_changed"]:
        end["warning"] = ("empreinte du banc ou du binaire swarm modifiée pendant la campagne : "
                          "les résultats ne viennent pas d'un seul code mesuré")
    append(out, end)
    print(f"{len(todo)} exécution(s) prévue(s) ; fichier {out}" + (" ; interrompu" if interrupted else ""))
    return 130 if interrupted else 0


if __name__ == "__main__":
    sys.exit(main())
