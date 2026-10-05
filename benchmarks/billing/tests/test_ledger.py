import sqlite3
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from bench import invoices, ledger


class LedgerTest(unittest.TestCase):
    def setUp(self):
        self.conn = ledger.connect(":memory:")
        self.addCleanup(self.conn.close)
        ledger.fund(self.conn, 1_000_000)

    def pay(self, key=None, amount=100_000, conn=None, iban="FR76X"):
        return ledger.pay(conn or self.conn, supplier="S01", number="F1", amount_cents=amount, iban=iban,
                          idem_key=key, attempt="a1", lot_sha256="0" * 64)

    def count(self, table, conn=None):
        return (conn or self.conn).execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]

    def payment_entries(self, conn=None):
        return (conn or self.conn).execute(
            "SELECT COUNT(*) FROM entries WHERE payment_id IS NOT NULL").fetchone()[0]

    def test_payment_writes_balanced_entries(self):
        pid, replay = self.pay()
        self.assertFalse(replay)
        rows = self.conn.execute("SELECT account, amount_cents FROM entries WHERE payment_id=? ORDER BY account",
                                 (pid,)).fetchall()
        self.assertEqual(rows, [("supplier:S01", 100_000), ("treasury", -100_000)])
        self.assertEqual(ledger.balance(self.conn, ledger.TREASURY), 900_000)
        self.assertEqual(ledger.violations(self.conn, 5_000_000), [])

    def test_same_key_has_one_effect(self):
        first = self.pay(key="S01:F1")
        second = self.pay(key="S01:F1")
        self.assertEqual(second, (first[0], True))
        self.assertEqual(self.count("payments"), 1)
        self.assertEqual(self.payment_entries(), 2)
        self.assertEqual(ledger.balance(self.conn, ledger.TREASURY), 900_000)

    def test_same_key_with_different_content_is_refused(self):
        self.pay(key="S01:F1")
        for changed in ({"amount": 100_001}, {"iban": "FR76Y"}):
            with self.assertRaises(ledger.IdempotencyConflict):
                self.pay(key="S01:F1", **changed)
        self.assertEqual(self.count("payments"), 1)
        self.assertEqual(self.payment_entries(), 2)
        self.assertEqual(ledger.balance(self.conn, ledger.TREASURY), 900_000)
        self.assertFalse(self.conn.in_transaction)

    def test_without_key_pays_twice(self):
        self.pay()
        self.pay()
        self.assertEqual(self.count("payments"), 2)

    def test_insufficient_funds_writes_nothing(self):
        with self.assertRaises(ledger.InsufficientFunds):
            self.pay(amount=2_000_000)
        self.assertEqual(self.count("payments"), 0)
        self.assertEqual(self.count("entries"), 2)  # dotation initiale seulement
        self.assertFalse(self.conn.in_transaction)

    def test_invalid_amount_is_refused_without_write(self):
        for amount in (-500, 0, "100", True):
            with self.subTest(amount=amount), self.assertRaises(ledger.InvalidPayment):
                self.pay(amount=amount)
        self.assertEqual(self.count("payments"), 0)
        self.assertEqual(self.count("entries"), 2)
        self.assertFalse(self.conn.in_transaction)
        with self.assertRaises(sqlite3.IntegrityError):
            self.conn.execute("INSERT INTO payments(supplier, number, amount_cents, iban, attempt, lot_sha256,"
                              " created_at) VALUES('S01', 'F1', 0, 'X', 'a', 'h', 0)")

    def test_treasury_exactly_equal_to_amount_is_accepted(self):
        self.pay(amount=1_000_000)
        self.assertEqual(ledger.balance(self.conn, ledger.TREASURY), 0)
        self.assertEqual(ledger.violations(self.conn, 5_000_000), [])

    def test_negative_treasury_is_detected(self):
        self.pay(amount=1_000_000)
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, 'treasury', -1)")
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, 'equity', 1)")
        self.assertEqual(ledger.violations(self.conn, 5_000_000), ["trésorerie négative"])

    def test_violations_detect_unbalanced_entry_and_cap(self):
        pid, _ = self.pay(amount=600_000)
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(?, 'treasury', -1)", (pid,))
        self.assertEqual(ledger.violations(self.conn, 500_000),
                         [f"écriture déséquilibrée : paiement {pid}", f"écritures incohérentes : paiement {pid}",
                          f"plafond dépassé : paiement {pid}"])

    def test_violations_detect_balanced_but_inconsistent_entries(self):
        pid, _ = self.pay()
        self.conn.execute("UPDATE entries SET account = 'supplier:S99' WHERE payment_id = ? AND account != 'treasury'",
                          (pid,))
        self.assertEqual(ledger.violations(self.conn, 5_000_000), [f"écritures incohérentes : paiement {pid}"])

    def test_violations_detect_payment_without_entries(self):
        pid, _ = self.pay()
        self.conn.execute("DELETE FROM entries WHERE payment_id = ?", (pid,))
        self.assertEqual(ledger.violations(self.conn, 5_000_000), [f"écritures incohérentes : paiement {pid}"])

    def test_violations_detect_unbalanced_funding(self):
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, 'equity', 5)")
        self.assertEqual(ledger.violations(self.conn, 5_000_000), ["écriture déséquilibrée : dotation"])

    def test_failed_generate_rolls_back_and_leaves_connection_usable(self):
        conn = ledger.connect(":memory:")
        self.addCleanup(conn.close)
        invoices.generate(conn, seed=5)
        treasury = ledger.balance(conn, ledger.TREASURY)
        with self.assertRaises(sqlite3.IntegrityError):
            invoices.generate(conn, seed=5)
        self.assertFalse(conn.in_transaction)
        self.assertEqual(ledger.balance(conn, ledger.TREASURY), treasury)
        self.assertEqual(self.pay(conn=conn)[1], False)
        self.assertEqual(self.count("payments", conn), 1)

    def test_transaction_rolls_back_on_base_exception(self):
        with self.assertRaises(KeyboardInterrupt):
            with ledger.transaction(self.conn):
                self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, 'x', 1)")
                raise KeyboardInterrupt
        self.assertFalse(self.conn.in_transaction)
        self.assertEqual(self.count("entries"), 2)

    def test_failed_commit_rolls_back_and_reraises(self):
        conn = FailingCommit(self.conn)
        with self.assertRaises(sqlite3.OperationalError):
            with ledger.transaction(conn):
                conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, 'x', 1)")
        self.assertTrue(conn.commit_attempted)
        self.assertFalse(self.conn.in_transaction)
        self.assertEqual(self.count("entries"), 2)

    def test_violations_detect_orphan_entries(self):
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(999, 'treasury', -5)")
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(999, 'supplier:S01', 5)")
        self.assertEqual(ledger.violations(self.conn, 5_000_000), ["écriture orpheline : paiement 999"])


