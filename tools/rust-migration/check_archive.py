"""Check the real archive helper, with a required fresh proof destination.

Never rerun an evidence script that writes into an earlier proof directory.
This script creates no product policy and imposes no global watchdog.
"""
from pathlib import Path
import gzip,hashlib,json,os,statistics,sys,tarfile,tempfile,time
from qualification_inputs import archive_sources
ROOT=Path(__file__).resolve().parents[2]
def check(output):
    output=Path(output).resolve()
    if output.exists():raise RuntimeError('Fresh archive-check destination required')
    sources=[Path(__file__).resolve(),ROOT/'tools/rust-migration/qualification_inputs.py',ROOT/'internal/engine/work_definition.go']
    before={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sources}
    output.mkdir(parents=True)
    fixture=Path(tempfile.mkdtemp(prefix='archive-fixture-',dir=ROOT/'.cache'))
    payloads={'text.go':sources[-1].read_bytes()*5000,'binary.dat':hashlib.shake_256(b'owned-reproducible-archive-fixture').digest(8*1024*1024)}
    for name,data in payloads.items():(fixture/name).write_bytes(data)
    entries=[(fixture/name,name)for name in payloads];rows=[];expected=None
    for pair in range(3):
        for level in ([9,1]if pair%2==0 else[1,9]):
            path=fixture/(str(pair)+'-'+str(level)+'.tar.gz');start=time.monotonic()
            if level==1:archive_sources(path,entries)
            else:
                with tarfile.open(path,'w:gz',compresslevel=9)as archive:
                    for source,name in entries:archive.add(source,arcname=name)
            seconds=time.monotonic()-start;raw=gzip.decompress(path.read_bytes())
            if expected is None:expected=raw
            assert raw==expected
            with tarfile.open(path)as archive:assert {m.name:archive.extractfile(m).read()for m in archive.getmembers()}==payloads
            rows.append(dict(pair=pair,level=level,seconds=seconds,bytes=path.stat().st_size,tar_sha256=hashlib.sha256(raw).hexdigest()))
    after={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in sources};assert before==after
    report=dict(status='PASS',scope='actual archive helper on owned deterministic mixed fixture; not whole-source benchmark',inputs_before=before,inputs_after=after,samples=rows,median_seconds={level:statistics.median(r['seconds']for r in rows if r['level']==level)for level in [1,9]},raw_tar_and_entries_identical=True,fixture=str(fixture))
    fd,tmp=tempfile.mkstemp(dir=output,prefix='.report.')
    with os.fdopen(fd,'w')as f:json.dump(report,f,indent=2);f.write('\n');f.flush();os.fsync(f.fileno())
    os.replace(tmp,output/'report.json');print(json.dumps(dict(status='PASS',output=str(output),median_seconds=report['median_seconds'])))
    return report
if __name__=='__main__':
    if len(sys.argv)!=2:raise SystemExit('usage: check_archive.py FRESH_OUTPUT_DIRECTORY')
    check(sys.argv[1])
