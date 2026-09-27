#!/usr/bin/env python3
"""Count local review text with a pinned tokenizer, without contacting an AI.

This measures input text only. It does not authorize provider admission, enlarge
the engine's byte limit, reserve calls, or account for hidden client instructions.
"""
import argparse
import hashlib
import json
from pathlib import Path

import tiktoken


def measure(raw, model):
    text = raw.decode("utf-8", errors="strict")
    # Do not silently guess an encoding if this library lacks the model mapping.
    encoding = tiktoken.encoding_for_model(model)
    # Evidence can contain special-token-looking strings. They are ordinary data,
    # not message delimiters or executable instructions.
    tokens = encoding.encode_ordinary(text)
    if encoding.decode(tokens) != text:
        raise ValueError("tokenization did not preserve input text")
    return {
        "schema_version": 1,
        "model": model,
        "tokenizer": "tiktoken",
        "tokenizer_version": tiktoken.__version__,
        "encoding": encoding.name,
        "sha256": hashlib.sha256(raw).hexdigest(),
        "utf8_bytes": len(raw),
        "text_tokens": len(tokens),
        "roundtrip_exact": True,
        "includes_client_instructions_or_message_framing": False,
        "provider_admission_authorized": False,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--model", required=True)
    args = parser.parse_args()
    try:
        result = measure(args.input.read_bytes(), args.model)
    except (OSError, UnicodeError, KeyError, ValueError) as error:
        # Do not echo evidence or a path's content on failure.
        parser.exit(2, "Unable to count text: " + type(error).__name__ + "\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
