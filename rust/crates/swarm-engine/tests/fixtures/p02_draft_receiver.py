"""Read a single fixture draft on the private connected socket and acknowledge it."""
import hashlib
import json
import sys

mode, expected_invoice = sys.argv[1:]
# Bound the test receiver's input to the closed draft representation.
data = sys.stdin.buffer.read(256)
if mode == "no-draft":
    if data:
        raise SystemExit("unexpected draft")
    sys.exit(0)

pairs = json.loads(data, object_pairs_hook=list)
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
