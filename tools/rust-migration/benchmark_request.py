"""Local single-CPU release benchmark of the actual ported Request codecs.

No providers, live databases, process interruption, or global watchdog. Seven
alternating pairs and measured pilots; latency is a batch mean, not request p99.
"""
import datetime
import hashlib
import json
import math
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import sys
import tarfile
import tempfile
import time
import tomllib

from qualification_inputs import collect

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = Path(sys.argv[1]).resolve()
if OUTPUT.exists() and any(OUTPUT.iterdir()):
    raise SystemExit('Fresh evidence directory required')
OUTPUT.mkdir(parents=True, exist_ok=True)
WORK = Path(tempfile.mkdtemp(prefix='swarm-request-performance-'))
ENV = dict(os.environ, GOMAXPROCS='1', CARGO_TARGET_DIR=str(ROOT/'rust/target/r24b'))
commands = []


def save(name, value):
    p = OUTPUT/name
    fd, temporary = tempfile.mkstemp(dir=p.parent, prefix='.'+p.name+'.')
    with os.fdopen(fd, 'w') as f:
        json.dump(value, f, indent=2); f.write('\n'); f.flush(); os.fsync(f.fileno())
    os.replace(temporary, p)


def run(label, args, data=None):
    start = time.monotonic()
    p = subprocess.run(args, cwd=ROOT, env=ENV, input=data, capture_output=True)
    (OUTPUT/(label+'.stdout')).write_bytes(p.stdout)
    (OUTPUT/(label+'.stderr')).write_bytes(p.stderr)
    commands.append(dict(label=label, command=args, seconds=time.monotonic()-start, exit=p.returncode))
    save('commands.json', commands)
    print(label, 'exit', p.returncode, flush=True)
    if p.returncode:
        raise RuntimeError(label+': failure retained')
    return p.stdout


def machine():
    wanted = {'MemTotal', 'MemAvailable', 'SwapTotal', 'SwapFree'}
    memory = {a:b.strip() for a,b in (l.split(':',1) for l in Path('/proc/meminfo').read_text().splitlines()) if a in wanted}
    return dict(utc=datetime.datetime.now(datetime.timezone.utc).isoformat(), load=os.getloadavg(), memory=memory,
                memory_pressure=Path('/proc/pressure/memory').read_text(), cpu_pressure=Path('/proc/pressure/cpu').read_text())


