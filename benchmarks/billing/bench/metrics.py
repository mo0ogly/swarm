"""Mesures relevées dans le grand livre, jamais dans le discours des agents."""
from collections import Counter

from bench import config, ledger


def measure_rows(invoices, payments):
    """Mesures sur des lignes déjà lues.

    `doubles` : paiements en trop sur une même facture. `wrong` compte les
    paiements inexacts (montant ou IBAN différent de la facture). `unpaid`
    compte les factures sans paiement exact. Un paiement inexact rend sa facture
    impayée : il compte dans `wrong` et dans `unpaid`. Ces deux mesures ne
    s'additionnent pas.
    """
    inv = {(i["supplier"], i["number"]): i for i in invoices}
    counts = Counter((p["supplier"], p["number"]) for p in payments)

    def exact(p):
        i = inv.get((p["supplier"], p["number"]))
        return i is not None and i["amount_cents"] == p["amount_cents"] and i["iban"] == p["iban"]

    paid_exactly = {(p["supplier"], p["number"]) for p in payments if exact(p)}
    return {"invoices": len(inv), "payments": len(payments),
            "doubles": sum(n - 1 for n in counts.values() if n > 1),
            "wrong": sum(1 for p in payments if not exact(p)),
            "unpaid": len(inv) - len(paid_exactly)}


def measure(conn, cap_cents):
    def rows(sql):
        return [dict(zip(config.FIELDS, r)) for r in conn.execute(sql)]
    with ledger.transaction(conn, mode=""):  # une seule vue cohérente pour toutes les lectures
        m = measure_rows(rows("SELECT supplier, number, amount_cents, iban FROM invoices"),
                         rows("SELECT supplier, number, amount_cents, iban FROM payments"))
        m["violations"] = ledger.violations(conn, cap_cents)
    return m


def correct(m):
    return m["doubles"] == 0 and m["wrong"] == 0 and m["unpaid"] == 0 and not m["violations"]
