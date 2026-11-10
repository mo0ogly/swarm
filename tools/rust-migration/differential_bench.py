#!/usr/bin/env python3
"""Compare explicitly supplied Go/Rust fixture binaries; never prepare/live-migrate Roots.

Only the audited read-only work list/show corpus is currently supported. Raw child
outputs are retained in a fresh private evidence directory and may contain fixture
data: do not publish them as sanitized diagnostics. No global watchdog is imposed.
"""
from __future__ import annotations

import argparse
import base64
import hashlib
import json
import math
import os
from pathlib import Path
import sqlite3
import subprocess
import sys


class BenchError(Exception):
    """A safe static refusal code; raw child failures remain in private evidence."""


def exact_keys(value, keys):
    """Require an object with exactly the declared contract keys, without defaults."""
    if not isinstance(value, dict) or set(value) != set(keys):
        raise BenchError("invalid-object-fields")


def strict_json(raw):
    """Decode UTF-8 JSON, refusing duplicate keys and nonstandard NaN/Infinity."""
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise BenchError("duplicate-json-key")
            result[key] = value
        return result
    def constant(_):
        raise BenchError("nonfinite-json-number")
    def floating(raw_number):
        value = float(raw_number)
        if not math.isfinite(value):
            raise BenchError("nonfinite-json-number")
        return value
    try:
        return json.loads(raw, object_pairs_hook=pairs, parse_constant=constant, parse_float=floating)
    except (ValueError, UnicodeError) as error:
        raise BenchError("invalid-json") from error


def differences(left, right, pointer=""):
    """Return exact RFC6901 paths; preserve types, array order and absent/null meaning."""
    if type(left) is not type(right):
        return [pointer]
    if isinstance(left, dict):
        found = []
        for key in sorted(set(left) | set(right)):
            child = pointer + "/" + key.replace("~", "~0").replace("/", "~1")
            found.extend([child] if key not in left or key not in right
                         else differences(left[key], right[key], child))
        return found
    if isinstance(left, list):
        if len(left) != len(right):
            return [pointer]
        return [item for index, pair in enumerate(zip(left, right))
                for item in differences(*pair, pointer + "/" + str(index))]
    return [] if left == right else [pointer]


def at_pointer(document, pointer):
    """Resolve one existing JSON pointer; an absent allowance target is refused."""
    if pointer == "":
        return document
    if not isinstance(pointer, str) or not pointer.startswith("/"):
        raise BenchError("invalid-json-pointer")
    value = document
    try:
        for token in pointer[1:].split("/"):
            if "~" in token.replace("~0", "").replace("~1", ""):
                raise BenchError("invalid-json-pointer")
            token = token.replace("~1", "/").replace("~0", "~")
            if isinstance(value, list):
                if not token.isdigit() or str(int(token)) != token:
                    raise BenchError("invalid-json-pointer")
                value = value[int(token)]
            elif isinstance(value, dict):
                value = value[token]
            else:
                raise BenchError("missing-allowance-target")
        return value
    except (KeyError, IndexError) as error:
        raise BenchError("missing-allowance-target") from error


def compare_json(left, right, allowances):
    """Accept only declared leaf differences with exact expected values and a reason.

    No wildcard, subtree deletion, absent target or unused allowance is accepted.
    Returns remaining difference paths without copying private differing values.
    """
    found = set(differences(left, right))
    used = set()
    for allowance in allowances:
        exact_keys(allowance, ("pointer", "go", "rust", "reason"))
        pointer = allowance["pointer"]
        if not isinstance(pointer, str) or pointer in used:
            raise BenchError("duplicate-or-invalid-allowance")
        if not isinstance(allowance["reason"], str) or not allowance["reason"].strip():
            raise BenchError("allowance-reason-required")
        go_value, rust_value = at_pointer(left, pointer), at_pointer(right, pointer)
        if isinstance(go_value, (dict, list)) or isinstance(rust_value, (dict, list)):
            raise BenchError("subtree-allowance-forbidden")
        if pointer not in found:
            raise BenchError("unused-allowance")
        if differences(go_value, allowance["go"]) or differences(rust_value, allowance["rust"]):
            raise BenchError("allowance-value-mismatch")
        used.add(pointer)
    return sorted(found - used)


