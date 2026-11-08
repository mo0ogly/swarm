"""Install exact article publication tools outside the public release.

Run on the authorized VPS with ARCHIVE SHA256 BASELINE. Holds the existing
publication lock, verifies unchanged public bytes/config/services/LIA witnesses,
extracts only three declared regular files, and runs the installed read-only
sitemap check before selecting the tool version. No content activation, reload,
application or LIA change. Failures preserve the tool candidate and diagnostics.
"""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import urllib.error
import urllib.request


def sha(data):
    """Hash complete supplied bytes without fallback."""
    return hashlib.sha256(data).hexdigest()


def validate_baseline(root, baseline):
    """Refuse public/config/service/witness changes; only perform read operations."""
    current=(root/'current').resolve()
    assert str(current)==baseline['release']
    assert not any(p.is_symlink() for p in current.rglob('*'))
    actual={str(p.relative_to(current)):sha(p.read_bytes()) for p in current.rglob('*') if p.is_file()}
    assert actual==baseline['inventory']
    assert {p:sha(Path(p).read_bytes()) for p in baseline['configHashes']}==baseline['configHashes']
    identity=subprocess.check_output(['systemctl','show','nginx','lia-backend','--property=ActiveState,MainPID,ExecMainStartTimestamp'],text=True)
    assert identity==baseline['services']
    for path,expected in baseline['liaWitnesses'].items():
        try:response=urllib.request.urlopen('https://liascan.tech'+path)
        except urllib.error.HTTPError as error:response=error
        with response:assert {'status':response.code,'sha256':sha(response.read())}==expected


def main():
    """Verify package/source version and read-only live sitemap, then select tools.

    Initial installation refuses an existing tools tree to preserve its owner.
    Any postselection failure removes only our owned new tools pointer, retains
    candidate files and propagates; public content and service state never change.
    """
    if not __debug__:raise RuntimeError('Optimized Python disables installation guards')
    archive=Path(sys.argv[1]);assert sha(archive.read_bytes())==sys.argv[2]
    baseline=json.loads(Path(sys.argv[3]).read_text());root=Path('/var/www/swarm-agentic')
    with (root/'.deployment.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB);validate_baseline(root,baseline)
        base=root/'publication-tools';assert not base.exists(),'Existing tool installation requires explicit review'
        with tarfile.open(archive) as source:
            members=source.getmembers();names={'manifest.json','deploy_article.py','publication_sitemap.py'}
            assert len(members)==len(names) and {m.name for m in members}==names and all(m.isfile() for m in members)
            manifest=json.loads(source.extractfile('manifest.json').read());version=manifest['version']
            assert version.startswith('publication-') and version.replace('-','').isalnum()
            destination=base/'releases'/version;destination.mkdir(parents=True,exist_ok=False)
            for member in members:
                data=source.extractfile(member).read()
                if member.name!='manifest.json':assert sha(data)==manifest['files'][member.name]
                target=destination/member.name;target.write_bytes(data);target.chmod(0o644)
        check=subprocess.run(['python3',str(destination/'deploy_article.py'),'--check-current'],capture_output=True,text=True)
        (base/'check-current.json').write_text(json.dumps({'exit':check.returncode,'stdout':check.stdout,'stderr':check.stderr},indent=2)+'\n');check.check_returncode()
        result=json.loads(check.stdout);validate_baseline(root,baseline)
        (base/'current').symlink_to(destination)
        try:validate_baseline(root,baseline)
        except BaseException:
            (base/'current').unlink();raise
        outcome={'state':'INSTALLED_READONLY_VERIFIED','version':version,'sitemapUrls':result['urls'],'sitemapSha256':result['sha256'],'publicReleaseUnchanged':baseline['release'],'publicFilesUnchanged':len(baseline['inventory']),'configsAndServiceIdentitiesAndLiaWitnessesUnchanged':True,'toolPath':str(base/'current/deploy_article.py'),'reloads':0}
        (base/'installation.json').write_text(json.dumps(outcome,indent=2)+'\n');print(json.dumps(outcome),flush=True)


if __name__=='__main__':main()
