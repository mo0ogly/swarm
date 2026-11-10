#!/usr/bin/env python3
"""Prepare M17 history from an explicit Swarm source export, without model calls."""
import argparse
import copy
import json
import os
from pathlib import Path
import sys

from compare import digest, load_json, unique_object
import summarization as S


def require(condition, message):
    if not condition:
        raise ValueError(message)


def parse(text):
    def invalid(_):
        raise ValueError("Non-finite JSON value")
    return json.loads(text, object_pairs_hook=unique_object, parse_constant=invalid)


def dialogue_source_id(ref):
    # Match json.Marshal(DialogueRef) in memory_summary_source.go, independent
    # of input object key order. Go also escapes HTML and the two JS separators.
    value = {key: ref[key] for key in ["task", "attempt", "sha256"]}
    raw = json.dumps(value, ensure_ascii=False, separators=(",", ":"))
    for char, escaped in [("&", "\\u0026"), ("<", "\\u003c"), (">", "\\u003e"),
                          ("\u2028", "\\u2028"), ("\u2029", "\\u2029")]:
        raw = raw.replace(char, escaped)
    return "dialogue-" + digest(raw.encode())


def prepare(packet, annotations):
    require(S.exact(packet, ["schema_version", "kind", "payload", "sha256"]) and type(packet["schema_version"]) is int and packet["schema_version"] == 1 and packet["kind"] == "swarm_summary_source_v1", "Invalid Swarm source envelope")
    require(isinstance(packet["payload"], str) and packet["sha256"] == digest(packet["payload"].encode()), "Source payload hash mismatch")
    payload = parse(packet["payload"])
    require(S.exact(payload, ["scope", "work_revision", "project_sha256", "audience", "provider_history_complete", "sources"]), "Invalid source payload fields")
    require(type(payload["work_revision"]) is int and payload["work_revision"] > 0 and S.hash_reference(payload["project_sha256"]), "Invalid source revision/project")
    require(payload["audience"] == "local_operator" and payload["provider_history_complete"] is False, "Unsupported source audience or completeness claim")
    scope = payload["scope"]
    require(S.exact(scope, ["work", "task"]) and all(S.label(v) for v in scope.values()), "Invalid source scope")
    sources = payload["sources"]
    require(isinstance(sources, list) and len(sources) >= 3 and len(sources) % 2 == 1, "Explicit task snapshot and selected exchange pairs required")
    ids, messages = set(), []
    for index, source in enumerate(sources):
        expected = ["id", "role", "kind", "text", "sha256"] + (["dialogue"] if index else [])
        require(S.exact(source, expected) and S.label(source["id"]) and source["id"] not in ids and S.label(source["text"]), "Invalid or duplicate source item")
        require(source["sha256"] == digest(source["text"].encode()), "Source text hash mismatch")
        ids.add(source["id"])
        kind, role = ("task_snapshot", "user") if index == 0 else (("operator_question", "user") if index % 2 else ("historical_answer", "assistant"))
        require(source["kind"] == kind and source["role"] == role, "Source role/kind mismatch")
        messages.append({"id": source["id"], "role": source["role"], "content": source["text"]})
    require(sources[0]["id"] == "current-task-snapshot", "Task snapshot source missing")
    snapshot = parse(sources[0]["text"])
    require(S.exact(snapshot, ["project_sha256", "work_title", "work_objective", "work_scope", "work_criteria", "task"]) and isinstance(snapshot["task"], dict), "Invalid task snapshot")
    require(snapshot["project_sha256"] == payload["project_sha256"] and snapshot["task"].get("id") == scope["task"] and snapshot["task"].get("brainstorm", False) is False, "Task snapshot scope mismatch")
    dialogues = set()
    for index in range(1, len(sources), 2):
        question, answer = sources[index:index+2]
        ref = question["dialogue"]
        require(S.exact(ref, ["task", "attempt", "sha256"]) and S.label(ref["task"]) and isinstance(ref["attempt"], str) and S.hash_reference(ref["sha256"]), "Invalid stored dialogue reference")
        require(ref == answer["dialogue"] and ref["sha256"] == digest((question["text"] + "\n" + answer["text"]).encode()), "Selected dialogue changed or pair mismatched")
        identity = (ref["task"], ref["attempt"], ref["sha256"])
        require(identity not in dialogues, "Stored dialogue selected more than once")
        dialogues.add(identity)
        source_id = dialogue_source_id(ref)
        require(question["id"] == source_id + "-question" and answer["id"] == source_id + "-answer",
                "Stored dialogue source identity mismatch")
    require(S.exact(annotations, ["schema_version", "source_sha256", "protected"]) and type(annotations["schema_version"]) is int and annotations["schema_version"] == 1 and annotations["source_sha256"] == packet["sha256"], "Annotations missing or stale for this source packet")
    history = {"schema_version": 1, "scope": copy.deepcopy(scope), "messages": messages, "protected": copy.deepcopy(annotations["protected"])}
    S.validate_history(history)
    require(any(anchor["message"] == "current-task-snapshot" for anchor in history["protected"]), "Current task snapshot must be protected explicitly")
    provenance = {"schema_version": 1, "kind": "swarm_summary_source_binding_v1", "scope": copy.deepcopy(scope), "project_sha256": payload["project_sha256"], "work_revision": payload["work_revision"], "source_payload_sha256": packet["sha256"], "history_sha256": S.checksum(history), "annotations_sha256": S.checksum(annotations), "provider_history_complete": False, "audience": "local_operator", "model_effectiveness": "NOT TESTED", "engine_admission": False}
    return history, provenance


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--annotations", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    args = parser.parse_args(argv)
    try:
        source_raw, annotations_raw = args.source.read_bytes(), args.annotations.read_bytes()
        # Parse the exact bytes that will be retained, with no second file read.
        packet, annotations = parse(source_raw.decode()), parse(annotations_raw.decode())
        history, provenance = prepare(packet, annotations)
        files = {"source-packet.json": source_raw, "annotations.json": annotations_raw,
                 "history.json": (json.dumps(history, ensure_ascii=False, indent=2) + "\n").encode()}
        provenance["files_sha256"] = {name: digest(raw) for name, raw in files.items()}
        args.output_dir.mkdir(mode=0o700, exist_ok=False)
        for name, raw in [*files.items(), ("provenance.json", (json.dumps(provenance, ensure_ascii=False, indent=2) + "\n").encode())]:
            fd = os.open(args.output_dir / name, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(fd, "wb") as file:
                file.write(raw)
        print(json.dumps({"result": "PASS", "sources": len(history["messages"]), "history_sha256": provenance["history_sha256"], "model_calls": 0, "engine_admission": False}))
        return 0
    except (OSError, ValueError, TypeError, KeyError) as error:
        print("Swarm summary source error: " + str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
