from pathlib import Path
import datetime, hashlib, json, subprocess, sys, time
ROOT=Path.cwd()
BASE=Path(__file__).resolve().parent

def inputs():
    listed=subprocess.check_output(["go","list","-json","./","./internal/engine","./cmd/swarm"],text=True)
    decoder=json.JSONDecoder(); packages=[]
    while listed.strip():
        package,n=decoder.raw_decode(listed.lstrip());packages.append(package);listed=listed.lstrip()[n:]
    paths={ROOT/"go.mod",ROOT/"go.sum"}
    for package in packages:
        for key in ["GoFiles","CgoFiles","TestGoFiles","XTestGoFiles","EmbedFiles","TestEmbedFiles"]:
            paths.update(Path(package["Dir"])/name for name in package.get(key,[]))
    return {str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted(paths)}

if __name__=="__main__":
    name=sys.argv[1]; command=sys.argv[2:];before=inputs()
    (BASE/(name+"-inputs.json")).write_text(json.dumps(before,indent=2)+"\n")
    start=datetime.datetime.now(datetime.timezone.utc).isoformat();stamp=time.monotonic()
    with (BASE/(name+".jsonl")).open("wb") as out,(BASE/(name+".stderr")).open("wb") as err:
        result=subprocess.run(command,stdout=out,stderr=err)
    after=inputs();changed=sorted(k for k in set(before)|set(after) if before.get(k)!=after.get(k))
    events=[]
    for line in (BASE/(name+".jsonl")).read_text().splitlines():
        try:events.append(json.loads(line))
        except json.JSONDecodeError:pass
    names={action:sorted({v["Test"] for v in events if v.get("Test") and v.get("Action")==action}) for action in ["pass","fail","skip"]}
    run={"command":command,"executor":"Codex alone; isolated fixture stores; local OIDC signed-token fixture, no real provider","start":start,"end":datetime.datetime.now(datetime.timezone.utc).isoformat(),"elapsed_seconds":time.monotonic()-stamp,"exit":result.returncode,"input_count":len(before),"changed_inputs":changed,"counts":{k:len(v) for k,v in names.items()},"names":names}
    (BASE/(name+"-run.json")).write_text(json.dumps(run,indent=2)+"\n")
    print(json.dumps({k:v for k,v in run.items() if k!="names"},indent=2))
    for v in events:
        if v.get("Action")=="output" and ("--- FAIL" in v.get("Output","") or "issuer authority" in v.get("Output","") or "permit transferred" in v.get("Output","")):
            print(v["Output"].rstrip())
    assert not changed,changed
    sys.exit(result.returncode)
