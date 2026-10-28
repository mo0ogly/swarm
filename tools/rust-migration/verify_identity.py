"""Differential tests only; Go is an oracle, never a Rust runtime dependency."""
import datetime
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = Path(sys.argv[1]).resolve()
if OUTPUT.exists() and any(OUTPUT.iterdir()):
    raise SystemExit("Use a fresh evidence directory; preserve successful and failed attempts")
OUTPUT.mkdir(parents=True, exist_ok=True)
work = Path(tempfile.mkdtemp(prefix="swarm-rust-contract-"))
env = dict(os.environ, CARGO_TARGET_DIR=str(work / "cargo-target"))
records = []


def run(label, command, data=None):
    start = time.monotonic()
    result = subprocess.run(command, cwd=ROOT, env=env, input=data, capture_output=True)
    (OUTPUT / f"{label}.stdout").write_bytes(result.stdout)
    (OUTPUT / f"{label}.stderr").write_bytes(result.stderr)
    records.append({"label": label, "command": command, "exit": result.returncode, "seconds": time.monotonic() - start})
    (OUTPUT / "commands.json").write_text(json.dumps(records, indent=2) + "\n")
    print(label, "exit", result.returncode, flush=True)
    if result.returncode:
        raise RuntimeError(f"{label} failed; original outputs retained")
    return result.stdout


raw = run("go-dependencies", ["go", "list", "-deps", "-test", "-json", "./internal/identitycontract", "./tools/rust-migration/oracle"])
remaining = raw.decode().lstrip()
decoder = json.JSONDecoder()
inputs = {ROOT / "go.mod", ROOT / "go.sum"}
local_packages = []
while remaining:
    package, length = decoder.raw_decode(remaining)
    remaining = remaining[length:].lstrip()
    directory = Path(package.get("Dir", "/nonexistent"))
    if not directory.is_relative_to(ROOT):
        continue
    local_packages.append(package["ImportPath"])
    for key in ("GoFiles", "TestGoFiles", "XTestGoFiles", "EmbedFiles"):
        inputs.update(directory / name for name in package.get(key, []) if (directory / name).is_file())
assert "swarm.local/companion" not in local_packages, "root/embed dependency unexpectedly entered this leaf check"
for folder, extensions in ((ROOT / "rust", (".rs", ".toml", ".lock")), (ROOT / "tools/rust-migration", (".py", ".go"))):
    inputs.update(p for p in folder.rglob("*") if p.is_file() and p.suffix in extensions and "target" not in p.relative_to(folder).parts)


def input_key(path):
    if path.is_relative_to(ROOT):
        return str(path.relative_to(ROOT))
    # go list -test supplies an absolute generated test-main in the Go cache.
    # Retain those actual compiled bytes, not just their in-repository producers.
    return "generated-go/" + path.name


assert input_key(ROOT / "go.mod") == "go.mod"
assert input_key(Path("/outside/go-cache/generated-d")) == "generated-go/generated-d"


