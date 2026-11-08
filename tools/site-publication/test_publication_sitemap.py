"""Exercise the real article publisher in isolated roots with deterministic IO.

Network/service doubles are explicit boundaries; the real candidate/archive,
filesystem, generation, activation pointer and rollback code run unchanged.
No production article is created or retired by these tests.
"""
import importlib.util
import json
from pathlib import Path
import shutil
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from publication_sitemap import ORIGIN, ROOT_PAIRS, regenerate_sitemap, verify_sitemap

HERE=Path(__file__).parent
spec=importlib.util.spec_from_file_location('article_deployer',HERE/'deploy_article.py')
publisher=importlib.util.module_from_spec(spec);spec.loader.exec_module(publisher)


class PublicationTests(unittest.TestCase):
    """Check publication success controls before faults and explicit retirement."""
    def setUp(self):
        """Create a private test release with all presentation routes and one pair."""
        self.temp=tempfile.TemporaryDirectory();self.addCleanup(self.temp.cleanup)
        self.root=Path(self.temp.name);self.old=self.root/'releases/old';self.old.mkdir(parents=True)
        (self.root/'current').symlink_to(self.old)
        self.config_file=self.root/'nginx-config';self.config_file.write_bytes(b'unchanged config')
        for pair in ROOT_PAIRS:
            for name,lang in zip(pair,('fr','en')):self.page(self.old,name,lang)
        self.article(self.old,'swarm-a');self.homes(self.old,['swarm-a'])
        regenerate_sitemap(self.old)
        self.base={'release':str(self.old),'inventory':publisher.inventory(self.old),'services':'services','configHashes':{str(self.config_file):publisher.digest(self.config_file.read_bytes())},'liaWitnesses':{}}

    def page(self,root,name,lang,links=''):
        """Write a valid local public page with an exact canonical and language."""
        p=root/name;p.parent.mkdir(parents=True,exist_ok=True)
        p.write_text(f'<html lang="{lang}"><head><link rel="canonical" href="{ORIGIN+name}"></head><body>{links}<p>Fixture</p></body></html>')

    def article(self,root,slug):
        """Create the two explicit public files for a fixture article."""
        for lang in ('fr','en'):self.page(root,f'articles/{slug}/{lang}.html',lang)

    def homes(self,root,slugs):
        """Make public publication membership explicit through both home catalogs."""
        for lang in ('fr','en'):
            links=''.join(f'<a href="articles/{slug}/{lang}.html">Article</a>' for slug in slugs)
            self.page(root,lang+'.html',lang,links)
        (root/'index.html').write_bytes((root/'fr.html').read_bytes())

    def prepare(self,operation='add',stale=False):
        """Prepare a real scoped delta, optionally retaining an obsolete sitemap."""
        candidate=self.root/'prepared';shutil.copytree(self.old,candidate)
        if operation=='add':self.article(candidate,'swarm-b');self.homes(candidate,['swarm-a','swarm-b']);slug='swarm-b'
        elif operation=='remove':shutil.rmtree(candidate/'articles/swarm-a');self.homes(candidate,[]);slug='swarm-a'
        else:slug='swarm-a'
        if not stale:regenerate_sitemap(candidate)
        expected=publisher.inventory(candidate);delta={n:h for n,h in expected.items() if h!=self.base['inventory'].get(n)}
        archive=self.root/'delta.tar'
        with tarfile.open(archive,'w') as a:
            for name in delta:a.add(candidate/name,arcname=name)
        config={'baseline':self.base,'articleSlug':slug,'releaseId':'new','archive':str(archive),'archiveSha256':publisher.digest(archive.read_bytes()),'inventory':expected,'deltaInventory':delta,'operation':operation,'removedPaths':sorted(set(self.base['inventory'])-set(expected)),'sitemapGeneratorSha256':publisher.digest(Path(sys.modules['publication_sitemap'].__file__).read_bytes())}
        path=self.root/'deployment.json';path.write_text(json.dumps(config));return path,config

    def run_publisher(self,path,fail_public=False,writer_failure=False):
        """Run the real publisher; make only service/network/permission IO explicit doubles."""
        def fetch(url):
            name=url.removeprefix(ORIGIN)
            data=(self.root/'current'/name).read_bytes()
            if fail_public and name=='sitemap.xml' and (self.root/'current').resolve()!=self.old:data=b'wrong public sitemap'
            return 200,data
        with patch.object(publisher,'ROOT',self.root),patch.object(publisher,'service_identity',return_value='services'),patch.object(publisher,'check_lia',return_value={}),patch.object(publisher,'fetch',side_effect=fetch),patch.object(publisher.subprocess,'run'):
            if writer_failure:
                with patch.object(publisher.publication_sitemap,'regenerate_sitemap',side_effect=OSError('sitemap write failed')):publisher.main(path)
            else:publisher.main(path)

    def test_add_and_private_exclusion(self):
        """Successful publication includes both added languages but never private files."""
        (self.old/'private').mkdir();(self.old/'private/secret.html').write_text('private')
        self.base['inventory']=publisher.inventory(self.old)
        path,_=self.prepare();self.run_publisher(path)
        result=verify_sitemap(self.root/'current');self.assertEqual(result['urls'],10)
        self.assertFalse(any('private' in p for p in result['pages']))

    def test_retirement(self):
        """Explicit full article removal regenerates XML without the retired URLs."""
        path,_=self.prepare('remove');self.run_publisher(path)
        self.assertEqual(verify_sitemap(self.root/'current')['urls'],6)
        self.assertFalse((self.root/'current/articles/swarm-a').exists())

    def test_stale_sitemap_blocks_activation_and_recovery(self):
        """A forgotten preparer sitemap fails even though deployment regenerates it."""
        path,_=self.prepare(stale=True)
        with self.assertRaisesRegex(AssertionError,'Generated sitemap'):self.run_publisher(path)
        self.assertEqual((self.root/'current').resolve(),self.old)
        self.assertEqual(verify_sitemap(self.old)['urls'],8)

    def test_generation_error_blocks_activation(self):
        """A generator/IO error propagates without selecting the candidate."""
        path,_=self.prepare()
        with self.assertRaisesRegex(OSError,'sitemap write failed'):self.run_publisher(path,writer_failure=True)
        self.assertEqual((self.root/'current').resolve(),self.old)

    def test_public_mismatch_rolls_back(self):
        """A real publication switch is undone when the public sitemap differs."""
        path,_=self.prepare()
        with self.assertRaisesRegex(AssertionError,'Public file mismatch'):self.run_publisher(path,fail_public=True)
        self.assertEqual((self.root/'current').resolve(),self.old)
        self.assertTrue(json.loads((self.root/'deployments/new/failure.json').read_text())['rollbackVerified'])

    def test_missing_generator_and_unacknowledged_retirement_refuse(self):
        """Missing generator identity and unacknowledged removals cannot activate."""
        path,config=self.prepare('remove')
        config.pop('sitemapGeneratorSha256');path.write_text(json.dumps(config))
        with self.assertRaisesRegex(ValueError,'required sitemap generator'):self.run_publisher(path)
        self.assertFalse((self.root/'releases/new').exists())
        config['sitemapGeneratorSha256']=publisher.digest(Path(sys.modules['publication_sitemap'].__file__).read_bytes())
        config['removedPaths']=[];path.write_text(json.dumps(config))
        with self.assertRaisesRegex(AssertionError,'Unacknowledged removal'):self.run_publisher(path)
        self.assertEqual((self.root/'current').resolve(),self.old)

    def test_language_and_noindex_refuse(self):
        """Missing language or explicit noindex cannot silently enter the sitemap."""
        self.homes(self.old,['swarm-a','swarm-b']);self.article(self.old,'swarm-b')
        (self.old/'articles/swarm-b/en.html').unlink()
        with self.assertRaisesRegex(ValueError,'Missing or unsafe'):regenerate_sitemap(self.old)
        self.article(self.old,'swarm-b');p=self.old/'articles/swarm-b/fr.html';p.write_text(p.read_text().replace('</head>','<meta name="robots" content="noindex"></head>'))
        with self.assertRaisesRegex(ValueError,'noindex'):regenerate_sitemap(self.old)

    def test_generator_write_refusal_preserves_old(self):
        """A conflicting temporary file refuses the write and preserves prior XML."""
        previous=(self.old/'sitemap.xml').read_bytes();self.article(self.old,'swarm-b');self.homes(self.old,['swarm-a','swarm-b'])
        temp=self.old/'sitemap.xml.publication-next';temp.write_bytes(b'original failed temporary')
        with self.assertRaises(FileExistsError):regenerate_sitemap(self.old)
        self.assertEqual((self.old/'sitemap.xml').read_bytes(),previous)
        temp.unlink();self.assertEqual(regenerate_sitemap(self.old)['urls'],10)

    def test_legacy_publisher_reproduces_rejected_sitemap_delta(self):
        """The retained original publisher rejects the exact valid sitemap addition."""
        path,_=self.prepare();original=HERE/'tests/fixtures/legacy_deploy.py'
        legacy_spec=importlib.util.spec_from_loader('legacy',loader=None);legacy=importlib.util.module_from_spec(legacy_spec);exec(compile(original.read_bytes(),str(original),'exec'),legacy.__dict__)
        def paths(value):return self.root if str(value)=='/var/www/swarm-agentic' else Path(value)
        with patch.object(legacy,'Path',side_effect=paths),patch.object(legacy,'service_identity',return_value='services'),patch.object(legacy,'check_lia',return_value={}),patch.object(sys,'argv',['legacy',str(path)]):
            with self.assertRaisesRegex(AssertionError,'Unrelated change refused'):legacy.main()
        self.assertEqual((self.root/'current').resolve(),self.old)


if __name__=='__main__':unittest.main(verbosity=2)
