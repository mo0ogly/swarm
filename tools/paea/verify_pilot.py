#!/usr/bin/env python3
"""Verify retained PAEA laboratory evidence against an explicitly pinned inventory.

No campaign is executed. Original files stay unchanged; a private temporary
recipe snapshot is checked and removed. Pin provenance/authenticity remains the
operator's responsibility; same-UID isolation, E2/E3 and authority are unproven.
"""
import argparse
import json
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from tools.paea.lab_evidence import Refusal, verify


def write_all(fd, raw):
    """Write every byte unbuffered; errors/zero progress propagate without retry.

    Already emitted bytes cannot be retracted after a later output failure.
    This is descriptor progress, not an execution attempt or operator limit.
    """
    pending = memoryview(raw)
    while pending:
        count = os.write(fd, pending)
        if count <= 0:
            raise OSError('laboratory-output-no-progress')
        pending = pending[count:]


def main(argv=None):
    """Read explicit campaign/pin; emit safe JSON, exit2 on any refused/IO/output issue.

    Diagnostics omit original paths/data. No source, binary, live Root or network
    is executed. A nonzero exit may follow partial stdout, never confirms success.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--campaign', type=Path, required=True)
    parser.add_argument('--inventory-sha256', required=True)
    args = parser.parse_args(argv)
    try:
        report = verify(args.campaign, args.inventory_sha256)
        write_all(1, (json.dumps(report) + '\n').encode())
        return 0
    except Refusal as error:
        code = str(error)
    except (OSError, ValueError, TypeError, KeyError, RecursionError, EOFError):
        code = 'laboratory-evidence-unavailable-or-invalid'
    try:
        write_all(2, (json.dumps({'status': 'refused', 'code': code}) + '\n').encode())
    except OSError:
        # Exit remains failure if both diagnostic and result destinations fail.
        return 2
    return 2


if __name__ == '__main__':
    raise SystemExit(main())
