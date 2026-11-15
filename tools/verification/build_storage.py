#!/usr/bin/env python3
"""Admit exact persisted external Cargo targets on an observed held Unix mount.

Internal targets remain the default. An external target requires registration;
no missing-mount fallback exists. Held directory FDs prevent ordinary unmount,
not privileged lazy/forced detach or hostile same-UID filesystem mutation. Checks
before/after execution detect identity/configuration drift; this is no sandbox.
"""
import argparse,contextlib,fcntl,hashlib,json,os,stat,subprocess,sys,tempfile
from pathlib import Path

if sys.version_info < (3,11):
    raise RuntimeError('External build admission requires Python 3.11 or later')

POLICY='.cargo/external-targets.json'
# Technical qualification scope: the ext4 hardlink/flock witness is established.
# Other filesystem implementations need their own qualification, not a new default.
QUALIFIED_FILESYSTEMS={'ext4'}


def require(value,reason):
    """Refuse explicitly, including under Python -O; no successful empty fallback."""
    if not value:raise ValueError(reason)


@contextlib.contextmanager
def _closing(close):
    """Preserve both the operation and final close failure, without retrying close."""
    try:yield
    finally:
        primary=sys.exception()
        try:close()
        except BaseException as cleanup:
            if primary is not None:raise BaseExceptionGroup('Storage operation and close failed',[primary,cleanup])
            raise


def canonical(value):
    """Validate an absolute normalized path without resolving absent mount points."""
    require(isinstance(value,str) and value and '\0' not in value,'storage-path-invalid')
    path=Path(value)
    require(path.is_absolute() and value==os.path.normpath(value) and not value.startswith('//'),'storage-path-not-canonical')
    return path


def _pairs(items):
    """Reject duplicate persistent JSON keys instead of silently replacing policy."""
    result={}
    for key,value in items:
        require(key not in result,'storage-policy-duplicate-key');result[key]=value
    return result


def load_policy(root):
    """Read strict owned configuration, returning policy plus exact-byte hash.

    Absence denies all external targets. No directory is created; malformed,
    aliased or writable-by-others configuration refuses with its original cause.
    """
    path=Path(root)/POLICY
    require(path.parent.resolve()==path.parent,'storage-policy-parent-symlink')
    try:fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
    except FileNotFoundError:return {'schema_version':1,'targets':[]},None
    stream=os.fdopen(fd,'rb')
    with _closing(stream.close):
        info=os.fstat(stream.fileno());require(stat.S_ISREG(info.st_mode) and info.st_uid==os.getuid() and not info.st_mode&0o022,'storage-policy-not-owned')
        raw=stream.read()
    value=json.loads(raw,object_pairs_hook=_pairs)
    require(isinstance(value,dict) and set(value)=={'schema_version','targets'} and type(value['schema_version']) is int and value['schema_version']==1 and isinstance(value['targets'],list),'storage-policy-invalid')
    seen=set()
    for entry in value['targets']:
        require(isinstance(entry,dict) and set(entry)=={'path','mountpoint','uuid','fstype'},'storage-target-invalid')
        target=canonical(entry['path']);mount=canonical(entry['mountpoint'])
        require(target!=mount and target.is_relative_to(mount),'storage-target-outside-mount')
        require(entry['path'] not in seen,'storage-target-duplicate');seen.add(entry['path'])
        require(isinstance(entry['uuid'],str) and entry['uuid'] and not any(c in entry['uuid'] for c in '\0\r\n'),'storage-uuid-invalid')
        require(isinstance(entry['fstype'],str) and entry['fstype'] in QUALIFIED_FILESYSTEMS,'storage-filesystem-not-qualified')
    return value,hashlib.sha256(raw).hexdigest()


def observe_mount(mount):
    """Read exact kernel mount facts using findmnt; missing/ambiguous mounts refuse.

    No mount, repair or privilege change is attempted. Subprocess failure retains
    its status/cause; command arguments contain no credential or shell expansion.
    """
    result=subprocess.run(['findmnt','--json','--mountpoint',str(mount),'--output','TARGET,SOURCE,FSTYPE,UUID,OPTIONS,MAJ:MIN,ID'],stdin=subprocess.DEVNULL,capture_output=True,check=True)
    values=json.loads(result.stdout).get('filesystems')
    require(isinstance(values,list) and len(values)==1,'storage-mount-not-unique')
    value=values[0]
    require(value.get('target')==str(mount),'storage-mountpoint-mismatch')
    options=str(value.get('options','')).split(',')
    require('rw' in options,'storage-mount-not-writable')
    require('noexec' not in options,'storage-mount-noexec')
    return {key:value.get(key) for key in ('target','source','fstype','uuid','maj:min','id','options')}


