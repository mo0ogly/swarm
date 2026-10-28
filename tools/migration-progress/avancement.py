#!/usr/bin/env python3
"""Reproducible Go → Rust migration progress report (read-only on sources).

Builds, from the CURRENT checkout and binaries, one status per reference item
of the Go application, at three surfaces:
- CLI operations: parsed from the Go help (`./swarm`) and the Rust help
  (`swarm-rust --help`), expanded to sub-commands (`a|b|c`);
- HTTP routes: the Go paths of Codex's latest behavior matrix
  (`matrix-integrated.json`) against the arms of Rust `route()`;
- web pages: `web/*.html` served by Go against the pages embedded by Rust.

Status ladder: `absent` < `partiel` (renamed or partial equivalent recorded
by Codex's matrix, or same name with a contract the matrix records as
partial/different) < `existe` (same name in Rust) <
`verifie` (existe + a cited proof file whose `status` is PASS, see
`preuves.json`) < `accepte` (parity accepted in the matrix). Counts are by
item, NOT weighted by effort: one simple read counts as much as the mission
conductor. Data migration and cutover are copied from `PLAN.md`, not
computed.

With `--record`, appends a dated snapshot to the history file; the report
then shows the observed rate between the oldest and newest snapshots and a
naive remaining-time projection, labelled as an estimate. Fails explicitly
(exit 2) when an input is missing or a help parses to zero items, rather
than reporting an empty success.
"""
import argparse
import datetime as dt
import hashlib
import itertools
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PLAN_DIR = ROOT / 'docs/plans/migration-rust'
LEVELS = ['absent', 'partiel', 'existe', 'verifie', 'accepte']
# Pages that exist only for the Rust console (no Go reference to migrate).
RUST_ONLY_PAGES = {'live.html', 'live-session.html', 'native-runtime.html'}
# Matrix states that mean "a renamed or partial Rust equivalent exists".
PARTIAL_STATES = {'RENAMED_PARTIAL_CONTRACT_PARITY_OPEN', 'PARTIAL_CONTRACT_DIFFERENT', 'PARTIAL_RENAMED_VIEW'}


# Code-size domains, first match wins (each file counted once). Keywords are
# matched in the path relative to the source root. `nouveau` domains have no
# Go reference (new Rust features): reported apart, never as a ratio.
DOMAINS = [
    ('Télémétrie et console live (nouveau)', ('telemetry', 'live', 'otel', 'failure_groups', 'attempt_evaluations', 'task_scenarios'), True),
    ('Relecture (review)', ('review',), False),
    ('Planification', ('planning', 'plan_'), False),
    ('Mission (conducteur, file, reprise)', ('mission', 'dispatch', 'fifo', 'continuation'), False),
    ('Agents (lancement, supervision)', ('agent', 'supervis', 'process', 'terminal'), False),
    ('Budget, coûts, quotas', ('budget', 'cost', 'pricing', 'quota', 'spending'), False),
    ('Fournisseurs', ('provider',), False),
    ('Confiance, identité, enrôlement', ('trust', 'identity', 'enrollment', 'authority', 'oidc'), False),
    ('Préparations', ('prepar',), False),
    ('Automatisation', ('automation',), False),
    ('Mémoire, projet', ('memory', 'project'), False),
    ('Réglages', ('settings', 'config'), False),
    ('Travaux et tâches', ('work', 'task'), False),
    ('Web, HTTP, transport', ('web', 'http', 'server', 'transport', 'access', 'session', 'service'), False),
]


def code_lines(path, rust):
    """Code lines of one source file: no blank or comment-only lines; for
    Rust, the trailing `#[cfg(test)]` module is excluded (inline tests)."""
    count = 0
    for line in path.read_text(encoding='utf-8', errors='replace').splitlines():
        stripped = line.strip()
        if rust and line.startswith('#[cfg(test)]'):
            break
        if stripped and not stripped.startswith('//'):
            count += 1
    return count