def main():
    before, snapshot, registry, close_graphs = collect(ROOT, OUTPUT, ENV, run, ['./tools/rust-migration/performance-go'], allow_root=True)
    extra = sorted((ROOT/'tools/rust-migration/performance-rust').rglob('*'))
    extra = [p for p in extra if p.is_file() and p.suffix in ('.rs','.toml','.lock')]
    extra_before = {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in extra}
    save('harness-inputs-before.json', extra_before)
    with tarfile.open(OUTPUT/'harness-sources.tar.gz','w:gz') as archive:
        for p in extra: archive.add(p, arcname=str(p.relative_to(ROOT)))
    affinity = sorted(os.sched_getaffinity(0)); cpu = affinity[0]
    save('environment.json', dict(before=machine(), original_affinity=affinity, benchmark_cpu=cpu,
         GOMAXPROCS='1', note='shared workstation; no other processes interrupted; single logical CPU, not isolated',
         cpuinfo=next(l.split(':',1)[1].strip() for l in Path('/proc/cpuinfo').read_text().splitlines() if l.startswith('model name'))))
    manifest = 'tools/rust-migration/performance-rust/Cargo.toml'
    meta = json.loads(run('benchmark-cargo-metadata', ['cargo','metadata','--manifest-path',manifest,'--locked','--offline','--format-version','1']))
    locked = {(p['name'],p['version']):p.get('checksum') for p in tomllib.loads((ROOT/'tools/rust-migration/performance-rust/Cargo.lock').read_text())['package']}
    known = {(p['name'],p['version']):p['crate_sha256'] for p in registry}
    assert all(known.get((p['name'],p['version'])) == locked[(p['name'],p['version'])] for p in meta['packages'] if p['source']), 'unbound benchmark dependency'
    run('benchmark-feature-tree', ['cargo','tree','--manifest-path',manifest,'--locked','--offline','-e','features'])
    assert b'test-fixtures' not in (OUTPUT/'benchmark-feature-tree.stdout').read_bytes()
    go = WORK/'go-performance'
    run('go-production-build', ['go','build','-o',str(go),'./tools/rust-migration/performance-go'])
    run('rust-production-build', ['cargo','build','--manifest-path',manifest,'--release','--locked','--offline'])
    rust = Path(ENV['CARGO_TARGET_DIR'])/'release/swarm-performance-probe'
    binaries = {'Go':go,'Rust':rust}
    small = dict(schema_version=1,event_id='benchmark-event',expected_revision=4,title='Titre local')
    structured = dict(small, id='task-1', objective='Vérifier <le contrat> & la sortie', criteria=['résultat vérifiable','aucune mutation'],
        depends=[], owner='agent-a', memory=['contexte conservé'], requirement_prerequisites={'r2':['r1'],'r1':None},
        validation_policy=dict(mode='automatic',controls=[dict(id='local-check',command=['echo','fixture'],criteria=[1,2],justification='preuve locale',timeout_seconds=0)],actor='opérateur'))
    large = dict(structured, summary=('Texte à encoder <>&\u2028 et vérifier. '*2048))
    datasets = {'small':small,'structured':structured,'large':large}
    records = []; summaries = []

    def sample(lang, name, mode, n, phase):
        label = f'{name}-{mode}-{phase}-{lang.lower()}'
        metrics = OUTPUT/(label+'.time')
        args = ['/usr/bin/time','-f','%U %S %M %e','-o',str(metrics),'taskset','-c',str(cpu),str(binaries[lang]),str(OUTPUT/(name+'.json')),mode,str(n)]
        state = machine(); value = json.loads(run(label,args))
        user,system,rss,wall = metrics.read_text().split()
        assert value['iterations']==n and value['checksum'] == n*(value['output_bytes']+248), 'loop/output integrity'
        record = dict(language=lang,dataset=name,mode=mode,phase=phase,iterations=n,ns_per_op=value['elapsed_ns']/n,
             output_sha256=value['output_sha256'],output_bytes=value['output_bytes'],peak_process_rss_kib=int(rss),
             process_user_seconds=float(user),process_system_seconds=float(system),process_wall_seconds=float(wall),state=state)
        records.append(record); save('samples.json',records)
        return record

    for name, value in datasets.items():
        (OUTPUT/(name+'.json')).write_text(json.dumps(value,ensure_ascii=False,separators=(',',':')))
        for mode in ['encode','roundtrip']:
            controls = [sample(lang,name,mode,1,'control') for lang in binaries]
            assert controls[0]['output_sha256']==controls[1]['output_sha256'] and controls[0]['output_bytes']==controls[1]['output_bytes'], 'output mismatch before timing'
            pilots = [sample(lang,name,mode,128,'pilot') for lang in binaries]
            # At least 150ms for the fastest observed implementation limits timer/startup
            # dominance; this is an experiment size, never a timeout or product policy.
            n = math.ceil(150_000_000/min(p['ns_per_op'] for p in pilots))
            paired = []
            for repetition in range(7):
                order = ['Go','Rust'] if repetition%2==0 else ['Rust','Go']
                pair = {lang:sample(lang,name,mode,n,'repeat-'+str(repetition)) for lang in order}
                assert all(p['output_sha256']==controls[0]['output_sha256'] for p in pair.values())
                paired.append(pair)
            summary = dict(dataset=name,mode=mode,input_bytes=(OUTPUT/(name+'.json')).stat().st_size,iterations_per_sample=n,
                paired_go_ns_over_rust_ns=[p['Go']['ns_per_op']/p['Rust']['ns_per_op'] for p in paired],languages={})
            for lang in binaries:
                rr=[p[lang] for p in paired]; ns=[p['ns_per_op'] for p in rr]; rss=[p['peak_process_rss_kib'] for p in rr]
                summary['languages'][lang]=dict(median_ns_per_op=statistics.median(ns),min_ns_per_op=min(ns),max_ns_per_op=max(ns),
                    median_ops_per_second=1e9/statistics.median(ns),median_peak_process_rss_kib=statistics.median(rss),min_peak_process_rss_kib=min(rss),max_peak_process_rss_kib=max(rss))
            summary['median_paired_go_over_rust']=statistics.median(summary['paired_go_ns_over_rust_ns'])
            summaries.append(summary); save('summary-in-progress.json',summaries)
            print('COMPARED',name,mode,'Go/Rust',round(summary['median_paired_go_over_rust'],3),flush=True)
    close_graphs(); after=snapshot(); assert before==after, 'source drift during experiment'
    assert extra_before=={str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in extra}, 'harness source drift'
    save('inputs-after.json',after)
    artifact_hashes={}
    for lang,binary in binaries.items():
        target=OUTPUT/(lang.lower()+'-performance.binary');shutil.copyfile(binary,target)
        artifact_hashes[lang]=hashlib.sha256(target.read_bytes()).hexdigest()
    save('result.json',dict(status='PASS',scope='Request codecs only; not the whole engine or language ranking',
        summaries=summaries,binary_sha256=artifact_hashes,after=machine(),stable_inputs=len(before),registry_packages=len(registry),
        repetitions=7,pilot_iterations=128,fastest_sample_target_ns=150_000_000,global_watchdog=None,
        benchmark_cpu=cpu,latency_definition='mean per operation in a batch, median over seven batches; not individual tail latency',
        memory_definition='peak whole-process resident KiB including runtime/imports/warmup; not per-operation allocations',
        deployment=False,independent_review=False,fixture_root=str(WORK)))
    print('Benchmark complete: source guards and paired output checks passed',flush=True)


try:
    main()
except Exception as error:
    save('failure.json',dict(type=type(error).__name__,cause=str(error),completed_commands=len(commands)))
    raise
