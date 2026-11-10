import ctypes,datetime,errno,hashlib,json,os,platform,socket,subprocess,sys,tempfile
from pathlib import Path
if platform.machine()!='x86_64':raise SystemExit('probe only supports its verified x86_64 syscall ABI')
lib=ctypes.CDLL(None,use_errno=True);lib.syscall.restype=ctypes.c_long
class Ruleset(ctypes.Structure):_fields_=[('fs',ctypes.c_uint64),('net',ctypes.c_uint64),('scoped',ctypes.c_uint64)]
class Beneath(ctypes.Structure):_pack_=1;_fields_=[('access',ctypes.c_uint64),('fd',ctypes.c_int32)]
def checked(number,*args):
 v=lib.syscall(ctypes.c_long(number),*args)
 if v<0:raise OSError(ctypes.get_errno(),'landlock probe syscall failed')
 return v
abi=checked(444,ctypes.c_void_p(),ctypes.c_size_t(0),ctypes.c_uint(1));assert abi>=6
root=Path(tempfile.mkdtemp(prefix='swarm-landlock-fixture-'));work=root/'work';private=root/'private';work.mkdir();private.mkdir();(work/'allowed').write_text('allowed');(private/'secret').write_text('isolated-secret-sentinel');(work/'linked-secret').symlink_to(private/'secret')
listener=socket.socket();listener.bind(('127.0.0.1',0));listener.listen();port=listener.getsockname()[1];reader,writer=os.pipe();pid=os.fork()
if pid==0:
 os.close(reader);listener.close();result={}
 try:
  attrs=Ruleset(0xffff,3,3);fd=checked(444,ctypes.byref(attrs),ctypes.c_size_t(ctypes.sizeof(attrs)),ctypes.c_uint(0));pathfd=os.open(work,os.O_PATH|os.O_CLOEXEC);rule=Beneath(0xffff & ~1,pathfd);checked(445,ctypes.c_int(fd),ctypes.c_int(1),ctypes.byref(rule),ctypes.c_uint(0));os.close(pathfd)
  assert lib.prctl(38,1,0,0,0)==0;checked(446,ctypes.c_int(fd),ctypes.c_uint(0));os.close(fd)
  result['allowed_read']=(work/'allowed').read_text()=='allowed';(work/'output').write_text('owned output');result['allowed_write']=True
  for name,target in [('outside',private/'secret'),('traversal',work/'../private/secret'),('symlink',work/'linked-secret')]:
   try:target.read_text();result[name+'_denied']=False
   except OSError as e:result[name+'_denied']=e.errno in [errno.EACCES,errno.EPERM]
  try:subprocess.run(['/bin/cat',str(private/'secret')],stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True);result['direct_binary_denied']=False
  except OSError as e:result['direct_binary_denied']=e.errno in [errno.EACCES,errno.EPERM]
  sock=socket.socket();sock.settimeout(1)
  try:sock.connect(('127.0.0.1',port));result['tcp_denied']=False
  except OSError as e:result['tcp_denied']=e.errno in [errno.EACCES,errno.EPERM]
  finally:sock.close()
 except Exception as e:result['failure_type']=type(e).__name__
 os.write(writer,json.dumps(result).encode());os.close(writer);os._exit(0)
os.close(writer);raw=os.read(reader,16384);os.close(reader);_,status=os.waitpid(pid,0);listener.close();result=json.loads(raw);assert os.waitstatus_to_exitcode(status)==0 and len(result)==7 and all(result.values()),result
result.update({'abi':abi,'fixture_root':str(root),'kernel':platform.release(),'at':datetime.datetime.now(datetime.timezone.utc).isoformat(),'scope':'isolated OS feasibility probe, not Swarm integration','limits':['TCP only; UDP and inherited descriptors are not claimed controlled','No secret broker or provider launch integration implemented'],'source_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest()});out=Path(sys.argv[1]);out.write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
