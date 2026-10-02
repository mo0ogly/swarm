# Task result — plan-10b0e692d2-V3

## Outcome in two sentences

Le candidat sale intègre un composant partagé « Version et nouveautés » dans le cockpit et Préparer : version/commit du binaire, sources séparées, historique embarqué vide ou indisponible explicite, lien GitHub sûr, FR/EN et styles Wattson. La vérification navigateur réelle et les captures restent **NOT TESTED** car la sandbox refuse d’abord l’écoute TCP du serveur puis le démarrage de Chrome ; ce résultat est partiel, sans revue indépendante ni acceptation moteur.

## Identity and scope

- Work / task / attempt / producer: w-10b0e692d202f36ee05a1b28 / plan-10b0e692d2-V3 / a-d153cc500e0c5c9039c1572e / auto-441a64b7b0c3654bf845
- Role and assigned scope: worker ; web/runtime-health.js, cockpit/préparation, styles avec jetons Wattson, locale EN et tests/version_ui.cjs ; aucun changement au graphe
- Base / candidate revision: 8d1e223053d7c047bde94276ea3094bd748c82f5 + diff sale
- Fichiers V3 modifiés/créés: web/runtime-health.js, web/index.html, web/prepare.html, web/cockpit.css, web/prephase.css, locales/en.json, web/i18n-en.js, tests/version_ui.cjs, présent rapport
- Changements V1/V2 préexistants et préservés: .dockerignore, Dockerfile, Makefile, install.sh, main.go, runtime_health.go, build.sh, version.go, version_history.go, version_history.json, version_test.go, autres rapports/outils listés par git status
- State: implémentation terminée dans le périmètre ; validation partielle et captures absentes ; pas d’acceptation moteur

## Findings the responsible planner must know

- L’interruption précédente a-46f9088b6ce9f81121402464 provenait d’un délai sans sortie fournisseur après 8 appels. Aucun diff web n’était préservé ; le bilan moteur ne validait aucun critère.
- Le cache Go fourni est /home/fpizzi/workspace/swarm-version-history/.swarm/cache/go-build ; aucun GOCACHE manuel n’a été fixé.
- La suite go test ./... -count=1 n’a émis aucune sortie pendant 180 s et a été interrompue une fois (Ctrl-C, code 1). Elle n’a pas été répétée ; les tests version/runtime-health ciblés passent.
- La recette réelle a échoué avant navigation: listen tcp 127.0.0.1:0: socket: operation not permitted. Après adaptation explicite en fixture, Chrome a aussi échoué avant page: setsockopt: Operation not permitted. Aucune capture réelle n’existe et aucun PASS visuel n’est revendiqué.
- Les deux explorations initiales avec chemins CSS supposés (web/styles.css, puis web/prepare.css) ont rendu le code 2. Les chemins réels web/cockpit.css et web/prephase.css ont ensuite été utilisés sans répéter ces commandes.
- Le chemin historique docs/plans/version-history/V3-handoff.md reste absent : le contrat courant imposait un seul rapport relayé, docs/plan-10b0e692d2-V3.md.

## Changes and verification

| Requirement / gate (mandatory=true) | Change / file | Exact check and environment | Observed effect | Result | Evidence / limit |
| --- | --- | --- | --- | --- | --- |
| plan-entry | État initial | pwd; git rev-parse --show-toplevel; git status --short --branch; git rev-parse HEAD; go env GOCACHE; lecture interruption/diff | Racine correcte, base et changements V1/V2 identifiés, aucun diff web initial | PASS | Premier appel de cette tentative |
| plan-criterion-1 / req-5 | Composant commun et deux entrées web | node --check web/runtime-health.js; contrôle VM du contrat courant + legacy; build et commandes version; tests Go ciblés | Structure courante binary/source/history et ancien contrat normalisés; binaire devel, commit 8d1e223053d7, modifié; intégration statique cockpit/préparation présente | PARTIAL | Parcours navigateur non exécutable dans la sandbox |
| plan-criterion-2 / req-6 | FR/EN, thèmes, clavier/focus, loading/error, recette | node --check tests/version_ui.cjs; recette réelle puis fixture | Recette écrite pour FR/EN × État/sombre, Entrée/Échap/focus, chargement, HTTP 503 et captures; serveur puis Chrome bloqués avant interaction | NOT TESTED | test-results/version-ui/failure.txt; test-results/version-ui-fixture/failure.txt; aucune capture |
| plan-validation | Régressions non graphiques | npm test; go vet ./...; tests Go ciblés; génération/test i18n; git diff --check | Tous ces contrôles sortent 0; suite Go complète sans verdict après interruption | PARTIAL | Revue indépendante et navigateur réel manquants |
| plan-delivery | Rapport courant | Présence et contenu du présent fichier; relais automatique annoncé par le moteur | Handoff factuel prêt; lecture, revue et acceptation non prouvées | NOT TESTED | Acceptation distincte, détenue par le moteur |

