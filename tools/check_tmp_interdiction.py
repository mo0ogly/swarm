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
  whose prefix derives from `tmpdir` (reported for review);
- Rust (.rs): `std::env::temp_dir()` allocations (reported for review, and as
  violations when they join-create a directory — see RUST_EXEMPT below). The
  D04 migration (2026-11-07) moved fixture allocation to
  `crate::fixture_root()`/`swarm_engine::fixture_root()` under the configured
  fixture area; remaining `temp_dir()` uses must be one of the documented
  exemptions (read-only probes of the system volume, paths asserted not to
  exist, assertions on the TMPDIR pinning itself, or pure path composition).
- Rust (.rs): bare `tempfile::tempdir()`, `TempDir::new()`,
  `NamedTempFile::new()` and `Builder...tempdir()` allocations (no `_in`
  suffix). The D05 migration (2026-11-07, same day) moved them to
  `tempdir_in(area)`/`new_in(area)` — RAII kept — with the area chosen by
  contract: the repository fixture area where ancestor privacy is not
  contracted, the operator fallback area where private-directory contracts
  refuse group-writable ancestors. Legitimate without migration: allocation
  into `CARGO_TARGET_TMPDIR` (cargo-managed target scratch, not /tmp) and the
  TMPDIR-pinning tests that read the pinned `TMPDIR` variable themselves.

Exit status: 0 when no allocation violation remains; 1 otherwise.
Review categories never affect the exit status. With `--json`, prints one
JSON object instead of text.

Default scan roots cover `tests/`, `tools/`, `web/` AND `rust/` (the Rust
mirror is part of the contract since D04/D05; before, `rust/` had to be
passed explicitly and was silently missed).

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

# Rust `std::env::temp_dir()` sites exempt from the D04 source rule (2026-11-07).
# Each entry: repo-relative path -> set of 1-based line numbers. An exempt line
# must never allocate a fixture: read-only probes of the system temp volume,
# paths asserted not to exist, assertions on the TMPDIR pinning itself, or pure
# path composition without creation. Additions require updating this map AND
# rust/crates/swarm-engine/src/fixture_area.rs module documentation in the same
# change, with the qualifying category stated in the accompanying plan entry.
RUST_EXEMPT = {
    'rust/crates/swarm-engine/src/mission_status_runtime.rs': {
        57,   # read-only temp-volume health probe entry ("temporary" row)
        538,  # test: path asserted NOT to exist, never created
        584,  # test: probe() assertion on the system volume itself
        586,  # test: path asserted NOT to exist, never created
    },
    'rust/crates/swarm-engine/src/root_store/preparation_call_cli.rs': {
        # 2026-11-08: the concurrent with_internet_denied snapshot insertion
        # (~line 589) shifted these assertions from 682/683 to 686/687.
        686,  # test: assertion — workspace must NOT be under temp_dir
        687,  # test: assertion — temp_dir must NOT be inside workspace
    },
    'rust/crates/swarm-engine/tests/runtime_temporary.rs': {
        29,   # test: asserts TMPDIR pinning (env::temp_dir() == applied)
    },
}

RS_TEMP_DIR = re.compile(r'\bstd::env::temp_dir\s*\(\s*\)|\benv::temp_dir\s*\(\s*\)')

# Rust bare-tempfile allocation forms (D05, 2026-11-07): `tempdir()`,
# `TempDir::new()`, `NamedTempFile::new()` and `Builder...tempdir()` without an
# `_in` variant allocate into the system temp directory (TMPDIR-honoring) and
# violate the operator /tmp interdiction on a bare cargo run. The governed
# forms end in `_in(area)` and are not matched here. `new()` inside a longer
# identifier (`new_in`) is excluded by the (?!\w) guard. A `Builder::new()` is
# NOT itself a violation — thread/hyper/tokio builders share the shape — the
# tempfile Builder chain is caught by the pending `.tempdir` window below.
RS_BARE_TEMP = re.compile(
    r'\btempfile::tempdir\s*\(\s*\)'
    r'|\btempdir\s*\(\s*\)(?!\w)'          # after `use tempfile::tempdir;`
    r'|\bTempDir::new\s*\(\s*\)(?!\w)'
    r'|\bNamedTempFile::new\s*\(\s*\)(?!\w)'
)

# Rust allocation sites that are legitimate without migration (D05 categories):
# - CARGO_TARGET_TMPDIR: cargo-managed target scratch directory, never /tmp;
# - `var_os("TMPDIR")` / `var("TMPDIR")`: the test reads the runner-pinned
#   TMPDIR itself (TMPDIR-pinning assertions and `tempdir_in(area)` on the
#   pinned value). A file using the pinned TMPDIR as its allocation area is
#   governed by recipe.py's net, which is the contractual runner coverage.
RS_LEGIT_TMP = re.compile(r'CARGO_TARGET_TMPDIR|var_os\(\s*"TMPDIR"\s*\)|var\(\s*"TMPDIR"\s*\)')


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


def scan_rust(path, rel):
    """Return (violations, review) for one Rust file.

    Every `env::temp_dir()` occurrence in code is reported; a line of the
    documented RUST_EXEMPT map is a review item (never a violation), any other
    line is an allocation violation — after the D04 migration, fixture
    allocation must go through fixture_root() and a new temp_dir() site needs
    an explicit, documented exemption before it lands. Every bare-tempfile
    allocation (`tempdir()`, `TempDir::new()`, `NamedTempFile::new()`,
    `Builder...tempdir()`) is a violation unless the line also references a
    legitimate governed area (CARGO_TARGET_TMPDIR or the runner-pinned TMPDIR
    variable) — after the D05 migration those allocations go through
    `tempdir_in(area)`/`new_in(area)` with the area chosen by the crate's
    contracts. A multi-line Builder chain (`.tempdir()` on a later line than
    `Builder::new()`) is caught by retaining one pending-Builder line window.
    Line comments (//, ///, //!) and everything after them are ignored: doc
    mentions of the migrated classes are not call sites.
    """
    review = []
    violations = []
    exempt = RUST_EXEMPT.get(rel, set())
    pending_builder = -10  # line number of a recent Builder::new( opening
    for i, line in enumerate(path.read_text(errors='replace').splitlines(), 1):
        code = line.split('//', 1)[0]
        hit = False
        what = None
        if RS_TEMP_DIR.search(code):
            hit = True
            what = 'env::temp_dir() — use fixture_root()'
        elif RS_BARE_TEMP.search(code):
            hit = True
            what = 'bare tempfile allocation — use tempdir_in(fixture area)'
        elif '.tempdir' in code and '.tempdir_in' not in code and i - pending_builder <= 4:
            hit = True
            what = 'bare tempfile allocation — use tempdir_in(fixture area)'
        if 'Builder::new' in code:
            pending_builder = i
        if hit:
            if i in exempt:
                review.append((i, 'exempt (documented)'))
            elif RS_LEGIT_TMP.search(code):
                review.append((i, 'governed area (CARGO_TARGET_TMPDIR / pinned TMPDIR)'))
            else:
                violations.append((i, what))
    return violations, review


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
    roots = [pathlib.Path(a) for a in args] or [
        REPO / 'tests', REPO / 'tools', REPO / 'web', REPO / 'rust']
    result = {'violations': {}, 'reviews': {}}
    fail = 0
    for path in iter_files(roots, {'.py', '.cjs', '.js', '.rs'}):
        text = path.read_text(errors='replace')
        if path.suffix == '.py':
            alloc, review = scan_python(path, text)
        elif path.suffix == '.rs':
            alloc, review = scan_rust(path, str(path.relative_to(REPO)))
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
