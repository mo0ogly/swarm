"""Contract-based typed request corpus and regression probes. No execution rights."""
import copy
import itertools
import json
from bootstrap_scenarios import canonical

def vectors(layout):
 result=[]
 request=dict(schema_version=1,event_id='event',expected_revision=0)
 config=dict(schema_version=1,event_id='event',expected_head=dict(revision='0',digest=''),values=dict(mode='local',local_socket_path='/run/swarm-fixture/authority.sock'),reason='explicit fixture choice')
 reply=dict(schema_version=1,head=dict(sequence=1,digest='a'*64),record=dict(id='active',body='e30=',enrolled_sequence=1))
 def add(name,mode,raw,control=None):
  if control is not None:result.append(dict(name='control-'+name,mode=mode,raw=canonical(control).hex()))
  result.append(dict(name=name,mode=mode,raw=raw.hex()))
 def payload(name,obj):
  for mode in ['request','external']:add(mode+'-'+name,mode,canonical(obj),request)
 for root in [b'{}',b'null',b'[]',b'0',b'"text"',b'',b'{}{}',b'{}\n',b'{"schema_version":-0}']:
  for mode in ['request','external']:add(mode+'-root-'+root.hex(),mode,root,request)
 def scope(name,field,value):
  d=copy.deepcopy(request)
  if name=='Request':d[field]=value
  elif name=='ValidationPolicy':d['validation_policy']={field:value}
  else:d['validation_policy']=dict(controls=[{field:value}])
  return d
 kinds=[None,False,True,0,1,-1,1.0,[],{},[None],['x'],[1],[False],9223372036854775807,-9223372036854775808,9223372036854775808,'', 'value']
 for owner,fields in layout['types'].items():
  for field in fields:
   key=field['json_tag'].split(',')[0]
   for i,value in enumerate(kinds):payload(owner+'-'+key+'-type-'+str(i),scope(owner,key,value))
   if field['go_type']=='string':
    for code in range(256):payload(owner+'-'+key+'-char-'+str(code),scope(owner,key,'x'+chr(code)+'x'))
    for text in ['<>&\u2028\u2029','surrogate-\ufffd',' ', '\u00a0value\u00a0']:
     payload(owner+'-'+key+'-unicode-'+text.encode().hex(),scope(owner,key,text))
   if field['go_type']=='int':
    for token in ['-0','-0.0','-0e0','0e0','1e0','9223372036854775807','-9223372036854775808','9223372036854775808','-9223372036854775809']:
     d=scope(owner,key,'TOKEN');raw=canonical(d).replace(b'"TOKEN"',token.encode())
     for mode in ['request','external']:add(mode+'-'+owner+'-'+key+'-token-'+token,mode,raw,request)
   d=scope(owner,key,kinds[-1]if field['go_type']=='string'else None);raw=canonical(d)
   for spelling in [key.upper(),key.replace('s','ſ'),key.replace('k','K')]:
    if spelling==key:continue
    for mode in ['request','external']:add(mode+'-'+owner+'-'+key+'-fold-'+spelling,mode,raw.replace(('"'+key+'"').encode(),('"'+spelling+'"').encode()),request)
 for raw in [
  b'{"schema_version":1,"origin":"conducteur"}',
  b'{"schema_version":1,"title":"first","title":null}',
  b'{"schema_version":1,"title":false,"title":"last"}',
  b'{"schema_version":1,"validation_policy":{"mode":"first"},"validation_policy":{"actor":"last"}}',
  b'{"schema_version":1,"validation_policy":{"mode":"first"},"validation_policy":null}',
  b'{"schema_version":1,"criteria":["first"],"criteria":[null,"last"]}',
  b'{"schema_version":1,"requirement_prerequisites":{"a":["old"],"b":null},"requirement_prerequisites":{"a":[],"c":[null]}}',
  b'{"schema_version":1,"depends":[],"origin":"conducteur","status":"todo"}',
  b'{"schema_version":1,"criteria":["first","old"],"criteria":["new"],"criteria":[null,null]}',
  b'{"schema_version":1,"criteria":["first"],"criteria":[],"criteria":[null]}',
  b'{"schema_version":1,"criteria":["first"],"criteria":null,"criteria":[null]}',
  b'{"schema_version":1,"validation_policy":{"controls":[{"id":"old","command":["echo"]}]},"validation_policy":{"controls":[{"criteria":[1]}]}}',
  b'{"schema_version":1,"requirement_prerequisites":{"a":["old"]},"requirement_prerequisites":{"a":[null]}}',
  b'{"schema_version":1,"unknown":true}', b'{"schema_version":1,"validation_policy":{"unknown":true}}',
  b'{"schema_version":1,"validation_policy":{"controls":[{"unknown":true}]}}',
  b'{"schema_version":1,"validation_policy":{"controls":[null]}}',
 ]:
  for mode in ['request','external']:add(mode+'-special-'+raw.hex(),mode,raw,request)
 for data in [b'\xff',b'\xed\xa0\x80',b'\\ud800',b'\\udfff']:
  raw=b'{"schema_version":1,"title":"'+data+b'"}'
  for mode in ['request','external']:add(mode+'-lossy-'+data.hex(),mode,raw,request)
 for mode,base in [('config-change',config),('reply',reply)]:
  add(mode+'-valid',mode,canonical(base))
  for token in ['-0','0','1','2','-0.0','-0e0','false','null','9223372036854775808']:
   raw=canonical(base).replace(b'"schema_version":1',b'"schema_version":'+token.encode(),1);add(mode+'-schema-'+token,mode,raw,base)
   raw=canonical(base).replace(b'"schema_version":1',b'"schema_version":-0,"schema_version":2',1);add(mode+'-duplicate-future-'+token,mode,raw,base)
  raw=canonical(base).replace(b'"schema_version":1',b'"schema_version":2',1)
  if mode=='reply':raw=raw.replace(b'"sequence":1',b'"sequence":-0',1)
  else:raw=raw.replace(b'"event_id":"event"',b'"event_id":null',1)
  add(mode+'-future-nested-negative-zero',mode,raw,base)
 for depth in [1,9999,10000]:
  for mode in ['request','external']:add(mode+'-unknown-depth-'+str(depth),mode,b'{"schema_version":1,"unknown":'+b'['*depth+b'0'+b']'*depth+b'}',request)
 assert len({v['name']for v in result})==len(result),'duplicate case name'
 return result
