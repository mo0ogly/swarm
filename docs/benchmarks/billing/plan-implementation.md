# Banc de facturation simulé — plan d'implémentation

> Document importé le 5 octobre 2026 depuis le dépôt Wattson, où le banc a été
> conçu (commit `53cf1b702`). Les chemins ont été adaptés à ce dépôt ; les règles
> d'exécution propres à Wattson (conteneur de développement, en-têtes, workflows)
> sont conservées comme historique et ne s'appliquent pas ici.

> **Pour l'agent d'exécution :** sous-skill requise : `executing-plans` (ou
> `subagent-driven-development`), tâche par tâche, avec arrêt aux points STOP.

**Objectif :** construire un banc reproductible qui mesure, sur un paiement de
factures fournisseurs, ce qu'empêchent une clé d'idempotence seule, un contrôle
par appel (B1) et le moteur Swarm complet (S), sous huit fautes injectées.

**Architecture :** un grand livre SQLite en partie double n'est écrit que par une
API de paiement simulée (processus séparé, fautes injectables). Un préparateur
propose `lot.json` ; en S, Swarm valide le lot (`check_lot.py`) puis une tâche de
règlement déterministe le consomme par échange et l'exécute. En B0/B1, un harnais
naïf enchaîne préparation, règlement et revue finale après l'effet. Chaque
exécution vit dans une racine temporaire isolée et produit une ligne JSONL.

**Pile :** Python 3.13 bibliothèque standard uniquement (sqlite3, http.server,
urllib, unittest), binaire Go `bin/swarm`.

**Spécification :** `docs/benchmarks/billing/conception.md`.

---

## Règles d'exécution (à lire avant la tâche 0)

- Répertoire de travail : `benchmarks/billing`.
  Commande de tests unitaires, notée **UT** plus bas :
  `python3 -m unittest discover -s tests -t . -v`
- Le banc s'exécute **sur l'hôte** : bibliothèque standard seulement, aucune
  dépendance de l'application Wattson (ce n'est pas du code applicatif). Ne
  jamais importer `utilitaires`, `core`, ni rien de `flaskProject/` hors `benchmarks/billing`.
- **Ne pas modifier `./`.** Si une faute ne peut pas être
  reproduite sans changer le moteur : STOP, consigner l'observation, prévenir l'opérateur.
- Commits : uniquement si l'opérateur l'a demandé. Toujours `/usr/bin/git`,
  chemins explicites (jamais `git add -A`), aucune ligne `Co-Authored-By`, aucune
  mention d'outil d'IA. Des sessions parallèles commitent sur ce dépôt : vérifier
  `/usr/bin/git diff --cached --name-only` avant chaque commit.
- Données synthétiques uniquement. Aucun secret dans les journaux ni les JSONL.
- Convention `tools/` : docstring de module en tête, pas d'en-tête ANSSI applicatif.

## Amendements après revue du lot A (2026-10-05)

Ces amendements priment sur le code des tâches 2 et 3 ci-dessous.

- `ledger.pay` : une clé déjà vue avec un contenu différent (fournisseur,
  facture, montant, IBAN) lève `ledger.IdempotencyConflict`, sans écriture,
  comme une API de paiement réelle. Un montant non entier ou ≤ 0 lève
  `ledger.InvalidPayment` (`CHECK(amount_cents > 0)` en base). Le plafond reste
  hors du grand livre : seul B1 le contrôle à l'appel.
- `ledger.transaction(conn)` : gestionnaire de contexte (BEGIN IMMEDIATE,
  ROLLBACK seulement si `conn.in_transaction`), utilisé par `pay`, `fund` et
  `invoices.generate` (factures et dotation dans une seule transaction).
- `metrics.measure` lit dans une seule transaction de lecture ; `violations`
  détecte aussi les écritures incohérentes avec le paiement.
- `connect` refuse un fichier dont le journal n'est pas en WAL.
- **Tâche 4, `payment_api.do_POST` :** `IdempotencyConflict` → HTTP 409
  `{"error": "conflit de clé d'idempotence"}` ; `InvalidPayment` → HTTP 400.
  `settle.pay_line` les traite déjà comme des refus définitifs (HTTPError).

## Décision du 2026-10-05 : condition S sur le modèle hiérarchique

