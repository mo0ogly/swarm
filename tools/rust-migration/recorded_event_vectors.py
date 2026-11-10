"""Actual SQL event representation and raw-output comparisons with valid witnesses."""
import copy
import math
import random
import struct

def vectors():
    result=[]
    def cell(kind,data=b''):
        return dict(Kind=kind,Data=data.hex() if isinstance(data,bytes) else str(data))
    valid=[cell('text',b'e'),cell('text',b'w'),cell('integer',1),cell('text',b'change'),cell('text',b'now'),cell('blob',b'{ "valid":true }')]
    def add(name,rows,work='w',control=False):
        for html in [False,True]:
            if not control:result.append(dict(Name=name+'-control-'+str(html),Work='w',Rows=[valid],EscapeHTML=html,control=True))
            result.append(dict(Name=name+'-'+str(html),Work=work,Rows=rows,EscapeHTML=html,control=control))
    add('valid',[valid],control=True);add('empty',[])
    second=copy.deepcopy(valid);second[2]=cell('integer',-1);add('ordered',[valid,second],control=True)
    for col in range(6):
        for kind,values in [('null',[b'']),('text',[b'',b'{}',b'+01',b'9223372036854775808',b'\xff',b'\xe2\x82',b'"\\\t\n\x00<>&\xe2\x80\xa8\xe2\x80\xa9']),('blob',[b'',b'{}',b'0001',b'\xff']),('integer',[0,-1,9223372036854775807]),('real',['1','1.5','999999','1000000','0.00001','NaN','Inf','-Inf'])]:
            for value in values:
                row=copy.deepcopy(valid);row[col]=cell(kind,value);add(f'column-{col}-{kind}-{len(result)}',[valid,row])
    for raw in [b'',b'null',b'{}',b'{',b'{}junk',b'\xff',b'"\xff"',b'"\xe2\x82"',b'{ "x":1e999,"x":-0,"s":"\\ud800\\udfff<>&\xe2\x80\xa8" }',b'[true,false,null,1e-99999]',b'"a\\\"  b"',b'\v{}',b'\"\\q\"']:
        row=copy.deepcopy(valid);row[5]=cell('blob',raw);add('payload-'+str(len(result)),[row])
    for depth in [127,128,9999,10000,10001]:
        row=copy.deepcopy(valid);row[5]=cell('blob',b'['*depth+b'0'+b']'*depth);add('depth-'+str(depth),[row])
    for work in ['missing',"w' OR 1=1",'', 'é']:add('filter-'+str(len(result)),[valid],work)
    randomizer=random.Random(20261018)
    floats=[0.,-0.,1e-4,1e-5,1e6,1e5,5e-324,1.7976931348623157e308]
    floats.extend(struct.unpack('!d',randomizer.getrandbits(64).to_bytes(8,'big'))[0]for _ in range(512))
    for n in floats:
        if not math.isfinite(n):continue
        row=copy.deepcopy(valid);row[0]=cell('real',repr(n));row[3]=cell('real',repr(-n));row[4]=cell('real',repr(n));add('float-'+str(len(result)),[row],control=True)
    return result
