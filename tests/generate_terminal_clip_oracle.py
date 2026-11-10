#!/usr/bin/env python3
"""Generate retained Go uniseg0.4.7 differential cases without touching Go product/cache.

Explicit source/output/fresh evidence required. Only offline Go runs in a fresh
fixture; no retry/watchdog. Original channels, source fingerprints and toolchain
are retained. Partial execution cannot produce a successful oracle fixture.
"""
import argparse, hashlib, importlib.util, json, os
from pathlib import Path
import re, subprocess, time

GO_MAIN = '\npackage main\n\nimport (\n\t"crypto/sha256"\n\t"encoding/binary"\n\t"encoding/json"\n\t"fmt"\n\t"os"\n\t"strings"\n\t"unicode"\n\n\t"github.com/rivo/uniseg"\n)\n\n// Supplied real Go helpers (from product), replicated verbatim.\nfunc terminalText(s string) string {\n\treturn strings.Map(func(r rune) rune {\n\t\tif unicode.IsControl(r) || unicode.Is(unicode.Cf, r) {\n\t\t\tif r == \'\\n\' || r == \'\\t\' {\n\t\t\t\treturn \' \'\n\t\t\t}\n\t\t\treturn -1\n\t\t}\n\t\treturn r\n\t}, s)\n}\n\nfunc truncateCells(s string, width int) string {\n\tif width < 1 {\n\t\treturn ""\n\t}\n\tg := uniseg.NewGraphemes(s)\n\tend, cells := 0, 0\n\tfor g.Next() {\n\t\tif cells+g.Width() > width {\n\t\t\tbreak\n\t\t}\n\t\tcells += g.Width()\n\t\t_, end = g.Positions()\n\t}\n\treturn s[:end]\n}\n\nfunc clip(text string, width int) string {\n\tr := []rune(terminalText(text))\n\tif width < 1 {\n\t\treturn ""\n\t}\n\tif uniseg.StringWidth(text) > width {\n\t\tif width < 2 {\n\t\t\treturn "…"\n\t\t}\n\t\treturn truncateCells(text, width-1) + "…"\n\t}\n\treturn string(r)\n}\n\ntype clusterOut struct {\n\tEnd   int `json:"end"`\n\tWidth int `json:"width"`\n}\n\ntype itemOut struct {\n\tText     string        `json:"text"`\n\tWidth    int           `json:"width"`\n\tClusters []clusterOut  `json:"clusters"`\n\tClips    map[string]string `json:"clips"`\n}\n\nfunc item(text string) itemOut {\n\tit := itemOut{Text: text, Width: uniseg.StringWidth(text), Clips: map[string]string{}, Clusters: []clusterOut{}}\n\tstate := -1\n\ts := text\n\tfor len(s) > 0 {\n\t\t_, rest, w, newState := uniseg.FirstGraphemeClusterInString(s, state)\n\t\tend := len(text) - len(rest)\n\t\tit.Clusters = append(it.Clusters, clusterOut{End: end, Width: w})\n\t\tstate = newState\n\t\ts = rest\n\t}\n\tfor _, w := range []int{-3,-1,0,1,2,3,5,8,13,100,599,600,601} {\n\t\tit.Clips[fmt.Sprint(w)] = clip(text, w)\n\t}\n\treturn it\n}\n\nfunc main() {\n\tvar texts []string\n\tdec := json.NewDecoder(os.Stdin)\n\tif err := dec.Decode(&texts); err != nil {\n\t\tfmt.Fprintln(os.Stderr, "input decode:", err)\n\t\tos.Exit(1)\n\t}\n\titems := make([]itemOut, 0, len(texts))\n\tfor _, t := range texts {\n\t\titems = append(items, item(t))\n\t}\n\n\t// Stream/hash every valid Unicode scalar value: u32LE scalar then u8 width.\n\th := sha256.New()\n\tvar buf [5]byte\n\tcount := 0\n\tfor r := rune(0); r <= 0x10FFFF; r++ {\n\t\tif r >= 0xD800 && r <= 0xDFFF {\n\t\t\tcontinue\n\t\t}\n\t\tbinary.LittleEndian.PutUint32(buf[:4], uint32(r))\n\t\tw := uniseg.StringWidth(string(r))\n\t\tbuf[4] = byte(w)\n\t\th.Write(buf[:])\n\t\tcount++\n\t}\n\n\tout := map[string]interface{}{\n\t\t"items": items, "unicode_version": unicode.Version,\n\t\t"scalar_width_sha256": fmt.Sprintf("%x", h.Sum(nil)),\n\t\t"scalar_count": count,\n\t}\n\tenc := json.NewEncoder(os.Stdout)\n\tenc.SetEscapeHTML(false)\n\tif err := enc.Encode(out); err != nil {\n\t\tfmt.Fprintln(os.Stderr, "encode:", err)\n\t\tos.Exit(1)\n\t}\n}\n'
EXPECTED = {'doc.go': '7df403cb9db73831ebcc70fd14c546b71dca153b0b5fe49d25a614d3439b4e91', 'eastasianwidth.go': '1cd2368ddedc56182d2f64576f2a713a2ad6234265d940575285f61006527c5b', 'emojipresentation.go': 'c4e1921aafedd787a29a0a6e329b85c6833eb1cfad0cfaf00450783f99e33b9c', 'gen_breaktest.go': '9896daff18016cd14af4f74cbcb5c207f15da84f7bad3f51db48f4e4eaddf3eb', 'gen_properties.go': '6fc3dda9e39d2346e516b463b1ccae7be7594fe9f7e665aeef69e01644ca4a88', 'grapheme.go': '9b19b4dab5926971b5da48e77d33c66bb4f8ffc795b0c61fe52f06ebd08d90ac', 'graphemeproperties.go': '321483625d9a4aa92e4a5730df51c59d5b49725cd20e3879749488e5628b3da8', 'graphemerules.go': '5a4414dfc0402ab1f2b415c0faf89166270f7201a584aee503da69ce6a65d738', 'line.go': 'c71b13b940922b440da48b4f1f48c6df1446aabaa4b96003385983bdb8a2d528', 'lineproperties.go': 'd73432ab989195312fdaa4f4dcea34c7bfb478160b98ad26fbc31df94f898b45', 'linerules.go': '08ad2a2fb188a2349ba76774d0e973711a7ab15d2a865c56e12438a63a433c67', 'properties.go': 'ff66c81730c902cac9d8d7bfaa4a1d4e914ef5f9a04f9b525aad4e1cb507b94f', 'sentence.go': '9883efae40e69051fcf1f1a8c2927f32fbe67e42aba344707a8aaf308fa1b0d7', 'sentenceproperties.go': '272502600803b8e99721cb9d76002a6546ea2350056bf66bb7860c39a2032bd9', 'sentencerules.go': '12ff3986697a1e31c9029c885c39662a267c6bd40e2df88b0466eb7ff34ebf56', 'step.go': '93c8f1f8b5eac3e00393eeb49a616fa08d3fdf5b20e0e29a069c73c4bffeb7e7', 'width.go': '9a76bbb3ef247b33e00a3d5d7b98380d8d3ea46e793b50ab13fbbc8b246008ee', 'word.go': '3b0856f8e6bc473e8129c2611dea482201acdef4e62c6f9ea2073c59fa77808e', 'wordproperties.go': '10136e6c3c8ea619276428d59278e312aa4ce33b2e50d98f3a618f162b3a5864', 'wordrules.go': '7c3ebaf4ed47c37a3e2aa14402b4233ca8d15f3114b48ea0bda183adab268729', 'go.mod': 'a1e174890fc9f252450211fadeb53756f9cf5ab7b19697765f9fb255f81ea898', 'graphemebreak_test.go': 'aecab421fdd529f691386f4015914c542b2f68c6a16301160644e1ad4d6caa2f', 'LICENSE.txt': 'a59885f5f0f3b3c07cf9444db5fe399b6f0791eac82055f7b85cc65500551039'}

