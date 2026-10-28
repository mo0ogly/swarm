#!/usr/bin/env python3
"""Retain actual project Go inputs, including root files/embeds, before a check.

Supports the default GOFLAGS profile, with no overlay. External selected source
hashes, module verification and tool identities are recorded; this is not a
hermetic proof of external C/system headers or authenticated execution.
"""
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
from recipe import publish

_FILE_FIELDS = ("GoFiles", "CgoFiles", "CFiles", "CXXFiles", "MFiles", "HFiles",
                "FFiles", "SFiles", "SwigFiles", "SwigCXXFiles", "SysoFiles",
                "TestGoFiles", "XTestGoFiles", "EmbedFiles", "TestEmbedFiles",
                "XTestEmbedFiles")


def digest(raw):
    """Hash complete bytes; a declared hash alone never proves retained content."""
    return hashlib.sha256(raw).hexdigest()


def unique(pairs):
    """Refuse ambiguous metadata/manifest keys rather than adopting a last value."""
    value = {}
    for key, item in pairs:
        if key in value:
            raise ValueError("go-input-duplicate-key")
        value[key] = item
    return value


def json_stream(raw):
    """Decode Go's concatenated package objects; malformed/empty data refuses."""
    decoder = json.JSONDecoder(object_pairs_hook=unique)
    rows = []
    position = 0
    while position < len(raw):
        while position < len(raw) and raw[position].isspace():
            position += 1
        if position == len(raw):
            break
        item, position = decoder.raw_decode(raw, position)
        if not isinstance(item, dict) or item.get("Error") or item.get("DepsErrors"):
            raise ValueError("go-input-package-invalid")
        rows.append(item)
    if not rows:
        raise ValueError("go-input-packages-empty")
    return rows


def observe(root, package, output, label):
    """Discover real selected files under the same default Go profile.

    Go metadata and module-check outputs are retained. Project paths must resolve
    within this root; external selected sources are hashed, not silently archived
    as project data. No generation command, provider, timeout or retry is invoked.
    Nonempty GOFLAGS refuses until actual overlay/flag handling is supported.
    """
    root = Path(root).resolve()
    def run(argv):
        """Run a metadata command without shell/retry; retain the original refusal."""
        result = subprocess.run(argv, cwd=root, capture_output=True, text=True)
        if result.returncode:
            publish(output / (label + "-discovery-error.json"),
                    {"argv": argv, "exit": result.returncode,
                     "stdout": result.stdout, "stderr": result.stderr})
            raise ValueError("go-input-discovery-failed")
        return result.stdout
    env = json.loads(run(["go", "env", "-json", "GOOS", "GOARCH", "CGO_ENABLED",
                          "GOVERSION", "GOROOT", "GOTOOLDIR", "GOFLAGS",
                          "GOEXPERIMENT", "GOWORK", "GOCACHE"]), object_pairs_hook=unique)
    if env["GOFLAGS"]:
        raise ValueError("go-input-nondefault-flags-unsupported")
    raw = run(["go", "list", "-mod=readonly", "-deps", "-test", "-json", package])
    publish(output / (label + "-packages.json"), raw.encode())
    rows = json_stream(raw)
    project = set()
    external = set()
    generated = {}
    cache = Path(env["GOCACHE"]).resolve()
    for item in rows:
        directory = item.get("Dir")
        if not directory:
            continue  # synthetic test-main is derived by the recorded Go toolchain
        directory = Path(directory)
        for field in _FILE_FIELDS:
            names = item.get(field, [])
            if not isinstance(names, list) or any(not isinstance(n, str) for n in names):
                raise ValueError("go-input-file-list-invalid")
            for name in names:
                path = directory / name
                resolved = path.resolve(strict=True)
                if (Path(name).is_absolute() and item.get("Name") == "main"
                        and item.get("ImportPath", "").endswith(".test")
                        and resolved.is_relative_to(cache)):
                    raw_generated = resolved.read_bytes()
                    key = ".go-source-generated/" + digest(raw_generated) + ".go"
                    generated[key] = {"sha256": digest(raw_generated),
                                      "bytes": len(raw_generated), "source": str(resolved)}
                    continue
                if directory.is_relative_to(root):
                    if not resolved.is_relative_to(root):
                        raise ValueError("go-input-project-symlink-escape")
                    project.add(path.relative_to(root).as_posix())
                else:
                    external.add(str(resolved))
    if not any(n.endswith(".go") for n in project):
        raise ValueError("go-input-local-package-missing")
    for name in ("go.mod", "go.sum", "resources.go",
                 "tools/agent-workflows/check.py",
                 "tools/verification/go_source_inputs.py",
                 "tools/verification/recipe.py"):
        if (root / name).is_file():
            project.add(name)
    if env["GOWORK"] not in ("", "off"):
        work = Path(env["GOWORK"]).resolve()
        if not work.is_relative_to(root):
            raise ValueError("go-input-external-workspace-unsupported")
        project.add(work.relative_to(root).as_posix())
        if work.with_name(work.name + ".sum").is_file():
            project.add(work.with_name(work.name + ".sum").relative_to(root).as_posix())
    files = {}
    for name in sorted(project):
        raw = (root / name).read_bytes()
        files[name] = {"sha256": digest(raw), "bytes": len(raw)}
    external_hashes = {name: digest(Path(name).read_bytes()) for name in sorted(external)}
    modules = run(["go", "mod", "verify"])
    publish(output / (label + "-module-verification.stdout"), modules.encode())
    tools = [Path(shutil.which("go")).resolve()]
    tools += [Path(env["GOTOOLDIR"]) / name for name in ("compile", "link")]
    tool_hashes = {str(path): digest(path.read_bytes()) for path in tools}
    # Override values can contain secrets; bind their hashes without rendering.
    overrides = {key: digest(os.environ[key].encode()) if key in os.environ else None
                 for key in ("CC", "CXX", "CGO_CFLAGS", "CGO_CPPFLAGS", "CGO_CXXFLAGS",
                             "CGO_FFLAGS", "CGO_LDFLAGS", "GOTOOLCHAIN")}
    return {"project": files, "generated": generated, "external": external_hashes, "go_environment": env,
            "tools": tool_hashes, "override_hashes": overrides,
            "packages": sorted({(r.get("ImportPath", ""), r.get("ForTest", ""))
                                for r in rows})}



