"""Check the documentation decision from the repository root; no runtime tests."""
import datetime
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[5]
EVIDENCE = Path(__file__).resolve().parent
SECTIONS = {
    "docs/plans/actions-confiance/identites-agents-replan.md": "## Replan IA3c.2f-04",
    "docs/plans/actions-confiance/identites-agents-enrolement-IA3c2.md": "### Démarrage sans fournisseur externe",
    "docs/architecture/AGENT-IDENTITY-CONTRACT.md": "### Authentification locale par défaut",
    "docs/en/AGENT-IDENTITY-CONTRACT.md": "### Local authentication by default",
    "INSTALL.md": "**Parcours d’authentification retenu",
    "docs/en/INSTALL.md": "**Selected authentication flow",
    "docs/publications/20261005-cursor-moteur/article-scientifique/identites-autorite-cycle-vie.md": "### Parcours local par défaut",
}
links, hashes = [], {}
for name, marker in SECTIONS.items():
    path = ROOT / name
    content = path.read_text()
    assert content.count(marker) == 1, (name, "missing or duplicate section")
    section = content.split(marker, 1)[1]
    if name.endswith("AGENT-IDENTITY-CONTRACT.md") or name in ("INSTALL.md", "docs/en/INSTALL.md"):
        section = section.split("\n## ", 1)[0]
    for target in re.findall(r"\]\(([^)]+)\)", section):
        if re.match(r"https?://|#|mailto:", target):
            continue
        assert (path.parent / target.split("#", 1)[0]).exists(), (name, target)
        links.append({"source": name, "target": target})
    hashes[name] = hashlib.sha256(path.read_bytes()).hexdigest()
for lang in ("fr", "en", "parcours"):
    assert (EVIDENCE / f"{lang}.png").stat().st_size > 0
diagram_sources = {
    "fr": ("docs/architecture/AGENT-IDENTITY-CONTRACT.md", "Frontière adoptée"),
    "en": ("docs/en/AGENT-IDENTITY-CONTRACT.md", "Adopted boundary"),
    "parcours": ("docs/plans/actions-confiance/identites-agents-enrolement-IA3c2.md", "Démarrage sans fournisseur externe"),
}
for lang, (name, marker) in diagram_sources.items():
    section = (ROOT / name).read_text().split(marker, 1)[1]
    source = re.search(r"```mermaid\n(.*?)\n```", section, re.S)
    assert source and source[1] + "\n" == (EVIDENCE / f"{lang}.mmd").read_text(), (lang, "stale diagram source")
normalize = lambda value: re.sub(r'"[^"]*"', '"LABEL"', value)
assert normalize((EVIDENCE / "fr.mmd").read_text()) == normalize((EVIDENCE / "en.mmd").read_text())
diff = subprocess.run(["git", "diff", "--check"], cwd=ROOT, capture_output=True, text=True)
assert diff.returncode == 0, diff.stderr + diff.stdout
checks = {
    "sections": len(SECTIONS),
    "resolved_local_links": len(links),
    "fr_en_graph_topology": "PASS",
    "git_diff_check": "PASS",
    "rendered_diagrams": 3,
    "visual_inspection": "performed in Codex; not asserted by this script",
}
report = {
    "date_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
    "scope": "documentation decision only; no application tests or activation",
    "command": "python3 " + str(Path(__file__).relative_to(ROOT)),
    "checks": checks,
    "documents_sha256": hashes,
    "local_links": links,
    "runtime_criteria": {"IA3c.2f-04-UX1..UX7": "NOT TESTED"},
    "account_created": False,
    "service_started": False,
    "workflow_started": False,
    "independent_review": False,
}
(EVIDENCE / "documentation-checks-final.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps(checks, ensure_ascii=False))
