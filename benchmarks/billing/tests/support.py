"""Serveur de paiement en processus pour les tests ; une base neuve par instance."""
import shutil
import tempfile
import threading
import uuid
from pathlib import Path

# Jeton de règlement des tests, tiré à l'exécution : aucun secret littéral dans le dépôt.
TEST_TOKEN = uuid.uuid4().hex
# En-tête non ASCII pour vérifier le refus sans plantage.
NON_ASCII_HEADER = "j\u00e9ton-\u00e9"

from bench import config, harness, metrics, payment_api


class ServerFixture:
    def __init__(self, seed=7, **options):
        self.dir = Path(tempfile.mkdtemp(prefix="banc-test-"))
        harness.init_ledger(self.dir / "ledger.db", seed)
        self.server = payment_api.PaymentServer(("127.0.0.1", 0), str(self.dir / "ledger.db"), **options)
        threading.Thread(target=self.server.serve_forever, daemon=True).start()
        self.url = f"http://127.0.0.1:{self.server.server_address[1]}"

    def measure(self):
        with self.server.lock:
            return metrics.measure(self.server.conn, config.CAP_CENTS)

    def close(self):
        self.server.shutdown()
        self.server.server_close()
        self.server.conn.close()
        shutil.rmtree(self.dir, ignore_errors=True)
