#!/usr/bin/env python3
"""Closed complete-core metadata only; CLI write/flush may still fail afterward.
No schema verdict establishes current authority, human/UID/locator provenance.
"""
from pathlib import Path
import json,unittest
from jsonschema import Draft202012Validator
ROOT=Path(__file__).resolve().parents[1]
class InitialReport(unittest.TestCase):
    """Create and explicit read reconciliation are separate observed operations."""
    def setUp(self):
        """Build explicit witnesses from each maintained completed-state contract."""
        self.schema=json.loads((ROOT/'schemas/paea/initial-creation-cli-v1.schema.json').read_text());Draft202012Validator.check_schema(self.schema);self.validator=Draft202012Validator(self.schema)
    def witness(self,action):
        """Synthetic private pins only; no material or service is generated here."""
        good={k:p['const']for k,p in self.schema['properties'].items()if 'const'in p};good.update(operation=action,request_pin='ab'*32,package_pin='cd'*32)
        branch=self.schema['allOf'][0]['then'if action=='initial-create'else'else']['properties'];good.update({k:p['const']for k,p in branch.items()});return good
    def test_two_completed_operations(self):
        """Both valid observed results fit without turning read into new generation."""
        for action in ['initial-create','initial-reconcile']:self.assertTrue(self.validator.is_valid(self.witness(action)))
    def test_missing_extra_private_or_pin_shape_refuses(self):
        """No blank/default success or extra private material accepted."""
        good=self.witness('initial-create')
        for key in good:
            bad=dict(good);del bad[key];self.assertFalse(self.validator.is_valid(bad))
        for key,value in [('secret','PRIVATE'),('package_pin','cd'*32+'\n'),('request_pin','AB'*32),('core_result','failed')]:self.assertFalse(self.validator.is_valid(dict(good,**{key:value})))
    def test_authority_and_phase_promotion_refuses(self):
        """Unproven authority and cross-operation facts cannot be promoted silently."""
        for action in ['initial-create','initial-reconcile']:
            good=self.witness(action)
            for key in ['locator_trust_verified','current_authority_verified','human_authentication_verified','separate_os_uid_verified','confinement_verified','service_installed','service_started']:self.assertFalse(self.validator.is_valid(dict(good,**{key:True})))
            if action=='initial-reconcile':
                for key,value in [('material_generation_attempted',True),('package_publication','durable'),('intent_read_validation','pin_matched')]:self.assertFalse(self.validator.is_valid(dict(good,**{key:value})))
            else:self.assertFalse(self.validator.is_valid(dict(good,anchor_publication='visible')))
if __name__=='__main__':unittest.main()
