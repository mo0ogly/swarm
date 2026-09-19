'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto');
const {spawn,execFileSync}=require('node:child_process'),assert=require('node:assert/strict'),puppeteer=require('puppeteer');
const before=path.resolve(process.argv[2]),after=path.resolve(process.argv[3]),output=path.resolve(process.argv[4]);
for(const binary of [before,after])assert.ok(fs.existsSync(binary),'Missing recorded binary: '+binary);
fs.mkdirSync(output,{recursive:true});
const fixture=JSON.parse(execFileSync('python3',[path.join(__dirname,'pilotage_fixture.py'),after],{encoding:'utf8'}));
const summarize=values=>{const sorted=[...values].sort((a,b)=>a-b);return {samples:values.length,median_ms:sorted[Math.floor(sorted.length/2)],p95_ms:sorted[Math.ceil(sorted.length*.95)-1],values_ms:values}};
async function nativeSelect(page,selector,value){
 const index=await page.$eval(selector,(e,v)=>[...e.options].findIndex(o=>o.value===v),value);assert.ok(index>=0);
 await page.focus(selector);await page.keyboard.press('Home');for(let i=0;i<index;i++)await page.keyboard.press('ArrowDown');await page.keyboard.press('Enter');
}
async function measure(binary,label,modern){
 const fingerprint=crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
 const server=spawn(binary,['--root',fixture.root,'web'],{stdio:['ignore','pipe','pipe']});let browser;
 try{
  const url=await new Promise((resolve,reject)=>{let text='';const timer=setTimeout(()=>reject(Error('No web URL: '+text)),15000);server.stdout.on('data',d=>{text+=d;const m=text.match(/http:\/\/\S+\/session\/\S+/);if(m){clearTimeout(timer);resolve(m[0])}});server.once('exit',code=>{clearTimeout(timer);reject(Error('Server exited '+code))})});
  browser=await puppeteer.launch({headless:true,executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});
  const page=await browser.newPage(),errors=[];page.on('pageerror',e=>errors.push(e.message));page.setDefaultTimeout(20000);await page.setViewport({width:1440,height:1000});
  await page.goto(url);await page.waitForSelector('#work option');
  const choose=async id=>{await page.select('#work',id);await page.waitForFunction(id=>snapshot?.work.id===id,{},id)};
  // Configure the new graph through its actual controls, then leave the work.
  // Timing starts after scripts load, with graph choice already remembered.
  if(modern){await choose(fixture.works['50']);await nativeSelect(page,'#pilot-view','dependencies');await page.waitForSelector('.graph-noeud')}
  await choose(fixture.works['0']);
  await page.evaluate(target=>{
   window.baselineFirst=null;window.baselineStart=null;
   document.getElementById('work').addEventListener('change',()=>{window.baselineStart=performance.now()},{capture:true,once:true});
   const observer=new MutationObserver(()=>{if(window.baselineStart!==null&&snapshot?.work.id===target&&document.querySelectorAll('.graph-noeud').length===50){window.baselineFirst=performance.now()-window.baselineStart;observer.disconnect()}});
   observer.observe(document.body,{subtree:true,childList:true,attributes:true});
  },fixture.works['50']);
  await choose(fixture.works['50']);await page.waitForFunction(()=>window.baselineFirst!==null);
  const firstGraph=await page.evaluate(()=>window.baselineFirst),refresh=[],selection=[];console.log(label+' first graph '+firstGraph+' ms');
  for(let i=0;i<30;i++)refresh.push(await page.evaluate(async()=>{const start=performance.now();await refresh(true);await new Promise(requestAnimationFrame);return performance.now()-start}));
  const dialog=modern?'#pilot-inspector':'#modal';
  for(let i=0;i<30;i++){
   await page.evaluate(selector=>{
    window.baselineSelection=null;let start;
    const onClick=e=>{if(e.target.closest('.graph-noeud')){start=performance.now();document.removeEventListener('click',onClick,true)}};
    document.addEventListener('click',onClick,true);
    const observer=new MutationObserver(()=>{if(start!==undefined&&document.querySelector(selector)?.open){requestAnimationFrame(()=>{window.baselineSelection=performance.now()-start});observer.disconnect()}});
    observer.observe(document.body,{subtree:true,childList:true,attributes:true});
   },dialog);
   await page.$eval('.graph-noeud',e=>e.scrollIntoView({block:'center',inline:'center'}));const point=await page.$eval('.graph-noeud rect',e=>{const r=e.getBoundingClientRect();return {x:r.x+Math.min(20,r.width/2),y:r.y+Math.min(20,r.height/2)}});await page.mouse.click(point.x,point.y);try{await page.waitForFunction(()=>window.baselineSelection!==null)}catch(error){console.error(label+' sample '+i,await page.evaluate(()=>({open:[...document.querySelectorAll('dialog')].map(d=>({id:d.id,open:d.open})),nodes:document.querySelectorAll('.graph-noeud').length})));await page.screenshot({path:path.join(output,label+'-failure.png'),fullPage:true});throw error}
   selection.push(await page.evaluate(()=>window.baselineSelection));
   await page.click(modern?'.pilot-inspector-head button':'#cancel');await page.waitForFunction(s=>!document.querySelector(s).open,{},dialog);
  }
  const idleStart=await page.metrics();await new Promise(resolve=>setTimeout(resolve,1000));const idleEnd=await page.metrics();
  await page.screenshot({path:path.join(output,label+'-50.png'),fullPage:true});assert.deepEqual(errors,[]);
  assert.equal(crypto.createHash('sha256').update(fs.readFileSync(binary)).digest('hex'),fingerprint,'Binary changed during recipe');
  return {label,binary,sha256:fingerprint,nodes:50,first_graph_ms:firstGraph,refresh:summarize(refresh),selection:summarize(selection),heap_bytes:idleEnd.JSHeapUsedSize,idle_main_thread_percent:100*(idleEnd.TaskDuration-idleStart.TaskDuration)/(idleEnd.Timestamp-idleStart.Timestamp),browser:await browser.version(),viewport:await page.viewport(),errors};
 }finally{await browser?.close();server.kill()}
}
(async()=>{
 const results=[];results.push(await measure(before,'before',false));results.push(await measure(after,'after',true));
 const report={status:'PASS',at:new Date().toISOString(),fixture_root:fixture.root,cpu:os.cpus()[0].model,platform:os.platform(),results,limits:[
  'Single browser run per binary, same isolated 50-task work. No real provider called.',
  'First graph: work-change event to 50 rendered nodes, after scripts load and graph preference is configured; not cold-start navigation.',
  'Selection: trusted click to visible command dialog before, inspection panel after, plus next animation frame. These are different product operations; no equivalent-task speed gain claimed.',
  '30 refreshes measure unchanged snapshots; progression-change stability is covered by a separate recipe.',
  'Heap is a point-in-time JS reading, not retained-memory measurement. Main-thread activity over one idle second is not total operating-system CPU.'
 ]};fs.writeFileSync(path.join(output,'comparison.json'),JSON.stringify(report,null,2));console.log(JSON.stringify(report,null,2));
})().catch(error=>{console.error(error.stack);process.exitCode=1});
