# T5 — Recette navigateur FR/EN x thèmes en CI (REQ-CI-01)

Tâche `plan-843bb3ce22-T5`, work `w-843bb3ce22cff2965c5e77b6`. Scope : brancher
la recette navigateur FR/EN x thèmes clair/sombre existante sur le pipeline CI
(`.github/workflows/ci.yml`), sans changer le périmètre fonctionnel. Dépend de
T1 (accepté, `docs/T1-inventaire-existant.md` / `docs/plan-843bb3ce22-T1.md`).

## 1. Constat repris de T1

`tests/i18n_ui.cjs` (lancé via `npm run test:i18n-ui`) est une recette
Puppeteer déjà existante et déjà réutilisable telle quelle :

- Parcourt le cockpit et `prepare.html` en français puis en anglais
  (`p.select('#swarm-language', ...)`, assertions sur `html[lang]` et les
  libellés traduits — `tests/i18n_ui.cjs:9-11,21-22,37,41-42`).
- Bascule les deux thèmes (`etat` = clair, `sombre` = sombre) à chaque étape
  et capture une paire de captures d'écran par thème
  (`tests/i18n_ui.cjs:16,18,32,35,40-41`).
- Vérifie en plus l'absence d'erreur JS (`p.on('pageerror', ...)`,
  `assert.deepEqual(errors, [])` en fin de script) et l'intégrité des données
  de mission au changement de langue (titres, `snapshot.work`).

Le seul écart identifié par T1 était le branchement CI : `.github/workflows/ci.yml`
(job `checks`) n'exécutait que `npm test`, qui ne couvre pas `test:i18n-ui`.
`test:budgets` / `test:pricing` / `test:quotas` / `test:task-models` /
`test:role-models` restent hors CI (hors périmètre REQ-CI-01, non touchés ici).

## 2. Changement appliqué

Un seul fichier modifié, une seule étape ajoutée en fin du job `checks`,
après `make smoke` (le binaire `bin/swarm` requis par la recette est déjà
construit à ce stade par la cible `smoke: build`) :

```diff
       - run: npm test
       - run: make smoke
+      - run: npm run test:i18n-ui
+        env:
+          CHROME_BIN: /usr/bin/google-chrome
```

Aucune nouvelle dépendance ajoutée : `puppeteer` est déjà en
`devDependencies` (`package.json`), et `PUPPETEER_SKIP_DOWNLOAD: 'true'` est
déjà défini au niveau du job (le script Chrome système est utilisé, pas le
Chromium empaqueté par Puppeteer). Aucune seconde source de vérité créée :
`npm run test:i18n-ui` est l'unique point d'entrée réutilisé tel quel.

## 3. Preuves — cycle vert / rouge bloquant / vert

Toutes les commandes ci-dessous ont été exécutées dans la racine de travail
`/home/fpizzi/workspace/swarm-engine-contract/source`, binaire local
`bin/swarm` reconstruit à chaque étape (`CGO_ENABLED=0 go build -trimpath -o
bin/swarm .`) puisque les assets web sont embarqués au build
(`web_server.go:22 //go:embed web/*`). Logs horodatés dans
`docs/ci-evidence/` (répertoire non suivi par git : `*.log` est exclu par
`.gitignore:13`, comme tout log de ce dépôt — extraits reproduits ci-dessous
pour la traçabilité du rapport).

### 3.1 Vert — baseline, avant tout changement

`docs/ci-evidence/T5-baseline-green-20260929T194312Z.log` :
```
PASS bilingual web UI
```
Code de sortie : 0.

### 3.2 Vert — séquence complète du job CI `checks` reproduite localement