Le lot D (tâche 8) a montré que le Swarm de l'arbre de travail impose une
organisation hiérarchique à toute mission autonome (`organization.go`, non
commité) et refuse `exchange send` dans ce cas ; `agent_exchange.go` n'est pas
commité non plus. Décision de l'opérateur : attendre le commit de Swarm par la
session qui le développe, construire `bin/swarm` depuis ce commit, puis refaire
la condition S sur le modèle hiérarchique (responsable de mission, remise par
planning handoff, contrôles explicites). Les tâches 8 à 11 seront réécrites
après lecture du code commité. Le code actuel de `run_s.py` et `provider_s.py`
(infrastructure, nettoyage, contrôles d'erreur) est conservé comme base.

## Arborescence cible

```
benchmarks/billing/
  README.md
  bench/
    __init__.py
    config.py            constantes figées du banc
    markers.py           marqueurs d'injection horodatés
    ledger.py            grand livre SQLite en partie double
    invoices.py          génération synthétique par graine
    metrics.py           mesures (doublons, écarts, impayés, invariants)
    client.py            client HTTP + erreur d'environnement
    payment_api.py       API simulée (seul écrivain du grand livre)
    preparer.py          préparateur scripté (agent factice)
    check_lot.py         contrôle du lot (politique de validation Swarm)
    settle.py            règlement déterministe
    verify_settlement.py contrôle du règlement / revue finale
    harness.py           racine isolée, API, mesure, enregistrement
    run_b.py             conditions B0 et B1
    provider_s.py        fournisseur scripté Swarm (préparation, règlement)
    run_s.py             condition S
    provider_real.py     préparateur réel (Claude/Codex) pour la campagne réelle
    campaign.py          grille d'exécutions, JSONL reprenable
    tables.py            tableaux de l'article, intervalles de Wilson
  tests/
    __init__.py
    support.py
    test_ledger.py  test_metrics.py  test_payment_api.py  test_check_lot.py
    test_settle.py  test_run_b.py    test_run_s.py        test_tables.py
```

---

### Tâche 0 : préparer l'environnement

**Fichiers :** créer `benchmarks/billing/bench/__init__.py`, `benchmarks/billing/tests/__init__.py` (vides).

**Étape 1 :** vérifier la chaîne d'outils.

Exécuter : `python3 --version && go version`
Attendu : `Python 3.13.x` et `go1.24.x`.

**Étape 2 :** reconstruire le binaire Swarm au commit courant et vérifier sa suite.

```bash
cd .
CGO_ENABLED=0 go build -trimpath -o bin/swarm . && go test ./... 2>&1 | tail -3
```
Attendu : build sans erreur, `ok` pour le paquet. Si la suite Swarm est rouge :
STOP, le banc mesurerait un moteur cassé.

**Étape 3 :** créer les deux `__init__.py` vides et le dossier `tests/`.

---

### Tâche 1 : constantes et marqueurs

**Fichiers :** créer `bench/config.py`, `bench/markers.py`.

**Étape 1 :** écrire `bench/config.py`.

```python
"""Constantes figées du banc de facturation. Toute valeur synthétique."""

CAP_CENTS = 5_000_000            # plafond par paiement : 50 000,00
INVOICE_COUNT = 12
SUPPLIER_COUNT = 4
FUNDING_FACTOR = 3               # trésorerie = 3 x dû : un doublon reste possible, donc mesurable
TAMPER_DELTA_CENTS = 1_000       # écart injecté (F4, F8), sous le plafond
FIELDS = ("supplier", "number", "amount_cents", "iban")

HTTP_TIMEOUT_S = 5
SETTLE_RETRIES = 3
SETTLE_RETRY_DELAY_S = 0.05
API_START_TIMEOUT_S = 5

B_PREPARE_TIMEOUT_S = 3          # garde-fou externe naïf de B0/B1
B_SETTLE_TIMEOUT_S = 30
S_RUN_TIMEOUT_S = 120
S_QUIET_S = 5                    # aucune activité ni changement d'état pendant 5 s : fin d'exécution
S_STALL_S = 35                   # > 30 s : missionConductorStaleAfter (swarm-companion/mission_supervision.go)
S_BUDGET_TOOL_CALLS = 5
S_GATE_TIMEOUT_S = 15

EXIT_NONCONFORME = 1
EXIT_REFUSED = 2
EXIT_ENVIRONMENT = 3
EXIT_DIGEST = 4
EXIT_CRASH = 137
```

**Étape 2 :** écrire `bench/markers.py`.

```python
"""Marqueurs d'injection : la preuve qu'une faute a réellement eu lieu."""
import json
import os
import time
from pathlib import Path


def mark(path, **fields):
    """Écrit le marqueur de façon atomique ; un lecteur ne voit jamais un fichier partiel."""
    path = Path(path)
    tmp = path.with_suffix(path.suffix + ".tmp")
    tmp.write_text(json.dumps({"at": time.time(), **fields}))
    os.replace(tmp, path)


def read_all(directory):
    """Retourne {nom: contenu} pour chaque marqueur `*.json` du dossier."""
    return {p.stem: json.loads(p.read_text()) for p in sorted(Path(directory).glob("*.json"))}
```

Pas de test dédié : couvert par les tests de l'API et du règlement.

---

### Tâche 2 : grand livre

**Fichiers :** créer `bench/ledger.py` ; test `tests/test_ledger.py`.

**Étape 1 : test en échec.**

```python
import sqlite3
import unittest

from bench import ledger


class LedgerTest(unittest.TestCase):
    def setUp(self):
        self.conn = ledger.connect(":memory:")
        ledger.fund(self.conn, 1_000_000)

    def pay(self, key=None, amount=100_000):
        return ledger.pay(self.conn, supplier="S01", number="F1", amount_cents=amount, iban="FR76X",
                          idem_key=key, attempt="a1", lot_sha256="0" * 64)

    def count(self, table):
        return self.conn.execute(f"SELECT COUNT(*) FROM {table}").fetchone()[0]

    def test_payment_writes_balanced_entries(self):
        pid, replay = self.pay()
        self.assertFalse(replay)
        rows = self.conn.execute("SELECT account, amount_cents FROM entries WHERE payment_id=? ORDER BY account",
                                 (pid,)).fetchall()
        self.assertEqual(rows, [("supplier:S01", 100_000), ("treasury", -100_000)])
        self.assertEqual(ledger.balance(self.conn, ledger.TREASURY), 900_000)
        self.assertEqual(ledger.violations(self.conn, 5_000_000), [])

    def test_same_key_has_one_effect(self):
        first = self.pay(key="S01:F1")
        second = self.pay(key="S01:F1")
        self.assertEqual(second, (first[0], True))
        self.assertEqual(self.count("payments"), 1)

    def test_without_key_pays_twice(self):
        self.pay()
        self.pay()
        self.assertEqual(self.count("payments"), 2)

    def test_insufficient_funds_writes_nothing(self):
        with self.assertRaises(ledger.InsufficientFunds):
            self.pay(amount=2_000_000)
        self.assertEqual(self.count("payments"), 0)
        self.assertEqual(self.count("entries"), 2)  # dotation initiale seulement

    def test_violations_detect_unbalanced_entry_and_cap(self):
        pid, _ = self.pay(amount=600_000)
        self.conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(?, 'treasury', -1)", (pid,))
        self.assertEqual(ledger.violations(self.conn, 500_000),
                         [f"écriture déséquilibrée : paiement {pid}", f"plafond dépassé : paiement {pid}"])
```

**Étape 2 :** UT → attendu : échec `ImportError: cannot import name 'ledger'`.

**Étape 3 : implémentation minimale** de `bench/ledger.py`.

```python
"""Grand livre simulé en partie double (SQLite). Données synthétiques uniquement."""
import sqlite3
import time

TREASURY = "treasury"
EQUITY = "equity"
SCHEMA = """
CREATE TABLE IF NOT EXISTS invoices(
  supplier TEXT NOT NULL, number TEXT NOT NULL,
  amount_cents INTEGER NOT NULL CHECK(amount_cents > 0), iban TEXT NOT NULL,
  PRIMARY KEY(supplier, number));
CREATE TABLE IF NOT EXISTS payments(
  id INTEGER PRIMARY KEY, idem_key TEXT UNIQUE,
  supplier TEXT NOT NULL, number TEXT NOT NULL, amount_cents INTEGER NOT NULL, iban TEXT NOT NULL,
  attempt TEXT NOT NULL, lot_sha256 TEXT NOT NULL, created_at REAL NOT NULL);
CREATE TABLE IF NOT EXISTS entries(
  id INTEGER PRIMARY KEY, payment_id INTEGER, account TEXT NOT NULL, amount_cents INTEGER NOT NULL);
"""


class InsufficientFunds(Exception):
    """La trésorerie ne couvre pas le paiement : refus, aucune écriture."""


def connect(path):
    conn = sqlite3.connect(path, timeout=10, isolation_level=None, check_same_thread=False)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.executescript(SCHEMA)
    return conn


def fund(conn, cents):
    conn.execute("BEGIN IMMEDIATE")
    conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, ?, ?)", (TREASURY, cents))
    conn.execute("INSERT INTO entries(payment_id, account, amount_cents) VALUES(NULL, ?, ?)", (EQUITY, -cents))
    conn.execute("COMMIT")


def balance(conn, account):
    return conn.execute("SELECT COALESCE(SUM(amount_cents), 0) FROM entries WHERE account = ?",
                        (account,)).fetchone()[0]


def pay(conn, *, supplier, number, amount_cents, iban, idem_key, attempt, lot_sha256):
    """Retourne (payment_id, replay). Une clé déjà vue renvoie le paiement existant, sans effet."""
    conn.execute("BEGIN IMMEDIATE")
    try:
        if idem_key is not None:
            row = conn.execute("SELECT id FROM payments WHERE idem_key = ?", (idem_key,)).fetchone()
            if row:
                conn.execute("COMMIT")
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
        conn.execute("COMMIT")
        return pid, False
    except Exception:
        conn.execute("ROLLBACK")
        raise


def violations(conn, cap_cents):
    """Invariants comptables. Le doublon de facture est une mesure, pas une violation comptable."""
    found = [f"écriture déséquilibrée : paiement {pid}"
             for pid, total in conn.execute("SELECT payment_id, SUM(amount_cents) FROM entries GROUP BY payment_id")
             if total != 0]
    if balance(conn, TREASURY) < 0:
        found.append("trésorerie négative")
    found += [f"plafond dépassé : paiement {pid}"
              for (pid,) in conn.execute("SELECT id FROM payments WHERE amount_cents > ?", (cap_cents,))]
    return found
```

**Étape 4 :** UT → attendu : 5 tests `ok`.

**Étape 5 :** commit (si demandé) : `bench/__init__.py bench/config.py bench/markers.py bench/ledger.py tests/__init__.py tests/test_ledger.py`,
message `feat(banc): grand livre en partie double`.

---

### Tâche 3 : factures synthétiques et mesures

**Fichiers :** créer `bench/invoices.py`, `bench/metrics.py` ; test `tests/test_metrics.py`.

**Étape 1 : test en échec.**

```python
import unittest

from bench import config, invoices, ledger, metrics

INV = [{"supplier": "S01", "number": "F1", "amount_cents": 100, "iban": "A"},
       {"supplier": "S02", "number": "F2", "amount_cents": 200, "iban": "B"},
       {"supplier": "S03", "number": "F3", "amount_cents": 300, "iban": "C"}]


class MetricsTest(unittest.TestCase):
    def test_counts_doubles_wrong_unpaid_exactly(self):
        payments = [INV[0], dict(INV[0]), {**INV[1], "amount_cents": 201}, {**INV[1], "iban": "Z"}]
        self.assertEqual(metrics.measure_rows(INV, payments),
                         {"invoices": 3, "payments": 4, "doubles": 2, "wrong": 2, "unpaid": 2})

    def test_generation_is_reproducible_and_funded(self):
        a, b = ledger.connect(":memory:"), ledger.connect(":memory:")
        rows_a, rows_b = invoices.generate(a, seed=5), invoices.generate(b, seed=5)
        self.assertEqual(rows_a, rows_b)
        self.assertEqual(len(rows_a), config.INVOICE_COUNT)
        self.assertTrue(all(0 < r[2] < config.CAP_CENTS for r in rows_a))
        self.assertEqual(ledger.balance(a, ledger.TREASURY), config.FUNDING_FACTOR * sum(r[2] for r in rows_a))

    def test_measure_on_fresh_ledger_is_all_unpaid_and_not_correct(self):
        conn = ledger.connect(":memory:")
        invoices.generate(conn, seed=5)
        m = metrics.measure(conn, config.CAP_CENTS)
        self.assertEqual((m["payments"], m["unpaid"], m["violations"]), (0, config.INVOICE_COUNT, []))
        self.assertFalse(metrics.correct(m))
```

**Étape 2 :** UT → échec `ImportError`.

**Étape 3 :** `bench/invoices.py`.

```python
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
    conn.execute("BEGIN IMMEDIATE")
    conn.executemany("INSERT INTO invoices(supplier, number, amount_cents, iban) VALUES(?, ?, ?, ?)", rows)
    conn.execute("COMMIT")
    ledger.fund(conn, config.FUNDING_FACTOR * sum(r[2] for r in rows))
    return rows
```

`bench/metrics.py` :

```python
"""Mesures relevées dans le grand livre, jamais dans le discours des agents."""
from collections import Counter

from bench import config, ledger


def measure_rows(invoices, payments):
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
    m = measure_rows(rows("SELECT supplier, number, amount_cents, iban FROM invoices"),
                     rows("SELECT supplier, number, amount_cents, iban FROM payments"))
    m["violations"] = ledger.violations(conn, cap_cents)
    return m


def correct(m):
    return m["doubles"] == 0 and m["wrong"] == 0 and m["unpaid"] == 0 and not m["violations"]
```

**Étape 4 :** UT → 8 tests `ok`.

**Étape 5 :** commit (si demandé) : `bench/invoices.py bench/metrics.py tests/test_metrics.py`, `feat(banc): factures synthétiques et mesures`.

---

### Tâche 4 : client HTTP et API de paiement simulée

**Fichiers :** créer `bench/client.py`, `bench/payment_api.py`, `tests/support.py` ; test `tests/test_payment_api.py`.

**Étape 1 :** `tests/support.py` (serveur en processus, port libre).

```python
"""Serveur de paiement en processus pour les tests ; une base neuve par instance."""
import tempfile
import threading
from pathlib import Path

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
```

`support.py` dépend de `harness.init_ledger` : écrire tout de suite ce premier
morceau de `bench/harness.py` (complété en tâche 7) :

```python
"""Outils communs aux exécutions : racine isolée, API, mesure finale, enregistrement."""
from bench import invoices, ledger


def init_ledger(db, seed):
    conn = ledger.connect(str(db))
    invoices.generate(conn, seed)
    conn.close()
```

**Étape 2 : test en échec** `tests/test_payment_api.py`.

```python
import json
import unittest
import urllib.error

from bench import client, config
from tests.support import TEST_TOKEN, ServerFixture


def body(line, key=None):
    return {**line, "idem_key": key, "attempt": "a1", "lot_sha256": "0" * 64}


class PaymentApiTest(unittest.TestCase):
    def fixture(self, **options):
        f = ServerFixture(**options)
        self.addCleanup(f.close)
        return f

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

    def test_snapshot_fails_once_then_answers(self):
        f = self.fixture()
        f.server.fail_snapshot_once = f.dir / "snapshot-503.json"
        with self.assertRaises(client.Unavailable):
            client.snapshot(f.url)
        self.assertEqual(len(client.snapshot(f.url)["invoices"]), config.INVOICE_COUNT)

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
```

**Étape 3 :** UT → échec `ImportError`.

**Étape 4 :** `bench/client.py`.

```python
"""Client HTTP minimal (bibliothèque standard) et faute d'environnement."""
import json
import urllib.error
import urllib.request

from bench import config


class Unavailable(Exception):
    """Service de paiement injoignable ou en erreur 5xx : faute d'environnement, pas de contenu."""


def get(base, path):
    with urllib.request.urlopen(base + path, timeout=config.HTTP_TIMEOUT_S) as response:
        return json.loads(response.read())


def post(base, path, body, token=None):
    headers = {"Content-Type": "application/json"}
    if token:
        headers["X-Settlement-Token"] = token
    request = urllib.request.Request(base + path, data=json.dumps(body).encode(), headers=headers, method="POST")
    with urllib.request.urlopen(request, timeout=config.HTTP_TIMEOUT_S) as response:
        return json.loads(response.read())


def snapshot(base):
    try:
        return get(base, "/snapshot")
    except urllib.error.HTTPError as e:   # avant URLError : HTTPError en hérite
        if e.code >= 500:
            raise Unavailable(f"service de paiement indisponible ({e.code})") from e
        raise
    except (ConnectionError, TimeoutError, urllib.error.URLError) as e:
        raise Unavailable(f"service de paiement injoignable ({e})") from e
```

`bench/payment_api.py` :

```python
#!/usr/bin/env python3
"""API de paiement simulée : seul processus qui écrit dans le grand livre.

Fautes injectables : réponse perdue après un paiement (une fois), instantané
indisponible (une fois). Politique B1 : contrôle par appel, sans état de
tentative ni de fraîcheur. Jeton de règlement optionnel (capacité séparée).
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
DUE_SQL = (f"SELECT {COLUMNS} FROM invoices i WHERE NOT EXISTS (SELECT 1 FROM payments p"
           " WHERE p.supplier = i.supplier AND p.number = i.number) ORDER BY supplier, number")


class PaymentServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(self, address, db_path, policy="none", lose_response_once=None, fail_snapshot_once=None,
                 token=None):
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

    def do_GET(self):
        s = self.server
        if self.path == "/due":
            with s.lock:
                return self.send_json(200, s.query(DUE_SQL))
        if self.path == "/snapshot":
            if s.fail_snapshot_once and not s.fail_snapshot_once.exists():
                markers.mark(s.fail_snapshot_once, fault="snapshot-503")
                return self.send_json(503, {"error": "service de paiement indisponible"})
            with s.lock:
                body = {"invoices": s.query(f"SELECT {COLUMNS} FROM invoices ORDER BY supplier, number"),
                        "payments": s.query(f"SELECT {COLUMNS} FROM payments ORDER BY id"),
                        "treasury_cents": ledger.balance(s.conn, ledger.TREASURY),
                        "cap_cents": config.CAP_CENTS}
            return self.send_json(200, body)
        self.send_json(404, {"error": "ressource inconnue"})

    def do_POST(self):
        s = self.server
        if self.path != "/pay":
            return self.send_json(404, {"error": "ressource inconnue"})
        if s.token and not hmac.compare_digest(self.headers.get("X-Settlement-Token", ""), s.token):
            return self.send_json(401, {"error": "jeton de règlement absent ou invalide"})
        body = json.loads(self.rfile.read(int(self.headers.get("Content-Length", "0"))))
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
        if not replay and s.lose_response_once and not s.lose_response_once.exists():
            markers.mark(s.lose_response_once, fault="response-lost", payment_id=payment_id)
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
```

**Étape 5 :** UT → 13 tests `ok`. Si `test_lost_response_has_paid_and_left_marker`
lève une autre exception que `ConnectionError`/`URLError` : l'afficher, adapter le
tuple d'exceptions **dans `settle.pay_line` aussi** (même tuple aux deux endroits).

**Étape 6 :** commit (si demandé) : `bench/client.py bench/payment_api.py bench/harness.py tests/support.py tests/test_payment_api.py`,
`feat(banc): API de paiement simulée et fautes injectables`.

---

### Tâche 5 : préparateur, contrôle de lot, vérification du règlement

**Fichiers :** créer `bench/preparer.py`, `bench/check_lot.py`, `bench/verify_settlement.py` ; test `tests/test_check_lot.py`.

**Étape 1 : test en échec.**

```python
import json
import unittest

from bench import check_lot, client, config, preparer, verify_settlement
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

    def test_verify_settlement_flags_unpaid_then_accepts(self):
        self.assertEqual(verify_settlement.main(["--api", self.f.url]), config.EXIT_NONCONFORME)
        for line in self.lot["lines"]:
            client.post(self.f.url, "/pay", {**line, "idem_key": None, "attempt": "a1", "lot_sha256": "0" * 64})
        self.assertEqual(verify_settlement.main(["--api", self.f.url]), 0)

    def test_partial_and_wrong_modes(self):
        self.assertEqual(len(preparer.build(self.f.url, "partial")["lines"]), config.INVOICE_COUNT // 2)
        wrong = preparer.build(self.f.url, "wrong-amount")["lines"][0]
        self.assertEqual(wrong["amount_cents"], self.lot["lines"][0]["amount_cents"] + config.TAMPER_DELTA_CENTS)
```

**Étape 2 :** UT → échec `ImportError`.

**Étape 3 :** `bench/preparer.py`.

```python
#!/usr/bin/env python3
"""Préparateur scripté (agent factice) : propose un lot, ne paie jamais."""
import argparse
import hashlib
import json
import os
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, markers

MODES = ("nominal", "wrong-amount", "partial")


def build(api, mode):
    if mode not in MODES:
        raise ValueError(mode)
    lines = [{k: inv[k] for k in config.FIELDS} for inv in client.get(api, "/due")]
    if mode == "wrong-amount" and lines:
        lines[0]["amount_cents"] += config.TAMPER_DELTA_CENTS
    elif mode == "partial":
        lines = lines[: len(lines) // 2]
    return {"schema_version": 1, "lines": lines}


def write_lot(path, lot):
    """Écriture atomique ; retourne l'empreinte SHA-256 du fichier écrit."""
    data = json.dumps(lot, indent=2, sort_keys=True).encode()
    path = Path(path)
    tmp = path.with_suffix(".tmp")
    tmp.write_bytes(data)
    os.replace(tmp, path)
    return hashlib.sha256(data).hexdigest()


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--mode", choices=MODES, default="nominal")
    p.add_argument("--hang-after-write", action="store_true", help="F6 : boucle sans fin après un lot partiel")
    p.add_argument("--fail-after-write", action="store_true", help="F8 : tentative qui échoue après écriture")
    p.add_argument("--marker")
    a = p.parse_args(argv)
    print(write_lot(a.out, build(a.api, a.mode)))
    if a.hang_after_write:
        markers.mark(a.marker, fault="budget-loop")
        while True:
            time.sleep(1)
    return 1 if a.fail_after_write else 0


if __name__ == "__main__":
    sys.exit(main())
```

`bench/check_lot.py` :

```python
#!/usr/bin/env python3
"""Contrôle d'un lot contre le grand livre.

Codes de sortie : 0 conforme, 1 non conforme, 3 environnement indisponible.
"""
import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config


def problems(lot, snap):
    if lot.get("schema_version") != 1 or not isinstance(lot.get("lines"), list):
        return ["format de lot invalide"]
    invoices = {(i["supplier"], i["number"]): i for i in snap["invoices"]}
    paid = {(p["supplier"], p["number"]) for p in snap["payments"]}
    found, seen, total = [], set(), 0
    for k, line in enumerate(lot["lines"]):
        key = (line.get("supplier"), line.get("number"))
        amount = line.get("amount_cents")
        if key in seen:
            found.append(f"ligne {k} : facture en double dans le lot")
        seen.add(key)
        invoice = invoices.get(key)
        if invoice is None:
            found.append(f"ligne {k} : facture inconnue {key}")
            continue
        if key in paid:
            found.append(f"ligne {k} : facture déjà payée")
        if not isinstance(amount, int) or isinstance(amount, bool) or amount <= 0:
            found.append(f"ligne {k} : montant invalide {amount!r}")
            continue
        if amount != invoice["amount_cents"]:
            found.append(f"ligne {k} : montant {amount} au lieu de {invoice['amount_cents']}")
        if line.get("iban") != invoice["iban"]:
            found.append(f"ligne {k} : bénéficiaire différent de la facture")
        if amount > snap["cap_cents"]:
            found.append(f"ligne {k} : plafond dépassé")
        total += amount
    missing = set(invoices) - paid - seen
    if missing:
        found.append(f"lot incomplet : {len(missing)} facture(s) due(s) absente(s)")
    if total > snap["treasury_cents"]:
        found.append("trésorerie insuffisante pour le lot")
    return found


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--lot", required=True)
    a = p.parse_args(argv)
    try:
        snap = client.snapshot(a.api)
    except client.Unavailable as e:
        print(f"environnement : {e}", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    try:
        lot = json.loads(Path(a.lot).read_text())
    except (OSError, json.JSONDecodeError) as e:
        print(f"lot illisible : {e}", file=sys.stderr)
        return config.EXIT_NONCONFORME
    found = problems(lot, snap)
    for line in found:
        print(line, file=sys.stderr)
    return config.EXIT_NONCONFORME if found else 0


if __name__ == "__main__":
    sys.exit(main())
```

`bench/verify_settlement.py` :

```python
#!/usr/bin/env python3
"""Contrôle du règlement (S) et revue finale (B0/B1), après l'effet.

Codes : 0 chaque facture payée une seule fois au montant exact, 1 sinon,
3 environnement indisponible.
"""
import argparse
import json
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, metrics


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    a = p.parse_args(argv)
    try:
        snap = client.snapshot(a.api)
    except client.Unavailable as e:
        print(f"environnement : {e}", file=sys.stderr)
        return config.EXIT_ENVIRONMENT
    m = metrics.measure_rows(snap["invoices"], snap["payments"])
    print(json.dumps(m))
    return 0 if m["doubles"] == m["wrong"] == m["unpaid"] == 0 else config.EXIT_NONCONFORME


if __name__ == "__main__":
    sys.exit(main())
```

**Étape 4 :** UT → 21 tests `ok`.

**Étape 5 :** commit (si demandé) : `bench/preparer.py bench/check_lot.py bench/verify_settlement.py tests/test_check_lot.py`,
`feat(banc): préparateur scripté et contrôles de lot`.

---

### Tâche 6 : règlement déterministe

**Fichiers :** créer `bench/settle.py` ; test `tests/test_settle.py`.

**Étape 1 : test en échec.**

```python
import subprocess
import sys
import unittest

from bench import config, preparer, settle
from tests.support import ServerFixture

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

    def test_nominal_business_pays_everything_once(self):
        out = self.run_settle("business", "a1", "p1")
        self.assertEqual(out["paid"], config.INVOICE_COUNT)
        m = self.f.measure()
        self.assertEqual((m["doubles"], m["wrong"], m["unpaid"]), (0, 0, 0))

    def test_lost_response_business_key_one_payment(self):
        self.run_settle("business", "a1", "p1", lose=True)
        self.assertEqual(self.f.measure()["doubles"], 0)

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

    def crash_then_restart(self, key_mode):
        marker = self.f.dir / "crash-before-record.json"
        args = [sys.executable, SCRIPT, "--api", self.f.url, "--lot", str(self.lot), "--key-mode", key_mode,
                "--progress", str(self.f.dir / "p1")]
        first = subprocess.run(args + ["--attempt", "a1", "--crash-after-pay", "1", "--crash-marker", str(marker)])
        self.assertEqual(first.returncode, config.EXIT_CRASH)
        self.assertTrue(marker.exists())
        second = subprocess.run(args + ["--attempt", "a1-relance"], capture_output=True)
        self.assertEqual(second.returncode, 0, second.stderr)
        return self.f.measure()

    def test_crash_restart_attempt_key_doubles_once(self):  # canari F3
        self.assertEqual(self.crash_then_restart("attempt")["doubles"], 1)

    def test_crash_restart_business_key_no_double(self):
        m = self.crash_then_restart("business")
        self.assertEqual((m["doubles"], m["unpaid"]), (0, 0))
```

**Étape 2 :** UT → échec `ImportError`.

**Étape 3 :** `bench/settle.py`.

```python
#!/usr/bin/env python3
"""Règlement déterministe d'un lot : une requête de paiement par ligne.

Clé d'idempotence selon le mode : aucune, par tentative, ou métier
(fournisseur, facture). La progression est enregistrée après chaque réponse ;
un crash entre la réponse et l'enregistrement est injectable (F3).
"""
import argparse
import hashlib
import json
import os
import sys
import time
import urllib.error
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import client, config, markers

KEY_MODES = ("none", "attempt", "business")


class DigestMismatch(Exception):
    """Le lot sur disque n'est pas celui qui a été remis : règlement refusé."""


def key_for(mode, attempt, line):
    if mode == "none":
        return None
    if mode == "attempt":
        return f"{attempt}:{line['supplier']}:{line['number']}"
    if mode == "business":
        return f"{line['supplier']}:{line['number']}"
    raise ValueError(mode)


def pay_line(api, body, token):
    """Réessaie après une coupure ; un refus HTTP est définitif et n'est pas réessayé."""
    for retry in range(config.SETTLE_RETRIES + 1):
        try:
            return client.post(api, "/pay", body, token)
        except urllib.error.HTTPError as e:   # avant URLError : HTTPError en hérite
            return {"refused": e.code, "error": json.loads(e.read() or b"{}").get("error", "")}
        except (ConnectionError, TimeoutError, urllib.error.URLError):
            if retry == config.SETTLE_RETRIES:
                raise
            time.sleep(config.SETTLE_RETRY_DELAY_S)


def settle(api, lot_path, key_mode, attempt, progress_path, *, expected_sha256=None, token=None,
           crash_after_pay=None, crash_marker=None):
    raw = Path(lot_path).read_bytes()
    sha = hashlib.sha256(raw).hexdigest()
    if expected_sha256 is not None and sha != expected_sha256:
        raise DigestMismatch(f"lot modifié depuis la remise : {sha} au lieu de {expected_sha256}")
    lines = json.loads(raw)["lines"]
    progress = Path(progress_path)
    done = {json.loads(row)["line"] for row in progress.read_text().splitlines()} if progress.exists() else set()
    outcome = {"paid": 0, "replayed": 0, "refused": [], "skipped": len(done)}
    answered = 0
    for index, line in enumerate(lines):
        if index in done:
            continue
        response = pay_line(api, {**line, "idem_key": key_for(key_mode, attempt, line), "attempt": attempt,
                                  "lot_sha256": sha}, token)
        answered += 1
        if "refused" in response:
            outcome["refused"].append({"line": index, **response})
        elif response["replay"]:
            outcome["replayed"] += 1
        else:
            outcome["paid"] += 1
        if crash_after_pay == answered and not Path(crash_marker).exists():
            markers.mark(crash_marker, fault="crash-before-record", line=index)
            os._exit(config.EXIT_CRASH)
        with progress.open("a") as f:
            f.write(json.dumps({"line": index, **response}) + "\n")
    return outcome


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--api", required=True)
    p.add_argument("--lot", required=True)
    p.add_argument("--key-mode", choices=KEY_MODES, required=True)
    p.add_argument("--attempt", required=True)
    p.add_argument("--progress", required=True)
    p.add_argument("--expected-sha256")
    p.add_argument("--token-file")
    p.add_argument("--crash-after-pay", type=int)
    p.add_argument("--crash-marker")
    a = p.parse_args(argv)
    token = Path(a.token_file).read_text().strip() if a.token_file else None
    try:
        outcome = settle(a.api, a.lot, a.key_mode, a.attempt, a.progress, expected_sha256=a.expected_sha256,
                         token=token, crash_after_pay=a.crash_after_pay, crash_marker=a.crash_marker)
    except DigestMismatch as e:
        print(str(e), file=sys.stderr)
        return config.EXIT_DIGEST
    print(json.dumps(outcome))
    return config.EXIT_REFUSED if outcome["refused"] else 0


if __name__ == "__main__":
    sys.exit(main())
```

**Étape 4 :** UT → 28 tests `ok`.

**Étape 5 :** commit (si demandé) : `bench/settle.py tests/test_settle.py`, `feat(banc): règlement déterministe et crash injectable`.

---

### Tâche 7 : harnais commun et conditions B0/B1

**Fichiers :** compléter `bench/harness.py` ; créer `bench/run_b.py` ; test `tests/test_run_b.py`.

**Étape 1 : test en échec** (canaris et contrôle négatif ; voir la spécification, « Tests du banc »).

```python
import unittest

from bench import config, run_b

N = config.INVOICE_COUNT


class RunBTest(unittest.TestCase):
    def run_ok(self, condition, key, fault):
        r = run_b.run(condition, key, fault, seed=11)
        self.assertEqual(r["status"], "OK", r)
        return r

    def test_negative_control_everything_correct(self):
        for condition in ("B0", "B1"):
            for key in ("none", "attempt", "business"):
                r = self.run_ok(condition, key, "none")
                self.assertTrue(r["correct"] and r["declared_success"], r)
                self.assertEqual(r["final_review"], "conforme")

    def test_canaries_produce_the_expected_defect(self):
        cases = [("F1", "none", "doubles", 1), ("F2", "attempt", "doubles", N), ("F3", "attempt", "doubles", 1),
                 ("F4", "business", "wrong", 1), ("F6", "business", "unpaid", N - N // 2),
                 ("F7", "none", "doubles", N), ("F8", "business", "wrong", 1)]
        for fault, key, metric, expected in cases:
            with self.subTest(fault=fault):
                r = self.run_ok("B0", key, fault)
                self.assertEqual(r["metrics"][metric], expected, r)

    def test_business_key_alone_absorbs_repetition(self):
        for fault in ("F1", "F2", "F3", "F7"):
            with self.subTest(fault=fault):
                self.assertEqual(self.run_ok("B0", "business", fault)["metrics"]["doubles"], 0)

    def test_effect_happens_before_final_review(self):
        r = self.run_ok("B0", "business", "F4")
        self.assertTrue(r["declared_success"] and r["false_success"])
        self.assertEqual(r["final_review"], "anomalie")

    def test_b1_misses_amount_but_not_beneficiary(self):
        self.assertEqual(self.run_ok("B1", "business", "F4")["metrics"]["wrong"], 1)

    def test_environment_fault_reaches_final_review(self):
        r = self.run_ok("B0", "business", "F5")
        self.assertIn("snapshot-503", r["markers"])
        self.assertTrue(r["correct"])
```

**Étape 2 :** UT → échec `ImportError` (`run_b`).

**Étape 3 :** compléter `bench/harness.py` (remplacer le contenu de la tâche 4).

```python
"""Outils communs aux exécutions : racine isolée, API, mesure finale, enregistrement."""
import hashlib
import json
import platform
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from bench import config, invoices, ledger, markers, metrics, preparer

BENCH_DIR = Path(__file__).resolve().parent
SCRIPTS = {name: str(BENCH_DIR / f"{name}.py") for name in (
    "payment_api", "preparer", "settle", "check_lot", "verify_settlement", "provider_s", "provider_real")}
EXPECTED_MARKER = {"F1": "response-lost", "F2": "dual-launch", "F3": "crash-before-record", "F4": "lot-tampered",
                   "F5": "snapshot-503", "F6": "budget-loop", "F7": "owner-stalled", "F8": "stale-report"}
FAULTS = ("none",) + tuple(EXPECTED_MARKER)
KEY_MODES = ("none", "attempt", "business")


def init_ledger(db, seed):
    conn = ledger.connect(str(db))
    invoices.generate(conn, seed)
    conn.close()


def new_run_dir(prefix):
    run_dir = Path(tempfile.mkdtemp(prefix=prefix))
    (run_dir / "faults").mkdir()
    return run_dir


class Api:
    """API de paiement en sous-processus, arrêtée à la sortie du bloc `with`."""

    def __init__(self, run_dir, policy="none", lose_response_once=None, fail_snapshot_once=None, token_file=None):
        self.port_file = run_dir / "api.port"
        self.log_path = run_dir / "api.log"
        self.args = [sys.executable, SCRIPTS["payment_api"], "--db", str(run_dir / "ledger.db"),
                     "--port-file", str(self.port_file), "--policy", policy]
        for flag, value in (("--lose-response-once", lose_response_once),
                            ("--fail-snapshot-once", fail_snapshot_once), ("--token-file", token_file)):
            if value:
                self.args += [flag, str(value)]

    def __enter__(self):
        self.log = open(self.log_path, "w")
        self.proc = subprocess.Popen(self.args, stdout=self.log, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + config.API_START_TIMEOUT_S
        while not self.port_file.exists():
            if self.proc.poll() is not None or time.monotonic() > deadline:
                raise RuntimeError(f"API de paiement non démarrée, voir {self.log_path}")
            time.sleep(0.05)
        self.url = f"http://127.0.0.1:{self.port_file.read_text()}"
        return self

    def __exit__(self, *exc):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()
        self.log.close()


def tamper(lot_path, marker):
    """F4 : modifie le montant de la première ligne après validation."""
    before = hashlib.sha256(Path(lot_path).read_bytes()).hexdigest()
    lot = json.loads(Path(lot_path).read_text())
    lot["lines"][0]["amount_cents"] += config.TAMPER_DELTA_CENTS
    after = preparer.write_lot(lot_path, lot)
    markers.mark(marker, fault="lot-tampered", before=before, after=after)


def provenance():
    git = ["git", "-C", str(BENCH_DIR)]
    commit = subprocess.run(git + ["rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    dirty = subprocess.run(git + ["status", "--porcelain", "--", str(BENCH_DIR.parent),
                                  str(BENCH_DIR.parents[1] / "swarm-companion")],
                           capture_output=True, text=True).stdout.strip()
    return {"git_commit": commit, "git_dirty": bool(dirty), "python": platform.python_version()}


def finish(run_dir, *, condition, key_mode, fault, seed, started, declared_success, timed_out, extra):
    """Mesure après arrêt de l'API : le grand livre ne bouge plus."""
    conn = ledger.connect(str(run_dir / "ledger.db"))
    m = metrics.measure(conn, config.CAP_CENTS)
    conn.close()
    seen = markers.read_all(run_dir / "faults")
    expected = EXPECTED_MARKER.get(fault)
    status = "DÉLAI" if timed_out else ("INVALIDE" if expected and expected not in seen else "OK")
    return {"condition": condition, "key_mode": key_mode, "fault": fault, "seed": seed, "status": status,
            "markers": seen, "metrics": m, "correct": metrics.correct(m), "declared_success": declared_success,
            "false_success": bool(declared_success) and not metrics.correct(m),
            "duration_s": round(time.monotonic() - started, 3), "run_dir": str(run_dir),
            "provenance": provenance(), **extra}
```

`bench/run_b.py` :

```python
#!/usr/bin/env python3
"""Conditions B0 (sans moteur) et B1 (contrôle par appel).

Chaîne naïve : préparer, régler, puis revue finale après l'effet. Chaque faute
suit le scénario de la spécification ; F2 et F7 ne diffèrent que par l'ordre,
faute de moteur pour les distinguer.
"""
import subprocess
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, markers

REVIEW = {0: "conforme", config.EXIT_NONCONFORME: "anomalie", config.EXIT_ENVIRONMENT: "indisponible"}


def run(condition, key_mode, fault, seed):
    if condition not in ("B0", "B1") or key_mode not in harness.KEY_MODES or fault not in harness.FAULTS:
        raise ValueError((condition, key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir(f"banc-{condition.lower()}-")
    faults = run_dir / "faults"
    harness.init_ledger(run_dir / "ledger.db", seed)
    launches, ok, review, timed_out = 0, [], None, False
    api_options = {"policy": "b1" if condition == "B1" else "none",
                   "lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "fail_snapshot_once": faults / "snapshot-503.json" if fault == "F5" else None}
    with harness.Api(run_dir, **api_options) as api:

        def prepare(name, mode="nominal", hang=False, fail=False):
            nonlocal launches
            launches += 1
            lot = run_dir / f"lot-{name}.json"
            args = [sys.executable, harness.SCRIPTS["preparer"], "--api", api.url, "--out", str(lot), "--mode", mode]
            if hang:
                args += ["--hang-after-write", "--marker", str(faults / "budget-loop.json")]
            if fail:
                args.append("--fail-after-write")
            proc = subprocess.Popen(args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                proc.wait(timeout=config.B_PREPARE_TIMEOUT_S)
            except subprocess.TimeoutExpired:   # garde-fou externe : le lot partiel reste sur disque
                proc.kill()
                proc.wait()
            return lot

        def settle(lot, attempt, progress, crash=False):
            nonlocal launches
            launches += 1
            args = [sys.executable, harness.SCRIPTS["settle"], "--api", api.url, "--lot", str(lot),
                    "--key-mode", key_mode, "--attempt", attempt,
                    "--progress", str(run_dir / f"progress-{progress}.jsonl")]
            if crash:
                args += ["--crash-after-pay", "1", "--crash-marker", str(faults / "crash-before-record.json")]
            return subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S).returncode

        def final_review():
            args = [sys.executable, harness.SCRIPTS["verify_settlement"], "--api", api.url]
            code = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S).returncode
            if code == config.EXIT_ENVIRONMENT:   # relance naïve, une fois
                code = subprocess.run(args, capture_output=True, timeout=config.B_SETTLE_TIMEOUT_S).returncode
            return REVIEW.get(code, "erreur")

        try:
            if fault in ("none", "F1", "F5"):
                ok.append(settle(prepare("a1"), "a1", "a1") == 0)
            elif fault == "F2":
                first, second = prepare("a1"), prepare("a2")
                markers.mark(faults / "dual-launch.json", launchers=2)
                ok += [settle(first, "a1", "a1") == 0, settle(second, "a2", "a2") == 0]
            elif fault == "F3":
                lot = prepare("a1")
                code = settle(lot, "a1", "a1", crash=True)
                if code == config.EXIT_CRASH:   # relance naïve : même progression, nouvelle exécution
                    code = settle(lot, "a1-relance", "a1")
                ok.append(code == 0)
            elif fault == "F4":
                lot = prepare("a1")
                harness.tamper(lot, faults / "lot-tampered.json")
                ok.append(settle(lot, "a1", "a1") == 0)
            elif fault == "F6":
                ok.append(settle(prepare("a1", mode="partial", hang=True), "a1", "a1") == 0)
            elif fault == "F7":
                stalled = prepare("a1")
                markers.mark(faults / "owner-stalled.json", owner="a1")
                ok.append(settle(prepare("a2"), "a2", "a2") == 0)
                ok.append(settle(stalled, "a1", "a1") == 0)
            elif fault == "F8":
                stale = prepare("a1", mode="wrong-amount", fail=True)
                prepare("a2")   # le bon lot est annoncé, puis supplanté par l'annonce tardive de a1
                markers.mark(faults / "stale-report.json", announced="lot-a1.json")
                ok.append(settle(stale, "a1", "a1") == 0)
            review = final_review()
        except subprocess.TimeoutExpired:
            timed_out = True
    return harness.finish(run_dir, condition=condition, key_mode=key_mode, fault=fault, seed=seed, started=started,
                          declared_success=bool(ok) and all(ok) and not timed_out, timed_out=timed_out,
                          extra={"launches": launches, "final_review": review})
```

**Étape 4 :** UT → tous `ok` (28 + 6). Si un canari ne produit pas le défaut
attendu : **ne pas changer la valeur attendue** ; trouver pourquoi l'injection
n'agit pas (systematic-debugging), c'est le banc qui est faux.

**Étape 5 :** commit (si demandé) : `bench/harness.py bench/run_b.py tests/test_run_b.py`,
`feat(banc): conditions B0 et B1, canaris`.

---

### Tâche 8 : fournisseur scripté et condition S, cas nominal

**Fichiers :** créer `bench/provider_s.py`, `bench/run_s.py` ; test `tests/test_run_s.py`.

**Étape 1 : test d'intégration en échec.** Ces tests lancent Swarm ; ils ne
tournent qu'avec `BANC_SWARM=1`, et échouent (pas de saut) si le binaire manque.

```python
import os
import unittest

from bench import config, run_s


@unittest.skipUnless(os.environ.get("BANC_SWARM") == "1", "intégration Swarm : relancer avec BANC_SWARM=1")
class RunSTest(unittest.TestCase):
    def setUp(self):
        self.assertTrue(run_s.SWARM.exists(), f"binaire absent : {run_s.SWARM} (tâche 0)")

    def test_negative_control_settles_everything_once(self):
        r = run_s.run("business", "none", seed=11)
        self.assertEqual(r["status"], "OK", r)
        self.assertTrue(r["correct"] and r["declared_success"], r)
        self.assertEqual(r["task_status"], {"prepare": "accepted", "settle": "accepted"})
```

**Étape 2 :** `BANC_SWARM=1` + UT → échec `ImportError` (`run_s`).

**Étape 3 :** `bench/provider_s.py`.

```python
#!/usr/bin/env python3
"""Fournisseur scripté pour la condition S : préparation ou règlement, jamais les deux.

Arguments : racine, binaire swarm, URL de l'API, mode de clé, faute, fichier de jeton ou '-'.
Le prompt Swarm donne mission, tâche, agent et tentative.
"""
import json
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, markers, preparer, settle

PROMPT = re.compile(r"Coordination structurée : mission ([^,]+), tâche ([^,]+), agent ([^,]+), tentative ([^.]+)[.]")


def emit(event):
    print(json.dumps(event), flush=True)


def tool_call(i, command, output="", code=0):
    """Événement d'outil au format attendu par Swarm : il compte dans le budget d'appels."""
    emit({"type": "item.started", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                           "status": "in_progress"}})
    emit({"type": "item.completed", "item": {"id": f"c{i}", "type": "command_execution", "command": command,
                                             "aggregated_output": output, "exit_code": code,
                                             "status": "completed" if code == 0 else "failed"}})


class Swarm:
    def __init__(self, root, binary):
        self.root, self.binary = root, binary

    def cli(self, args, data=None):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args]
                           + (["--input", "-"] if data is not None else []),
                           input=json.dumps(data) if data is not None else None, text=True, capture_output=True)
        if p.returncode:
            raise RuntimeError(p.stdout + p.stderr)
        return json.loads(p.stdout) if p.stdout.strip() else None


def first_attempt(sw, work, task, attempt):
    tasks = sw.cli(["work", "show", work])["work"]["tasks"]
    ids = [a["id"] for a in next(t for t in tasks if t["id"] == task)["attempts"]]
    return ids.index(attempt) == 0


def send_handoff(sw, work, agent, attempt, sha):
    return sw.cli(["exchange", "send", work], dict(
        schema_version=1, event_id=uuid.uuid4().hex, kind="handoff", agent_id=agent, task_id="prepare",
        attempt_id=attempt, recipient_task_id="settle", recipient_role="worker", result_state="completed",
        artifacts=[dict(path="lot.json", sha256=sha)]))


def run_prepare(sw, root, api, fault, work, agent, attempt):
    faults = root / "faults"
    first = first_attempt(sw, work, "prepare", attempt)
    mode = "partial" if fault == "F6" else ("wrong-amount" if fault == "F8" and first else "nominal")
    tool_call(1, "préparer le lot depuis /due")
    sha = preparer.write_lot(root / "lot.json", preparer.build(api, mode))
    if fault == "F6":
        markers.mark(faults / "budget-loop.json", attempt=attempt)
        for i in range(2, 60):
            tool_call(i, "recompter les factures")
            time.sleep(0.1)
    send_handoff(sw, work, agent, attempt, sha)
    if fault == "F8" and first:
        markers.mark(faults / "stale-report.json", attempt=attempt)
        tool_call(99, "publier", "temporary failure: connection reset", 1)
        raise SystemExit(1)
    emit({"type": "item.completed", "item": {"type": "agent_message", "text": "Lot proposé et remis."}})


def run_settle(sw, root, api, key_mode, fault, token_file, work, agent, attempt):
    faults = root / "faults"
    if fault == "F4":
        deadline = time.monotonic() + config.S_GATE_TIMEOUT_S
        while not (faults / "tamper-done").exists() and time.monotonic() < deadline:
            time.sleep(0.1)
    consumed, refusals = None, []
    for x in sw.cli(["exchange", "list", work]):   # consommateur naïf : la plus ancienne d'abord
        if x["source_task_id"] != "prepare" or x["state"] == "consumed":
            continue
        try:
            sw.cli(["exchange", "consume", work], dict(
                schema_version=1, event_id=uuid.uuid4().hex, exchange_id=x["id"], agent_id=agent,
                task_id="settle", attempt_id=attempt, acknowledgement="Empreinte contrôlée par le moteur"))
        except RuntimeError as e:
            refusals.append(str(e)[:300])
            continue
        consumed = x
        break
    (root / f"settle-{attempt}-refusals.json").write_text(json.dumps(refusals, ensure_ascii=False))
    if consumed is None:
        tool_call(1, "consommer la remise", "aucune remise valide", 1)
        raise SystemExit(1)
    tool_call(1, "régler le lot")
    outcome = settle.settle(api, root / "lot.json", key_mode, attempt, root / "settle-progress.jsonl",
                            expected_sha256=consumed["artifacts"][0]["sha256"],
                            token=Path(token_file).read_text().strip() if token_file != "-" else None,
                            crash_after_pay=1 if fault == "F3" else None,
                            crash_marker=faults / "crash-before-record.json")
    (root / "settlement.json").write_text(json.dumps(outcome))
    emit({"type": "item.completed", "item": {"type": "agent_message", "text": "Règlement exécuté."}})


def main():
    root, binary, api, key_mode, fault, token_file = Path(sys.argv[1]), *sys.argv[2:7]
    work, task, agent, attempt = PROMPT.search(sys.stdin.read()).groups()
    sw = Swarm(root, binary)
    if task == "prepare":
        run_prepare(sw, root, api, fault, work, agent, attempt)
    elif task == "settle":
        run_settle(sw, root, api, key_mode, fault, token_file, work, agent, attempt)
    else:
        raise SystemExit(f"tâche inattendue : {task}")


if __name__ == "__main__":
    main()
```

`bench/run_s.py` :

```python
#!/usr/bin/env python3
"""Condition S : préparation et règlement conduits par Swarm, fautes injectées de l'extérieur."""
import json
import os
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import config, harness, markers

SWARM = Path(__file__).resolve().parents[2] / "swarm-companion" / "bin" / "swarm"
ACTIVE = {"running", "starting", "queued"}


class Swarm:
    def __init__(self, binary, root):
        self.binary, self.root = str(binary), root

    def cli(self, args, data=None):
        p = subprocess.run([self.binary, "--root", str(self.root), "--json", *args]
                           + (["--input", "-"] if data is not None else []),
                           input=json.dumps(data) if data is not None else None,
                           text=True, capture_output=True, timeout=30)
        if p.returncode:
            raise RuntimeError(p.stdout + p.stderr)
        return json.loads(p.stdout) if p.stdout.strip() else None

    def mutate(self, args, work, **fields):
        out = self.cli(args, dict(schema_version=1, event_id=uuid.uuid4().hex,
                                  expected_revision=work.get("revision", 0), **fields))
        return out.get("work", out)

    def work(self, wid):
        out = self.cli(["work", "show", wid])
        return out.get("work", out)

    def agents(self, wid):
        return [x.get("agent", x) for x in self.cli(["agent", "list", wid])["agents"]]

    def conductor(self, log):
        return subprocess.Popen([self.binary, "--root", str(self.root), "web"], stdout=log, stderr=subprocess.STDOUT)


def profile(root, provider="banc"):
    return {"provider": provider, "role": "worker", "workspace": str(root), "capture_output": True,
            "timeout_seconds": 120}


def setup_work(sw, api_url, fault, prepare_provider):
    w = sw.mutate(["work", "create"], {}, title="Banc de facturation",
                  objective="Payer chaque facture due une seule fois, au montant exact",
                  scope="banc synthétique isolé", criteria=["grand livre conforme"], next="lancer")

    def control(task, script, *extra, why):
        return {"mode": "automatic", "controls": [{
            "id": task + "-controle", "command": ["python3", harness.SCRIPTS[script], "--api", api_url, *extra],
            "criteria": [1], "justification": why, "timeout_seconds": 15}]}

    specs = [
        ("prepare", [], "lot.json", "lot conforme au grand livre",
         control("prepare", "check_lot", "--lot", "lot.json",
                 why="Compare chaque ligne au grand livre : facture, montant, bénéficiaire, plafond, complétude."),
         prepare_provider),
        ("settle", ["prepare"], "settlement.json", "chaque facture payée une fois au montant exact",
         control("settle", "verify_settlement",
                 why="Relit les paiements du grand livre : aucun doublon, aucun écart, aucune facture impayée."),
         "banc"),
    ]
    for tid, deps, deliverable, criterion, policy, provider in specs:
        w = sw.mutate(["task", "add", w["id"]], w, id=tid, title=tid, deliverable=deliverable, criteria=[criterion],
                      depends=deps, next="Produire uniquement le livrable demandé")
        update = {"id": tid, "max_attempts": 2, "validation_policy": policy}
        if fault == "F6" and tid == "prepare":
            update["max_tool_calls"] = config.S_BUDGET_TOOL_CALLS
        w = sw.mutate(["task", "update", w["id"]], w, **update)
        sw.cli(["profile", w["id"], tid], profile(sw.root, provider))
        w = sw.work(w["id"])
    return w


def stall_and_take_over(sw, faults, conductors, log):
    """F7 : le premier conducteur est figé au-delà du seuil de péremption, un second prend la main."""
    first = conductors[0]
    os.kill(first.pid, signal.SIGSTOP)
    markers.mark(faults / "owner-stalled.json", pid=first.pid, stalled_for_s=config.S_STALL_S)
    time.sleep(config.S_STALL_S)
    conductors.append(sw.conductor(log))
    time.sleep(2)
    os.kill(first.pid, signal.SIGCONT)


def watch(sw, wid, run_dir, fault, conductors, log):
    faults = run_dir / "faults"
    deadline = time.monotonic() + config.S_RUN_TIMEOUT_S
    tasks, last, quiet_since, injected = {}, None, None, False
    while time.monotonic() < deadline:
        tasks = {t["id"]: t["status"] for t in sw.work(wid)["tasks"]}
        if not injected and tasks.get("prepare") == "accepted":
            if fault == "F4" and (run_dir / "lot.json").exists():
                harness.tamper(run_dir / "lot.json", faults / "lot-tampered.json")
                (faults / "tamper-done").write_text("1")
                injected = True
            elif fault == "F7":
                stall_and_take_over(sw, faults, conductors, log)
                injected = True
        if tasks.get("settle") == "accepted":
            return tasks, False
        busy = any(a["status"] in ACTIVE for a in sw.agents(wid))
        if not busy and tasks == last:
            quiet_since = quiet_since or time.monotonic()
            if time.monotonic() - quiet_since >= config.S_QUIET_S:
                return tasks, False
        else:
            quiet_since = None
        last = tasks
        time.sleep(0.3)
    return tasks, True


def stop(sw, wid, conductors, run_dir):
    errors = []
    try:
        sw.cli(["mission", "stop", wid])
    except RuntimeError as e:
        errors.append(f"mission stop : {e}")
    for c in conductors:
        try:
            os.kill(c.pid, signal.SIGCONT)
        except ProcessLookupError:
            pass
        c.terminate()
        try:
            c.wait(timeout=10)
        except subprocess.TimeoutExpired:
            c.kill()
            c.wait()
    for a in sw.agents(wid):
        if a["status"] in ACTIVE:
            sw.cli(["agent", "stop", a["id"]])
    (run_dir / "cleanup.json").write_text(json.dumps(errors, ensure_ascii=False))


def run(key_mode, fault, seed, binary=SWARM, real_agent=None):
    """`real_agent` : commande d'un agent réel pour la préparation (tâche 11), sinon préparateur scripté."""
    if key_mode not in harness.KEY_MODES or fault not in harness.FAULTS:
        raise ValueError((key_mode, fault))
    started = time.monotonic()
    run_dir = harness.new_run_dir("banc-s-")
    faults = run_dir / "faults"
    harness.init_ledger(run_dir / "ledger.db", seed)
    token_file = run_dir / "settle.token"
    token_file.write_text(uuid.uuid4().hex)
    sw = Swarm(binary, run_dir)
    sw.cli(["init"])
    conductors, tasks, timed_out, agents = [], {}, False, []
    api_options = {"lose_response_once": faults / "response-lost.json" if fault == "F1" else None,
                   "fail_snapshot_once": faults / "snapshot-503.json" if fault == "F5" else None,
                   "token_file": token_file}
    with harness.Api(run_dir, **api_options) as api, open(run_dir / "conductor.log", "w") as log:
        providers = {"banc": {"command": sys.executable, "env_allow": [],
                              "args": [harness.SCRIPTS["provider_s"], str(run_dir), str(binary), api.url,
                                       key_mode, fault, str(token_file)]}}
        if real_agent:
            providers["banc-reel"] = {"command": sys.executable, "env_allow": ["HOME", "PATH"],
                                      "args": [harness.SCRIPTS["provider_real"], str(run_dir), str(binary),
                                               api.url, "--", *real_agent]}
        (run_dir / ".swarm" / "providers.json").write_text(json.dumps({"schema_version": 1, "providers": providers}))
        w = setup_work(sw, api.url, fault, "banc-reel" if real_agent else "banc")
        sw.cli(["autonomy", w["id"], "autonome", "2"])
        sw.cli(["mission", "start", w["id"]], profile(run_dir))
        try:
            conductors.append(sw.conductor(log))
            if fault == "F2":
                conductors.append(sw.conductor(log))
                time.sleep(1)
                if all(c.poll() is None for c in conductors):
                    markers.mark(faults / "dual-launch.json", conductors=[c.pid for c in conductors])
            tasks, timed_out = watch(sw, w["id"], run_dir, fault, conductors, log)
            agents = sw.agents(w["id"])
        finally:
            stop(sw, w["id"], conductors, run_dir)
    return harness.finish(run_dir, condition="S", key_mode=key_mode, fault=fault, seed=seed, started=started,
                          declared_success=tasks.get("settle") == "accepted", timed_out=timed_out,
                          extra={"launches": len(agents), "task_status": tasks,
                                 "agent_status": [a["status"] for a in agents],
                                 "real_agent": real_agent[0] if real_agent else None})
```

Note : en S, l'API exige le jeton de règlement, et seul `provider_s` (tâche
`settle`) le reçoit. Le préparateur, scripté ou réel, n'a donc aucune capacité de
paiement.

