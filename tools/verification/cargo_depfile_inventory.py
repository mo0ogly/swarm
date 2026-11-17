#!/usr/bin/env python3
"""Post-build, read-only inventory of Cargo dep-info files.

Purpose
    Read a caller-captured ``cargo --message-format=json`` stdout file, require
    that the build finished successfully and produced artifacts, resolve the
    dep-info (``.d``) file of every ``compiler-artifact`` event, hash and
    classify the sources named by the depfile rule line, and publish one fresh
    JSON report.  The report is always unqualified observational data.

Artifact rules (Cargo's observed layout; nothing else is admitted)
    library rlib     ``deps/libNAME-HASH.rlib`` -> adjacent ``NAME-HASH.d``
    proc-macro       ``deps/libNAME-HASH.so``   -> adjacent ``NAME-HASH.d``
    test executable  ``executable`` path        -> exact adjacent ``.d``
    custom-build     ``filenames[0]`` is the emitted ``build-script-build``;
                     exactly one sibling matching ``NAME_HASH`` (target name
                     with ``-`` normalised to ``_``) must share ``st_dev`` +
                     ``st_ino`` with it; that sibling's adjacent ``.d`` is
                     used.  Missing, ambiguous, wrong-inode or non-regular
                     siblings are refused.
    Any other shape (non-test executables, dylibs, rmeta-only, ...) is recorded
    as an unresolved artifact with cause ``unsupported-artifact-kind``.

Refusals
    Fatal (exit 2; no complete report guaranteed after IO failure): unreadable or non-UTF-8 stdout, malformed
    JSON lines, non-object events, missing or failed ``build-finished``
    success, empty artifact set, unknown fields on ``build-finished``, missing
    CLI directories, missing output parent, exclusive-create or write/fsync
    failures.  Per artifact (report still published, exit nonzero): unknown or
    mistyped artifact fields, artifact/depfile missing, symlinked, not regular
    or outside ``--target``, ambiguous rlib/so, missing ``lib`` prefix,
    custom-build sibling problems, depfile continuations, inconsistent rules,
    lexical/syntax errors, stat drift between the reads bracketing a file, and
    conflicting observations of one canonical source.  There is no
    content-only or mtime fallback, no retry, no timeout, no alternate paths.

Depfile parsing
    Repeated Rust artifact rules must name the same prerequisite set; phony
    rules name those sources, and comment metadata is retained without inference.  Tokens are split with the
    established shlex convention (POSIX mode, whitespace split, backslash
    escapes, no comment character); the first token must end with ``:`` and is
    the rule target (observed to be the ``.d`` path itself, not the
    executable); every further token is a prerequisite.  Continuations, inconsistent rules, unrelated phony targets and zero first
    prerequisites are rejected instead of silently dropping
    inputs.  Absolute prerequisite paths stay absolute; relative tokens
    resolve against ``<workspace>/rust``, the manifest root of the actual
    Cargo invocation.

Sources
    Manifest paths, target ``src_path`` values and all depfile prerequisites
    are deduplicated by canonical absolute path, hashed (SHA-256) when they
    are regular files, and classified as ``target-generated``,
    ``caller-declared-registry``, ``workspace`` or ``unclassified-external``
    purely from the caller-declared roots, without inferring provenance or
    functionality.  ``--registry-root`` is a classification declaration, never
    evidence of authentic provenance.  Source and depfile body observations are bracketed by lstat
    calls; drift or conflicting observations are refused.

Report
    ``status`` is always ``OBSERVED_POSTBUILD_ONLY`` with ``scope``
    ``postbuild-depfile-inventory-only``, ``complete_build_qualification`` and
    ``behavioral_admission`` false even when every artifact maps, plus
    explicit still-unproved flags: input bytes at compiler read time, the
    exact compiler/linker/C/ancestor configuration environment, generated or
    proc-macro side inputs omitted from depfiles, and the full linkage and
    runtime closure.  The tool never emits any admission record and never
    asserts that all dependencies are proven; a nonzero exit only means
    "unresolved artifacts or source issues observed", never qualification.

Inputs (CLI, all required, no defaults, no environment reads)
    --cargo-stdout PATH   trusted, caller-owned captured Cargo JSON stdout
    --workspace PATH      workspace root; ``<root>/rust`` is the manifest root
    --target PATH         Cargo target directory; artifacts, executables,
                          depfiles and build-script siblings must be inside it
    --registry-root PATH  caller-declared registry source root (classification)
    --output PATH         fresh report path; must not exist; parent must exist

Side effects
    Reads inputs and the observed trees; creates only ``--output`` (exclusive
    create, flushed and fsynced, never overwritten); writes one summary line
    to stdout.  Standard library only; no subprocess, network, compilation,
    ELF parsing or test execution.

Limits
    The caller owns mount, reservation and file-descriptor admission and
    freezes the source and target trees; this tool neither reimplements that
    orchestration nor re-verifies it, and assumes no concurrent writer.
    Containment of artifacts and depfiles is lexical from the admitted
    ``--target`` root plus per-component symlink checks below it; the root and
    its ancestors are the caller's admission responsibility, and none of this
    is protection against hostile same-UID replacement.  ``fresh`` artifacts
    are inventoried from their on-disk depfiles like any other artifact.
"""

