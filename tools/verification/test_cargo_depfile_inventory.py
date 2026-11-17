"""Focused unittest cases for cargo_depfile_inventory.py.

The sibling production module is loaded by path with importlib and exercised
through its named functions and CLI entry point.  All fixtures are synthetic
trees built inside caller-selected temporary directories: the ``TMPDIR``
environment variable is honoured when the caller exports it, otherwise the
platform default of :mod:`tempfile` is used.  No subprocess is imported, no
real executable is produced or run, and no project snapshot or historical
campaign data is read; every Cargo stdout event and every tree is synthesized
here from the observed JSON shapes only.
"""

import hashlib
import contextlib
import io
from unittest.mock import patch
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path

_MODULE_PATH = Path(__file__).resolve().with_name("cargo_depfile_inventory.py")
_SPEC = importlib.util.spec_from_file_location(
    "cargo_depfile_inventory_under_test", _MODULE_PATH)
MOD = importlib.util.module_from_spec(_SPEC)
_SPEC.loader.exec_module(MOD)


def _tmpdir_kwargs():
    """Honour a caller-selected TMPDIR; never invent a machine-specific path."""
    selected = os.environ.get("TMPDIR")
    return {"dir": selected} if selected else {}


class InventoryTestCase(unittest.TestCase):
    """Shared fixture helpers for the inventory tool tests."""

    def tempdir(self):
        """Own one synthetic temporary tree under caller TMPDIR; cleanup errors propagate."""
        return tempfile.TemporaryDirectory(**_tmpdir_kwargs())

    def build_tree(self, root):
        """Create workspace (with ``rust`` manifest root), target and registry."""
        root = Path(root)
        workspace = root / "ws"
        manifest_root = workspace / "rust"
        (manifest_root / "src").mkdir(parents=True)
        target = root / "target"
        (target / "deps").mkdir(parents=True)
        (target / "build").mkdir(parents=True)
        registry = root / "registry" / "src" / "example-index"
        registry.mkdir(parents=True)
        return workspace, manifest_root, target, registry

    @staticmethod
    def write_stdout(path, events):
        """Serialize synthetic Cargo events into the owned fixture; never call Cargo."""
        lines = "".join(json.dumps(event) + "\n" for event in events)
        Path(path).write_text(lines, encoding="utf-8")

    @staticmethod
    def write_depfile(path, rule_target, prereqs):
        """Write a depfile rule line with the observed escaping convention."""
        def escape(token):
            """Escape synthetic Make tokens, without shell evaluation."""
            return (token.replace("\\", "\\\\")
                         .replace(" ", "\\ ")
                         .replace("$", "\\$"))
        body = rule_target + ": " + " ".join(escape(t) for t in prereqs) + "\n"
        Path(path).write_text(body, encoding="utf-8")

    @staticmethod
    def lib_event(manifest, src, artifact, name="cfg_one", fresh=False):
        """Construct a fictional library event; this grants no provenance or execution."""
        return {
            "reason": "compiler-artifact",
            "package_id": "registry+https://example.invalid/index#" + name + "@1.0.0",
            "manifest_path": str(manifest),
            "target": {"kind": ["lib"], "crate_types": ["lib"], "name": name,
                       "src_path": str(src), "edition": "2021", "doc": True,
                       "doctest": True, "test": True},
            "profile": {"opt_level": "0", "debuginfo": 0,
                        "debug_assertions": True, "overflow_checks": True,
                        "test": False},
            "features": [],
            "filenames": [str(artifact)],
            "executable": None,
            "fresh": fresh,
        }

    @classmethod
    def proc_macro_event(cls, manifest, src, so, name="my_derive"):
        """Construct macro metadata without loading or executing a macro."""
        event = cls.lib_event(manifest, src, so, name=name)
        event["target"]["kind"] = ["proc-macro"]
        event["target"]["crate_types"] = ["proc-macro"]
        return event

    @classmethod
    def executable_event(cls, manifest, src, executable, name="engine_bin"):
        """Construct test-image metadata; the fake image is never executed."""
        event = cls.lib_event(manifest, src, executable, name=name)
        event["target"]["kind"] = ["bin"]
        event["target"]["crate_types"] = ["bin"]
        event["profile"]["test"] = True
        event["executable"] = str(executable)
        return event

    @classmethod
    def custom_build_event(cls, manifest, src, emitted):
        """Construct build-script metadata without executing a build script."""
        event = cls.lib_event(manifest, src, emitted, name="build-script-build")
        event["target"]["kind"] = ["custom-build"]
        event["target"]["crate_types"] = ["bin"]
        return event

    def run_tool(self, workspace, target, registry, stdout, output):
        """Invoke the pure Python CLI over synthetic inputs and return its real status."""
        return MOD.main([
            "--cargo-stdout", str(stdout),
            "--workspace", str(workspace),
            "--target", str(target),
            "--registry-root", str(registry),
            "--output", str(output),
        ])

    @staticmethod
    def load_report(path):
        """Read a retained fixture JSON report; IO/JSON errors propagate."""
        with open(path, encoding="utf-8") as handle:
            return json.load(handle)

    def healthy_fixture(self, root, fresh=False):
        """One valid library artifact plus a healthy depfile and sources."""
        root = Path(root)
        workspace, manifest_root, target, registry = self.build_tree(root)
        crate = registry / "cfg-one-1.0.4"
        (crate / "src").mkdir(parents=True)
        src = crate / "src" / "lib.rs"
        src.write_bytes(b"pub fn one() {}\n")
        manifest = crate / "Cargo.toml"
        manifest.write_bytes(b"[package]\nname = \"cfg-one\"\n")
        rlib = target / "deps" / "libcfg_one-b9f863d2a7a34542.rlib"
        rlib.write_bytes(b"fixture-rlib-bytes")
        depfile = target / "deps" / "cfg_one-b9f863d2a7a34542.d"
        self.write_depfile(depfile, str(depfile), [str(src)])
        stdout = root / "cargo.json"
        self.write_stdout(stdout, [self.lib_event(manifest, src, rlib, fresh=fresh),
                                   {"reason": "build-finished", "success": True}])
        return workspace, target, registry, stdout, src

    def custom_fixture(self, root):
        """Directories and paths for a custom-build (build script) artifact."""
        root = Path(root)
        workspace, manifest_root, target, registry = self.build_tree(root)
        crate = registry / "serde-1.0.228"
        (crate / "src").mkdir(parents=True)
        src = crate / "src" / "lib.rs"
        src.write_bytes(b"fixture\n")
        manifest = crate / "Cargo.toml"
        manifest.write_bytes(b"[package]\n")
        build_dir = target / "build" / "serde-81b1bd61f2c99ad7"
        build_dir.mkdir(parents=True)
        emitted = build_dir / "build-script-build"
        depfile = build_dir / "build_script_build-81b1bd61f2c99ad7.d"
        stdout = root / "cargo.json"
        return (workspace, target, registry, stdout, manifest, src,
                build_dir, emitted, depfile)

    def write_custom_stdout(self, stdout, manifest, src, emitted):
        """Serialize a fictional custom-build artifact and successful finish marker."""
        self.write_stdout(stdout, [self.custom_build_event(manifest, src, emitted),
                                   {"reason": "build-finished", "success": True}])

    def assert_unresolved(self, report, cause):
        """Require explicit refusal and verify any retained raw depfile hash."""
        self.assertEqual(len(report["artifacts"]), 1)
        artifact = report["artifacts"][0]
        self.assertTrue(artifact["unresolved"])
        self.assertEqual(artifact["cause"], cause)
        if artifact["depfile"] is not None:
            raw = artifact["depfile"]["content_text"].encode("utf-8")
            self.assertEqual(hashlib.sha256(raw).hexdigest(), artifact["depfile"]["sha256"])