**Étape 4 :** `BANC_SWARM=1 python3 -m unittest tests.test_run_s -v` → attendu `ok`.
En cas d'échec, lire dans l'ordre `<run_dir>/conductor.log`, `<run_dir>/api.log`,
puis la sortie de `swarm agent list`. Champs à vérifier en priorité, car ils sont
supposés d'après le harnais A9 et non lus dans le code Go : `x["artifacts"][0]["sha256"]`
dans la liste des échanges, et le statut `accepted` des tâches. Corriger le banc,
**pas** Swarm.

**Étape 5 :** commit (si demandé) : `bench/provider_s.py bench/run_s.py tests/test_run_s.py`,
`feat(banc): condition S, cas nominal`.

---

### Tâche 9 : exploration des huit fautes en S — STOP opérateur

But : observer le comportement réel de Swarm sous chaque faute **avant** d'écrire
des assertions. Les hypothèses de la spécification ne sont pas des résultats.

**Étape 1 :** script jetable dans le scratchpad (hors dépôt) :

```python
import json, sys
sys.path.insert(0, "benchmarks/billing")
from bench import run_s
for fault in ("F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8"):
    for key in ("none", "business"):
        r = run_s.run(key, fault, seed=11)
        print(json.dumps({k: r[k] for k in ("fault", "key_mode", "status", "metrics", "declared_success",
                                             "false_success", "task_status", "launches", "run_dir")},
                         ensure_ascii=False))
```