Commandes exécutées dans l'ordre du job `checks` (`go vet ./...`, `npm test`,
`make smoke`, puis la nouvelle étape `npm run test:i18n-ui` avec
`CHROME_BIN=/usr/bin/google-chrome`) — `docs/ci-evidence/T5-local-ci-sequence-green-20260929T194341Z.log` :
```
== go vet ==
== npm test ==
PASS: shared branches, nested folds, reveal, identity-safe preferences
PASS: multiple roots, topology growth and removed node identity
PASS: deterministic reveal, shorter path and removed target
PASS: orchestrator, delegated responsibility, independent reviewer and honest missing-role state
PASS: décisions de pilotage, coordination A8, triplet factuel, diagnostic Q3 et supervision Q5
PASS: visible refresh, hidden-tab suspension, immediate return, timer and listener cleanup
PASS: audit acceptance runner fails closed for DOM, Go, zero tests and unknown cases
PASS i18n locale, fallback, interpolation, engine formats and catalogue parity
== make smoke ==
PASS: fresh processes, duplicate create, active-task warning, 80 columns, portable import
== npm run test:i18n-ui (CI-wired step, CHROME_BIN=/usr/bin/google-chrome) ==
PASS bilingual web UI
```
Code de sortie global : 0.

### 3.3 Rouge bloquant — régression injectée

Régression : dans `web/i18n-en.js:94`, remplacement temporaire de la
traduction `"Ajouter une IA": "Add an AI connection"` par
`"Ajouter une IA": "REGRESSION-T5-INJECTED"`, rebuild du binaire (asset
embarqué), puis `npm run test:i18n-ui` —
`docs/ci-evidence/T5-regression-red-20260929T194420Z.log` :
```
AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:
+ actual - expected

+ 'REGRESSION-T5-INJECTED'
- 'Add an AI connection'

    at .../tests/i18n_ui.cjs:33:168 {
  generatedMessage: true,
  code: 'ERR_ASSERTION',
  actual: 'REGRESSION-T5-INJECTED',
  expected: 'Add an AI connection',
  operator: 'strictEqual',
  diff: 'simple'
}
```
Code de sortie : 1 (`process.exitCode = 1` dans le `.catch` du script). Une
étape `- run: npm run test:i18n-ui` échoue le job GitHub Actions correspondant
(`run:` sans `continue-on-error`) — bloquant par construction du workflow.

### 3.4 Vert — régression retirée

Traduction restaurée à l'identique (`"Ajouter une IA": "Add an AI connection"`),
rebuild, `npm run test:i18n-ui` —
`docs/ci-evidence/T5-regression-removed-green-20260929T194432Z.log` :
```
PASS bilingual web UI
```
Code de sortie : 0. `rg -n "REGRESSION-T5-INJECTED"` sur l'arbre de travail
(hors `docs/ci-evidence/`) ne renvoie plus rien : aucune trace résiduelle.

## 4. Couverture FR/EN x thèmes clair/sombre

Confirmée par lecture directe de `tests/i18n_ui.cjs` (voir §1) : chaque page
visitée (cockpit — vue graphe, modale de connexion — et `prepare.html`) est
capturée dans les 4 combinaisons langue × thème (FR/clair, FR/sombre,
EN/clair, EN/sombre), avec assertions de contenu traduit à chaque bascule de
langue, pas seulement des captures d'écran.

## 5. Exécution GitHub Actions réellement terminée

État courant constaté par le superviseur via gh run view, après la tentative
initiale du worker : le run 36628140996 est terminé avec conclusion success.
La limitation historique « pas de run distant » est levée. Elle est conservée
dans le RETEX et les refus précédents, pas présentée comme l'état courant.

Lien : https://github.com/mo0ogly/swarm/actions/runs/36628140996
Branche : codex/browser-ci-proof. Révision : c5f1dea91310608f1b4b327a0cf6349e1bbc2c55.
Le diff depuis 097e745 porte uniquement sur trois lignes dans le workflow CI.
Cette exécution qualifie le branchement T5 ; elle ne qualifie pas les changements
locaux T2/T3 absents de ce commit, qui ont leurs contrôles séparés.

### Réponse GitHub horodatée, collectée directement

Commande : gh run view 36628140996 --repo mo0ogly/swarm --json status,conclusion,headSha,url,jobs
Code 0. Extrait sans transformation des valeurs :

