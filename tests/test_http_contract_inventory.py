"""Regression guards for bounded HTTP evidence collection, with no application IO.

These fixtures test collector refusal and lexical evidence, not Go or Rust HTTP
behavior. Real source collection is a separate recorded command.
"""
import copy
import importlib.util
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('http_contract_inventory', ROOT / 'tools/rust-migration/collect_http_contracts.py')
COLLECTOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(COLLECTOR)


def witness():
    """Create one valid declared route/source witness before refusal variants."""
    raw = b'owned source fixture'
    inventory = dict(schema_version=1, reference_commit='1234567',
                     current_source_sha256={COLLECTOR.SERVICE: COLLECTOR.sha(raw)},
                     http=dict(reference_count=1, same_path_present=['/api/v1/works'], same_path_absent=[],
                               rows=[dict(path='/api/v1/works', go_source='internal/engine/web_server.go', go_line=1, same_path_rust_dispatch=True)]))
    return inventory, {COLLECTOR.SERVICE: raw}


class HTTPContractInventory(unittest.TestCase):
    """Protect no-silent-success boundaries and literal/comment preservation."""

    def test_valid_then_stale_source_refusal_and_recovery(self):
        """Exact bytes pass; changed bytes fail; restoring exact bytes recovers."""
        inventory, sources = witness()
        COLLECTOR.validate_inventory(inventory, sources)
        original = sources[COLLECTOR.SERVICE]
        sources[COLLECTOR.SERVICE] += b' changed'
        with self.assertRaisesRegex(ValueError, 'dispatcher changed'):
            COLLECTOR.validate_inventory(inventory, sources)
        sources[COLLECTOR.SERVICE] = original
        COLLECTOR.validate_inventory(inventory, sources)

    def test_duplicate_malformed_empty_and_inconsistent_rows_refuse(self):
        """A valid witness precedes each independent malformed inventory case."""
        inventory, sources = witness()
        COLLECTOR.validate_inventory(inventory, sources)
        mutations = [lambda p: p['http']['rows'].append(copy.deepcopy(p['http']['rows'][0])),
                     lambda p: p['http']['rows'][0].update(go_line=True),
                     lambda p: p['http']['rows'][0].update(path=''),
                     lambda p: p['http'].update(rows=[]),
                     lambda p: p['http'].update(reference_count=2),
                     lambda p: p['http'].update(same_path_absent=['/api/v1/works']),
                     lambda p: p['http']['rows'][0].update(same_path_rust_dispatch=False)]
        for mutation in mutations:
            with self.subTest(mutation=mutations.index(mutation)):
                bad = copy.deepcopy(inventory)
                mutation(bad)
                with self.assertRaises(ValueError):
                    COLLECTOR.validate_inventory(bad, sources)
        COLLECTOR.validate_inventory(inventory, sources)

    def test_registration_ignores_delimiters_in_literals_and_comments(self):
        """Nested closure delimiters remain exact; following route is excluded."""
        source = '''package fixture
mux.HandleFunc("/api/v1/works", func(w W, r R) {
    // ) } fake close; if r.Method != "DELETE" { }
    x := ") }"; y := `) }`; z := ')'
    /* ) } */
    if r.Method != "GET" { fail(w, x) }
    send(w, s.list(r.URL.Query().Get("work")))
})
mux.HandleFunc("/api/v1/other", delegated)
'''
        statement = COLLECTOR.registration(source, 2)
        self.assertTrue(statement.endswith('})'))
        self.assertNotIn('/api/v1/other', statement)
        self.assertIn('y := `) }`', statement)
        observed = COLLECTOR.go_observations(statement)
        self.assertEqual(observed['method_comparison_candidates'], ['GET'])
        self.assertEqual(observed['get_key_candidates'], ['work'])
        self.assertEqual(observed['delegated_store_calls'], ['list'])
        self.assertIn('delegated', COLLECTOR.registration(source, 9))
        named = 'handler := func(w W, r R) { if r.Method != "POST" { return } }\nmux.HandleFunc("/api/v1/x", handler)'
        resolved = COLLECTOR.delegated_observation(named, COLLECTOR.registration(named, 2))
        self.assertEqual(resolved['observations']['method_comparison_candidates'], ['POST'])
        self.assertIn('handler := func', resolved['source'])
        for line in [0, True, 1, 100]:
            with self.subTest(line=line), self.assertRaises(ValueError):
                COLLECTOR.registration(source, line)

    def test_incomplete_syntax_and_unreviewed_adapter_refuse(self):
        """No partial statement or changed semantic anchor becomes a report."""
        for source in ['mux.HandleFunc("/api/v1/x", func() {', 'mux.HandleFunc("unterminated', '/* unterminated']:
            with self.subTest(source=source), self.assertRaises(ValueError):
                COLLECTOR.registration(source, 1) if source.startswith('mux') else COLLECTOR.masked(source)
        current = {name: (ROOT / name).read_bytes() for name in [COLLECTOR.SERVICE] + COLLECTOR.EXTRA}
        notes = COLLECTOR.reviewed_common(current)
        self.assertEqual(set(notes), {'/api/v1/works', '/api/v1/action', '/api/v1/planning', '/api/v1/quotas'})
        changed = dict(current)
        changed[COLLECTOR.EXTRA[0]] = changed[COLLECTOR.EXTRA[0]].replace(b'Some("enable")', b'Some("extra")')
        with self.assertRaisesRegex(ValueError, 'vocabulary changed'):
            COLLECTOR.reviewed_common(changed)


if __name__ == '__main__':
    unittest.main(verbosity=2)
