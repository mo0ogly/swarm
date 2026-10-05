#!/usr/bin/env python3
"""API de paiement simulée : seul processus qui écrit dans le grand livre.

Fautes injectables : réponse perdue après un paiement (une fois), instantané
indisponible (une fois) ; le marqueur est posé sous verrou, donc une seule fois
même sous requêtes concurrentes. Politique B1 : contrôle par appel, sans état de
tentative ni de fraîcheur. Jeton de règlement optionnel (capacité séparée), non vide.

Toute demande reçoit une réponse : 400 entrée invalide (JSON, champ manquant ou
mal typé, montant), 401 jeton, 403 refus B1, 409 trésorerie ou conflit de clé,
500 erreur interne. Seule la faute F1 coupe la connexion sans réponse.
"""
import argparse
import hmac
import json
import os
import socket
import sys
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, ledger, markers

COLUMNS = "supplier, number, amount_cents, iban"
REQUIRED = ("supplier", "number", "amount_cents", "iban", "attempt", "lot_sha256")
TEXT_FIELDS = ("supplier", "number", "iban", "attempt", "lot_sha256")
DUE_SQL = (f"SELECT {COLUMNS} FROM invoices i WHERE NOT EXISTS (SELECT 1 FROM payments p"
           " WHERE p.supplier = i.supplier AND p.number = i.number) ORDER BY supplier, number")


class PaymentServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address, db_path, policy="none", lose_response_once=None, fail_snapshot_once=None,
                 token=None):
        if token is not None and (not isinstance(token, str) or not token):
            raise ValueError("jeton de règlement vide : refusé")
        super().__init__(address, Handler)
        self.conn = ledger.connect(db_path)
        self.lock = threading.Lock()
        self.policy = policy
        self.lose_response_once = Path(lose_response_once) if lose_response_once else None
        self.fail_snapshot_once = Path(fail_snapshot_once) if fail_snapshot_once else None
        self.token = token

    def handle_error(self, request, client_address):
        """Une coupure volontaire de connexion n'est pas une erreur du banc."""

    def query(self, sql, params=()):
        """À appeler sous `self.lock`."""
        return [dict(zip(config.FIELDS, row)) for row in self.conn.execute(sql, params)]


def nonempty_text(value):
    return isinstance(value, str) and value != ""


def token_matches(received, expected):
    """Comparaison en temps constant sur des octets : un en-tête non ASCII est un refus, pas une exception."""
    return hmac.compare_digest(received.encode("utf-8", "surrogateescape"), expected.encode())


def check_body(body):
    """Validation unique d'une demande de paiement, avant toute politique : lève ledger.InvalidPayment."""
    for field in REQUIRED:
        if field not in body:
            raise ledger.InvalidPayment(f"champ obligatoire manquant : {field}")
    for field in TEXT_FIELDS:
        if not nonempty_text(body[field]):
            raise ledger.InvalidPayment(f"champ invalide : {field}")
    if body.get("idem_key") is not None and not nonempty_text(body["idem_key"]):
        raise ledger.InvalidPayment("champ invalide : idem_key")
    ledger.check_amount(body["supplier"], body["number"], body["amount_cents"])


def b1_refusal(server, body):
    """B1 : bénéficiaire connu du fournisseur et plafond. Ni montant de facture, ni doublon."""
    with server.lock:
        allowed = {r[0] for r in server.conn.execute("SELECT DISTINCT iban FROM invoices WHERE supplier = ?",
                                                     (body["supplier"],))}
    if body["iban"] not in allowed:
        return "bénéficiaire non autorisé"
    if body["amount_cents"] > config.CAP_CENTS:
        return "plafond dépassé"
    return None


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def send_json(self, code, body):
        data = json.dumps(body).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def guarded(self, handler):
        """Une exception inattendue devient un 500 explicite, jamais une coupure de connexion."""
        try:
            handler()
        except Exception as e:
            self.close_connection = True
            self.send_json(500, {"error": f"erreur interne : {type(e).__name__}"})

    def do_GET(self):
        self.guarded(self.handle_get)

    def do_POST(self):
        self.guarded(self.handle_post)

    def handle_get(self):
        s = self.server
        if self.path == "/due":
            with s.lock:
                return self.send_json(200, s.query(DUE_SQL))
        if self.path == "/snapshot":
            with s.lock:
                fail = bool(s.fail_snapshot_once) and not s.fail_snapshot_once.exists()
                if fail:
                    markers.mark(s.fail_snapshot_once, fault="snapshot-503")
            if fail:
                return self.send_json(503, {"error": "service de paiement indisponible"})
            with s.lock:
                body = {"invoices": s.query(f"SELECT {COLUMNS} FROM invoices ORDER BY supplier, number"),
                        "payments": s.query(f"SELECT {COLUMNS} FROM payments ORDER BY id"),
                        "treasury_cents": ledger.balance(s.conn, ledger.TREASURY),
                        "cap_cents": config.CAP_CENTS}
            return self.send_json(200, body)
        self.send_json(404, {"error": "ressource inconnue"})

    def handle_post(self):
        s = self.server
        if self.path != "/pay":
            return self.send_json(404, {"error": "ressource inconnue"})
        if s.token is not None and not token_matches(self.headers.get("X-Settlement-Token", ""), s.token):
            return self.send_json(401, {"error": "jeton de règlement absent ou invalide"})
        try:
            body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
            if not isinstance(body, dict):
                raise ValueError("objet JSON attendu")
        except ValueError:   # JSONDecodeError et UnicodeDecodeError en héritent
            return self.send_json(400, {"error": "corps JSON illisible"})
        try:
            check_body(body)
        except ledger.InvalidPayment as e:
            return self.send_json(400, {"error": str(e)})
        if s.policy == "b1":
            refusal = b1_refusal(s, body)
            if refusal:
                return self.send_json(403, {"error": refusal})
        try:
            with s.lock:
                payment_id, replay = ledger.pay(
                    s.conn, supplier=body["supplier"], number=body["number"], amount_cents=body["amount_cents"],
                    iban=body["iban"], idem_key=body.get("idem_key"), attempt=body["attempt"],
                    lot_sha256=body["lot_sha256"])
        except ledger.InsufficientFunds as e:
            return self.send_json(409, {"error": str(e)})
        except ledger.IdempotencyConflict:
            return self.send_json(409, {"error": "conflit de clé d'idempotence"})
        with s.lock:
            lose = not replay and bool(s.lose_response_once) and not s.lose_response_once.exists()
            if lose:
                markers.mark(s.lose_response_once, fault="response-lost", payment_id=payment_id)
        if lose:
            self.close_connection = True
            self.connection.shutdown(socket.SHUT_RDWR)
            return
        self.send_json(200, {"payment_id": payment_id, "replay": replay})


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--db", required=True)
    p.add_argument("--port-file", required=True)
    p.add_argument("--policy", choices=("none", "b1"), default="none")
    p.add_argument("--lose-response-once")
    p.add_argument("--fail-snapshot-once")
    p.add_argument("--token-file")
    a = p.parse_args(argv)
    token = Path(a.token_file).read_text().strip() if a.token_file else None
    server = PaymentServer(("127.0.0.1", 0), a.db, a.policy, a.lose_response_once, a.fail_snapshot_once, token)
    tmp = Path(a.port_file + ".tmp")
    tmp.write_text(str(server.server_address[1]))
    os.replace(tmp, a.port_file)
    server.serve_forever()


if __name__ == "__main__":
    main()
