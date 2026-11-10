#!/usr/bin/env python3
"""Prepare a declarative engine selection from verified M18 sources and M17 reference."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys

import summarization as S
import swarm_source as B


def selection(packet, annotations, reference, reason):
    history, _ = B.prepare(packet, annotations)
    B.require(S.label(reason), "Explicit operator reason required")
    B.require(isinstance(reference, dict) and reference.get("parent_sha256") is None,
              "Only a first extractive reference is supported for engine selection")
    candidate = {"schema_version": 1, "source_sha256": reference.get("source_sha256"),
                 "cutoff": reference.get("cutoff"), "parent_sha256": None, "status": "ok",
                 "entries": reference.get("summary_entries")}
    expected = S.project(history, candidate)
    B.require(expected["result"] == "PASS" and S.checksum(reference) == S.checksum(expected),
              "Reference receipt changed, refused or unrelated to the supplied sources")
    protected = history["protected"]
    prefix = {m["id"] for m in history["messages"][1:reference["cutoff"]]}
    B.require(any(anchor["message"] in prefix for anchor in protected),
              "Protect at least one dialogue message in the selected prefix")
    payload = B.parse(packet["payload"])
    return {"schema_version": 1, "source_sha256": packet["sha256"],
            "dialogues": [copy.deepcopy(source["dialogue"]) for source in payload["sources"][1::2]],
            "cutoff": reference["cutoff"], "protected": copy.deepcopy(protected), "reason": reason}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["source", "annotations", "reference", "output"]:
        parser.add_argument("--" + name, type=Path, required=True)
    parser.add_argument("--reason", required=True)
    args = parser.parse_args(argv)
    try:
        values = [B.parse(path.read_bytes().decode()) for path in [args.source, args.annotations, args.reference]]
        result = selection(*values, args.reason)
        fd = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(fd, "w", encoding="utf-8") as file:
            file.write(json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False) + "\n")
        print(json.dumps({"result": "SELECTION_PREPARED", "engine_admission": False, "model_calls": 0}))
        return 0
    except (OSError, ValueError, TypeError, KeyError) as error:
        print("Extractive selection error: " + str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