import argparse
import hashlib
import json
import os
import re
import shlex
import stat
import sys

SCHEMA_ID = "cargo-depfile-inventory/1"
STATUS = "OBSERVED_POSTBUILD_ONLY"
SCOPE = "postbuild-depfile-inventory-only"

UNPROVED_FLAGS = {
    "input_bytes_at_compiler_read_time": True,
    "exact_compiler_linker_c_ancestor_config_environment": True,
    "generated_or_proc_macro_side_inputs_omitted_from_depfiles": True,
    "full_linkage_and_runtime_closure": True,
}

ARTIFACT_EVENT_KEYS = frozenset({
    "reason", "package_id", "manifest_path", "target", "profile",
    "features", "filenames", "executable", "fresh",
})
BUILD_FINISHED_KEYS = frozenset({"reason", "success"})


class FatalRefusal(Exception):
    """Run-fatal input or IO problem; ``str()`` is a safe cause token.

    Input refusals precede publication; publication IO failures may leave a
    partial new file. A nonzero exit never proves absence or rollback.  Never swallowed.
    """


class ArtifactRefusal(Exception):
    """Refusal scoped to one artifact or one observation.

    Attributes:
        cause: safe, input-independent kebab token recorded in the report.
    """

    def __init__(self, cause):
        """Store a safe diagnostic code without IO or acceptance authority."""
        super().__init__(cause)
        self.cause = cause


class DepfileError(ArtifactRefusal):
    """Refusal scoped to one depfile's syntax, encoding or observation."""


def _norm(path):
    """Absolute, normalized form of a path without resolving symlinks."""
    return os.path.normpath(os.path.abspath(path))


def _is_within_strict(path, root):
    """True when the normalized path is strictly inside the normalized root."""
    p = _norm(path)
    r = _norm(root)
    if r == os.sep:
        return p != os.sep
    return p.startswith(r + os.sep)


def observations_differ(before, after):
    """True when identity, size or mtime drifted between two bracketing lstats.

    Inputs are two ``os.stat_result`` values taken before and after a read;
    atime is deliberately ignored.  Drift marks the bytes read as untrusted.
    """
    return (
        before.st_dev != after.st_dev
        or before.st_ino != after.st_ino
        or before.st_size != after.st_size
        or before.st_mtime_ns != after.st_mtime_ns
    )


def load_cargo_stdout(path):
    """Parse the captured Cargo JSON stdout (JSON lines).

    Returns ``(artifact_events, build_finished_events, build_script_count,
    other_count, sha256_hex, size_bytes)``.  Raises FatalRefusal when the file
    cannot be read or decoded, when any non-empty line is not a JSON object
    carrying a string ``reason``, or when a ``build-finished`` event carries
    unknown fields or a non-boolean ``success``.  The caller owns and trusts
    this file; no other channel is read.
    """
    try:
        with open(path, "rb") as handle:
            raw = handle.read()
    except OSError:
        raise FatalRefusal("cargo-stdout-unreadable")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        raise FatalRefusal("cargo-stdout-not-utf8")
    artifacts, finished, scripts, other = [], [], 0, 0
    for lineno, line in enumerate(text.splitlines(), 1):
        if not line.strip():
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            raise FatalRefusal("cargo-stdout-malformed-json:line=%d" % lineno)
        if not isinstance(event, dict) or not isinstance(event.get("reason"), str):
            raise FatalRefusal("cargo-stdout-malformed-event:line=%d" % lineno)
        reason = event["reason"]
        if reason == "compiler-artifact":
            artifacts.append(event)
        elif reason == "build-finished":
            if set(event) - BUILD_FINISHED_KEYS:
                raise FatalRefusal("cargo-json-unknown-field:build-finished")
            if not isinstance(event.get("success"), bool):
                raise FatalRefusal("cargo-stdout-malformed-event:build-finished")
            finished.append(event)
        elif reason == "build-script-executed":
            scripts += 1
        else:
            other += 1
    return (artifacts, finished, scripts, other,
            hashlib.sha256(raw).hexdigest(), len(raw))


