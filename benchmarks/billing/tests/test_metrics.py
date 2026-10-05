import unittest

from bench import config, invoices, ledger, metrics

INV = [{"supplier": "S01", "number": "F1", "amount_cents": 100, "iban": "A"},
       {"supplier": "S02", "number": "F2", "amount_cents": 200, "iban": "B"},
       {"supplier": "S03", "number": "F3", "amount_cents": 300, "iban": "C"}]


class MetricsTest(unittest.TestCase):
    def test_counts_doubles_wrong_unpaid_exactly(self):
        payments = [INV[0], dict(INV[0]), {**INV[1], "amount_cents": 201}, {**INV[1], "iban": "Z"}]
        self.assertEqual(metrics.measure_rows(INV, payments),
                         {"invoices": 3, "payments": 4, "doubles": 2, "wrong": 2, "unpaid": 2})

    def test_generation_is_reproducible_and_funded(self):
        a, b = ledger.connect(":memory:"), ledger.connect(":memory:")
        self.addCleanup(a.close)
        self.addCleanup(b.close)
        rows_a, rows_b = invoices.generate(a, seed=5), invoices.generate(b, seed=5)
        self.assertEqual(rows_a, rows_b)
        self.assertEqual(len(rows_a), config.INVOICE_COUNT)
        self.assertTrue(all(0 < r[2] < config.CAP_CENTS for r in rows_a))
        self.assertEqual(ledger.balance(a, ledger.TREASURY), config.FUNDING_FACTOR * sum(r[2] for r in rows_a))

    def test_measure_on_fresh_ledger_is_all_unpaid_and_not_correct(self):
        conn = ledger.connect(":memory:")
        self.addCleanup(conn.close)
        invoices.generate(conn, seed=5)
        m = metrics.measure(conn, config.CAP_CENTS)
        self.assertFalse(conn.in_transaction)
        self.assertEqual((m["payments"], m["unpaid"], m["violations"]), (0, config.INVOICE_COUNT, []))
        self.assertFalse(metrics.correct(m))