**Étape 2 :** pour chaque faute, consigner dans `docs/benchmarks/billing/observations.md` :
statut, marqueur présent, mesures, statut des tâches, et l'extrait du journal
qui explique le comportement. Points à trancher explicitement :

| Faute | Question |
|-------|----------|
| F1 | Avec la clé `none`, Swarm ne protège pas la relance interne de `settle.py` (attendu : 1 doublon, règlement non accepté). Confirmer. |
| F3 | Un crash `137` du règlement déclenche-t-il une nouvelle tentative ? Clé `attempt` → doublon ? |
| F4 | `exchange consume` refuse-t-il une remise dont l'artefact a changé ? Sinon, c'est `DigestMismatch` côté règlement qui protège : le noter, les deux ne valent pas la même chose dans l'article. |
| F5 | Le contrôle en code 3 est-il diagnostiqué comme faute d'environnement ? La préparation est-elle relancée (lot régénéré) ou seule la validation ? |
| F6 | Le budget de 5 appels arrête-t-il la tentative ? Le lot partiel est-il réglé ? |
| F7 | Le conducteur réveillé lance-t-il ou règle-t-il quelque chose ? Le marqueur est-il posé avant le lancement du règlement ? |
| F8 | La remise de la tentative 1 est-elle refusée comme obsolète ? |
| F2 | Les deux conducteurs tournent-ils réellement en même temps (marqueur) ? |

