#!/usr/bin/env python3
"""Offline owned-fixture snapshot/restore for migration verification only.

Caller must close all fixture writers and own stable source/destination trees.
No live discovery/lock, active Root backup, authority migration or production
restore is provided. Reject SQLite WAL/SHM rather than copying a hot database.
Every regular file/directory is inventoried, including private fixture configs;
proofs must remain private. Identical data never transfers physical execution
bindings or actor identity. No retry, size policy, watchdog or source deletion.
"""
from pathlib import Path,PurePosixPath
import argparse,hashlib,importlib.util,json,os,stat,tempfile
spec=importlib.util.spec_from_file_location('snapshot_io',Path(__file__).resolve().parents[1]/'verification/verify_native_profile.py')
io=importlib.util.module_from_spec(spec);spec.loader.exec_module(io)


def inventory(root):
    """Hash exact owned stable fixture bytes/modes; reject links/special objects.

    Paths are local observations, not a race-free sandbox. Concurrent mutation
    violates the caller precondition; before/after comparison detects observed
    drift but cannot establish a globally atomic snapshot or closed processes.
    """
    result={}
    for path in [root,*sorted(root.rglob('*'))]:
        st=path.lstat();io.require(st.st_uid==os.geteuid(),'snapshot-unowned-entry')
        name='.' if path==root else path.relative_to(root).as_posix()
        io.require(stat.S_ISDIR(st.st_mode) or stat.S_ISREG(st.st_mode),'snapshot-link-or-special-entry')
        result[name]=dict(kind='directory' if stat.S_ISDIR(st.st_mode) else 'file',mode=stat.S_IMODE(st.st_mode),sha256=io.digest(path) if stat.S_ISREG(st.st_mode) else None)
    io.require(result.get('.',{}).get('kind')=='directory','snapshot-root-not-directory')
    return result


def disjoint(left,right):
    """Refuse overlapping explicit trees before copying; no path authority follows."""
    io.require(not (left==right or left.is_relative_to(right) or right.is_relative_to(left)),'snapshot-overlapping-trees')


def copy_tree(source,destination,entries):
    """Copy declared files atomically, then apply directory modes from children up.

    Destination must be exclusively owned/fresh. Failure preserves originals and
    partial destination evidence; no rollback is asserted and no cleanup repeats.
    Directories are initially private/writable. Existing targets refuse.
    """
    destination.mkdir(mode=0o700,exist_ok=False)
    for name,value in entries.items():
        if name=='.':continue
        target=destination/name
        if value['kind']=='directory':target.mkdir(mode=0o700)
        else:
            io.publish(target,(source/name).read_bytes());target.chmod(value['mode'])
    for name,value in reversed(list(entries.items())):
        if value['kind']=='directory':(destination if name=='.' else destination/name).chmod(value['mode'])
    io.require(inventory(destination)==entries,'snapshot-copy-readback-mismatch')


def capture(root,out):
    """Capture one closed inactive fixture into fresh private proof storage.

    WAL/SHM refuse even if empty. Full tree bytes/modes are compared before and
    after copy; absence of observed drift is not a guarantee against writers.
    No source mutation, SQL connection, process, lease or source cleanup occurs.
    """
    root=Path(root).absolute();out=Path(out).absolute();disjoint(root,out)
    io.require(not root.is_symlink() and root.is_dir(),'snapshot-root-not-real-directory')
    io.require((root/'.swarm/state.db').is_file(),'snapshot-fixture-datastore-missing')
    io.require(not any(os.path.lexists(root/'.swarm'/name) for name in ['state.db-wal','state.db-shm']),'snapshot-wal-or-shm-present')
    entries=inventory(root);out.mkdir(mode=0o700,exist_ok=False)
    copy_tree(root,out/'data',entries)
    io.require(inventory(root)==entries,'snapshot-source-drift')
    manifest=dict(schema_version=1,scope='closed-owned-fixture-only',entries=entries,execution_authority_transferred=False)
    io.publish(out/'manifest.json',json.dumps(manifest,sort_keys=True,indent=2)+'\n')
    verify(out);return manifest


