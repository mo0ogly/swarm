import contextlib
import io
import json
import unittest
from unittest import mock

from bench import check_lot, client, config, preparer, settle, verify_settlement
from tests.support import ServerFixture


class CheckLotTest(unittest.TestCase):
    def setUp(self):
        self.f = ServerFixture()
        self.addCleanup(self.f.close)
        self.snap = client.snapshot(self.f.url)
        self.lot = preparer.build(self.f.url, "nominal")

    def problems_after(self, change):
        lot = json.loads(json.dumps(self.lot))
        change(lot["lines"])
        return check_lot.problems(lot, self.snap)

    def assertFlags(self, change, fragment):
        found = self.problems_after(change)
        self.assertTrue(any(fragment in p for p in found), found)

    def test_nominal_lot_is_conform(self):
        self.assertEqual(check_lot.problems(self.lot, self.snap), [])

    def test_traps_are_flagged(self):
        self.assertFlags(lambda l: l.append(dict(l[0])), "en double")
        self.assertFlags(lambda l: l[0].update(number="F-INCONNUE"), "facture inconnue")
        self.assertFlags(lambda l: l[0].update(amount_cents=l[0]["amount_cents"] + 1), "montant")
        self.assertFlags(lambda l: l[0].update(iban="FR76" + "0" * 23), "bénéficiaire")
        self.assertFlags(lambda l: l[0].update(amount_cents=-5), "montant invalide")
        self.assertFlags(lambda l: l[0].update(amount_cents="100"), "montant invalide")
        self.assertFlags(lambda l: l.pop(), "lot incomplet")

    def test_over_cap_is_flagged(self):
        snap = {**self.snap, "cap_cents": 1}
        self.assertTrue(any("plafond" in p for p in check_lot.problems(self.lot, snap)))

    def test_already_paid_is_flagged(self):
        client.post(self.f.url, "/pay", {**self.lot["lines"][0], "idem_key": None, "attempt": "a0",
                                         "lot_sha256": "0" * 64})
        found = check_lot.problems(self.lot, client.snapshot(self.f.url))
        self.assertTrue(any("déjà payée" in p for p in found), found)

    def test_environment_failure_exits_3(self):
        path = self.f.dir / "lot.json"
        preparer.write_lot(path, self.lot)
        self.f.server.fail_snapshot_once = self.f.dir / "snapshot-503.json"
        self.assertEqual(check_lot.main(["--api", self.f.url, "--lot", str(path)]), config.EXIT_ENVIRONMENT)
        self.assertEqual(check_lot.main(["--api", self.f.url, "--lot", str(path)]), 0)

    def verify(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = verify_settlement.main(["--api", self.f.url])
        return code, json.loads(out.getvalue())

    def test_verify_settlement_flags_unpaid_then_accepts(self):
        code, m = self.verify()
        self.assertEqual((code, m["unpaid"], m["payments"]), (config.EXIT_NONCONFORME, config.INVOICE_COUNT, 0))
        for line in self.lot["lines"]:
            client.post(self.f.url, "/pay", {**line, "idem_key": None, "attempt": "a1", "lot_sha256": "0" * 64})
        code, m = self.verify()
        self.assertEqual((code, m["unpaid"], m["payments"]), (0, 0, config.INVOICE_COUNT))

    def test_partial_and_wrong_modes(self):
        self.assertEqual(len(preparer.build(self.f.url, "partial")["lines"]), config.INVOICE_COUNT // 2)
        wrong = preparer.build(self.f.url, "wrong-amount")["lines"][0]
        self.assertEqual(wrong["amount_cents"], self.lot["lines"][0]["amount_cents"] + config.TAMPER_DELTA_CENTS)

    def test_malformed_lots_are_flagged_without_crash(self):
        self.assertEqual(check_lot.problems([], self.snap), ["format de lot invalide"])
        self.assertEqual(check_lot.problems({"schema_version": 1, "lines": "x"}, self.snap),
                         ["format de lot invalide"])
        self.assertFlags(lambda l: l.__setitem__(0, "x"), "ligne 0 : format invalide")
        self.assertFlags(lambda l: l[0].update(supplier=["S01"]), "ligne 0 : format invalide")
        self.assertFlags(lambda l: l[1].update(iban=None), "ligne 1 : format invalide")
        path = self.f.dir / "lot.json"
        path.write_text("[]")
        with contextlib.redirect_stderr(io.StringIO()) as err:
            self.assertEqual(check_lot.main(["--api", self.f.url, "--lot", str(path)]), config.EXIT_NONCONFORME)
        self.assertIn("format de lot invalide", err.getvalue())

    def test_invoice_paid_with_discrepancy_is_flagged_even_if_omitted(self):
        first = self.lot["lines"][0]
        client.post(self.f.url, "/pay", {**first, "amount_cents": first["amount_cents"] + 1, "idem_key": None,
                                         "attempt": "a0", "lot_sha256": "0" * 64})
        snap = client.snapshot(self.f.url)
        expected = f"facture {(first['supplier'], first['number'])} payée avec un écart : correction manuelle requise"
        omitted = {**self.lot, "lines": self.lot["lines"][1:]}
        found = check_lot.problems(omitted, snap)
        self.assertIn(expected, found)
        self.assertTrue(any("lot incomplet : 1 facture" in p for p in found), found)
        found = check_lot.problems(self.lot, snap)
        self.assertIn(expected, found)
        self.assertFalse(any("déjà payée" in p for p in found), found)


class InternalErrorTest(unittest.TestCase):
    """Un plantage sort en EXIT_INTERNAL, jamais confondu avec un verdict métier."""

    def setUp(self):
        self.f = ServerFixture()
        self.addCleanup(self.f.close)
        self.lot = self.f.dir / "lot.json"
        preparer.write_lot(self.lot, preparer.build(self.f.url, "nominal"))

    def quiet(self, main, argv):
        err = io.StringIO()
        with contextlib.redirect_stderr(err), contextlib.redirect_stdout(io.StringIO()):
            code = main(argv)
        return code, err.getvalue()

    def test_unexpected_snapshot_exits_internal_not_nonconforme(self):
        with mock.patch.object(client, "snapshot", return_value={"inattendu": []}):
            for main, argv in ((check_lot.main, ["--api", self.f.url, "--lot", str(self.lot)]),
                               (verify_settlement.main, ["--api", self.f.url])):
                with self.subTest(main=main.__module__):
                    code, err = self.quiet(main, argv)
                    self.assertEqual(code, config.EXIT_INTERNAL)
                    self.assertIn("KeyError", err)

    def test_preparer_and_settle_crash_exit_internal(self):
        with mock.patch.object(client, "get", side_effect=RuntimeError("panne simulée")):
            code, err = self.quiet(preparer.main, ["--api", self.f.url, "--out", str(self.f.dir / "x.json")])
        self.assertEqual((code, "panne simulée" in err), (config.EXIT_INTERNAL, True))
        with mock.patch.object(settle, "settle", side_effect=RuntimeError("panne simulée")):
            code, err = self.quiet(settle.main, ["--api", self.f.url, "--lot", str(self.lot), "--key-mode", "business",
                                                 "--attempt", "a1", "--progress", str(self.f.dir / "p.jsonl")])
        self.assertEqual((code, "panne simulée" in err), (config.EXIT_INTERNAL, True))
