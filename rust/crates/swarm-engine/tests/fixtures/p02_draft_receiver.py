"""Read a single fixture draft on the private connected socket and acknowledge it."""
import hashlib
import json
import sys

class ObjectPairs(list):
    """Preserve object type and duplicate keys before validating the schema."""


mode, expected_invoice = sys.argv[1:]
# An ASCII invoice ID is at most 100 bytes. Allow the closed JSON plus framing
# margin, and read one additional byte to distinguish EOF from overflow.
max_draft_bytes = 256
data = sys.stdin.buffer.read(max_draft_bytes + 1)
if len(data) > max_draft_bytes:
    raise SystemExit("oversized draft")
if mode == "no-draft":
    if data:
        raise SystemExit("unexpected draft")
    sys.exit(0)

pairs = json.loads(data, object_pairs_hook=ObjectPairs)
if type(pairs) is not ObjectPairs:
    raise SystemExit("draft must be a JSON object")
draft = dict(pairs)
if (
    len(pairs) != 2
    or set(draft) != {"invoice_id", "account_matches"}
    or draft["invoice_id"] != expected_invoice
    or type(draft["account_matches"]) is not bool
):
    raise SystemExit("invalid draft")

# The test driver independently observes the received object on stderr.
print(json.dumps(draft, sort_keys=True), file=sys.stderr)
ack = hashlib.sha256(data).digest()
responses = {
    "accept": ack,
    "empty": b"",
    "truncated": ack[:-1],
    "trailing": ack + b"\0",
    "mismatch": bytes([ack[0] ^ 1]) + ack[1:],
}
sys.stdout.buffer.write(responses[mode])
