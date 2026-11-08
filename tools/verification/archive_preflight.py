#!/usr/bin/env python3
"""Check explicit expected SHA-256 members in a plain tar or ordered tar parts.

Read-only: never extracts, restores, mutates or authenticates an author. Requires
Python 3.13+ tar streaming metadata support. Scans through the logical end and
requires a two-zero-block terminator with zero-only padding: a parser stopping
at corrupt/truncated later headers cannot become success. Selected members must
be unique regular files. Manifest expectations are supplied by the caller, not
inferred from an index. No retry, global deadline or volume cutoff.
"""
import argparse
import bisect
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
import tarfile


class Refusal(Exception):
    """Preserve a stable failure code and optional typed cause for CLI reporting."""
    def __init__(self, code):
        """Create an explicit refusal without echoing archive payload bytes."""
        super().__init__(code)
        self.code = code


def identity(info):
    """Return observed inode/size/time metadata, not a lock or content proof."""
    return (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


def regular(path):
    """Open a regular file without following the final symlink or blocking on FIFO.

    Returns an owned binary stream. IO/type failures refuse with their cause;
    already opened descriptors are closed on rejection. Ancestor directories are
    not confined, so this is not OS isolation.
    """
    fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    try:
        if not stat.S_ISREG(os.fstat(fd).st_mode):
            raise Refusal('archive_source')
        stream = os.fdopen(fd, 'rb')
    except BaseException:
        os.close(fd)
        raise
    return stream


class SplitArchive(io.RawIOBase):
    """Read-only seekable concatenation with retained descriptors and freshness checks."""
    def __init__(self, paths):
        """Pin explicit ordered regular parts; refuse aliases and close partial opens."""
        self.paths = list(map(Path, paths))
        self.streams, self.stamps, self.offsets = [], [], [0]
        self.position = 0
        try:
            for path in self.paths:
                stream = regular(path)
                self.streams.append(stream)
                stamp = identity(os.fstat(stream.fileno()))
                if stamp[:2] in [s[:2] for s in self.stamps]:
                    raise Refusal('archive_source')
                self.stamps.append(stamp)
                self.offsets.append(self.offsets[-1] + stamp[2])
            if not self.streams:
                raise Refusal('archive_source')
        except BaseException:
            self.close()
            raise

    def readable(self):
        """Declare the read capability required by tarfile and hashlib."""
        return True

    def seekable(self):
        """Declare logical seeking; this does not concatenate parts on disk."""
        return True

    def tell(self):
        """Return current logical byte position."""
        return self.position

    def seek(self, offset, whence=io.SEEK_SET):
        """Set logical position; negative or unsupported seeks fail without IO."""
        if whence not in (io.SEEK_SET, io.SEEK_CUR, io.SEEK_END):
            raise ValueError('invalid whence')
        target = offset + (0 if whence == io.SEEK_SET else
                           self.position if whence == io.SEEK_CUR else self.offsets[-1])
        if target < 0:
            raise ValueError('negative offset')
        self.position = target
        return target

    def read(self, size=-1):
        """Read logical bytes across part boundaries; unexpected EOF propagates.

        This is an IO adapter, not a volume policy. Consumers use bounded buffers;
        requesting all remaining bytes explicitly has normal IO memory costs.
        """
        remaining = max(0, self.offsets[-1] - self.position)
        wanted = remaining if size < 0 else min(size, remaining)
        chunks = []
        while wanted:
            index = bisect.bisect_right(self.offsets, self.position) - 1
            stream = self.streams[index]
            stream.seek(self.position - self.offsets[index])
            raw = stream.read(min(wanted, self.offsets[index + 1] - self.position))
            if not raw:
                raise Refusal('archive_invalid')
            chunks.append(raw)
            self.position += len(raw)
            wanted -= len(raw)
        return b''.join(chunks)

    def readinto(self, buffer):
        """Fill a caller-owned buffer and return observed byte count."""
        raw = self.read(len(buffer))
        buffer[:len(raw)] = raw
        return len(raw)

    def check_stable(self):
        """Refuse changed descriptors or replaced paths; never infer continuous locking."""
        for path, stream, stamp in zip(self.paths, self.streams, self.stamps):
            if identity(os.fstat(stream.fileno())) != stamp or identity(path.lstat()) != stamp:
                raise Refusal('archive_changed')

    def close(self):
        """Release all owned descriptors even when an earlier close fails."""
        failures = []
        for stream in self.streams:
            try:
                stream.close()
            except OSError as error:
                failures.append(error)
        super().close()
        if failures:
            raise ExceptionGroup('archive close failures', failures)


def unique_object(pairs):
    """Reject duplicate JSON keys rather than silently adopting a later expectation."""
    value = {}
    for key, item in pairs:
        if key in value:
            raise Refusal('manifest_invalid')
        value[key] = item
    return value


def member_name(name):
    """Accept canonical relative POSIX names, never traversal or ambiguous aliases."""
    if (not name or '\0' in name or '\\' in name or name.startswith('/')
            or '..' in name.split('/') or PurePosixPath(name).as_posix() != name
            or name == '.'):
        raise Refusal('manifest_invalid')
    return name


def expected_members(path):
    """Read one strict version-1 nonempty member→SHA256 manifest; never fallback."""
    try:
        with regular(path) as stream:
            before = identity(os.fstat(stream.fileno()))
            value = json.loads(stream.read(), object_pairs_hook=unique_object)
            if identity(os.fstat(stream.fileno())) != before or identity(Path(path).lstat()) != before:
                raise Refusal('manifest_invalid')
        if (not isinstance(value, dict) or set(value) != {'schema_version', 'members'}
                or type(value['schema_version']) is not int or value['schema_version'] != 1
                or not isinstance(value['members'], dict) or not value['members']):
            raise Refusal('manifest_invalid')
        for name, digest in value['members'].items():
            member_name(name)
            if not isinstance(digest, str) or not re.fullmatch('[0-9a-f]{64}', digest):
                raise Refusal('manifest_invalid')
        return value['members']
    except (OSError, ValueError, Refusal) as error:
        raise Refusal('manifest_invalid') from error


def inspect_archive(parts, expected):
    """Return selected-member results after full header/trailer scan and hash reads.

    Reject missing/nonregular/duplicate selected members and hash mismatches.
    Unselected file payloads are skipped, not authenticated. Exact part order is
    caller input; metadata freshness is not a concurrent-writer exclusion lock.
    """
    rows, errors, seen = [], [], set()
    with SplitArchive(parts) as stream:
        stream.check_stable()
        with tarfile.open(fileobj=stream, mode='r:', stream=True) as tar:
            for member in tar:
                if member.name not in expected:
                    continue
                if member.name in seen:
                    errors.append({'code':'member_duplicate','member':member.name})
                    continue
                seen.add(member.name)
                if not member.isfile():
                    errors.append({'code':'member_non_regular','member':member.name})
                    continue
                with tar.extractfile(member) as payload:
                    actual = hashlib.file_digest(payload, 'sha256').hexdigest()
                rows.append({'member':member.name,'size':member.size,
                             'expected_sha256':expected[member.name],'actual_sha256':actual})
                if actual != expected[member.name]:
                    errors.append({'code':'member_digest','member':member.name})
            # tarfile can treat a corrupt later header as end-of-archive. Require
            # the plain-tar terminator explicitly before claiming a complete scan.
            stream.seek(tar.offset)
            if stream.read(2 * tarfile.BLOCKSIZE) != bytes(2 * tarfile.BLOCKSIZE):
                raise Refusal('archive_invalid')
            # Buffer size is an IO allocation, not a volume/time limit.
            while chunk := stream.read(1024 * 1024):
                if any(chunk):
                    raise Refusal('archive_invalid')
        stream.check_stable()
    errors.extend({'code':'member_missing','member':n} for n in sorted(set(expected) - seen))
    return {'schema_version':1,'status':'FAIL' if errors else 'PASS','members':rows,
            'errors':errors,'scope':'Selected regular member bytes and logical plain-tar framing only; no extraction, author authentication or unselected payload hashing.'}


def main():
    """Expose the read-only CLI: JSON result, exit 0 PASS / 1 failure / argparse 2."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--part', action='append', required=True, type=Path)
    parser.add_argument('--manifest', required=True, type=Path)
    args = parser.parse_args()
    try:
        expected = expected_members(args.manifest)
        try:
            result = inspect_archive(args.part, expected)
        except OSError as error:
            raise Refusal('archive_source') from error
        except tarfile.TarError as error:
            raise Refusal('archive_invalid') from error
    except (Refusal, ValueError, ExceptionGroup) as error:
        result = {'schema_version':1,'status':'FAIL','errors':[{
            'code':getattr(error,'code','archive_invalid'),'cause_type':type(error.__cause__ or error).__name__,
            'errno':getattr(error.__cause__ or error,'errno',None)}], 'members':[]}
    print(json.dumps(result, ensure_ascii=True))
    return 0 if result['status'] == 'PASS' else 1


if __name__ == '__main__':
    sys.exit(main())
