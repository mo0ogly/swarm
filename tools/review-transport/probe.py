#!/usr/bin/env python3
"""Offline, lossless transport experiment. Never calls a provider or the Store.

Outputs sizes only. The framing is a prototype, NOT an accepted Swarm protocol.
"""
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path


def canonical(value):
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"), allow_nan=False).encode("utf-8")


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def encode(value):
    body = copy.deepcopy(value)
    replacements, blocks, identities = [], [], {}

    def visit(node, path):
        if isinstance(node, str) and len(node.encode("utf-8")) >= 128:
            raw = node.encode("utf-8")
            key = digest(raw)
            if key not in identities:
                identities[key] = len(blocks)
                blocks.append(raw)
            index = identities[key]
            if blocks[index] != raw:
                raise ValueError("hash collision")
            replacements.append({"path": path, "block": index})
            return ""
        if isinstance(node, dict):
            return {key: visit(item, path + [key]) for key, item in node.items()}
        if isinstance(node, list):
            return [visit(item, path + [i]) for i, item in enumerate(node)]
        return node

    body = visit(body, [])
    header = {"prototype": "lossless-text-pool-v1", "canonical_sha256": digest(canonical(value)),
              "body": body, "replacements": replacements, "block_count": len(blocks)}
    parts = [canonical(header), b"\n"]
    for block in blocks:
        parts.extend([canonical({"bytes": len(block), "sha256": digest(block)}), b"\n", block, b"\n"])
    return b"".join(parts)


def decode(raw):
    stream = io.BytesIO(raw)
    header = json.loads(stream.readline())
    if header.get("prototype") != "lossless-text-pool-v1":
        raise ValueError("unknown prototype")
    count = header["block_count"]
    if type(count) is not int or count < 0 or count > len(raw):
        raise ValueError("invalid block count")
    blocks = []
    for _ in range(count):
        item = json.loads(stream.readline())
        size = item["bytes"]
        if type(size) is not int or size < 0 or size > len(raw):
            raise ValueError("invalid byte count")
        block = stream.read(size)
        if len(block) != size or stream.read(1) != b"\n" or digest(block) != item["sha256"]:
            raise ValueError("truncated or substituted block")
        blocks.append(block.decode("utf-8"))
    if stream.read():
        raise ValueError("trailing bytes")
    body, seen, used = header["body"], set(), set()
    for replacement in header["replacements"]:
        path, index = replacement["path"], replacement["block"]
        if not isinstance(path, list) or any(type(k) not in (int, str) for k in path):
            raise ValueError("invalid path")
        key = tuple(path)
        if key in seen or type(index) is not int or index < 0 or index >= len(blocks):
            raise ValueError("duplicate path or unknown block")
        seen.add(key)
        used.add(index)
        if not path:
            if body != "":
                raise ValueError("nonempty root placeholder")
            body = blocks[index]
            continue
        parent = body
        for part in path[:-1]:
            if isinstance(parent, list) and (type(part) is not int or part < 0):
                raise ValueError("invalid list index")
            parent = parent[part]
        last = path[-1]
        if isinstance(parent, list) and (type(last) is not int or last < 0):
            raise ValueError("invalid list index")
        if parent[last] != "":
            raise ValueError("nonempty placeholder")
        parent[last] = blocks[index]
    if used != set(range(count)) or digest(canonical(body)) != header["canonical_sha256"]:
        raise ValueError("missing or substituted evidence")
    return body


def measure(value):
    raw, wire = canonical(value), encode(value)
    if decode(wire) != value:
        raise ValueError("lossless reconstruction failed")
    return {"canonical_json_bytes": len(raw), "prototype_bytes": len(wire),
            "saved_bytes": len(raw) - len(wire), "roundtrip_exact": True,
            "includes_ai_instructions_or_schema": False, "production_protocol": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    args = parser.parse_args()
    print(json.dumps(measure(json.loads(args.input.read_text())), indent=2))
