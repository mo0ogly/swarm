#!/usr/bin/env python3
"""D02 documentation checks against a freshly built isolated Swarm binary."""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


ROOT = Path(__file__).resolve().parents[1]
DOCUMENTS = (ROOT / "INSTALL.md", ROOT / "docs/en/INSTALL.md")
CAPTURES = {'fr-sombre-programs.png': 'ed9147643deb65b86fd017318f378498bce52734aa01b051ea5c208b7e19733a', 'fr-sombre-graph.png': 'e2d15e10c8d8124339c383fe2ec13324675f2af03fa226813aa9866862bac17e', 'fr-etat-programs.png': '7c0811167dd599b56f6f6f0e59b543b2b1bc53305481ef7bab9fdb85725dc162', 'fr-etat-graph.png': '3e978ce9ac23f7c094e9dea069ad2251b4a9339a9d4f9501726edbfb7f3a95d3', 'en-sombre-programs.png': '58b9f551b1aa5dba50d6535f55329ddbf9942c7f5877f3c47bcb6fc282b4dc98', 'en-sombre-graph.png': '2a7b8ef6d1d776321f3022365b90a83fece7edd4921028c9f0729b3ccf04edd3', 'en-etat-programs.png': '74c00de6f1c4c0ade932e5afcb9825112b6202705cfb3ace5151182e5ef7e721', 'en-etat-graph.png': '2922f3c51d0b281260630a5f0479b92005e4768a76d22d0e07c10ec7c20074f1'}


def run(command: list[str], expected: int = 0) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, timeout=60)
    if result.returncode != expected:
        raise AssertionError(
            f"command {command!r}: expected {expected}, got {result.returncode}\n"
            f"stdout={result.stdout[-2000:]}\nstderr={result.stderr[-2000:]}"
        )
    return result


def local_links(document: Path) -> list[Path]:
    targets: list[Path] = []
    for match in re.finditer(r"!?\[[^]]*\]\(([^)]+)\)", document.read_text(encoding="utf-8")):
        target = match.group(1).split("#", 1)[0]
        if not target or re.match(r"^[a-z]+://", target):
            continue
        targets.append((document.parent / target).resolve())
    return targets


def links_exist(document: Path) -> bool:
    return all(path.is_file() for path in local_links(document))


def require_semantic_parity() -> None:
    french = DOCUMENTS[0].read_text(encoding="utf-8").lower()
    english = DOCUMENTS[1].read_text(encoding="utf-8").lower()
    french_markers = (
        "sauvegarde et retour arrière vérifiables",
        "l’export d’une mission ne remplace pas la sauvegarde complète",
        "les programmes importés restent désactivés",
        "state-pre-v27-",
    )
    english_markers = (
        "verifiable backup and rollback",
        "a mission export does not replace the complete backup",
        "imported programs remain disabled",
        "state-pre-v27-",
    )
    missing = [item for item in french_markers if item not in french]
    missing += [item for item in english_markers if item not in english]
    if missing:
        raise AssertionError("missing FR/EN migration semantics: " + ", ".join(missing))


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: graph_delivery_docs.py OUTPUT")
    output = Path(sys.argv[1]).resolve()
    output.mkdir(parents=True, exist_ok=False)
    binary = output / "swarm"
    run(["sh", "build.sh", str(binary)])

    help_text = run([str(binary), "--help"]).stdout
    required_help = (
        "swarm init",
        "swarm lifecycle preview|apply",
        "swarm export",
        "swarm import",
        "swarm automation list|show|preview|create|enable|pause|archive|cancel|params",
    )
    if not all(command in help_text for command in required_help):
        raise AssertionError("documented command missing from canonical --help")
    run([str(binary), "--lang", "fr", "aide", "cycle-vie"])
    run([str(binary), "--lang", "en", "help", "lifecycle"])

    with tempfile.TemporaryDirectory(prefix="swarm-d02-docs-") as project:
        run([str(binary), "--root", project, "--json", "init"])
        run([str(binary), "--root", project, "--json", "work", "list"])
        run([str(binary), "--root", project, "--json", "lifecycle", "list"])
        run([str(binary), "--root", project, "--json", "automation", "list"])
        state = Path(project) / ".swarm/state.db"
        if not state.is_file() or (state.stat().st_mode & 0o777) != 0o600:
            raise AssertionError("isolated init did not create a private state.db")

    stale = subprocess.run(
        [str(binary), "definitely-stale-command"], cwd=ROOT,
        capture_output=True, text=True, timeout=20,
    )
    if stale.returncode == 0:
        raise AssertionError("negative command probe was unexpectedly accepted")
    negative_link = output / "negative-link.md"
    negative_link.write_text("[missing](does-not-exist-d02.md)\n", encoding="utf-8")
    if links_exist(negative_link):
        raise AssertionError("negative link probe did not detect a missing document")
    negative_link.unlink()
    for document in DOCUMENTS:
        if not links_exist(document):
            missing = [str(path.relative_to(ROOT)) for path in local_links(document) if not path.is_file()]
            raise AssertionError(f"stale local links in {document.relative_to(ROOT)}: {missing}")
    require_semantic_parity()

    secret_patterns = (
        re.compile(r"\bsk-[A-Za-z0-9_-]{16,}"),
        re.compile(r"https?://[^\s)]+[?&](?:token|session_token)=[^\s)]+", re.I),
    )
    for document in DOCUMENTS:
        text = document.read_text(encoding="utf-8")
        if any(pattern.search(text) for pattern in secret_patterns):
            raise AssertionError(f"possible secret in {document.relative_to(ROOT)}")

    capture_root = ROOT / "docs/screenshots/graph-delivery-d02"
    observed_captures: dict[str, dict[str, object]] = {}
    for name, expected_digest in CAPTURES.items():
        path = capture_root / name
        raw = path.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        if raw[:8] != b"\x89PNG\r\n\x1a\n" or len(raw) <= 1000 or digest != expected_digest:
            raise AssertionError(f"stale or invalid fresh D02 capture: {name}")
        observed_captures[name] = {"sha256": digest, "bytes": len(raw)}

    receipt = {
        "schema_version": 1,
        "surface": "swarm-documentation",
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "commands_checked": list(required_help),
        "documents": [str(path.relative_to(ROOT)) for path in DOCUMENTS],
        "captures": observed_captures,
        "negative_checks": ["unknown command rejected", "missing local link rejected"],
        "limits": [
            "Fresh D02 captures from candidate-bound host browser run; this documentation checker verifies hashes, host manifest verifies candidate identity.",
            "fixtures use temporary roots and no real AI provider",
        ],
    }
    (output / "results.json").write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "PASS", "receipt": str(output / "results.json")}, ensure_ascii=False))


if __name__ == "__main__":
    main()
