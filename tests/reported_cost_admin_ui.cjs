'use strict';
/** Exercise the shared Admin editor with a qualified normal CLI on a fresh Root.
 * FR/EN, both themes and mobile/desktop use real preview/save/reopen and help.
 * Costs are not executed here; the native-worker recipe separately proves the
 * admission consumer. No installed service, provider or global watchdog is used.
 * Failed commands, browser inputs and fixture Root remain in fresh evidence.
 * Read calls omit the optional exposed-function argument: passing undefined
 * explicitly across Puppeteer can serialize null into a forbidden CLI input.
 */
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process');
const puppeteer=require('puppeteer'),runtimeHelper=require('./helpers/browser_verification_runtime.cjs'),adminState=require('./helpers/engine_settings_admin_state.cjs');
assert.equal(process.argv.length,4,'Usage: node tests/reported_cost_admin_ui.cjs QUALIFIED_BINARY FRESH_OUTPUT');
const repo=path.resolve(__dirname,'..'),binary=fs.realpathSync(process.argv[2]),out=path.resolve(process.argv[3]),root=path.join(out,'root');
fs.mkdirSync(out,{recursive:false});fs.mkdirSync(root,{mode:0o700});
const commands=[],checks=[],errors=[];let sequence=0,browser,runtime,server;
const field='mission_dispatch_cost_reserve_percent';
let rejectBrowser;
const browserFailure=new Promise((resolve,reject)=>{rejectBrowser=reject;});
browserFailure.catch(()=>{});
/** Reject the active browser run immediately, retaining the first cause and
 * all observed diagnostics. This adds no timeout or retry to DOM waits. */
function browserFault(message){errors.push(message);rejectBrowser(Error(message));}
/** Invoke actual CLI, retain all channels/argv and preserve typed failure codes.
 * A request is a private fixture file; a nonzero exit never becomes empty JSON.
 */
function cli(op,request){
 const args=['--root',root,'--json',...op];
 if(request!==undefined){const file=path.join(out,'request-'+(++sequence)+'.json');fs.writeFileSync(file,JSON.stringify(request),{mode:0o600});args.push('--input',file);}
 const result=spawnSync(binary,args,{cwd:repo,input:'',encoding:'utf8',timeout:0});
 commands.push({argv:[binary,...args],code:result.status,stdout:result.stdout,stderr:result.stderr});
 fs.writeFileSync(path.join(out,'commands.json'),JSON.stringify(commands,null,2)+'\n');
 if(result.error)throw result.error;
 if(result.status!==0){const failure=JSON.parse(result.stderr).failure;const error=Error(failure.message);error.code=failure.code;throw error;}
 return JSON.parse(result.stdout);
}
/** Preserve source hashes before/after, runtime inputs and exact public effects.
 * Exceptions retain the fresh fixture; browser close errors are independent.
 */