def validate_artifact_event(event):
    """Validate one compiler-artifact event against the observed schema.

    Unknown top-level fields, missing essential fields and mistyped values are
    refused with a safe cause; nested ``target``/``profile`` keys beyond the
    essential ones are ignored, as they are not consumed here.  Returns the
    event unchanged; raises ArtifactRefusal otherwise.
    """
    if set(event) - ARTIFACT_EVENT_KEYS:
        raise ArtifactRefusal("cargo-json-unknown-field")
    for key in ("package_id", "manifest_path", "target", "profile",
                "filenames", "executable"):
        if key not in event:
            raise ArtifactRefusal("cargo-json-field-missing")
    typed = (
        ("package_id", str), ("manifest_path", str), ("target", dict),
        ("profile", dict), ("filenames", list),
        ("executable", (str, type(None))),
    )
    for key, kinds in typed:
        if not isinstance(event[key], kinds):
            raise ArtifactRefusal("cargo-json-field-type")
    if "features" in event and not (
        isinstance(event["features"], list)
        and all(isinstance(item, str) for item in event["features"])
    ):
        raise ArtifactRefusal("cargo-json-field-type")
    if "fresh" in event and not isinstance(event["fresh"], bool):
        raise ArtifactRefusal("cargo-json-field-type")
    target = event["target"]
    for key in ("kind", "name", "src_path"):
        if key not in target:
            raise ArtifactRefusal("cargo-json-field-missing")
    if not (isinstance(target["kind"], list)
            and all(isinstance(kind, str) for kind in target["kind"])):
        raise ArtifactRefusal("cargo-json-field-type")
    if "crate_types" in target and not (
        isinstance(target["crate_types"], list)
        and all(isinstance(kind, str) for kind in target["crate_types"])
    ):
        raise ArtifactRefusal("cargo-json-field-type")
    if not isinstance(target["name"], str) or not isinstance(target["src_path"], str):
        raise ArtifactRefusal("cargo-json-field-type")
    if not all(isinstance(name, str) for name in event["filenames"]):
        raise ArtifactRefusal("cargo-json-field-type")
    profile = event["profile"]
    if "test" not in profile:
        raise ArtifactRefusal("cargo-json-field-missing")
    if not isinstance(profile["test"], bool):
        raise ArtifactRefusal("cargo-json-field-type")
    if not os.path.isabs(event["manifest_path"]) or not os.path.isabs(target["src_path"]):
        raise ArtifactRefusal("cargo-json-field-not-absolute")
    return event


def verify_inside_target(path_str, target_root, cause_prefix):
    """Verify one file is absolute, inside the target root and symlink-free.

    Walks every path component below the admitted target root and refuses any
    symlinked component; the target root and its ancestors are the caller's
    admission responsibility and are not re-checked.  Returns the lstat result
    of the file; raises ArtifactRefusal with ``<cause_prefix>-...`` causes for
    relative paths, escape outside the root, symlinked components, missing
    files, stat errors or non-regular files.
    """
    if not os.path.isabs(path_str):
        raise ArtifactRefusal(cause_prefix + "-path-not-absolute")
    p = _norm(path_str)
    base = _norm(target_root)
    if not _is_within_strict(p, base):
        raise ArtifactRefusal(cause_prefix + "-outside-target")
    rel = p[len(base):]
    if rel.startswith(os.sep):
        rel = rel[len(os.sep):]
    current = base
    for part in rel.split(os.sep):
        if not part or part == ".":
            continue
        current = os.path.join(current, part)
        if os.path.islink(current):
            raise ArtifactRefusal(cause_prefix + "-symlink")
    try:
        info = os.lstat(p)
    except FileNotFoundError:
        raise ArtifactRefusal(cause_prefix + "-missing")
    except OSError:
        raise ArtifactRefusal(cause_prefix + "-stat-error")
    if not stat.S_ISREG(info.st_mode):
        raise ArtifactRefusal(cause_prefix + "-not-regular")
    return info


