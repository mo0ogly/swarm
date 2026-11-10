"""Guard reuse of actual native shared DTOs; no provider/process/database opens."""
import copy
import json
from pathlib import Path
import subprocess
import unittest
from unittest.mock import patch

from generate_agent_models import generate, load_layout

ROOT = Path(__file__).resolve().parents[2]


class AgentGeneratorTests(unittest.TestCase):
    """Read the frozen wire contract; successful witness precedes each fault."""
    @classmethod
    def setUpClass(cls):
        """Read historical compatibility input without Go IO/execution."""
        cls.layout = load_layout()

    def test_native_contract_reproduces_checked_bytes_without_legacy_source_io(self):
        """Exact rustfmt-normalized regeneration needs no .go reads or Go tool.

        Compare a healthy witness before denying legacy reads. Real rustfmt is
        used only by this fixture; generation itself opens no process. This is
        not a hermetic whole-process or runtime identity proof.
        """
        witness = generate(self.layout)
        formatted = subprocess.run(['rustfmt', '--emit', 'stdout', '--edition', '2021'],
                                   input=witness, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
        self.assertEqual(formatted.stdout, (ROOT / 'rust/crates/swarm-engine/src/agent_models.rs').read_bytes())
        read_bytes, read_text = Path.read_bytes, Path.read_text
        def bytes_without_go(path, *args, **kwargs):
            """Refuse legacy source reads; preserve real native IO failures."""
            if path.suffix == '.go':
                raise AssertionError('legacy-go-source-read')
            return read_bytes(path, *args, **kwargs)
        def text_without_go(path, *args, **kwargs):
            """Refuse legacy source reads; preserve real native IO failures."""
            if path.suffix == '.go':
                raise AssertionError('legacy-go-source-read')
            return read_text(path, *args, **kwargs)
        with patch.object(Path, 'read_bytes', bytes_without_go), patch.object(Path, 'read_text', text_without_go):
            self.assertEqual(generate(load_layout()), witness)

    def test_shared_omitempty_change_refuses_reuse(self):
        """A changed shared serialization flag must fail before source publication."""
        self.assertTrue(generate(self.layout))
        changed = copy.deepcopy(self.layout)
        field = next(f for f in changed['types']['RunLimits']['fields'] if f['json_tag'] == 'max_tool_calls')
        field['json_tag'] += ',omitempty'
        with self.assertRaisesRegex(ValueError, 'shared-agent-fields-changed'):
            generate(changed)

    def test_native_extension_is_retained_and_wire_collision_refuses(self):
        """Four native optional extensions stay additive; wire collisions refuse."""
        witness = generate(self.layout)
        self.assertIn(b'pub native_tool_guard_policy:Option<Box<crate::AgentGuardSnapshot>>', witness)
        self.assertIn(b'pub native_execution:Option<crate::GoRaw>', witness)
        self.assertIn(b'pub native_execution_result:Option<crate::GoRaw>', witness)
        self.assertIn(b'pub native_provider_facts:Option<crate::GoRaw>', witness)
        for key in ('native_tool_guard_policy','native_execution','native_execution_result','native_provider_facts'):
            changed = copy.deepcopy(self.layout)
            changed['types']['Agent']['fields'].append({'json_tag':key,'go_type':'string'})
            with self.assertRaisesRegex(ValueError, 'native-agent-extension-wire-collision'):
                generate(changed)
        self.assertEqual(generate(self.layout), witness)

    def test_noncanonical_native_aliases_refuse_before_publication(self):
        """Canonical ASCII schema rejects case/long-s aliases before publication.

        Valid real inventory is checked before each independent mutation. A near
        prefix stays allowed; the developer boundary rejects ambiguity, not names
        merely resembling a native key. No runtime or Go source is modified.
        """
        witness = generate(self.layout)
        for key in ('native_tool_guard_policy', 'native_execution',
                    'native_execution_result', 'native_provider_facts'):
            for alias in (key.upper(), key.replace('s', 'ſ')):
                if alias == key:
                    continue
                self.assertEqual(generate(self.layout), witness)
                changed = copy.deepcopy(self.layout)
                changed['types']['Agent']['fields'].append(
                    {'json_tag': alias, 'go_type': 'string'})
                with self.assertRaisesRegex(ValueError, 'unsupported-agent-json-field'):
                    generate(changed)
        self.assertEqual(generate(self.layout), witness)
        allowed = copy.deepcopy(self.layout)
        allowed['types']['Agent']['fields'].append(
            {'json_tag': 'native_provider_facts_extra', 'go_type': 'string'})
        self.assertIn(b'pub native_provider_facts_extra:String', generate(allowed))

    def test_stale_producer_refuses_and_retains_valid_generation(self):
        """Source hash mismatch is refused; the unchanged inventory still works."""
        witness = generate(self.layout)
        changed = copy.deepcopy(self.layout)
        changed['types']['Agent']['source_sha256'] = '0' * 64
        with self.assertRaisesRegex(ValueError, 'stale-or-unsupported-agent-producer'):
            generate(changed)
        self.assertEqual(generate(self.layout), witness)


if __name__ == '__main__':
    unittest.main()