class _Admission:
    """Private state for admitted(), the sole supported lifetime-owning entry point.

    Hold/recheck a single admitted target until the caller finishes execution.

    External parents are traversed through directory FDs with O_NOFOLLOW, on the
    recorded device; only owned missing components are created after mount/UUID
    validation. No existing cache is moved. Close errors propagate, preserving a
    primary error when used by admitted(). Facts are observations, not authority.
    """
    def __init__(self,root,target):
        self.root=Path(root).resolve();self.target=canonical(str(target));self.fds=[]
        self.policy,self.policy_hash=load_policy(self.root)
        self.method_hash=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
        require(self.target.resolve()==self.target,'storage-target-symlink')
        self.entry=None;self.mount_facts=None
        if not self.target.is_relative_to(self.root):
            self.entry=next((e for e in self.policy['targets'] if e['path']==str(self.target)),None)
            require(self.entry is not None,'storage-external-target-not-registered')
        anchor=Path(self.entry['mountpoint']) if self.entry else self.root
        if self.entry:
            require(anchor.resolve()==anchor,'storage-mount-symlink')
            self.mount_facts=observe_mount(anchor)
            require(self.mount_facts['uuid']==self.entry['uuid'] and self.mount_facts['fstype']==self.entry['fstype'],'storage-mount-identity-mismatch')
        fd=os.open(anchor,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW);self.fds.append(fd)
        self.anchor=anchor;self.anchor_identity=self._identity(os.fstat(fd))
        require(self._identity(anchor.stat())==self.anchor_identity,'storage-mount-path-changed')
        if self.entry:
            self._check_device(fd)
            # Every component stays on the held admitted volume; never follow
            # a nested symlink/mount into the checkout or another output volume.
            for part in self.target.relative_to(anchor).parts:
                try:child=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
                except FileNotFoundError:
                    self.check();os.mkdir(part,mode=0o700,dir_fd=fd)
                    child=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd)
                self.fds.append(child);info=os.fstat(child)
                require(info.st_dev==self.anchor_identity[0] and info.st_uid==os.getuid(),'storage-target-owner-or-device-mismatch')
                fd=child
            self.target_identity=self._identity(os.fstat(fd))
        self.check()

    @staticmethod
    def _identity(info):
        """Descriptor identity includes device/inode; a path string is insufficient."""
        return (info.st_dev,info.st_ino)

    def _check_device(self,fd):
        """Bind the opened directory device to findmnt's observed major/minor."""
        value=os.fstat(fd);actual=f'{os.major(value.st_dev)}:{os.minor(value.st_dev)}'
        require(actual==self.mount_facts['maj:min'],'storage-open-device-mismatch')

    def check(self):
        """Revalidate configuration, helper bytes and held mount/target identities.

        Call immediately before Cargo and after execution. Failure never permits
        fallback; already executed work is retained as unqualified evidence.
        """
        _,current=load_policy(self.root);require(current==self.policy_hash,'storage-policy-changed')
        require(hashlib.sha256(Path(__file__).read_bytes()).hexdigest()==self.method_hash,'storage-method-changed')
        require(self._identity(os.fstat(self.fds[0]))==self.anchor_identity and self._identity(self.anchor.stat())==self.anchor_identity,'storage-held-path-changed')
        if self.entry:
            require(self.anchor.resolve()==self.anchor,'storage-mount-symlink')
            require(observe_mount(self.anchor)==self.mount_facts,'storage-mount-changed')
            self._check_device(self.fds[0]);require(self.target.resolve()==self.target,'storage-target-symlink')
            if hasattr(self,'target_identity'):
                require(self._identity(self.target.stat())==self.target_identity and self._identity(os.fstat(self.fds[-1]))==self.target_identity,'storage-target-changed')
        return self.proof()

    def proof(self):
        """Return safe build facts and persistent hashes; never local-web secrets."""
        return dict(mode='registered-external' if self.entry else 'checkout',target=str(self.target),policy_path=POLICY,policy_sha256=self.policy_hash,method_sha256=self.method_hash,mount=self.mount_facts,anchor_device=self.anchor_identity[0],anchor_inode=self.anchor_identity[1],target_identity=getattr(self,'target_identity',None),held_directory_fds=True,limits='Pre/post identity checks and ordinary-unmount hold; no forced/lazy-unmount or same-UID sandbox guarantee')

    def close(self):
        """Close all owned descriptors; preserve every cleanup failure."""
        errors=[]
        while self.fds:
            try:os.close(self.fds.pop())
            except OSError as error:errors.append(error)
        if errors:raise ExceptionGroup('Storage descriptor cleanup failed',errors)


@contextlib.contextmanager
def admitted(root,target):
    """Admit/hold a target; close on refusal too, without masking primary errors."""
    guard=_Admission.__new__(_Admission);guard.fds=[]
    try:
        guard.__init__(root,target)
        yield guard
    finally:
        primary=sys.exception()
        try:guard.close()
        except BaseException as cleanup:
            if primary is not None:raise BaseExceptionGroup('Storage operation and cleanup failed',[primary,cleanup])
            raise


