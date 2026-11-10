#!/usr/bin/env python3
"""Observe actual owned provider fate after abrupt native supervisor death.

Linux subreaper/pidfd/socket credentials prove child ownership and reap it without
arbitrary test timeouts. Only inactive fixture Roots and inert Python providers
are used. This is leader lifetime, not descendant confinement or crash settlement.
"""
import argparse,ctypes,json,os,selectors,signal,socket,struct,subprocess,time,traceback
from pathlib import Path
import rust_agent_model_route as fixture

PROGRAM=fixture.PROGRAM+r'''
import ctypes,socket
libc=ctypes.CDLL(None,use_errno=True)
def observation():
    value=ctypes.c_int()
    if libc.prctl(2,ctypes.byref(value),0,0,0)!=0:raise OSError(ctypes.get_errno(),'fixture PR_GET_PDEATHSIG')
    return dict(pid=os.getpid(),ppid=os.getppid(),parent_death_signal=value.value)
with socket.socket(socket.AF_UNIX,socket.SOCK_STREAM) as control:
    control.connect('parent-death.sock')
    control.sendall((json.dumps(observation())+'\n').encode())
    while True:
        command=control.recv(1)
        if command==b'Q':control.sendall((json.dumps(observation())+'\n').encode())
        else:break
'''


def subreaper(value=None):
    """Read/set this test process's Linux subreaper flag, checking syscall errors.

    PR_GET/SET_CHILD_SUBREAPER37/36 are Linux ABI, not an execution policy. No
    unrelated process is signalled; caller restores the original value after reap.
    """
    libc=ctypes.CDLL(None,use_errno=True);old=ctypes.c_int()
    if libc.prctl(37,ctypes.byref(old),0,0,0)!=0:raise OSError(ctypes.get_errno(),'PR_GET_CHILD_SUBREAPER')
    if value is not None and libc.prctl(36,int(value),0,0,0)!=0:raise OSError(ctypes.get_errno(),'PR_SET_CHILD_SUBREAPER')
    return old.value


def frame(connection):
    """Read one bounded-by-fixture complete control frame; EOF is explicit None.

    No temporal watchdog or provider authority is inferred from the socket text.
    """
    data=bytearray()
    while True:
        byte=connection.recv(1)
        if not byte:
            if not data:return None
            raise AssertionError('Partial fixture control frame')
        if byte==b'\n':return json.loads(data)
        data.extend(byte)