**Étape 3 — STOP.** Présenter le tableau à l'opérateur. Toute faute qui exige
une modification de Swarm pour être reproduite, ou dont le résultat contredit la
spécification, est décidée par l'opérateur (adapter l'injection, publier le
résultat tel quel, ou retirer la faute avec justification). Ne pas continuer sans réponse.

---

### Tâche 10 : figer les comportements observés en tests S

**Fichiers :** modifier `tests/test_run_s.py`.

**Étape 1 :** pour chaque comportement confirmé en tâche 9, ajouter un test qui
l'affirme sur le **contenu** (mesures, statuts, marqueurs), jamais seulement sur
`status == "OK"`. Modèle pour F1, à ajuster selon l'observation validée :

```python
    def test_f1_without_key_engine_does_not_protect_external_retry(self):
        r = run_s.run("none", "F1", seed=11)
        self.assertEqual(r["status"], "OK", r)
        self.assertEqual(r["metrics"]["doubles"], 1)
        self.assertFalse(r["declared_success"])   # le contrôle de règlement voit le doublon

    def test_f1_business_key_settles_once(self):
        r = run_s.run("business", "F1", seed=11)
        self.assertTrue(r["correct"] and r["declared_success"], r)
```

**Étape 2 :** `BANC_SWARM=1` + UT → tout `ok`.

