import json
import time
import unittest
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from unittest import mock

from bench import client, config, markers, payment_api
from tests.support import NON_ASCII_HEADER, TEST_TOKEN, ServerFixture


def body(line, key=None):
    return {**line, "idem_key": key, "attempt": "a1", "lot_sha256": "0" * 64}


class PaymentApiTest(unittest.TestCase):
    def fixture(self, **options):
        f = ServerFixture(**options)
        self.addCleanup(f.close)
        return f

    def rejected(self, call, code):
        """Refus HTTP explicite (pas une coupure) ; retourne le corps JSON de l'erreur."""
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            call()
        self.assertEqual(ctx.exception.code, code)
        return json.loads(ctx.exception.read())

    def test_due_lists_unpaid_only(self):
        f = self.fixture()
        due = client.get(f.url, "/due")
        self.assertEqual(len(due), config.INVOICE_COUNT)
        client.post(f.url, "/pay", body(due[0]))
        self.assertEqual(client.get(f.url, "/due"), due[1:])

    def test_lost_response_has_paid_and_left_marker(self):
        f = self.fixture(lose_response_once=None)
        f.server.lose_response_once = f.dir / "response-lost.json"
        line = client.get(f.url, "/due")[0]
        with self.assertRaises((ConnectionError, urllib.error.URLError)):
            client.post(f.url, "/pay", body(line, key="k"))
        self.assertEqual(f.measure()["payments"], 1)
        self.assertEqual(json.loads((f.dir / "response-lost.json").read_text())["fault"], "response-lost")
        self.assertTrue(client.post(f.url, "/pay", body(line, key="k"))["replay"])

    def test_snapshot_fails_n_times_then_answers(self):
        """F5 persistante : F5_SNAPSHOT_FAILURES réponses 503 consécutives, marqueur au premier 503."""
        f = self.fixture()
        f.server.fail_snapshot = f.dir / "snapshot-503.json"
        for i in range(config.F5_SNAPSHOT_FAILURES):
            with self.assertRaises(client.Unavailable):
                client.snapshot(f.url)
            self.assertEqual(self.served(f, i + 1), i + 1)
        self.assertFalse((f.dir / "snapshot-recovered.json").exists())
        self.assertEqual(len(client.snapshot(f.url)["invoices"]), config.INVOICE_COUNT)
        deadline = time.monotonic() + 2   # marqueur écrit après l'envoi de l'instantané : juste après la réponse
        while not (f.dir / "snapshot-recovered.json").exists() and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue((f.dir / "snapshot-recovered.json").exists())   # faute absorbée : instantané servi

    def test_b1_refuses_unknown_beneficiary_and_over_cap(self):
        f = self.fixture(policy="b1")
        line = client.get(f.url, "/due")[0]
        for bad in ({**line, "iban": "FR76" + "0" * 23}, {**line, "amount_cents": config.CAP_CENTS + 1}):
            with self.assertRaises(urllib.error.HTTPError) as ctx:
                client.post(f.url, "/pay", body(bad))
            self.assertEqual(ctx.exception.code, 403)
        self.assertEqual(f.measure()["payments"], 0)

    def test_token_is_required_when_configured(self):
        f = self.fixture(token=TEST_TOKEN)
        line = client.get(f.url, "/due")[0]
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            client.post(f.url, "/pay", body(line))
        self.assertEqual(ctx.exception.code, 401)
        self.assertFalse(client.post(f.url, "/pay", body(line), token=TEST_TOKEN)["replay"])

    def test_same_key_different_content_is_409_without_write(self):
        f = self.fixture()
        line = client.get(f.url, "/due")[0]
        client.post(f.url, "/pay", body(line, key="k"))
        with self.assertRaises(urllib.error.HTTPError) as ctx:
            client.post(f.url, "/pay", body({**line, "amount_cents": line["amount_cents"] + 1}, key="k"))
        self.assertEqual(ctx.exception.code, 409)
        self.assertEqual(json.loads(ctx.exception.read()), {"error": "conflit de clé d'idempotence"})
        m = f.measure()
        self.assertEqual((m["payments"], m["wrong"], m["violations"]), (1, 0, []))

    def test_invalid_amount_is_400_without_write(self):
        f = self.fixture()
        line = client.get(f.url, "/due")[0]
        for amount in (-5, 0, "100"):
            with self.subTest(amount=amount):
                with self.assertRaises(urllib.error.HTTPError) as ctx:
                    client.post(f.url, "/pay", body({**line, "amount_cents": amount}))
                self.assertEqual(ctx.exception.code, 400)
                self.assertIn("montant invalide", json.loads(ctx.exception.read())["error"])
        self.assertEqual(f.measure()["payments"], 0)

    def test_b1_invalid_input_is_400_without_write(self):
        f = self.fixture(policy="b1")
        line = client.get(f.url, "/due")[0]
        for field in ("supplier", "number", "amount_cents", "iban", "attempt", "lot_sha256"):
            missing = {k: v for k, v in body(line).items() if k != field}
            with self.subTest(missing=field):
                with self.assertRaises(urllib.error.HTTPError) as ctx:
                    client.post(f.url, "/pay", missing)
                self.assertEqual(ctx.exception.code, 400)
                self.assertEqual(json.loads(ctx.exception.read()),
                                 {"error": f"champ obligatoire manquant : {field}"})
        for amount in ("100", True, 1.5, -5, 0):
            with self.subTest(amount=amount):
                with self.assertRaises(urllib.error.HTTPError) as ctx:
                    client.post(f.url, "/pay", body({**line, "amount_cents": amount}))
                self.assertEqual(ctx.exception.code, 400)
                self.assertIn("montant invalide", json.loads(ctx.exception.read())["error"])
        for raw in (b"{pas du json", b"[1, 2]"):
            request = urllib.request.Request(f.url + "/pay", data=raw, method="POST",
                                             headers={"Content-Type": "application/json"})
            with self.subTest(raw=raw):
                with self.assertRaises(urllib.error.HTTPError) as ctx:
                    urllib.request.urlopen(request, timeout=config.HTTP_TIMEOUT_S)
                self.assertEqual(ctx.exception.code, 400)
                self.assertEqual(json.loads(ctx.exception.read()), {"error": "corps JSON illisible"})
        self.assertEqual(f.measure()["payments"], 0)
        self.assertEqual(client.get(f.url, "/due")[0], line)

    def test_fixture_close_removes_its_directory(self):
        f = ServerFixture()
        self.assertTrue((f.dir / "ledger.db").exists())
        f.close()
        self.assertFalse(f.dir.exists())

    def test_wrong_field_types_are_400_without_write(self):
        cases = {"supplier": ["S01"], "number": 7, "iban": None, "attempt": "", "lot_sha256": 5,
                 "idem_key": {"k": 1}}
        for policy in ("none", "b1"):
            f = self.fixture(policy=policy)
            line = client.get(f.url, "/due")[0]
            for field, value in cases.items():
                with self.subTest(policy=policy, field=field):
                    error = self.rejected(lambda: client.post(f.url, "/pay", {**body(line), field: value}), 400)
                    self.assertEqual(error, {"error": f"champ invalide : {field}"})
            with self.subTest(policy=policy, field="idem_key vide"):
                error = self.rejected(lambda: client.post(f.url, "/pay", body(line, key="")), 400)
                self.assertEqual(error, {"error": "champ invalide : idem_key"})
            self.assertEqual(f.measure()["payments"], 0)
            self.assertFalse(client.post(f.url, "/pay", body(line, key="k"))["replay"])

    def test_non_ascii_token_is_401_without_write(self):
        f = self.fixture(token=TEST_TOKEN)
        line = client.get(f.url, "/due")[0]
        error = self.rejected(lambda: client.post(f.url, "/pay", body(line), token=NON_ASCII_HEADER), 401)
        self.assertEqual(error, {"error": "jeton de règlement absent ou invalide"})
        self.assertEqual(f.measure()["payments"], 0)

    def test_unexpected_error_is_500_not_a_disconnection(self):
        f = self.fixture()
        line = client.get(f.url, "/due")[0]
        with mock.patch.object(payment_api.ledger, "pay", side_effect=RuntimeError("panne simulée")):
            error = self.rejected(lambda: client.post(f.url, "/pay", body(line)), 500)
        self.assertEqual(error, {"error": "erreur interne : RuntimeError"})
        with mock.patch.object(f.server, "query", side_effect=ZeroDivisionError):
            error = self.rejected(lambda: client.get(f.url, "/due"), 500)
        self.assertEqual(error, {"error": "erreur interne : ZeroDivisionError"})
        self.assertEqual(f.measure()["payments"], 0)
        self.assertEqual(len(client.get(f.url, "/due")), config.INVOICE_COUNT)

    def test_empty_token_is_refused_at_start(self):
        f = self.fixture()
        with self.assertRaises(ValueError):
            payment_api.PaymentServer(("127.0.0.1", 0), str(f.dir / "ledger.db"), token="")

    def test_client_sends_token_header_even_when_empty(self):
        with mock.patch.object(client.urllib.request, "urlopen", side_effect=RuntimeError("arrêt")) as opened:
            with self.assertRaises(RuntimeError):
                client.post("http://127.0.0.1:9", "/pay", {}, token="")
        self.assertEqual(opened.call_args.args[0].get_header("X-settlement-token"), "")

    def concurrent(self, calls):
        with ThreadPoolExecutor(max_workers=len(calls)) as pool:
            futures = [pool.submit(c) for c in calls]
        return [f.exception() for f in futures]

    def slow_markers(self):
        real = markers.mark

        def slow(*args, **kwargs):
            time.sleep(0.2)   # élargit la fenêtre entre le test d'existence et la pose du marqueur
            real(*args, **kwargs)
        return mock.patch.object(payment_api.markers, "mark", side_effect=slow)

    @staticmethod
    def served(f, expected, timeout=2.0):
        """Compte publié dans le marqueur ; il est écrit après l'envoi du 503, donc juste après la réponse."""
        deadline = time.monotonic() + timeout
        value = None
        while time.monotonic() < deadline:
            path = f.dir / "snapshot-503.json"
            value = json.loads(path.read_text())["served"] if path.exists() else None
            if value == expected:
                break
            time.sleep(0.01)
        return value

    def test_snapshot_503_counted_only_once_sent(self):
        f = self.fixture()
        f.server.fail_snapshot = f.dir / "snapshot-503.json"
        with mock.patch.object(payment_api.Handler, "send_json", side_effect=ConnectionResetError("client parti")):
            with self.assertRaises(Exception):
                client.snapshot(f.url)
        time.sleep(0.2)
        self.assertFalse((f.dir / "snapshot-503.json").exists(), "503 non envoyé : rien à compter")

    def test_snapshot_fault_count_holds_under_concurrency(self):
        f = self.fixture()
        f.server.fail_snapshot = f.dir / "snapshot-503.json"
        n = config.F5_SNAPSHOT_FAILURES
        with self.slow_markers():
            errors = self.concurrent([lambda: client.snapshot(f.url)] * (n + 2))
        self.assertEqual(sum(isinstance(e, client.Unavailable) for e in errors), n, errors)
        self.assertEqual(sum(e is None for e in errors), 2, errors)
        self.assertEqual(self.served(f, n), n)

    def test_lost_response_fires_once_under_concurrency(self):
        f = self.fixture()
        f.server.lose_response_once = f.dir / "response-lost.json"
        due = client.get(f.url, "/due")[:6]
        with self.slow_markers():
            errors = self.concurrent([lambda line=line: client.post(f.url, "/pay", body(line)) for line in due])
        self.assertEqual(sum(isinstance(e, ConnectionError) for e in errors), 1, errors)
        self.assertEqual(sum(e is None for e in errors), 5, errors)
        self.assertEqual(f.measure()["payments"], 6)