def domain_sizes():
    """Non-test code lines per domain, Go (`internal/`, `cmd/`, no `_test.go`)
    and Rust (`rust/crates/*/src`, no test-only files: names containing
    `test` or ending in `_sdk.rs`, and no trailing test module). Heuristic:
    keyword matching on paths, documented in DOMAINS."""
    sizes = {name: {'go': 0, 'rust': 0, 'new': new} for name, _, new in DOMAINS}
    sizes['Autres'] = {'go': 0, 'rust': 0, 'new': False}
    def place(rel):
        low = rel.lower()
        for name, keys, _ in DOMAINS:
            if any(k in low for k in keys):
                return name
        return 'Autres'
    for base in ('internal', 'cmd'):
        for path in (ROOT / base).rglob('*.go'):
            if path.name.endswith('_test.go'):
                continue
            sizes[place(str(path.relative_to(ROOT / base)))]['go'] += code_lines(path, False)
    for path in (ROOT / 'rust/crates').glob('*/src/**/*.rs'):
        if 'test' in path.name or path.name.endswith('_sdk.rs'):
            continue
        # Crate name included: `swarm-enrollment-*`, `swarm-identity-*`, `swarm-web`.
        sizes[place(str(path.relative_to(ROOT / 'rust/crates')))]['rust'] += code_lines(path, True)
    return sizes


class InputError(Exception):
    """An input is missing or unusable; the report must not be produced."""


def run(command):
    """Run a help command and return stdout+stderr; a missing program is an InputError."""
    try:
        done = subprocess.run(command, capture_output=True, text=True, cwd=ROOT, stdin=subprocess.DEVNULL)
    except OSError as error:
        raise InputError(f'cannot run {command[0]}: {error}') from error
    return done.stdout + done.stderr


def expand(tokens):
    """Expand command tokens with `a|b` alternatives into full operation names."""
    return [' '.join(p) for p in itertools.product(*[t.split('|') for t in tokens])]


def go_operations(text):
    """Operations of the Go help: every `swarm a b|c …` run of lowercase tokens."""
    ops = set()
    for match in re.finditer(r'(?:^|\s)swarm((?: [a-z][a-z0-9|-]*)+)(.*)$', text, re.M):
        tokens = match.group(1).split()
        ops.update(expand(tokens))
        # `swarm a b show <x> | decide <x>`: an alternative after a spaced `|`
        # shares the parent tokens of the first operation.
        for alt in re.findall(r'\| ([a-z][a-z0-9-]*) <', match.group(2)):
            ops.update(expand(tokens[:-1] + [alt]))
    return ops


def rust_operations(text):
    """Operations of the Rust help (`  swarm-rust a b|c ID …`)."""
    ops = set()
    for match in re.finditer(r'^\s+swarm-rust((?: [a-z][a-z0-9|-]*)+)', text, re.M):
        ops.update(expand(match.group(1).split()))
    return ops


def rust_routes():
    """Exact paths dispatched by Rust `route()` (service.rs), read from source."""
    source = (ROOT / 'rust/crates/swarm-web/src/service.rs').read_text(encoding='utf-8')
    start = source.find('fn route(')
    end = source.find('web-route-unported', start)
    if start < 0 or end < 0:
        raise InputError('service.rs: route() dispatch not found')
    return set(re.findall(r'"(/[^"]+)"', source[start:end]))


def rust_pages():
    """HTML pages embedded by the Rust web crate (`include_bytes!(…web/x.html)`, single- or multi-line)."""
    pages = set()
    for path in (ROOT / 'rust/crates/swarm-web/src').rglob('*.rs'):
        pages.update(re.findall(r'include_(?:bytes|str)!\(\s*"[^"]*web/([a-z0-9_-]+\.html)"', path.read_text(encoding='utf-8')))
    return pages


def latest_matrix():
    """Codex's newest behavior matrix (outside review copies)."""
    found = [p for p in PLAN_DIR.glob('evidence/*/matrix-integrated.json')]
    if not found:
        raise InputError('no evidence/*/matrix-integrated.json')
    return max(found, key=lambda p: p.stat().st_mtime)


def proof_state(entry):
    """Whether every cited proof file exists and declares status PASS."""
    files = entry if isinstance(entry, list) else entry.get('preuves', [])
    if not files:
        return False, 'aucune preuve citée'
    for rel in files:
        path = ROOT / rel
        if not path.is_file():
            return False, f'preuve absente : {rel}'
        try:
            status = json.loads(path.read_text(encoding='utf-8')).get('status')
        except (OSError, ValueError) as error:
            return False, f'preuve illisible : {rel} ({error})'
        if status != 'PASS':
            return False, f'preuve non PASS ({status}) : {rel}'
    return True, ', '.join(files)