```json
{
  "headSha": "c5f1dea91310608f1b4b327a0cf6349e1bbc2c55",
  "url": "https://github.com/mo0ogly/swarm/actions/runs/36628140996",
  "status": "completed",
  "conclusion": "success",
  "jobs": [
    {
      "name": "install",
      "conclusion": "success",
      "startedAt": "2026-09-29T20:41:40Z",
      "completedAt": "2026-09-29T20:43:06Z",
      "steps": []
    },
    {
      "name": "checks",
      "conclusion": "success",
      "startedAt": "2026-09-29T20:41:40Z",
      "completedAt": "2026-09-29T20:47:22Z",
      "steps": [
        {
          "completedAt": "2026-09-29T20:47:19Z",
          "conclusion": "success",
          "name": "Run npm run test:i18n-ui",
          "number": 13,
          "startedAt": "2026-09-29T20:47:05Z",
          "status": "completed"
        }
      ]
    }
  ]
}
```

### Journal brut de l'étape navigateur sur le runner GitHub

Commande : gh run view 36628140996 --repo mo0ogly/swarm --log ; code 0.
Les lignes suivantes sont celles du job checks / étape navigateur ; ce ne sont
pas les journaux de la reproduction locale présentée en section 3.

```text
checks	Run npm run test:i18n-ui	﻿2026-09-29T20:47:05.7035034Z ##[group]Run npm run test:i18n-ui
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7035373Z ^[[36;1mnpm run test:i18n-ui^[[0m
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7098211Z shell: /usr/bin/bash -e {0}
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7098484Z env:
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7098705Z   PUPPETEER_SKIP_DOWNLOAD: true
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7098979Z   CHROME_BIN: /usr/bin/google-chrome
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.7099261Z ##[endgroup]
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.8113216Z
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.8113870Z > test:i18n-ui
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.8114941Z > node tests/i18n_ui.cjs bin/swarm test-results/i18n
checks	Run npm run test:i18n-ui	2026-09-29T20:47:05.8115526Z
checks	Run npm run test:i18n-ui	2026-09-29T20:47:19.2155049Z PASS bilingual web UI
```

## 6. Résultat courant des trois critères

- CI verte : jobs checks et install success sur le commit indiqué, avec étape
  npm run test:i18n-ui success et sortie distante PASS bilingual web UI.
- Régression bloquante : cycle rouge/vert local section 3 ; l'étape distante
  est obligatoire et sans continue-on-error. Aucun échec distant volontaire
  n'est revendiqué.
- Couverture : source exacte de la recette ci-dessous ; quatre combinaisons
  langue/thème, assertions de traduction et absence d'erreurs JavaScript.

## 7. Source exacte de la recette exécutée