def normalized_lib_depfile(path_str, cause_prefix):
    """Strip the ``lib`` prefix and swap the extension for ``.d``.

    Maps ``deps/libNAME-HASH.rlib`` and ``deps/libNAME-HASH.so`` to the
    adjacent ``deps/NAME-HASH.d``.  Raises ArtifactRefusal when the file name
    lacks the ``lib`` prefix or would produce an empty stem.
    """
    name = os.path.basename(path_str)
    if not name.startswith("lib") or len(name) <= len("lib"):
        raise ArtifactRefusal(cause_prefix + "-prefix-missing")
    if name.endswith(".rlib"):
        stem = name[len("lib"):-len(".rlib")]
    else:
        stem = name[len("lib"):-len(".so")]
    if not stem:
        raise ArtifactRefusal(cause_prefix + "-prefix-missing")
    return os.path.join(os.path.dirname(path_str), stem + ".d")


def adjacent_depfile(path_str):
    """Exact adjacent depfile: the file's last suffix replaced by ``.d``."""
    directory, name = os.path.split(path_str)
    stem = name.rsplit(".", 1)[0] if "." in name else name
    if not stem:
        raise ArtifactRefusal("depfile-path-unsupported")
    return os.path.join(directory, stem + ".d")


def find_custom_build_sibling(emitted_path, normalized_name):
    """Find exactly one regular ``NAME-HASH`` sibling sharing the inode.

    Scans the directory of the emitted ``build-script-build`` for entries
    matching ``<normalized_name>-<hex>`` and keeps the regular, non-symlink
    ones whose ``st_dev`` + ``st_ino`` equal the emitted file's.  Raises
    ArtifactRefusal ``custom-build-sibling-missing-or-wrong-inode`` when none
    matches, ``custom-build-sibling-ambiguous`` when more than one matches,
    and ``custom-build-sibling-scan-error`` on directory/lstat failures.
    """
    pattern = re.compile(re.escape(normalized_name) + r"-[0-9a-f]+\Z")
    directory = os.path.dirname(emitted_path) or "."
    try:
        emitter_stat = os.lstat(emitted_path)
        with os.scandir(directory) as entries:
            candidates = list(entries)
    except OSError:
        raise ArtifactRefusal("custom-build-sibling-scan-error")
    matches = []
    for entry in candidates:
        if not pattern.match(entry.name):
            continue
        try:
            entry_stat = os.lstat(entry.path)
        except OSError:
            continue
        if not stat.S_ISREG(entry_stat.st_mode):
            continue
        if (entry_stat.st_dev == emitter_stat.st_dev
                and entry_stat.st_ino == emitter_stat.st_ino):
            matches.append(entry.path)
    if not matches:
        raise ArtifactRefusal("custom-build-sibling-missing-or-wrong-inode")
    if len(matches) > 1:
        raise ArtifactRefusal("custom-build-sibling-ambiguous")
    return matches[0]