### Commandes et résultats exacts

- timeout 120s go test . -run 'Test(Version|BinaryVersion|ReleaseBuild|EmbeddedVersionHistory|ServerVersion|RuntimeHealth)' -count=1 -v → code 0, 17 tests/sous-tests pertinents PASS, package ok en 0,757 s.
- sh ./build.sh bin/swarm && ./bin/swarm --version && ./bin/swarm version --json → code 0; Swarm devel, commit complet 8d1e223053d7c047bde94276ea3094bd748c82f5, modified=true, build 2026-10-02T18:48:58Z, provenance injected.
- timeout 120s npm test && timeout 120s go vet ./... && contrôle normalisation VM && npm run i18n:build && node tests/i18n_test.cjs && git diff --check → code 0; tests frontend existants PASS, vet PASS, contrats version courant/legacy PASS, catalogue i18n PASS, diff check PASS.
- go test ./... -count=1 → aucune sortie pendant 180 s; interruption volontaire unique, code 1; aucun verdict de suite.
- node tests/version_ui.cjs bin/swarm test-results/version-ui → code 1, serveur bloqué par listen tcp 127.0.0.1:0: socket: operation not permitted.
- node --check tests/version_ui.cjs && node tests/version_ui.cjs bin/swarm test-results/version-ui-fixture --fixture → code 1, Chrome bloqué par setsockopt: Operation not permitted.

## APEX / PDCA checkpoint

- Analysis / PLAN: reprise du diff préservé, contrat V2 runtime_health.version.binary/source/history, état vide réel de version_history.json, anciens champs à tolérer.
- Execution / DO: composant partagé autonome, modale accessible native, retour focus, contenu via textContent, liens HTTPS GitHub filtrés, entrées cockpit/préparation, traductions et styles Wattson; recette navigateur sur racine temporaire.
- Verification / CHECK: contrôles Go/Node/i18n/vet/diff réussis; parcours réel et captures non exécutés à cause des permissions sockets.
- Adjustment / ACT: remplacement unique de execFileSync par spawn après EPERM; arrêt des tentatives UI après blocages serveur puis navigateur; aucune répétition identique.
- Recovery limits: départ 2/3; aucun budget augmenté; prochaine précondition requise: superviseur autorisant l’écoute localhost et Chrome.

## Next action and limits

Le responsable doit rejouer node tests/version_ui.cjs bin/swarm test-results/version-ui dans l’environnement superviseur autorisé, conserver les huit captures FR/EN × État/sombre plus loading-fr-etat.png et unavailable-en-sombre.png, puis demander une revue indépendante sur la même révision candidate. Req-6 et les gates validation/delivery ne peuvent pas être déclarées PASS avant cette preuve; l’acceptation reste une opération distincte du moteur.


