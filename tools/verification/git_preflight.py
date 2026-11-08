#!/usr/bin/env python3
"""Observe local Git baseline before announcing it; never switch, merge or fetch.

Explicit expected branch/tracking/source inputs. Behind/diverged/missing or
unrelated baselines refuse; ahead is reported and allowed when source is present.
Dirty status is reported, never discarded or saved as a backup. Compares local
observations before/after; no exclusion lock or guarantee that a later operation
still sees this state. Tracking freshness against the remote is not established.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys


class Refusal(Exception):
    """Carry a safe stable code plus observed Git exit, without private stderr."""
    def __init__(self, code, exit_code=None):
        """Record failure category; the subprocess result remains its typed cause."""
        super().__init__(code)
        self.code = code
        self.exit_code = exit_code


def git(root, args, allowed=(0,)):
    """Run local read-only plumbing with repository-selector overrides removed.

    No shell, timeout, alias operation, external diff, hook or network command.
    The selected repository must later match show-toplevel. Optional Git locks
    disabled so inspection does not refresh/write the user's index. Nonallowed
    exits preserve CalledProcessError as a cause; stderr is not published because
    caller-controlled paths/configuration may contain private text.
    """
    env = {k:v for k,v in os.environ.items() if not k.startswith('GIT_')}
    env['GIT_OPTIONAL_LOCKS'] = '0'
    argv = ['git','--no-pager','-c','core.fsmonitor=false','-C',str(root),*args]
    result = subprocess.run(argv, env=env, capture_output=True)
    if result.returncode not in allowed:
        error = subprocess.CalledProcessError(result.returncode, argv,
                                             output=result.stdout, stderr=result.stderr)
        raise Refusal('git_command', result.returncode) from error
    return result


def commit(root, ref, code):
    """Resolve exactly the caller's commit/ref, with no fallback or option injection."""
    try:
        return git(root, ['rev-parse','--verify','--end-of-options',ref+'^{commit}']).stdout.strip().decode('ascii')
    except Refusal as error:
        raise Refusal(code, error.exit_code) from error


def observe(root, tracking, source):
    """Read branch/HEAD/tracking/source and dirty status/diff fingerprints.

    Dirty paths are NUL-parsed, including rename source/destination and embedded
    newlines. The diff hash covers tracked HEAD→index/worktree bytes only;
    untracked contents and nested submodule contents are not saved or hashed.
    Observation stability is not a concurrency lock or a backup.
    """
    branch = git(root, ['symbolic-ref','--quiet','--short','HEAD'], allowed=(0,1))
    status = git(root, ['status','--porcelain=v1','-z','--untracked-files=all']).stdout
    diff = git(root, ['diff','--no-ext-diff','--no-textconv','--binary','HEAD','--']).stdout
    paths, entries, i = [], status.split(b'\0'), 0
    while i < len(entries) and entries[i]:
        item = entries[i]
        if len(item) < 4 or item[2:3] != b' ':
            raise Refusal('git_status_format')
        paths.append(os.fsdecode(item[3:]))
        if b'R' in item[:2] or b'C' in item[:2]:
            i += 1
            if i >= len(entries) or not entries[i]:
                raise Refusal('git_status_format')
            paths.append(os.fsdecode(entries[i]))
        i += 1
    return {'branch':branch.stdout.strip().decode('utf-8') if branch.returncode == 0 else None,
            'head':commit(root,'HEAD','head_missing'),
            'tracking':commit(root,tracking,'tracking_missing'),
            'source':commit(root,source,'source_missing'),
            'dirty_paths':paths,'dirty':bool(status),
            'dirty_status_sha256':hashlib.sha256(status).hexdigest(),
            'tracked_diff_sha256':hashlib.sha256(diff).hexdigest()}


def inspect_git(root, expected_branch, tracking_ref, source_ref):
    """Report baseline checks without granting mutation authority or changing refs.

    Require selected physical worktree root, expected branch, source ancestry and
    no missing commits from locally recorded tracking. Local-ahead is explicit,
    not an assertion of publication. Fail any inconsistent before/after sample.
    """
    root = Path(root).resolve(strict=True)
    actual = Path(os.fsdecode(git(root,['rev-parse','--show-toplevel']).stdout.rstrip(b'\n'))).resolve()
    if actual != root:
        raise Refusal('git_root_mismatch')
    before = observe(root, tracking_ref, source_ref)
    counts = git(root,['rev-list','--left-right','--count',before['head']+'...'+before['tracking']]).stdout.split()
    if len(counts) != 2:
        raise Refusal('git_count_format')
    ahead, behind = map(int, counts)
    relation = 'diverged' if ahead and behind else 'ahead' if ahead else 'behind' if behind else 'equal'
    ancestor = git(root,['merge-base','--is-ancestor',before['source'],before['head']],allowed=(0,1)).returncode == 0
    after = observe(root, tracking_ref, source_ref)
    errors = []
    if before != after:
        errors.append({'code':'git_observation_changed'})
    if before['branch'] != expected_branch:
        errors.append({'code':'branch_mismatch'})
    if behind:
        errors.append({'code':'tracking_diverged' if ahead else 'tracking_behind'})
    if not ancestor:
        errors.append({'code':'source_not_in_head'})
    return {'schema_version':1,'status':'FAIL' if errors else 'PASS','root':str(root),
            **before,'tracking_ref':tracking_ref,'source_ref':source_ref,
            'expected_branch':expected_branch,'tracking_relation':relation,'ahead':ahead,'behind':behind,
            'source_in_head':ancestor,'errors':errors,'dirty_contents_saved':False,
            'checkpoint_required_before_mutation':before['dirty'],
            'scope':'Local refs/status/tracked-diff observation only; no fetch, backup, author authentication, lock or mutation permission. Untracked contents not hashed; remote freshness unknown.'}


def main():
    """Expose explicit local Git preflight; JSON exit 0 PASS / 1 failure / usage 2."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', required=True, type=Path)
    parser.add_argument('--expected-branch', required=True)
    parser.add_argument('--tracking-ref', required=True)
    parser.add_argument('--source-ref', required=True)
    args = parser.parse_args()
    try:
        result = inspect_git(args.root, args.expected_branch, args.tracking_ref, args.source_ref)
    except (Refusal, OSError, ValueError) as error:
        result = {'schema_version':1,'status':'FAIL','errors':[{
            'code':getattr(error,'code','git_io'),'exit':getattr(error,'exit_code',None),
            'cause_type':type(error.__cause__ or error).__name__,
            'errno':getattr(error,'errno',None)}]}
    print(json.dumps(result, ensure_ascii=True))
    return 0 if result['status'] == 'PASS' else 1


if __name__ == '__main__':
    sys.exit(main())
