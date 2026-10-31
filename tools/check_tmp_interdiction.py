#!/usr/bin/env python3
"""Permanent guard for the operator /tmp interdiction (2026-10-31).

Operator interdiction: project data must never be allocated under `/tmp`.
This scanner is the non-regression guard: it statically detects temporary
allocations that would land in the system temp directory instead of the
repository fixture area (`<repo>/.swarm-fixtures/`).

Detected violation classes:
- Python: `tempfile.mkdtemp`, `tempfile.TemporaryDirectory`, `tempfile.mkstemp`,
  `tempfile.NamedTemporaryFile` called without an explicit `dir=` argument
  (positional `dir` counts as explicit);
- Python: `tempfile.gettempdir()` / `tempfile.tempdir` reads (reported for
  review: some are legitimate capacity observations, not allocations);
- JavaScript (.cjs/.js): `os.tmpdir()` occurrences and `mkdtempSync` calls
  whose prefix derives from `tmpdir` (reported for review).

Rust `env::temp_dir()` uses are covered at runner level (recipe.py pins
TMPDIR) and by the documented source rule; they are intentionally not part
of this guard's exit status.

Exit status: 0 when no Python allocation violation remains; 1 otherwise.
Review categories never affect the exit status. With `--json`, prints one
JSON object instead of text.

Usage: python3 tools/check_tmp_interdiction.py [--json] [paths...]
"""
import ast
import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parents[1]
SKIP_DIRS = {'.git', 'node_modules', '__pycache__', '.venv', 'venv', 'target',
             '.swarm-fixtures', 'dist', 'build'}

PY_ALLOC = {'mkdtemp', 'TemporaryDirectory', 'mkstemp', 'NamedTemporaryFile'}


def tempfile_bindings(tree):
    """Return (module_alias_set, from_imported_name_set) for tempfile imports."""
    mods, names = set(), set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            for alias in node.names:
                if alias.name == 'tempfile':
                    mods.add(alias.asname or 'tempfile')
        elif isinstance(node, ast.ImportFrom) and node.module == 'tempfile':
            for alias in node.names:
                names.add(alias.asname or alias.name)
    return mods, names


def scan_python(path, text):
    """Return (allocation_violations, review_items) for one Python file.

    Allocation violations are (line, call) tuples for tempfile allocation
    calls without an explicit `dir`; review items cover gettempdir/tempdir
    reads that a human must qualify.
    """
    try:
        tree = ast.parse(text)
    except SyntaxError as exc:
        return [(exc.lineno or 0, f'SyntaxError: {exc.msg}')], []
    mods, names = tempfile_bindings(tree)
    star_import = any(
        isinstance(n, ast.ImportFrom) and n.module == 'tempfile'
        and any(a.name == '*' for a in n.names)
        for n in ast.walk(tree))
    alloc, review = [], []
    for node in ast.walk(tree):
        if isinstance(node, ast.Call):
            func = node.func
            attr = None
            if isinstance(func, ast.Attribute) and isinstance(func.value, ast.Name) \
                    and func.value.id in mods:
                attr = func.attr
            elif isinstance(func, ast.Name) and (func.id in names or (star_import and func.id in PY_ALLOC)):
                attr = func.id
            if attr in PY_ALLOC:
                has_dir = any(kw.arg == 'dir' for kw in node.keywords) or len(node.args) >= 3
                if not has_dir:
                    alloc.append((node.lineno, attr))
            elif attr == 'gettempdir' or (isinstance(func, ast.Attribute)
                                          and isinstance(func.value, ast.Name)
                                          and func.value.id in mods and func.attr == 'tempdir'):
                review.append((node.lineno, 'gettempdir/tempdir read'))
        elif isinstance(node, ast.Attribute) and node.attr == 'tempdir' \
                and isinstance(node.value, ast.Name) and node.value.id in mods:
            # bare `tempfile.tempdir` reference outside a call
            review.append((node.lineno, 'tempdir attribute read'))
    return alloc, review


JS_TMP = re.compile(r"os\.tmpdir\(\)|require\('os'\)\.tmpdir\(\)")


def scan_js(path, text):
    """Return review items for os.tmpdir()/mkdtempSync-on-tmpdir in JS files."""
    review = []
    for i, line in enumerate(text.splitlines(), 1):
        if JS_TMP.search(line):
            review.append((i, 'os.tmpdir()'))
    return review


def iter_files(roots, suffixes):
    for root in roots:
        root = root.resolve()
        if root.is_file():
            if root.suffix in suffixes:
                yield root
            continue
        for p in sorted(root.rglob('*')):
            if p.suffix in suffixes and not (set(p.parts) & SKIP_DIRS):
                yield p


def main():
    args = [a for a in sys.argv[1:] if a != '--json']
    as_json = '--json' in sys.argv
    roots = [pathlib.Path(a) for a in args] or [REPO / 'tests', REPO / 'tools', REPO / 'web']
    result = {'violations': {}, 'reviews': {}}
    fail = 0
    for path in iter_files(roots, {'.py', '.cjs', '.js'}):
        text = path.read_text(errors='replace')
        if path.suffix == '.py':
            alloc, review = scan_python(path, text)
        else:
            alloc, review = [], scan_js(path, text)
        rel = str(path.relative_to(REPO))
        if alloc:
            result['violations'][rel] = alloc
            fail += len(alloc)
        if review:
            result['reviews'][rel] = review
    if as_json:
        print(json.dumps(result, indent=1))
    else:
        for rel, items in sorted(result['violations'].items()):
            for line, what in items:
                print(f"VIOLATION {rel}:{line}: {what}")
        for rel, items in sorted(result['reviews'].items()):
            for line, what in items:
                print(f"review    {rel}:{line}: {what}")
        print(f"== {fail} allocation violation(s), "
              f"{sum(len(v) for v in result['reviews'].values())} review item(s)")
    return 1 if fail else 0


if __name__ == '__main__':
    raise SystemExit(main())
