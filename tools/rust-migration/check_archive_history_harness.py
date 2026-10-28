#!/usr/bin/env python3
"""Check explicit closure of the actual archive recipe's SQLite snapshot helper.

Extract its nested function, then run it against an owned empty fixture with
retained real connections. Destructors cannot mask an omitted close. A valid
read precedes a simulated iterator failure; both must close. This is a harness
resource check, not a live Root, CLI behavior, model call or OS confinement test.
The optional source is developer-owned Python, executed as code, not safe input.
No global watchdog or implicit retry. IO/parse/snapshot errors remain visible.
"""
import argparse
import ast
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import tempfile
from types import SimpleNamespace


def check(source):
    """Require explicit closes after successful and failed read-only snapshots.

    Connections stay referenced until observed; all are then closed independently
    in cleanup. Failure of the injected read must preserve the original exception.
    Files belong only to this temporary fixture and are removed by its context.
    """
    tree = ast.parse(source.read_text())
    run = next(n for n in tree.body if isinstance(n, ast.FunctionDef) and n.name == 'run')
    function = next(n for n in run.body if isinstance(n, ast.FunctionDef) and n.name == 'sql')
    held = []
    failure = RuntimeError('owned-iterator-failure')
    fail_dump = False

    class Connection(sqlite3.Connection):
        """Actual owned SQLite connection with observable explicit close calls."""
        closed_explicitly = False

        def close(self):
            """Record explicit closure while preserving real close errors."""
            self.closed_explicitly = True
            return super().close()

        def iterdump(self):
            """Read the real control, or inject the exact owned iterator cause."""
            if fail_dump:
                raise failure
            return super().iterdump()

    def connect(*args, **kwargs):
        """Retain each real handle so GC cannot impersonate explicit closure."""
        connection = sqlite3.connect(*args, **kwargs, factory=Connection)
        held.append(connection)
        return connection

    with tempfile.TemporaryDirectory(prefix='archive-sql-close-') as directory:
        root = Path(directory)
        (root / '.swarm').mkdir()
        with closing(sqlite3.connect(root / '.swarm/state.db')) as connection:
            connection.execute('CREATE TABLE witness(value INTEGER)')
            connection.commit()
        scope = {'sqlite3': SimpleNamespace(connect=connect),
                 'f': SimpleNamespace(root=root), 'closing': closing}
        exec(compile(ast.Module(body=[function], type_ignores=[]), str(source), 'exec'), scope)
        try:
            result = scope['sql']()
            assert 'witness' in result
            assert len(held) == 1 and held[0].closed_explicitly, 'successful snapshot did not close'
            print('PASS successful-snapshot-closes', flush=True)
            fail_dump = True
            try:
                scope['sql']()
            except RuntimeError as caught:
                assert caught is failure, 'snapshot lost primary cause'
            else:
                raise AssertionError('snapshot hid iterator failure')
            assert len(held) == 2 and held[1].closed_explicitly, 'failed snapshot did not close'
            print('PASS failed-snapshot-closes-and-preserves-cause', flush=True)
        finally:
            for connection in held:
                connection.close()
    print('ARCHIVE_HARNESS_CONNECTIONS_VERIFIED 2', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=Path(__file__).resolve().parents[2] / 'tests/rust_agent_archive_history.py')
    check(parser.parse_args().source)