def classify(reference, present, partial, proofs, accepted, prefix):
    """Status of each reference item, with the proof or reason kept."""
    rows = []
    for item in sorted(reference):
        key = f'{prefix}:{item}'
        if item in accepted:
            level, note = 'accepte', 'parité acceptée (matrice)'
        elif item in present:
            ok, note = proof_state(proofs[key]) if key in proofs else (False, '')
            if ok:
                level = 'verifie'
            elif item in partial:
                # Same name, but the matrix records a different/partial contract.
                level, note = 'partiel', f'même nom, contrat partiel ou différent (matrice) ; équivalent : {partial[item]}' + (f' ; {note}' if note else '')
            else:
                level = 'existe'
        elif item in partial:
            level, note = 'partiel', f'équivalent : {partial[item]}'
        else:
            level, note = 'absent', ''
        rows.append({'item': item, 'level': level, 'note': note})
    return rows


def counts(rows):
    """Cumulative counts: an item at level L also counts for every lower level.

    Keys are the levels above `absent` (`partiel` = at least partial, so
    `partiel - existe` is the partial-only count) plus `total`; absent items
    are `total - partiel`. History records from before 2026-10-23 also carry an
    always-zero `absent` key, ignored by every reader.
    """
    out = {lvl: 0 for lvl in LEVELS[1:]}
    for r in rows:
        for lvl in LEVELS[1:LEVELS.index(r['level']) + 1]:
            out[lvl] += 1
    out['total'] = len(rows)
    return out


def plan_rows():
    """R0–R7 rows and the data-migration limit, copied verbatim from PLAN.md."""
    plan = (PLAN_DIR / 'PLAN.md').read_text(encoding='utf-8')
    # Rows of the three-column R0–R7 table only (other tables mention R6/R7 too).
    lots = [line for line in plan.splitlines() if re.match(r'^\| R[0-7](?:/R[0-7])? \|[^|]+\|[^|]+\|$', line)]
    # The whole bullet that states the data-migration limit (it spans lines).
    data = [' '.join(m.split()) for m in re.findall(r'^- ((?:.*\n  .*)*.*)', plan, re.M) if 'migrations1..36' in m]
    return lots, data


def publish(path, text):
    """Atomic UTF-8 write in the destination directory (ENOSPC leaves the original)."""
    fd, tmp = tempfile.mkstemp(dir=path.parent, prefix='.' + path.name)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8') as f:
            f.write(text)
            f.flush()
            os.fsync(f.fileno())
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


def pct(n, total):
    return f'{round(100 * n / total)} %' if total else '—'


