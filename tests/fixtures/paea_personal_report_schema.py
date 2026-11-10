"""Validate public nonsecret CLI observation bytes against the retained schema.

No secret frame, Root, provider or network input. A schema/report refusal exits
nonzero; it does not authorize or replay a lifecycle operation.
"""
import json, pathlib, sys
from jsonschema import Draft202012Validator
schema = json.loads(pathlib.Path(sys.argv[1]).read_text())
Draft202012Validator.check_schema(schema)
Draft202012Validator(schema).validate(json.load(sys.stdin))