async function main(){
 const crypto=require('node:crypto');
 const sources=['tests/reported_cost_admin_ui.cjs','frontend/engine-settings/admin.js','frontend/engine-settings/admin.css','web/i18n-en.js','locales/en.json','tests/helpers/browser_verification_runtime.cjs','tests/helpers/engine_settings_admin_state.cjs'];
 const hashes=()=>Object.fromEntries(sources.map(p=>[p,crypto.createHash('sha256').update(fs.readFileSync(path.join(repo,p))).digest('hex')]));
 const before=hashes();fs.writeFileSync(path.join(out,'sources.json'),JSON.stringify(before,null,2)+'\n');
 cli(['init']);runtime=runtimeHelper.begin(repo,out);
 server=require('node:http').createServer((req,res)=>{res.writeHead(200,{'content-type':'text/html'});res.end('<!doctype html><html><head><title>Admin cost fixture</title></head><body><main id="admin"></main></body></html>')});
 await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
 browser=await puppeteer.launch({...runtimeHelper.launchOptions,headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 await runtimeHelper.recordBrowser(runtime,browser);
 for(const lang of ['fr','en'])for(const theme of ['etat','sombre'])for(const width of [390,1280]){
  const label=lang+'-'+theme+'-'+width;console.log('CHECK '+label);
  const page=await browser.newPage();page.setDefaultTimeout(0);page.setDefaultNavigationTimeout(0);await page.setViewport({width,height:900});
  page.on('pageerror',e=>browserFault(label+': '+e.message));page.on('console',m=>{if(m.type()==='error')browserFault(label+': '+m.text())});page.on('requestfailed',r=>browserFault(label+': '+r.url()));
  await page.exposeFunction('nativeSettings',(op,request)=>{try{return {value:cli(['engine-settings',op],request)}}catch(e){return {failure:{code:e.code,message:e.message}}}});
  await page.goto('http://127.0.0.1:'+server.address().port);
  for(const file of ['web/wattson_themes.css','web/processes.css','web/swarm-design.css','frontend/engine-settings/admin.css'])await page.addStyleTag({path:path.join(repo,file)});
  for(const file of ['web/i18n-en.js','frontend/engine-settings/admin.js'])await page.addScriptTag({path:path.join(repo,file)});
  await page.evaluate(({lang,theme})=>{
   document.documentElement.lang=lang;document.documentElement.dataset.theme=theme;
   const translate=s=>{if(lang==='fr')return s;if(!Object.hasOwn(SwarmEnglish,s))throw Error('Missing translation: '+s);return SwarmEnglish[s]};
   const call=async(op,request)=>{const r=await (request===undefined?nativeSettings(op):nativeSettings(op,request));if(r.failure){const e=Error(r.failure.message);e.code=r.failure.code;throw e;}return r.value};
   window.component=SwarmEngineSettingsAdmin.mount(document.querySelector('#admin'),{describe:()=>call('describe'),show:()=>call('show'),preview:r=>call('preview',r),apply:r=>call('apply',r)},{translate});
  },{lang,theme});
  await adminState.waitForSettingsState(page,'loaded');
  if(process.env.SWARM_REPORTED_COST_ADMIN_INJECT_PAGE_ERROR==='1'){
   fs.writeFileSync(path.join(out,'valid-witness.json'),JSON.stringify({schema_version:cli(['engine-settings','show']).schema_version,loaded:true})+'\n');
   await page.evaluate(()=>{queueMicrotask(()=>{throw Error('fixture page fault')})});
  }
  const edit='[data-action="edit"]',modal='dialog[data-engine-settings]',input=modal+' [data-setting="'+field+'"]',confirm=modal+' [data-action="confirm"]';
  await page.click(edit);await page.waitForSelector(modal);
  await page.$eval(input,e=>e.closest('[data-settings-group]').open=true);
  assert.equal(await page.$eval(input,e=>e.max),'','no hidden percent cap');
  const unlimited=modal+' [data-unlimited="'+field+'"]';
  if(await page.$eval(unlimited,e=>e.checked))await page.click(unlimited);
  const value=width===390?'2500':'9007199254740993';
  await page.$eval(input,(e,v)=>{e.value=v;e.dispatchEvent(new Event('input',{bubbles:true}))},value);
  await page.$eval('[data-setting-reason]',e=>{e.value='Owned Admin cost policy fixture';e.dispatchEvent(new Event('input',{bubbles:true}))});
  const settings=path.join(root,'.swarm/engine-settings.json'),beforePreview=fs.readFileSync(settings);
  await page.click(confirm);await adminState.waitForSettingsState(page,'preview');assert(fs.readFileSync(settings).equals(beforePreview),'preview changed saved policy');
  await page.click(confirm);await adminState.waitForSettingsState(page,'saved');
  assert.equal(cli(['engine-settings','show']).values[field],value);
  await page.click(edit);await page.waitForSelector(modal);assert.equal(await page.$eval(input,e=>e.value),value);
  await page.$eval(input,e=>e.closest('[data-settings-group]').open=true);
  const help=modal+' [data-settings-help="'+field+'"]';
  // Shared component uses settingsHelp camelcase dataset => data-settings-help.
  await page.click(help);await page.waitForSelector('dialog[open]:not([data-engine-settings])');
  const text=await page.$eval('dialog[open]:not([data-engine-settings])',e=>e.textContent);
  assert(text.includes(lang==='fr'?'200 signifie':'200 means'));
  await page.keyboard.press('Escape');assert(await page.$eval(help,e=>document.activeElement===e));
  const overflow=await page.evaluate(()=>document.documentElement.scrollWidth>innerWidth);
  assert.equal(overflow,false,'horizontal overflow');
  await page.screenshot({path:path.join(out,label+'.png')});await page.keyboard.press('Escape');
  assert(await page.$eval(edit,e=>document.activeElement===e));assert.equal(errors.length,0,errors.join('\n'));
  checks.push({label,value,preview_readonly:true,saved_reopened:true,help_and_focus:true,no_overflow:true});await page.close();
 }
 assert.deepEqual(hashes(),before,'UI sources changed during verification');
 const runtimeResult=runtimeHelper.finish(runtime,repo);
 fs.writeFileSync(path.join(out,'result.json'),JSON.stringify({status:'PASS',scope:'Shared Admin component with real CLI; no HTTP auth or installed delivery claim',binary_sha256:crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex'),checks,errors,runtime:runtimeResult,global_watchdog:null},null,2)+'\n');
 console.log('PASS Admin reported-cost policy FR/EN themes widths save reopen help');
}
Promise.race([main(),browserFailure]).catch(error=>{fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({message:error.message,stack:error.stack,errors},null,2)+'\n');process.exitCode=1;}).finally(async()=>{
 const cleanup=await Promise.allSettled([browser?browser.close():Promise.resolve(),server?new Promise((resolve,reject)=>server.close(e=>e?reject(e):resolve())):Promise.resolve()]);
 const failed=cleanup.filter(r=>r.status==='rejected').map(r=>String(r.reason));
 if(failed.length){fs.writeFileSync(path.join(out,'cleanup-failure.json'),JSON.stringify(failed)+'\n');process.exitCode=1;}
});
