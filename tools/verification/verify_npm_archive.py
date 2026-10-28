#!/usr/bin/env python3
"""Read an npm tar archive without extracting and compare every source to stdin JSON.

The caller supplies its freshly observed file/symlink map. Missing, duplicate,
unsafe or mismatching entries fail closed. Directories carry no source bytes;
hard links must resolve to an archived regular file. This verifies source
content, not filesystem permissions, OS libraries or browser execution.
"""
import hashlib
import json
import posixpath
import sys
import tarfile


def checked_name(name):
    """Normalize a relative tar name; reject absolute paths and traversal."""
    if name.startswith('/') or '..' in name.split('/'):
        raise ValueError('unsafe archive name')
    return posixpath.normpath(name)


def verify(archive, expected):
    """Return the source count after full readback; raise on any discrepancy.

    Reads archive bytes only, with no extraction or global verification deadline.
    Expected keys are the collector's actual relative paths and digest/link maps.
    """
    observed, links, names = {}, {}, set()
    with tarfile.open(archive, 'r:gz') as source:
        for member in source:
            name = checked_name(member.name)
            if name in names:
                raise ValueError('duplicate archive entry')
            names.add(name)
            if member.isdir():
                if name in expected:
                    raise ValueError('directory replaced a source')
                continue
            if name not in expected:
                raise ValueError('unexpected archive source')
            if member.isfile():
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError('missing archive file stream')
                with stream:
                    observed[name] = {'sha256': hashlib.file_digest(stream, 'sha256').hexdigest()}
            elif member.issym():
                observed[name] = {'symlink': member.linkname}
            elif member.islnk():
                links[name] = checked_name(member.linkname)
            else:
                raise ValueError('unsupported archive entry')
    for name in links:
        target, seen = name, set()
        while target in links:
            if target in seen:
                raise ValueError('cyclic archive hard link')
            seen.add(target)
            target = links[target]
        if target not in observed or 'sha256' not in observed[target]:
            raise ValueError('unresolved archive hard link')
        observed[name] = observed[target]
    if observed != expected:
        raise ValueError('archive source content differs from installed npm inputs')
    return len(observed)


if __name__ == '__main__':
    print(json.dumps({'status': 'PASS', 'files': verify(sys.argv[1], json.load(sys.stdin))}))
