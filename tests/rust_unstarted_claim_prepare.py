#!/usr/bin/env python3
"""Prepare a NEW public source/control fixture for the cfg(test) claim producer.

Future authorized fixture execution only. This preparation command does not run
Cargo, an SDK, a reviewer or the claim producer. Existing inert planner/worker
helpers and one actual control batch create all positive evidence through CLI.
Normal candidate/profile bytes and the static helper closure are retained first.
No TTL change, private SQL write, historical Root copy or paid provider.
"""
import argparse
import json
import secrets
from pathlib import Path

import rust_gieth_effective_dependency as binding
import rust_native_review_inputs as inputs
import rust_native_reviewer as reviewer
import rust_native_managed_candidate as helpers


def main():
    """Emit token-bound producer input only after genuine public controls pass.

    Requires a qualified normal CLI and profile, a frozen checkout and fresh
    evidence path. Errors propagate with original helper channels retained.
    The resulting fixture still has no review claim or expiry/departure proof.
    Selecting/starting/waiting the producer is a separate reserved operation.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--profile", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not __debug__:
        raise RuntimeError("unstarted-fixture-oracle-requires-assertions")
    output = args.output.resolve()
    output.mkdir(mode=0o700, exist_ok=False)
    frozen = helpers.helper_inputs(Path(__file__))
    inputs.fixture.publish(output / "helper-inputs.before.json", frozen)
    binary = args.binary.resolve()
    profile = args.profile.resolve()
    effective = binding.qualify(binary, profile, output)
    root = None
    try:
        root, work, request = inputs.source(binary, output / "public-source")
        state = binding.shown(binary, root, output, "public-controls-work", work)
        task = next(item for item in state["tasks"] if item["id"] == request["task_id"])
        # Same inert program/ordinary reply contract as the maintained native
        # reviewer recipe; declared BEFORE the producer's actual host claim.
        reply = dict(reason="Owned inert fixture, no independent expert judgement",
                     criteria=[dict(index=index + 1, verdict="pass", evidence="Owned fixture criterion")
                               for index, _ in enumerate(task["criteria"])])
        needle = b"reply=Path('fixture-review-reply.json').read_text()"
        assert reviewer.PROGRAM.count(needle) == 1
        # Normal native execution uses an I0 output leaf. Embed the inert reply
        # before claim instead of assuming a Root-relative read is permitted.
        program = reviewer.PROGRAM.replace(needle, b"reply=" + repr(json.dumps(reply)).encode())
        inputs.fixture.publish(root / "codex", program)
        (root / "codex").chmod(0o700)
        inputs.fixture.publish(output / "inert-reviewer.py", program)
        inputs.fixture.publish(output / "inert-reply.json", reply)
        observed = inputs.observed(binary, root, output, "fresh-inputs", work, request)
        assert observed["preview"]["native_control_evidence_verified"]
        assert not observed["preview"]["provider_started"]
        assert observed["preview"]["remaining_review_calls"] >= 2
        # Token is a mixup guard for this fresh synthetic fixture, not identity,
        # permission, a model verdict or a substitute for the Root's OS guards.
        token = secrets.token_hex(32)
        inputs.fixture.publish(root / ".swarm/owned-unstarted-review-fixture", token.encode())
        current = binding.shown(binary, root, output, "before-producer", work)
        request = dict(request, event_id="owned-unstarted-producer", expected_revision=current["revision"])
        spec = dict(root=str(root), work=work, request=request, token=token,
                    scope="NEW public source/control fixture; no claim or reviewer yet")
        inputs.fixture.publish(output / "producer-input.json", spec)
        assert all(binding.digest(Path(n).read_bytes()) == h for n, h in effective.items())
        assert all(binding.digest(Path(n).read_bytes()) == h for n, h in frozen.items())
        inputs.fixture.publish(output / "result.json", dict(
            status="PUBLIC_CONTROLS_READY_FOR_RESERVED_PRODUCER",
            root=str(root), work=work, profile=str(profile),
            binary_sha256=binding.digest(binary.read_bytes()),
            provider_review_started=False, claim_created=False,
            expiry_observed=False, owner_departure_observed=False,
            global_watchdog=None))
    except BaseException as error:
        inputs.fixture.publish(output / "failure.json", dict(
            type=type(error).__name__, cause=str(error), root=str(root) if root else None,
            scope="partial new fixture; public helper channels retained; no rollback inferred"))
        raise


if __name__ == "__main__":
    main()