def originals(source):
    """Read every official Go interpreted original literal, rejecting unsupported escapes."""
    text=(source/'graphemebreak_test.go').read_text()
    matches=re.findall(r'\{original:\s*("(?:[^"\\]|\\.)*")\s*,',text)
    if not matches or len(matches)!=text.count('{original:'):raise ValueError('incomplete official case extraction')
    out=[]
    for literal in matches:
        literal=re.sub(r'\\U([0-9A-Fa-f]{8})',lambda m:chr(int(m[1],16)),literal)
        value=json.loads(literal);value.encode('utf-8');out.append(value)
    return out

def validate_source(source):
    """Require the exact pinned Go input set; extra compiled source cannot escape pins."""
    expected={name for name in EXPECTED if name.endswith('.go') and not name.endswith('_test.go')}
    actual={p.name for p in source.glob('*.go') if not p.name.endswith('_test.go')}
    if actual!=expected:raise ValueError('unexpected Go compilation input set')
    if any(p.suffix in {'.s','.S','.c','.C','.h'} for p in source.iterdir()):
        raise ValueError('unexpected native compilation input')
    for name,sha in EXPECTED.items():
        if hashlib.sha256((source/name).read_bytes()).hexdigest()!=sha:
            raise ValueError('oracle source fingerprint mismatch: '+name)

