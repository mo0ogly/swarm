#!/usr/bin/env python3
"""Isolated Linux channel experiment; not a Swarm launcher or authority source."""
import argparse
import array
import hashlib
import json
import os
from pathlib import Path
import select
import shutil
import socket
import struct
import subprocess
import tempfile
import time


WORKER = r'''
import array,json,os,socket,struct,sys
identity=sys.argv[1]
s=socket.socket(socket.AF_UNIX,socket.SOCK_SEQPACKET)
s.connect('/broker.sock')
s.recv(4096)
def send(value):
    s.send(json.dumps(value).encode())
    s.recv(4096)
send({'claimed_instance':identity,'operation':'hello','inside_pid':os.getpid(),
      'namespaces':{n:os.readlink('/proc/self/ns/'+n) for n in ('pid','mnt','user')}})
while True:
    raw,anc,flags,_=s.recvmsg(4096,socket.CMSG_SPACE(3*array.array('i').itemsize))
    if not raw: break
    cmd=json.loads(raw)
    if cmd['command']=='quit':break
    if cmd['command']=='send':send({'claimed_instance':cmd['claim'],'operation':'send'})
    elif cmd['command']=='child':
        pid=os.fork()
        if pid==0:
            send({'claimed_instance':identity,'operation':'child'})
            os._exit(0)
        os.waitpid(pid,0)
        send({'claimed_instance':identity,'operation':'child-complete'})
    elif cmd['command']=='transfer':
        fds=array.array('i',[s.fileno()])
        s.sendmsg([json.dumps({'claimed_instance':identity,'operation':'transfer'}).encode()],
                  [(socket.SOL_SOCKET,socket.SCM_RIGHTS,fds)])
        sys.stdin.readline() # test harness holds this reader while the FD is copied
    elif cmd['command']=='borrow':
        descriptors=array.array('i')
        for level,kind,data in anc:
            if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:descriptors.frombytes(data)
        if len(descriptors)!=1:raise RuntimeError('exactly one borrowed endpoint required')
        borrowed=socket.socket(fileno=descriptors[0])
        borrowed.send(json.dumps({'claimed_instance':cmd['claim'],'operation':'borrowed'}).encode())
        borrowed.recv(4096)
        borrowed.close()
        send({'claimed_instance':identity,'operation':'borrow-complete'})
    elif cmd['command']=='join':
        descriptors=array.array('i')
        for level,kind,data in anc:
            if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:descriptors.frombytes(data)
        if len(descriptors)!=3:raise RuntimeError('three namespace descriptors required')
        attempts=[]
        for fd in descriptors:
            try:
                os.setns(fd,0)
                attempts.append({'joined':True})
            except OSError as exc:
                attempts.append({'joined':False,'errno':exc.errno})
            finally:os.close(fd)
        send({'claimed_instance':identity,'operation':'join-result','attempts':attempts})
    elif cmd['command']=='spoof':
        blocked=False
        error=None
        try:
            s.sendmsg([json.dumps({'claimed_instance':cmd['claim'],'operation':'spoofed'}).encode()],
                      [(socket.SOL_SOCKET,socket.SCM_CREDENTIALS,struct.pack('iII',cmd['pid'],os.getuid(),os.getgid()))])
        except OSError as exc:
            blocked=True
            error=exc.errno
        send({'claimed_instance':identity,'operation':'spoof-result','blocked':blocked,'errno':error})
s.close()
'''


