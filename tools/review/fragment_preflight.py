#!/usr/bin/env python3
"""Read-only feasibility check. Does not call a provider or accept any result."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

LIMIT = 192 * 1024
RESERVE = 24 * 1024  # Instructions, response schema and future framing, not proof.


def digest(data):
    return hashlib.sha256(data).hexdigest()


def encode(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':')).encode()


def split_diff(diff):
    if not diff:
        raise ValueError('Empty diff: this tool plans changed-file inspection only')
    if not diff.startswith(b'diff --git '):
        raise ValueError('Unexpected Git diff header')
    sections = [b'diff --git ' + part for part in diff[len(b'diff --git '):].split(b'\ndiff --git ')]
    # split() consumes the line separator; restore every byte exactly.
    sections = [part + b'\n' if i < len(sections)-1 else part for i, part in enumerate(sections)]
    if b''.join(sections) != diff:
        raise ValueError('Diff reconstruction mismatch')
    headers = set()
    for section in sections:
        section.decode('utf-8', errors='strict')
        header = section.split(b'\n', 1)[0]
        if header in headers:
            raise ValueError('Duplicate file section')
        headers.add(header)
        for line in section.splitlines():
            if line.startswith(b'Binary files ') or line == b'GIT binary patch':
                raise ValueError('Binary review is unsupported')
    return sections


def plan(diff, identity, calls_available, final_calls):
    if final_calls < 1 or calls_available <= final_calls:
        raise ValueError('No room for fragment inspection and final review')
    sections = split_diff(diff)
    manifest = [{'index': i, 'header': part.split(b'\n', 1)[0].decode(),
                 'bytes': len(part), 'sha256': digest(part)} for i, part in enumerate(sections)]
    header = {'version': 1, 'kind': 'feasibility_only', 'identity': identity,
              'diff_sha256': digest(diff), 'inventory_sha256': digest(encode(manifest)), 'file_count': len(manifest)}

    def packet(indices):
        return encode({'manifest': header, 'sections': [manifest[i] for i in indices], 'section_indices': indices,
                       'diff': b''.join(sections[i] for i in indices).decode()})

    groups, current = [], []
    for i in range(len(sections)):
        if len(packet(current + [i])) + RESERVE > LIMIT:
            if not current:
                raise ValueError('One file exceeds fragment capacity; no truncation')
            groups.append(current)
            current = []
        current.append(i)
        if len(packet(current)) + RESERVE > LIMIT:
            raise ValueError('One file exceeds fragment capacity; no truncation')
    if current:
        groups.append(current)
    if len(groups) + final_calls > calls_available:
        raise ValueError(f'Insufficient budget: {len(groups)} fragments + {final_calls} final calls > {calls_available}')
    packets = [packet(ids) for ids in groups]
    result = {'schema_version': 1, 'status': 'FEASIBLE_DIFF_ONLY',
              'identity': identity, 'diff_bytes': len(diff), 'diff_sha256': digest(diff),
              'files': len(sections), 'inventory': manifest, 'inspection_calls': len(groups),
              'reserved_final_calls': final_calls, 'available_calls': calls_available,
              'prompt_limit': LIMIT, 'reserved_prompt_bytes': RESERVE,
              'fragments': [{'index': i, 'files': ids, 'packet_bytes': len(packets[i]),
                             'packet_sha256': digest(packets[i])} for i, ids in enumerate(groups)],
              'limitations': ['No provider called; no verdict produced.',
                 'Reports, supplemental sources, final review and cross-file reasoning still need a complete runtime protocol.',
                 'The reserved final calls are a planning allowance, not proof that final review fits.',
                 'This tool cannot authorize publication or replace an engine review.']}
    validate(diff, result, packets)
    return result, packets


def validate(diff, result, packets):
    if len(packets) != len(result['fragments']):
        raise ValueError('Missing packet')
    rebuilt, seen, manifest = [], [], None
    for info, raw in zip(result['fragments'], packets):
        if digest(raw) != info['packet_sha256'] or len(raw) != info['packet_bytes'] or len(raw)+RESERVE > LIMIT:
            raise ValueError('Packet changed or oversized')
        data = json.loads(raw)
        if manifest is None:
            manifest = data['manifest']
        if data['manifest'] != manifest or data['manifest']['identity'] != result['identity'] or manifest['inventory_sha256'] != digest(encode(result['inventory'])):
            raise ValueError('Manifest or candidate changed')
        indices = data['section_indices']
        if indices != info['files'] or data['sections'] != [result['inventory'][i] for i in indices]:
            raise ValueError('Section assignment changed')
        parts = split_diff(data['diff'].encode())
        if len(parts) != len(indices):
            raise ValueError('Section coverage mismatch')
        for i, part in zip(indices, parts):
            entry = result['inventory'][i]
            if digest(part) != entry['sha256'] or len(part) != entry['bytes']:
                raise ValueError('Section changed')
        seen.extend(indices)
        rebuilt.append(data['diff'].encode())
    if seen != list(range(result['files'])) or b''.join(rebuilt) != diff or digest(diff) != result['diff_sha256']:
        raise ValueError('Incomplete, reordered or duplicate coverage')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--git-dir', required=True)
    parser.add_argument('--base', required=True)
    parser.add_argument('--candidate', required=True)
    parser.add_argument('--calls-available', required=True, type=int)
    parser.add_argument('--reserve-final-calls', required=True, type=int)
    parser.add_argument('--output', required=True)
    args = parser.parse_args()
    def git(*parts):
        return subprocess.check_output(['git', '--git-dir', args.git_dir, *parts])
    # Resolve revisions to immutable identities before reading the diff.
    base = git('rev-parse', '--verify', args.base+'^{commit}').decode().strip()
    candidate = git('rev-parse', '--verify', args.candidate+'^{commit}').decode().strip()
    git('merge-base', '--is-ancestor', base, candidate)
    diff = git('diff', '--no-ext-diff', '--no-textconv', '--full-index', '--unified=3', base, candidate, '--')
    result, packets = plan(diff, {'base': base, 'candidate': candidate}, args.calls_available, args.reserve_final_calls)
    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=False)
    for i, raw in enumerate(packets):
        (out/f'fragment-{i+1:02}.json').write_bytes(raw)
    (out/'plan.json').write_text(json.dumps(result, ensure_ascii=False, indent=2)+'\n')
    print(json.dumps(result, ensure_ascii=False))


if __name__ == '__main__':
    main()
