import tempfile
import threading
import unittest
import re
from pathlib import Path
from types import SimpleNamespace
from urllib.parse import urlparse

from app import CasaPizzaHandler, init_db

ROOT = Path(__file__).resolve().parents[1]


class HandlerHarness:
    """Exécute les gestionnaires réels sans socket, interdit dans la sandbox."""

    def __init__(self, db_path, server=None):
        self.server = server or SimpleNamespace(db_path=db_path, restaurant_password="secret-test", sessions=set(), sessions_lock=threading.Lock())
        self.handler = object.__new__(CasaPizzaHandler)
        self.handler.server = self.server
        self.handler.headers = {}
        self.response = None
        self.handler.send_json = self.capture

    def capture(self, status, data, headers=None):
        self.response = (int(status), data, headers or {})

    def call(self, method, *args):
        self.response = None
        getattr(self.handler, method)(*args)
        return self.response

    def cookie(self, value=None):
        self.handler.headers = {"Cookie": value} if value else {}


class CasaPizzaBehaviorTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.db_path = str(Path(self.temp.name) / "orders.db")
        init_db(self.db_path)
        self.app = HandlerHarness(self.db_path)

    def tearDown(self):
        self.temp.cleanup()

    def valid_order(self):
        return {"customer_name": "Camille Test", "address": "12 rue Fictive, 75000 Paris", "phone": "0600000000", "items": [{"pizza_id": "margherita", "quantity": 1}, {"pizza_id": "reine", "quantity": 2}]}

    def create_order(self, payload=None):
        status, data, _ = self.app.call("create_order", payload or self.valid_order())
        self.assertEqual(status, 201, data)
        return data

    def login(self):
        status, data, headers = self.app.call("restaurant_login", {"password": "secret-test"})
        self.assertEqual(status, 200, data)
        cookie = headers["Set-Cookie"].split(";", 1)[0]
        self.app.cookie(cookie)
        return cookie

    def test_catalog_filter_basket_and_responsive_interface_are_wired(self):
        html = (ROOT / "static/index.html").read_text(encoding="utf-8")
        javascript = (ROOT / "static/app.js").read_text(encoding="utf-8")
        css = (ROOT / "static/style.css").read_text(encoding="utf-8")
        from app import PIZZAS
        self.assertEqual(len(PIZZAS), 6)
        self.assertEqual(sum(pizza["vegetarian"] for pizza in PIZZAS), 3)
        self.assertTrue(all(pizza["ingredients"] and isinstance(pizza["price_cents"], int) for pizza in PIZZAS))
        self.assertIn('id="vegetarian-filter"', html)
        self.assertIn('id="checkout"', html)
        self.assertIn("catalog.filter", javascript)
        self.assertIn("basket.set", javascript)
        mobile_breakpoints = re.findall(r"@media\s*\(max-width:\s*(\d+)px\)", css)
        self.assertTrue(any(int(width) <= 600 for width in mobile_breakpoints))
        self.assertEqual(len(list((ROOT / "static/images").glob("*.svg"))), 6)

    def test_invalid_orders_are_rejected_and_prices_are_server_owned(self):
        status, data, _ = self.app.call("create_order", {"customer_name": "", "address": "", "phone": "", "items": []})
        self.assertEqual(status, 400)
        self.assertIn("obligatoires", data["error"])
        under_minimum = self.valid_order()
        under_minimum["items"] = [{"pizza_id": "margherita", "quantity": 1}]
        status, data, _ = self.app.call("create_order", under_minimum)
        self.assertEqual(status, 400)
        self.assertIn("minimum", data["error"])
        invalid = self.valid_order()
        invalid["items"] = [{"pizza_id": "intrus", "quantity": 1}]
        self.assertEqual(self.app.call("create_order", invalid)[0], 400)
        invalid["items"] = [{"pizza_id": "reine", "quantity": 0}]
        self.assertEqual(self.app.call("create_order", invalid)[0], 400)
        invalid = self.valid_order()
        invalid["phone"] = "abc"
        self.assertEqual(self.app.call("create_order", invalid)[0], 400)
        invalid = self.valid_order()
        invalid["items"] = [{"pizza_id": "reine", "quantity": 6}, {"pizza_id": "reine", "quantity": 5}]
        self.assertEqual(self.app.call("create_order", invalid)[0], 400)
        spoofed = self.valid_order()
        spoofed["items"][0]["price_cents"] = 1
        result = self.create_order(spoofed)
        self.assertEqual(result["subtotal_cents"], 3450)
        self.assertEqual(result["delivery_cents"], 290)
        self.assertEqual(result["total_cents"], 3740)

    def test_private_tracking_restaurant_auth_and_strict_status_flow(self):
        order = self.create_order()
        path = f'/api/orders/{order["number"]}'
        self.assertEqual(self.app.call("get_order", urlparse(path))[0], 404)
        self.assertEqual(self.app.call("get_order", urlparse(path + "?token=mauvais"))[0], 404)
        self.assertEqual(self.app.call("get_restaurant_orders")[0], 401)
        self.assertEqual(self.app.call("restaurant_login", {"password": "incorrect"})[0], 401)
        cookie = self.login()
        self.assertEqual(self.app.call("advance_order", order["number"], {"status": "livraison"})[0], 409)
        self.app.cookie()
        self.assertEqual(self.app.call("advance_order", order["number"], {"status": "preparation"})[0], 401)
        self.app.cookie(cookie)
        status, data, _ = self.app.call("advance_order", order["number"], {"status": "preparation"})
        self.assertEqual((status, data["status"]), (200, "preparation"))
        status, data, _ = self.app.call("advance_order", order["number"], {"status": "livraison"})
        self.assertEqual((status, data["status"]), (200, "livraison"))
        status, data, _ = self.app.call("advance_order", order["number"], {"status": "livree"})
        self.assertEqual((status, data["status"]), (200, "livree"))
        self.assertEqual(self.app.call("advance_order", order["number"], {"status": "livree"})[0], 409)
        status, data, _ = self.app.call("get_order", urlparse(path + "?token=" + order["tracking_token"]))
        self.assertEqual((status, data["status"]), (200, "livree"))
        self.assertNotIn("phone", data)

    def test_order_survives_application_recreation(self):
        order = self.create_order()
        restarted = HandlerHarness(self.db_path)
        status, data, _ = restarted.call("get_order", urlparse(f'/api/orders/{order["number"]}?token={order["tracking_token"]}'))
        self.assertEqual(status, 200)
        self.assertEqual(data["total_cents"], 3740)
        status, _, headers = restarted.call("restaurant_login", {"password": "secret-test"})
        self.assertEqual(status, 200)
        restarted.cookie(headers["Set-Cookie"].split(";", 1)[0])
        status, orders, _ = restarted.call("get_restaurant_orders")
        self.assertEqual(status, 200)
        self.assertEqual([item["number"] for item in orders], [order["number"]])


if __name__ == "__main__":
    unittest.main(verbosity=2)
