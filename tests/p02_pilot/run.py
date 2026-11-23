"""Build the fixture sender, run two local journeys and verify the evidence."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
from tempfile import TemporaryDirectory
from uuid import uuid4
from hashlib import file_digest, sha256

import governance
import transport

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests"))
from fixture_area import fixture_area


def executable():
    """Resolve the exact executable and engine features observed by offline Cargo."""
    command = [
        "cargo", "build", "--offline", "--locked", "--manifest-path", "rust/Cargo.toml",
        "-p", "swarm-engine", "--features", "test-fixtures", "--example", "p02_pilot_sender",
        "--message-format=json",
    ]
    process = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, text=True, check=True)
    rows = [json.loads(line) for line in process.stdout.splitlines()]
    paths = [
        row["executable"] for row in rows
        if row.get("reason") == "compiler-artifact" and row["target"]["name"] == "p02_pilot_sender"
        and row["target"]["kind"] == ["example"] and row.get("executable")
    ]
    engines = [
        row["features"] for row in rows
        if row.get("reason") == "compiler-artifact" and row["target"]["name"] == "swarm_engine"
        and row["target"]["kind"] == ["lib"] and not row["profile"]["test"]
    ]
    if len(paths) != 1 or len(engines) != 1 or "test-fixtures" not in engines[0]:
        raise ValueError("fixture-build-artifact-mismatch")
    path = Path(paths[0]).resolve(strict=True)
    if not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError("fixture-build-artifact-not-executable")
    return path, engines[0], command


def binary_hash(path):
    """Return the actual fixture executable digest, checked before and after both runs."""
    with path.open("rb") as stream:
        return file_digest(stream, sha256).hexdigest()


def verify(record):
    """Reopen retained JSON and independently recompute all governance transitions."""
    run = record["run"]
    qualification = record["qualification"]
    acts = record["acts"]
    admitted = governance.admission(run, qualification, acts)
    launched = governance.launch(run, record["admission"], admitted)
    if record["launch"] != launched:
        raise ValueError("launch-readback-mismatch")
    closed = governance.closure(run, record["launch"], launched, qualification, record["pilot"])
    governance.verify_closure(record["closure"], closed)
    rejected = governance.negative_controls(run, qualification, acts, admitted, launched,
                                           record["pilot"], closed)
    return dict(status="VERIFIED", signed_test_acts=len(acts), qualifications=len(qualification["cases"]),
                pilot_cases=closed["technical_cases"], refused_controls=rejected,
                remaining_rows=closed["rows_after_purge"], real_execution_authorized=False)


def main():
    """Run qualification before approval, then replay only in a fresh local fixture."""
    if not __debug__:
        raise SystemExit("fixture assertions must be enabled")
    sender, features, command = executable()
    before = binary_hash(sender)
    run = str(uuid4())
    with TemporaryDirectory(prefix="p02-pilot-", dir=fixture_area()) as temporary:
        root = Path(temporary)
        qualified = root / "qualification"
        qualified.mkdir()
        qualification = transport.exercise(qualified, sender)
        acts = governance.fixture_acts(run, qualification)
        admitted = governance.admission(run, qualification, acts)
        launched = governance.launch(run, admitted, admitted)
        fixture = root / "pilot"
        fixture.mkdir()
        pilot = transport.exercise(fixture, sender)
        closed = governance.closure(run, launched, launched, qualification, pilot)
        if binary_hash(sender) != before:
            raise ValueError("fixture-executable-changed")
        record = dict(run=run, qualification=qualification, acts=acts, admission=admitted,
                      launch=launched, pilot=pilot, closure=closed,
                      build={"command": command, "engine_features": features, "executable_sha256": before})
        receipt = root / "closure.json"
        receipt.write_bytes(governance.encode(record))
        reopened = json.loads(receipt.read_bytes())
        verification = verify(reopened)
        print(json.dumps({"evidence": reopened, "verification": verification}, sort_keys=True))
    print("P02 pilot fixture: {qualifications} qualification cases, {pilot_cases} pilot cases, "
          "{signed_test_acts} acts, {refusals} refusals, {remaining_rows} rows after purge".format(
              refusals=len(verification["refused_controls"]), **verification))


def verify_output(path):
    """Check the retained standard output without building or invoking a sender."""
    evidence_line, _ = path.read_text().splitlines()
    output = json.loads(evidence_line)
    actual = verify(output["evidence"])
    if actual != output["verification"]:
        raise ValueError("verification-readback-mismatch")
    print(json.dumps(actual, sort_keys=True))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--verify-output", type=Path, help="verify a retained step stdout without running fixtures")
    args = parser.parse_args()
    if args.verify_output is None:
        main()
    else:
        verify_output(args.verify_output)
