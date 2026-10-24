"""Qualify the native proof protocol; never open mission state or call a provider."""
import datetime
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = Path(sys.argv[1]).resolve()
if OUTPUT.exists() and any(OUTPUT.iterdir()):
    raise SystemExit("Fresh evidence directory required; preserve previous attempts")
OUTPUT.mkdir(parents=True, exist_ok=True)
WORK = Path(tempfile.mkdtemp(prefix="swarm-rust-proof-"))
ENV = dict(os.environ, CARGO_TARGET_DIR=str(WORK / "target"))
records = []


def run(label, command, data=None, expected=0):
    started = datetime.datetime.now(datetime.timezone.utc).isoformat()
    clock = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, env=ENV, input=data, capture_output=True)
    (OUTPUT / f"{label}.stdout").write_bytes(result.stdout)
    (OUTPUT / f"{label}.stderr").write_bytes(result.stderr)
    records.append({"label": label, "command": command, "start_utc": started,
                    "end_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                    "seconds": time.monotonic() - clock, "exit": result.returncode, "expected_exit": expected})
    (OUTPUT / "commands.json").write_text(json.dumps(records, indent=2) + "\n")
    print(label, "exit", result.returncode, flush=True)
    if result.returncode != expected:
        raise RuntimeError(f"{label}: unexpected exit; original outputs retained")
    return result


def json_stream(raw):
    remaining = raw.decode().lstrip()
    decoder = json.JSONDecoder()
    while remaining:
        value, length = decoder.raw_decode(remaining)
        yield value
        remaining = remaining[length:].lstrip()


def qualify():
    go_environment = json.loads(run("go-environment", ["go", "env", "-json", "GOROOT", "GOOS", "GOARCH", "GOFLAGS", "GOWORK"]).stdout)
    if "overlay" in go_environment["GOFLAGS"] or go_environment["GOWORK"]:
        raise RuntimeError("A Go overlay/workspace requires an effective-input collector before qualification")
    rust_version = run("rust-version", ["rustc", "-vV"]).stdout.decode()
    host = next(line.split(": ", 1)[1] for line in rust_version.splitlines() if line.startswith("host: "))
    run("go-version", ["go", "version"])
    run("cargo-version", ["cargo", "--version"])
    inputs = {ROOT / "go.mod", ROOT / "go.sum"}
    for folder, endings in ((ROOT / "rust", (".rs", ".toml", ".lock")), (ROOT / "tools/rust-migration", (".go", ".py"))):
        inputs.update(path for path in folder.rglob("*") if path.is_file() and path.suffix in endings and "target" not in path.relative_to(folder).parts)
    for label, flags in (("normal", []), ("race", ["-race"])):
        listed = run("go-inputs-" + label, ["go", "list", *flags, "-compiled", "-deps", "-test", "-json", "./internal/enrollmentproof", "./internal/identitycontract", "./tools/rust-migration/proof-oracle"])
        for package in json_stream(listed.stdout):
            if package["ImportPath"] == "swarm.local/companion":
                raise RuntimeError("Unexpected root/embed dependency: scope requires reassessment")
            directory = Path(package.get("Dir", "/nonexistent"))
            for field in ("GoFiles", "CompiledGoFiles", "TestGoFiles", "XTestGoFiles", "CgoFiles", "CFiles", "CXXFiles", "HFiles", "SFiles", "SysoFiles", "EmbedFiles"):
                inputs.update(directory / name for name in package.get(field, []) if (directory / name).is_file())
    metadata = json.loads(run("cargo-metadata", ["cargo", "metadata", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "--all-features", "--format-version", "1", "--filter-platform", host]).stdout)
    lock = tomllib.loads((ROOT / "rust/Cargo.lock").read_text())
    checksums = {(p["name"], p["version"]): p.get("checksum") for p in lock["package"]}
    package_roots = {}
    registry = []
    crate_archives = OUTPUT / "registry-crates"
    crate_archives.mkdir()
    for package in metadata["packages"]:
        if not package["source"]:
            continue
        directory = Path(package["manifest_path"]).parent
        name = f"{package['name']}-{package['version']}"
        cache = directory.parents[2] / "cache" / directory.parent.name / f"{name}.crate"
        raw = cache.read_bytes()
        digest = hashlib.sha256(raw).hexdigest()
        assert digest == checksums[(package["name"], package["version"])], (name, "locked package archive mismatch")
        shutil.copyfile(cache, crate_archives / cache.name)
        package_roots[directory] = "registry/" + name
        count = 0
        with tarfile.open(cache) as archive:
            for member in archive.getmembers():
                if not member.isfile():
                    continue
                parts = PurePosixPath(member.name).parts
                assert parts[0] == name and ".." not in parts, "invalid registry archive path"
                path = directory.joinpath(*parts[1:])
                expected_hash = hashlib.sha256(archive.extractfile(member).read()).hexdigest()
                assert hashlib.sha256(path.read_bytes()).hexdigest() == expected_hash, (name, member.name, "modified cached source")
                inputs.add(path)
                count += 1
        registry.append({"name": package["name"], "version": package["version"], "crate_sha256": digest, "actual_files": count})
    goroot = Path(go_environment["GOROOT"])

    def input_key(path):
        if path.is_relative_to(ROOT):
            return str(path.relative_to(ROOT))
        if path.is_relative_to(goroot):
            return "go-toolchain/" + str(path.relative_to(goroot))
        for directory, prefix in package_roots.items():
            if path.is_relative_to(directory):
                return prefix + "/" + str(path.relative_to(directory))
        return "generated-go/" + hashlib.sha256(str(path).encode()).hexdigest() + "/" + path.name

    def snapshot():
        result = {input_key(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(inputs)}
        assert len(result) == len(inputs), "input evidence key collision"
        return result

    before = snapshot()
    (OUTPUT / "inputs-before.json").write_text(json.dumps(before, indent=2) + "\n")
    (OUTPUT / "registry.json").write_text(json.dumps(registry, indent=2) + "\n")
    with tarfile.open(OUTPUT / "sources.tar.gz", "w:gz") as archive:
        for path in sorted(inputs):
            if not any(path.is_relative_to(directory) for directory in package_roots):
                archive.add(path, arcname=input_key(path))
    run("go-tests", ["go", "test", "-json", "-count=1", "-timeout=0", "./internal/enrollmentproof", "./internal/identitycontract"])
    run("go-race", ["go", "test", "-race", "-json", "-count=1", "-timeout=0", "./internal/enrollmentproof", "-run", "^(TestFreshChallengeAndConcurrentReplay|TestChallengeValueCopyCannotDuplicateConsumption)$"])
    run("go-vet", ["go", "vet", "./internal/enrollmentproof", "./internal/identitycontract", "./tools/rust-migration/proof-oracle"])
    run("rust-format", ["cargo", "fmt", "--manifest-path", "rust/Cargo.toml", "--all", "--", "--check"])
    run("rust-default", ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "-p", "swarm-identity-contract", "-p", "swarm-wire-json", "-p", "swarm-enrollment-proof", "--all-targets"])
    run("rust-fixtures", ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "-p", "swarm-identity-contract", "-p", "swarm-wire-json", "-p", "swarm-enrollment-proof", "--all-features", "--all-targets"])
    run("rust-clippy", ["cargo", "clippy", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "-p", "swarm-identity-contract", "-p", "swarm-wire-json", "-p", "swarm-enrollment-proof", "--all-features", "--all-targets", "--", "-D", "warnings"])
    run("rust-build", ["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "--release", "-p", "swarm-enrollment-proof", "--features", "test-fixtures", "--example", "proof_probe"])
    go = WORK / "go-oracle"
    rust = WORK / "target/release/examples/proof_probe"
    run("go-build", ["go", "build", "-o", str(go), "./tools/rust-migration/proof-oracle"])
    vectors_raw = run("go-vectors", [str(go)]).stdout
    rust_raw = run("rust-results", [str(rust)], vectors_raw).stdout
    vectors = [json.loads(line) for line in vectors_raw.splitlines()]
    results = [json.loads(line) for line in rust_raw.splitlines()]
    assert len(vectors) == len(results) and vectors, "missing or extra differential results"
    mismatches = []
    for vector, result in zip(vectors, results):
        if result["name"] != vector["name"] or result["code"] != vector["expected"] or result["recovery"] != vector["recovery"] or result["replay"] != vector["replay"] or not all(result[key] for key in ("signature_equal", "payload_equal", "request_equal")):
            mismatches.append({"vector": vector, "actual": result})
    (OUTPUT / "mismatches.json").write_text(json.dumps(mismatches, indent=2) + "\n")
    assert not mismatches, "Go/Rust protocol divergence; originals retained"
    go_strings = json.loads(run("go-json", [str(go), "--json-table"]).stdout)
    rust_strings = json.loads(run("rust-json", [str(rust), "--json-table"]).stdout)
    assert go_strings == rust_strings and go_strings["string_cases"] == 1243136, "JSON string-domain divergence or incomplete domain"
    # Positive compile control before proving test-only entropy is absent by default.
    client = WORK / "default-client"
    (client / "src").mkdir(parents=True)
    (client / "Cargo.toml").write_text('[package]\nname="proof-default-control"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nswarm-enrollment-proof={path=' + json.dumps(str(ROOT / "rust/crates/swarm-enrollment-proof")) + '}\n')
    (client / "src/lib.rs").write_text('pub fn control(v: &swarm_enrollment_proof::Verifier) { let _ = v.new_challenge([0;32]); }\n')
    shutil.copyfile(client / "src/lib.rs", OUTPUT / "feature-control.rs")
    run("feature-lock", ["cargo", "generate-lockfile", "--manifest-path", str(client / "Cargo.toml"), "--offline"])
    run("feature-control", ["cargo", "check", "--manifest-path", str(client / "Cargo.toml"), "--offline", "--locked"])
    (client / "src/lib.rs").write_text('pub fn denied(v: &swarm_enrollment_proof::Verifier) { let _ = v.challenge_with_reader([0;32], &mut std::io::Cursor::new([0;32])); }\n')
    shutil.copyfile(client / "src/lib.rs", OUTPUT / "feature-denied.rs")
    denied = run("feature-denied", ["cargo", "check", "--manifest-path", str(client / "Cargo.toml"), "--offline", "--locked"], expected=101)
    assert b"E0599" in denied.stderr and b"challenge_with_reader" in denied.stderr, "feature rejection caused by the wrong prerequisite"
    shutil.copyfile(client / "Cargo.lock", OUTPUT / "feature-client.Cargo.lock")
    after = snapshot()
    (OUTPUT / "inputs-after.json").write_text(json.dumps(after, indent=2) + "\n")
    assert before == after, "source changed during qualification"
    go_passes = [event["Test"] for event in map(json.loads, (OUTPUT / "go-tests.stdout").read_text().splitlines()) if event.get("Action") == "pass" and "Test" in event]
    race_passes = [event["Test"] for event in map(json.loads, (OUTPUT / "go-race.stdout").read_text().splitlines()) if event.get("Action") == "pass" and "Test" in event]
    rust_passes = [line for line in (OUTPUT / "rust-fixtures.stdout").read_text().splitlines() if line.startswith("test ") and line.endswith(" ... ok")]
    assert go_passes and len(race_passes) == 2 and len(rust_passes) == 25, "tests not actually collected"
    report = {"status": "PASS", "scope": "R2.1 native enrollment proof; no live authority or Swarm engine cutover", "inputs": len(before), "registry_packages": len(registry), "go_named_passes": go_passes, "go_race_named_passes": race_passes, "rust_named_passes": rust_passes, "protocol_cases": len(vectors), "json_strings": go_strings, "test_entropy_absent_from_default": True, "source_stable": True, "binary_sha256": {"go_oracle": hashlib.sha256(go.read_bytes()).hexdigest(), "rust_probe": hashlib.sha256(rust.read_bytes()).hexdigest()}, "temporary_fixture_root": str(WORK), "global_watchdog": "none; Go -timeout=0", "deployment": False, "independent_review": False}
    (OUTPUT / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps({key: report[key] for key in ("status", "inputs", "registry_packages", "protocol_cases", "source_stable")}))


try:
    qualify()
except Exception as error:
    (OUTPUT / "failure.json").write_text(json.dumps({"type": type(error).__name__, "error": str(error), "stage_count": len(records)}, indent=2) + "\n")
    raise
