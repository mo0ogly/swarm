# plan-843bb3ce22-T4 — Web Administration (lecture/édition, preview, historique, rollback)

État : LIVRABLE PRODUIT, critères vérifiés cette tentative (2026-09-30). Pas
d'acceptation moteur déclarée ici ; contrôle personnel du worker (DO/CHECK).

## Identité (cette tentative)

Mission w-843bb3ce22cff2965c5e77b6 / tâche plan-843bb3ce22-T4 /
agent 07be2e8a-6306-47ae-9d2e-388230685a80 / tentative a-6d399840bdcc9aaea0fe857d /
départ 4/3 / révision du travail 180. Répertoire vérifié :
`/home/fpizzi/workspace/swarm-engine-contract/source`
(== `git rev-parse --show-toplevel`). Branche `codex/engine-review-contract`.
HEAD `097e745a9b1404f16ed6fdf4ad32124a3f661e05`, diff local non commité présent
(89 fichiers modifiés/non suivis, autres tâches du plan incluses).

Tentative précédente `1d0f0b55-1f50-4020-b5b7-050ddf4f3226` (a-3f933b5720e170f3af506a4f) :
`interrupted`, 61 appels d'outils, exploration seule (aucun fichier applicatif modifié).

## Critères (req-14..17)

- [x] req-14 Aucun secret affiché à l'écran (vérifié — scan DOM automatisé, FR+EN)
- [x] req-15 Parcours FR/EN x thèmes clair/sombre couverts (8 captures, 0 erreur console)
- [x] req-16 Navigation clavier et états d'erreur/vide fonctionnels (Échap+focus, état vide, révision périmée refusée)
- [x] req-17 Preview, validation moteur, historique et rollback opérationnels (aperçu sans écriture, apply, historique, rollback, effectif REQ-ADM-05)

## Constat au démarrage de cette tentative

L'ancien rapport (départ 2/3) s'arrêtait avant la construction de l'écran web.
L'inspection de l'arbre de travail a montré que l'écran existe déjà (fichiers non
commités d'une tentative intermédiaire non documentée) : `web/admin.js` (150 l.),
`run_limits_web.go` (175 l.), `run_limits_web_test.go` (190 l.),
`tests/run_limits_admin_ui.cjs` (146 l.), câblage dans `web/index.html`
(section `#admin`, bouton `data-view="admin"`) et `web/cockpit.js`
(`showView` appelle `loadRunLimitsAdmin`), script npm `test:admin` déjà déclaré.
Aucune de ces réalisations n'a été refaite ; seuls les contrôles ont été rejoués.

## Preuves rejouées cette tentative (commandes et codes de sortie réels)

```
go build ./...                          → exit 0
go test -run TestRunLimits ./... -v     → 12/12 PASS, exit 0
node tests/i18n_test.cjs                → PASS catalogue parity, exit 0
go build -o bin/swarm .                 → exit 0
npm run test:admin                      → status PASS, 20 checks, errors: [], exit 0
python3 tools/verification/t4_admin.py all → PASS T4 control all, exit 0
```

Artefacts : `test-results/admin/result.json` (`status: PASS`, `errors: []`,
racine isolée `/tmp/swarm-run-limits-ui-Si8oCi` puis `/tmp/swarm-t4-control-*`
pour le script de contrôle) et 8 PNG FR/EN × {état, sombre} ×
{admin, observation}.

## Réutilisation (REQ-ADM-03)

`run_limits_web.go` est un adaptateur HTTP pur : `preview` n'écrit rien et
réutilise les validateurs de `configureRunLimits` (T2) ; `apply`/`rollback`
délèguent respectivement à `s.configureRunLimits`/`s.rollbackRunLimits` (T2,
non modifiés par T4). Aucune ligne de `run_limits_admin.go` (T3) ni
`run_limits.go` (T2) modifiée par cette tâche.

## Livrables

- `docs/T4-web-admin.md` (rapport détaillé par critère, preuves datées)
- `tools/verification/t4_admin.py` (script de contrôle reproductible, base
  isolée, modes `entry|unit|browser|validation|delivery|all`, vérifié PASS
  cette tentative)

## Limites

- Contrôle personnel du worker, pas une revue indépendante.
- `run_limits.go` modifié par une tâche tierce (T2) au moment du contrôle ;
  T4 s'appuie dessus en lecture seule.
