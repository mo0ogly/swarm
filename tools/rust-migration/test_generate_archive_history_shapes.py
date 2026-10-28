"""Frozen schema replay, rejection and failed-publication recovery, no Go source."""
from pathlib import Path
import copy,errno,subprocess,sys,tempfile,unittest
from unittest.mock import patch
from generate_archive_history_shapes import ROOT,generate,load_layout,main
class ArchiveShapes(unittest.TestCase):
    """Developer-only checks; no native Root or provider operation is authorized."""
    def test_exact_formatted_replay_without_go_source(self):
        """Reproduce checked bytes while denying all reads of legacy .go files."""
        read_bytes=Path.read_bytes;read_text=Path.read_text
        def checked_bytes(p,*a,**kw):
            if p.suffix=='.go':raise AssertionError('legacy source read')
            return read_bytes(p,*a,**kw)
        def checked_text(p,*a,**kw):
            if p.suffix=='.go':raise AssertionError('legacy source read')
            return read_text(p,*a,**kw)
        with patch.object(Path,'read_bytes',checked_bytes),patch.object(Path,'read_text',checked_text):raw=generate(load_layout())
        r=subprocess.run(['rustfmt','--edition','2021','--emit','stdout'],input=raw,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
        self.assertEqual(r.stdout,(ROOT/'rust/crates/swarm-engine/src/archive_history_shapes.rs').read_bytes())
    def test_unknown_external_method_and_type_refuse(self):
        """Each poison starts from a valid witness, no partial generated source."""
        for target in ['external','custom','field']:
            original=load_layout();self.assertTrue(generate(original));bad=copy.deepcopy(original)
            if target=='external':bad['external']=['net.Conn']
            elif target=='custom':bad['custom_methods']={'Bundle':[{'method':'UnmarshalJSON'}]}
            else:bad['types']['Bundle']['fields'][0]['go_type']='Uninventoried'
            with self.assertRaises(ValueError):generate(bad)
    def test_failed_replace_preserves_original_and_recovers(self):
        """ENOSPC before rename leaves the original; a later public run recovers."""
        self.assertTrue(generate(load_layout()))
        with tempfile.TemporaryDirectory(prefix='archive-shapes-',dir=ROOT/'.cache') as d:
            output=Path(d)/'shapes.rs';output.write_bytes(b'original source\n')
            with patch.object(sys,'argv',['generator','--output',str(output)]):
                with patch('generate_archive_history_shapes.os.replace',side_effect=OSError(errno.ENOSPC,'owned injected refusal')):
                    with self.assertRaises(OSError) as raised:main()
                self.assertEqual(raised.exception.errno,errno.ENOSPC);self.assertEqual(output.read_bytes(),b'original source\n');self.assertEqual(list(Path(d).glob('.archive-shapes-*')),[])
                main();self.assertEqual(output.read_bytes(),generate(load_layout()))
if __name__=='__main__':unittest.main(verbosity=2)
