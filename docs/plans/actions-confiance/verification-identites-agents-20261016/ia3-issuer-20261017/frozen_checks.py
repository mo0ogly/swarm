from pathlib import Path
import datetime,hashlib,json,subprocess,sys,tempfile,time
B=Path(__file__).resolve().parent;ROOT=Path.cwd()
VIEW=json.loads((B/"frozen-embed-view.json").read_text());OVERLAY=VIEW["overlay"]
REPLACE=json.loads(Path(OVERLAY).read_text())["Replace"]

def digest(p):return hashlib.sha256(Path(p).read_bytes()).hexdigest()
def inputs():
    raw=subprocess.check_output(["go","list","-overlay="+OVERLAY,"-json","./","./internal/engine","./cmd/swarm"],text=True)
    decoder=json.JSONDecoder();paths={ROOT/"go.mod",ROOT/"go.sum"}
    while raw.strip():
        package,n=decoder.raw_decode(raw.lstrip());raw=raw.lstrip()[n:]
        for key in ["GoFiles","CgoFiles","TestGoFiles","XTestGoFiles","EmbedFiles","TestEmbedFiles"]:
            paths.update(Path(package["Dir"])/name for name in package.get(key,[]))
    return {str(p.relative_to(ROOT)):digest(REPLACE.get(str(p),p)) for p in sorted(paths)}

def run(name,command,binary=None):
    before=inputs();(B/(name+"-inputs.json")).write_text(json.dumps(before,indent=2)+"\n")
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();stamp=time.monotonic();binary_before=digest(binary) if binary else None
    with (B/(name+".jsonl")).open("wb") as out,(B/(name+".stderr")).open("wb") as err:
        result=subprocess.run(command,stdout=out,stderr=err)
    after=inputs();changed=sorted(k for k in set(before)|set(after) if before.get(k)!=after.get(k));events=[]
    for line in (B/(name+".jsonl")).read_text().splitlines():
        try:events.append(json.loads(line))
        except ValueError:pass
    names={a:sorted({e["Test"] for e in events if e.get("Action")==a and e.get("Test")}) for a in ["pass","fail","skip"]}
    record={"command":command,"overlay":OVERLAY,"start":start,"end":datetime.datetime.now(datetime.timezone.utc).isoformat(),"elapsed_seconds":time.monotonic()-stamp,"exit":result.returncode,"input_count":len(before),"changed_inputs":changed,"counts":{k:len(v) for k,v in names.items()},"names":names,"binary_sha256":binary_before,"binary_unchanged":not binary or digest(binary)==binary_before}
    (B/(name+"-run.json")).write_text(json.dumps(record,indent=2)+"\n")
    print(json.dumps({"step":name,"exit":result.returncode,"counts":record["counts"],"changed_inputs":changed,"elapsed_seconds":record["elapsed_seconds"]}),flush=True)
    assert not changed and record["binary_unchanged"],record
    if result.returncode:sys.exit(result.returncode)
    return record

if __name__=="__main__":
    folder=Path(tempfile.mkdtemp(prefix="swarm-identity-issuer-tests-"));race=folder/"engine-race.test";normal=folder/"engine.test"
    run("frozen-race-build",["go","test","-overlay="+OVERLAY,"-race","-c","-o",str(race),"./internal/engine"])
    race_result=run("frozen-race-final",["go","tool","test2json","-p","swarm.local/companion/internal/engine","-t",str(race),"-test.v","-test.run=^TestAgentIdentityIssuer","-test.count=1","-test.timeout=90s"],race)
    expected={n for n in json.loads((B/"tests-final-run.json").read_text())["names"]["pass"] if n.startswith("TestAgentIdentityIssuer")}
    assert set(race_result["names"]["pass"])==expected and not race_result["names"]["fail"] and not race_result["names"]["skip"],race_result
    run("frozen-vet-final",["go","vet","-overlay="+OVERLAY,"./internal/engine"])
    run("frozen-normal-build",["go","test","-overlay="+OVERLAY,"-c","-o",str(normal),"./internal/engine"])
    normal_result=run("frozen-normal-final",["go","tool","test2json","-p","swarm.local/companion/internal/engine","-t",str(normal),"-test.v","-test.run=^TestAgentIdentityIssuer","-test.count=1","-test.timeout=90s"],normal)
    assert set(normal_result["names"]["pass"])==expected and not normal_result["names"]["fail"] and not normal_result["names"]["skip"],normal_result
    (B/"frozen-binaries.json").write_text(json.dumps({"race":{"path":str(race),"sha256":digest(race)},"normal":{"path":str(normal),"sha256":digest(normal)},"overlay":OVERLAY,"deployed":False,"expected_names":sorted(expected)},indent=2)+"\n")
    print("FROZEN ISSUER CHECKS COMPLETE: "+str(len(expected))+" normal and race names, no skipped tests",flush=True)