class TestValidAssociations(InventoryTestCase):
    def test_library_rlib_depfile_sources_hashes_and_unqualified_report(self):
        """Library inventory preserves escaped paths and remains explicitly unqualified."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            crate = registry / "cfg-one-1.0.4"
            (crate / "src").mkdir(parents=True)
            reg_src = crate / "src" / "lib.rs"
            reg_src.write_bytes(b"pub fn one() {}\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            ws_src = manifest_root / "src" / "extra.rs"
            ws_src.write_bytes(b"pub fn extra() {}\n")
            spaced = manifest_root / "src" / "odd name.rs"
            spaced.write_bytes(b"pub fn odd() {}\n")
            rlib = target / "deps" / "libcfg_one-b9f863d2a7a34542.rlib"
            rlib.write_bytes(b"fixture-rlib-bytes")
            depfile = target / "deps" / "cfg_one-b9f863d2a7a34542.d"
            self.write_depfile(depfile, str(depfile),
                               [str(reg_src), "src/extra.rs", "src/odd name.rs"])
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [
                self.lib_event(manifest, reg_src, rlib, fresh=True),
                {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertFalse(out.exists())
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 0)
            report = self.load_report(out)
            self.assertEqual(len(report["artifacts"]), 1)
            art = report["artifacts"][0]
            self.assertEqual(art["artifact_kind"], "library-rlib")
            self.assertFalse(art["unresolved"])
            self.assertIsNone(art["cause"])
            self.assertTrue(art["fresh"])  # fresh artifact recovered like any other
            self.assertEqual(art["depfile"]["path"], str(depfile))
            self.assertEqual(art["depfile"]["sha256"],
                             hashlib.sha256(depfile.read_bytes()).hexdigest())
            self.assertEqual(art["depfile"]["content_text"],
                             depfile.read_text(encoding="utf-8"))
            resolved = [entry["resolved"] for entry in art["depfile_sources"]]
            self.assertIn(str(spaced), resolved)  # escaped space decoded, not dropped
            canon_reg = os.path.realpath(reg_src)
            self.assertEqual(art["sources"][0], canon_reg)
            self.assertEqual(len(art["sources"]), 3)
            sources = report["sources"]
            self.assertEqual(len(sources), 4)  # reg_src, ws_src, spaced, manifest
            self.assertEqual(sources[canon_reg]["classification"],
                             "caller-declared-registry")
            self.assertEqual(sources[canon_reg]["sha256"],
                             hashlib.sha256(reg_src.read_bytes()).hexdigest())
            self.assertEqual(sources[os.path.realpath(ws_src)]["classification"],
                             "workspace")
            self.assertEqual(sources[os.path.realpath(spaced)]["classification"],
                             "workspace")
            self.assertEqual(sources[os.path.realpath(manifest)]["classification"],
                             "caller-declared-registry")
            self.assertEqual(report["counts"]["artifacts_mapped"], 1)
            self.assertEqual(report["counts"]["artifacts_unresolved"], 0)
            self.assertEqual(report["counts"]["sources_with_issues"], 0)
            # Even a fully mapped run stays observational and unqualified.
            self.assertEqual(report["status"], "OBSERVED_POSTBUILD_ONLY")
            self.assertEqual(report["scope"], "postbuild-depfile-inventory-only")
            self.assertFalse(report["complete_build_qualification"])
            self.assertFalse(report["behavioral_admission"])
            self.assertTrue(all(report["unproved"].values()))
            self.assertEqual(set(report["unproved"]), {
                "input_bytes_at_compiler_read_time",
                "exact_compiler_linker_c_ancestor_config_environment",
                "generated_or_proc_macro_side_inputs_omitted_from_depfiles",
                "full_linkage_and_runtime_closure",
            })

    def test_proc_macro_normalized_depfile(self):
        """Associate a fake proc-macro image with its hashed depfile without execution."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            crate = registry / "my-derive-0.1.1"
            (crate / "src").mkdir(parents=True)
            src = crate / "src" / "lib.rs"
            src.write_bytes(b"proc macro fixture\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            so = target / "deps" / "libmy_derive-5641a9a3843a6264.so"
            so.write_bytes(b"fixture-so-bytes")
            depfile = target / "deps" / "my_derive-5641a9a3843a6264.d"
            self.write_depfile(depfile, str(depfile), [str(src)])
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [self.proc_macro_event(manifest, src, so),
                                       {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 0)
            art = self.load_report(out)["artifacts"][0]
            self.assertEqual(art["artifact_kind"], "proc-macro")
            self.assertFalse(art["unresolved"])
            self.assertEqual(art["depfile"]["path"], str(depfile))
            self.assertIsNone(art["depfile_owner"])

    def test_test_executable_adjacent_depfile(self):
        """Bind a fake test executable to its exact adjacent dependency file."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            crate = registry / "engine-0.1.0"
            (crate / "src").mkdir(parents=True)
            src = crate / "src" / "main.rs"
            src.write_bytes(b"fn main() {}\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            exe = target / "deps" / "engine_bin-2c5d1e0f9a8b7c6d"
            exe.write_bytes(b"fixture-bin-bytes")
            depfile = target / "deps" / "engine_bin-2c5d1e0f9a8b7c6d.d"
            self.write_depfile(depfile, str(depfile), [str(src)])
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [self.executable_event(manifest, src, exe),
                                       {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 0)
            art = self.load_report(out)["artifacts"][0]
            self.assertEqual(art["artifact_kind"], "test-executable")
            self.assertFalse(art["unresolved"])
            self.assertEqual(art["artifact_file"], str(exe))
            self.assertEqual(art["depfile"]["path"], str(depfile))

    def test_custom_build_inode_sibling_association(self):
        """Require the actual shared inode when associating a fake build script."""
        with self.tempdir() as root:
            root = Path(root)
            (workspace, target, registry, stdout, manifest, src,
             build_dir, emitted, depfile) = self.custom_fixture(root)
            sibling = build_dir / "build_script_build-81b1bd61f2c99ad7"
            sibling.write_bytes(b"fixture-build-script")
            os.link(sibling, emitted)  # shared st_dev + st_ino, as observed
            self.write_depfile(depfile, str(depfile), [str(src)])
            self.write_custom_stdout(stdout, manifest, src, emitted)
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 0)
            art = self.load_report(out)["artifacts"][0]
            self.assertEqual(art["artifact_kind"], "custom-build")
            self.assertFalse(art["unresolved"])
            self.assertEqual(art["artifact_file"], str(emitted))
            self.assertEqual(art["depfile_owner"], str(sibling))
            self.assertEqual(art["depfile"]["path"], str(depfile))

    def test_fresh_artifact_recovered(self):
        """A cached/fresh marker changes no provenance or qualification claim."""
        with self.tempdir() as root:
            workspace, target, registry, stdout, src = self.healthy_fixture(
                root, fresh=True)
            out = Path(root) / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 0)
            art = self.load_report(out)["artifacts"][0]
            self.assertTrue(art["fresh"])
            self.assertFalse(art["unresolved"])


class TestRefusals(InventoryTestCase):
    def setUp(self):
        """Valid association controls precede each negative fixture variant."""
        TestValidAssociations.test_library_rlib_depfile_sources_hashes_and_unqualified_report(self)
        TestValidAssociations.test_proc_macro_normalized_depfile(self)
        TestValidAssociations.test_test_executable_adjacent_depfile(self)
        TestValidAssociations.test_custom_build_inode_sibling_association(self)

    def tearDown(self):
        """Fresh association controls establish recovery after the variant."""
        self.setUp()

    def test_custom_build_sibling_missing_refused(self):
        """Missing matching inode refuses while preserving partial source context."""
        with self.tempdir() as root:
            root = Path(root)
            (workspace, target, registry, stdout, manifest, src,
             build_dir, emitted, depfile) = self.custom_fixture(root)
            emitted.write_bytes(b"fixture-build-script")
            self.write_depfile(depfile, str(depfile), [str(src)])
            self.write_custom_stdout(stdout, manifest, src, emitted)
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            report = self.load_report(out)
            self.assert_unresolved(report, "custom-build-sibling-missing-or-wrong-inode")
            self.assertTrue(report["sources"])  # partial observations preserved

    def test_custom_build_sibling_wrong_inode_refused(self):
        """A similarly named different inode cannot own the emitted build script."""
        with self.tempdir() as root:
            root = Path(root)
            (workspace, target, registry, stdout, manifest, src,
             build_dir, emitted, depfile) = self.custom_fixture(root)
            emitted.write_bytes(b"fixture-build-script")
            sibling = build_dir / "build_script_build-81b1bd61f2c99ad7"
            sibling.write_bytes(b"different-bytes-not-linked")  # distinct inode
            self.write_depfile(depfile, str(depfile), [str(src)])
            self.write_custom_stdout(stdout, manifest, src, emitted)
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out),
                                   "custom-build-sibling-missing-or-wrong-inode")

    def test_custom_build_sibling_ambiguous_refused(self):
        """Multiple matching inode aliases refuse without guessing one owner."""
        with self.tempdir() as root:
            root = Path(root)
            (workspace, target, registry, stdout, manifest, src,
             build_dir, emitted, depfile) = self.custom_fixture(root)
            emitted.write_bytes(b"fixture-build-script")
            os.link(emitted, build_dir / "build_script_build-1111111111111111")
            os.link(emitted, build_dir / "build_script_build-2222222222222222")
            self.write_depfile(depfile, str(depfile), [str(src)])
            self.write_custom_stdout(stdout, manifest, src, emitted)
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out),
                                   "custom-build-sibling-ambiguous")

    def test_unsupported_executable_kind_refused(self):
        """A non-test executable is outside the declared artifact format scope."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            crate = registry / "engine-0.1.0"
            (crate / "src").mkdir(parents=True)
            src = crate / "src" / "main.rs"
            src.write_bytes(b"fn main() {}\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            exe = target / "deps" / "engine_bin-2c5d1e0f9a8b7c6d"
            exe.write_bytes(b"fixture-bin-bytes")
            depfile = target / "deps" / "engine_bin-2c5d1e0f9a8b7c6d.d"
            self.write_depfile(depfile, str(depfile), [str(src)])
            event = self.executable_event(manifest, src, exe)
            event["profile"]["test"] = False  # non-test executable: out of scope
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [event,
                                       {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out), "unsupported-artifact-kind")

    def test_artifact_outside_target_refused(self):
        """Declared artifacts outside the admitted target are rejected."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            crate = registry / "cfg-one-1.0.4"
            (crate / "src").mkdir(parents=True)
            src = crate / "src" / "lib.rs"
            src.write_bytes(b"x\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            outside = root / "outside"
            outside.mkdir()
            rlib = outside / "libcfg_one-b9f863d2a7a34542.rlib"
            rlib.write_bytes(b"fixture-rlib-bytes")
            self.write_depfile(outside / "cfg_one-b9f863d2a7a34542.d",
                               str(outside / "cfg_one-b9f863d2a7a34542.d"), [str(src)])
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [self.lib_event(manifest, src, rlib),
                                       {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out), "artifact-outside-target")

    def test_depfile_symlink_refused(self):
        """A symlinked dependency file cannot substitute for its owned regular file."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            deps = target / "deps"
            expected = deps / "cfg_one-b9f863d2a7a34542.d"
            real = deps / "elsewhere.d"
            real.write_text(str(real) + ": " + str(src) + "\n", encoding="utf-8")
            expected.unlink()
            os.symlink(str(real), str(expected))
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out), "depfile-symlink")

    def test_depfile_continuation_refused(self):
        """Unsupported continuation retains read bytes/hash and an explicit refusal."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            depfile = target / "deps" / "cfg_one-b9f863d2a7a34542.d"
            depfile.write_text(str(depfile) + ": \\\n    " + str(src) + "\n",
                               encoding="utf-8")
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out),
                                   "depfile-continuation-unsupported")
            observed = self.load_report(out)["artifacts"][0]["depfile"]
            self.assertIsNotNone(observed)
            self.assertEqual(observed["content_text"], depfile.read_text())

    def test_consistent_rust_rules_and_phony_entries_are_preserved(self):
        """Repeated Rust rules and phony entries preserve the dependency observations."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            depfile = target / "deps" / "cfg_one-b9f863d2a7a34542.d"
            line = str(depfile) + ": " + str(src) + "\n"
            depfile.write_text(line + line + str(src) + ":\n# env-dep:EXAMPLE=fixture\n", encoding="utf-8")
            output = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, output), 0)
            report = self.load_report(output)
            self.assertEqual(report["artifacts"][0]["depfile"]["content_text"], depfile.read_text())
            self.assertFalse(report["complete_build_qualification"])

    def test_unknown_artifact_field_refused(self):
        """Unknown consumed event schema is refused without empty success."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            events = [json.loads(line)
                      for line in Path(stdout).read_text(encoding="utf-8").splitlines()
                      if line.strip()]
            events[0]["unexpected_field"] = True
            self.write_stdout(stdout, events)
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            self.assert_unresolved(self.load_report(out), "cargo-json-unknown-field")

    def test_missing_source_reported_artifact_still_mapped(self):
        """A mapped artifact does not hide missing source observations."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            depfile = target / "deps" / "cfg_one-b9f863d2a7a34542.d"
            ghost = root / "ghost.rs"
            self.write_depfile(depfile, str(depfile), [str(ghost)])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            report = self.load_report(out)
            art = report["artifacts"][0]
            self.assertFalse(art["unresolved"])  # depfile association still valid
            canon_ghost = os.path.realpath(str(ghost))
            self.assertEqual(report["sources"][canon_ghost]["issue"], "source-missing")
            self.assertIsNone(report["sources"][canon_ghost]["sha256"])
            self.assertEqual(report["counts"]["sources_with_issues"], 1)

    def test_source_classification_conflict_reported(self):
        """Alias classifications cannot silently overwrite conflicting observations."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, manifest_root, target, registry = self.build_tree(root)
            twin = manifest_root / "src" / "twin.rs"
            twin.write_bytes(b"twin\n")
            crate = registry / "alias-crate-0.1.0"
            (crate / "src").mkdir(parents=True)
            src = crate / "src" / "lib.rs"
            src.write_bytes(b"x\n")
            manifest = crate / "Cargo.toml"
            manifest.write_bytes(b"[package]\n")
            alias_dir = root / "alias"
            os.symlink(str(workspace), str(alias_dir))
            alias_token = str(alias_dir / "rust" / "src" / "twin.rs")
            rlib = target / "deps" / "libalias_crate-1111111111111111.rlib"
            rlib.write_bytes(b"fixture-rlib-bytes")
            depfile = target / "deps" / "alias_crate-1111111111111111.d"
            self.write_depfile(depfile, str(depfile), [str(twin), alias_token])
            stdout = root / "cargo.json"
            self.write_stdout(stdout, [self.lib_event(manifest, src, rlib,
                                                      name="alias_crate"),
                                       {"reason": "build-finished", "success": True}])
            out = root / "report.json"
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 1)
            report = self.load_report(out)
            self.assertFalse(report["artifacts"][0]["unresolved"])
            entry = report["sources"][os.path.realpath(twin)]
            self.assertTrue(entry["conflict"])
            self.assertEqual(entry["issue"], "source-conflicting-observation")
            self.assertEqual(entry["sha256"],
                             hashlib.sha256(twin.read_bytes()).hexdigest())


class TestFatalRefusals(InventoryTestCase):
    def setUp(self):
        """Valid association controls precede each negative fixture variant."""
        TestValidAssociations.test_library_rlib_depfile_sources_hashes_and_unqualified_report(self)
        TestValidAssociations.test_proc_macro_normalized_depfile(self)
        TestValidAssociations.test_test_executable_adjacent_depfile(self)
        TestValidAssociations.test_custom_build_inode_sibling_association(self)

    def tearDown(self):
        """Fresh association controls establish recovery after the variant."""
        self.setUp()

    def test_fatal_inputs_publish_nothing(self):
        """Malformed or unsuccessful input refuses before report publication."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            out = root / "report.json"
            cases = {
                "malformed-json": lambda p: Path(p).write_text(
                    '{"reason": "compiler-artifact"\n', encoding="utf-8"),
                "no-build-finished": lambda p: self.write_stdout(
                    p, [{"reason": "build-script-executed"}]),
                "failed-build": lambda p: self.write_stdout(
                    p, [{"reason": "build-finished", "success": False}]),
                "empty-artifacts": lambda p: self.write_stdout(
                    p, [{"reason": "build-finished", "success": True}]),
            }
            original_stdout = Path(stdout).read_bytes()
            for label, writer in cases.items():
                Path(stdout).write_bytes(original_stdout)
                self.assertEqual(self.run_tool(workspace, target, registry, stdout, root / (label + "-control.json")), 0)
                with self.subTest(label=label):
                    writer(stdout)
                    self.assertEqual(
                        self.run_tool(workspace, target, registry, stdout, out), 2)
                    self.assertFalse(out.exists())

    def test_existing_output_refused_without_overwrite(self):
        """Existing output sentinel remains byte-identical after exclusive-create refusal."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            out = root / "report.json"
            out.write_text("sentinel\n", encoding="utf-8")
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, out), 2)
            self.assertEqual(out.read_text(encoding="utf-8"), "sentinel\n")

    def test_missing_directory_argument_refused(self):
        """A missing caller directory fails before any report is created."""
        with self.tempdir() as root:
            root = Path(root)
            workspace, target, registry, stdout, src = self.healthy_fixture(root)
            out = root / "report.json"
            rc = MOD.main([
                "--cargo-stdout", str(stdout),
                "--workspace", str(root / "missing"),
                "--target", str(target),
                "--registry-root", str(registry),
                "--output", str(out),
            ])
            self.assertEqual(rc, 2)
            self.assertFalse(out.exists())


class TestNamedHelpers(unittest.TestCase):
    def test_parse_depfile_accepts_observed_convention(self):
        """Decode escaped source spaces in the observed Make-style rule."""
        rule, tokens = MOD.parse_depfile("/t/deps/x-1.d: /a\\ b.rs rel.rs\n")
        self.assertEqual(rule, "/t/deps/x-1.d")
        self.assertEqual(tokens, ["/a b.rs", "rel.rs"])

    def test_parse_depfile_rejects_unsupported_syntax(self):
        """Unknown or inconsistent rule syntax is a refusal, never omitted input."""
        cases = {
            "depfile-continuation-unsupported": "/t/x.d: /a.rs \\\n  /b.rs\n",
            "depfile-inconsistent-rule": "/t/x.d: /a.rs\n/t/y.d: /b.rs\n",
            "depfile-no-prerequisites": "/t/x.d:\n",
            "depfile-unsupported-syntax": "/t/x.d /a.rs\n",
            "depfile-lexical-error": "/t/x.d: 'unterminated\n",
        }
        for expected, text in cases.items():
            with self.subTest(expected=expected):
                with self.assertRaises(MOD.DepfileError) as ctx:
                    MOD.parse_depfile(text)
                self.assertEqual(ctx.exception.cause, expected)

    def test_resolve_source_token_convention(self):
        """Resolve relative source names at the observed Cargo manifest root."""
        self.assertEqual(MOD.resolve_source_token("/abs/x.rs", "/ws/rust"),
                         "/abs/x.rs")
        self.assertEqual(MOD.resolve_source_token("src/lib.rs", "/ws/rust"),
                         "/ws/rust/src/lib.rs")
        self.assertEqual(MOD.resolve_source_token("./src/lib.rs", "/ws/rust"),
                         "/ws/rust/src/lib.rs")

    def test_observations_differ_detects_drift(self):
        """Identity/size/mtime changes invalidate one bracketed read observation."""
        before = os.stat_result((0o100644, 11, 21, 1, 0, 0, 10, 0, 0, 0))
        same = os.stat_result((0o100644, 11, 21, 1, 0, 0, 10, 0, 0, 0))
        grown = os.stat_result((0o100644, 11, 21, 1, 0, 0, 11, 0, 0, 0))
        self.assertFalse(MOD.observations_differ(before, same))
        self.assertTrue(MOD.observations_differ(before, grown))




class TestPublicationDiagnostics(InventoryTestCase):
    def test_write_and_close_failure_preserve_typed_causes_and_visible_partial(self):
        """Healthy then synthetic write/close refusal, followed by fresh recovery.

        No disk exhaustion is induced. The file is deliberately partial and
        retained; CLI errors must carry both errno values rather than rollback.
        """
        with self.tempdir() as directory:
            root = Path(directory)
            workspace, target, registry, stdout, source = self.healthy_fixture(root)
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, root / "control.json"), 0)
            real_fdopen = MOD.os.fdopen
            class FaultStream:
                """Own and release the real fixture descriptor; inject diagnostics."""
                def __init__(self, stream):
                    """Own the fixture stream until its one close; no independent descriptor acquisition."""
                    self.stream = stream
                def __getattr__(self, key):
                    """Delegate fixture stream operations without hiding their errors."""
                    return getattr(self.stream, key)
                def write(self, text):
                    """Leave a visible partial fixture file then inject a synthetic write refusal."""
                    self.stream.write("{")
                    self.stream.flush()
                    raise OSError(28, "synthetic report write refusal")
                def close(self):
                    """Release the actual fixture descriptor then inject a synthetic close diagnostic."""
                    self.stream.close()
                    raise OSError(5, "synthetic report close refusal")
            stderr = io.StringIO()
            with patch.object(MOD.os, "fdopen", side_effect=lambda *a, **k: FaultStream(real_fdopen(*a, **k))), contextlib.redirect_stderr(stderr):
                code = self.run_tool(workspace, target, registry, stdout, root / "partial.json")
            self.assertEqual(code, 2)
            self.assertEqual((root / "partial.json").read_text(), "{")
            self.assertEqual(self.run_tool(workspace, target, registry, stdout, root / "recovered.json"), 0)
            try:
                diagnostic = json.loads(stderr.getvalue().splitlines()[-1])
            except json.JSONDecodeError:
                diagnostic = {}
            def nodes(value):
                """Walk retained typed error records for errno assertions."""
                if isinstance(value, dict):
                    yield value
                    for child in value.values():
                        yield from nodes(child)
                elif isinstance(value, list):
                    for child in value:
                        yield from nodes(child)
            observed = {node.get("errno") for node in nodes(diagnostic) if "errno" in node}
            self.assertTrue({28, 5}.issubset(observed), diagnostic)

if __name__ == "__main__":
    unittest.main()
