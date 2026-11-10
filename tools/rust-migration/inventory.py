"""Read-only migration inventory; retain dirty checkout and actual Linux embeds."""
import collections
import datetime
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
output = Path(sys.argv[1]).resolve()
if any((output / name).exists() for name in ("go-list.stdout.json", "go-list.stderr", "inventory.json")):
    raise SystemExit("Use a fresh inventory output; preserve previous observations and failures")
output.mkdir(parents=True, exist_ok=True)
files = sorted([*ROOT.glob("internal/**/*.go"), *ROOT.glob("cmd/**/*.go"), ROOT / "resources.go"])
sources = {}
for path in files:
    raw = path.read_bytes()
    sources[str(path.relative_to(ROOT))] = {"sha256": hashlib.sha256(raw).hexdigest(), "lines": raw.count(b"\n")}
changed = [str(path.relative_to(ROOT)) for path in files if hashlib.sha256(path.read_bytes()).hexdigest() != sources[str(path.relative_to(ROOT))]["sha256"]]
listed = subprocess.run(["go", "list", "-deps", "-json", "./cmd/swarm"], cwd=ROOT, capture_output=True)
(output / "go-list.stdout.json").write_bytes(listed.stdout)
(output / "go-list.stderr").write_bytes(listed.stderr)
if listed.returncode:
    raise SystemExit("Go dependency inventory failed; originals retained")
decoder = json.JSONDecoder()
remaining = listed.stdout.decode().lstrip()
packages = []
while remaining:
    package, length = decoder.raw_decode(remaining)
    packages.append(package)
    remaining = remaining[length:].lstrip()
local = [p for p in packages if p.get("Module", {}).get("Main")]
assets = {}
for package in local:
    for name in package.get("EmbedFiles", []):
        path = Path(package["Dir"]) / name
        assets[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
product = [p for p in sources if not p.endswith("_test.go")]
tests = [p for p in sources if p.endswith("_test.go")]
report = {
    "date_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "scope": "internal/**/*.go + cmd/**/*.go + resources.go; historical Go evidence copies excluded",
    "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
    "dirty_status": subprocess.check_output(["git", "status", "--porcelain=v1"], cwd=ROOT, text=True).splitlines(),
    "production_files": len(product),
    "production_lines": sum(sources[p]["lines"] for p in product),
    "test_files": len(tests),
    "test_lines": sum(sources[p]["lines"] for p in tests),
    "linux_product_files": sum(p.endswith("_linux.go") for p in product),
    "local_compiled_packages": [{"import_path": p["ImportPath"], "go_files": p.get("GoFiles", [])} for p in local],
    "source_sha256_and_lines": sources,
    "embedded_asset_sha256": assets,
    "source_changes_during_initial_read": changed,
    "adoption": "inventory only; not a frozen migration candidate or parity proof",
}
(output / "inventory.json").write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
print(json.dumps({k: report[k] for k in ("production_files", "production_lines", "test_files", "test_lines", "linux_product_files")}))
print("Embedded inputs:", len(assets), "; compiled local packages:", len(local))
