#!/usr/bin/env python3
"""Inventory direct/archived Go references in a Git publication candidate.

Read-only: never removes files/index entries or rewrites history. Inspect index
blobs AND physical tracked candidates; patches use source headers, archives use
member metadata without extraction. Nested containers remain REVIEW_REQUIRED;
this finite scan cannot establish absence of arbitrary encoded source strings.
Git recovery bundles are local-only under the repository publication contract.
"""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import shlex
import subprocess
import tarfile
import zipfile
from recipe import publish

SUFFIXES=('.tar.gz','.tgz','.tar','.zip','.bundle','.patch','.diff')


class InventoryError(Exception):
    """Safe operation family with original typed cause and partial report.

    Raw archive/patch contents are not displayed. Callers can inspect the chained
    cause; CLI persists safe status/identities and exits nonzero on incomplete IO.
    """
    def __init__(self,code,report=None):
        super().__init__(code);self.code=code;self.report=report


def digest(raw):
    """Hash exact indexed/physical bytes; a name alone never binds a candidate."""
    return hashlib.sha256(raw).hexdigest()


def require_explicit_git_context():
    """Refuse inherited repository/index/object redirection before any Git call.

    The same precondition protects fixture initialization and inventory reads.
    No variable is cleared or echoed; refusal grants no right to alter user state.
    """
    if any(os.environ.get(k) for k in ('GIT_DIR','GIT_WORK_TREE','GIT_INDEX_FILE','GIT_COMMON_DIR','GIT_OBJECT_DIRECTORY','GIT_ALTERNATE_OBJECT_DIRECTORIES')):
        raise InventoryError('git-context-override')


def git(repo,*args):
    """Run explicit read-only Git argv without shell or context overrides.

    Errors preserve subprocess causes; no global configuration or index mutates.
    Explicit repository selection must not be redirected by inherited Git env.
    """
    require_explicit_git_context()
    try:return subprocess.run(['git','-c','core.fsmonitor=false','-C',str(repo),*args],check=True,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env={**os.environ,'GIT_OPTIONAL_LOCKS':'0'}).stdout
    except subprocess.CalledProcessError as error:raise InventoryError('git-read-failed') from error


def go_path(name):
    """Recognize Go source/module/workspace path vocabulary, not content strings."""
    return name.endswith('.go') or PurePosixPath(name).name in ('go.mod','go.sum','go.work','go.work.sum')


def kind(name):
    """Classify declared archive/patch containers; unknown encodings are outside scope."""
    return next((suffix for suffix in SUFFIXES if name.endswith(suffix)),None)


def inspect_payload(name,raw):
    """Inspect bytes without archive extraction or execution.

    Go members/header paths flag a candidate; nested payloads are explicitly
    unreviewed, never treated as Go-free. Malformed format/UTF8 retains a cause.
    This does not prove source identity, licensing, execution or global absence.
    """
    if go_path(name):return dict(go=[name],nested=[],local_only_bundle=False)
    category=kind(name);go=[];nested=[]
    try:
        if category in ('.tar.gz','.tgz','.tar'):
            with tarfile.open(fileobj=io.BytesIO(raw),mode='r:*') as archive:
                for entry in archive:
                    if not entry.isdir() and go_path(entry.name):go.append(entry.name)
                    elif entry.isfile() and kind(entry.name):nested.append(entry.name)
        elif category=='.zip':
            with zipfile.ZipFile(io.BytesIO(raw)) as archive:
                for entry in archive.infolist():
                    if not entry.is_dir() and go_path(entry.filename):go.append(entry.filename)
                    elif not entry.is_dir() and kind(entry.filename):nested.append(entry.filename)
        elif category in ('.patch','.diff'):
            recognized=False
            for line in raw.decode('utf-8').splitlines():
                paths=[]
                if line.startswith('diff --git '):
                    parts=shlex.split(line)
                    if len(parts)<4:raise InventoryError('publication-patch-format-unreviewed')
                    paths=parts[2:];recognized=True
                elif line.startswith(('--- ','+++ ')):
                    paths=shlex.split(line[4:].split('\t',1)[0]);recognized=recognized or bool(paths)
                elif line.startswith(('*** Add File: ','*** Update File: ','*** Delete File: ')):
                    paths=[line.split(': ',1)[1]];recognized=recognized or bool(paths[0])
                go.extend(path for path in paths if go_path(path))
            if raw.strip() and not recognized:raise InventoryError('publication-patch-format-unreviewed')
        return dict(go=sorted(set(go)),nested=sorted(set(nested)),local_only_bundle=category=='.bundle')
    except (tarfile.TarError,zipfile.BadZipFile,UnicodeError,ValueError,OSError) as error:
        raise InventoryError('publication-payload-invalid') from error


