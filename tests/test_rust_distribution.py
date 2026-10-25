"""Rust-only distribution regression guards, with an isolated installer fixture.

No real project, daemon or compiler invocation in the fixture: actual native and
Docker installation have separate public recipes. Original binary must survive a
failed build; no Go prerequisite or fallback may appear in delivery entry points.
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT=Path(__file__).resolve().parents[1]

class RustDistribution(unittest.TestCase):
    """Delivery contracts, not proof that every historical capability is ported."""
    def test_no_go_delivery_commands(self):
        """Check actual production installer/build/CI entries, not all history."""
        import re
        for name in ['install.sh','Dockerfile','.github/workflows/ci.yml','Makefile','build.sh','swarm.sh']:
            with self.subTest(path=name):
                self.assertIsNone(re.search(r'golang:|actions/setup-go|\bgo (?:build|mod|vet|test)\b|command -v go\b|supervision_go_suite', (ROOT/name).read_text()))
    def test_help_names_rust_prerequisite(self):
        """Help is side-effect free and advertises the actual build toolchain."""
        r=subprocess.run([str(ROOT/'install.sh'),'--help'],capture_output=True,text=True,check=True)
        self.assertIn('Rust',r.stdout);self.assertNotIn('Go 1.',r.stdout)
    def test_native_failure_preserves_installed_binary(self):
        """Failed candidate cannot replace a user's existing executable or init Root.

        Fake tools deliberately isolate the shell publication contract; they do
        not establish native compilation, Root initialization or runtime parity.
        """
        with tempfile.TemporaryDirectory(prefix='swarm-installer-guard-') as td:
            root=Path(td);repo=root/'repo';repo.mkdir();shutil.copy2(ROOT/'install.sh',repo/'install.sh');(repo/'rust').mkdir();(repo/'rust/Cargo.toml').write_text('[workspace]\n')
            fake=root/'tools';fake.mkdir();log=root/'calls';project=root/'project';project.mkdir();dest=root/'bin';dest.mkdir();binary=dest/'swarm';binary.write_bytes(b'ORIGINAL');binary.chmod(0o755)
            for tool in ['cargo','rustc','cc','git']:
                p=fake/tool;p.write_text('#!/bin/sh\nprintf "%s\\n" "'+tool+' $*" >> "$INSTALL_GUARD_LOG"\nexit 0\n');p.chmod(0o755)
            p=fake/'go';p.write_text('#!/bin/sh\nexit 97\n');p.chmod(0o755)
            (repo/'build.sh').write_text('#!/bin/sh\nprintf "BROKEN" > "$1"\nexit 23\n')
            r=subprocess.run([str(repo/'install.sh'),'--mode','native','--bin-dir',str(dest),'--project',str(project)],env={**os.environ,'PATH':str(fake)+':'+os.environ['PATH'],'INSTALL_GUARD_LOG':str(log)},capture_output=True)
            self.assertEqual(r.returncode,23);self.assertEqual(binary.read_bytes(),b'ORIGINAL');self.assertEqual(list(project.iterdir()),[]);self.assertEqual(sorted(p.name for p in dest.iterdir()),['swarm'])
            self.assertIn('cargo fetch --locked',log.read_text())

    def test_docker_dispatch_has_no_arbitrary_global_wait_limit(self):
        """Mock only Docker invocation; actual images/API have a separate recipe.

        Verify persisted paths/port, private config and caller arguments without
        a daemon, Root mutation, shared installation config or model execution.
        """
        with tempfile.TemporaryDirectory(prefix='swarm-compose-shell-') as td:
            root=Path(td);repo=root/'repo';repo.mkdir();(repo/'deploy').mkdir();shutil.copy2(ROOT/'install.sh',repo/'install.sh')
            project=root/'project';project.mkdir();agents=root/'agents';fake=root/'tools';fake.mkdir();log=root/'calls'
            docker=fake/'docker';docker.write_text('#!/bin/sh\nprintf \'%s\\n\' "$*" >> "$INSTALL_GUARD_LOG"\nexit 0\n');docker.chmod(0o755)
            r=subprocess.run([str(repo/'install.sh'),'--project',str(project),'--agent-home',str(agents),'--port','18788'],env={**os.environ,'PATH':str(fake)+':'+os.environ['PATH'],'INSTALL_GUARD_LOG':str(log)},capture_output=True,text=True)
            self.assertEqual(r.returncode,0,r.stderr)
            calls=log.read_text();self.assertIn('up -d --wait',calls);self.assertNotIn('--wait-timeout',calls)
            config=repo/'deploy/install.env';self.assertEqual(config.stat().st_mode&0o777,0o600);self.assertIn('SWARM_PORT=18788',config.read_text())
            self.assertIn('/login',r.stdout);self.assertNotIn('/session/',r.stdout)

if __name__=='__main__':unittest.main()
