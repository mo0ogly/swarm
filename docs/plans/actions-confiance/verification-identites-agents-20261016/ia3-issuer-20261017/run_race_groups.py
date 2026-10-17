from pathlib import Path
import json,re,subprocess,sys
B=Path(__file__).resolve().parent
normal=json.loads((B/"tests-final-run.json").read_text())
expected={n for n in normal["names"]["pass"] if n.startswith("TestAgentIdentityIssuer")}
parent="TestAgentIdentityIssuerRejectsUnenrolledOrExpandedAuthority"
children=sorted(n.split("/",1)[1] for n in expected if n.startswith(parent+"/"))
other_roots=sorted({n.split("/",1)[0] for n in expected if n.split("/",1)[0]!=parent})
groups=[]
for i in range(0,len(children),7):
    group=children[i:i+7]
    groups.append("^"+parent+"$/^("+"|".join(re.escape(x) for x in group)+")$")
groups.append("^("+"|".join(re.escape(x) for x in other_roots)+")$")
(B/"race-groups-selection.json").write_text(json.dumps({"expected":sorted(expected),"groups":groups,"reason":"cumulative schema migration cost timed out unpartitioned run; same frozen inputs/cases, sequential groups, compiled cache reused, same 90s per-process deadline"},indent=2)+"\n")
covered=set();names=[]
for i,expression in enumerate(groups,1):
    name="race-recovery-"+str(i).zfill(2)
    result=subprocess.run([sys.executable,str(B/"run_checks.py"),name,"go","test","-race","./internal/engine","-run",expression,"-count=1","-timeout","90s","-json"],capture_output=True,text=True)
    (B/(name+"-runner.stdout")).write_text(result.stdout)
    (B/(name+"-runner.stderr")).write_text(result.stderr)
    run=json.loads((B/(name+"-run.json")).read_text())
    print(json.dumps({"group":name,"exit":result.returncode,"counts":run["counts"],"changed_inputs":run["changed_inputs"]}),flush=True)
    if result.returncode or not run["counts"]["pass"] or run["counts"]["fail"] or run["counts"]["skip"] or run["changed_inputs"]:sys.exit(1)
    covered.update(run["names"]["pass"]);names.append(name)
missing=expected-covered;extra=covered-expected
summary={"groups":names,"expected_count":len(expected),"unique_pass_count":len(covered),"missing":sorted(missing),"extra":sorted(extra),"status":"PASS" if not missing and not extra else "FAIL","source_inputs_unchanged":True,"original_timeout_retained":"race-final-run.json"}
(B/"race-groups-summary.json").write_text(json.dumps(summary,indent=2)+"\n")
print(json.dumps(summary),flush=True)
assert not missing and not extra,summary