def worktree_bytes(repo,name):
    """Read one regular tracked path beneath no-follow directory descriptors.

    Missing stays explicit; symlink/nonregular/changing files or read/close
    errors refuse. All descriptors close with original plus cleanup causes kept.
    No external target is followed and no original is written or removed.
    """
    parts=PurePosixPath(name).parts
    if not parts or PurePosixPath(name).is_absolute() or any(x in ('.','..') for x in parts):raise InventoryError('publication-path-invalid')
    descriptors=[];primary=None;raw=None
    try:
        fd=os.open(repo,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW);descriptors.append(fd)
        for part in parts[:-1]:
            fd=os.open(part,os.O_RDONLY|os.O_DIRECTORY|os.O_NOFOLLOW,dir_fd=fd);descriptors.append(fd)
        fd=os.open(parts[-1],os.O_RDONLY|os.O_NOFOLLOW,dir_fd=fd);descriptors.append(fd)
        import stat
        before=os.fstat(fd)
        if not stat.S_ISREG(before.st_mode):raise InventoryError('publication-worktree-not-regular')
        chunks=[]
        while True:
            chunk=os.read(fd,65536) # Copy buffer, not an inspection/volume cutoff.
            if not chunk:break
            chunks.append(chunk)
        raw=b''.join(chunks);after=os.fstat(fd)
        if (before.st_dev,before.st_ino,before.st_size,before.st_mtime_ns)!=(after.st_dev,after.st_ino,after.st_size,after.st_mtime_ns) or len(raw)!=after.st_size:raise InventoryError('publication-worktree-changed')
    except FileNotFoundError:raw=None
    except BaseException as error:primary=error
    cleanup=[]
    for fd in reversed(descriptors):
        try:os.close(fd)
        except BaseException as error:cleanup.append(error)
    if primary or cleanup:
        causes=([primary] if primary else [])+cleanup
        raise InventoryError('publication-worktree-read') from BaseExceptionGroup('publication read/close causes',causes)
    return raw


def inventory(repo):
    """Bind a complete current Git-index candidate inventory and safe findings.

    Stage conflicts, Git changes, missing/corrupt objects or IO failures cannot
    become PASS. Index and worktree payloads remain distinct. Errors retain a
    partial report and chained originals; no mutation/publication is authorized.
    """
    repo=Path(repo).resolve()
    if Path(os.fsdecode(git(repo,'rev-parse','--show-toplevel')).strip()).resolve()!=repo:raise InventoryError('publication-repository-mismatch')
    original=git(repo,'ls-files','--stage','-z');rows=[];errors=[];causes=[]
    for record in original.split(b'\0'):
        if not record:continue
        header,name=record.split(b'\t',1);mode,oid,stage=header.decode('ascii').split();name=os.fsdecode(name)
        if stage!='0':raise InventoryError('publication-index-unmerged')
        if not (go_path(name) or kind(name)):continue
        row=dict(path=name,mode=mode,oid=oid);origin='index'
        try:
            if mode not in ('100644','100755'):raise InventoryError('publication-index-not-regular')
            raw=git(repo,'cat-file','blob',oid);row['index_sha256']=digest(raw);row['index']=inspect_payload(name,raw)
            origin='worktree';physical=worktree_bytes(repo,name);row['worktree_missing']=physical is None
            if physical is not None:
                row['worktree_sha256']=digest(physical);row['worktree']=row['index'] if physical==raw else inspect_payload(name,physical)
        except InventoryError as error:
            errors.append(dict(path=name,origin=origin,code=error.code,cause_type=type(error.__cause__).__name__ if error.__cause__ else None));causes.append(error)
        rows.append(row)
    if git(repo,'ls-files','--stage','-z')!=original:
        errors.append(dict(code='publication-index-changed'));causes.append(InventoryError('publication-index-changed'))
    flagged=[r['path'] for r in rows if any(r.get(origin,{}).get('go') or r.get(origin,{}).get('local_only_bundle') for origin in ('index','worktree'))]
    nested=[dict(path=r['path'],origin=origin,members=r[origin]['nested']) for r in rows for origin in ('index','worktree') if r.get(origin,{}).get('nested')]
    report=dict(schema_version=1,scope='Indexed Go paths; named tar/zip member metadata and patch source headers; Git recovery bundles local-only. Nested/unknown encodings are not certified absent.',repository=str(repo),index_entries_sha256=digest(original),candidates=len(rows),flagged_paths=flagged,unreviewed_nested=nested,errors=errors,paths=rows,status='INCOMPLETE' if errors else 'REVIEW_REQUIRED' if nested else 'LEGACY_SOURCE_PRESENT' if flagged else 'PASS')
    if errors:raise InventoryError('publication-inventory-incomplete',report) from BaseExceptionGroup('publication inspection causes',causes)
    return report


def main():
    """Save fresh atomic evidence and exit nonzero for Go, bundles or unknowns.

    CLI never untracks/deletes data. Safe output excludes payload/error strings;
    original blobs and hashes remain the reproducible diagnostic source.
    """
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--repo',type=Path,default=Path.cwd());parser.add_argument('--output',required=True,type=Path);args=parser.parse_args()
    try:report=inventory(args.repo)
    except InventoryError as error:
        report=error.report or dict(schema_version=1,status='INCOMPLETE',code=error.code)
    args.output.parent.mkdir(parents=True,exist_ok=True);publish(args.output,(json.dumps(report,ensure_ascii=True,indent=2)+'\n').encode())
    print(json.dumps({k:report[k] for k in ('status','candidates') if k in report},ensure_ascii=True),flush=True)
    return 0 if report['status']=='PASS' else 2 if report['status']=='INCOMPLETE' else 1


if __name__=='__main__':raise SystemExit(main())