def retained_inputs(observation):
    """Return root and actual generated test-main bytes required by this check.

    Generated names are archive-only identities, not files created in the
    repository. Cache paths remain provenance, never authority or a symlink
    exception for ordinary project source. Reserved-name collisions refuse.
    """
    values = dict(observation["project"])
    for name, facts in observation["generated"].items():
        if name in values:
            raise ValueError("go-input-generated-name-collision")
        values[name] = {"sha256": facts["sha256"], "bytes": facts["bytes"]}
    return values


def sources(root, directory):
    """Verify a baseline chain's actual archive bytes and return retained inputs.

    Artifact references stay in the explicit project root; cycles, duplicate
    members, traversal, altered hashes or unavailable inherited bytes refuse.
    Reads only, never extraction, mutation or authenticity/authority inference.
    """
    root = Path(root).resolve()
    chain = []
    seen = set()
    current = Path(directory).resolve()
    while True:
        if not current.is_relative_to(root) or current in seen:
            raise ValueError("go-input-baseline-location-or-cycle")
        seen.add(current)
        frame = json.loads((current / "source.json").read_bytes(), object_pairs_hook=unique)
        if frame.get("schema_version") != 1:
            raise ValueError("go-input-baseline-schema")
        chain.append((current, frame))
        parent = frame.get("baseline")
        if parent is None:
            break
        target = (current / parent).resolve()
        if not target.is_relative_to(root):
            raise ValueError("go-input-baseline-escape")
        if digest((target / "source.json").read_bytes()) != frame["baseline_sha256"]:
            raise ValueError("go-input-baseline-manifest-changed")
        current = target
    available = {}
    for current, frame in reversed(chain):
        archive = current / "source.tar.gz"
        if digest(archive.read_bytes()) != frame["archive_sha256"]:
            raise ValueError("go-input-baseline-archive-changed")
        with tarfile.open(archive) as tar:
            members = tar.getmembers()
            names = [m.name for m in members]
            if len(names) != len(set(names)) or set(names) != set(frame["delta"]):
                raise ValueError("go-input-baseline-members-invalid")
            for member in members:
                name = Path(member.name)
                if name.is_absolute() or ".." in name.parts or not member.isfile():
                    raise ValueError("go-input-baseline-member-unsafe")
                raw = tar.extractfile(member).read()
                facts = {"sha256": digest(raw), "bytes": len(raw)}
                if facts != frame["files"][member.name]:
                    raise ValueError("go-input-baseline-content-changed")
                available[member.name] = facts
        if any(available.get(name) != facts for name, facts in frame["files"].items()):
            raise ValueError("go-input-baseline-incomplete")
        available = {name: available[name] for name in frame["files"]}
    return available