def validate_corpus(corpus):
    """Validate the audited read-only command grammar before either binary runs."""
    exact_keys(corpus, ("schema_version", "scenarios"))
    if type(corpus["schema_version"]) is not int or corpus["schema_version"] != 1:
        raise BenchError("unsupported-corpus-version")
    if not isinstance(corpus["scenarios"], list) or not corpus["scenarios"]:
        raise BenchError("empty-corpus")
    seen = set()
    for case in corpus["scenarios"]:
        exact_keys(case, ("id", "argv", "expected_exit", "json_differences", "database_tables", "readonly"))
        name = case["id"]
        if not isinstance(name, str) or not name or any(c not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for c in name) or name in seen:
            raise BenchError("invalid-scenario-id")
        seen.add(name)
        if case["argv"] not in (["work", "list"], ["work", "show", "{work_id}"]):
            raise BenchError("unaudited-command")
        if type(case["expected_exit"]) is not int or case["expected_exit"] != 0:
            raise BenchError("unaudited-exit-contract")
        if case["database_tables"] != ["works"] or case["readonly"] is not True:
            raise BenchError("unaudited-database-contract")
        if not isinstance(case["json_differences"], list):
            raise BenchError("invalid-allowances")
        for allowance in case["json_differences"]:
            exact_keys(allowance, ("pointer", "go", "rust", "reason"))


def fixture_root(path):
    """Require an explicit existing owned fixture Root and marker; never initialize.

    A marker records the operator's fixture designation, not OS confinement. Both
    binary and Root trust remain the caller's responsibility; live Roots are banned.
    """
    supplied = Path(path)
    root = supplied.resolve(strict=True)
    marker = root / ".swarm/differential-fixture.json"
    database = root / ".swarm/state.db"
    if supplied.is_symlink() or (root / ".swarm").is_symlink() or marker.is_symlink() or database.is_symlink():
        raise BenchError("fixture-symlink-refused")
    if root.stat().st_uid != os.getuid() or not database.is_file():
        raise BenchError("fixture-ownership-or-database-required")
    value = strict_json(marker.read_bytes())
    exact_keys(value, ("schema_version", "kind"))
    if type(value["schema_version"]) is not int or value != {"schema_version": 1, "kind": "owned-differential-fixture"}:
        raise BenchError("fixture-marker-required")
    return root


def cell(value):
    """Preserve SQLite cell type, integer/float distinctions and complete BLOB bytes."""
    if value is None:
        return ["null", None]
    if type(value) is int:
        return ["integer", value]
    if type(value) is float:
        return ["real", value.hex()]
    if isinstance(value, bytes):
        return ["blob", base64.b64encode(value).decode("ascii")]
    return ["text", value]


def safe_diagnostics(error):
    """Preserve all error branches as safe types/codes, without exception messages."""
    result, pending, seen = [], [("primary", error)], set()
    while pending:
        path, current = pending.pop()
        if id(current) in seen:
            continue
        seen.add(id(current))
        result.append({"path": path, "code": str(current) if isinstance(current, BenchError) else type(current).__name__,
                       "os_errno": getattr(current, "errno", None),
                       "sqlite_code": getattr(current, "sqlite_errorcode", None),
                       "sqlite_name": getattr(current, "sqlite_errorname", None)})
        if isinstance(current, BaseExceptionGroup):
            pending.extend((path + "/branch/" + str(index), child) for index, child in enumerate(current.exceptions))
        cause = current.__cause__ or current.__context__
        if cause is not None:
            pending.append((path + "/cause", cause))
    return result


def logical_state(root):
    """Read all audited works columns/rows coherently through SQLite read-only mode.

    Schema, row multiplicity and cell bytes are compared; physical SQLite layout is
    intentionally excluded. Missing tables/locking/read errors fail, never empty-PASS.
    This observes works only, not every table or filesystem/process effects. Primary
    and close failures both survive when cleanup also fails.
    """
    connection = sqlite3.connect((root / ".swarm/state.db").as_uri() + "?mode=ro", uri=True, timeout=0)
    primary = None
    try:
        connection.execute("BEGIN")
        schema = connection.execute("PRAGMA table_info(works)").fetchall()
        if [row[1] for row in schema] != ["id", "revision", "body"]:
            raise BenchError("unexpected-works-schema")
        rows = [[cell(value) for value in row] for row in connection.execute("SELECT id,revision,body FROM works")]
        state = {"schema": schema, "rows": sorted(rows, key=lambda row: json.dumps(row, ensure_ascii=True))}
    except Exception as error:
        primary = error
    try:
        connection.close()
    except Exception as cleanup:
        if primary is not None:
            raise ExceptionGroup("database-read-and-close", [primary, cleanup]) from None
        raise
    if primary is not None:
        raise primary
    return state


def save(path, value):
    """Create private evidence exactly once; write failures propagate to the caller."""
    with path.open("xb") as stream:
        stream.write(value if isinstance(value, bytes) else json.dumps(value, ensure_ascii=True, indent=2).encode() + b"\n")
        stream.flush()
        os.fsync(stream.fileno())
    path.chmod(0o600)