@contextlib.contextmanager
def _writer_lock(root):
    """Serialize cooperative registration writers without waiting or deleting locks.

    Hold the owned O_NOFOLLOW descriptor through read/replace. A busy lock refuses;
    original operation and unlock/close failures are preserved together.
    """
    parent=Path(root)/'.cargo';require(parent.resolve()==parent,'storage-policy-parent-symlink');parent.mkdir(exist_ok=True,mode=0o700)
    require(parent.stat().st_uid==os.getuid(),'storage-policy-parent-not-owned')
    path=parent/'external-targets.lock';fd=os.open(path,os.O_CREAT|os.O_RDWR|os.O_NOFOLLOW,0o600);locked=False
    try:
        info=os.fstat(fd);require(stat.S_ISREG(info.st_mode) and info.st_uid==os.getuid() and not info.st_mode&0o022 and (info.st_dev,info.st_ino)==(path.stat().st_dev,path.stat().st_ino),'storage-policy-lock-not-owned')
        try:fcntl.flock(fd,fcntl.LOCK_EX|fcntl.LOCK_NB);locked=True
        except BlockingIOError as error:raise ValueError('storage-registration-busy') from error
        yield
    finally:
        primary=sys.exception();errors=[]
        if locked:
            try:fcntl.flock(fd,fcntl.LOCK_UN)
            except OSError as error:errors.append(error)
        try:os.close(fd)
        except OSError as error:errors.append(error)
        if errors:raise BaseExceptionGroup('Storage registration cleanup failed',([primary] if primary is not None else [])+errors)


def register(root,target,mountpoint,uuid,fstype):
    """Register an explicitly selected temporary cache under the policy writer lock.

    No repository, installed binary or runtime moves. Busy/invalid/missing mounts
    refuse, and policy/descriptor cleanup errors retain their original causes.
    """
    root=Path(root).resolve()
    with _writer_lock(root):return _register_locked(root,target,mountpoint,uuid,fstype)


def _register_locked(root,target,mountpoint,uuid,fstype):
    """Persist one explicit target after a noncreating real mount/device witness.

    Existing registrations are preserved, exact path replacement is explicit.
    Atomic fsynced write preserves the previous policy before replacement; a
    directory-sync failure after replace is an uncertain applied outcome. This
    registers facts only; Cargo and delivered/runtime checks are separate. Failed
    temporary candidates remain for diagnosis. Existing checkout-directory modes
    are preserved: availability against a writer of that namespace is not promised.
    """
    root=Path(root).resolve();target=canonical(str(target));mount=canonical(str(mountpoint))
    require(target!=mount and target.is_relative_to(mount) and target.resolve()==target and mount.resolve()==mount,'storage-registration-path-invalid')
    require(fstype in QUALIFIED_FILESYSTEMS,'storage-filesystem-not-qualified')
    actual=observe_mount(mount);require(actual['uuid']==uuid and actual['fstype']==fstype,'storage-registration-mismatch')
    fd=os.open(mount,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW)
    with _closing(lambda:os.close(fd)):
        value=os.fstat(fd);require(f'{os.major(value.st_dev)}:{os.minor(value.st_dev)}'==actual['maj:min'] and (value.st_dev,value.st_ino)==(mount.stat().st_dev,mount.stat().st_ino),'storage-registration-device-mismatch')
        policy,prior_hash=load_policy(root);entry=dict(path=str(target),mountpoint=str(mount),uuid=uuid,fstype=fstype);policy['targets']=[e for e in policy['targets'] if e['path']!=str(target)]+[entry]
        path=root/POLICY;require(path.parent.resolve()==path.parent,'storage-policy-parent-symlink');path.parent.mkdir(exist_ok=True,mode=0o700)
        tmpfd,tmp=tempfile.mkstemp(prefix='.external-targets-',dir=path.parent)
        payload=(json.dumps(policy,indent=2)+'\n').encode()
        stream=os.fdopen(tmpfd,'wb')
        with _closing(stream.close):
            stream.write(payload);stream.flush();os.fsync(stream.fileno())
        require(Path(tmp).read_bytes()==payload,'storage-policy-readback-mismatch')
        require(observe_mount(mount)==actual and (os.fstat(fd).st_dev,os.fstat(fd).st_ino)==(mount.stat().st_dev,mount.stat().st_ino),'storage-registration-mount-changed')
        require(load_policy(root)[1]==prior_hash,'storage-registration-policy-changed')
        os.replace(tmp,path);directory=os.open(path.parent,os.O_DIRECTORY)
        with _closing(lambda:os.close(directory)):os.fsync(directory)
    return entry


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--root',type=Path,default=Path.cwd());sub=parser.add_subparsers(dest='command',required=True);add=sub.add_parser('register')
    for key in ('target','mountpoint','uuid','fstype'):add.add_argument('--'+key,required=True)
    args=parser.parse_args();print(json.dumps(register(args.root,args.target,args.mountpoint,args.uuid,args.fstype)))
