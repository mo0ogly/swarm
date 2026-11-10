'use strict';
/** Render the actual workspace Trust/PAEA assets with explicit read/preview doubles.
 * Scope: primary-button contrast, preview affordance, focus and PAEA navigation.
 * No native service, actual configuration apply, provider, live Root or compiler.
 * Pass a fresh output directory; no global browser watchdog or automatic retry.
 */
const fs=require('node:fs'),path=require('node:path'),http=require('node:http');
const assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const output=process.argv[2];if(!output||process.argv.length!==3)throw Error('Usage: node tests/trust_primary_ui_test.cjs FRESH_OUTPUT');
fs.mkdirSync(output);const checks=[],requests=[],contrasts=[],closeEvents=[],errors=[];let server,browser;
const csrf='a'.repeat(64),state={schema_version:1,revision:1,actions:{},amount_minor_text:{},
  policy:{mandate_ttl_seconds:3600,unknown_reconcile_seconds:0,evidence_retention_days:365,require_separation:true}};
const assets=new Map(['/paea.html','/paea.js','/paea.css','/trust-native.js','/trust-native.css',
  '/i18n-en.js','/i18n.js','/swarm-design.css','/wattson_themes.css','/swarm-logo.png'].map(url=>[url,'web'+url]));
assets.set('/trust-actions.html','web/trust-native.html');
/** Retain each named assertion and its progress, propagating a failed criterion. */
function check(name,value){assert(value,name);checks.push(name);console.log('PASS '+name);}
/** WCAG relative luminance ratio for the opaque computed colors of enabled controls. */
function contrast(foreground,background){
  const lum=color=>color.match(/[\d.]+/g).slice(0,3).map(Number).map(x=>x/255)
    .map(x=>x<=.04045?x/12.92:((x+.055)/1.055)**2.4)
    .reduce((sum,x,i)=>sum+x*[.2126,.7152,.0722][i],0);
  const a=lum(foreground),b=lum(background);return(Math.max(a,b)+.05)/(Math.min(a,b)+.05);
}
/** Serve fixed workspace resources and only nonmutating operator projections.
 * Any attempted config-apply or business operation fails the test; stub responses
 * demonstrate UI behavior, never native policy/identity/preview correctness.
 */
function serve(request,response){
  const url=new URL(request.url,'http://localhost');
  if(url.pathname==='/favicon.ico'){response.statusCode=204;response.end();return;}
  if(url.pathname==='/api/v1/native-session'){
    response.setHeader('Content-Type','application/json');response.end(JSON.stringify({schema_version:1,csrf_token:csrf}));return;
  }
  if(url.pathname==='/api/v1/trust-actions'){
    const chunks=[];request.on('data',chunk=>chunks.push(chunk));request.on('end',()=>{
      const raw=Buffer.concat(chunks).toString('utf8'),op=url.searchParams.get('action')||'show';
      requests.push({method:request.method,operation:op,raw});
      try{
        response.setHeader('Content-Type','application/json');
        if(request.method==='GET'&&op==='show'){response.end(JSON.stringify(state));return;}
        assert.equal(op,'config-preview','no configuration apply/business call permitted');
        assert.equal(request.method,'POST');assert.equal(request.headers['x-swarm-csrf'],csrf);
        const input=JSON.parse(raw);assert.equal(input.expected_revision,1);assert.equal(input.policy.require_separation,true);
        response.end(JSON.stringify({applied:false,current_revision:1}));
      }catch(error){errors.push(error.message);response.statusCode=500;response.end('{"failure":{"code":"fixture-refused"}}');}
    });return;
  }
  if(assets.has(url.pathname)){
    response.setHeader('Content-Type',{'.html':'text/html; charset=utf-8','.css':'text/css','.js':'application/javascript','.png':'image/png'}[path.extname(url.pathname)]);
    response.end(fs.readFileSync(assets.get(url.pathname)));return;
  }
  errors.push('unknown fixture path '+url.pathname);response.statusCode=404;response.end();
}
/** Sample settled normal/hover/keyboard states after a valid load. Await actual
 * CSS animation completion, not a convenient fixed sleep: sampling a theme
 * transition can otherwise hide a failing final color. Disabled is checked apart.
 */