def plan_artifact_depfile(event, target_root):
    """Derive the depfile location for one validated artifact event.

    Returns a dict with ``kind``, ``artifact_file``, ``depfile_owner`` (the
    custom-build inode sibling, else None) and ``depfile``.  Raises
    ArtifactRefusal for unsupported kinds and for artifact or depfile paths
    that are relative, outside the target root, symlinked, non-regular or
    missing.  No content-only or mtime fallback exists.
    """
    kinds = event["target"]["kind"]
    executable = event["executable"]
    filenames = event["filenames"]
    if "custom-build" in kinds:
        if executable is not None:
            raise ArtifactRefusal("unsupported-artifact-kind")
        if len(filenames) != 1:
            raise ArtifactRefusal("custom-build-emitter-ambiguous")
        emitted = filenames[0]
        verify_inside_target(emitted, target_root, "artifact")
        normalized = event["target"]["name"].replace("-", "_")
        sibling = find_custom_build_sibling(emitted, normalized)
        verify_inside_target(sibling, target_root, "depfile-owner")
        depfile = sibling + ".d"
        verify_inside_target(depfile, target_root, "depfile")
        return {"kind": "custom-build", "artifact_file": emitted,
                "depfile_owner": sibling, "depfile": depfile}
    if executable is not None:
        if event["profile"]["test"] is not True:
            raise ArtifactRefusal("unsupported-artifact-kind")
        verify_inside_target(executable, target_root, "artifact")
        depfile = adjacent_depfile(executable)
        verify_inside_target(depfile, target_root, "depfile")
        return {"kind": "test-executable", "artifact_file": executable,
                "depfile_owner": None, "depfile": depfile}
    crate_types = event["target"].get("crate_types") or []
    if "proc-macro" in kinds or "proc-macro" in crate_types:
        shared = [name for name in filenames if name.endswith(".so")]
        if not shared:
            raise ArtifactRefusal("proc-macro-artifact-missing")
        if len(shared) > 1:
            raise ArtifactRefusal("proc-macro-artifact-ambiguous")
        verify_inside_target(shared[0], target_root, "artifact")
        depfile = normalized_lib_depfile(shared[0], "proc-macro")
        verify_inside_target(depfile, target_root, "depfile")
        return {"kind": "proc-macro", "artifact_file": shared[0],
                "depfile_owner": None, "depfile": depfile}
    rlibs = [name for name in filenames if name.endswith(".rlib")]
    if rlibs:
        if len(rlibs) > 1:
            raise ArtifactRefusal("library-rlib-ambiguous")
        verify_inside_target(rlibs[0], target_root, "artifact")
        depfile = normalized_lib_depfile(rlibs[0], "library-rlib")
        verify_inside_target(depfile, target_root, "depfile")
        return {"kind": "library-rlib", "artifact_file": rlibs[0],
                "depfile_owner": None, "depfile": depfile}
    raise ArtifactRefusal("unsupported-artifact-kind")


def read_depfile(path_str):
    """Read one already-verified depfile with before/after drift detection.

    Returns a dict with ``path``, ``sha256``, ``size_bytes`` and the UTF-8
    ``content_text`` (this preserves the depfile bytes and hash in the
    report).  Raises DepfileError on stat drift, read failure or invalid
    UTF-8; a non-UTF-8 depfile is refused, never transcoded.
    """
    try:
        before = os.lstat(path_str)
        with open(path_str, "rb") as handle:
            raw = handle.read()
        after = os.lstat(path_str)
    except OSError:
        raise DepfileError("depfile-read-error")
    if observations_differ(before, after):
        raise DepfileError("depfile-drift")
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError:
        raise DepfileError("depfile-not-utf8")
    return {
        "path": _norm(path_str),
        "sha256": hashlib.sha256(raw).hexdigest(),
        "size_bytes": len(raw),
        "content_text": text,
    }


def split_depfile_tokens(line):
    """Split one depfile rule line with the established shlex convention.

    POSIX mode, whitespace split, backslash escapes, no comment character, so
    escaped spaces survive and ``#`` in paths is not treated as a comment.
    Raises DepfileError ``depfile-lexical-error`` on unbalanced quoting or a
    dangling escape; nothing is dropped silently.
    """
    lexer = shlex.shlex(line, posix=True)
    lexer.whitespace_split = True
    lexer.commenters = ""
    try:
        return list(lexer)
    except ValueError:
        raise DepfileError("depfile-lexical-error")


def parse_depfile(text):
    """Read Rust dep-info repeated artifact rules and source phony rules.

    Nonempty rules must repeat the first prerequisite set; phony targets must
    name an already listed prerequisite. Comment metadata is retained in the
    raw depfile but is not interpreted as compiler environment proof. Unknown
    syntax, continuations or differing source sets refuse without omission.
    Return the first target and its prerequisite sequence, preserving order.
    """
    normalized = text.replace("\r\n", "\n")
    if "\\\n" in normalized:
        raise DepfileError("depfile-continuation-unsupported")
    rules = [line for line in normalized.splitlines()
             if line.strip() and not line.lstrip().startswith("#")]
    if not rules:
        raise DepfileError("depfile-empty")
    first_target, first_sources = None, None
    for line in rules:
        tokens = split_depfile_tokens(line)
        if not tokens or not tokens[0].endswith(":") or len(tokens[0]) < 2:
            raise DepfileError("depfile-unsupported-syntax")
        target, sources = tokens[0][:-1], tokens[1:]
        if first_sources is None:
            if not sources:
                raise DepfileError("depfile-no-prerequisites")
            first_target, first_sources = target, sources
        elif sources:
            if set(sources) != set(first_sources):
                raise DepfileError("depfile-inconsistent-rule")
        elif target not in first_sources:
            raise DepfileError("depfile-unrelated-phony-rule")
    return first_target, first_sources


