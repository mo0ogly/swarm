#!/usr/bin/env python3
"""Standalone guards for the managed reviewer fixture workspace contract.

Proven here, in isolation: the generated inert provider program carries its
frozen config as embedded serialized JSON (round-tripped quoting-safe, no
Root-relative config file, no cwd fallback), and when really Popen-launched
with a private cwd DIFFERENT from the fixture root it writes its observation
marker exclusively in THAT cwd with O_CREAT|O_EXCL 0600, prints the two
simulated protocol frames on stdout, keeps stderr empty and exits 0 — the
marker carries exactly pid/ppid/stdin_sha256/context_sha256/catalog_sha256/
native_invocation_supported. A further guard Popen-launches the program with a
SIMULATED I0-like leaf as its cwd (plain fixture directories, no engine label or
permission) and proves `read_reviewer_marker` returns the provider's EXACT
bytes — preserved spacing, provably different from a compact re-serialization —
and refuses an absent marker after the child's real exit (no wait case exists).
The claim/containment guards bind marker reads
to `root/.swarm/biba-i0/reviewer/{request.event_id}` and refuse arbitrary
paths, symlinks, replaced leaves (owner identity) and the historical
root-level marker (incompatible legacy schema, never a silent fallback).

NOT proven here, by design: this standalone run is a clearly simulated
fixture protocol — it proves no Biba confinement, no native permission, no
engine leaf creation and no acceptance; the engine primary
(native_provider_workspace.rs) selects the real I0 reviewer leaf and Codex
runs the real NEW producer after source stability. No live database, no
watchdog/timeout/retry cap; stdout/stderr/exit codes stay conserved; all
fixtures live in the repository fixture area and are cleaned by this owner.
"""
import ast,hashlib,json,os,shutil,stat,subprocess,sys,unittest,uuid
from pathlib import Path
if __package__ in (None,''):sys.path.insert(0,str(Path(__file__).resolve().parent))
import fixture_area
import rust_native_managed_reviewer as native

SIMULATED_PROTOCOL_MARKER='Do not use tools.\n'