class Experiment:
    def __init__(self, output):
        self.output = output
        self.root = Path(tempfile.mkdtemp(prefix='swarm-ia2-runtime-'))
        self.worker = self.root / 'worker.py'
        self.worker.write_text(WORKER)
        self.listener = socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET)
        self.listener.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        self.listener.settimeout(4)
        self.address = self.root / 'broker.sock'
        self.listener.bind(str(self.address))
        self.listener.listen(4)
        self.cages = {}
        self.records = []
        self.checks = []
        self.cleanup = []
        self.extra_fds = []

    def persist(self):
        (self.output / 'observations.json').write_text(json.dumps({
            'root': str(self.root), 'cages': [{k: v for k, v in c.items() if k not in ('process', 'connection', 'fds')} for c in self.cages.values()],
            'messages': self.records, 'checks': self.checks, 'cleanup': self.cleanup,
        }, indent=2) + '\n')

    def launch(self, identity):
        info_read, info_write = os.pipe()
        gate_read, gate_write = os.pipe()
        command = [shutil.which('bwrap'), '--unshare-user', '--unshare-pid', '--unshare-net',
                   '--unshare-ipc', '--unshare-uts', '--die-with-parent', '--new-session',
                   '--cap-drop', 'ALL', '--ro-bind', '/usr', '/usr']
        for name in ('bin', 'sbin', 'lib', 'lib64'):
            path = Path('/') / name
            if path.is_symlink():
                command += ['--symlink', os.readlink(path), '/' + name]
        command += ['--proc', '/proc', '--dev', '/dev', '--tmpfs', '/tmp',
                    '--ro-bind', str(self.worker), '/worker.py',
                    '--ro-bind', str(self.address), '/broker.sock',
                    '--clearenv', '--setenv', 'PATH', '/usr/bin',
                    '--info-fd', str(info_write), '--block-fd', str(gate_read),
                    '--', '/usr/bin/python3', '/worker.py', identity]
        process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                   stderr=subprocess.PIPE, text=True, pass_fds=(info_write, gate_read))
        os.close(info_write)
        os.close(gate_read)
        cage = {'id': identity, 'process': process, 'command': command, 'active': True, 'fds': {}, 'inodes': {}}
        self.cages[identity] = cage
        try:
            if not select.select([info_read], [], [], 4)[0]:
                raise RuntimeError('bootstrap information missing before execution gate')
            info = json.loads(os.read(info_read, 65536))
            cage['bootstrap'] = info
            for name in ('pid', 'mnt'):
                fd = os.open('/proc/' + str(info['child-pid']) + '/ns/' + name, os.O_RDONLY)
                cage['fds'][name] = fd  # pin namespace objects through the whole experiment
                cage['inodes'][name] = os.fstat(fd).st_ino
                assert cage['inodes'][name] == info[name + '-namespace']
            os.write(gate_write, b'1')
        finally:
            os.close(info_read)
            os.close(gate_write)
        connection, _ = self.listener.accept()
        connection.setsockopt(socket.SOL_SOCKET, socket.SO_PASSCRED, 1)
        connection.settimeout(4)
        cage['connection'] = connection
        connection.send(b'READY')
        row, _ = self.receive(identity)
        self.check('legitimate-' + identity, row['accepted'] and row['actual_instance'] == identity)
        cage['worker_namespaces'] = row['payload']['namespaces']
        self.persist()

    def instance_for_pid(self, pid):
        opened = []
        try:
            actual = {}
            for name in ('pid', 'mnt'):
                fd = os.open('/proc/' + str(pid) + '/ns/' + name, os.O_RDONLY)
                opened.append(fd)
                actual[name] = os.fstat(fd).st_ino
            matches = [key for key, cage in self.cages.items() if cage['inodes'] == actual]
            return matches[0] if len(matches) == 1 else None, actual
        except OSError as error:
            return None, {'unavailable': str(error)}
        finally:
            for fd in opened:
                os.close(fd)

    def receive(self, channel):
        connection = self.cages[channel]['connection']
        peer_pid, peer_uid, peer_gid = struct.unpack('iII', connection.getsockopt(socket.SOL_SOCKET, socket.SO_PEERCRED, 12))
        raw, ancillary, flags, _ = connection.recvmsg(4096, socket.CMSG_SPACE(12) + socket.CMSG_SPACE(4))
        if flags & (socket.MSG_CTRUNC | socket.MSG_TRUNC):
            raise RuntimeError('truncated credentials or payload')
        credentials = []
        descriptors = array.array('i')
        for level, kind, data in ancillary:
            if level == socket.SOL_SOCKET and kind == socket.SCM_CREDENTIALS:
                credentials.append(struct.unpack('iII', data))
            if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
                descriptors.frombytes(data)
        if len(credentials) != 1:
            raise RuntimeError('exactly one kernel credential required')
        self.extra_fds.extend(descriptors)
        pid, uid, gid = credentials[0]
        actual, namespaces = self.instance_for_pid(pid)
        peer_instance, _ = self.instance_for_pid(peer_pid)
        payload = json.loads(raw)
        accepted = actual is not None and uid == os.getuid() and self.cages[actual]['active'] and payload.get('claimed_instance') == actual
        row = {'channel': channel, 'peer_pid': peer_pid, 'peer_uid': peer_uid, 'peer_instance': peer_instance,
               'message_pid': pid, 'message_uid': uid, 'actual_instance': actual, 'namespace_inodes': namespaces,
               'accepted': accepted, 'payload': payload}
        self.records.append(row)
        if payload.get('operation') != 'transfer':
            connection.send(json.dumps({'accepted': accepted}).encode())
        self.persist()
        return row, descriptors

    def command(self, identity, command, descriptor=None):
        raw = json.dumps(command).encode()
        connection = self.cages[identity]['connection']
        if descriptor is None:
            connection.send(raw)
        else:
            descriptors = descriptor if isinstance(descriptor, list) else [descriptor]
            connection.sendmsg([raw], [(socket.SOL_SOCKET, socket.SCM_RIGHTS, array.array('i', descriptors))])

    def check(self, name, passed):
        self.checks.append({'name': name, 'result': 'PASS' if passed else 'FAIL'})
        self.persist()
        if not passed:
            raise AssertionError(name)

    def run(self):
        self.launch('A')
        self.launch('B')
        self.check('distinct-pinned-namespaces', self.cages['A']['inodes'] != self.cages['B']['inodes'])
        self.command('A', {'command': 'send', 'claim': 'B'})
        row, _ = self.receive('A')
        self.check('forged-instance-label-denied', not row['accepted'] and row['actual_instance'] == 'A')
        self.command('A', {'command': 'child'})
        row, _ = self.receive('A')
        self.check('authorized-tool-child-attributed', row['accepted'] and row['actual_instance'] == 'A' and row['message_pid'] != row['peer_pid'])
        complete, _ = self.receive('A')
        self.check('tool-child-completed', complete['accepted'] and complete['payload']['operation'] == 'child-complete')
        self.command('B', {'command': 'transfer'})
        transfer, descriptors = self.receive('B')
        self.check('endpoint-copy-fixture', transfer['accepted'] and len(descriptors) == 1)
        borrowed = socket.socket(fileno=os.dup(descriptors[0]))
        try:
            borrowed.send(json.dumps({'claimed_instance': 'B', 'operation': 'host-borrowed'}).encode())
            row, _ = self.receive('B')
            borrowed.recv(4096)
            self.check('copied-endpoint-host-denied', not row['accepted'] and row['actual_instance'] is None and row['peer_instance'] == 'B')
        finally:
            borrowed.close()
        self.command('A', {'command': 'borrow', 'claim': 'B'}, descriptors[0])
        row, _ = self.receive('B')
        self.check('copied-endpoint-sibling-denied', not row['accepted'] and row['actual_instance'] == 'A' and row['peer_instance'] == 'B')
        complete, _ = self.receive('A')
        self.check('sibling-copy-completed', complete['accepted'] and complete['payload']['operation'] == 'borrow-complete')
        self.cages['B']['process'].stdin.write('continue\n')
        self.cages['B']['process'].stdin.flush()
        self.command('A', {'command': 'spoof', 'claim': 'B', 'pid': transfer['message_pid']})
        row, _ = self.receive('A')
        self.check('kernel-pid-forgery-denied', row['accepted'] and row['payload'].get('blocked') is True and row['payload'].get('errno') in (1, 3))
        user_fd = os.open('/proc/' + str(transfer['message_pid']) + '/ns/user', os.O_RDONLY)
        self.extra_fds.append(user_fd)
        self.command('A', {'command': 'join'}, [user_fd, self.cages['B']['fds']['pid'], self.cages['B']['fds']['mnt']])
        row, _ = self.receive('A')
        attempts = row['payload'].get('attempts', [])
        self.check('copied-namespace-descriptors-cannot-join-sibling', row['accepted'] and len(attempts) == 3 and all(not a.get('joined') and a.get('errno') == 1 for a in attempts))
        self.cages['A']['active'] = False
        self.command('A', {'command': 'send', 'claim': 'A'})
        row, _ = self.receive('A')
        self.check('revoked-instance-channel-denied', not row['accepted'] and row['actual_instance'] == 'A')

    def close(self):
        for identity, cage in self.cages.items():
            process = cage['process']
            try:
                if process.poll() is None and 'connection' in cage:
                    self.command(identity, {'command': 'quit'})
                stdout, stderr = process.communicate('continue\n', timeout=4)
            except Exception as error:
                process.terminate()
                try:
                    stdout, stderr = process.communicate(timeout=3)
                except subprocess.TimeoutExpired:
                    process.kill()
                    stdout, stderr = process.communicate(timeout=3)
                self.cleanup.append({'id': identity, 'forced_cleanup': str(error)})
            self.cleanup.append({'id': identity, 'exit_code': process.returncode, 'stdout': stdout, 'stderr': stderr})
            if 'connection' in cage:
                cage['connection'].close()
            for fd in cage['fds'].values():
                os.close(fd)
        for fd in self.extra_fds:
            os.close(fd)
        self.listener.close()
        self.persist()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    experiment = Experiment(args.output)
    result = {'result': 'FAIL', 'model_calls': 0, 'script_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest()}
    try:
        experiment.run()
        result['result'] = 'PASS_ISOLATED_CHANNEL_EXPERIMENT'
    except Exception as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    finally:
        experiment.close()
    result['checks'] = experiment.checks
    result['cleanup'] = experiment.cleanup
    if any(row.get('exit_code', 0) != 0 or 'forced_cleanup' in row for row in experiment.cleanup):
        result['result'] = 'FAIL'
    result['limits'] = ['Standalone Linux experiment, not an engine launcher or protected authority producer',
                        'Only PID and mount namespaces pinned; user namespace changes during bwrap startup',
                        'No real provider, egress, persistent revocation, human approval, restart or effect reconciliation',
                        'No guarantee against a host administrator or an unconfined compromise of the controller account',
                        'SO_PASSCRED message attribution is distinct from connection SO_PEERCRED; descriptors are intentionally transferred by the fixture']
    (args.output / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    shutil.copyfile(__file__, args.output / 'recipe.py')
    print(json.dumps({'result': result['result'], 'checks': len(experiment.checks), 'failure': result.get('failure'), 'cleanup': experiment.cleanup}))
    return 0 if result['result'] == 'PASS_ISOLATED_CHANNEL_EXPERIMENT' else 1


if __name__ == '__main__':
    raise SystemExit(main())