- Aucune acceptation moteur déclarée ; aucune modification de la base Swarm.

## Prochaine action

Aucune lacune sur req-14..req-17. Transmettre au responsable pour revue
indépendante de la révision candidate (diff non commité inclus), en
coordination avec T2/T3/T5 dont T4 dépend.

## Contrôles exécutables préautorisés depuis le web

Le vérificateur doit s’appuyer sur les reçus engine_controls de cette
tentative. La politique exécute entry, browser, validation (critères 2/3),
unit (critère 4), validation et delivery ; aucun nouveau worker requis.
La recette browser exerce les quatre critères ; les contrôles validation
corroborent sa fraîcheur, ils ne rejouent pas à eux seuls les interactions.
La recherche de champs secrets ne garantit pas l’absence de toute donnée
sensible dans tous les états futurs : elle couvre le scénario testé.
Voici le script exact expliquant la portée de ces commandes :

```python
#!/usr/bin/env python3
"""Execute T4 checks on a temporary binary and an isolated browser fixture."""
import hashlib
import pathlib
import subprocess
import tempfile
import sys
import json

ROOT = pathlib.Path(__file__).resolve().parents[2]
FILES = ('run_limits_web.go', 'run_limits_web_test.go', 'web/admin.js',
         'web/cockpit.js', 'web/index.html', 'tests/run_limits_admin_ui.cjs')

def run(command):
    print('COMMAND', command, flush=True)
    subprocess.run(command, cwd=ROOT, check=True, timeout=150)

mode = sys.argv[1] if len(sys.argv) > 1 else 'all'
fingerprints = {name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in FILES}
out = ROOT / 'test-results/admin'

if mode in ('entry', 'all'):
    for name in FILES:
        assert (ROOT / name).is_file(), name
    assert (ROOT / 'node_modules/puppeteer').is_dir()
    print('PASS prerequisites: source, tests and browser dependency present')

if mode in ('unit', 'all'):
    run(['go', 'build', './...'])
    run(['go', 'test', '-run', 'TestRunLimits', './...', '-count=1', '-v'])
    run(['node', 'tests/i18n_test.cjs'])
    print('PASS go build, targeted run-limits tests and i18n catalogue parity')

if mode in ('browser', 'all'):
    with tempfile.TemporaryDirectory(prefix='swarm-t4-control-') as directory:
        binary = str(pathlib.Path(directory) / 'swarm')
        run(['go', 'build', '-o', binary, '.'])
        run(['node', 'tests/run_limits_admin_ui.cjs', binary, str(out)])
    (out / 'sources.json').write_text(json.dumps(fingerprints, sort_keys=True))

if mode in ('validation', 'delivery', 'all'):
    assert json.loads((out / 'sources.json').read_text()) == fingerprints, 'stale browser evidence'
    result = json.loads((out / 'result.json').read_text())
    assert result['status'] == 'PASS' and not result['errors']
    for prefix in ('admin', 'observation'):
        for lang in ('fr', 'en'):
            for theme in ('etat', 'sombre'):
                assert (out / f'{prefix}-{lang}-{theme}.png').stat().st_size > 1000
    print('PASS current browser artifacts, no JS errors, eight screenshots (admin + observation, FR/EN x 2 themes)')

if mode in ('delivery', 'all'):
    report = (ROOT / 'docs/T4-web-admin.md').read_text()
    for req in ('req-14', 'req-15', 'req-16', 'req-17'):
        assert req in report
    print('PASS report covers req-14..req-17; independent review remains separate')

assert mode in ('entry', 'unit', 'browser', 'validation', 'delivery', 'all')
print('PASS T4 control', mode, flush=True)

```

## Assertions navigateur exactes pour la revue indépendante

Fichier tests/run_limits_admin_ui.cjs ; SHA-256 594c299f85423b9d9f5d4ddf96fadb71e334e001924ebc94272010637b5aca06.
Le contrôle browser exécute ce fichier sur le candidat compilé.

