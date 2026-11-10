"""Exhaustive two-byte output boundary; deterministic ID/UTC representation."""
import hashlib
def vectors():
    result=[]
    for first in range(256):
        for second in range(256):
            frame=dict(Name=f'text-{first:02x}{second:02x}',Mode='text',Bytes=bytes([first,second]).hex(),Prefix='',Seconds=0,Nanos=0,Root='');result.append(frame)
    for i,raw in enumerate([b'\xe2\x82',b'\xf0\x9f\x8c',b'"\\\x00\n',*[(s+'value'+s).encode()for s in ['\u0085','\u00a0','\u1680','\u2000','\u2028','\u2029','\u202f','\u205f','\u3000','\ufeff','\u200b']],*[s.encode()for s in ['é漢字🌍','<>&','\ud7ff','\ue000','\U0010ffff']]]):
        frame=dict(Name='text-long-'+str(i),Mode='text',Bytes=raw.hex(),Prefix='',Seconds=0,Nanos=0,Root='');result.append(frame)
    for i in range(96):
        frame=dict(Name='id-'+str(i),Mode='id',Bytes=hashlib.sha256(('id-fixture-'+str(i)).encode()).digest()[:12].hex(),Prefix=['w-','a-','','é <&>','x'*101][i%5],Seconds=0,Nanos=0,Root='');result.append(frame)
    for seconds in [-30_000_000_000_000,-62198755200,-62167219200,-2208988800,-1,0,1,951782400,1700000000,253402300799,253402300800,30_000_000_000_000]:
        for nanos in [-9223372036854775808,-1_200_000_000,-1,0,1,100,123456789,1_000_000_000,9223372036854775807]:
            frame=dict(Name='clock-'+str(seconds)+'-'+str(nanos),Mode='clock',Bytes='',Prefix='',Seconds=seconds,Nanos=nanos,Root='');result.append(frame)
    assert len(result)==len({v['Name']for v in result});return result
