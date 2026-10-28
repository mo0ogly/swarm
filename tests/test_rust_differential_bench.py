"""Targeted differential-runner guards using owned SQLite fixtures and inert doubles.

These checks exercise the bench contract, not Go/Rust product equivalence. Each
refusal follows a valid comparison or fixture control; no provider is contacted.
"""
import copy
from contextlib import closing
import importlib.util
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location("differential_bench", Path(__file__).resolve().parents[1] / "tools/rust-migration/differential_bench.py")
BENCH = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BENCH)
CORPUS = json.loads((Path(__file__).resolve().parents[1] / "tools/rust-migration/differential-corpus.json").read_text())


class DifferentialBenchTests(unittest.TestCase):
    """Verify type fidelity, allowance scope and full invocation failure retention."""

    def test_primary_and_secondary_diagnostics_keep_safe_codes(self):
        """Known numeric causes survive every grouped branch without raw messages."""
        primary = OSError(13, "PRIVATE primary")
        cleanup = OSError(28, "PRIVATE cleanup")
        nodes = BENCH.safe_diagnostics(ExceptionGroup("PRIVATE group", [primary, cleanup]))
        self.assertEqual({node["os_errno"] for node in nodes if node["os_errno"] is not None}, {13, 28})
        self.assertNotIn("PRIVATE", json.dumps(nodes))

    def test_matching_empty_or_wrong_key_outputs_fail_witness(self):
        """Differential equality alone cannot establish a successful Work read."""
        self.assertTrue(BENCH.response_witness([{"id": "w"}], ["work", "list"], "w"))
        for value in ([], {}, [{"id": "other"}], [{"wrong": "w"}], [{"id": "w"}, {"id": "w"}]):
            self.assertFalse(BENCH.response_witness(value, ["work", "list"], "w"))
        self.assertTrue(BENCH.response_witness({"work": {"id": "w"}}, ["work", "show", "w"], "w"))
        self.assertFalse(BENCH.response_witness({"work": {}}, ["work", "show", "w"], "w"))

    def test_binary_changed_during_run_refuses_success(self):
        """Final hash validation rejects a changed executable despite equal outputs."""
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            control = Path(directory) / "control"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, control, "w"), 0)
            self.assertTrue(json.loads((control / "result.json").read_text())["binary_stability"])
            script = binaries[1].read_text()
            binaries[1].write_text(script + "from pathlib import Path\np=Path(__file__)\np.write_text(p.read_text()+'#mutated\\n')\n")
            output = Path(directory) / "changed"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, output, "w"), 1)
            result = json.loads((output / "result.json").read_text())
            self.assertEqual(result["diagnostic"][0]["code"], "binary-changed-during-run")
            self.assertFalse(result["binary_stability"])
            self.assertEqual(len(result["scenarios"]), 2)
            self.assertTrue(all(all(case["assertions"].values()) for case in result["scenarios"]))

    def test_json_types_and_array_order(self):
        control = {"x": [True, 1, 1.0, None]}
        self.assertEqual(BENCH.differences(control, copy.deepcopy(control)), [])
        self.assertEqual(BENCH.differences(True, 1), [""])
        self.assertEqual(BENCH.differences(1, 1.0), [""])
        self.assertEqual(BENCH.differences([1, 2], [2, 1]), ["/0", "/1"])
        self.assertEqual(BENCH.differences({}, {"x": None}), ["/x"])

    def test_allowance_exact_leaf_with_expected_values(self):
        allowed = [{"pointer": "/a~1b~0c", "go": "old", "rust": "new", "reason": "explicit fixture representation"}]
        self.assertEqual(BENCH.compare_json({"a/b~c": "old"}, {"a/b~c": "new"}, allowed), [])
        self.assertEqual(BENCH.compare_json({"x": 1}, {"x": 2}, []), ["/x"])
        with self.assertRaisesRegex(BENCH.BenchError, "allowance-value-mismatch"):
            BENCH.compare_json({"a/b~c": "unexpected"}, {"a/b~c": "new"}, allowed)
        self.assertEqual(BENCH.compare_json({"a/b~c": "old"}, {"a/b~c": "new"}, allowed), [])

    def test_allowance_cannot_hide_subtree_absence_or_unused_change(self):
        self.assertEqual(BENCH.compare_json({"x": 1}, {"x": 1}, []), [])
        for left, right, pointer, expected in [({"x": {}}, {"x": {"bad": 1}}, "/x", "subtree"),
                                               ({}, {"x": 2}, "/x", "missing"),
                                               ({"x": 1}, {"x": 1}, "/x", "unused")]:
            with self.subTest(expected=expected), self.assertRaisesRegex(BENCH.BenchError, expected):
                BENCH.compare_json(left, right, [{"pointer": pointer, "go": 1, "rust": 2, "reason": "test"}])

    def test_json_duplicate_nan_and_invalid_utf8_refused(self):
        self.assertEqual(BENCH.strict_json(b'{"value":1}'), {"value": 1})
        for raw in (b'{"value":1,"value":2}', b'NaN', b'Infinity', b'1e999', b'\xff'):
            with self.subTest(raw=raw), self.assertRaises(BENCH.BenchError):
                BENCH.strict_json(raw)

    def test_manifest_rejects_mutation_and_silent_defaults(self):
        BENCH.validate_corpus(CORPUS)
        for key, value in (("argv", ["work", "create"]), ("readonly", False),
                           ("expected_exit", True), ("database_tables", [])):
            changed = copy.deepcopy(CORPUS)
            changed["scenarios"][0][key] = value
            with self.subTest(key=key), self.assertRaises(BENCH.BenchError):
                BENCH.validate_corpus(changed)
        BENCH.validate_corpus(CORPUS)

    def fixtures(self, directory):
        """Create two equivalent empty private databases, marker and inert executables."""
        base = Path(directory)
        roots = [base / "go-root", base / "rust-root"]
        for root in roots:
            (root / ".swarm").mkdir(parents=True)
            (root / ".swarm/differential-fixture.json").write_text(json.dumps({"schema_version": 1, "kind": "owned-differential-fixture"}))
            with closing(sqlite3.connect(root / ".swarm/state.db")) as connection, connection:
                connection.execute("CREATE TABLE works(id TEXT PRIMARY KEY, revision INTEGER NOT NULL, body BLOB NOT NULL)")
                connection.execute("INSERT INTO works VALUES(?,?,?)", ("w", 1, b'{"id":"w"}'))
        binaries = [base / "go-double", base / "rust-double"]
        for binary in binaries:
            binary.write_text("#!/usr/bin/env python3\nimport json,sys\nprint(json.dumps([{'id':'w'}] if sys.argv[2]=='list' else {'work':{'id':'w'}}))\n")
            binary.chmod(0o700)
        return roots, binaries

    def test_real_double_invocation_and_private_retained_streams(self):
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            output = Path(directory) / "evidence"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, output, "w"), 0)
            result = json.loads((output / "result.json").read_text())
            self.assertEqual(len(result["scenarios"]), 2)
            self.assertTrue(all(all(row["assertions"].values()) for row in result["scenarios"]))
            self.assertEqual((output / "work-show-go.stdout").stat().st_mode & 0o777, 0o600)
            command = json.loads((output / "work-show-go.command.json").read_text())["argv"]
            self.assertEqual(command[1:], ["work", "show", "w", "--root", str(roots[0]), "--json"])
            with self.assertRaises(FileExistsError):
                BENCH.run(*binaries, *roots, CORPUS, output, "w")

    def test_same_root_hardlinked_database_and_unmarked_roots_refused(self):
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            self.assertEqual(BENCH.logical_state(roots[0]), BENCH.logical_state(roots[1]))
            for pair in ((roots[0], roots[0]),):
                with self.assertRaisesRegex(BENCH.BenchError, "distinct-nonnested"):
                    BENCH.run(*binaries, *pair, CORPUS, Path(directory) / "same", "w")
            database = roots[1] / ".swarm/state.db"
            database.unlink()
            database.hardlink_to(roots[0] / ".swarm/state.db")
            with self.assertRaisesRegex(BENCH.BenchError, "distinct-databases"):
                BENCH.run(*binaries, *roots, CORPUS, Path(directory) / "linked", "w")
            (roots[0] / ".swarm/differential-fixture.json").unlink()
            with self.assertRaises(FileNotFoundError):
                BENCH.fixture_root(roots[0])

    def test_non_equivalent_baseline_runs_neither_binary(self):
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            self.assertEqual(BENCH.logical_state(roots[0]), BENCH.logical_state(roots[1]))
            with closing(sqlite3.connect(roots[1] / ".swarm/state.db")) as connection, connection:
                connection.execute("UPDATE works SET revision=2")
            output = Path(directory) / "evidence"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, output, "w"), 1)
            self.assertEqual(json.loads((output / "result.json").read_text())["diagnostic"][0]["code"], "fixtures-not-equivalent")
            self.assertEqual(list(output.glob("*.command.json")), [])

    def test_failed_exit_and_invalid_json_keep_original_outputs(self):
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, Path(directory) / "control", "w"), 0)
            binaries[1].write_text("#!/usr/bin/env python3\nimport sys\nprint('invalid original')\nprint('PRIVATE child cause',file=sys.stderr)\nsys.exit(17)\n")
            output = Path(directory) / "failure"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, output, "w"), 1)
            self.assertEqual((output / "work-list-rust.stdout").read_text(), "invalid original\n")
            self.assertIn("PRIVATE child cause", (output / "work-list-rust.stderr").read_text())
            self.assertEqual(json.loads((output / "work-list-rust.exit.json").read_text())["exit"], 17)
            self.assertNotIn("PRIVATE", (output / "result.json").read_text())

    def test_read_mutation_fails_even_when_outputs_and_final_databases_match(self):
        with tempfile.TemporaryDirectory() as directory:
            roots, binaries = self.fixtures(directory)
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, Path(directory) / "control", "w"), 0)
            for binary in binaries:
                binary.write_text("#!/usr/bin/env python3\nimport sys,sqlite3\nfrom pathlib import Path\nroot=Path(sys.argv[sys.argv.index('--root')+1])\ndb=sqlite3.connect(root/'.swarm/state.db')\ndb.execute('UPDATE works SET revision=revision+1')\ndb.commit()\ndb.close()\nprint('{}')\n")
            output = Path(directory) / "mutation"
            self.assertEqual(BENCH.run(*binaries, *roots, CORPUS, output, "w"), 1)
            checks = json.loads((output / "result.json").read_text())["scenarios"][0]["assertions"]
            self.assertTrue(checks["database"])
            self.assertFalse(checks["readonly"])

    def test_sqlite_cell_types_bytes_and_duplicate_rows_are_not_collapsed(self):
        self.assertNotEqual(BENCH.cell(1), BENCH.cell(1.0))
        self.assertNotEqual(BENCH.cell("x"), BENCH.cell(b"x"))
        with tempfile.TemporaryDirectory() as directory:
            roots, _ = self.fixtures(directory)
            before = BENCH.logical_state(roots[0])
            self.assertEqual(len(before["rows"]), 1)
            with closing(sqlite3.connect(roots[0] / ".swarm/state.db")) as connection, connection:
                connection.execute("UPDATE works SET body=?", (b'changed\x00',))
            self.assertNotEqual(before, BENCH.logical_state(roots[0]))


if __name__ == "__main__":
    unittest.main(verbosity=2)