## Qualification native complémentaire du superviseur
Les limites sandbox du worker restent historiques et explicites. Le superviseur a corrigé l’authentification du test via le lien de session réel, la normalisation source du contrat legacy et la préservation du bouton de diagnostic de stockage. Le contrôle visuel a révélé un lien trop sombre dans le cockpit sombre : jeton Wattson appliqué et test de contraste >= 4.5 ajouté, avec surfaces secondaires distinctes. La recette native des deux pages a réussi dans les quatre combinaisons langues/thèmes, avec Échap, retour focus, chargement, indisponibilité et absence d’erreurs JS. La politique courante compile directement le candidat puis exécute la recette navigateur complète (sans --fixture), dans une racine temporaire distincte de la mission. Source du contrôle effectivement autorisé :
```python
#!/usr/bin/env python3
"""Build the current candidate and run its actual web journey on an isolated root."""
import pathlib, subprocess
root = pathlib.Path(__file__).resolve().parents[2]
subprocess.run(['sh', './build.sh', 'bin/swarm'], cwd=root, check=True)
subprocess.run(['node', 'tests/version_ui.cjs', 'bin/swarm', 'test-results/version-ui-engine'], cwd=root, check=True)

```
Source de la recette (inclut la couverture et les assertions) :
```javascript
'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),os=require('node:os'),path=require('node:path'),{spawn}=require('node:child_process'),puppeteer=require('puppeteer');
const binary=path.resolve(process.argv[2]||'bin/swarm'),out=path.resolve(process.argv[3]||'test-results/version-ui'),root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-version-ui-'));
const fixture=process.argv.includes('--fixture'),web=path.resolve(__dirname,'../web');fs.mkdirSync(out,{recursive:true});let server,browser,fixtureHealth;
const errors=[],failed=[],checks=[];
function run(args){return new Promise((resolve,reject)=>{const child=spawn(binary,args);let out='',err='';child.stdout.on('data',chunk=>out+=chunk);child.stderr.on('data',chunk=>err+=chunk);child.on('error',reject);child.on('exit',code=>code===0?resolve(out):reject(Error(`${args.join(' ')}: ${code}: ${err||out}`)))})}
async function start(){
 await run(['--root',root,'init']);if(fixture){const version=JSON.parse(await run(['--root',root,'--json','version']));fixtureHealth={state:'ready',message:'',next_step:'',volumes:[],version:{binary:version.binary,source:{available:true,commit:version.binary.commit,modified:version.binary.modified,compared:true,matches_binary:true},history:{schema_version:1,available:true,releases:[]}}};return 'http://swarm.test'}server=spawn(binary,['--root',root,'web','127.0.0.1:0']);
 return new Promise((resolve,reject)=>{let text='',err='';server.stdout.on('data',chunk=>{text+=chunk;const match=text.match(/http:\/\/[^\s]+/);if(match)resolve(match[0])});server.stderr.on('data',chunk=>err+=chunk);server.on('exit',code=>reject(Error(`server ${code}: ${err}`)))})
}
function diagnostics(page){page.on('pageerror',error=>errors.push(error.message));page.on('requestfailed',request=>failed.push(`${request.method()} ${request.url()} ${request.failure()?.errorText||''}`))}
async function installFixture(page,mode='normal'){
 if(!fixture)return null;await page.setRequestInterception(true);let release;
 const shell=surface=>`<!doctype html><html lang="fr" data-theme="etat"><head><meta charset="utf-8"><script src="/i18n-en.js" defer></script><script src="/i18n.js" defer></script><link rel="stylesheet" href="/wattson_themes.css"><link rel="stylesheet" href="/${surface==='cockpit'?'cockpit':'prephase'}.css"><script src="/runtime-health.js" defer></script></head><body>${surface==='cockpit'?'<aside class="rail"><div class="rail-bottom"><button id="version-open" type="button" data-version-open><span>Version et nouveautés</span><small id="server-version" data-version-summary role="status">Chargement de la version…</small></button></div></aside><main><section id="runtime-health" hidden></section></main>':'<header class="prep-header"><a class="brand">SWARM <span>Préparer</span></a><nav><button id="version-open" type="button" data-version-open>Version et nouveautés</button></nav></header><main><div class="prep-heading"><div><h1>Préparer un projet</h1></div><div class="prep-statuses"><span data-version-summary class="status info">Chargement de la version…</span></div></div></main>'}</body></html>`;
 page.on('request',async request=>{const url=new URL(request.url()),pathname=url.pathname;if(pathname==='/api/v1/runtime-health'){if(mode==='loading')await new Promise(resolve=>release=resolve);if(mode==='unavailable')return request.respond({status:503,contentType:'application/json',body:'{"error":"fixture unavailable"}'});return request.respond({status:200,contentType:'application/json',body:JSON.stringify(fixtureHealth)})}if(pathname==='/'||pathname==='/prepare.html')return request.respond({status:200,contentType:'text/html',body:shell(pathname==='/'?'cockpit':'prepare')});const file={'/i18n-en.js':'i18n-en.js','/i18n.js':'i18n.js','/runtime-health.js':'runtime-health.js','/wattson_themes.css':'wattson_themes.css','/cockpit.css':'cockpit.css','/prephase.css':'prephase.css'}[pathname];if(file)return request.respond({status:200,contentType:file.endsWith('.css')?'text/css':'text/javascript',body:fs.readFileSync(path.join(web,file))});return request.respond({status:204,body:''})});return ()=>release?.();
}
async function verifySurface(base,surface,lang,theme){
 const page=await browser.newPage();diagnostics(page);await installFixture(page);await page.setViewport({width:1440,height:1000});
 const url=new URL(surface==='cockpit'?'/' : '/prepare.html',base);url.searchParams.set('lang',lang);await page.goto(url.href);await page.waitForFunction(()=>!document.querySelector('[data-version-summary]').textContent.includes('Chargement')&&!document.querySelector('[data-version-summary]').textContent.includes('Loading'));
 await page.evaluate(value=>{document.documentElement.dataset.theme=value;localStorage.setItem('swarm-theme',value)},theme);
 const health=await page.evaluate(()=>fetch('/api/v1/runtime-health').then(response=>response.json())),summary=await page.$eval('[data-version-summary]',element=>element.textContent);
 assert.ok(summary.includes(health.version.binary.version),summary);assert.ok(summary.includes(health.version.binary.commit?.slice(0,12)|| (lang==='en'?'unknown':'inconnu')),summary);
 const buttonText=await page.$eval('[data-version-open]',element=>element.textContent);assert.match(buttonText,lang==='en'?/Version and what's new/:/Version et nouveautés/);
 await page.focus('[data-version-open]');await page.keyboard.press('Enter');await page.waitForSelector('#version-dialog[open]');
 const modal=await page.$eval('#version-dialog',element=>element.textContent);assert.match(modal,lang==='en'?/Running binary/:/Binaire lancé/);assert.match(modal,lang==='en'?/No published version has been declared yet/:/Aucune version publiée n’est encore déclarée/);
 const link=await page.$eval('#version-dialog a[href*="github.com/mo0ogly/swarm/commits/main"]',element=>({target:element.target,rel:element.rel,href:element.href}));assert.equal(link.target,'_blank');assert.match(link.rel,/noopener/);assert.equal(link.href,'https://github.com/mo0ogly/swarm/commits/main/');
 const colors=await page.$eval('#version-dialog a',a=>({ink:getComputedStyle(a).color,background:getComputedStyle(a.closest('section')).backgroundColor}));const luminance=value=>{const c=value.match(/[0-9.]+/g).slice(0,3).map(Number).map(x=>{x/=255;return x<=.04045?x/12.92:((x+.055)/1.055)**2.4});return .2126*c[0]+.7152*c[1]+.0722*c[2]};const l=[luminance(colors.ink),luminance(colors.background)].sort((a,b)=>b-a);assert.ok((l[0]+.05)/(l[1]+.05)>=4.5,'GitHub link contrast below 4.5:1');
 await page.screenshot({path:path.join(out,`${surface}-${lang}-${theme}.png`),fullPage:true});await page.keyboard.press('Escape');await page.waitForFunction(()=>!document.querySelector('#version-dialog').open);assert.equal(await page.evaluate(()=>document.activeElement.id),'version-open');
 checks.push(`${surface} ${lang}/${theme}: binary version+commit, empty history, safe link, keyboard/Escape/focus`);await page.close();
}
async function verifyLoading(base){
 const page=await browser.newPage();diagnostics(page);let release;if(fixture)release=await installFixture(page,'loading');else{await page.setRequestInterception(true);page.on('request',async request=>{if(request.url().endsWith('/api/v1/runtime-health')){await new Promise(resolve=>release=resolve);return request.continue()}request.continue()})}
 await page.goto(new URL('/prepare.html?lang=fr',base).href);await page.waitForSelector('[data-version-open]');await page.click('[data-version-open]');await page.waitForSelector('#version-dialog[open]');assert.match(await page.$eval('#version-dialog',element=>element.textContent),/Chargement de la version et des nouveautés/);await page.screenshot({path:path.join(out,'loading-fr-etat.png')});release();await page.waitForFunction(()=>document.querySelector('[data-version-summary]').textContent!=='Chargement de la version…');checks.push(`loading state before delayed ${fixture?'fixture':'real'} runtime-health response`);await page.close();
}
async function verifyUnavailable(base){
 const page=await browser.newPage();diagnostics(page);if(fixture)await installFixture(page,'unavailable');else{await page.setRequestInterception(true);page.on('request',request=>request.url().endsWith('/api/v1/runtime-health')?request.respond({status:503,contentType:'application/json',body:'{"error":"fixture unavailable"}'}):request.continue())}
 await page.goto(new URL('/prepare.html?lang=en',base).href);await page.waitForFunction(()=>document.querySelector('[data-version-summary]').textContent==='Version unavailable');await page.evaluate(()=>document.documentElement.dataset.theme='sombre');await page.click('[data-version-open]');await page.waitForSelector('#version-dialog[open]');assert.match(await page.$eval('#version-dialog',element=>element.textContent),/Version and history unavailable/);await page.screenshot({path:path.join(out,'unavailable-en-sombre.png')});checks.push('explicit unavailable state on HTTP 503');await page.close();
}
(async()=>{try{
 const base=await start();browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 if(!fixture){const session=await browser.newPage();diagnostics(session);await session.goto(base,{waitUntil:'networkidle0'});await session.close()} // Authenticate through the actual session link before navigating away.
 for(const surface of ['cockpit','prepare'])for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])await verifySurface(base,surface,lang,theme);
 await verifyLoading(base);await verifyUnavailable(base);assert.deepEqual(errors,[],'page errors');assert.deepEqual(failed,[],'unexpected failed requests');
 const status=fixture?'PARTIAL':'PASS';fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status,mode:fixture?'browser fixture; server socket unavailable':'real candidate server',root,binary,checks,errors,failed},null,2));console.log(`${status} version UI (${checks.length} checks; ${out})`);
}finally{await browser?.close();server?.kill('SIGTERM')}})().catch(error=>{fs.writeFileSync(path.join(out,'failure.txt'),error.stack);console.error(error);process.exitCode=1});

```
Les résultats worker ne sont pas transformés en succès autonome ; il s’agit d’une qualification supervisée, avec revue indépendante encore requise.