def snapshot():
    result = {input_key(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in sorted(inputs)}
    assert len(result) == len(inputs), "input evidence keys collided"
    return result


before = snapshot()
(OUTPUT / "inputs-before.json").write_text(json.dumps(before, indent=2) + "\n")
with tarfile.open(OUTPUT / "sources.tar.gz", "w:gz") as archive:
    for path in sorted(inputs):
        archive.add(path, arcname=input_key(path))
run("go-tests", ["go", "test", "-json", "-count=1", "-timeout=0", "./internal/identitycontract"])
run("go-vet", ["go", "vet", "./internal/identitycontract", "./tools/rust-migration/oracle"])
run("go-build-oracle", ["go", "build", "-o", str(work / "go-oracle"), "./tools/rust-migration/oracle"])
run("rust-format", ["cargo", "fmt", "--manifest-path", "rust/Cargo.toml", "--all", "--", "--check"])
run("rust-tests", ["cargo", "test", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "-p", "swarm-identity-contract", "--all-targets"])
run("rust-clippy", ["cargo", "clippy", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "-p", "swarm-identity-contract", "--all-targets", "--", "-D", "warnings"])
run("rust-build-probe", ["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--offline", "--locked", "--release", "-p", "swarm-identity-contract", "--example", "contract_probe"])
rust = str(work / "cargo-target/release/examples/contract_probe")
go = str(work / "go-oracle")
cases, expectations = [], {}


def add(operation, args, expected=None):
    fields = [operation] + [str(arg) if isinstance(arg, int) else arg.hex() for arg in args]
    if expected is not None:
        expectations[len(cases)] = b"1" if expected else b"0"
    cases.append("\t".join(fields))


controls = {"name": b"work-A_01", "text": "équipe[eu]".encode(), "digest": b"a" * 64}
faults = {"name": [b"", b"a" * 101, b"../work", b"work\0", "équipe".encode()], "text": [b"", b" a", b"a ", b"a\0b", b"a\n", b"\xff", b"\xc0\xaf", b"\xed\xa0\x80", b"\xf4\x90\x80\x80"], "digest": [b"", b"a" * 63, b"a" * 65, b"A" * 64, b"g" * 64]}
for operation, bad_values in faults.items():
    for value in bad_values:
        add(operation, [controls[operation]], True)
        add(operation, [value], False)
add("name", [b"a" * 100], True)
for a in range(256):
    add("name", [bytes([a])])
    add("text", [bytes([a])])
    for b in range(256):
        add("name", [bytes([a, b])])
        add("text", [bytes([a, b])])
principal = [b"human-1", b"human", b"https://[::1]/issuer", b"employee[eu]"]
for index in range(4):
    for value in (b"", b"../x", b" x", b"x\0", b"\xff", "équipe".encode(), b"urn:swarm:local"):
        add("principal", principal, True)
        changed = principal.copy()
        changed[index] = value
        add("principal", changed)
for kind in (b"human", b"service", b"local", b"admin"):
    for issuer in (b"urn:swarm:local", b"urn:swarm:human:installation-1"):
        add("principal", [b"human-1", kind, issuer, b"human-1"])
for version in (-2**63, -1, 0, 1, 2**63 - 1):
    add("scope", [b"work-1", 1, b"a" * 64], True)
    add("scope", [b"work-1", version, b"a" * 64], version > 0)
for operation in (b"agent.create", b"memory.read", b"memory.summarize", b"memory.transmit", b"tool.invoke", b"effect.execute", b"memory.write", b"*", b""):
    for resource in (b"", b"memory/1", b"memory/*", b"memory/?", b"memory/[eu]", b"memory/x]", b" memory", b"memory\0", "memory/équipe".encode()):
        add("permission", [b"memory.read", b"memory/1"], True)
        add("permission", [operation, resource])
payload = ("\n".join(cases) + "\n").encode()
(OUTPUT / "corpus.txt").write_bytes(payload)
go_results = run("go-corpus", [go], payload).splitlines()
rust_results = run("rust-corpus", [rust], payload).splitlines()
assert len(go_results) == len(cases) == len(rust_results), "missing or extra probe results"
assert all(value in (b"0", b"1") for value in go_results + rust_results), "non-boolean success output"
for index, expected in expectations.items():
    assert go_results[index] == expected == rust_results[index], ("curated contract", index)
assert go_results == rust_results, "Go/Rust data-contract mismatch"
go_unicode = run("go-unicode", [go, "--unicode"])
rust_unicode = run("rust-unicode", [rust, "--unicode"])
assert len(go_unicode) == len(rust_unicode) == 0x110000 - 0x800, "incomplete Unicode scalar domain"
assert go_unicode == rust_unicode, "Go/Rust Unicode behavior mismatch"
after = snapshot()
(OUTPUT / "inputs-after.json").write_text(json.dumps(after, indent=2) + "\n")
assert before == after, "tested source or inputs changed during verification"
go_pass = [json.loads(line)["Test"] for line in (OUTPUT / "go-tests.stdout").read_text().splitlines() if json.loads(line).get("Action") == "pass" and "Test" in json.loads(line)]
rust_tests = [line for line in (OUTPUT / "rust-tests.stdout").read_text().splitlines() if line.startswith("test ") and line.endswith(" ... ok")]
assert go_pass and len(rust_tests) == 7, "tests not actually collected"
result = {"status": "PASS", "date_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(), "scope": "identitycontract data validation only; not engine migration", "inputs": len(before), "go_named_passes": go_pass, "rust_named_passes": rust_tests, "differential_cases": len(cases), "curated_expectations": len(expectations), "unicode_scalars": len(go_unicode), "unicode_checks_per_scalar": 6, "unicode_table_sha256": hashlib.sha256(go_unicode).hexdigest(), "binaries_sha256": {"go_oracle": hashlib.sha256(Path(go).read_bytes()).hexdigest(), "rust_probe": hashlib.sha256(Path(rust).read_bytes()).hexdigest()}, "source_stable": True, "local_go_dependencies": local_packages, "temporary_fixture_root": str(work), "global_test_watchdog": "none; Go -timeout=0", "deployment": False, "independent_review": False}
(OUTPUT / "result.json").write_text(json.dumps(result, indent=2) + "\n")
print(json.dumps({key: result[key] for key in ("status", "inputs", "differential_cases", "unicode_scalars", "source_stable")}))