def resolve_source_token(token, manifest_root):
    """Resolve one depfile prerequisite token to a normalized absolute path.

    Absolute tokens stay absolute; relative tokens resolve against the Cargo
    invocation manifest root ``<workspace>/rust``.  Purely lexical; raises
    nothing.
    """
    if os.path.isabs(token):
        return _norm(token)
    return _norm(os.path.join(manifest_root, token))


def classify_source(path, workspace_root, target_root, registry_root):
    """Classify one absolute source path from caller-declared roots only.

    Precedence: target-generated, caller-declared-registry, workspace, then
    unclassified-external.  Purely lexical; no provenance or functionality is
    inferred and the registry root is never treated as authentic provenance.
    """
    if _is_within_strict(path, target_root):
        return "target-generated"
    if _is_within_strict(path, registry_root):
        return "caller-declared-registry"
    if _is_within_strict(path, workspace_root):
        return "workspace"
    return "unclassified-external"


def observe_source(path):
    """Observe one absolute source path: canonical form, hash, issues.

    lstat is taken before and after the read; drift between the two marks the
    observation with ``source-drift`` while keeping the computed hash visible.
    Symlinked or non-regular files, missing files and read errors are recorded
    as issues instead of being guessed at.  Never raises; always returns a
    dict with ``path``, ``canonical``, ``regular_file``, ``sha256``,
    ``size_bytes`` and ``issue``.
    """
    entry = {
        "path": _norm(path),
        "canonical": os.path.realpath(_norm(path)),
        "regular_file": None,
        "sha256": None,
        "size_bytes": None,
        "issue": None,
        "error_details": None,
    }
    try:
        before = os.lstat(entry["path"])
    except FileNotFoundError as error:
        entry["error_details"] = error_details(error)
        entry["issue"] = "source-missing"
        return entry
    except OSError as error:
        entry["error_details"] = error_details(error)
        entry["issue"] = "source-read-error"
        return entry
    if stat.S_ISLNK(before.st_mode):
        entry["issue"] = "source-symlink"
        return entry
    if not stat.S_ISREG(before.st_mode):
        entry["issue"] = "source-not-regular"
        return entry
    try:
        with open(entry["path"], "rb") as handle:
            data = handle.read()
        after = os.lstat(entry["path"])
    except OSError as error:
        entry["error_details"] = error_details(error)
        entry["issue"] = "source-read-error"
        return entry
    entry["regular_file"] = True
    entry["sha256"] = hashlib.sha256(data).hexdigest()
    entry["size_bytes"] = len(data)
    if observations_differ(before, after):
        entry["issue"] = "source-drift"
    return entry