## Captures désormais conservées et rattachées
Les dix PNG du contrôle moteur réel réussi sont conservés sous docs/screenshots/version-history/ et dans captures.zip, avec le result.json natif. Cette archive immutable est un artefact du contrôle autorisé ; le manifeste et le vérificateur confirment chacun des dix SHA, dimensions, résultat des dix parcours et absence d’erreurs. La revue Codex examine ces preuves structurées ; elle ne reçoit pas une revue visuelle implicite. Le superviseur a rendu et inspecté les modales dans les deux thèmes ; le contraste du lien est aussi contrôlé par le navigateur. Les assertions historiques du worker « captures absentes » sont remplacées par cette qualification native postérieure, sans réécrire son rapport original. Manifeste complet :
```json
{
  "origin": "Engine control version-web-real; actual native server on isolated root, no --fixture",
  "captures": {
    "cockpit-en-etat.png": "9ee0e6d1576eefe43b99f49165999b6384022e58a205ca2dc314742660838c66",
    "cockpit-en-sombre.png": "6c28117a884cd71894f56a0e0b2a9090c3d22bb91a78691b4bf891b73fd5bb7f",
    "cockpit-fr-etat.png": "d56af7740249863a935566ef74e57c62b92d721b171ce0932497a84c8a65acdb",
    "cockpit-fr-sombre.png": "cb2f866a44077a4ed5b6db46ab39c148908b60fd183f66fe18b796179711c046",
    "loading-fr-etat.png": "6ad3b8f5b11449a8f20d9701d26d5abe5935d65db198b1342804a39405cdea46",
    "prepare-en-etat.png": "29cd5fdbb19144acacf1f66a47cd2d24a3fff4b97dc060ac0bd0fefff18c5542",
    "prepare-en-sombre.png": "74faa2d7a565b598f4445b7ab0e3354c53c91046a85172e905291a0f3aa19666",
    "prepare-fr-etat.png": "90730ce3c457a19e95f9dc174c1a6278faacdf5cdaab726eebdfc6520d8552a9",
    "prepare-fr-sombre.png": "699ae751cf6e1e3dafcaf4fc91b165e04e025fdb3049003eb12b7ea3ef301405",
    "unavailable-en-sombre.png": "630d5109de948c46bc733e26a68fff833f758b473b1f59049a6413af479242da"
  }
}
```
Source du contrôle de conservation :
```python
#!/usr/bin/env python3
"""Verify receipt-bound screenshot bundle. This is not a visual AI review."""
import hashlib, json, pathlib, struct, zipfile
root = pathlib.Path(__file__).resolve().parents[2]
manifest = json.loads((root/'docs/screenshots/version-history/manifest.json').read_text())
with zipfile.ZipFile(root/'docs/screenshots/version-history/captures.zip') as bundle:
    assert set(bundle.namelist()) == set(manifest['captures']) | {'result.json'}
    result = json.loads(bundle.read('result.json'))
    assert result['status'] == 'PASS' and len(result['checks']) == 10 and not result['errors'] and not result['failed']
    for name, digest in manifest['captures'].items():
        raw = bundle.read(name)
        assert hashlib.sha256(raw).hexdigest() == digest
        assert raw[:8] == b'\x89PNG\r\n\x1a\n'
        width, height = struct.unpack('>II', raw[16:24])
        assert width > 0 and height > 0
assert len(manifest['captures']) == 10
print('PASS: 10 preserved real-journey PNGs, SHA-256, dimensions and native browser result; visual inspection remains supervisor evidence')

```
