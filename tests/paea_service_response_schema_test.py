"""Frame-schema contracts only; no service admission, history or OS proof."""
import json
from pathlib import Path
import unittest
from jsonschema import Draft202012Validator, ValidationError

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = json.loads((ROOT / 'schemas/paea/personal-service-response-v1.schema.json').read_text())
VALIDATOR = Draft202012Validator(SCHEMA)


def frame():
    """Synthetic successful init observation, no identity or protected state."""
    return {'schema_version': 1, 'protocol': 'swarm-personal-service-response-v1',
            'operation': 1, 'request_digest': 'ab' * 32, 'challenge': 'A' * 43,
            'completed': True, 'failure_layers': [], 'intent': 'not_attempted',
            'journal': 'not_attempted', 'checkpoint': 'durable', 'sequence': None,
            'configuration_effect_unknown': False, 'view': None, 'projection': None}


class FrameContracts(unittest.TestCase):
    """Validate the schema itself and its bounded claims, never infer a grant."""

    def test_valid_frame_and_failed_known_view_can_coexist(self):
        """Known data after failure is permitted; nested semantic checks separate."""
        Draft202012Validator.check_schema(SCHEMA)
        VALIDATOR.validate(frame())
        failed = frame()
        failed.update(operation=6, completed=False,
                      failure_layers=['personal_after_use', 'business_after_storage'],
                      configuration_effect_unknown=True,
                      view={'snapshot': {}, 'head': {}})
        VALIDATOR.validate(failed)
        # The empty view intentionally demonstrates this schema's framing scope.
        self.assertIn('full configuration semantics', SCHEMA['description'])

    def test_valid_witness_before_unknown_missing_private_and_invalid_fields(self):
        """No omitted/extra/private field or malformed correlation silently passes."""
        variants = []
        for key in frame():
            bad = frame()
            del bad[key]
            variants.append(bad)
        for key, value in [('password', 'synthetic'), ('operation', 0),
                           ('request_digest', 'G' * 64), ('challenge', 'bad'),
                           ('sequence', 0), ('schema_version', 2)]:
            bad = frame()
            bad[key] = value
            variants.append(bad)
        for bad in variants:
            VALIDATOR.validate(frame())
            with self.assertRaises(ValidationError):
                VALIDATOR.validate(bad)

    def test_failure_verdict_layers_and_exclusive_result_shape(self):
        """Successful transport cannot erase failure; two result kinds conflict."""
        variants = []
        bad = frame()
        bad['completed'] = False
        variants.append(bad)
        bad = frame()
        bad['failure_layers'] = ['business']
        variants.append(bad)
        bad = frame()
        bad.update(completed=False, failure_layers=['business', 'business'])
        variants.append(bad)
        bad = frame()
        bad.update(view={'snapshot': {}, 'head': {}},
                   projection={'base': {}, 'proposed': {'snapshot': {}, 'head': {}},
                               'current_event_replay': False})
        variants.append(bad)
        for bad in variants:
            VALIDATOR.validate(frame())
            with self.assertRaises(ValidationError):
                VALIDATOR.validate(bad)


if __name__ == '__main__':
    unittest.main()
