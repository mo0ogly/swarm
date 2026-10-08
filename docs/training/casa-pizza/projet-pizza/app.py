#!/usr/bin/env python3
"""Casa Pizza — serveur local sans dépendance externe."""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import hmac
import json
import os
import re
import secrets
import sqlite3
import threading
from datetime import datetime, timezone
from http import HTTPStatus
from http.cookies import SimpleCookie
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

ROOT = Path(__file__).resolve().parent
STATIC = ROOT / "static"
DELIVERY_CENTS = 290
MINIMUM_CENTS = 1000
STATUSES = ("recue", "preparation", "livraison", "livree")
PIZZAS = (
    {"id": "margherita", "name": "Margherita", "ingredients": "Tomate, mozzarella, basilic", "price_cents": 950, "vegetarian": True, "image": "margherita.svg"},
    {"id": "reine", "name": "Reine", "ingredients": "Tomate, mozzarella, jambon, champignons", "price_cents": 1250, "vegetarian": False, "image": "reine.svg"},
    {"id": "quatre-fromages", "name": "Quatre fromages", "ingredients": "Mozzarella, chèvre, gorgonzola, parmesan", "price_cents": 1350, "vegetarian": True, "image": "fromages.svg"},
    {"id": "vegetarienne", "name": "Végétarienne", "ingredients": "Tomate, mozzarella, poivron, courgette, olive", "price_cents": 1200, "vegetarian": True, "image": "vegetarienne.svg"},
    {"id": "diavola", "name": "Diavola", "ingredients": "Tomate, mozzarella, salami piquant", "price_cents": 1300, "vegetarian": False, "image": "diavola.svg"},
    {"id": "calzone", "name": "Calzone", "ingredients": "Tomate, mozzarella, jambon, œuf", "price_cents": 1400, "vegetarian": False, "image": "calzone.svg"},
)
PIZZA_BY_ID = {pizza["id"]: pizza for pizza in PIZZAS}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


@contextmanager
def connect_db(path: str):
    connection = sqlite3.connect(path, timeout=10)
    connection.row_factory = sqlite3.Row
    connection.execute("PRAGMA foreign_keys = ON")
    try:
        with connection:
            yield connection
    finally:
        connection.close()


def init_db(path: str) -> None:
    with connect_db(path) as db:
        db.executescript(
            """
            CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY,
                public_id TEXT NOT NULL UNIQUE,
                tracking_hash TEXT NOT NULL,
                customer_name TEXT NOT NULL,
                address TEXT NOT NULL,
                phone TEXT NOT NULL,
                subtotal_cents INTEGER NOT NULL,
                delivery_cents INTEGER NOT NULL,
                total_cents INTEGER NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS order_items (
                order_id INTEGER NOT NULL REFERENCES orders(id) ON DELETE CASCADE,
                pizza_id TEXT NOT NULL,
                name TEXT NOT NULL,
                unit_price_cents INTEGER NOT NULL,
                quantity INTEGER NOT NULL
            );
            """
        )


def tracking_hash(token: str) -> str:
    return hashlib.sha256(token.encode("utf-8")).hexdigest()


def public_order(row: sqlite3.Row, items: list[sqlite3.Row]) -> dict:
    return {
        "number": row["public_id"],
        "customer_name": row["customer_name"],
        "address": row["address"],
        "items": [dict(item) for item in items],
        "subtotal_cents": row["subtotal_cents"],
        "delivery_cents": row["delivery_cents"],
        "total_cents": row["total_cents"],
        "status": row["status"],
        "created_at": row["created_at"],
        "updated_at": row["updated_at"],
    }


class CasaPizzaServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address, db_path: str, restaurant_password: str):
        super().__init__(address, CasaPizzaHandler)
        self.db_path = db_path
        self.restaurant_password = restaurant_password
        self.sessions: set[str] = set()
        self.sessions_lock = threading.Lock()


