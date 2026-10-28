#!/usr/bin/env python3
"""Read-only PAEA Linux prerequisite observation, never launch authority.

No namespace, sudo, package, account, daemon or protected-project operation runs.
A present helper/range/euid0 is not a verified launch, isolation or human identity.
Output may describe only the current namespace; its labels are not owned handles.
"""
import argparse
import json
import os
from pathlib import Path
import pwd
import shutil
import stat
import sys
import tempfile

# Linux uid_t's all-ones value is invalid, not an operator allocation ceiling.
UID_EXCLUSIVE = (1 << 32) - 1


def uint(value):
    """Parse unsigned decimal kernel/config UID representation.

    Reject Unicode/signs/negative/sentinel/overflow. This validates representation,
    not ownership or permission to map any returned UID.
    """
    if not value or not value.isascii() or not value.isdecimal():
        raise ValueError('invalid-decimal')
    number = int(value)
    if number >= UID_EXCLUSIVE:
        raise ValueError('invalid-uid')
    return number


def parse_ranges(text, username, uid):
    """Read only this namespace-local user's assigned subordinate ranges.

    Return all explicit ranges; never pick an identity or infer exclusive leases.
    Matching malformed, empty, overlapping or overflowing ranges refuse instead
    of becoming a successful empty assignment. Other users' data is not returned.
    """
    result = []
    for line in text.splitlines():
        if not line or line.lstrip().startswith('#'):
            continue
        fields = line.split(':')
        if fields[0] not in [username, str(uid)]:
            continue
        if len(fields) != 3:
            raise ValueError('invalid-own-range')
        start, count = uint(fields[1]), uint(fields[2])
        if not start or not count or start + count > UID_EXCLUSIVE:
            raise ValueError('invalid-own-range')
        if any(start < r['start'] + r['count'] and r['start'] < start + count for r in result):
            raise ValueError('overlapping-own-range')
        result.append({'start': start, 'count': count})
    return result


def parse_map(text):
    """Decode actual three-column kernel UID/GID maps, not a desired mapping.

    Initial full-range maps are legal. No live namespace identity, host-root
    privilege or separation is inferred from these unowned observations.
    """
    result = []
    for line in text.splitlines():
        fields = line.split()
        if len(fields) != 3:
            raise ValueError('invalid-map')
        inside, outside = uint(fields[0]), uint(fields[1])
        count = int(fields[2]) if fields[2].isascii() and fields[2].isdecimal() else 0
        if count <= 0 or inside + count > UID_EXCLUSIVE or outside + count > UID_EXCLUSIVE:
            raise ValueError('invalid-map')
        result.append({'inside': inside, 'outside': outside, 'count': count})
    if not result:
        raise ValueError('empty-map')
    return result


def read(path):
    """Read one known public host/kernel metadata file, retaining failed status.

    Safe errors expose errno/category only; unavailable is not zero/empty success.
    No writes, arbitrary caller file, retry or fallback is performed.
    """
    try:
        return {'status': 'read', 'text': Path(path).read_text()}
    except OSError as error:
        return {'status': 'unavailable', 'errno': error.errno, 'cause': type(error).__name__}


def parsed(path, parser):
    """Preserve unavailable/invalid metadata distinctly from validated records."""
    value = read(path)
    if value['status'] != 'read':
        return value
    try:
        return {'status': 'observed', 'value': parser(value['text'])}
    except (ValueError, OverflowError):
        return {'status': 'invalid', 'cause': 'metadata-format'}


def helper(path):
    """Observe a candidate mapping helper's file custody, never execute/admit it.

    Resolve installed symlinks; require root-owned executable regular setuid file
    and root-owned non-group/other-writable ancestors. Even safe metadata does
    not establish a valid mapping, inode lifetime, launch or permission grant.
    """
    if path is None:
        return {'status': 'unavailable'}
    try:
        real = Path(path).resolve(strict=True)
        value = real.stat()
        safe = stat.S_ISREG(value.st_mode) and value.st_uid == 0 and bool(value.st_mode & stat.S_ISUID) and bool(value.st_mode & 0o111) and not value.st_mode & 0o022
        for ancestor in real.parents:
            metadata = ancestor.stat()
            safe = safe and metadata.st_uid == 0 and not metadata.st_mode & 0o022
        return {'status': 'metadata_safe' if safe else 'unsafe', 'path': str(real),
                'owner': value.st_uid, 'mode': oct(stat.S_IMODE(value.st_mode))}
    except OSError as error:
        return {'status': 'unavailable', 'errno': error.errno, 'cause': type(error).__name__}


def observe():
    """Collect current namespace-local prerequisites without mutating the host.

    Report missing/invalid sources and false launch/confinement/human claims.
    No range, UID, default policy or privileged alternative is automatically chosen.
    """
    uid = os.geteuid()
    try:
        username = pwd.getpwuid(uid).pw_name
    except KeyError:
        username = str(uid)
    helpers = {name: helper(shutil.which(name)) for name in ['newuidmap', 'newgidmap']}
    ranges = {name: parsed('/etc/' + name, lambda text: parse_ranges(text, username, uid)) for name in ['subuid', 'subgid']}
    maps = {name: parsed('/proc/self/' + name, parse_map) for name in ['uid_map', 'gid_map']}
    missing = ['helper:' + name for name, value in helpers.items() if value['status'] != 'metadata_safe']
    missing += ['range:' + name for name, value in ranges.items() if value['status'] != 'observed' or not value['value']]
    missing += ['map:' + name for name, value in maps.items() if value['status'] != 'observed']
    return {'schema_version': 1, 'kind': 'paea-linux-prerequisite-observation',
            'current_namespace': {'euid': uid, 'egid': os.getegid(), 'groups': os.getgroups(),
                                  'passwd_name': username, 'maps': maps},
            'mapping_helpers': helpers, 'assigned_ranges': ranges,
            'missing_prerequisites': missing, 'read_only': True,
            'launch_verified': False, 'separate_host_uid_verified': False,
            'confinement_verified': False, 'human_verified': False,
            'limits': 'Current namespace observations are not owned handles, exclusive UID leases, authority or a qualified launch.'}


def publish(directory, value):
    """Publish one report in a new private directory, preserving existing paths.

    Atomic same-directory file write/readback/fsync/rename; no truncation of an
    existing report. Failure keeps original paths and reports the real exception.
    """
    directory.mkdir(mode=0o700)
    raw = (json.dumps(value, indent=2) + '\n').encode()
    fd, name = tempfile.mkstemp(prefix='.report-', dir=directory)
    try:
        with os.fdopen(fd, 'wb') as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        if Path(name).read_bytes() != raw:
            raise OSError('report-readback')
        os.rename(name, directory / 'report.json')
    finally:
        if os.path.exists(name):
            os.unlink(name)


def main():
    """Print factual JSON and optionally retain it in a fresh caller-owned path.

    Exit0 means observation completed, never protected launch ready. Unsupported
    systems and publication errors are nonzero. No password is requested/read.
    """
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path)
    args = parser.parse_args()
    if not sys.platform.startswith('linux'):
        raise SystemExit('unsupported-platform')
    value = observe()
    if args.output is not None:
        publish(args.output, value)
    print(json.dumps(value, indent=2))


if __name__ == '__main__':
    main()