def run(binary,out,label,kill_owner):
    """Run healthy/owner-loss variants under identical persisted launch policy.

    Kernel peer credentials and stored child/supervisor IDs correlate before any
    owned signal. Reap/close errors join original failures. No private SQL write,
    consumption reset, acceptance, provider/model account or live Root is used.
    """
    owned=fixture.Fixture(binary,out,label);fixture.publish(owned.root/'codex',PROGRAM.encode());os.chmod(owned.root/'codex',0o700)
    listener=socket.socket(socket.AF_UNIX,socket.SOCK_STREAM);connection=None;selector=selectors.DefaultSelector();owner_fd=None;child_fd=None;process=None;child=None;child_reaped=False;owner_lost=False;primary=None;result=None
    stdout=(out/(label+'-run.stdout')).open('xb');stderr=(out/(label+'-run.stderr')).open('xb')
    try:
        listener.bind(str(owned.root/'parent-death.sock'));os.chmod(owned.root/'parent-death.sock',0o600);listener.listen(1);listener.setblocking(False);prepared=owned.prepare();money=owned.reservation()
        argv=[str(binary),'agent','run','session','--root',str(owned.root),'--json'];process=subprocess.Popen(argv,env=owned.environment,stdin=subprocess.DEVNULL,stdout=stdout,stderr=stderr);owner_fd=os.pidfd_open(process.pid);selector.register(owner_fd,selectors.EVENT_READ,'owner');selector.register(listener,selectors.EVENT_READ,'provider')
        fixture.publish(out/(label+'-owner.json'),dict(argv=argv,pid=process.pid,root=str(owned.root),global_watchdog=None,kill_owner=kill_owner))
        while connection is None:
            events=selector.select()
            if any(key.data=='owner' for key,_ in events):raise AssertionError('Owned CLI exited before fixture handshake')
            connection,_=listener.accept()
        selector.unregister(listener);peer,uid,_=struct.unpack('3i',connection.getsockopt(socket.SOL_SOCKET,socket.SO_PEERCRED,12));assert uid==os.getuid();first=frame(connection);assert first and first['pid']==peer and first['ppid']==process.pid;child=peer;child_fd=os.pidfd_open(child)
        agent=owned.ok(['agent','show','session'])['agent'];assert agent['child_pid']==peer and agent['supervisor_pid']==process.pid;assert json.loads((owned.root/'model-received.json').read_bytes())['stdin_sha256']==fixture.digest(prepared['agent']['prompt'].encode())
        if not kill_owner:
            connection.sendall(b'X');connection.close();connection=None;code=process.wait();stdout.flush();stderr.flush();assert code==0,(out/(label+'-run.stderr')).read_text();wire=json.loads((out/(label+'-run.stdout')).read_bytes());assert wire['task_accepted'] is False;result=dict(healthy_completed=True,first=first,provider_started=True)
        else:
            process.kill();assert process.wait()==-signal.SIGKILL;owner_lost=True
            # The owner is confirmed reaped before the query. A live reply proves
            # continuing provider execution, not merely a stale PID observation.
            transport=[]
            try:connection.sendall(b'Q')
            except BrokenPipeError as error:transport.append(dict(operation='query-write',errno=error.errno))
            try:reply=frame(connection)
            except ConnectionResetError as error:transport.append(dict(operation='query-read',errno=error.errno));reply=None
            # Transport disappearance alone is never PASS; waitpid below must
            # still prove this actual reparented child died with SIGKILL.
            survived=reply is not None
            if survived:assert reply['pid']==peer and reply['ppid']==os.getpid();connection.sendall(b'X')
            connection.close();connection=None
            waited,status=os.waitpid(child,0);assert waited==child;child_reaped=True
            assert owned.reservation()==money
            after=owned.ok(['agent','show','session'])['agent'];assert after['status'] in ('running','starting') and not after.get('native_execution_result') and not after.get('ended')
            result=dict(first=first,query_transport=transport,reply_after_owner_reaped=reply,survived_parent_death=survived,child_signal=os.WTERMSIG(status) if os.WIFSIGNALED(status) else None,child_exit=os.WEXITSTATUS(status) if os.WIFEXITED(status) else None,child_reaped=True,original_reservation_retained=True,crash_settlement_unproved=True)
        fixture.publish(out/(label+'-observation.json'),result)
    except BaseException as error:primary=error
    cleanup=[]
    for close in [lambda:connection.close() if connection is not None else None,listener.close,selector.close]:
        try:close()
        except BaseException as error:cleanup.append(error)
    if process is not None and process.poll() is None:
        try:
            if primary is not None:owned.ok(['agent','stop','session'])
            process.wait()
        except BaseException as error:cleanup.append(error)
    if child_fd is not None and owner_lost and not child_reaped:
        try:
            waited,status=os.waitpid(child,os.WNOHANG)
            absent_signal=None
            if waited==0:
                try:signal.pidfd_send_signal(child_fd,signal.SIGKILL)
                except ProcessLookupError as error:absent_signal=error.errno
                waited,status=os.waitpid(child,0)
            assert waited==child;child_reaped=True
            fixture.publish(out/(label+'-cleanup-reap.json'),dict(pid=child,status=status,signal_already_absent_errno=absent_signal,reaped=True))
        except BaseException as error:cleanup.append(error)
    for resource in [stdout,stderr]:
        for operation in [resource.flush,lambda resource=resource:os.fsync(resource.fileno()),resource.close]:
            try:operation()
            except BaseException as error:cleanup.append(error)
    for descriptor in [child_fd,owner_fd]:
        if descriptor is not None:
            try:os.close(descriptor)
            except BaseException as error:cleanup.append(error)
    if primary or cleanup:raise BaseExceptionGroup('Owned parent-death proof/cleanup failed',([primary] if primary else [])+cleanup)
    return result


def main():
    """Healthy completion precedes actual parent loss; retain original failures."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--binary',required=True,type=Path);p.add_argument('--output',required=True,type=Path);a=p.parse_args();binary=a.binary.resolve();out=a.output.resolve();out.mkdir(parents=True,exist_ok=False)
    fixture.publish(out/'inputs.json',dict(binary=str(binary),binary_sha256=fixture.digest(binary.read_bytes()),sources={str(x):fixture.digest(x.read_bytes()) for x in [Path(__file__),Path('tests/rust_agent_model_route.py')]}));old=subreaper();subreaper(True)
    try:
        run(binary,out,'healthy-parent',False);result=run(binary,out,'lost-parent',True);assert not result['survived_parent_death'] and result['child_signal']==signal.SIGKILL,result
        fixture.publish(out/'result.json',dict(status='PASS',scope='Actual owned provider leader stops on abrupt supervisor death; no descendant/isolation or crash-settlement claim',provider_account_used=False,observation=result))
    except BaseException:
        fixture.publish(out/'failure.json',dict(status='FAIL',cause=traceback.format_exc()));raise
    finally:subreaper(old)


if __name__=='__main__':main()
