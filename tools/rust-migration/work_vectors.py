"""Finite discriminating corpus from every field in the actual Go DTO closure."""
import json
import math
import struct


def vectors(layout):
    out=[]
    def add(name,model,raw,control=None):
        if control is not None:out.append(dict(name=name+'-valid-control',model=model,raw=control.hex(),control=True))
        out.append(dict(name=name,model=model,raw=raw.hex(),control=False))
    def encode(v):return json.dumps(v,ensure_ascii=True,separators=(',',':')).encode()
    def valid(t):
        if t.startswith('*'):return valid(t[1:])
        if t.startswith('[]'):return [valid(t[2:])]
        if t.startswith('map[string]'):return {'key':valid(t[11:])}
        if t=='string':return 'value'
        if t in ['int','int64']:return 7
        if t=='float64':return 1.25
        if t=='bool':return True
        if t=='json.RawMessage':return {'opaque':1}
        assert t in layout['types'];return {}
    choices=[None,True,False,0,1,-1,1.5,1e100,[],{},['x'],[0],[None],[{}],{'x':1},'text','',9223372036854775807,-9223372036854775808,9223372036854775808]
    chars=[0,1,8,9,10,13,31,32,34,92,127,255,0xd800,0xdfff,0x2028,0x2029]
    for model,definition in sorted(layout['types'].items()):
        control=b'{}'
        for i,raw in enumerate([b'{}',b'null',b'[]',b'1',b'"text"',b'',b'{}{}',b'{"ignored":{"x":1e9999}}',b'{"ignored":"\xff"}']):add(model+'-root-'+str(i),model,raw,control)
        for field in definition['fields']:
            key=field['json_tag'].split(',')[0];t=field['go_type'];control=encode({key:valid(t)});prefix=model+'-'+key
            for i,value in enumerate(choices):add(prefix+'-type-'+str(i),model,encode({key:value}),control)
            for spelling in [key.upper(),key.replace('s','ſ').replace('k','K')]:
                if spelling!=key:add(prefix+'-fold-'+spelling,model,encode({spelling:valid(t)}),control)
            for null in [b'null',b'[]',b'{}']:
                raw=b'{'+encode(key)+b':'+encode(valid(t))+b','+encode(key)+b':'+null+b'}';add(prefix+'-duplicate-'+null.decode(),model,raw,control)
            if t.lstrip('*')=='string':
                for c in chars:add(prefix+'-char-'+str(c),model,encode({key:'a'+chr(c)+'z'}),control)
            if t.lstrip('*')in ['int','int64']:
                for token in ['-0','-0.0','-0e0','1e0','9223372036854775808','-9223372036854775809']:
                    add(prefix+'-token-'+token,model,b'{'+encode(key)+b':'+token.encode()+b'}',control)
            if t in ['*float64','map[string]float64']:
                tokens=['-0','0','-0.0','1e-7','1e-6','1e20','1e21','1e22','-1e-7','1.2345678901234567','1e-9999','-1e-9999','1e9999','-1e9999','5e-324','1.7976931348623157e308']
                seed=0xA9631FBBCA73D91
                for _ in range(512):
                    seed=(seed+0x9E3779B97F4A7C15)&((1<<64)-1);v=struct.unpack('>d',seed.to_bytes(8,'big'))[0]
                    if math.isfinite(v):tokens.append(repr(v))
                for i,token in enumerate(tokens):
                    value=token.encode()
                    if t.startswith('map'):value=b'{"score":'+value+b'}'
                    add(prefix+'-float-'+str(i),model,b'{'+encode(key)+b':'+value+b'}',control)
            if t=='json.RawMessage':
                for i,value in enumerate([b'{ "x":1, "x":2 }',b'1e9999',b'"\\ud800<>&\\u2028"',b'[null,{},[]]',b'null',b'false']):add(prefix+'-raw-'+str(i),model,b'{'+encode(key)+b':'+value+b'}',control)
    for i,raw in enumerate([
        b'{"criteria":["old","keep"],"criteria":["new"],"criteria":[null,null]}',
        b'{"instruction_base":"old","instruction_base":null}',
        b'{"instruction_base":"old","instruction_base":null,"instruction_base":""}',
        b'{"validation_policy":{"controls":[{"id":"old"}]},"validation_policy":{"controls":[{}]}}',
        b'{"memory_selections":[{"version":4,"id":"old"}],"memory_selections":[{"id":"new","version":null}]}',
        b'{"plan_checks":{"one":"old"},"plan_checks":{"one":null,"two":"new"}}',
        b'{"gate":{"document":{"x":1}},"gate":{"document":null}}',
    ]):add('Task-specific-'+str(i),'Task',raw,b'{}')
    assert len({v['name']for v in out})==len(out)
    return out