class ManagedReviewerFixtureWorkspaceTest(unittest.TestCase):
 """Targeted standalone guards; see the module docstring for proven limits."""

 def setUp(self):
  """Allocate a private durable fixture root, program and simulated stdin."""
  self.root=fixture_area.fixture_root('managed-reviewer-fixture-ws-')
  self.addCleanup(shutil.rmtree,self.root,True)
  self.cwd=self.root/'provider-cwd-i0';self.cwd.mkdir(mode=0o700)
  self.event_id='managed-native-review-'+uuid.uuid4().hex
  self.config=dict(case='pass',source=str(self.root/'tracked"quote\\back.txt'),marker_event=self.event_id)
  self.program=self.root/'codex';self.program.write_bytes(native.provider_program(self.config));self.program.chmod(0o700)
  self.raw=(SIMULATED_PROTOCOL_MARKER+json.dumps(dict(
   managed_protocol=dict(context_sha256='context-sha-fixture-0001',catalog_sha256='catalog-sha-fixture-0001',
    catalog=dict(originals=[],quotes=[]),
    context=dict(candidate_commit='candidate-commit-fixture-0001',tasks=[])),
   native_invocation_supported=True))).encode()

 def _launch(self):
  """Really Popen the inert program with the private cwd; return all channels."""
  process=subprocess.Popen([sys.executable,str(self.program)],stdin=subprocess.PIPE,
   stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=str(self.cwd))
  out,err=process.communicate(self.raw)
  return process.returncode,out,err

 def test_provider_publishes_exclusive_marker_in_cwd_and_frames_on_stdout(self):
  """The provider marker lands ONLY in its cwd, 0600 exclusive, with frames."""
  code,out,err=self._launch()
  self.assertEqual(code,0,(out,err));self.assertEqual(err,b'')
  frames=out.splitlines();self.assertEqual(len(frames),2)
  first=json.loads(frames[0]);self.assertEqual(first['type'],'item.completed')
  self.assertEqual(first['item']['type'],'agent_message');self.assertEqual(first['item']['id'],'review')
  reply=json.loads(first['item']['text'])
  self.assertEqual(reply,dict(catalog_sha256='catalog-sha-fixture-0001',
   candidate_commit='candidate-commit-fixture-0001',tasks=[]))
  self.assertEqual(json.loads(frames[1]),dict(type='turn.completed',usage=dict(input_tokens=7,output_tokens=8)))
  marker=self.cwd/'managed-reviewer-invoked.json'
  self.assertEqual(stat.S_IMODE(os.stat(marker).st_mode),0o600)
  witness=json.loads(marker.read_bytes())
  self.assertEqual(sorted(witness),['catalog_sha256','context_sha256','native_invocation_supported','pid','ppid','stdin_sha256'])
  self.assertGreater(witness['pid'],0);self.assertEqual(witness['ppid'],os.getpid())
  self.assertEqual(witness['stdin_sha256'],hashlib.sha256(self.raw).hexdigest())
  self.assertEqual(witness['context_sha256'],'context-sha-fixture-0001')
  self.assertEqual(witness['catalog_sha256'],'catalog-sha-fixture-0001')
  self.assertTrue(witness['native_invocation_supported'])
  # No config file was read and no root-level marker exists: the historical
  # root marker location stays untouched by the program.
  self.assertFalse((self.root/'managed-review-fixture.json').exists())
  self.assertFalse((self.cwd/'managed-review-fixture.json').exists())
  self.assertFalse((self.root/'managed-reviewer-invoked.json').exists())
  # The claim-based readers refuse to observe this isolated cwd layout (no
  # `.swarm/biba-i0/reviewer` scope under this root): absent scope is absent
  # evidence, not an empty success.
  with self.assertRaises(FileNotFoundError):native.read_reviewer_marker(self.root,self.event_id)

 def test_program_embeds_frozen_config_quoting_safe(self):
  """The embedded config round-trips exactly, without any Root file read."""
  line=next(l for l in self.program.read_text().splitlines() if l.startswith('config=json.loads('))
  self.assertEqual(json.loads(ast.literal_eval(line[len('config=json.loads('):-1])),self.config)

 def test_helper_returns_exact_provider_bytes_and_refuses_absent_marker(self):
  """One read after real exit: exact bytes kept, compaction is NOT equality."""
  # A CLEARLY SIMULATED I0-like leaf: plain fixture directories, no engine
  # label, no enrolment, no permission — layout only, nothing is granted.
  scope=self.root/'.swarm'/'biba-i0'/'reviewer';leaf=scope/self.event_id
  leaf.mkdir(parents=True,mode=0o700)
  process=subprocess.Popen([sys.executable,str(self.program)],stdin=subprocess.PIPE,
   stdout=subprocess.PIPE,stderr=subprocess.PIPE,cwd=str(leaf))
  out,err=process.communicate(self.raw);self.assertEqual(process.returncode,0,(out,err))
  marker=native.read_reviewer_marker(self.root,self.event_id)
  written=leaf/'managed-reviewer-invoked.json'
  self.assertEqual(marker,written.read_bytes())
  # json.dump writes with separators; the frozen bytes differ from a compact
  # re-serialization of their content, and the helper keeps the originals.
  self.assertNotEqual(marker,json.dumps(json.loads(marker),separators=(',',':')).encode())
  # The provider run above really exited: an absent marker after that known
  # outcome must refuse causally — no wait case exists.
  held=leaf/'held-marker';written.rename(held)
  try:
   with self.assertRaises(FileNotFoundError):native.read_reviewer_marker(self.root,self.event_id)
  finally:held.rename(written)

 def test_claim_and_marker_guards_refuse_invalid_paths(self):
  """Leaf resolution and marker reads refuse every non-contract path."""
  with self.assertRaises(ValueError):native.reviewer_leaf(self.root,'managed-native-review-../escape')
  with self.assertRaises(ValueError):native.reviewer_leaf(self.root,'UPPER-not-simple')
  with self.assertRaises(FileNotFoundError):native.reviewer_leaf(self.root,self.event_id)
  scope=self.root/'.swarm'/'biba-i0'/'reviewer';scope.mkdir(parents=True,mode=0o700)
  leaf=scope/self.event_id;leaf.mkdir(mode=0o700)
  with self.assertRaises(FileNotFoundError):native.read_reviewer_marker(self.root,self.event_id)
  (leaf/'managed-reviewer-invoked.json').write_bytes(b'{"pid":1}')
  self.assertEqual(native.read_reviewer_marker(self.root,self.event_id),b'{"pid":1}')
  owner=os.stat(leaf)
  self.assertEqual(native.read_reviewer_marker(self.root,self.event_id,expected_owner=(owner.st_dev,owner.st_ino)),b'{"pid":1}')
  with self.assertRaises(RuntimeError):native.read_reviewer_marker(self.root,self.event_id,expected_owner=(owner.st_dev,owner.st_ino+1))
  marker_path=leaf/'managed-reviewer-invoked.json';held=leaf/'held-marker'
  # held keeps the real marker out of the leaf for the remaining cases: the
  # symlink guard needs the free path, and the historical-root guard below
  # requires an ABSENT leaf marker next to the legacy root marker.
  marker_path.rename(held)
  with self.assertRaises(FileNotFoundError):native.read_reviewer_marker(self.root,self.event_id)
  marker_path.symlink_to(self.root/'outside-target.txt')
  try:
   with self.assertRaises(RuntimeError):native.read_reviewer_marker(self.root,self.event_id)
  finally:marker_path.unlink()
  linked=scope/'managed-native-review-linked';linked.symlink_to(leaf)
  try:
   with self.assertRaises(RuntimeError):native.reviewer_leaf(self.root,'managed-native-review-linked')
  finally:linked.unlink()
  # Historical root-level marker: incompatible legacy schema, recognized as
  # such — never silently consumed, never rewritten to fabricate a replay.
  (self.root/'managed-reviewer-invoked.json').write_bytes(b'{"legacy":true}')
  try:
   with self.assertRaises(RuntimeError) as legacy:native.read_reviewer_marker(self.root,self.event_id)
   self.assertIn('incompatible',str(legacy.exception))
  finally:(self.root/'managed-reviewer-invoked.json').unlink()
  with self.assertRaises(FileNotFoundError):native.read_reviewer_marker(self.root,self.event_id)


if __name__=='__main__':unittest.main(verbosity=2)
