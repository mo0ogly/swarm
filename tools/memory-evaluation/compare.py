#!/usr/bin/env python3
"""Grade supplied decisions against frozen engine-preview fixtures; calls no model."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
from pathlib import Path


class InvalidComparison(ValueError):
    pass


def digest(raw: bytes) -> str:
    return hashlib.sha256(raw).hexdigest()


def unique_object(pairs):
    value = {}
    for key, item in pairs:
        if key in value:
            raise InvalidComparison("Duplicate JSON field: " + key)
        value[key] = item
    return value


def load_json(path: Path):
    def invalid_number(_):
        raise InvalidComparison("Non-finite JSON number")
    return json.loads(path.read_text(), object_pairs_hook=unique_object,
                      parse_constant=invalid_number)


def expected(facts: dict) -> dict:
    """Decide from current observations, never the historical accepted status."""
    if not isinstance(facts, dict):
        raise InvalidComparison("Current observations must be an object")
    required = facts.get("required_sha256")
    if not isinstance(required, str) or len(required) != 64 or any(c not in "0123456789abcdef" for c in required):
        raise InvalidComparison("Invalid required artifact hash")
    if facts.get("target_state") not in ("missing", "different") or type(facts.get("archive_available")) is not bool:
        raise InvalidComparison("Unsupported current observations")
    if facts["target_state"] == "missing" and facts.get("target_sha256") is not None:
        raise InvalidComparison("Missing target cannot have a current hash")
    if facts["target_state"] == "different" and (not facts.get("target_sha256") or facts["target_sha256"] == required):
        raise InvalidComparison("Different target must have a different hash")
    if not facts["archive_available"] and facts.get("archive_sha256") is not None:
        raise InvalidComparison("Unavailable archive cannot have a verified hash")
    recoverable = (facts["target_state"] == "missing" and facts["archive_available"]
                   and facts.get("archive_sha256") == required)
    return {"decision": "restore_and_evaluate" if recoverable else "rebuild_evidence",
            "actions": ["verify_archive", "restore_artifact", "evaluate_current"] if recoverable else ["block_admission", "request_fresh_evidence"],
            "admission": "after_fresh_pass" if recoverable else "blocked",
            "acceptance": "unchanged"}


def read_pack(root: Path, facts_validator=expected, pair_count=2) -> tuple[dict, str]:
    root = root.resolve()
    path = root / "manifest.json"
    pack = load_json(path)
    if type(pair_count) is not int or pair_count < 1:
        raise InvalidComparison("Positive matched pair count required")
    if not isinstance(pack, dict) or type(pack.get("schema_version")) is not int or pack["schema_version"] != 1 or not isinstance(pack.get("cases"), list) or len(pack["cases"]) != 2 * pair_count:
        raise InvalidComparison("Unexpected number of version-1 cases")
    ids, pairs = set(), {}
    for case in pack["cases"]:
        if not isinstance(case, dict) or not isinstance(case.get("scenario"), str) or not case["scenario"]:
            raise InvalidComparison("Case and scenario must be explicit")
        if not isinstance(case.get("id"), str) or not case["id"] or case["id"] in ids:
            raise InvalidComparison("Missing or duplicate case ID")
        ids.add(case["id"])
        def read_local(name):
            prompt = root / name
            if prompt.is_symlink() or not prompt.resolve().is_relative_to(root):
                raise InvalidComparison("Prompt must be a local regular file")
            return prompt.read_bytes()
        raw = read_local(case["raw_prompt"])
        normalized = read_local(case["prompt"])
        context = case.get("context", {})
        if digest(normalized) != case.get("prompt_sha256") or context.get("sha256") != digest(raw) or context.get("bytes") != len(raw):
            raise InvalidComparison("Prompt or context manifest changed: " + case["id"])
        if case.get("normalization") != "work-revision-only-v1" or type(case.get("source_revision")) is not int:
            raise InvalidComparison("Unknown input normalization")
        needle = (" ; révision du travail %d.\n" % case["source_revision"]).encode()
        if raw.count(needle) != 1 or raw.replace(needle, " ; révision du travail [neutralisée pour comparaison].\n".encode(), 1) != normalized:
            raise InvalidComparison("Input changed beyond the declared revision normalization")
        facts_validator(case["facts"])
        treatment = case.get("treatment")
        if treatment not in ("with", "without"):
            raise InvalidComparison("Unknown treatment")
        pair = pairs.setdefault(case["scenario"], {})
        if treatment in pair:
            raise InvalidComparison("Duplicate treatment in a pair")
        pair[treatment] = case
    if len(pairs) != pair_count or any(set(pair) != {"with", "without"} for pair in pairs.values()):
        raise InvalidComparison("Expected complete matched pairs")
    for pair in pairs.values():
        if pair["with"]["facts"] != pair["without"]["facts"]:
            raise InvalidComparison("Paired current observations differ")
    return pack, digest(path.read_bytes())


def grade(pack: dict, answers: dict) -> dict:
    if not isinstance(answers, dict) or type(answers.get("schema_version")) is not int or answers["schema_version"] != 1 or not isinstance(answers.get("source"), dict) or answers["source"].get("kind") not in ("fixture", "supplied_outputs"):
        raise InvalidComparison("Declare version and answer provenance")
    if answers["source"]["kind"] == "supplied_outputs" and any(not isinstance(answers["source"].get(key), str) or not answers["source"][key].strip() for key in ("provider", "model")):
        raise InvalidComparison("Declare the provider and model of supplied outputs")
    entries = answers.get("answers")
    if not isinstance(entries, list) or len(entries) != len(pack["cases"]):
        raise InvalidComparison("Every case needs exactly one answer")
    by_id = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("case_id"), str) or entry["case_id"] in by_id:
            raise InvalidComparison("Invalid or duplicate answer ID")
        by_id[entry["case_id"]] = entry
    if set(by_id) != {case["id"] for case in pack["cases"]}:
        raise InvalidComparison("Answers do not match the frozen cases")
    results = []
    for case in pack["cases"]:
        entry = by_id[case["id"]]
        if entry.get("prompt_sha256") != case["prompt_sha256"]:
            raise InvalidComparison("Answer is not bound to its prompt: " + case["id"])
        measurements = {}
        for key in ("elapsed_seconds", "input_tokens", "output_tokens", "cost"):
            value = entry.get(key)
            allowed = (int,) if key.endswith("tokens") else (int, float)
            if value is not None and (type(value) not in allowed or not math.isfinite(value) or value < 0):
                raise InvalidComparison("Invalid measurement: " + key)
            measurements[key] = value
        response = entry.get("response")
        required_keys = {"decision", "actions", "admission", "acceptance", "reason"}
        invalid = (not isinstance(response, dict) or set(response) != required_keys)
        if not invalid:
            invalid = (not isinstance(response["reason"], str) or not response["reason"].strip()
                       or not isinstance(response["actions"], list)
                       or any(not isinstance(x, str) for x in response["actions"])
                       or any(not isinstance(response[key], str) for key in ("decision", "admission", "acceptance")))
        violations = ["Response schema incomplete or invalid"] if invalid else [key for key, value in expected(case["facts"]).items() if response[key] != value]
        results.append({"case_id": case["id"], "scenario": case["scenario"], "treatment": case["treatment"],
                        "result": "INVALID" if invalid else "FAIL" if violations else "PASS",
                        "violations": violations, "measurements": measurements})
    comparisons = []
    for scenario in sorted({c["scenario"] for c in results}):
        pair = {r["treatment"]: r["result"] for r in results if r["scenario"] == scenario}
        change = "same" if (pair["with"] == "PASS") == (pair["without"] == "PASS") else "improvement" if pair["with"] == "PASS" else "regression"
        comparisons.append({"scenario": scenario, **pair, "observed_answer_change": change})
    return {"schema_version": 1, "source": answers["source"], "results": results, "comparisons": comparisons,
            "result": "PASS" if all(r["result"] == "PASS" for r in results) else "FAIL",
            "model_effectiveness": "NOT TESTED" if answers["source"]["kind"] == "fixture" else "UNPROVEN: supplied outputs alone do not establish provider provenance or general effectiveness",
            "scope": "Grades proposed decisions only; no autonomous execution or statistical improvement established"}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--pack", type=Path, required=True)
    parser.add_argument("--answers", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        pack, pack_hash = read_pack(args.pack)
        result = grade(pack, load_json(args.answers))
        result.update(pack_sha256=pack_hash, answers_sha256=digest(args.answers.read_bytes()))
        with args.output.open("x") as target:
            json.dump(result, target, ensure_ascii=False, indent=2)
            target.write("\n")
        print(json.dumps({"result": result["result"], "cases": len(result["results"]), "model_effectiveness": result["model_effectiveness"]}))
        return 0 if result["result"] == "PASS" else 1
    except (InvalidComparison, ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps({"result": "ERROR", "reason": str(error)}))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
