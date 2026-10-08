#!/usr/bin/env python3
"""Assemble les brouillons de l'article en un seul document (Markdown, puis Word par pandoc).

Usage : assemble.py SORTIE.docx

De chaque brouillon, seul le corps est repris : l'en-tête de statut (avant le premier « --- ») et
les « Points à vérifier », « à décider » ou « ouverts » sont retirés ; références et annexes restent. Le
protocole d'attribution (QR3) est placé en annexe. Numérotation provisoire : la section 8 du plan
est absorbée par les sections 4 et 6.
"""
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
TITLE = "Proposer n'est pas agir : un moniteur déterministe pour agents LLM sur processus à effets irréversibles"
PARTS = [
    ("brouillon-resume-conclusion.md", "Résumé"),
    ("brouillon-sections-2-3.md", None),
    ("brouillon-section-4.md", None),
    ("brouillon-section-5.md", None),
    ("brouillon-section-6-protocole.md", None),
    ("brouillon-section-7-resultats.md", None),
    ("brouillon-sections-9-10.md", None),
    ("brouillon-resume-conclusion.md", "Conclusion"),
    ("brouillon-section-7-protocole.md", "Annexe"),
]


def body(text):
    """Corps du brouillon : après l'en-tête de statut, jusqu'aux points à vérifier ou à décider."""
    text = re.split(r"^---\s*$", text, maxsplit=1, flags=re.M)[-1]
    text = re.split(r"^## Points ", text, maxsplit=1, flags=re.M)[0]
    return re.sub(r"^---\s*$", "", text, flags=re.M).strip()


def section(name, which):
    text = body((HERE / name).read_text())
    if which == "Résumé":
        return text.split("## 11.")[0].strip()
    if which == "Conclusion":
        return "## 11." + text.split("## 11.", 1)[1]
    if which == "Annexe":
        text = text.replace("## 7. Attribution des fautes", "## Annexe A. Protocole d'attribution des blocages (QR3)")
        return re.sub(r"^### 7\.(\d)", r"### A.\1", text, flags=re.M)
    return text


def main(out):
    doc = [f"% {TITLE}", "% Fabrice Pizzi", "% Brouillon de travail — numérotation provisoire", ""]
    main_parts, tail = [], []
    for name, which in PARTS:   # références et annexes reportées en fin de document
        blocks = re.split(r"^(?=## (?:Références|Annexe))", section(name, which), flags=re.M)
        main_parts.append(blocks[0])
        tail += blocks[1:]
    refs = [b for b in tail if b.startswith("## Références")]
    doc += [x.strip() + "\n" for x in main_parts + refs + [b for b in tail if b not in refs]]
    with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as f:
        f.write("\n\n".join(doc))
        source = f.name
    subprocess.run(["pandoc", source, "-o", out, "--from", "markdown+pipe_tables-yaml_metadata_block-multiline_tables-simple_tables", "--toc"], check=True)
    print(out)


if __name__ == "__main__":
    main(sys.argv[1])
