#!/usr/bin/env python3
"""Move an idle owned Cargo target by rename, never by recursive copy fallback.

Caller must own both paths/parents exclusively and stop its Cargo writers first.
This is cache management, not worktree/data/evidence cleanup or authorization.
"""
from pathlib import Path
import argparse,json,os


def move_target(source, destination):
    """Create missing parent then rename once, preserving target directory inode.

    Existing destination and symlink endpoints refuse. Cross-device/IO errors
    propagate without copying/deleting source; newly created empty parents may
    remain. Exclusive caller ownership is required: this is not a race-free
    rename-noreplace API or writer lock. A failed post-check may follow an applied
    rename; inspect both paths before recovery rather than retrying blindly.
    Returns observed device/inode, not compiler or content qualification.
    """
    source=Path(source).absolute();destination=Path(destination).absolute()
    if source.is_symlink() or not source.is_dir():raise ValueError('source must be an owned real idle directory')
    if destination.is_symlink() or destination.exists():raise FileExistsError(destination)
    before=source.stat();destination.parent.mkdir(parents=True,exist_ok=True)
    os.rename(source,destination)
    after=destination.stat()
    if (before.st_dev,before.st_ino)!=(after.st_dev,after.st_ino) or source.exists():
        raise RuntimeError('rename outcome requires inspection; destination may already be published')
    return dict(device=after.st_dev,inode=after.st_ino,copied=False)


def main():
    """Move explicitly supplied owned cache paths; errors remain nonzero."""
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('--source',type=Path,required=True);p.add_argument('--destination',type=Path,required=True);a=p.parse_args()
    print(json.dumps(move_target(a.source,a.destination)))

if __name__=='__main__':main()
