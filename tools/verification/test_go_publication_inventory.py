"""Permanent isolated guards for Go publication inventories; no commit or push."""
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import unittest
from unittest import mock
import zipfile
import go_publication_inventory as audit


class PublicationInventoryTests(unittest.TestCase):
    """Own temporary Git indexes only; failures retain source/evidence fixtures."""
    def setUp(self):
        """Create an isolated index without inventing an author or committing."""
        audit.require_explicit_git_context()
        self.root=Path(tempfile.mkdtemp(prefix='swarm-go-publication122-'))
        subprocess.run(['git','-c','core.hooksPath=/dev/null','-c','core.fsmonitor=false','init','--quiet',str(self.root)],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)

    def tearDown(self):
        """Remove only this successful fixture, retaining failed original state."""
        outcome=self._outcome.result
        failed=any(getattr(case,'test_case',case) is self for case,_ in outcome.errors+outcome.failures)
        if failed:print('Retained publication fixture: '+str(self.root),flush=True)
        else:shutil.rmtree(self.root)

    def put(self,name,raw):
        """Atomically publish owned fixture bytes; ENOSPC leaves original intact.

        A failure after rename is not claimed to roll back. No live Root is used.
        """
        path=self.root/name;path.parent.mkdir(parents=True,exist_ok=True)
        fd,temp=tempfile.mkstemp(dir=path.parent)
        try:
            with os.fdopen(fd,'wb') as stream:stream.write(raw);stream.flush();os.fsync(stream.fileno())
            assert Path(temp).read_bytes()==raw;os.replace(temp,path)
            directory=os.open(path.parent,os.O_RDONLY|os.O_DIRECTORY)
            try:os.fsync(directory)
            finally:os.close(directory)
        finally:Path(temp).unlink(missing_ok=True)
        return path

    def stage(self,*names):
        """Force-add malicious fixture candidates to test the guard, never real sources."""
        subprocess.run(['git','-c','core.hooksPath=/dev/null','-c','core.fsmonitor=false','-C',str(self.root),'add','-f','--',*names],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE)

    def tar(self,name,members):
        """Create in-memory archive fixtures without extracting any entry."""
        raw=io.BytesIO()
        with tarfile.open(fileobj=raw,mode='w:gz') as archive:
            for path,value in members.items():
                info=tarfile.TarInfo(path);info.size=len(value);archive.addfile(info,io.BytesIO(value))
        return self.put(name,raw.getvalue())

    def test_direct_source_and_module_candidate_refuse_without_index_mutation(self):
        """Detect direct Go/module entries while proving the real fixture index is unchanged."""
        self.put('safe.rs',b'fn main() {}');self.stage('safe.rs');self.assertEqual(audit.inventory(self.root)['status'],'PASS')
        self.put('root.go',b'package main');self.put('go.mod',b'module fixture');self.stage('root.go','go.mod');before=(self.root/'.git/index').read_bytes();result=audit.inventory(self.root)
        self.assertEqual(result['status'],'LEGACY_SOURCE_PRESENT');self.assertEqual(set(result['flagged_paths']),{'root.go','go.mod'});self.assertEqual((self.root/'.git/index').read_bytes(),before)

    def test_tar_zip_and_patches_keep_go_free_artifacts_distinct(self):
        """Classify archive members and source headers without treating narrative text as code."""
        self.tar('docs-only.tar.gz',{'docs/README.md':b'Go migration narrative'});self.stage('docs-only.tar.gz');self.assertEqual(audit.inventory(self.root)['status'],'PASS')
        self.tar('source.tar.gz',{'internal/kernel.go':b'package engine','go.sum':b'proof'})
        raw=io.BytesIO()
        with zipfile.ZipFile(raw,'w') as archive:archive.writestr('source.go','package main')
        self.put('source.zip',raw.getvalue());self.put('source.patch',b'diff --git a/root.go b/root.go\n--- a/root.go\n+++ b/root.go\n+package main\n');self.put('safe.diff',b'diff --git a/README.md b/README.md\n+Go migration\n');self.stage('source.tar.gz','source.zip','source.patch','safe.diff')
        result=audit.inventory(self.root);self.assertEqual(set(result['flagged_paths']),{'source.tar.gz','source.zip','source.patch'});self.assertFalse(result['unreviewed_nested'])

    def test_staged_and_unstaged_archive_bytes_are_not_conflated(self):
        """Retain distinct indexed/physical hashes when only one version contains Go."""
        self.tar('changed.tar.gz',{'safe.rs':b'fn main() {}'});self.stage('changed.tar.gz');self.assertEqual(audit.inventory(self.root)['status'],'PASS');self.tar('changed.tar.gz',{'root.go':b'package main'})
        result=audit.inventory(self.root);row=result['paths'][0];self.assertNotEqual(row['index_sha256'],row['worktree_sha256']);self.assertEqual(row['index']['go'],[]);self.assertEqual(row['worktree']['go'],['root.go']);self.assertEqual(result['status'],'LEGACY_SOURCE_PRESENT')

    def test_nested_archives_stay_unreviewed_instead_of_false_pass(self):
        """Keep nested payloads explicitly unverified; no extraction or false absence claim."""
        self.tar('nested.tar.gz',{'safe.rs':b'fn main() {}','inner.zip':b'unknown nested source'});self.stage('nested.tar.gz');result=audit.inventory(self.root);self.assertEqual(result['status'],'REVIEW_REQUIRED');self.assertTrue(result['unreviewed_nested']);self.assertEqual(result['flagged_paths'],[])

    def test_corrupt_archive_preserves_cause_and_never_becomes_empty_success(self):
        """Refuse corrupt metadata with original grouped cause and safe partial report."""
        self.put('broken.tar.gz',b'PRIVATE malformed archive');self.stage('broken.tar.gz')
        with self.assertRaises(audit.InventoryError) as caught:audit.inventory(self.root)
        error=caught.exception;self.assertEqual(error.report['status'],'INCOMPLETE');self.assertIsInstance(error.__cause__,BaseExceptionGroup);self.assertNotIn('PRIVATE',str(error));self.assertTrue(error.report['errors']);self.assertEqual(error.report['errors'][0]['origin'],'index')

    def test_symlink_worktree_is_not_followed(self):
        """Refuse a physical symlink instead of reading its target as the indexed artifact."""
        path=self.tar('source.tar.gz',{'safe.rs':b'fn main() {}'});self.stage('source.tar.gz');self.assertEqual(audit.inventory(self.root)['status'],'PASS');self.put('private-target',b'PRIVATE');path.unlink();path.symlink_to('private-target')
        with self.assertRaises(audit.InventoryError) as caught:audit.inventory(self.root)
        self.assertEqual(caught.exception.report['status'],'INCOMPLETE');self.assertEqual(caught.exception.report['errors'][0]['code'],'publication-worktree-read');self.assertEqual(caught.exception.report['errors'][0]['origin'],'worktree')

    def test_cached_removal_preserves_local_original_and_restores_guard(self):
        """Demonstrate reversible cached-only retirement on an owned fixture repository."""
        path=self.tar('source.tar.gz',{'root.go':b'package main'});self.stage('source.tar.gz');before=path.read_bytes();self.assertEqual(audit.inventory(self.root)['status'],'LEGACY_SOURCE_PRESENT');subprocess.run(['git','-c','core.hooksPath=/dev/null','-c','core.fsmonitor=false','-C',str(self.root),'rm','--cached','--quiet','--','source.tar.gz'],check=True);self.assertEqual(path.read_bytes(),before);self.assertEqual(audit.inventory(self.root)['status'],'PASS')

    def test_unrecognized_patch_format_cannot_hide_renamed_source(self):
        """Reject a declared patch without recognized headers, even after a healthy control."""
        self.put('source.patch',b'diff --git a/README.md b/README.md\n+Documentation\n');self.stage('source.patch');self.assertEqual(audit.inventory(self.root)['status'],'PASS')
        self.put('source.patch',b'package main\nfunc main() {}\n');self.stage('source.patch')
        with self.assertRaises(audit.InventoryError) as caught:audit.inventory(self.root)
        self.assertEqual(caught.exception.report['status'],'INCOMPLETE');self.assertEqual(caught.exception.report['errors'][0]['code'],'publication-patch-format-unreviewed')

    def test_git_context_override_refuses_before_fixture_initialization(self):
        """No inherited Git index/object context may redirect fixture writes."""
        extra=PublicationInventoryTests('test_bundle_is_local_only_policy_not_a_claim_of_decoded_history')
        try:
            with mock.patch.dict(os.environ,{'GIT_INDEX_FILE':'PRIVATE index override'}):
                with mock.patch.object(subprocess,'run',side_effect=AssertionError('Git must not run before context refusal')):
                    with self.assertRaises(audit.InventoryError):extra.setUp()
        finally:
            if hasattr(extra,'root'):print('Retained pre-init fixture: '+str(extra.root),flush=True)

    def test_bundle_is_local_only_policy_not_a_claim_of_decoded_history(self):
        """Enforce local recovery-container policy without claiming its Git history was decoded."""
        self.put('recovery.bundle',b'fixture opaque Git container');self.stage('recovery.bundle');result=audit.inventory(self.root);self.assertEqual(result['status'],'LEGACY_SOURCE_PRESENT');self.assertTrue(result['paths'][0]['index']['local_only_bundle']);self.assertEqual(result['paths'][0]['index']['go'],[])


if __name__=='__main__':unittest.main()
