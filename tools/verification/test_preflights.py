#!/usr/bin/env python3
"""Exercise archive/Git public CLIs on retained, isolated fictional fixtures.

No project refs, settings, archives or services are mutated. Git commits use an
explicit fixture identity, not project attribution. Every fault follows a valid
control. Fixtures remain under the configured area even on failure; raw CLI
outputs are retained there. No provider, network request or global watchdog.
"""
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tests'))
from fixture_area import fixture_root


class Fixtures(unittest.TestCase):
    """Allocate retained governed roots and capture every public command result."""
    def setUp(self):
        """Create an owned test root; never reuse or clean historical evidence."""
        self.root = fixture_root('preflight-')
        self.calls = 0
        print('PREFLIGHT_FIXTURE ' + str(self.root), flush=True)

    def cli(self, script, args, extra_env=None):
        """Run a CLI without timeout, retaining raw argv/exit/streams in the fixture."""
        env = dict(os.environ)
        env.update(extra_env or {})
        argv = [sys.executable, str(ROOT / 'tools/verification' / script), *map(str, args)]
        result = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True)
        self.calls += 1
        prefix = self.root / f'call-{self.calls}'
        prefix.with_suffix('.stdout').write_bytes(result.stdout)
        prefix.with_suffix('.stderr').write_bytes(result.stderr)
        prefix.with_suffix('.json').write_text(json.dumps({'argv':argv,'exit':result.returncode})+'\n')
        report = json.loads(result.stdout)
        return result.returncode, report