class FailingCommit:
    """Connexion enveloppée dont le COMMIT échoue (verrou, disque plein...)."""

    def __init__(self, conn):
        self.conn = conn
        self.commit_attempted = False

    @property
    def in_transaction(self):
        return self.conn.in_transaction

    def execute(self, sql, *args):
        if sql.strip().upper() == "COMMIT":
            self.commit_attempted = True
            raise sqlite3.OperationalError("database is locked")
        return self.conn.execute(sql, *args)


class LedgerFileTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.path = Path(tmp.name) / "ledger.sqlite"

    def open(self):
        conn = ledger.connect(self.path)
        self.addCleanup(conn.close)
        return conn

    def test_file_connection_uses_wal(self):
        conn = self.open()
        self.assertEqual(conn.execute("PRAGMA journal_mode").fetchone()[0], "wal")

    def test_connect_refuses_file_without_wal(self):
        fake = mock.MagicMock()
        fake.execute.return_value.fetchone.return_value = ("delete",)
        with mock.patch.object(ledger.sqlite3, "connect", return_value=fake):
            with self.assertRaises(RuntimeError):
                ledger.connect(self.path)
        fake.close.assert_called_once()

    def test_two_connections_same_key_pay_once(self):
        a, b = self.open(), self.open()
        ledger.fund(a, 1_000_000)
        args = dict(supplier="S01", number="F1", amount_cents=100_000, iban="FR76X",
                    idem_key="S01:F1", lot_sha256="0" * 64)
        first = ledger.pay(a, attempt="a1", **args)
        second = ledger.pay(b, attempt="a2", **args)
        self.assertEqual(second, (first[0], True))
        self.assertEqual(b.execute("SELECT COUNT(*) FROM payments").fetchone()[0], 1)
        self.assertEqual(b.execute("SELECT COUNT(*) FROM entries WHERE payment_id IS NOT NULL").fetchone()[0], 2)
        self.assertEqual(ledger.balance(b, ledger.TREASURY), 900_000)
