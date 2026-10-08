"""Grand livre simulé en partie double (SQLite). Données synthétiques uniquement.

Contrat de concurrence : une connexion partagée entre threads doit être utilisée
sous un verrou détenu par l'appelant ; payment_api le fait. Le module ne pose
aucun verrou interne. Des connexions distinctes sur un même fichier sont
sérialisées par SQLite (WAL, BEGIN IMMEDIATE).
"""
import sqlite3
import time
from contextlib import contextmanager

TREASURY = "treasury"
EQUITY = "equity"
SCHEMA = """
CREATE TABLE IF NOT EXISTS invoices(
  supplier TEXT NOT NULL, number TEXT NOT NULL,
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0), iban TEXT NOT NULL,
  PRIMARY KEY(supplier, number));
CREATE TABLE IF NOT EXISTS payments(
  id INTEGER PRIMARY KEY, idem_key TEXT UNIQUE,
  supplier TEXT NOT NULL, number TEXT NOT NULL,
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0), iban TEXT NOT NULL,
  attempt TEXT NOT NULL, lot_sha256 TEXT NOT NULL, created_at REAL NOT NULL);
CREATE TABLE IF NOT EXISTS entries(
  id INTEGER PRIMARY KEY, payment_id INTEGER, account TEXT NOT NULL, amount_cents INTEGER NOT NULL);
"""


class InsufficientFunds(Exception):
    """La trésorerie ne couvre pas le paiement : refus, aucune écriture."""


class IdempotencyConflict(Exception):
    """Clé déjà utilisée pour un paiement de contenu différent : refus, aucune écriture."""


class InvalidPayment(ValueError):
    """Montant non entier ou non strictement positif : refus avant toute écriture."""


@contextmanager
def transaction(conn, mode="IMMEDIATE"):
    """BEGIN <mode> ; COMMIT. Toute exception (BaseException comprise, échec du COMMIT compris)
    annule puis est relancée telle quelle."""
    conn.execute(f"BEGIN {mode}".strip())
    try:
        yield conn
        conn.execute("COMMIT")
    except BaseException as exc:
        if conn.in_transaction:
            try:
                conn.execute("ROLLBACK")
            except sqlite3.Error as rollback_error:
                exc.add_note(f"ROLLBACK échoué : {rollback_error}")
        raise


def connect(path):
    conn = sqlite3.connect(path, timeout=10, isolation_level=None, check_same_thread=False)
    mode = conn.execute("PRAGMA journal_mode=WAL").fetchone()[0]
    if str(path) != ":memory:" and str(mode).lower() != "wal":
        conn.close()
        raise RuntimeError(f"journal WAL indisponible pour {path} (mode obtenu : {mode})")
    conn.executescript(SCHEMA)
    return conn


def _fund_in(conn, cents):
    """Dotation de trésorerie, à appeler dans une transaction ouverte."""
    conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, ?, ?)", (TREASURY, cents))
    conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, ?, ?)", (EQUITY, -cents))


def fund(conn, cents):
    with transaction(conn):
        _fund_in(conn, cents)


def balance(conn, account):
    return conn.execute("SELECT COALESCE(SUM(amount_cents), 0) FROM entries WHERE account = ?",
                        (account,)).fetchone()[0]


def check_amount(supplier, number, amount_cents):
    """Lève InvalidPayment si le montant n'est pas un entier (booléen exclu) strictement positif."""
    if not isinstance(amount_cents, int) or isinstance(amount_cents, bool) or amount_cents <= 0:
        raise InvalidPayment(f"montant invalide pour {supplier}/{number} : {amount_cents!r}")


def pay(conn, *, supplier, number, amount_cents, iban, idem_key, attempt, lot_sha256):
    """Retourne (payment_id, replay).

    Une clé déjà vue au contenu identique renvoie le paiement existant, sans effet ;
    au contenu différent, elle lève IdempotencyConflict sans écriture.
    """
    check_amount(supplier, number, amount_cents)
    with transaction(conn):
        if idem_key is not None:
            row = conn.execute("SELECT id, supplier, number, amount_cents, iban FROM payments WHERE idem_key = ?",
                               (idem_key,)).fetchone()
            if row:
                if row[1:] != (supplier, number, amount_cents, iban):
                    raise IdempotencyConflict(f"clé {idem_key} déjà utilisée pour un autre paiement")
                return row[0], True
        if balance(conn, TREASURY) < amount_cents:
            raise InsufficientFunds(f"trésorerie insuffisante pour {supplier}/{number}")
        pid = conn.execute(
            "INSERT INTO payments(idem_key, supplier, number, amount_cents, iban, attempt, lot_sha256, created_at)"
            " VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
            (idem_key, supplier, number, amount_cents, iban, attempt, lot_sha256, time.time())).lastrowid
        conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(?, ?, ?)",
                     (pid, TREASURY, -amount_cents))
        conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(?, ?, ?)",
                     (pid, "supplier:" + supplier, amount_cents))
        return pid, False


def violations(conn, cap_cents):
    """Invariants comptables. Le doublon de facture est une mesure, pas une violation comptable.

    Le plafond n'est pas imposé par le grand livre (la banque simulée ne le connaît
    pas ; c'est le contrôle B1 qui le fait respecter) : il est seulement constaté ici.
    """
    found = [f"écriture déséquilibrée : {'dotation' if pid is None else f'paiement {pid}'}"
             for pid, total in conn.execute(
                 "SELECT payment_id, SUM(amount_cents) FROM entries GROUP BY payment_id ORDER BY payment_id")
             if total != 0]
    if balance(conn, TREASURY) < 0:
        found.append("trésorerie négative")
    found += [f"écritures incohérentes : paiement {pid}" for (pid,) in conn.execute(
        "SELECT p.id FROM payments p"
        " WHERE (SELECT COUNT(*) FROM entries e WHERE e.payment_id = p.id) != 2"
        " OR NOT EXISTS (SELECT 1 FROM entries e WHERE e.payment_id = p.id"
        "                AND e.account = ? AND e.amount_cents = -p.amount_cents)"
        " OR NOT EXISTS (SELECT 1 FROM entries e WHERE e.payment_id = p.id"
        "                AND e.account = 'supplier:' || p.supplier AND e.amount_cents = p.amount_cents)"
        " ORDER BY p.id", (TREASURY,))]
    found += [f"écriture orpheline : paiement {pid}" for (pid,) in conn.execute(
        "SELECT DISTINCT e.payment_id FROM entries e WHERE e.payment_id IS NOT NULL"
        " AND NOT EXISTS (SELECT 1 FROM payments p WHERE p.id = e.payment_id) ORDER BY e.payment_id")]
    found += [f"plafond dépassé : paiement {pid}"
              for (pid,) in conn.execute("SELECT id FROM payments WHERE amount_cents > ? ORDER BY id", (cap_cents,))]
    return found
