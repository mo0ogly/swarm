"""Negative corruption of actual immutable claim receipts in an owned fixture.

No positive seed, permission change, live mission or blind restoration. Original
context/hash tuples are retained by the caller; exact CAS restoration preserves
both a primary refusal-test failure and cleanup errors.
"""
import sqlite3
from contextlib import contextmanager
from rust_native_review_decision import rows


def replace(root,work,claim,expected,replacement):
    """CAS one genuine context/hash tuple, or fail without replacing another row.

    Uses SQLite's immediate no-wait ownership check rather than a guessed retry
    deadline. Transaction/rollback/close failures are joined; no silent success.
    """
    db=sqlite3.connect(root/'.swarm/state.db',timeout=0);errors=[]
    try:
        db.execute('BEGIN IMMEDIATE')
        cursor=db.execute('UPDATE native_planning_claims SET context_json=?,context_sha256=? WHERE claim_id=? AND work_id=? AND context_json=? AND context_sha256=?',(*replacement,claim,work,*expected));count=cursor.rowcount;cursor.close()
        if count!=1:raise ValueError('Owned immutable claim changed unexpectedly')
        db.commit()
    except BaseException as error:
        errors.append(error)
        if db.in_transaction:
            try:db.rollback()
            except BaseException as error:errors.append(error)
    try:db.close()
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Negative receipt mutation and cleanup failed',errors)


@contextmanager
def negative_receipt(root,work,claim,original,changed):
    """Restore only the exact injected negative tuple; retain all failure causes."""
    errors=[]
    try:
        replace(root,work,claim,original,changed)
        yield
    except BaseException as error:errors.append(error)
    try:
        actual=rows(root,'SELECT context_json,context_sha256 FROM native_planning_claims WHERE claim_id=? AND work_id=?',(claim,work))
        if actual==[changed]:replace(root,work,claim,changed,original)
        elif actual!=[original]:raise ValueError('Unexpected receipt; refusing blind restoration')
    except BaseException as error:errors.append(error)
    if errors:raise BaseExceptionGroup('Receipt corruption probe and restoration failed',errors)