def main():
    """Validate immutable inputs, run Go once offline, then atomically publish complete JSON."""
    parser=argparse.ArgumentParser(description=__doc__)
    for name in ['source','output','evidence']:parser.add_argument('--'+name,type=Path,required=True)
    args=parser.parse_args();source=args.source.resolve();output=args.output.resolve();evidence=args.evidence.resolve()
    if output.exists():raise ValueError('oracle output must be new')
    validate_source(source)
    evidence.mkdir(mode=0o700)
    spec=importlib.util.spec_from_file_location('terminal_table_generator',Path(__file__).resolve().parents[1]/'tools/rust-migration/generate_terminal_clip.py')
    gen=importlib.util.module_from_spec(spec);spec.loader.exec_module(gen)
    official=originals(source)
    extras=['', 'x'*600,'x'*601,'é'*600,'\u2e3b'*200,'\u2e3a'*250,'中'*301,
            'e\u0301'*601,'👨\u200d👩\u200d👧\u200d👦','🇫🇷🇺🇸🇩','♥\ufe0e','♥\ufe0f','\r\n','a\u200bb','\x00\ttext\n']
    texts=list(dict.fromkeys(official+extras))
    gen.publish(evidence/'texts.json',(json.dumps(texts,ensure_ascii=False)+'\n').encode())
    gen.publish(evidence/'go.mod',('module owned-terminal-oracle\n\ngo 1.23\nrequire github.com/rivo/uniseg v0.4.7\nreplace github.com/rivo/uniseg => '+json.dumps(str(source))+'\n').encode())
    gen.publish(evidence/'main.go',GO_MAIN.encode())
    env={**os.environ,'GOPROXY':'off','GOSUMDB':'off','GOTOOLCHAIN':'local','GOENV':'off','GOFLAGS':'','GOWORK':'off','CGO_ENABLED':'0'}
    version=subprocess.run(['go','version'],capture_output=True,text=True,check=True).stdout.strip()
    command={'argv':['go','run','-mod=mod','.'],'cwd':str(evidence),'global_watchdog':None,'go_version':version,'source_hashes':EXPECTED,'started':time.time()}
    command['go_environment']={key:env[key] for key in ['GOPROXY','GOSUMDB','GOTOOLCHAIN','GOENV','GOFLAGS','GOWORK','CGO_ENABLED']}
    gen.publish(evidence/'command.json',(json.dumps(command,indent=2)+'\n').encode())
    with (evidence/'texts.json').open('rb') as inp,(evidence/'stdout.json').open('xb') as stdout,(evidence/'stderr.txt').open('xb') as stderr:
        run=subprocess.run(command['argv'],cwd=evidence,env=env,stdin=inp,stdout=stdout,stderr=stderr)
    gen.publish(evidence/'execution.json',(json.dumps({'exit':run.returncode,'finished':time.time()})+'\n').encode())
    if run.returncode:raise RuntimeError('Go oracle failed; original channels retained')
    value=json.loads((evidence/'stdout.json').read_bytes())
    if len(value['items'])!=len(texts) or value['scalar_count']!=0x110000-0x800:raise ValueError('oracle coverage incomplete')
    value.update(schema_version=1,case_count=len(texts),official_source_case_count=len(official),source_hashes=EXPECTED,go_version=version)
    gen.publish(output,(json.dumps(value,ensure_ascii=False,indent=2)+'\n').encode())
    print('PASS Go oracle',len(texts),'cases',value['scalar_count'],'Unicode scalars',value['unicode_version'])
if __name__=='__main__':main()