def main():
    parser = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    parser.add_argument('--rust-binary', type=Path, default=ROOT / '.cache/rust-build-tel/debug/swarm-rust')
    parser.add_argument('--go-command', default=str(ROOT / 'swarm'))
    parser.add_argument('--proofs', type=Path, default=Path(__file__).with_name('preuves.json'))
    parser.add_argument('--output', type=Path, default=PLAN_DIR / 'AVANCEMENT.md')
    parser.add_argument('--history', type=Path, default=PLAN_DIR / 'avancement-historique.jsonl')
    parser.add_argument('--record', action='store_true', help='append this snapshot to the history')
    a = parser.parse_args()
    try:
        if not a.rust_binary.is_file():
            raise InputError(f'Rust binary missing: {a.rust_binary} (build swarm-cli first)')
        newest_source = max(p.stat().st_mtime for p in (ROOT / 'rust/crates').rglob('*.rs'))
        stale = a.rust_binary.stat().st_mtime < newest_source
        if stale and a.record:
            raise InputError('Rust binary older than the sources: rebuild swarm-cli before --record (a stale snapshot would mislead the rate)')
        go_ops = go_operations(run([a.go_command]))
        rust_ops = rust_operations(run([str(a.rust_binary), '--help']))
        if not go_ops or not rust_ops:
            raise InputError(f'help parsed to zero items (go {len(go_ops)}, rust {len(rust_ops)})')
        matrix_path = latest_matrix()
        matrix = json.loads(matrix_path.read_text(encoding='utf-8'))
        proofs = json.loads(a.proofs.read_text(encoding='utf-8')) if a.proofs.is_file() else {}
        proofs = {k: v for k, v in proofs.items() if not k.startswith('_')}
    except (InputError, OSError, ValueError) as error:
        print(f'avancement: {error}', file=sys.stderr)
        return 2
    # Matrix rows name the Go operation either `reference` or `operation`.
    name = lambda r: r.get('reference') or ' '.join(x for x in (r.get('family'), r.get('operation')) if x)
    partial_ops = {name(r): r['rust_equivalent'] for r in matrix['operation_rows'] if r['state'] in PARTIAL_STATES and r.get('rust_equivalent')}
    accepted_ops = {name(r) for r in matrix['operation_rows'] if r.get('parity_accepted')}
    go_paths = {r['path'] for r in matrix['http_contract_rows']}
    accepted_paths = {r['path'] for r in matrix['http_contract_rows'] if r.get('parity_accepted')}
    routes = rust_routes()
    go_pages = {p.name for p in (ROOT / 'web').glob('*.html')} - RUST_ONLY_PAGES
    surfaces = {
        'CLI (opérations)': classify(go_ops, rust_ops, partial_ops, proofs, accepted_ops, 'cli'),
        'Web (routes API)': classify(go_paths, routes, {}, proofs, accepted_paths, 'http'),
        'Web (pages)': classify(go_pages, rust_pages(), {}, proofs, set(), 'page'),
    }
    totals = {name: counts(rows) for name, rows in surfaces.items()}
    sizes = domain_sizes()
    now = dt.datetime.now().astimezone()
    snapshot = {'at': now.isoformat(timespec='seconds'), 'totals': totals, 'sizes': sizes,
                'rust_binary_sha256': hashlib.sha256(a.rust_binary.read_bytes()).hexdigest(), 'matrix': str(matrix_path.relative_to(ROOT))}
    history = []
    if a.history.is_file():
        history = [json.loads(line) for line in a.history.read_text(encoding='utf-8').splitlines() if line.strip()]
    if a.record:
        with a.history.open('a', encoding='utf-8') as f:
            f.write(json.dumps(snapshot, ensure_ascii=False) + '\n')
        history.append(snapshot)
    lots, data = plan_rows()

    out = [f'# Avancement de la migration Go → Rust — {now:%Y-%m-%d %H:%M}', '',
           'Généré par `tools/migration-progress/avancement.py` (relancer après chaque lot ; '
           '`--record` ajoute un point d\'historique). Comptes **par élément, non pondérés par l\'effort**. '
           'Niveaux cumulés : un élément « vérifié » compte aussi comme « existe ».', '']
    if stale:
        out += ['> **Attention** : le binaire Rust est plus ancien que les sources ; reconstruire `swarm-cli` avant de lire ces chiffres.', '']
    out += ['| Surface | Total Go | Existe en Rust | dont vérifié (preuve PASS citée) | Parité acceptée | Partiel (équivalent renommé) |', '|---|---|---|---|---|---|']
    for name, c in totals.items():
        out.append(f"| {name} | {c['total']} | {c['existe']} ({pct(c['existe'], c['total'])}) | {c['verifie']} ({pct(c['verifie'], c['total'])}) | {c['accepte']} ({pct(c['accepte'], c['total'])}) | {c['partiel'] - c['existe']} |")
    out += ['', '**Données et bascule** (copié de `PLAN.md`, non calculé) :', '']
    out += [f'- {line}' for line in data] or ['- (ligne non trouvée dans PLAN.md)']
    out += ['', '| Lot | État | Condition restante |', '|---|---|---|'] + lots
    if len(history) >= 2:
        first, last = history[0], history[-1]
        days = (dt.datetime.fromisoformat(last['at']) - dt.datetime.fromisoformat(first['at'])).total_seconds() / 86400
        out += ['', f"## Vitesse observée ({first['at'][:16]} → {last['at'][:16]})", '']
        if days >= 0.5:
            out += ['| Surface | Gagnés « existe » | Par jour | Restant | Projection naïve (estimation) |', '|---|---|---|---|---|']
            for name in totals:
                gained = last['totals'][name]['existe'] - first['totals'][name]['existe']
                rate = gained / days
                left = last['totals'][name]['total'] - last['totals'][name]['existe']
                proj = f'≈ {left / rate:.0f} jours' if rate > 0 else 'non calculable (aucun gain)'
                out.append(f'| {name} | {gained} | {rate:.1f} | {left} | {proj} |')
            out += ['', 'Projection linéaire sur la seule présence : elle ignore la parité, la migration des données, R6/R7 et la difficulté inégale des éléments.']
        else:
            out += ['Historique trop court (moins de 12 h) pour une vitesse.']
    else:
        out += ['', 'Vitesse : pas encore d\'historique (lancer avec `--record` à chaque lot).']
    # Code-size ratio per domain: a high ratio flags effort that did not turn
    # into migrated surface (hardening, defensive code, proof scaffolding).
    out += ['', '## Taille du code par domaine (Rust / Go, hors tests et commentaires)', '',
            '| Domaine | Go (lignes) | Rust (lignes) | Rapport Rust/Go | Évolution depuis le premier relevé |', '|---|---|---|---|---|']
    first_sizes = next((h.get('sizes') for h in history if h.get('sizes')), None)
    def ratio(d):
        return d['rust'] / d['go'] if d['go'] else None
    migrated = {'go': 0, 'rust': 0}
    for name, d in sizes.items():
        if d['new']:
            out.append(f"| {name} | — | {d['rust']} | sans équivalent Go | — |")
            continue
        migrated['go'] += d['go']
        migrated['rust'] += d['rust']
        r = ratio(d)
        before = ratio(first_sizes[name]) if first_sizes and name in first_sizes else None
        trend = f'{before:.1f} → {r:.1f}' if (before is not None and r is not None and first_sizes is not sizes) else '—'
        out.append(f"| {name} | {d['go']} | {d['rust']} | {'×%.1f' % r if r is not None else '—'} | {trend} |")
    r = ratio(migrated)
    out += [f"| **Total (hors nouveau)** | {migrated['go']} | {migrated['rust']} | ×{r:.1f} | |", '',
            'Le rapport compare des volumes, pas des fonctions : un domaine encore peu porté a un rapport bas, '
            'un domaine porté avec beaucoup de code défensif ou de durcissement un rapport haut. '
            'Rattachement des fichiers par mots-clés de chemin (heuristique, voir `DOMAINS`).']
    for name, rows in surfaces.items():
        out += ['', f'## {name}', '', '| Élément | Niveau | Preuve ou équivalent |', '|---|---|---|']
        order = {lvl: i for i, lvl in enumerate(LEVELS)}
        for r in sorted(rows, key=lambda r: (-order[r['level']], r['item'])):
            out.append(f"| `{r['item']}` | {r['level']} | {r['note']} |")
    extra_routes = sorted(routes - go_paths)
    out += ['', '## Routes Rust sans homonyme Go (non comptées)', '', ', '.join(f'`{r}`' for r in extra_routes) or '—',
            '', 'Certaines remplacent une route Go sous un autre nom (`native-*`) ; tant que la matrice ne le consigne pas, elles ne sont pas comptées.']
    extra = sorted(rust_ops - go_ops)
    out += ['', '## Opérations Rust sans homonyme Go (non comptées)', '', ', '.join(f'`{o}`' for o in extra) or '—', '',
            f"Sources : aide de `{Path(a.go_command).name}`, aide de `{a.rust_binary.relative_to(ROOT) if a.rust_binary.is_relative_to(ROOT) else a.rust_binary}` "
            f"(sha256 {snapshot['rust_binary_sha256'][:12]}…), matrice `{snapshot['matrix']}`, `route()` de `service.rs`, pages `web/*.html`.", '']
    publish(a.output, '\n'.join(out))
    print(json.dumps({'output': str(a.output), 'cumulative_totals': totals, 'stale_binary': stale, 'recorded': a.record}, ensure_ascii=False))
    return 0


if __name__ == '__main__':
    sys.exit(main())