def build_report(opts):
    """Produce the report dict and a health flag from parsed CLI options.

    Raises FatalRefusal for run-fatal input problems (no report is published
    in that case); per-artifact problems are embedded as unresolved records
    with safe causes while all partial observations are preserved.  Read-only
    over the observed trees; publishing is done separately by
    :func:`publish_report`.
    """
    artifacts, finished, scripts, other, stdout_sha256, stdout_size = (
        load_cargo_stdout(opts.cargo_stdout))
    if any(event["success"] is False for event in finished):
        raise FatalRefusal("build-finished-failure")
    if not any(event["success"] is True for event in finished):
        raise FatalRefusal("build-finished-success-missing")
    if not artifacts:
        raise FatalRefusal("compiler-artifacts-empty")

    workspace = _norm(opts.workspace)
    manifest_root = os.path.join(workspace, "rust")
    target_root = _norm(opts.target)
    registry_root = _norm(opts.registry_root)

    sources = {}

    def intern_source(abs_path):
        """Observe one absolute path; dedupe by canonical absolute path.

        Returns the canonical key.  Conflicting classifications or differing
        observations (hash, issue, kind) of one canonical path are refused by
        flagging the shared entry; nothing is overwritten.
        """
        entry = observe_source(abs_path)
        key = entry["canonical"]
        classification = classify_source(entry["path"], workspace,
                                         target_root, registry_root)
        existing = sources.get(key)
        if existing is None:
            entry["classification"] = classification
            entry["conflict"] = False
            sources[key] = entry
            return key
        if existing["classification"] != classification:
            existing["conflict"] = True
            existing["issue"] = existing["issue"] or "source-conflicting-observation"
        if ((existing["sha256"], existing["issue"], existing["regular_file"])
                != (entry["sha256"], entry["issue"], entry["regular_file"])):
            existing["conflict"] = True
            existing["issue"] = existing["issue"] or "source-conflicting-observation"
        return key

    records = []
    for event in artifacts:
        target_obj = event.get("target") if isinstance(event.get("target"), dict) else {}
        record = {
            "package_id": event.get("package_id"),
            "manifest_path": event.get("manifest_path"),
            "target_kind": target_obj.get("kind"),
            "target_name": target_obj.get("name"),
            "target_src_path": target_obj.get("src_path"),
            "filenames": event.get("filenames"),
            "executable": event.get("executable"),
            "fresh": event.get("fresh"),
            "artifact_kind": None,
            "artifact_file": None,
            "depfile_owner": None,
            "depfile": None,
            "depfile_rule_target": None,
            "depfile_sources": [],
            "sources": [],
            "unresolved": True,
            "cause": None,
        }
        try:
            validate_artifact_event(event)
        except ArtifactRefusal as exc:
            record["cause"] = exc.cause
            record["error_details"] = error_details(exc)
            records.append(record)
            continue
        record["target_kind"] = event["target"]["kind"]
        record["target_name"] = event["target"]["name"]
        record["target_src_path"] = event["target"]["src_path"]
        intern_source(event["manifest_path"])
        intern_source(event["target"]["src_path"])
        try:
            plan = plan_artifact_depfile(event, target_root)
            record["artifact_kind"] = plan["kind"]
            record["artifact_file"] = plan["artifact_file"]
            record["depfile_owner"] = plan["depfile_owner"]
            depfile = read_depfile(plan["depfile"])
            record["depfile"] = depfile
            rule_target, tokens = parse_depfile(depfile["content_text"])
            record["depfile_rule_target"] = rule_target
            seen = set()
            for token in tokens:
                resolved = resolve_source_token(token, manifest_root)
                key = intern_source(resolved)
                record["depfile_sources"].append(
                    {"token": token, "resolved": resolved, "source": key})
                if key not in seen:
                    seen.add(key)
                    record["sources"].append(key)
            record["unresolved"] = False
        except ArtifactRefusal as exc:
            record["cause"] = exc.cause
            record["error_details"] = error_details(exc)
        records.append(record)

    mapped = sum(1 for item in records if not item["unresolved"])
    unresolved_count = len(records) - mapped
    sources_with_issues = sum(
        1 for entry in sources.values() if entry["issue"] or entry["conflict"])
    report = {
        "schema": SCHEMA_ID,
        "status": STATUS,
        "scope": SCOPE,
        "complete_build_qualification": False,
        "behavioral_admission": False,
        "unproved": dict(UNPROVED_FLAGS),
        "inputs": {
            "cargo_stdout": os.path.abspath(opts.cargo_stdout),
            "cargo_stdout_sha256": stdout_sha256,
            "cargo_stdout_size_bytes": stdout_size,
            "workspace": workspace,
            "manifest_root": manifest_root,
            "target": target_root,
            "registry_root": registry_root,
        },
        "counts": {
            "compiler_artifact_events": len(artifacts),
            "build_finished_events": len(finished),
            "build_script_executed_events": scripts,
            "other_events": other,
            "artifacts_mapped": mapped,
            "artifacts_unresolved": unresolved_count,
            "sources_total": len(sources),
            "sources_with_issues": sources_with_issues,
        },
        "artifacts": records,
        "sources": {
            key: {
                "path": entry["path"],
                "classification": entry["classification"],
                "regular_file": entry["regular_file"],
                "sha256": entry["sha256"],
                "size_bytes": entry["size_bytes"],
                "conflict": entry["conflict"],
                "issue": entry["issue"],
                "error_details": entry.get("error_details"),
            }
            for key, entry in sources.items()
        },
    }
    return report, (unresolved_count == 0 and sources_with_issues == 0)