async function measure(page,selector,label){
  for(const mode of ['normal','hover','keyboard']){
    await page.mouse.move(0,0);
    if(mode==='hover')await page.hover(selector);
    if(mode==='keyboard'){
      await page.focus(selector);await page.keyboard.press('Tab');
      await page.keyboard.down('Shift');await page.keyboard.press('Tab');await page.keyboard.up('Shift');
    }
    const colors=await page.$eval(selector,async element=>{
      await Promise.all(element.getAnimations().map(animation=>animation.finished));
      return{foreground:getComputedStyle(element).color,background:getComputedStyle(element).backgroundColor,disabled:element.disabled};
    });
    assert.equal(colors.disabled,false);const ratio=contrast(colors.foreground,colors.background);
    contrasts.push({label,selector,mode,...colors,ratio});check(label+'-'+mode+'-contrast-AA',ratio>=4.5);
  }
}
/** Fill an input using its real input event so preview invalidation still runs. */
async function fill(page,id,value){await page.$eval('#'+id,(element,text)=>{element.value=text;element.dispatchEvent(new Event('input',{bubbles:true}));},value);}
(async()=>{try{
  server=http.createServer(serve);await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const base='http://127.0.0.1:'+server.address().port;
  browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/opt/google/chrome/chrome',timeout:0,protocolTimeout:0,args:['--no-sandbox'],userDataDir:path.join(path.resolve(output),'browser')});
  const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);
  page.on('pageerror',error=>errors.push(error.message));page.on('requestfailed',request=>errors.push(request.url()+' '+request.failure()?.errorText));
  page.on('console',message=>{if(message.type()==='error')errors.push(message.text());});
  for(const language of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [1440,1024,760,390,320]){
    const label=language+'-'+theme+'-'+width;await page.setViewport({width,height:1100});
    await page.goto(base+'/trust-actions.html?lang='+language,{waitUntil:'networkidle0'});
    await page.waitForFunction(()=>/Prêt|Ready/.test(document.querySelector('#message').textContent));
    check(label+'-valid-control-loaded',(await page.$eval('h1',element=>element.textContent))===(language==='fr'?'Biba — Actions de confiance':'Biba — Trusted actions'));
    if(theme==='etat')await page.click('#theme');
    check(label+'-theme',await page.$eval('html',(element,value)=>element.dataset.theme===value,theme));
    await measure(page,'#command button[type=submit]',label+'-execute');
    await page.click('#admin');await page.waitForFunction(()=>document.querySelector('#settings').open&&!document.querySelector('#preview').disabled);
    check(label+'-save-initially-disabled',await page.$eval('#save',element=>element.disabled));
    await fill(page,'reason','Synthetic preview only');await fill(page,'identities',JSON.stringify([{subject:'fixture',token:'synthetic-preview-token-only',integrity:3,roles:['admin']} ]));
    const preview=page.waitForResponse(response=>response.url().endsWith('action=config-preview'));
    await page.click('#preview');check(label+'-preview-no-apply',(await(await preview).json()).applied===false);
    await page.waitForFunction(()=>!document.querySelector('#save').disabled);await measure(page,'#save',label+'-save');
    await page.focus('#preview');await page.keyboard.press('Tab');
    check(label+'-save-keyboard-focus',await page.evaluate(()=>document.activeElement.id==='save'&&getComputedStyle(document.activeElement).outlineStyle!=='none'));
    check(label+'-dialog-no-overflow',await page.$eval('#settings',element=>element.scrollWidth<=element.clientWidth));
    await page.screenshot({path:path.join(output,label+'-preview.png'),fullPage:true});
    // Observe close after the page's already-registered handler, not merely the
    // earlier `open=false` mutation. The event supplies the semantic barrier.
    await page.$eval('#settings',element=>{
      globalThis.__trustTestClosed=new Promise(resolve=>element.addEventListener('close',()=>resolve({open:element.open,focused:document.activeElement.id}),{once:true}));
    });
    await page.keyboard.press('Escape');const closed=await page.evaluate(()=>globalThis.__trustTestClosed);
    closeEvents.push({label,...closed});check(label+'-escape-restores-focus',!closed.open&&closed.focused==='admin');
    check(label+'-page-no-overflow',await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
    await page.click('a[href="/paea.html"]');await page.waitForFunction(()=>document.querySelector('#paea-check')&&!document.querySelector('#paea-check').disabled);
    check(label+'-paea-navigation-language',(await page.$eval('h1',element=>element.textContent))===(language==='fr'?'Vérifications PAEA':'PAEA checks'));
    // Reexamine the earlier PAEA surface for the same transition-sampling risk.
    await measure(page,'#paea-check',label+'-paea');
  }
  check('only-read-and-preview-operations',requests.every(request=>['show','config-preview'].includes(request.operation)));
  check('no-browser-errors',errors.length===0);console.log('TRUST_PRIMARY_UI_PASS');
}finally{
  if(browser)await browser.close();if(server)await new Promise(resolve=>server.close(resolve));
  fs.writeFileSync(path.join(output,'result.json'),JSON.stringify({scope:'workspace browser component, read/preview doubles only',checks,requests,contrasts,closeEvents,errors},null,2));
}})().catch(error=>{console.error(error.stack);process.exitCode=1});
