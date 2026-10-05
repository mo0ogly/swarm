import contextlib
import io
import json
import socket
import subprocess
import sys
import unittest
import urllib.error
from unittest import mock

from bench import config, preparer, settle
from tests.support import TEST_TOKEN, ServerFixture

SCRIPT = str(settle.__file__)


class SettleTest(unittest.TestCase):
    def setUp(self):
        self.f = ServerFixture()
        self.addCleanup(self.f.close)
        self.lot = self.f.dir / "lot.json"
        self.sha = preparer.write_lot(self.lot, preparer.build(self.f.url, "nominal"))

    def run_settle(self, key_mode, attempt, progress, lose=False):
        if lose:
            self.f.server.lose_response_once = self.f.dir / "response-lost.json"
        return settle.settle(self.f.url, self.lot, key_mode, attempt, self.f.dir / progress)

    def run_main(self, *extra, api=None):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = settle.main(["--api", api or self.f.url, "--lot", str(self.lot), "--key-mode", "business",
                                "--attempt", "a1", "--progress", str(self.f.dir / "p1"), *extra])
        return code, out.getvalue(), err.getvalue()

    def progress_rows(self, name="p1"):
        path = self.f.dir / name
        return [json.loads(r) for r in path.read_text().splitlines()] if path.exists() else []

    def test_nominal_business_pays_everything_once(self):
        out = self.run_settle("business", "a1", "p1")
        self.assertEqual(out["paid"], config.INVOICE_COUNT)
        m = self.f.measure()
        self.assertEqual((m["doubles"], m["wrong"], m["unpaid"]), (0, 0, 0))
        self.assertEqual({r["lot_sha256"] for r in self.progress_rows()}, {self.sha})

    def test_lost_response_business_key_one_payment(self):
        out = self.run_settle("business", "a1", "p1", lose=True)
        self.assertEqual(json.loads((self.f.dir / "response-lost.json").read_text())["fault"], "response-lost")
        m = self.f.measure()
        self.assertEqual((m["doubles"], m["payments"], m["unpaid"]), (0, config.INVOICE_COUNT, 0))
        self.assertEqual((out["paid"], out["replayed"]), (config.INVOICE_COUNT - 1, 1))

    def test_lost_response_without_key_doubles_once(self):  # canari F1
        self.run_settle("none", "a1", "p1", lose=True)
        self.assertEqual(self.f.measure()["doubles"], 1)

    def test_same_attempt_key_replays_on_rerun(self):
        self.run_settle("attempt", "a1", "p1")
        out = self.run_settle("attempt", "a1", "p2")
        self.assertEqual(out["replayed"], config.INVOICE_COUNT)
        self.assertEqual(self.f.measure()["doubles"], 0)

    def test_digest_mismatch_pays_nothing(self):
        with self.assertRaises(settle.DigestMismatch):
            settle.settle(self.f.url, self.lot, "business", "a1", self.f.dir / "p1", expected_sha256="f" * 64)
        self.assertEqual(self.f.measure()["payments"], 0)

    def crash(self, key_mode, after, lot=None, extra=()):
        marker = self.f.dir / "crash-before-record.json"
        args = [sys.executable, SCRIPT, "--api", self.f.url, "--lot", str(lot or self.lot), "--key-mode", key_mode,
                "--progress", str(self.f.dir / "p1"), "--attempt", "a1", "--crash-after-pay", str(after),
                "--crash-marker", str(marker), *extra]
        first = subprocess.run(args, capture_output=True)
        self.assertEqual(first.returncode, config.EXIT_CRASH, first.stderr)
        return json.loads(marker.read_text())

    def restart(self, key_mode, lot=None):
        second = subprocess.run([sys.executable, SCRIPT, "--api", self.f.url, "--lot", str(lot or self.lot),
                                 "--key-mode", key_mode, "--progress", str(self.f.dir / "p1"),
                                 "--attempt", "a1-relance"], capture_output=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        return json.loads(second.stdout)

    def crash_then_restart(self, key_mode, after=1):
        self.crash(key_mode, after)
        out = self.restart(key_mode)
        return self.f.measure(), out

    def test_crash_restart_attempt_key_doubles_once(self):  # canari F3
        self.assertEqual(self.crash_then_restart("attempt")[0]["doubles"], 1)

    def test_crash_restart_business_key_no_double(self):
        m, _ = self.crash_then_restart("business")
        self.assertEqual((m["doubles"], m["unpaid"]), (0, 0))

    def test_crash_after_third_response_attempt_key(self):
        self.crash("attempt", 3)
        out = self.restart("attempt")
        m = self.f.measure()
        self.assertEqual((out["skipped"], m["doubles"], m["unpaid"]), (2, 1, 0))

    def test_progress_of_another_lot_is_not_reused(self):
        self.crash("business", 3)
        self.assertEqual(len(self.progress_rows()), 2)
        lot_b = self.f.dir / "lot-b.json"
        preparer.write_lot(lot_b, preparer.build(self.f.url, "nominal"))
        out = self.restart("business", lot=lot_b)
        m = self.f.measure()
        self.assertEqual((out["skipped"], out["paid"]), (0, config.INVOICE_COUNT - 3))
        self.assertEqual((m["doubles"], m["unpaid"], m["payments"]), (0, 0, config.INVOICE_COUNT))

    def test_crash_counter_ignores_refusals(self):
        self.f.server.policy = "b1"
        lot = preparer.build(self.f.url, "nominal")
        lot["lines"][0]["iban"] = "FR76" + "0" * 23
        bad = self.f.dir / "lot-refus.json"
        preparer.write_lot(bad, lot)
        self.assertEqual(self.crash("business", 1, lot=bad)["line"], 1)
        self.assertEqual(self.f.measure()["payments"], 1)
        self.assertEqual([(r["line"], r["refused"]) for r in self.progress_rows()], [(0, 403)])

    def test_unreachable_api_exits_3_without_traceback(self):
        with socket.socket() as probe:
            probe.bind(("127.0.0.1", 0))
            port = probe.getsockname()[1]
        code, _, err = self.run_main(api=f"http://127.0.0.1:{port}")
        self.assertEqual(code, config.EXIT_ENVIRONMENT)
        self.assertIn("injoignable", err)
        self.assertNotIn("Traceback", err)
        self.assertEqual(self.progress_rows(), [])

    def test_401_is_not_recorded_then_retried_with_token(self):
        self.f.server.token = TEST_TOKEN
        code, out, _ = self.run_main()
        self.assertEqual(code, config.EXIT_REFUSED)
        self.assertEqual(json.loads(out)["stopped"]["refused"], 401)
        self.assertEqual(self.progress_rows(), [])
        token_file = self.f.dir / "token"
        token_file.write_text(TEST_TOKEN + "\n")
        code, out, _ = self.run_main("--token-file", str(token_file))
        self.assertEqual((code, json.loads(out)["paid"]), (0, config.INVOICE_COUNT))
        m = self.f.measure()
        self.assertEqual((m["payments"], m["unpaid"], m["doubles"]), (config.INVOICE_COUNT, 0, 0))

    def http_error(self, code, raw):
        return urllib.error.HTTPError(self.f.url + "/pay", code, "erreur", {}, io.BytesIO(raw))

    def test_non_json_error_body_is_kept_truncated(self):
        with mock.patch.object(settle.client, "post", side_effect=self.http_error(403, b"<html>" + b"x" * 1000)):
            response = settle.pay_line(self.f.url, {}, None)
        self.assertEqual(response["refused"], 403)
        self.assertTrue(response["error"].startswith("<html>xx"))
        self.assertEqual(len(response["error"]), config.ERROR_TEXT_MAX)

    def test_5xx_stops_without_recording(self):
        with mock.patch.object(settle.client, "post", side_effect=self.http_error(502, b"Bad gateway")):
            out = settle.settle(self.f.url, self.lot, "business", "a1", self.f.dir / "p1")
        self.assertEqual(out["stopped"], {"line": 0, "refused": 502, "error": "Bad gateway"})
        self.assertEqual((out["paid"], out["refused"]), (0, []))
        self.assertEqual(self.progress_rows(), [])