def prepare(root, output, baseline=None, package="./internal/engine"):
    """Create fresh private proof and retain full source or a verified delta.

    Source reads, metadata and atomic publication failures propagate; no existing
    artifact is overwritten and project sources remain intact. A baseline is
    reusable only after verified successful completion. This prepares evidence,
    never claims the check ran. Archives use measured inputs, no volume cutoff.
    """
    root = Path(root).resolve()
    output = Path(output).resolve()
    if not output.is_relative_to(root):
        raise ValueError("go-input-output-outside-root")
    output.mkdir(mode=0o700)
    before = observe(root, package, output, "before")
    publish(output / "before.json", before)
    previous = {}
    baseline_path = None
    if baseline is not None:
        baseline_path = Path(baseline).resolve()
        if not baseline_path.is_relative_to(root):
            raise ValueError("go-input-baseline-outside-root")
        result = json.loads((baseline_path / "result.json").read_bytes(), object_pairs_hook=unique)
        if result.get("schema_version") != 1 or result.get("status") != "PASS":
            raise ValueError("go-input-baseline-not-verified")
        if digest((baseline_path / "source.json").read_bytes()) != result["source_manifest_sha256"]:
            raise ValueError("go-input-baseline-result-unbound")
        previous = sources(root, baseline_path)
    retained = retained_inputs(before)
    delta = {name: facts for name, facts in retained.items()
             if previous.get(name) != facts}
    print(f"Go input archive: {len(before['project'])} project files; {len(delta)} new/changed", flush=True)
    # Compression takes place on private temporary storage; only an atomic,
    # read-back-checked publication is linked into the permanent proof directory.
    with tempfile.TemporaryDirectory(prefix="swarm-go-source-") as temporary:
        archive = Path(temporary) / "source.tar.gz"
        with tarfile.open(archive, "w:gz") as tar:
            for name, facts in delta.items():
                actual = Path(before["generated"][name]["source"]) if name in before["generated"] else root / name
                raw = actual.read_bytes()
                if {"sha256": digest(raw), "bytes": len(raw)} != facts:
                    raise ValueError("go-input-changed-before-archive")
                item = tarfile.TarInfo(name); item.size = len(raw); item.mode = 0o644
                tar.addfile(item, io.BytesIO(raw))
        raw = archive.read_bytes()
        publish(output / "source.tar.gz", raw)
    frame = {"schema_version": 1, "files": retained,
             "project_paths": sorted(before["project"]), "generated_sources": before["generated"], "delta": sorted(delta),
             "archive_sha256": digest(raw), "baseline": None}
    if baseline_path is not None:
        frame["baseline"] = os.path.relpath(baseline_path, output)
        frame["baseline_sha256"] = digest((baseline_path / "source.json").read_bytes())
    publish(output / "source.json", frame)
    if sources(root, output) != retained:
        raise ValueError("go-input-archive-readback-incomplete")
    return {"root": root, "output": output, "package": package, "before": before}


def finish(context, execution, validated):
    """Retain execution metadata and verify input stability before a result.

    Caller supplies actual command/result and named-test validation; this helper
    proves input retention/stability, not that a caller is trustworthy. Changed
    inputs, failed command/validation or archive refusal cannot become PASS.
    No retry, watchdog, permission, independent acceptance or deployment follows.
    """
    output = context["output"]
    publish(output / "execution.json", execution)
    after = observe(context["root"], context["package"], output, "after")
    publish(output / "after.json", after)
    stable = after == context["before"]
    result = {"schema_version": 1, "status": "PASS" if stable and validated and execution["exit"] == 0 else "FAIL",
              "execution_sha256": digest((output / "execution.json").read_bytes()),
              "source_stable": stable, "named_checks_validated": validated,
              "project_files": len(after["project"]), "generated_files": len(after["generated"]),
              "source_manifest_sha256": digest((output / "source.json").read_bytes()),
              "scope": "Actual selected project Go inputs and supplied check outcome; not hermetic external C/system build, authenticity or independent acceptance",
              "go_flags_profile": "default; overlays/GOFLAGS overrides unsupported"}
    publish(output / "result.json", result)
    if result["status"] != "PASS":
        raise ValueError("go-input-check-or-stability-failed")
    return result
