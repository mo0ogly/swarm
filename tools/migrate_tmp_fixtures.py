#!/usr/bin/env python3
"""Migrate dir-less tempfile allocations to the repository fixture area.

Operator interdiction (2026-10-31): project data must never live in `/tmp`.
This tool rewrites `tempfile.mkdtemp(prefix=...)` and
`tempfile.TemporaryDirectory(prefix=...)` calls that lack `dir=`, adding
`dir=fixture_area()`. The transformation is purely mechanical:

- rewrites only call sites lacking `dir=`: `tempfile.mkdtemp(prefix=X)`
  becomes `tempfile.mkdtemp(prefix=X, dir=fixture_area())`;
- adds the location-correct import of the shared contract `fixture_area`:
  bare `from fixture_area import fixture_area` for `tests/` (those scripts
  run with their own directory as sys.path[0]) and a bounded sys.path shim
  for `tools/` (depth varies);
- normalizes previously inserted package-style imports
  (`from tests.fixture_area import fixture_area`) into that same
  location-correct form — the package form fails on direct execution;
- mkstemp calls are never touched (adjacent-to-destination atomic writes);
- every rewrite is verified: after migration the file must still parse
  (ast.parse) before being written.

Usage: python3 tools/migrate_tmp_fixtures.py [--dry-run] [paths...]
"""
import ast
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
PKG_IMPORT = "from tests.fixture_area import fixture_area"
DIRLESS_MD = re.compile(r"tempfile\.mkdtemp\(([^()]*)\)")
DIRLESS_TD = re.compile(r"tempfile\.TemporaryDirectory\(([^()]*)\)")


def import_block(path):
    """Return the import lines to insert for one migrated file.

    Files under `tests/` get a bare import: scripts there run with their own
    directory as sys.path[0] (direct execution, unittest discovery, recipe
    subprocesses), matching the existing cross-imports between tests scripts.
    Files under `tools/` get a bounded sys.path shim to `<repo>/tests`: depth
    varies (tools/ vs tools/verification/ vs tools/rust-migration/), so the
    shim is computed from the file's depth below the repository root.
    """
    rel = path.resolve().relative_to(REPO)
    if rel.parts[0] == 'tests':
        return "from fixture_area import fixture_area"
    depth = len(rel.parts) - 1  # directory levels between file and repo root
    return ("import sys as _fa_sys\n"
            "from pathlib import Path as _fa_Path\n"
            f"sys.path.insert(0, str(_fa_Path(__file__).resolve().parents[{depth}] / 'tests'))\n"
            "from fixture_area import fixture_area")


def fix_imports(text, path):
    """Rewrite a package-style fixture_area import into the location-correct form.

    Returns (text, changed). Idempotent: the correct form is left untouched.
    """
    if PKG_IMPORT not in text:
        return text, False
    block = import_block(path)
    if block in text:
        return text, False
    return text.replace(PKG_IMPORT, block), True


def rewrite(text):
    """Rewrite dir-less mkdtemp/TemporaryDirectory calls; return (text, count).

    Only calls lacking `dir=` are rewritten; those with `dir=` are left as is.
    """
    counts = {}

    def sub(match):
        args = match.group(1)
        if 'dir=' in args:
            return match.group(0)
        counts[name] = counts.get(name, 0) + 1
        sep = '' if not args.strip() else args.rstrip() + ', '
        return f"tempfile.{name}({sep}dir=fixture_area())"

    for name, pat in (('mkdtemp', DIRLESS_MD), ('TemporaryDirectory', DIRLESS_TD)):
        text = pat.sub(sub, text)
    return text, sum(counts.values())


def _insert_after_docstring(text, stmt):
    """Insert a python statement after the module docstring (or at top)."""
    try:
        tree = ast.parse(text)
    except SyntaxError:
        return stmt + '\n' + text
    lines = text.splitlines(keepends=True)
    if tree.body and isinstance(tree.body[0], ast.Expr) and isinstance(tree.body[0].value, ast.Constant):
        end = tree.body[0].end_lineno
        return ''.join(lines[:end]) + stmt + '\n' + ''.join(lines[end:])
    return stmt + '\n' + text


def add_import(text, path):
    """Insert the fixture_area import when absent; return (text, inserted).

    Prefers the line `import tempfile` (insertion right after it), then any
    single-line combined import containing tempfile, then top-of-module after
    the docstring.
    """
    if re.search(r"^from fixture_area import fixture_area$", text, re.M) \
            or re.search(r"^from fixture_area import fixture_area$", text, re.M):
        return text, False
    block = import_block(path)
    if block in text:
        return text, False
    m = re.search(r"^import tempfile$", text, re.M)
    if m:
        return (text[:m.start()] + "import tempfile\n" + block + text[m.end():], True)
    m = re.search(r"^import [a-z0-9_,]*tempfile[a-z0-9_,]*$", text, re.M)
    if m:
        seg = m.group(0)
        newseg = seg[:-len('tempfile')] + 'tempfile\n' + block
        return text[:m.start()] + newseg + text[m.end():], True
    return _insert_after_docstring(text, block), True


def process(path, dry):
    """Migrate one file; return (calls_rewritten, changes_made, status_note)."""
    text = path.read_text(errors='strict')
    text, n = rewrite(text)
    text, imp = add_import(text, path) if n else (text, False)
    text, fixed = fix_imports(text, path)
    if n == 0 and not fixed:
        return 0, 0, 'no-dirless-calls'
    if not dry:
        ast.parse(text)  # must still parse before writing
        path.write_text(text)
    return n, int(imp) + int(fixed), 'migrated'


META = {pathlib.Path(__file__).resolve(),  # this tool's own docstrings
        REPO / 'tests' / 'fixture_area.py'}


def main():
    dry = '--dry-run' in sys.argv
    args = [a for a in sys.argv[1:] if a != '--dry-run']
    paths = [pathlib.Path(p) for p in args] or [
        p for base in ('tests', 'tools') for p in pathlib.Path(base).rglob('*.py')
    ]
    paths = [p for p in paths if p.resolve() not in META]
    migrated, calls, files = [], 0, 0
    for p in sorted(paths):
        try:
            n, changes, note = process(p, dry)
        except SyntaxError as e:
            print(f"SKIP {p}: parse error {e}")
            continue
        except UnicodeDecodeError as e:
            print(f"SKIP {p}: decode error {e}")
            continue
        if n or changes:
            migrated.append(str(p))
            calls += n
            files += 1
            print(f"{p}: {n} call(s), import-ops={changes}")
        elif note == 'no-dirless-calls' and not dry:
            pass
    print(f"== {files} file(s), {calls} call(s) migrated{' (dry-run)' if dry else ''}")
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