class ArchivePreflight(Fixtures):
    """Check expected regular members, split reads, refusals and recovery."""
    def setUp(self):
        """Create a valid control and manifest before each independent fault."""
        super().setUp()
        self.member = 'evidence/report.json'
        self.payload = b'{"result":"fictional control"}\n'
        self.valid = self.root / 'valid.tar'
        self.write_tar(self.valid, [(self.member, 'file', self.payload)])
        self.manifest = self.root / 'expected.json'
        self.manifest.write_text(json.dumps({'schema_version':1,'members':{
            self.member:hashlib.sha256(self.payload).hexdigest()}}))
        code, report = self.check(self.valid)
        self.assertEqual((code, report['status']), (0, 'PASS'))

    def write_tar(self, path, entries):
        """Write an owned archive with explicit fictional member type and bytes."""
        with tarfile.open(path, 'w') as tar:
            for name, kind, raw in entries:
                item = tarfile.TarInfo(name)
                if kind == 'file':
                    item.size = len(raw)
                    tar.addfile(item, io.BytesIO(raw))
                else:
                    item.type = {'directory':tarfile.DIRTYPE,'symlink':tarfile.SYMTYPE,
                                 'hardlink':tarfile.LNKTYPE}[kind]
                    item.linkname = self.member
                    tar.addfile(item)

    def check(self, *parts, manifest=None):
        """Invoke the actual archive CLI with caller-ordered explicit parts."""
        args = ['--manifest', manifest or self.manifest]
        for p in parts:
            args += ['--part', p]
        return self.cli('archive_preflight.py', args)

    def refuses(self, parts, code, manifest=None):
        """Require a precise refusal, then prove the unchanged valid control recovers."""
        status, report = self.check(*parts, manifest=manifest)
        self.assertNotEqual(status, 0)
        self.assertEqual(report['status'], 'FAIL')
        self.assertIn(code, [r['code'] for r in report['errors']])
        self.assertEqual(self.check(self.valid)[0], 0)

    def test_archive_read_only_exact_control(self):
        """Verify archive read only exact control using owned fixture inputs."""
        before = hashlib.sha256(self.valid.read_bytes()).hexdigest()
        paths = set(self.root.iterdir())
        code, report = self.check(self.valid)
        self.assertEqual(code, 0)
        self.assertEqual(report['members'][0]['actual_sha256'],
                         hashlib.sha256(self.payload).hexdigest())
        self.assertEqual(hashlib.sha256(self.valid.read_bytes()).hexdigest(), before)
        self.assertFalse((self.root/'evidence').exists())
        self.assertTrue(paths.issubset(set(self.root.iterdir())))

    def test_archive_split_crosses_header_and_payload(self):
        """Verify archive split crosses header and payload using owned fixture inputs."""
        raw = self.valid.read_bytes()
        paths = [self.root/'one.part', self.root/'two.part', self.root/'three.part']
        for path, payload in zip(paths, [raw[:509], raw[509:521], raw[521:]]):
            path.write_bytes(payload)
        self.assertEqual(self.check(*paths)[0], 0)
        self.refuses(list(reversed(paths)), 'archive_invalid')

    def test_archive_directory_only_pointer_has_no_original(self):
        """Verify archive directory only pointer has no original using owned fixture inputs."""
        bad = self.root/'directories.tar'
        self.write_tar(bad, [('evidence', 'directory', b'')])
        self.refuses([bad], 'member_missing')

    def test_archive_selected_non_regular_refuses(self):
        """Verify archive selected non regular refuses using owned fixture inputs."""
        for kind in ['directory','symlink','hardlink']:
            with self.subTest(kind=kind):
                self.assertEqual(self.check(self.valid)[0], 0)
                bad=self.root/(kind+'.tar')
                self.write_tar(bad, [(self.member,kind,b'')])
                self.refuses([bad], 'member_non_regular')

    def test_archive_late_duplicate_refuses(self):
        """Verify archive late duplicate refuses using owned fixture inputs."""
        bad=self.root/'duplicate.tar'
        self.write_tar(bad, [(self.member,'file',self.payload),('unselected','file',b'x'),
                             (self.member,'file',self.payload)])
        self.refuses([bad], 'member_duplicate')

    def test_archive_wrong_digest_refuses(self):
        """Verify archive wrong digest refuses using owned fixture inputs."""
        bad=self.root/'wrong.tar'
        self.write_tar(bad, [(self.member,'file',b'changed')])
        self.refuses([bad], 'member_digest')

    def test_archive_truncated_selected_payload_refuses(self):
        """Verify archive truncated selected payload refuses using owned fixture inputs."""
        bad=self.root/'truncated.tar'
        bad.write_bytes(self.valid.read_bytes()[:513])
        self.refuses([bad], 'archive_invalid')

    def test_archive_bad_manifests_refuse(self):
        """Verify archive bad manifests refuse using owned fixture inputs."""
        values=['{}','{"schema_version":1,"members":{}}',
                '{"schema_version":true,"members":{"x":"'+'0'*64+'"}}',
                '{"schema_version":1,"members":{"../x":"'+'0'*64+'"}}',
                '{"schema_version":1,"members":{"x":"no-hash"}}',
                '{"schema_version":1,"members":{"x":"'+'0'*64+'","x":"'+'0'*64+'"}}']
        for i, text in enumerate(values):
            with self.subTest(i=i):
                self.assertEqual(self.check(self.valid)[0], 0)
                path=self.root/f'invalid-{i}.json';path.write_text(text)
                self.refuses([self.valid], 'manifest_invalid', path)

    def test_archive_missing_or_alias_part_refuses(self):
        """Verify archive missing or alias part refuses using owned fixture inputs."""
        self.refuses([self.root/'absent.tar'], 'archive_source')
        self.refuses([self.valid,self.valid], 'archive_source')
        alias=self.root/'alias.tar';alias.symlink_to(self.valid)
        self.refuses([alias], 'archive_source')

    def test_archive_bad_terminator_or_trailing_data_refuses(self):
        """Verify archive bad terminator or trailing data refuses using owned fixture inputs."""
        raw = self.valid.read_bytes()
        variants = [raw[:1024], raw[:1024] + b'corrupt later header' + bytes(1024),
                    raw + b'nonzero after terminator']
        for i, payload in enumerate(variants):
            with self.subTest(i=i):
                self.assertEqual(self.check(self.valid)[0], 0)
                bad=self.root/f'bad-ending-{i}.tar';bad.write_bytes(payload)
                self.refuses([bad], 'archive_invalid')

    def test_archive_descriptor_detects_changed_source(self):
        """Verify archive descriptor detects changed source using owned fixture inputs."""
        spec=importlib.util.spec_from_file_location('archive_preflight',ROOT/'tools/verification/archive_preflight.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        source=self.root/'changing.tar';source.write_bytes(self.valid.read_bytes())
        with module.SplitArchive([source]) as stream:
            stream.check_stable()
            source.write_bytes(source.read_bytes()+b'changed')
            with self.assertRaises(module.Refusal) as raised:
                stream.check_stable()
            self.assertEqual(raised.exception.code, 'archive_changed')
        self.assertEqual(self.check(self.valid)[0], 0)


class GitPreflight(Fixtures):
    """Inspect fictional local refs/dirty work without mutating them via the CLI."""
    def setUp(self):
        """Initialize a private local Git fixture and a valid tracking/source control."""
        super().setUp()
        self.repo=self.root/'repo';self.repo.mkdir()
        self.git('init','--initial-branch=main')
        (self.repo/'data.txt').write_text('base\n')
        self.git('add','data.txt');self.git('commit','-m','fixture base')
        self.base=self.git('rev-parse','HEAD').strip()
        self.git('update-ref','refs/remotes/origin/main',self.base)
        self.assertEqual(self.check()[0],0)

    def git(self,*args):
        """Mutate only this fixture; disable external config/hooks/signing and network."""
        env={k:v for k,v in os.environ.items() if not k.startswith('GIT_')}
        env.update(GIT_CONFIG_GLOBAL=os.devnull,GIT_CONFIG_NOSYSTEM='1')
        argv=['git','-C',str(self.repo),'-c','user.name=Fixture','-c',
              'user.email=fixture@example.invalid','-c','core.hooksPath='+os.devnull,
              '-c','commit.gpgsign=false',*args]
        return subprocess.run(argv,env=env,capture_output=True,text=True,check=True).stdout

    def check(self, source=None, extra_env=None):
        """Run the Git CLI against explicit expected branch/tracking/source refs."""
        return self.cli('git_preflight.py',['--root',self.repo,'--expected-branch','main',
                        '--tracking-ref','refs/remotes/origin/main','--source-ref',source or self.base],extra_env)

    def refuses(self,code,source=None):
        """Require the intended refusal after the setUp valid witness."""
        status, report=self.check(source)
        self.assertNotEqual(status,0)
        self.assertEqual(report['status'],'FAIL')
        self.assertIn(code,[x['code'] for x in report['errors']])

    def advance(self):
        """Create one additional commit in this fixture, returning its real SHA."""
        (self.repo/'data.txt').write_text('second\n')
        self.git('add','data.txt');self.git('commit','-m','fixture second')
        return self.git('rev-parse','HEAD').strip()

    def test_git_equal_and_dirty_work_are_observed_without_writes(self):
        """Verify git equal and dirty work are observed without writes using owned fixture inputs."""
        (self.repo/'data.txt').write_text('preserve this\n')
        name='untracked with space\nand newline.txt'
        (self.repo/name).write_bytes(b'preserve untracked')
        before={p:hashlib.sha256(p.read_bytes()).hexdigest() for p in
                [self.repo/'data.txt',self.repo/name,self.repo/'.git/index']}
        code,report=self.check()
        self.assertEqual(code,0)
        self.assertTrue(report['dirty'])
        self.assertIn(name,report['dirty_paths'])
        self.assertFalse(report['dirty_contents_saved'])
        self.assertTrue(report['checkpoint_required_before_mutation'])
        self.assertEqual(report['tracking_relation'],'equal')
        for path,digest in before.items():self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(),digest)

    def test_git_ahead_contains_source(self):
        """Verify git ahead contains source using owned fixture inputs."""
        self.advance();code,report=self.check()
        self.assertEqual(code,0);self.assertEqual(report['tracking_relation'],'ahead')
        self.assertTrue(report['source_in_head'])

    def test_git_behind_refuses_then_local_tracking_recovery(self):
        """Verify git behind refuses then local tracking recovery using owned fixture inputs."""
        tip=self.advance();self.git('update-ref','refs/remotes/origin/main',tip)
        self.git('update-ref','refs/heads/main',self.base)
        self.refuses('tracking_behind')
        self.git('update-ref','refs/heads/main',tip)
        self.assertEqual(self.check()[0],0)

    def test_git_divergence_refuses(self):
        """Verify git divergence refuses using owned fixture inputs."""
        tip=self.advance()
        self.git('update-ref','refs/remotes/origin/main',tip)
        self.git('update-ref','refs/heads/main',self.base)
        (self.repo/'data.txt').write_text('other history\n')
        self.git('add','data.txt');self.git('commit','-m','fixture diverged')
        self.refuses('tracking_diverged')

    def test_git_missing_source_refuses(self):
        """Verify git missing source refuses using owned fixture inputs."""
        self.refuses('source_missing','refs/heads/absent')

    def test_git_source_not_in_head_refuses(self):
        """Verify git source not in head refuses using owned fixture inputs."""
        tip=self.advance();self.git('update-ref','refs/heads/main',self.base)
        self.refuses('source_not_in_head',tip)

    def test_git_missing_tracking_refuses(self):
        """Verify git missing tracking refuses using owned fixture inputs."""
        self.git('update-ref','-d','refs/remotes/origin/main')
        self.refuses('tracking_missing')

    def test_git_wrong_branch_refuses(self):
        """Verify git wrong branch refuses using owned fixture inputs."""
        self.git('checkout','-b','fixture-other')
        self.refuses('branch_mismatch')

    def test_git_detached_refuses(self):
        """Verify git detached refuses using owned fixture inputs."""
        self.git('checkout','--detach',self.base)
        self.refuses('branch_mismatch')

    def test_git_changed_observation_refuses(self):
        """Verify git changed observation refuses using owned fixture inputs."""
        from unittest.mock import patch
        spec=importlib.util.spec_from_file_location('git_preflight',ROOT/'tools/verification/git_preflight.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        original=module.observe
        count=0
        def changed(*args):
            """Alter only the second fixture observation to exercise freshness refusal."""
            nonlocal count
            count += 1
            result=original(*args)
            if count == 2:result['dirty_status_sha256']='changed-observation'
            return result
        with patch.object(module,'observe',changed):
            report=module.inspect_git(self.repo,'main','refs/remotes/origin/main',self.base)
        self.assertEqual(report['status'],'FAIL')
        self.assertIn('git_observation_changed',[x['code'] for x in report['errors']])
        self.assertEqual(self.check()[0],0)

    def test_git_wrong_root_refuses(self):
        """Verify git wrong root refuses using owned fixture inputs."""
        code,report=self.cli('git_preflight.py',['--root',self.root,
             '--expected-branch','main','--tracking-ref','refs/remotes/origin/main',
             '--source-ref',self.base])
        self.assertNotEqual(code,0)
        self.assertEqual(report['status'],'FAIL')
        self.assertEqual(self.check()[0],0)

    def test_git_renamed_paths_and_injected_repository_env(self):
        """Verify git renamed paths and injected repository env using owned fixture inputs."""
        self.git('mv','data.txt','renamed data.txt')
        code,report=self.check(extra_env={'GIT_DIR':str(self.root/'absent-git')})
        self.assertEqual(code,0)
        self.assertIn('renamed data.txt',report['dirty_paths'])
        self.assertIn('data.txt',report['dirty_paths'])
        self.assertEqual(Path(report['root']),self.repo.resolve())


if __name__ == '__main__':
    suite=unittest.defaultTestLoader.loadTestsFromModule(sys.modules[__name__])
    result=unittest.TextTestRunner(verbosity=2).run(suite)
    print(json.dumps({'status':'PASS' if result.wasSuccessful() else 'FAIL',
                      'tests':result.testsRun,'failures':len(result.failures),'errors':len(result.errors)}))
    if result.wasSuccessful():print('PREFLIGHT_TESTS_PASS')
    raise SystemExit(0 if result.wasSuccessful() else 1)