def invoke(binary, root, argv, prefix):
    """Execute one direct argv without shell/watchdog; retain both raw streams first.

    No stdin/provider configuration is supplied. Exceptions retain previously saved
    command evidence. Partial filesystem effects remain unknown after spawn failure.
    """
    command = [str(binary), *argv, "--root", str(root), "--json"]
    save(prefix.with_suffix(".command.json"), {"argv": command})
    result = subprocess.run(command, stdin=subprocess.DEVNULL, capture_output=True, check=False)
    save(prefix.with_suffix(".stdout"), result.stdout)
    save(prefix.with_suffix(".stderr"), result.stderr)
    save(prefix.with_suffix(".exit.json"), {"exit": result.returncode})
    return result


def response_witness(value, argv, work_id):
    """Reject matching empty/wrong-shape successes: the supplied Work must be present."""
    if argv == ["work", "list"]:
        return (isinstance(value, list) and
                all(isinstance(work, dict) and isinstance(work.get("id"), str) for work in value) and
                sum(work["id"] == work_id for work in value) == 1)
    return (isinstance(value, dict) and isinstance(value.get("work"), dict) and
            value["work"].get("id") == work_id)


def run(go_binary, rust_binary, go_root, rust_root, corpus, output, work_id):
    """Run audited scenarios on separate caller-owned Roots; return 0 only on parity.

    A fresh private output is mandatory. Baseline inequality refuses execution.
    Per-command exit, JSON, stderr and works state are compared, including refusal
    of mutation for these reads. This is infrastructure evidence, not acceptance,
    production isolation or parity for commands absent from the corpus. Binary
    hashes must match before/after success; this is observed stability, not atomic
    exec binding or source/profile qualification.
    """
    validate_corpus(corpus)
    roots = [fixture_root(go_root), fixture_root(rust_root)]
    if roots[0] == roots[1] or roots[0] in roots[1].parents or roots[1] in roots[0].parents:
        raise BenchError("distinct-nonnested-fixtures-required")
    if os.path.samefile(roots[0] / ".swarm/state.db", roots[1] / ".swarm/state.db"):
        raise BenchError("distinct-databases-required")
    if not isinstance(work_id, str) or not work_id or work_id.startswith("-") or "\x00" in work_id:
        raise BenchError("invalid-work-id")
    binaries = [Path(go_binary).resolve(strict=True), Path(rust_binary).resolve(strict=True)]
    for binary in binaries:
        if not binary.is_file() or not os.access(binary, os.X_OK):
            raise BenchError("executable-required")
    output = Path(output).absolute()
    if any(output == root or root in output.parents for root in roots):
        raise BenchError("evidence-outside-fixtures-required")
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    binary_inputs = [{"path": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()} for binary in binaries]
    save(output / "inputs.json", {"corpus": corpus, "work_id": work_id, "roots": [str(root) for root in roots],
         "binaries": binary_inputs})
    report = {"status": "FAIL", "scope": "work-list-show-json-and-works-table", "scenarios": [], "diagnostic": None}
    try:
        before = [logical_state(root) for root in roots]
        save(output / "baseline.json", before)
        if before[0] != before[1]:
            raise BenchError("fixtures-not-equivalent")
        for case in corpus["scenarios"]:
            argv = [work_id if value == "{work_id}" else value for value in case["argv"]]
            prior = [logical_state(root) for root in roots]
            results = [invoke(binary, root, argv, output / (case["id"] + "-" + label))
                       for binary, root, label in zip(binaries, roots, ("go", "rust"))]
            after = [logical_state(root) for root in roots]
            save(output / (case["id"] + "-state.json"), {"before": prior, "after": after})
            left, right = [strict_json(result.stdout) for result in results]
            delta = compare_json(left, right, case["json_differences"])
            assertions = {"exit": all(result.returncode == case["expected_exit"] for result in results),
                          "witness": all(response_witness(value, argv, work_id) for value in (left, right)),
                          "json": not delta, "stderr": results[0].stderr == results[1].stderr,
                          "database": after[0] == after[1], "readonly": prior == after}
            report["scenarios"].append({"id": case["id"], "assertions": assertions, "json_difference_paths": delta})
            if not all(assertions.values()):
                raise BenchError("scenario-mismatch")
        binary_after = [{"path": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()} for binary in binaries]
        save(output / "binaries-after.json", binary_after)
        report["binary_stability"] = binary_inputs == binary_after
        if not report["binary_stability"]:
            raise BenchError("binary-changed-during-run")
        report["status"] = "PASS"
    except Exception as error:
        report["diagnostic"] = safe_diagnostics(error)
    finally:
        save(output / "result.json", report)
    return 0 if report["status"] == "PASS" else 1


def main():
    """Parse explicit inputs; print only safe outcome codes and return nonzero on errors."""
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("go-binary", "rust-binary", "go-root", "rust-root", "corpus", "output", "work-id"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    try:
        return run(args.go_binary, args.rust_binary, args.go_root, args.rust_root,
                   strict_json(Path(args.corpus).read_bytes()), args.output, args.work_id)
    except Exception as error:
        print("differential-bench: " + json.dumps(safe_diagnostics(error)), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