**Étape 3 :** commit (si demandé) : `tests/test_run_s.py docs/benchmarks/billing/observations.md`,
`test(banc): comportements S observés sous faute`.

---

### Tâche 11 : préparateur réel (Claude ou Codex)

**Fichiers :** créer `bench/provider_real.py`.

**Étape 1 :** écrire le fournisseur. Le LLM écrit seulement `lot.json` ; la remise
reste déterministe ; aucun jeton de règlement ne lui est transmis.

```python
#!/usr/bin/env python3
"""Préparateur réel : un agent LLM écrit lot.json, la remise reste déterministe.

Arguments : racine, binaire swarm, URL de l'API, '--', commande de l'agent.
La sortie de l'agent est transmise telle quelle à Swarm (appels, consommation).
"""
import hashlib
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench.provider_s import PROMPT, Swarm, send_handoff

INSTRUCTIONS = """

Mission concrète de ce banc (données synthétiques) :
1. Obtenir les factures dues : python3 -c "import urllib.request; print(urllib.request.urlopen('{api}/due').read().decode())"
2. Écrire à la racine de l'espace de travail le fichier lot.json, exactement au format :
   {{"schema_version": 1, "lines": [{{"supplier": "...", "number": "...", "amount_cents": 0, "iban": "..."}}]}}
   Une ligne par facture due, montants entiers en centimes, sans arrondi ni conversion.
3. Ne jamais appeler /pay. Tu n'as aucun droit de paiement.
"""


def main():
    root, binary, api = Path(sys.argv[1]), sys.argv[2], sys.argv[3]
    command = sys.argv[sys.argv.index("--") + 1:]
    prompt = sys.stdin.read()
    work, task, agent, attempt = PROMPT.search(prompt).groups()
    if task != "prepare":
        raise SystemExit(f"le fournisseur réel ne traite que la préparation, pas {task}")
    code = subprocess.run(command, input=prompt + INSTRUCTIONS.format(api=api), text=True, cwd=root).returncode
    lot = root / "lot.json"
    if code != 0 or not lot.exists():
        raise SystemExit(code or 1)
    send_handoff(Swarm(root, binary), work, agent, attempt, hashlib.sha256(lot.read_bytes()).hexdigest())


if __name__ == "__main__":
    main()
```

