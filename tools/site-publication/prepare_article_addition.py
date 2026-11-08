"""Prepare one authorised bilingual article route against the observed public release.

Retain and reread a baseline archive on the identified USB, then add only the
configured bilingual route and one homepage card per language. Regenerate the
sitemap from the resulting public home links before binding the delta. Refuse duplicates,
unexpected baseline bytes, missing scenes or unrelated changes. No activation.
"""
from pathlib import Path
import hashlib, json, shutil, subprocess, tarfile, sys
from bs4 import BeautifulSoup
from publication_sitemap import regenerate_sitemap

SPEC_PATH = Path(sys.argv[1]).resolve()
SPEC = json.loads(SPEC_PATH.read_text())
HERE = SPEC_PATH.parent
SITE = HERE.parent.parent


def sha(data):
    """Hash complete bytes; IO errors remain visible at the call site."""
    return hashlib.sha256(data).hexdigest()


def inventory(root):
    """Inventory all regular files and refuse unreviewed symlinks."""
    if any(p.is_symlink() for p in root.rglob('*')):
        raise ValueError('Symlink in staging')
    return {str(p.relative_to(root)):sha(p.read_bytes()) for p in root.rglob('*') if p.is_file()}


def main():
    """Reread the baseline, stage the single addition, and bind its exact delta.

    Extraction accepts regular files/directories only within the caller-owned
    fresh stage. A failure leaves staged evidence and propagates; it never changes
    the public pointer. Existing output/archive refuses rather than overwrite.
    """
    evidence = Path(json.loads((HERE/'evidence-location.json').read_text())['usb'])
    mount = json.loads(subprocess.check_output(['findmnt','-J','-T',str(evidence)]))['filesystems'][0]
    if mount['target'] != '/media/fpizzi/Lexar' or mount['source'] != '/dev/sda1':
        raise ValueError('Identified USB unavailable')
    baseline = json.loads((HERE/'baseline.json').read_text())
    stage = evidence/'stage'
    stage.mkdir(exist_ok=False)
    with tarfile.open(evidence/'baseline-public.tar.gz') as archive:
        for member in archive:
            path = Path(member.name)
            if path.is_absolute() or '..' in path.parts or not (member.isfile() or member.isdir()):
                raise ValueError('Unsafe baseline member')
            target = stage/path
            if member.isdir(): target.mkdir(parents=True, exist_ok=True)
            else:
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(archive.extractfile(member).read())
    if inventory(stage) != baseline['inventory']:
        raise ValueError('Reread baseline differs')
    slug = SPEC['articleSlug']
    if Path(slug).name != slug or not slug.startswith('swarm-'):
        raise ValueError('Invalid article route')
    prefix = 'articles/'+slug+'/'
    target = stage/'articles'/slug
    if target.exists(): raise ValueError('Duplicate article route')
    source = (SITE/SPEC['bundle']).resolve()
    if not source.is_relative_to(SITE.resolve()):
        raise ValueError('Bundle escapes maintained site sources')
    shutil.copytree(source, target)
    for lang in ['fr','en']:
        page = BeautifulSoup((target/f'{lang}.html').read_text(),'html.parser')
        if len(page.select('figure video')) != SPEC['figures'] or len(page.select('h2[id^=scenario-]')) != SPEC['scenarios']:
            raise ValueError('Incomplete article article')
        home = stage/f'{lang}.html'
        original = home.read_text()
        soup = BeautifulSoup(original,'html.parser')
        if soup.select(f'a[href="articles/{slug}/{lang}.html"]'):
            raise ValueError('Duplicate article card')
        card = BeautifulSoup('<article class="article-card"><a><img alt="" loading="lazy" width="1600" height="900"><h3></h3></a><p></p><a class="read-link"></a></article>','html.parser').article
        for a in card.select('a'): a['href'] = prefix+lang+'.html'
        card.img['src'] = prefix+f'assets/couverture-{lang}.png'
        card.h3.string = page.h1.get_text()
        card.p.string = page.figcaption.get_text()
        card.select_one('.read-link').string = 'Lire l’article ↗' if lang=='fr' else 'Read the article ↗'
        opening = '<div class="article-grid">'
        if original.count(opening) != 1: raise ValueError('Ambiguous homepage insertion')
        home.write_text(original.replace(opening,opening+str(card),1))
    (stage/'index.html').write_bytes((stage/'fr.html').read_bytes())
    sitemap = regenerate_sitemap(stage)
    expected = inventory(stage)
    delta = {n:h for n,h in expected.items() if baseline['inventory'].get(n) != h}
    if not set(baseline['inventory']) <= set(expected) or any(n not in ['fr.html','en.html','index.html','sitemap.xml'] and not n.startswith(prefix) for n in delta):
        raise ValueError('Unrelated change')
    archive = evidence/'public-delta.tar.gz'
    with tarfile.open(archive,'x:gz',compresslevel=1) as tar:
        for n in sorted(delta): tar.add(stage/n,arcname=n,recursive=False)
    with tarfile.open(archive) as tar:
        if {m.name:sha(tar.extractfile(m).read()) for m in tar} != delta:
            raise ValueError('Delta reread differs')
    release = SPEC['releaseBase']+'-'+sha(archive.read_bytes())[:12]
    if not release.replace('-', '').isalnum():
        raise ValueError('Invalid release identifier')
    config = {'baseline':baseline,'articleSlug':slug,'releaseId':release,'archive':'/var/www/swarm-agentic/incoming/'+release+'/public-delta.tar.gz','archiveSha256':sha(archive.read_bytes()),'inventory':expected,'deltaInventory':delta,'operation':'add','removedPaths':[],'sitemapGeneratorSha256':sha((Path(__file__).parent/'publication_sitemap.py').read_bytes()),'sitemap':sitemap}
    for name, value in [('public-inventory.json',expected),('delta-inventory.json',delta),('deployment.json',config)]:
        (HERE/name).write_text(json.dumps(value,indent=2)+'\n')
    print(json.dumps({'release':release,'publicFiles':len(expected),'deltaFiles':len(delta),'bytes':archive.stat().st_size}),flush=True)


if __name__ == '__main__':
    main()
