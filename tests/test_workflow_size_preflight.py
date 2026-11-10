"""Exercise initial workflow fit and typed descriptor refusal without live state."""
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location('workflow_size_preflight', ROOT / 'tools/agent-workflows/check_workflow_size.py')
GUARD = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GUARD)


class WorkflowSizeGuard(unittest.TestCase):
    """Use a real canonical positive control before changing one descriptor field."""
    def test_schema_version_requires_integer_after_valid_control(self):
        """Bool/float/string/null version1 substitutes must not become a fit success."""
        self.assertTrue(GUARD.inspect(ROOT)['fits'])
        descriptor_path = ROOT / 'rust/crates/swarm-engine/config/agent-workflow-descriptor.json'
        original = json.loads(descriptor_path.read_text())
        read_text = Path.read_text
        for value in (True, 1.0, '1', None):
            descriptor = {**original, 'schema_version': value}
            def altered(path, *args, **kwargs):
                """Alter only the in-memory descriptor; all other inputs stay canonical."""
                return json.dumps(descriptor) if path == descriptor_path else read_text(path, *args, **kwargs)
            with self.subTest(version=value), patch.object(Path, 'read_text', altered):
                with self.assertRaisesRegex(ValueError, 'workflow-descriptor-invalid'):
                    GUARD.inspect(ROOT)
        self.assertTrue(GUARD.inspect(ROOT)['fits'])


if __name__ == '__main__':
    unittest.main()
