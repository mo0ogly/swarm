import importlib.util,json,sys,unittest
from pathlib import Path
path=Path(__file__).with_name('controls.py');spec=importlib.util.spec_from_file_location('swarm_controls',path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
class MappingClaims(unittest.TestCase):
 def test_unsupported_claims(self):
  for key in ['certification','global_compliance','independent_assessment']:
   with self.subTest(key=key):
    claims={'certification':False,'global_compliance':False,'independent_assessment':False};claims[key]=True
    with self.assertRaises(ValueError):module.claims(claims)
 def test_false_claims_explicit(self):
  self.assertIsNone(module.claims({'certification':False,'global_compliance':False,'independent_assessment':False}))

import copy,hashlib,tempfile
class MappingValidation(unittest.TestCase):
 def setUp(self):
  self.tmp=tempfile.TemporaryDirectory();self.addCleanup(self.tmp.cleanup);self.root=Path(self.tmp.name);repo=path.resolve().parents[2]
  self.matrix=json.loads((repo/'web/trust-controls.json').read_text());self.reference=json.loads((repo/'docs/architecture/actions-confiance-controls-reference.json').read_text())
  primary={'catalog':{'metadata':{'version':self.reference['catalog_version']},'controls':[{'id':r['id'].lower(),'title':r['title']}for r in self.reference['controls']]}}
  self.put('catalog.json',primary);self.reference['source_path']='catalog.json';self.reference['source_sha256']=self.sha('catalog.json');self.matrix['framework']['source_sha256']=self.sha('catalog.json');self.update_reference()
  self.put('source.go','fixture source');self.put('receipt.json',{'input_sha256':{'source.go':self.sha('source.go')}})
  for e in self.matrix['evidence']:e['receipt']={'path':'receipt.json','sha256':self.sha('receipt.json')};e['source_bindings']=[{'path':'source.go','sha256':self.sha('source.go')}]
 def put(self,name,value):
  p=self.root/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(value if isinstance(value,str)else json.dumps(value))
 def sha(self,name):return hashlib.sha256((self.root/name).read_bytes()).hexdigest()
 def update_reference(self):
  self.put('reference.json',self.reference);self.matrix['framework']['reference_path']='reference.json';self.matrix['framework']['reference_sha256']=self.sha('reference.json')
 def validate(self):
  self.put('matrix.json',self.matrix);return module.validate(self.root,'matrix.json')
 def test_valid_is_not_compliance(self):
  v=self.validate();self.assertEqual(v['mapped_controls'],12);self.assertFalse(v['runtime_qualified']);self.assertFalse(v['global_compliance']);self.assertFalse(v['tests_rerun'])
 def test_invalid_matrix_contracts(self):
  original=copy.deepcopy(self.matrix)
  changes=[('unknown-control',lambda m:m['mappings'][0].update(control_id='ZZ-999')),('duplicate-control',lambda m:m['mappings'][1].update(control_id=m['mappings'][0]['control_id'])),('compliant-status',lambda m:m['mappings'][0].update(status='compliant')),('no-gap',lambda m:m['mappings'][0].update(gaps=[])),('empty-owner',lambda m:m['mappings'][0].update(owner='')),('unknown-evidence',lambda m:m['mappings'][0].update(evidence_ids=['unknown'])),('no-evidence',lambda m:m['mappings'][0].update(evidence_ids=[])),('global-context',lambda m:m['context'].update(runtime_assessment=True)),('missing-area',lambda m:m['context'].update(areas=['access'])),('private-classification',lambda m:m['context'].update(classifications=['restricted'])),('schema-bool',lambda m:m.update(schema_version=True)),('claimed-assessment',lambda m:m['mappings'][0]['external_assessment'].update(status='passed')),('blank-English',lambda m:m['mappings'][0]['rationale'].update(en='')),('unknown-field',lambda m:m.update(compliant=True))]
  for label,change in changes:
   with self.subTest(label=label):
    self.matrix=copy.deepcopy(original);change(self.matrix)
    with self.assertRaises(ValueError):self.validate()
 def test_evidence_and_source_integrity(self):
  for name in ['receipt.json','source.go','catalog.json']:
   with self.subTest(name=name):
    original=(self.root/name).read_bytes();self.put(name,'changed')
    with self.assertRaises(ValueError):self.validate()
    (self.root/name).write_bytes(original)
 def test_unbound_source(self):
  self.put('receipt.json',{'input_sha256':{}})
  for e in self.matrix['evidence']:e['receipt']['sha256']=self.sha('receipt.json')
  with self.assertRaises(ValueError):self.validate()
 def test_symlink_and_escape(self):
  (self.root/'link').symlink_to(self.root/'receipt.json');(self.root/'directory').symlink_to(self.root,target_is_directory=True)
  for name in ['../receipt.json','/etc/passwd','link','directory/receipt.json']:
   with self.subTest(name=name):
    with self.assertRaises(ValueError):module.safe_read(self.root,name)
 def test_duplicate_json(self):
  with self.assertRaises(ValueError):module.decode('{"schema_version":1,"schema_version":1}')
 def test_non_regular_file(self):
  import os
  os.mkfifo(self.root/'pipe')
  with self.assertRaises(ValueError):module.safe_read(self.root,'pipe')
 def test_reference_bool_schema(self):
  self.reference['schema_version']=True;self.update_reference()
  with self.assertRaises(ValueError):self.validate()

if __name__=='__main__':unittest.main(verbosity=2)