class CasaPizzaHandler(BaseHTTPRequestHandler):
    server: CasaPizzaServer

    def log_message(self, fmt: str, *args) -> None:
        # Évite d'inscrire les jetons de suivi présents dans les URL.
        return

    def send_json(self, status: int, data: dict | list, headers: dict | None = None) -> None:
        raw = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(status)
        self.send_header("Content-Type", "application/json; charset=utf-8")
        self.send_header("Content-Length", str(len(raw)))
        self.send_header("Cache-Control", "no-store")
        self.send_header("X-Content-Type-Options", "nosniff")
        for key, value in (headers or {}).items():
            self.send_header(key, value)
        self.end_headers()
        self.wfile.write(raw)

    def read_json(self) -> dict:
        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError as exc:
            raise ValueError("Corps invalide.") from exc
        if length < 1 or length > 100_000:
            raise ValueError("Corps invalide.")
        try:
            value = json.loads(self.rfile.read(length))
        except (json.JSONDecodeError, UnicodeDecodeError) as exc:
            raise ValueError("JSON invalide.") from exc
        if not isinstance(value, dict):
            raise ValueError("Objet JSON attendu.")
        return value

    def session_token(self) -> str | None:
        cookie = SimpleCookie(self.headers.get("Cookie", ""))
        morsel = cookie.get("casa_session")
        return morsel.value if morsel else None

    def restaurant_authorized(self) -> bool:
        token = self.session_token()
        if not token:
            return False
        with self.server.sessions_lock:
            return token in self.server.sessions

    def require_restaurant(self) -> bool:
        if self.restaurant_authorized():
            return True
        self.send_json(HTTPStatus.UNAUTHORIZED, {"error": "Authentification restaurant requise."})
        return False

    def serve_file(self, path: Path) -> None:
        try:
            path = path.resolve()
            path.relative_to(STATIC.resolve())
            raw = path.read_bytes()
        except (ValueError, FileNotFoundError, IsADirectoryError):
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        suffixes = {".html": "text/html; charset=utf-8", ".css": "text/css; charset=utf-8", ".js": "text/javascript; charset=utf-8", ".svg": "image/svg+xml"}
        content_type = suffixes.get(path.suffix)
        if not content_type:
            self.send_error(HTTPStatus.NOT_FOUND)
            return
        self.send_response(HTTPStatus.OK)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(raw)))
        self.send_header("X-Content-Type-Options", "nosniff")
        self.send_header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'")
        self.end_headers()
        self.wfile.write(raw)

    def do_GET(self) -> None:
        parsed = urlparse(self.path)
        if parsed.path == "/api/catalog":
            self.send_json(HTTPStatus.OK, list(PIZZAS))
            return
        if parsed.path.startswith("/api/orders/"):
            self.get_order(parsed)
            return
        if parsed.path == "/api/restaurant/orders":
            self.get_restaurant_orders()
            return
        routes = {"/": "index.html", "/restaurant": "restaurant.html"}
        if parsed.path in routes:
            self.serve_file(STATIC / routes[parsed.path])
            return
        if parsed.path.startswith("/static/"):
            relative = parsed.path.removeprefix("/static/")
            self.serve_file(STATIC / relative)
            return
        self.send_error(HTTPStatus.NOT_FOUND)

    def do_POST(self) -> None:
        parsed = urlparse(self.path)
        try:
            body = self.read_json()
        except ValueError as exc:
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": str(exc)})
            return
        if parsed.path == "/api/orders":
            self.create_order(body)
        elif parsed.path == "/api/restaurant/login":
            self.restaurant_login(body)
        elif parsed.path.startswith("/api/restaurant/orders/") and parsed.path.endswith("/status"):
            self.advance_order(parsed.path.split("/")[4], body)
        else:
            self.send_error(HTTPStatus.NOT_FOUND)

    def create_order(self, body: dict) -> None:
        name = body.get("customer_name")
        address = body.get("address")
        phone = body.get("phone")
        items = body.get("items")
        if not all(isinstance(value, str) and value.strip() for value in (name, address, phone)):
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Nom, adresse et téléphone sont obligatoires."})
            return
        if not (2 <= len(name.strip()) <= 80 and 5 <= len(address.strip()) <= 240 and re.fullmatch(r"[0-9 +().-]{6,30}", phone.strip())):
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Nom, adresse ou téléphone invalide."})
            return
        if not isinstance(items, list) or not items:
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Le panier est vide."})
            return
        quantities: dict[str, int] = {}
        for requested in items:
            if not isinstance(requested, dict):
                self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Produit invalide."})
                return
            pizza = PIZZA_BY_ID.get(requested.get("pizza_id"))
            quantity = requested.get("quantity")
            if pizza is None or isinstance(quantity, bool) or not isinstance(quantity, int) or not 1 <= quantity <= 10:
                self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Produit ou quantité invalide."})
                return
            quantities[pizza["id"]] = quantities.get(pizza["id"], 0) + quantity
            if quantities[pizza["id"]] > 10:
                self.send_json(HTTPStatus.BAD_REQUEST, {"error": "La quantité maximale est de 10 par pizza."})
                return
        normalized = [(PIZZA_BY_ID[pizza_id], quantity) for pizza_id, quantity in quantities.items()]
        subtotal = sum(pizza["price_cents"] * quantity for pizza, quantity in normalized)
        if subtotal < MINIMUM_CENTS:
            self.send_json(HTTPStatus.BAD_REQUEST, {"error": "Le minimum hors livraison est de 10,00 €."})
            return
        token = secrets.token_urlsafe(24)
        public_id = "CP-" + secrets.token_hex(4).upper()
        now = utc_now()
        with connect_db(self.server.db_path) as db:
            cursor = db.execute(
                "INSERT INTO orders(public_id, tracking_hash, customer_name, address, phone, subtotal_cents, delivery_cents, total_cents, status, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (public_id, tracking_hash(token), name.strip(), address.strip(), phone.strip(), subtotal, DELIVERY_CENTS, subtotal + DELIVERY_CENTS, STATUSES[0], now, now),
            )
            db.executemany(
                "INSERT INTO order_items(order_id, pizza_id, name, unit_price_cents, quantity) VALUES (?, ?, ?, ?, ?)",
                [(cursor.lastrowid, pizza["id"], pizza["name"], pizza["price_cents"], quantity) for pizza, quantity in normalized],
            )
        self.send_json(HTTPStatus.CREATED, {"number": public_id, "tracking_token": token, "subtotal_cents": subtotal, "delivery_cents": DELIVERY_CENTS, "total_cents": subtotal + DELIVERY_CENTS, "status": STATUSES[0]})

    def get_order(self, parsed) -> None:
        public_id = parsed.path.removeprefix("/api/orders/")
        token = parse_qs(parsed.query).get("token", [""])[0]
        with connect_db(self.server.db_path) as db:
            row = db.execute("SELECT * FROM orders WHERE public_id = ?", (public_id,)).fetchone()
            if row is None or not hmac.compare_digest(row["tracking_hash"], tracking_hash(token)):
                self.send_json(HTTPStatus.NOT_FOUND, {"error": "Commande introuvable."})
                return
            items = db.execute("SELECT pizza_id, name, unit_price_cents, quantity FROM order_items WHERE order_id = ?", (row["id"],)).fetchall()
        self.send_json(HTTPStatus.OK, public_order(row, items))

    def restaurant_login(self, body: dict) -> None:
        password = body.get("password", "")
        if not isinstance(password, str) or not hmac.compare_digest(password, self.server.restaurant_password):
            self.send_json(HTTPStatus.UNAUTHORIZED, {"error": "Mot de passe incorrect."})
            return
        token = secrets.token_urlsafe(32)
        with self.server.sessions_lock:
            self.server.sessions.add(token)
        self.send_json(HTTPStatus.OK, {"ok": True}, {"Set-Cookie": f"casa_session={token}; HttpOnly; SameSite=Strict; Path=/"})

    def get_restaurant_orders(self) -> None:
        if not self.require_restaurant():
            return
        with connect_db(self.server.db_path) as db:
            rows = db.execute("SELECT * FROM orders ORDER BY id DESC").fetchall()
            result = []
            for row in rows:
                items = db.execute("SELECT pizza_id, name, unit_price_cents, quantity FROM order_items WHERE order_id = ?", (row["id"],)).fetchall()
                result.append(public_order(row, items))
        self.send_json(HTTPStatus.OK, result)

    def advance_order(self, public_id: str, body: dict) -> None:
        if not self.require_restaurant():
            return
        requested_status = body.get("status")
        with connect_db(self.server.db_path) as db:
            row = db.execute("SELECT status FROM orders WHERE public_id = ?", (public_id,)).fetchone()
            if row is None:
                self.send_json(HTTPStatus.NOT_FOUND, {"error": "Commande introuvable."})
                return
            current_index = STATUSES.index(row["status"])
            expected = STATUSES[current_index + 1] if current_index + 1 < len(STATUSES) else None
            if requested_status != expected:
                self.send_json(HTTPStatus.CONFLICT, {"error": "Transition de statut invalide."})
                return
            db.execute("UPDATE orders SET status = ?, updated_at = ? WHERE public_id = ?", (requested_status, utc_now(), public_id))
        self.send_json(HTTPStatus.OK, {"number": public_id, "status": requested_status})


def make_server(host: str, port: int, db_path: str, restaurant_password: str) -> CasaPizzaServer:
    init_db(db_path)
    return CasaPizzaServer((host, port), db_path, restaurant_password)


def main() -> None:
    parser = argparse.ArgumentParser(description="Serveur local Casa Pizza")
    parser.add_argument("--host", default="127.0.0.1", choices=("127.0.0.1",), help="adresse locale (127.0.0.1 uniquement)")
    parser.add_argument("--port", type=int, default=18841)
    parser.add_argument("--db", default=str(ROOT / "casa_pizza.db"))
    args = parser.parse_args()
    password = os.environ.get("CASA_PIZZA_RESTAURANT_PASSWORD", "atelier")
    server = make_server(args.host, args.port, args.db, password)
    print(f"Casa Pizza : http://{args.host}:{server.server_port}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nArrêt de Casa Pizza.")
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
