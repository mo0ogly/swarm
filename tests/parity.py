"""Run the existing Python behavioral cases against both implementations."""
import importlib.util
import copy
import random
import json
import os
from pathlib import Path
import subprocess
import sys
import unittest

base = Path(__file__).resolve().parents[2] / 'agent-workflows'
spec = importlib.util.spec_from_file_location('reference_tests', base / 'tests/test_evaluate.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
reference = module.evaluate
binary = os.environ['SWARM_BINARY']
comparisons = 0

def both(document, root, phase='delivery'):
    global comparisons
    comparisons += 1
    process = subprocess.run([binary, '--root', str(root), 'evaluate', '--phase', phase, '--input', '-'],
                             input=json.dumps(document, ensure_ascii=False), text=True, capture_output=True)
    try:
        expected = reference(document, root, phase)
    except (ValueError, TypeError, OSError):
        if process.returncode != 2:
            raise AssertionError(('Go accepted invalid input', process.stdout, process.stderr))
        raise
    actual = json.loads(process.stdout)
    assert actual == expected, (expected, actual, process.stderr)
    assert process.returncode == (0 if expected['allowed'] else 1)
    return expected

module.evaluate = both
result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromModule(module))
if result.wasSuccessful():
    fixture = module.GateTests()
    fixture.setUp()
    rng = random.Random(91)
    try:
        for i in range(120):
            doc = copy.deepcopy(fixture.doc)
            for check, row in zip(doc['checks'], doc['results']):
                check['gate'] = rng.choice(['entry', 'validation', 'delivery'])
                check['mandatory'] = rng.choice([True, False])
                check['severity'] = rng.choice(['none', 'minor', 'major', 'critical'])
                row['status'] = rng.choice(['PASS', 'FAIL', 'PARTIEL', 'NON TESTÉ', 'NON APPLICABLE'])
                row['count'] = rng.choice([0.1, 0.25, 0.3333, 0.5, 1, 2, 10]) if row['status']=='FAIL' else 0
                if row['status']=='NON APPLICABLE': row['reason']='synthetic case'
            both(doc, fixture.root, ['entry', 'validation', 'delivery', 'audit'][i % 4])
    finally:
        fixture.doCleanups()
print(f'{comparisons} differential evaluations, including invalid inputs and all four phases')
sys.exit(not result.wasSuccessful())
