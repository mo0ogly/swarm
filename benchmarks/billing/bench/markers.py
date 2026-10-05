"""Marqueurs d'injection : la preuve qu'une faute a réellement eu lieu."""
import json
import os
import threading
import time
from pathlib import Path


def mark(path, **fields):
    """Écrit le marqueur de façon atomique ; un lecteur ne voit jamais un fichier partiel."""
    path = Path(path)
    tmp = path.with_suffix(path.suffix + f".{os.getpid()}.{threading.get_ident()}.tmp")
    tmp.write_text(json.dumps({**fields, "at": time.time()}))
    os.replace(tmp, path)


def read_all(directory):
    """Retourne {nom: contenu} pour chaque marqueur `*.json` du dossier."""
    return {p.stem: json.loads(p.read_text()) for p in sorted(Path(directory).glob("*.json"))}
