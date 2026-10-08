#!/usr/bin/env python3
"""Dossier de recherche en Word : assemble les chapitres de ce répertoire par pandoc.

Usage : build.py SORTIE.docx
Les chapitres sont pris dans l'ordre de leur nom (01-… à 95-…).
"""
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
HEAD = ["% Médiation déterministe des agents LLM sur processus à effets irréversibles",
        "% Fabrice Pizzi", "% Dossier de recherche, base de thèse — 7 octobre 2026", ""]


def main(out):
    chapters = sorted(p for p in HERE.glob("[0-9][0-9]-*.md"))
    text = "\n\n".join(HEAD + [p.read_text().strip() for p in chapters])
    with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as f:
        f.write(text)
        source = f.name
    subprocess.run(["pandoc", source, "-o", out, "--toc", "--toc-depth=2",
                    "--from", "markdown+pipe_tables-yaml_metadata_block-multiline_tables-simple_tables"], check=True)
    print(out)


if __name__ == "__main__":
    main(sys.argv[1])