```javascript
'use strict';
const fs=require('fs'),path=require('path'),os=require('os'),assert=require('assert/strict'),{spawn,execFile}=require('child_process'),{promisify}=require('util'),puppeteer=require('puppeteer');
const exec=promisify(execFile),binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/i18n');fs.mkdirSync(out,{recursive:true});
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-readme-i18n-'));let app,browser,page;
async function cli(args,input){return new Promise((resolve,reject)=>{const p=spawn(binary,['--root',root,'--json',...args,...(input?['--input','-']:[])]);let text='',err='';p.stdout.on('data',x=>text+=x);p.stderr.on('data',x=>err+=x);p.on('exit',code=>code?reject(Error(err||text)):resolve(text?JSON.parse(text):null));p.stdin.end(input?JSON.stringify(input):undefined)})}
(async()=>{
 await cli(['init']);let w=(await cli(['work','create'],{schema_version:1,event_id:'create',expected_revision:0,title:'À vérifier',objective:'Ne pas traduire mon besoin français.',scope:'UI fixture only',criteria:['Conserver mes données'],next:'Enregistrer'})).work;
 for(const [id,depends] of [['source',[]],['verification',['source']]])w=(await cli(['task','add',w.id],{schema_version:1,event_id:id,expected_revision:w.revision,id,title:id==='source'?'Enregistrer':'Résultat français',owner:'fixture',deliverable:'docs/'+id+'.md',criteria:['À vérifier'],depends,next:'Enregistrer'})).work;
 const fr=await exec(binary,['--root',root,'--lang','fr','--json','work','show',w.id]);
 const en=await exec(binary,['--root',root,'--lang','en','--json','work','show',w.id]);
 assert.deepEqual(JSON.parse(en.stdout),JSON.parse(fr.stdout),'CLI JSON is language-independent');
 assert.match((await exec(binary,['--lang','en','help','management'])).stdout,/mission/i);
 await exec('python3',['scripts/readme-team-fixture.py',root]);
 app=spawn(binary,['--root',root,'web','127.0.0.1:0']);const url=await new Promise((resolve,reject)=>{let text='';app.stdout.on('data',x=>{text+=x;const m=text.match(/http:\/\/[^\s]+/);if(m)resolve(m[0])});app.on('exit',code=>reject(Error('server '+code)))});
 browser=await puppeteer.launch({executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',headless:true,args:['--no-sandbox']});const p=page=await browser.newPage(),errors=[];p.on('pageerror',e=>{errors.push(e.message);console.error('page error:',e.message)});p.on('response',async r=>{if(r.url().includes('/api/v1/snapshot')&&r.status()>=400)console.error('snapshot error:',await r.text())});await p.setViewport({width:1440,height:1050});await p.goto(url);await p.waitForSelector('#swarm-language');await p.waitForFunction(()=>document.querySelector('#connection').textContent.includes('Connecté'));
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'graph-fr-'+theme+'.png')})}
 await p.click('[data-view="providers"]');await p.waitForSelector('#connections-add');await p.click('#connections-add');await p.waitForSelector('#modal[open]');assert.equal(await p.$eval('#modal-title',e=>e.textContent),'Ajouter une IA');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'connection-fr-'+theme+'.png')})}
 await p.keyboard.press('Escape');await p.click('[data-view="conduite"]');
 const before=await p.evaluate(()=>JSON.stringify(snapshot.work));
 await Promise.all([p.waitForNavigation(),p.select('#swarm-language','en')]);await p.waitForFunction(()=>document.querySelector('#connection').textContent.includes('Connected'));
 assert.equal(await p.$eval('html',e=>e.lang),'en');
 await p.setRequestInterception(true);p.on('request',r=>{if(r.url().endsWith('/api/v1/i18n-error-fixture'))return r.respond({status:400,contentType:'application/json',body:JSON.stringify({error:'Le serveur a répondu HTTP 401. Vérifiez la clé, le modèle et l’adresse.',failure:{code:'validation'}})});r.continue()});
 const displayedError=await p.evaluate(async()=>{try{await api('/api/v1/i18n-error-fixture')}catch(e){return {message:e.message,code:e.code,status:e.status}}});
 assert.deepEqual(displayedError,{message:'The server returned HTTP 401. Check the key, model and URL.',code:'validation',status:400});
 await p.setRequestInterception(false);p.removeAllListeners('request');
 assert.equal(await p.$eval('[data-view="providers"]',e=>e.textContent),'AI and connections');
 assert.equal(await p.evaluate(()=>JSON.stringify(snapshot.work)),before,'language must not rewrite mission data');
 const titles=await p.evaluate(()=>snapshot.work.tasks.map(t=>t.title));assert.deepEqual(titles,['Enregistrer','Résultat français']);
 assert.match(await p.$eval('.team-role-flow',e=>e.textContent),/production tasks/);assert.doesNotMatch(await p.$eval('.team-role-flow',e=>e.textContent),/décision|Réalise|Examine chaque|appels utilisés/);
 assert.match(await p.$eval('#pilot-canvas',e=>e.textContent),/Orchestrator/);assert.match(await p.$eval('#pilot-canvas',e=>e.textContent),/AI reviewer/);
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'graph-en-'+theme+'.png')});await p.$eval('#pilot-canvas',e=>e.scrollIntoView({block:'center'}));await p.screenshot({path:path.join(out,'roles-en-'+theme+'.png')});await p.evaluate(()=>scrollTo(0,0))}
 await p.click('[data-view="providers"]');await p.waitForSelector('#connections-add');await p.click('#connections-add');await p.waitForSelector('#modal[open]');assert.equal(await p.$eval('#modal-title',e=>e.textContent),'Add an AI connection');
 await p.type('#field-connection_label','À vérifier');assert.equal(await p.$eval('#field-connection_label',e=>e.value),'À vérifier');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>setTheme(t),theme);await p.screenshot({path:path.join(out,'connection-en-'+theme+'.png')})}
 await p.keyboard.press('Escape');await p.click('#help');await p.waitForSelector('#modal[open]');assert.match(await p.$eval('#modal',e=>e.textContent),/Add your model and test the connection/);await p.keyboard.press('Escape');
 await p.goto(new URL('/prepare.html',url).href);await p.waitForFunction(()=>document.documentElement.lang==='en');assert.equal(await p.$eval('#title',e=>e.textContent),'Prepare a project');
 await p.type('#new-need','Mon besoin français reste intact.');await p.select('#swarm-language','fr');assert.equal(await p.$eval('html',e=>e.lang),'en','dirty preparation blocks language navigation');assert.equal(await p.$eval('#new-need',e=>e.value),'Mon besoin français reste intact.');assert.match(await p.$eval('#error',e=>e.textContent),/before changing language/);
 await p.$eval('#new-need',e=>e.value='');
 for(const theme of ['etat','sombre']){await p.evaluate(t=>document.documentElement.dataset.theme=t,theme);await p.screenshot({path:path.join(out,'prepare-en-'+theme+'.png')})}
 await Promise.all([p.waitForNavigation(),p.select('#swarm-language','fr')]);assert.equal(await p.$eval('html',e=>e.lang),'fr');assert.equal(await p.$eval('#title',e=>e.textContent),'Préparer un projet');for(const theme of ['etat','sombre']){await p.evaluate(t=>document.documentElement.dataset.theme=t,theme);await p.screenshot({path:path.join(out,'prepare-fr-'+theme+'.png')})}
 await p.goto(new URL('/?lang=en',url).href);await p.waitForSelector('#swarm-language');assert.equal(await p.$eval('html',e=>e.lang),'en');
 assert.deepEqual(errors,[]);fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',checks:['language switch and persistence','mission titles and payload unchanged','planner worker reviewer graph','English connection modal','help dialog','preparation draft navigation guard','both themes','French restored','no JS errors']},null,2));console.log('PASS bilingual web UI');
})().catch(async e=>{console.error(e);if(page)console.error(await page.evaluate(()=>document.body.innerText));process.exitCode=1}).finally(async()=>{if(browser)await browser.close();if(app)app.kill()});
```

