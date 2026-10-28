#!/usr/bin/env python3
"""Verify pinned table generation/refusal/recovery with explicit upstream source.

Never alter the real cache or repository tables. Copies only the five generator
inputs into a private fixture; no Go/provider/network runs and no skipped pass.
"""
import argparse
import errno
import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT=Path(__file__).resolve().parents[1]
SPEC=importlib.util.spec_from_file_location('verified_terminal_tables',ROOT/'tools/rust-migration/generate_terminal_clip.py')
generator=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(generator)
SOURCE=None

class TerminalTools(unittest.TestCase):
    """Public generator contracts and atomic pre-publication failure recovery."""
    def setUp(self):
        """Copy only source inputs to this test's isolated filesystem fixture."""
        self.temp=tempfile.TemporaryDirectory(prefix='terminal-tools-')
        self.addCleanup(self.temp.cleanup);self.root=Path(self.temp.name)
        self.source=self.root/'source';self.source.mkdir()
        for name in generator.EXPECTED:shutil.copyfile(SOURCE/name,self.source/name)
        self.output=self.root/'tables.rs'
    def run_generator(self,check=False):
        """Invoke the actual generator CLI once, retaining complete original channels."""
        argv=[sys.executable,str(ROOT/'tools/rust-migration/generate_terminal_clip.py'),
              '--source',str(self.source),'--output',str(self.output)]
        if check:argv.append('--check')
        return subprocess.run(argv,capture_output=True,text=True)
    def test_generate_check_refusal_recovery(self):
        """Valid output precedes corrupt-output check refusal and exact recovery."""
        good=self.run_generator();self.assertEqual(good.returncode,0,good.stderr)
        expected=self.output.read_bytes()
        self.assertEqual(expected,(ROOT/'rust/crates/swarm-engine/src/terminal_clip/properties.rs').read_bytes())
        self.assertEqual(self.run_generator(True).returncode,0)
        self.output.write_bytes(expected+b'corrupt fixture')
        refused=self.run_generator(True);self.assertNotEqual(refused.returncode,0)
        self.assertEqual(self.output.read_bytes(),expected+b'corrupt fixture')
        self.assertEqual(self.run_generator().returncode,0)
        self.assertEqual(self.output.read_bytes(),expected)
        self.assertEqual(self.run_generator(True).returncode,0)
    def test_invalid_source_preserves_output_and_recovers(self):
        """A pinned-source mismatch refuses before publication, preserving the original."""
        self.assertEqual(self.run_generator().returncode,0);before=self.output.read_bytes()
        path=self.source/'properties.go';original=path.read_bytes();path.write_bytes(original+b'\n')
        refused=self.run_generator();self.assertNotEqual(refused.returncode,0)
        self.assertIn('fingerprint mismatch',refused.stderr);self.assertEqual(self.output.read_bytes(),before)
        path.write_bytes(original);self.assertEqual(self.run_generator(True).returncode,0)
    def test_replace_failure_preserves_original_and_recovers(self):
        """After a valid write, injected pre-rename ENOSPC keeps original bytes/mode."""
        generator.publish(self.output,b'valid control');self.output.chmod(0o640)
        with patch.object(generator.os,'replace',side_effect=OSError(errno.ENOSPC,'fixture disk full')):
            with self.assertRaises(OSError) as caught:generator.publish(self.output,b'failed replacement')
        self.assertEqual(caught.exception.errno,errno.ENOSPC)
        self.assertEqual(self.output.read_bytes(),b'valid control')
        self.assertEqual(self.output.stat().st_mode&0o777,0o640)
        self.assertEqual(list(self.root.glob('.unicode-*')),[])
        generator.publish(self.output,b'recovered');self.assertEqual(self.output.read_bytes(),b'recovered')
        self.assertEqual(self.output.stat().st_mode&0o777,0o640)

    def test_fdopen_failure_closes_descriptor_and_recovers(self):
        """Stream-construction failure closes its descriptor and preserves original output."""
        generator.publish(self.output,b'valid control');closed=[];close=os.close
        def record_close(fd):
            """Close the real fixture descriptor and record the observed release."""
            close(fd);closed.append(fd)
        with patch.object(generator.os,'fdopen',side_effect=OSError(errno.EIO,'fixture fdopen failure')):
            with patch.object(generator.os,'close',side_effect=record_close):
                with self.assertRaises(OSError):generator.publish(self.output,b'failed')
        self.assertEqual(len(closed),1)
        with self.assertRaises(OSError) as error:os.fstat(closed[0])
        self.assertEqual(error.exception.errno,errno.EBADF)
        self.assertEqual(self.output.read_bytes(),b'valid control')
        generator.publish(self.output,b'recovered');self.assertEqual(self.output.read_bytes(),b'recovered')
    def test_fdopen_and_close_errors_both_survive(self):
        """Preserve the original stream error and secondary descriptor-close error."""
        generator.publish(self.output,b'valid control');descriptors=[]
        def fail_close(fd):
            """Retain the descriptor for test cleanup and inject a secondary error."""
            descriptors.append(fd);raise OSError(errno.EACCES,'fixture close failure')
        try:
            with patch.object(generator.os,'fdopen',side_effect=OSError(errno.EIO,'fixture fdopen failure')):
                with patch.object(generator.os,'close',side_effect=fail_close):
                    with self.assertRaises(BaseExceptionGroup) as errors:generator.publish(self.output,b'failed')
            self.assertEqual([e.errno for e in errors.exception.exceptions],[errno.EIO,errno.EACCES])
            self.assertEqual(self.output.read_bytes(),b'valid control')
        finally:
            for fd in descriptors:os.close(fd)

class OracleInputs(unittest.TestCase):
    """Unexpected compiler inputs must refuse before invoking the Go toolchain."""
    def test_extra_go_and_native_sources_refuse_then_recover(self):
        """Valid pinned inputs precede extra-source refusals and exact recovery."""
        spec=importlib.util.spec_from_file_location('checked_terminal_oracle',ROOT/'tests/generate_terminal_clip_oracle.py')
        oracle=importlib.util.module_from_spec(spec);spec.loader.exec_module(oracle)
        with tempfile.TemporaryDirectory(prefix='oracle input ') as temp:
            source=Path(temp)
            for name in oracle.EXPECTED:shutil.copyfile(SOURCE/name,source/name)
            oracle.validate_source(source)
            extra=source/'unexpected.go';extra.write_text('package uniseg\n')
            with self.assertRaisesRegex(ValueError,'compilation input set'):oracle.validate_source(source)
            extra.unlink();oracle.validate_source(source)
            native=source/'unexpected.s';native.write_text('')
            with self.assertRaisesRegex(ValueError,'native compilation input'):oracle.validate_source(source)
            native.unlink();oracle.validate_source(source)

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--source',type=Path,required=True)
    args,remaining=parser.parse_known_args();SOURCE=args.source.resolve()
    unittest.main(argv=[sys.argv[0],*remaining])