```javascript
'use strict';
// End-to-end recette for the run-limits Administration screen (REQ-ADM-01..07,
// req-14..17 of plan-843bb3ce22-T4): read/edit, preview (no write), engine
// validation, history and rollback, in an isolated store, FR/EN x 2 themes.
// Each language exercises its own "role" scope (same work, distinct scope_key)
// so the two iterations never share mutable state.
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn}=require('child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]),out=path.resolve(process.argv[3]),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-run-limits-ui-'));fs.mkdirSync(out,{recursive:true});let app,browser;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',c=>c?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
async function fillEffective(page,values){for(const [k,v] of Object.entries(values))await page.$eval('#admin-eff-'+k,(e,v)=>e.value=v,v)}
(async()=>{
 await cli(['init']);
 const w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'Admin run-limits test',objective:'Verify run-limits admin screen',scope:'isolated',criteria:['No secret shown'],next:'Configure'})).work;
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',c=>reject(Error('server '+c)))});
 browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});const checks=[],errors=[];
 try{
 for(const lang of ['fr','en']){
  const key='qa-'+lang;
  const show=()=>cli(['run-limits','show','role',w.id,key]);
  const p=await browser.newPage();p.setDefaultTimeout(30000);p.on('pageerror',e=>errors.push(lang+': '+e.message));await p.setViewport({width:1400,height:1000});
  const u=new URL(url);u.searchParams.set('work',w.id);u.searchParams.set('lang',lang);await p.goto(u.href);await p.waitForFunction(()=>typeof snapshot!=='undefined' && snapshot?.work);
  if(await p.$eval('[data-view="admin"]',e=>e.getClientRects().length===0))await p.click('#mode');
  await p.click('[data-view="admin"]');
  await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);

  await p.select('#admin-scope','role');
  await p.$eval('#admin-mission',(e,v)=>{e.value=v},w.id);
  await p.$eval('#admin-key',(e,v)=>{e.value=v},key);
  await p.click('#admin-load');
  await p.waitForFunction(()=>document.querySelector('#admin-current').textContent.length>0);

  const currentText=await p.$eval('#admin-current',e=>e.textContent);
  assert(/Aucun réglage|No setting/.test(currentText),currentText);
  const historyText=await p.$eval('#admin-history',e=>e.textContent);
  assert(/Aucun historique|No history/.test(historyText),historyText);
  checks.push(lang+': état vide portée rôle "'+key+'" (aucune valeur, aucun historique)');

  const adminText=await p.$eval('#admin',e=>e.textContent);
  const scanText=adminText.replace(/Aucun secret n.est affiché sur cet écran\.?|No secret is shown on this screen\.?/gi,'');
  assert(!/api[_-]?key|bearer |secret|password|mot de passe/i.test(scanText),'possible secret on admin screen: '+lang+' :: '+scanText);
  checks.push(lang+': aucun secret affiché à l’écran');

  const fill=async values=>{for(const [k,v] of Object.entries(values))await p.$eval('#field-'+k,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},v)};

  let openBtn=await p.$('#admin-current button');
  await openBtn.click();
  await p.waitForSelector('#field-silence_seconds');
  await fill({silence_seconds:'30',tool_seconds:'120',max_tool_calls:'50',max_repeated_calls:'3',max_consecutive_errors:'2',reason:'ui recette'});
  const before=await show();
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
  const previewText=await p.$eval('#preview',e=>e.textContent);
  assert(/VALEURS ACTUELLES|CURRENT VALUES/.test(previewText),previewText);
  assert.equal((await show()).revision,before.revision);
  checks.push(lang+': aperçu affiché sans écriture (validation moteur avant confirmation)');
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'admin-'+lang+'-'+theme+'.png')})}

  await p.keyboard.press('Escape');
  await p.waitForFunction(()=>!document.querySelector('#modal').open);
  assert.equal(await p.evaluate(b=>b===document.activeElement,openBtn),true);
  assert.equal((await show()).revision,before.revision);
  checks.push(lang+': Échap ferme sans écrire et restitue le focus au bouton d’origine');

  openBtn=await p.$('#admin-current button');
  await openBtn.click();
  await p.waitForSelector('#field-silence_seconds');
  await fill({silence_seconds:'30',tool_seconds:'120',max_tool_calls:'50',max_repeated_calls:'3',max_consecutive_errors:'2',reason:'ui recette'});
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#modal').open);
  const afterApply=await show();
  assert.equal(afterApply.revision,before.revision+1);
  assert.equal(afterApply.values.silence_seconds,30);
  checks.push(lang+': configuration enregistrée en révision '+afterApply.revision+' (prochains départs uniquement)');

  openBtn=await p.$('#admin-current button');
  await openBtn.click();
  await p.waitForSelector('#field-silence_seconds');
  const outside=await cli(['run-limits','apply','role',w.id,key],{schema_version:1,event_id:'outside-'+lang,scope:'role',mission_id:w.id,scope_key:key,expected_revision:afterApply.revision,values:{silence_seconds:99,tool_seconds:99,max_tool_calls:99,max_repeated_calls:9,max_consecutive_errors:9},reason:'external concurrent change'});
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#modal-error').hidden);
  const errText=await p.$eval('#modal-error',e=>e.textContent);
  assert(/a changé depuis le chargement|changed since it was loaded/.test(errText),errText);
  checks.push(lang+': révision périmée détectée et refusée (aucune écriture silencieuse)');
  await p.click('#cancel');
  await p.waitForFunction(()=>!document.querySelector('#modal').open);

  await p.click('#admin-load');
  await p.waitForFunction(rev=>document.querySelector('#admin-current').textContent.includes('Révision '+rev)||document.querySelector('#admin-current').textContent.includes('Revision '+rev),{},outside.revision);
  checks.push(lang+': rechargement de la portée reflète le changement externe (révision '+outside.revision+')');

  const rows=await p.$$('#admin-history tbody tr');
  assert(rows.length>=2,'history rows: '+rows.length);
  let revertBtn=null;
  for(const row of rows){const t=await row.$eval('td:first-child',e=>e.textContent);if(t.trim().startsWith('1')){revertBtn=await row.$('button');break}}
  assert(revertBtn,'revision 1 row not found');
  await revertBtn.click();
  await p.waitForSelector('#field-reason');
  const rollbackPreview=await p.$eval('#preview',e=>e.textContent);
  assert(/VALEURS DE LA RÉVISION 1|VALUES OF REVISION 1/.test(rollbackPreview),rollbackPreview);
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#modal').open);
  const afterRollback=await show();
  assert.equal(afterRollback.values.silence_seconds,30);
  assert.equal(afterRollback.revision,outside.revision+1);
  checks.push(lang+': retour à la révision 1 confirmé, historique conservé (révision '+afterRollback.revision+')');

  await p.click('#admin summary');
  await fillEffective(p,{mission:w.id,role:key,task:'plan-demo-'+lang});
  await p.click('#admin-effective-form button');
  await p.waitForFunction(()=>!document.querySelector('#admin-effective-output').hidden);
  const effText=await p.$eval('#admin-effective-output',e=>e.textContent);
  assert(effText.includes('30'),effText);
  checks.push(lang+': effectif d’un futur départ résolu pour mission/rôle/tâche (REQ-ADM-05)');
  await p.select('#admin-scope','mission');
  await p.$eval('#admin-mission',(e,v)=>{e.value=v},w.id);
  await p.$eval('#admin-key',e=>{e.value=''});
  await p.click('#admin-load');
  await p.waitForFunction(()=>runLimitsAdmin?.entry?.scope==='mission');
  await p.click('#admin-current button');
  await p.waitForSelector('#field-observation_mode');
  await p.select('#field-observation_mode','1');
  await fill({reason:'operator requests observation for this mission'});
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
  assert.match(await p.$eval('#preview',e=>e.textContent),lang==='fr'?/Observation — sans plafonds/:/Observation — no execution caps/);
  for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'observation-'+lang+'-'+theme+'.png')})}
  await p.click('#confirm');
  await p.waitForFunction(()=>!document.querySelector('#modal').open);
  assert.equal((await cli(['run-limits','show','mission',w.id,'-'])).values.observation_mode,1);
  await p.click('#admin-current button');
  await p.waitForSelector('#field-observation_mode');
  await p.select('#field-observation_mode','0');
  await fill({reason:'end observation experiment'});
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#preview').hidden);
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
  assert.equal((await cli(['run-limits','show','mission',w.id,'-'])).values.observation_mode||0,0);
  checks.push(lang+': observation enabled and disabled through preview and confirmation; both themes captured');

 }
 assert.deepEqual(errors,[]);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks,errors,at:new Date().toISOString(),root},null,2));
 console.log(checks.join('\n'));
 }finally{await browser.close();app.kill()}
})().catch(e=>{console.error(e);process.exitCode=1}).finally(()=>{try{app&&app.kill()}catch{}});

```