## Export compact du résultat GitHub

Même résultat distant ci-dessus, sérialisé sur une ligne pour une citation contiguë (aucune valeur changée).

```json
{"status": "completed", "conclusion": "success", "headSha": "c5f1dea91310608f1b4b327a0cf6349e1bbc2c55", "url": "https://github.com/mo0ogly/swarm/actions/runs/36628140996"}
```

## Contrôles autorisés exécutables par le moteur — 30 septembre

La politique T5 préautorise six contrôles bornés, 300 secondes cumulées.
Le moteur exécute les commandes avant la revue ; leurs reçus apparaissent
dans engine_controls, distinctement du présent rapport. Seuls ces reçus
attestent leur exécution par le moteur. L’essai supervisé préalable a vérifié
GitHub et réussi le cycle navigateur vert/rouge/vert dans une copie temporaire.

Le contrôle ci interroge le vrai run GitHub 36628140996, son commit exact,
son étape navigateur et son journal. Le contrôle mutation construit le candidat
local en copie temporaire, constate vert, injecte REGRESSION-T5-INJECTED dans
la traduction anglaise, exige une AssertionError et un code non nul, restaure
puis exige vert. Le contrôle browser vérifie aussi les captures FR/EN x
etat/sombre. La preuve distante porte sur le commit CI ; la recette locale
porte sur les sources courantes copiées. Aucun état réel de mission modifié.

Empreinte du script imposée comme argument dans chaque commande :
`e52f1359db10b1e370d5b91b3792f4feadf75fa14d6be54096faa29b43c43a2f`.
Voici le code complet du contrôle, pour examiner ce que son succès démontre :

