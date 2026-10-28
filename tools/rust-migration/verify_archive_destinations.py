"""Exercise actual archive-check call expressions from each affected runner."""
import ast,hashlib,json,subprocess,sys
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
def main(output):
    output=Path(output).resolve()
    if output.exists():raise RuntimeError('Fresh destination proof required')
    output.mkdir(parents=True)
    historical=ROOT/'docs/plans/migration-rust/evidence/20261018-08/archive-helper-check.json'
    before=hashlib.sha256(historical.read_bytes()).hexdigest();rows=[]
    names=['verify_work_definition.py','verify_local_path.py','verify_task_definition.py','verify_recorded_hook.py']
    for name in names:
        source=ROOT/'tools/rust-migration'/name;raw=source.read_bytes()
        calls=[n for n in ast.walk(ast.parse(raw))if isinstance(n,ast.Call)and isinstance(n.func,ast.Name)and n.func.id=='run'and len(n.args)>1 and isinstance(n.args[0],ast.Constant)and n.args[0].value=='archive-helper-check']
        assert len(calls)==1,name+': archive caller missing or ambiguous'
        call=calls[0];directory=output/name.removesuffix('.py');directory.mkdir()
        # Only this inspected argument expression is evaluated, not the runner.
        args=eval(compile(ast.Expression(call.args[1]),str(source),'eval'),{'__builtins__':{},'str':str,'output':directory})
        assert args==['python3','tools/rust-migration/check_archive.py',str(directory/'archive-helper-check')]
        p=subprocess.run(args,cwd=ROOT,capture_output=True)
        (directory/'stdout').write_bytes(p.stdout);(directory/'stderr').write_bytes(p.stderr)
        assert p.returncode==0 and json.loads((directory/'archive-helper-check/report.json').read_text())['status']=='PASS'
        assert source.read_bytes()==raw and hashlib.sha256(historical.read_bytes()).hexdigest()==before
        rows.append(dict(runner=name,runner_sha256=hashlib.sha256(raw).hexdigest(),command=args,exit=p.returncode))
    result=dict(status='PASS',actual_call_expressions_executed=rows,historical_report_sha256_before=before,historical_report_sha256_after=before,scope='Archive invocation paths only, not full application recipes')
    (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(dict(status='PASS',callers=len(rows),historical_unchanged=True)))
if __name__=='__main__':main(sys.argv[1])
