#!/usr/bin/env python3
"""Closed operator report data, never authentication or proof of a real effect."""
from pathlib import Path
import copy
import json
import unittest
from jsonschema import Draft202012Validator, ValidationError
ROOT=Path(__file__).resolve().parents[1]
SCHEMA=json.loads((ROOT/'schemas/paea/operator-package-cli-v1.schema.json').read_text())
Draft202012Validator.check_schema(SCHEMA)
VALIDATE=Draft202012Validator(SCHEMA)


def witness(operation='operator-check'):
    """Synthetic known source/target facts, without any business or human authority."""
    value={'schema_version':1,'protocol':'swarm-operator-package-cli-v1','operation':operation,
           'operation_result':'completed','operator_package_pin':'ab'*32,'service_artifact_pin':'cd'*32,
           'controller_uid_matches_operator':True,'source_read':'decoded','source_durability':'directory_acknowledged',
           'publication':'not_attempted','output_validation':'not_attempted','output_durability':'not_acknowledged',
           **{f:False for f in ['human_authentication_verified','current_authority_verified','service_installed',
                               'service_started','separate_os_uid_verified','confinement_verified']}}
    if operation=='service-export':
        value['publication']='durable'
    elif operation=='service-export-reconcile':
        value['output_validation']='pin_matched'
        value['output_durability']='directory_acknowledged'
    return value


class Report(unittest.TestCase):
    """Valid witnesses precede closed-field/phase/claim mutations and recovery."""
    def test_realistic_operations_and_failed_known_publication_remain_distinct(self):
        """Failure may retain visible/durable publication; no fictitious rollback."""
        for operation in ['operator-check','service-export','service-export-reconcile']:
            VALIDATE.validate(witness(operation))
        for phase in ['visible','durable']:
            value=witness('service-export')
            value.update(operation_result='failed',publication=phase)
            VALIDATE.validate(value)
    def test_missing_unknown_secret_or_promoted_authority_refuse(self):
        """No empty success, secret field or promotion of unproved claims."""
        for field in ['service_artifact_pin','business_seed','human_authentication_verified','current_authority_verified','service_installed']:
            good=witness()
            VALIDATE.validate(good)
            bad=copy.deepcopy(good)
            if field=='service_artifact_pin':
                del bad[field]
            elif field=='business_seed':
                bad[field]='SYNTHETIC_FORBIDDEN_FIELD'
            else:
                bad[field]=True
            with self.assertRaises(ValidationError):
                VALIDATE.validate(bad)
            VALIDATE.validate(good)
    def test_wrong_operation_phase_refuses_without_discarding_original(self):
        """A read never claims creation; completed export must be acknowledged."""
        for operation,field,value in [('operator-check','publication','durable'),
                                      ('service-export','publication','visible'),
                                      ('service-export-reconcile','output_durability','not_acknowledged')]:
            good=witness(operation)
            VALIDATE.validate(good)
            bad=copy.deepcopy(good)
            bad[field]=value
            with self.assertRaises(ValidationError):
                VALIDATE.validate(bad)
            VALIDATE.validate(good)


if __name__=='__main__':
    unittest.main(verbosity=2)