**Étape 2 : essai unique, STOP opérateur avant tout lancement** (consomme un
abonnement ou des crédits). Commande proposée, à exécuter seulement après accord :

```bash
cd benchmarks/billing
python3 -c "
import json; from bench import run_s
r = run_s.run('business', 'none', seed=11, real_agent=['claude', '-p', '--output-format', 'stream-json', '--verbose'])
print(json.dumps({k: r[k] for k in ('status','metrics','declared_success','task_status','launches')}, ensure_ascii=False))"
```

Attendu : `status` `OK`. Si l'agent n'est pas authentifié avec `env_allow`
`HOME`/`PATH` : STOP, demander à l'opérateur quelles variables autoriser ; ne
jamais écrire de secret dans le dépôt ni dans les JSONL.

**Étape 3 :** commit (si demandé) : `bench/provider_real.py`, `feat(banc): préparateur réel`.

---

### Tâche 12 : campagne et tableaux

**Fichiers :** créer `bench/campaign.py`, `bench/tables.py` ; test `tests/test_tables.py`.

**Étape 1 : test en échec.**

```python
import unittest

from bench import tables


class TablesTest(unittest.TestCase):
    def test_wilson_bounds(self):
        low, high = tables.wilson(0, 100)
        self.assertEqual(low, 0.0)
        self.assertAlmostEqual(high, 0.0370, places=4)
        low, high = tables.wilson(50, 100)
        self.assertAlmostEqual(low, 0.4038, places=4)
        self.assertAlmostEqual(high, 0.5962, places=4)

    def test_summary_excludes_invalid_runs_and_counts_them(self):
        base = {"condition": "B0", "key_mode": "none", "fault": "F1", "false_success": False,
                "metrics": {"doubles": 0, "wrong": 0}, "duration_s": 1.0}
        records = [{**base, "status": "OK", "metrics": {"doubles": 1, "wrong": 0}},
                   {**base, "status": "OK"},
                   {**base, "status": "INVALIDE"}]
        row = tables.summarize(records)[("B0", "none", "F1")]
        self.assertEqual((row["ok"], row["invalid"], row["double_runs"]), (2, 1, 1))
```

**Étape 2 :** UT → échec `ImportError`.

**Étape 3 :** `bench/tables.py`.

```python
#!/usr/bin/env python3
"""Tableaux de l'article à partir des JSONL : taux par case et intervalle de Wilson à 95 %."""
import argparse
import json
import math
import statistics
import sys
from collections import defaultdict
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))


def wilson(k, n, z=1.96):
    if n == 0:
        return 0.0, 1.0
    p = k / n
    d = 1 + z * z / n
    centre = (p + z * z / (2 * n)) / d
    half = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n)) / d
    return max(0.0, centre - half), min(1.0, centre + half)


def summarize(records):
    cells = defaultdict(list)
    for r in records:
        cells[(r["condition"], r["key_mode"], r["fault"])].append(r)
    out = {}
    for cell, runs in sorted(cells.items()):
        ok = [r for r in runs if r["status"] == "OK"]
        out[cell] = {"ok": len(ok), "invalid": sum(r["status"] == "INVALIDE" for r in runs),
                     "timeout": sum(r["status"] == "DÉLAI" for r in runs),
                     "error": sum(r["status"] == "ERREUR" for r in runs),
                     "double_runs": sum(r["metrics"]["doubles"] > 0 for r in ok),
                     "wrong_runs": sum(r["metrics"]["wrong"] > 0 for r in ok),
                     "false_success_runs": sum(bool(r["false_success"]) for r in ok),
                     "median_duration_s": statistics.median(r["duration_s"] for r in ok) if ok else None}
    return out


def markdown(summary):
    def rate(k, n):
        low, high = wilson(k, n)
        return f"{k}/{n} [{low:.2f} ; {high:.2f}]"
    lines = ["| Condition | Clé | Faute | Exécutions valides | Doublons | Paiements faux | Faux succès |"
             " Invalides | Délais | Erreurs |",
             "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"]
    for (condition, key, fault), s in summary.items():
        n = s["ok"]
        lines.append(f"| {condition} | {key} | {fault} | {n} | {rate(s['double_runs'], n)} | "
                     f"{rate(s['wrong_runs'], n)} | {rate(s['false_success_runs'], n)} | "
                     f"{s['invalid']} | {s['timeout']} | {s['error']} |")
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("jsonl")
    a = p.parse_args(argv)
    records = [json.loads(line) for line in Path(a.jsonl).read_text().splitlines() if line.strip()]
    print(markdown(summarize(records)))
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

`bench/campaign.py` :

```python
#!/usr/bin/env python3
"""Grille d'exécutions condition x clé x faute x répétition ; JSONL reprenable.

Une exécution déjà présente dans le fichier (même case, même graine) est sautée.
Une exception devient une ligne ERREUR, jamais un succès silencieux.
"""
import argparse
import itertools
import json
import sys
import threading
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from bench import harness, run_b, run_s


