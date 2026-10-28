#!/usr/bin/env python3
"""Freeze a recipe's declared inputs into a private snapshot and run it there.

Concurrent work in the same checkout changes declared sources during a run and
the recipe correctly refuses (`source_stable: false`). This tool copies ONLY the
files selected by the recipe's own `inputs` (same selection and refusals as
`recipe.py`), plus the recipe manifest and runner, into a fresh directory, links
explicitly named dependency directories (never copied, never declared inputs)
and runs `recipe.py run --root <snapshot>`. If the snapshot compiles and passes,
the declared inputs were sufficient: an undeclared input fails visibly instead
of being read from the live checkout.

It grants no rights and touches no live state: `.swarm`, `.git`, dependency and
build caches are excluded by `recipe.snapshot`. The snapshot must be on a disk
with room for the build cache the recipe's environment points at (a tmpfs such
as /tmp may be too small). Exit codes follow recipe.py: 0 PASS, 1 FAIL, 2 refusal.
"""
import argparse
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('swarm_recipe', HERE / 'recipe.py')
recipe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recipe)


EMBEDDED = re.compile(r'include_(?:str|bytes)!\(\s*"([^"]+)"')


def undeclared_embedded(root, inventory):
    """Literal `include_str!`/`include_bytes!` targets of declared `.rs` files missing from `inventory`.

    Resolved as rustc does, relative to the including file's directory. A target
    escaping the repository is returned as written relative to the root (it can
    never be declared). Limits: macro arguments built with `concat!`/`env!` are
    not resolved (the compile step still fails visibly in the snapshot), and a
    commented-out include is counted, which can only cause an extra refusal.
    Returns a sorted list of repository-relative paths; empty when complete.
    """
    missing = set()
    for name in inventory:
        if not name.endswith('.rs'):
            continue
        text = (root / name).read_text(encoding='utf-8', errors='replace')
        for match in EMBEDDED.finditer(text):
            target = os.path.normpath(os.path.join(os.path.dirname(name), match.group(1)))
            if target not in inventory:
                missing.add(target)
    return sorted(missing)


def freeze(root, recipe_path, destination, links):
    """Copy declared inputs byte-exactly into a new `destination`.

    Refuses an existing destination, a link name escaping the snapshot, a
    missing link target, a declared Rust source embedding an undeclared file
    (`snapshot-embedded-input-undeclared: <paths>`, before anything is created),
    and any file that changes while it is copied (digest re-checked on the copy).
    Returns the inventory {relative path: sha256}.
    """
    manifest = recipe.load_recipe(recipe_path.read_bytes())
    # Validate every link before creating anything: a refusal leaves no partial snapshot.
    for link in links:
        relative = Path(link)
        if relative.is_absolute() or '..' in relative.parts or not relative.parts:
            raise ValueError('snapshot-link-invalid')
        if not (root / relative).exists():
            raise ValueError('snapshot-link-target-missing')
    inventory = recipe.snapshot(root, manifest['inputs'], destination, recipe_path)
    missing = undeclared_embedded(root, inventory)
    if missing:
        raise ValueError('snapshot-embedded-input-undeclared: ' + ' '.join(missing))
    runner = root / 'tools/verification/recipe.py'
    if not runner.is_file():
        raise ValueError('snapshot-runner-missing')
    inventory.setdefault(str(runner.relative_to(root)), recipe.digest(runner.read_bytes()))
    destination.mkdir(parents=False, exist_ok=False)
    for name, expected in inventory.items():
        target = destination / name
        target.parent.mkdir(parents=True, exist_ok=True)
        raw = (root / name).read_bytes()
        if recipe.digest(raw) != expected:
            raise ValueError('snapshot-source-changed-during-copy')
        target.write_bytes(raw)
        shutil.copymode(root / name, target)
    for link in links:
        relative = Path(link)
        source = root / relative
        place = destination / relative
        place.parent.mkdir(parents=True, exist_ok=True)
        os.symlink(source, place)
    recipe.publish(destination / '.snapshot-inventory.json',
                   json.dumps(dict(schema_version=1, recipe=manifest['id'], root=str(root), links=list(links), files=inventory), indent=1).encode())
    return inventory


def main():
    """Freeze then run; prints the snapshot path and recipe.py's own result line."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--recipe', type=Path, required=True, help='Recipe manifest inside the repository')
    parser.add_argument('--snapshot', type=Path, required=True, help='Fresh directory to create (main disk recommended)')
    parser.add_argument('--output', type=Path, help='Fresh evidence directory; omit to freeze only')
    parser.add_argument('--link', action='append', default=[], help='Repository-relative dependency directory to symlink (e.g. node_modules); repeatable')
    parser.add_argument('--root', type=Path, default=recipe.REPOSITORY)
    args = parser.parse_args()
    root = args.root.resolve()
    recipe_path = args.recipe.resolve()
    try:
        inventory = freeze(root, recipe_path, args.snapshot.resolve(), args.link)
    except Exception as error:
        print(json.dumps({'status': 'REFUSED', 'cause': str(error), 'type': type(error).__name__}))
        return 2
    print(json.dumps({'snapshot': str(args.snapshot.resolve()), 'files': len(inventory), 'links': args.link}))
    if args.output is None:
        return 0
    snap = args.snapshot.resolve()
    frozen_recipe = snap / recipe_path.relative_to(root)
    run = subprocess.run([sys.executable, str(snap / 'tools/verification/recipe.py'), 'run', '--root', str(snap), '--recipe', str(frozen_recipe), '--output', str(args.output.resolve())])
    return run.returncode


if __name__ == '__main__':
    raise SystemExit(main())