def verify(snapshot):
    """Verify strict manifest and exact regular tree before any restore writes.

    Local hashes detect retained-byte corruption, not malicious trusted-owner
    rewriting of both data and manifest. Unknown fields, traversal, special files,
    invalid hashes/modes and incomplete snapshots refuse with their original cause.
    """
    snapshot=Path(snapshot);manifest=snapshot/'manifest.json';st=manifest.lstat()
    io.require(stat.S_ISREG(st.st_mode) and st.st_uid==os.geteuid(),'snapshot-manifest-not-owned-regular')
    m=json.loads(manifest.read_bytes())
    io.require(set(m)=={'schema_version','scope','entries','execution_authority_transferred'} and type(m['schema_version']) is int and m['schema_version']==1 and m['scope']=='closed-owned-fixture-only' and m['execution_authority_transferred'] is False,'snapshot-manifest-contract')
    entries=m['entries'];io.require(isinstance(entries,dict) and '.' in entries,'snapshot-inventory-missing')
    for name,value in entries.items():
        io.require(isinstance(name,str) and (name=='.' or (str(PurePosixPath(name))==name and not PurePosixPath(name).is_absolute() and '..' not in PurePosixPath(name).parts)),'snapshot-path-invalid')
        io.require(isinstance(value,dict) and set(value)=={'kind','mode','sha256'} and value['kind'] in ['file','directory'] and type(value['mode']) is int and 0<=value['mode']<=0o7777,'snapshot-entry-invalid')
        sha=value['sha256'];io.require((value['kind']=='directory' and sha is None) or (value['kind']=='file' and isinstance(sha,str) and len(sha)==64 and all(c in '0123456789abcdef' for c in sha)),'snapshot-hash-invalid')
    io.require(inventory(snapshot/'data')==entries,'snapshot-retained-tree-mismatch')
    return m


def restore(snapshot,destination):
    """Verify, stage and publish to a fresh exclusively owned fixture destination.

    Never overwrite or delete an existing Root. A failed copy leaves a private
    sibling staging tree for diagnosis; post-rename sync failure means publication
    may already have occurred. Success removes only its empty staging container.
    No source DB inode/authority is adopted or rebased.
    Caller exclusivity is required; this is not rename-noreplace against attackers.
    """
    snapshot=Path(snapshot).absolute();destination=Path(destination).absolute();disjoint(snapshot,destination)
    m=verify(snapshot);io.require(not os.path.lexists(destination),'snapshot-destination-exists')
    staging=Path(tempfile.mkdtemp(prefix='.'+destination.name+'-restore-',dir=destination.parent))
    copy_tree(snapshot/'data',staging/'tree',m['entries']);io.require(not os.path.lexists(destination),'snapshot-destination-appeared')
    os.rename(staging/'tree',destination)
    fd=os.open(destination.parent,os.O_DIRECTORY)
    try:os.fsync(fd)
    finally:os.close(fd)
    io.require(inventory(destination)==m['entries'],'snapshot-published-readback-mismatch')
    staging.rmdir()  # Only our now-empty container; never source or retained failure data.
    return dict(status='RESTORED_FIXTURE',entries=len(m['entries']),execution_authority_transferred=False)


def main():
    """Explicit fixture paths only; emit counts, never private data or discovery."""
    p=argparse.ArgumentParser(description=__doc__);sub=p.add_subparsers(dest='op',required=True)
    c=sub.add_parser('capture');c.add_argument('--fixture-root',type=Path,required=True);c.add_argument('--output',type=Path,required=True)
    v=sub.add_parser('verify');v.add_argument('--snapshot',type=Path,required=True)
    r=sub.add_parser('restore');r.add_argument('--snapshot',type=Path,required=True);r.add_argument('--destination',type=Path,required=True)
    a=p.parse_args()
    if a.op=='capture':m=capture(a.fixture_root,a.output);print(json.dumps(dict(status='CAPTURED_FIXTURE',entries=len(m['entries']),execution_authority_transferred=False)))
    elif a.op=='verify':m=verify(a.snapshot);print(json.dumps(dict(status='VERIFIED_FIXTURE',entries=len(m['entries']))))
    else:print(json.dumps(restore(a.snapshot,a.destination)))

if __name__=='__main__':main()