def one(condition, key, fault, seed):
    try:
        if condition == "S":
            return run_s.run(key, fault, seed)
        return run_b.run(condition, key, fault, seed)
    except Exception as e:   # une case en erreur ne doit pas arrêter la campagne ; elle est publiée
        return {"condition": condition, "key_mode": key, "fault": fault, "seed": seed, "status": "ERREUR",
                "error": repr(e)[:500], "provenance": harness.provenance()}


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--out", required=True)
    p.add_argument("--reps", type=int, required=True)
    p.add_argument("--seed-base", type=int, default=1000)
    p.add_argument("--conditions", default="B0,B1,S")
    p.add_argument("--keys", default=",".join(harness.KEY_MODES))
    p.add_argument("--faults", default=",".join(harness.FAULTS))
    p.add_argument("--jobs", type=int, default=1)
    a = p.parse_args(argv)
    out = Path(a.out)
    done = set()
    if out.exists():
        for line in out.read_text().splitlines():
            r = json.loads(line)
            done.add((r["condition"], r["key_mode"], r["fault"], r["seed"]))
    todo = [(c, k, f, a.seed_base + rep)
            for c, k, f, rep in itertools.product(a.conditions.split(","), a.keys.split(","),
                                                  a.faults.split(","), range(a.reps))
            if (c, k, f, a.seed_base + rep) not in done]
    lock = threading.Lock()

    def task(args):
        record = one(*args)
        with lock, out.open("a") as fh:
            fh.write(json.dumps(record, ensure_ascii=False) + "\n")

    with ThreadPoolExecutor(max_workers=a.jobs) as pool:
        list(pool.map(task, todo))
    print(f"{len(todo)} exécution(s) ajoutée(s) à {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
```

**Étape 4 :** UT → tout `ok`.

**Étape 5 : répétition générale** (petite, à exécuter avant la vraie campagne) :

```bash
cd benchmarks/billing
OUT=docs/publications/20261005-cursor-moteur/banc
mkdir -p "$OUT"
python3 bench/campaign.py --out "$OUT/repetition.jsonl" --reps 2 --jobs 2
python3 bench/tables.py "$OUT/repetition.jsonl"
```
Attendu : 3 conditions x 3 clés x 9 fautes x 2 = 162 lignes ; aucune ligne
`ERREUR` ; les cases canari B0 montrent le défaut attendu. Toute case `INVALIDE`
signale une injection à réparer avant la campagne.

**Étape 6 — STOP opérateur :** la campagne complète (`--reps 100`) dure plusieurs
heures (F7 seul impose plus de 35 s par exécution en S). L'opérateur choisit
`--reps`, `--jobs` et le moment. Les fichiers `.jsonl` de campagne sont des
résultats : les committer ou non relève de l'opérateur.

**Étape 7 :** commit (si demandé) : `bench/campaign.py bench/tables.py tests/test_tables.py`,
`feat(banc): campagne reprenable et tableaux`.

---

### Tâche 13 : README et mise à jour du plan d'article

**Fichiers :** créer `benchmarks/billing/README.md`.

**Étape 1 :** README (scénario, invariants, conditions, fautes, commandes de
reproduction, limites). Chaque affirmation doit renvoyer à un fichier ou à une
commande du banc. Limites à écrire telles quelles :
- B1 contrôle bénéficiaire et plafond, pas le montant de facture ni le doublon :
  c'est un choix de conception, à présenter comme tel dans l'article.
- En B0/B1, F2 et F7 ne diffèrent que par l'ordre d'exécution.
- Le règlement vérifie l'empreinte du lot remis (`--expected-sha256`) en S
  seulement : distinguer dans l'article ce qui relève du moteur et ce qui relève du règlement.
- Ce banc ne valide aucune banque réelle ; données synthétiques uniquement.

**Étape 2 :** vérification finale.

```bash
cd benchmarks/billing
python3 -m unittest discover -s tests -t . -v 2>&1 | tail -3
BANC_SWARM=1 python3 -m unittest tests.test_run_s -v 2>&1 | tail -3
grep -rn "flaskProject/utilitaires\|from core\|import utilitaires" bench tests || echo "aucune dépendance applicative"
```
Attendu : `OK` deux fois, puis `aucune dépendance applicative`.

**Étape 3 :** signaler à l'opérateur que la section 6 du plan d'article
(document partagé) peut passer de « conception en cours » à « banc disponible »,
avec le commit du banc. Ne pas l'écrire avant que les tests soient verts.

**Étape 4 :** commit (si demandé) : `benchmarks/billing/README.md`, `docs(banc): README du banc de facturation`.

---

## Condition S hiérarchique (2026-10-05) — remplace les tâches 8 à 10

Décision opérateur du 5 octobre 2026 : la condition S est mesurée sur le dépôt
swarm de référence (`main`, commit `53f2564` et suivants), en mode
hiérarchique. Référence fonctionnelle : `tests/organized_coordination_process.py`
et `tests/organized_fixture.py` (PASS sur `bin/swarm` construit par `make build`).

### Faits établis dans le code (à ne pas re-supposer)

- Un agent d'une mission hiérarchique ne reçoit plus la ligne « Coordination
  structurée » ; son prompt contient « Tâche <id> : » et la consigne d'écrire
  `docs/<id>.md`. Le conducteur remet automatiquement ce rapport au responsable
  de mission avec `{"path", "sha256"}` et l'identité de la tentative, et
  seulement pour une tentative terminée normalement (`review_dialog.go:214-236`).
- `exchange send` est refusé en mission hiérarchique (`agent_exchange.go:261`).
- Une décision du responsable qui s'appuie sur une tentative périmée est
  refusée (`planning.go:413`) ; l'empreinte des artefacts est revérifiée à la
  décision (`planning.go:416`, `agent_exchange.go:156`).
- Les contrôles sont déclarés par exigence (`checks={"req-N": [...]}` dans
  `planning enable`) et les tâches sont créées par une décision du responsable
  (`planning claim` puis `planning decide` avec des opérations `task`).
- Une revue indépendante par le fournisseur du responsable s'ajoute aux
  contrôles automatiques.

### Conception

- Responsable de mission scripté (`bench/planner_fixture.py`), adapté de
  `tests/organized_fixture.py` : revue « pass » si le rapport est non vide,
  clôture quand toutes les tâches sont acceptées. Aucun modèle appelé.
- `prepare` (exigence `req-1`, contrôle `check_lot`) : le préparateur écrit le
  lot JSON dans `docs/prepare.md`, son livrable ; le moteur le remet avec son
  empreinte.
- `settle` (dépend de `prepare`, exigence `req-2`, contrôle
  `verify_settlement`) : le règlement lit dans `work show` la remise de
  `prepare` correspondant à la tentative acceptée, relit le fichier, compare
  l'empreinte (E3), paie avec la clé configurée, écrit `docs/settle.md`.
- Jeton de règlement transmis au seul règlement (E1).
- Fautes : F1, F2, F5, F7 inchangées ; F3 crash du règlement ; F4 modification
  de `docs/prepare.md` après acceptation, avant lecture par le règlement ;
  F6 budget d'appels bas sur `prepare` ; F8 redéfinie : la tentative 1 de
  `prepare` se termine normalement avec un lot faux (rejeté par `check_lot`),
  la tentative 2 produit le bon lot ; deux remises coexistent.
- Succès déclaré en S : `settle` acceptée et périmètre du responsable clos.

### Tâche 8H.0 — essai jetable (STOP si démenti)

Établir et consigner dans `docs/benchmarks/billing/observations.md` :
1. le `path` réel de l'artefact remis quand l'agent a un espace propre
   (`racine/prepare`) ou partagé (`racine`) ;
2. le champ de `work show` qui identifie la tentative acceptée d'une tâche ;
3. le comportement du moteur sur un code 137 du règlement et un code 3 du
   contrôle.

### Tâche 8H.1 à 8H.3 — implémentation, cas nominal seulement

Réécrire `bench/provider_s.py` et `bench/run_s.py` selon la conception, ajouter
`bench/planner_fixture.py`, conserver l'infrastructure existante (`Swarm.cli`,
conducteurs, nettoyage, `BenchError`, marqueurs). Test d'intégration
`tests/test_run_s.py` (avec `BANC_SWARM=1`) : clé métier, sans faute →
`status` OK, `correct` et `declared_success` vrais, tâches acceptées,
responsable clos, deux remises avec artefacts. Les fautes F1 à F8 ne sont pas
exercées dans ce lot.

### Décisions opérateur sur les fautes en S (2026-10-05)

Validées après la relecture de la condition S hiérarchique :

- **F6 (budget)** : la mutation du contrat d'une tâche planifiée est refusée
  (`store.go:444-445`). La limite d'appels est passée par le profil de
  lancement (`limits.max_tool_calls`, à vérifier en essai jetable) et le
  préparateur varie ses commandes pour ne pas déclencher le garde-fou de
  répétition (`loop_guard.go`) avant le budget.
- **F3 (crash du règlement)** : le responsable scripté émet une reprise bornée
  unique (`PlanMaxAttempts` = 2). Sans elle, aucune seconde exécution n'a lieu
  et les modes de clé sont indiscernables.
- **F4 (lot modifié)** : le banc enregistre si `settle` a été lancée ; les
  résultats distinguent « arrêté par le moteur » (fraîcheur des preuves avant
  lancement) et « arrêté par le règlement » (empreinte, code 4).
- **F8 (remise ancienne)** : conservée ; décrite comme une relance par
  correction automatique du moteur suivie du choix de la bonne remise par le
  règlement. Elle ne mesure ni la coordination ni la qualité de la correction.

### Décisions opérateur après l'exploration (2026-10-05)

- **F4 en deux variantes.** `F4` : lot modifié après le départ de `settle`
  (teste le règlement, E3). `F4e` (nouvelle) : lot modifié après acceptation de
  `prepare` mais avant le départ de `settle` (teste la fraîcheur des preuves
  côté moteur, I3). En B0/B1, `F4e` s'injecte comme `F4` (pas de moteur).
- **F5 persistante.** L'instantané renvoie 503 sur plusieurs appels consécutifs
  (nombre fixé en constante), dans toutes les conditions, pour forcer le
  diagnostic d'environnement au lieu d'être absorbé par un rejeu.
- **F7 plus tôt.** Le premier conducteur est figé pendant l'exécution de
  `prepare`, de sorte que le départ de `settle` dépende de la prise de main par
  le second conducteur.
