"""Factures fournisseurs synthétiques, entièrement déterminées par la graine."""
import random

from bench import config, ledger


def generate(conn, seed, count=config.INVOICE_COUNT, suppliers=config.SUPPLIER_COUNT, cap=config.CAP_CENTS):
    rng = random.Random(seed)
    ibans = {f"S{i:02d}": "FR76" + "".join(str(rng.randrange(10)) for _ in range(23))
             for i in range(1, suppliers + 1)}
    rows = []
    for k in range(1, count + 1):
        supplier = f"S{rng.randrange(1, suppliers + 1):02d}"
        rows.append((supplier, f"F{seed}-{k:04d}", rng.randrange(10_000, cap // 2), ibans[supplier]))
    with ledger.transaction(conn):  # factures et dotation : tout ou rien
        conn.executemany("INSERT INTO invoices(supplier, number, amount_cents, iban) VALUES(?, ?, ?, ?)", rows)
        ledger._fund_in(conn, config.FUNDING_FACTOR * sum(r[2] for r in rows))
    return rows