```python
#!/usr/bin/env python3
"""Bounded, reproducible T5 checks. Never operates on the live Swarm database."""
import hashlib, json, os, pathlib, shutil, subprocess, sys, tempfile
ROOT = pathlib.Path(__file__).resolve().parents[2]
COMMIT = 'c5f1dea91310608f1b4b327a0cf6349e1bbc2c55'
RUN = '36628140996'

def run(args, cwd=ROOT, timeout=90):
    p = subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE,
                       stderr=subprocess.STDOUT, timeout=timeout, env={**os.environ, 'CHROME_BIN':'/usr/bin/google-chrome'})
    return p

def identity():
    for name in ['.github/workflows/ci.yml', 'tests/i18n_ui.cjs', 'package.json', 'package-lock.json']:
        p = subprocess.run(['git','show',COMMIT+':'+name],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True)
        assert p.stdout == (ROOT/name).read_bytes(), 'CI input differs: '+name
    workflow=(ROOT/'.github/workflows/ci.yml').read_text()
    step=workflow.split('- run: npm run test:i18n-ui',1)
    assert len(step)==2 and 'continue-on-error' not in step[1].split('  install:',1)[0]
    print('PASS exact CI inputs and mandatory browser step',COMMIT)

def ci():
    identity()
    p=run(['gh','run','view',RUN,'--repo','mo0ogly/swarm','--json','status,conclusion,headSha,jobs'],timeout=30)
    assert p.returncode==0,'Cannot read GitHub run'
    data=json.loads(p.stdout)
    assert data['status']=='completed' and data['conclusion']=='success' and data['headSha']==COMMIT
    steps=[s for j in data['jobs'] if j['name']=='checks' for s in j['steps'] if s['name']=='Run npm run test:i18n-ui']
    assert len(steps)==1 and steps[0]['conclusion']=='success'
    p=run(['gh','run','view',RUN,'--repo','mo0ogly/swarm','--log'],timeout=30)
    assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout
    print('PASS GitHub run',RUN,COMMIT,'browser step success, runner log confirmed')

def browser(mutate=False):
    # Copy the current candidate, including new Go files; isolate mutations.
    with tempfile.TemporaryDirectory(prefix='swarm-t5-check-') as temp:
        dst=pathlib.Path(temp)
        for p in ROOT.glob('*.go'): shutil.copy2(p,dst/p.name)
        for name in ['go.mod','go.sum','package.json']: shutil.copy2(ROOT/name,dst/name)
        for name in ['web','locales','contracts','scripts','tools/agent-workflows','.claude/skills']:
            shutil.copytree(ROOT/name,dst/name)
        (dst/'node_modules').symlink_to(ROOT/'node_modules',target_is_directory=True)
        (dst/'tests').mkdir();shutil.copy2(ROOT/'tests/i18n_ui.cjs',dst/'tests/i18n_ui.cjs')
        def build():
            p=run(['go','build','-o','swarm','.'],dst)
            assert p.returncode==0,p.stdout[-3000:]
        def check():
            return run(['node','tests/i18n_ui.cjs','swarm','screens'],dst,timeout=60)
        build(); p=check();assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout,p.stdout[-3000:]
        if mutate:
            path=dst/'web/i18n-en.js'; original=path.read_text()
            old='"Ajouter une IA": "Add an AI connection"'
            assert old in original
            path.write_text(original.replace(old,'"Ajouter une IA": "REGRESSION-T5-INJECTED"',1))
            build();p=check()
            assert p.returncode!=0 and 'REGRESSION-T5-INJECTED' in p.stdout and 'AssertionError' in p.stdout, p.stdout[-3000:]
            path.write_text(original);build();p=check()
            assert p.returncode==0 and 'PASS bilingual web UI' in p.stdout,p.stdout[-3000:]
            print('PASS browser green/red/green; injected English-label regression fails assertion; restored candidate passes')
        else:
            for lang in ['fr','en']:
                for theme in ['etat','sombre']:
                    assert (dst/'screens'/f'graph-{lang}-{theme}.png').is_file()
            print('PASS browser FR/EN x etat/sombre; real Chrome; current isolated candidate')

if __name__=='__main__':
    if len(sys.argv)>2:
        assert hashlib.sha256(pathlib.Path(__file__).read_bytes()).hexdigest()==sys.argv[2], 'Authorized check script changed'
    mode=sys.argv[1]
    if mode=='identity': identity()
    elif mode=='ci': ci()
    elif mode=='mutation': browser(True)
    elif mode=='browser': browser()
    else: raise SystemExit('Unknown check')

```
