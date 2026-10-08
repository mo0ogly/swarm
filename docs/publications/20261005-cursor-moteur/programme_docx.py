#!/usr/bin/env python3
"""Programme de recherche en Word, détaillé : le cadrage suivi d'annexes reprises des documents sources.

Usage : programme_docx.py SORTIE.docx [RACINE_VERIFICATION]

Les annexes reprennent tel quel le corps des documents déjà rédigés et vérifiés (modèle,
protocole, résultats, défauts, vérification des correctifs, étude S-collectif, protocole QR3) ;
aucun texte n'est ajouté ici. RACINE_VERIFICATION désigne l'arbre de travail de la branche de
vérification des correctifs (par défaut ~/workspace/swarm-verif), qui porte les deux documents les
plus récents de l'étude.
"""
import re
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[2]
ART = HERE / "article-scientifique"
sys.path.insert(0, str(ART))
from assemble import body  # noqa: E402


def doc_body(path):
    """Corps d'un document : brouillon (en-tête de statut retiré) ou note (titre de niveau 1 retiré)."""
    text = Path(path).read_text()
    if re.search(r"^---\s*$", text, flags=re.M):
        return body(text)
    return re.sub(r"\A# .*\n", "", text).strip()


def demote(text, levels=1):
    return re.sub(r"^(#+) ", lambda m: "#" * min(6, len(m.group(1)) + levels) + " ", text, flags=re.M)


def main(out, verif_root):
    verif = Path(verif_root) / "docs" / "benchmarks" / "billing"
    annexes = [
        ("A", "Modèle : acteurs, état, fautes, invariants", ART / "brouillon-section-4.md"),
        ("B", "Protocole d'évaluation et amendements", ART / "brouillon-section-6-protocole.md"),
        ("C", "Résultats de la campagne scriptée et du lot réel", ART / "brouillon-section-7-resultats.md"),
        ("D", "Défauts du moteur relevés par le banc", REPO / "docs" / "benchmarks" / "billing" / "defauts-moteur.md"),
        ("E", "Vérification des correctifs (pré-enregistrement ; résultats en cours)", verif / "verification-correctifs.md"),
        ("F", "Étude S-collectif : coordination par des agents réels (conception)", verif / "conception-collectif.md"),
        ("G", "Protocole d'attribution des blocages (QR3)", ART / "brouillon-section-7-protocole.md"),
    ]
    parts = ["% Programme de recherche : médiation déterministe des agents LLM sur processus à effets irréversibles",
             "% Fabrice Pizzi", "% Document de travail — 7 octobre 2026", "",
             re.sub(r"^## ", "# ", doc_body(HERE / "programme-recherche.md"), flags=re.M)]
    for letter, title, path in annexes:
        parts += [f"# Annexe {letter}. {title}", f"*Source : `{path.relative_to(path.parents[3])}`*",
                  demote(doc_body(path), 1)]
    with tempfile.NamedTemporaryFile("w", suffix=".md", delete=False) as f:
        f.write("\n\n".join(parts))
        source = f.name
    subprocess.run(["pandoc", source, "-o", out, "--toc", "--toc-depth=2",
                    "--from", "markdown+pipe_tables-yaml_metadata_block-multiline_tables-simple_tables"], check=True)
    print(out)


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2] if len(sys.argv) > 2 else str(Path.home() / "workspace" / "swarm-verif"))