def error_details(error, seen=None):
    """Render safe typed causes/errno without exposing arbitrary environment.

    Preserve groups, explicit and implicit causes. This is a diagnostic, never
    a status, retry or authority decision. Cycle references remain explicit.
    """
    seen = set() if seen is None else seen
    if id(error) in seen:
        return {"type": type(error).__name__, "cycle": True}
    seen.add(id(error))
    value = {"type": type(error).__name__}
    if isinstance(error, OSError):
        value.update(errno=error.errno, reason=error.strerror)
    elif isinstance(error, (FatalRefusal, ArtifactRefusal)):
        value["reason"] = str(error)
    if getattr(error, "exceptions", None):
        value["exceptions"] = [error_details(child, seen) for child in error.exceptions]
    if error.__cause__ is not None:
        value["cause"] = error_details(error.__cause__, seen)
    elif error.__context__ is not None and not error.__suppress_context__:
        value["context"] = error_details(error.__context__, seen)
    return value


def publish_report(output_path, report):
    """Exclusively publish a private report, preserving IO and cleanup causes.

    Parent is caller-admitted. Existing files remain unchanged; failed writes,
    flushes or close/sync can leave a visible partial or complete file whose
    publication is uncertain. Nothing is overwritten, deleted or retried.
    Successful file/directory sync is observational durability, not admission.
    """
    payload = json.dumps(report, indent=2, sort_keys=True) + "\n"
    parent = os.path.dirname(os.path.abspath(output_path))
    if not os.path.isdir(parent):
        raise FatalRefusal("output-parent-missing")
    fd = os.open(output_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        handle = os.fdopen(fd, "w", encoding="utf-8")
    except BaseException as primary:
        try:
            os.close(fd)
        except BaseException as cleanup:
            raise BaseExceptionGroup("output opening and cleanup failed", [primary, cleanup])
        raise
    try:
        handle.write(payload)
        handle.flush()
        os.fsync(handle.fileno())
    finally:
        primary = sys.exception()
        try:
            handle.close()
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup("output publication and cleanup failed", [primary, cleanup])
            raise
    directory = os.open(parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        primary = sys.exception()
        try:
            os.close(directory)
        except BaseException as cleanup:
            if primary is not None:
                raise BaseExceptionGroup("directory sync and cleanup failed", [primary, cleanup])
            raise


def main(argv=None):
    """CLI entry point.

    Returns 0 when every artifact mapped and every source was observed
    cleanly, 1 when the report contains unresolved artifacts/source issues,
    2 on fatal refusal or IO failure. IO failure may leave a partial new file;
    absence or rollback is never inferred from this exit code.
    Emits one summary line on success/partial success and a single stderr
    line per fatal refusal; no other logging and no environment reads.
    """
    parser = argparse.ArgumentParser(
        prog="cargo_depfile_inventory.py",
        description="Post-build read-only Cargo dep-info inventory; see the "
                    "module docstring for scope, refusals and limits.")
    parser.add_argument("--cargo-stdout", required=True, metavar="PATH",
                        help="captured Cargo JSON stdout file (trusted, caller-owned)")
    parser.add_argument("--workspace", required=True, metavar="PATH",
                        help="workspace root; <root>/rust resolves relative depfile tokens")
    parser.add_argument("--target", required=True, metavar="PATH",
                        help="Cargo target directory admitted by the caller")
    parser.add_argument("--registry-root", required=True, metavar="PATH",
                        help="caller-declared registry source root, classification only")
    parser.add_argument("--output", required=True, metavar="PATH",
                        help="fresh report path; must not exist; parent must exist")
    opts = parser.parse_args(argv)
    try:
        for name in ("workspace", "target", "registry_root"):
            if not os.path.isdir(getattr(opts, name)):
                raise FatalRefusal("argument-not-a-directory:" + name)
        report, healthy = build_report(opts)
        publish_report(opts.output, report)
    except (FatalRefusal, OSError, ExceptionGroup) as exc:
        print(json.dumps({"tool": "cargo-depfile-inventory", "error": error_details(exc),
              "publication": "No complete report guaranteed; a new output may be visible or partial. Existing output is never overwritten."}), file=sys.stderr)
        return 2
    counts = report["counts"]
    print("cargo-depfile-inventory: mapped=%d unresolved=%d sources=%d "
          "source_issues=%d report=%s"
          % (counts["artifacts_mapped"], counts["artifacts_unresolved"],
             counts["sources_total"], counts["sources_with_issues"],
             opts.output))
    return 0 if healthy else 1


if __name__ == "__main__":
    sys.exit(main())
