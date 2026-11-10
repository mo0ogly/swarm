/** Verify these local mockups through file URLs; no server or provider calls.
 * Evidence output is explicit. Failures retain their screenshots and diagnostics. */
const puppeteer=require('puppeteer');
const path=require('path');
const fs=require('fs');
(async()=>{
 const output=process.argv[2]; if(!output)throw Error('Expected evidence directory');fs.mkdirSync(output,{recursive:true});
 const browser=await puppeteer.launch({headless:true,executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox']});
 const results=[];
 try{
 for(const [board,file] of [['cockpit','swarm/Main.dc.html'],['preparation','swarm/Preparation.dc.html'],['admin-reference','admin/Main.dc.html'],['admin','admin/Proposition.dc.html'],['lanes','swarm/Couloirs.dc.html'],['security','swarm/Securite.dc.html'],['audit','swarm/Audit.dc.html']]){
 console.log('Checking '+board);const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('requestfailed',r=>errors.push(r.url()+': '+r.failure().errorText));
 await page.setViewport({width:1440,height:1100});await page.goto('file://'+path.resolve(__dirname,file));if (!(await page.$('.preview-bar'))) throw Error(board+': missing preview controls');
 const click=async text=>page.evaluate(text=>{const b=[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===text||b.textContent.includes(text));if(!b)throw Error('Missing '+text);b.click()},text);
 const fill=async(selector,value)=>page.evaluate((selector,value)=>{const input=document.querySelector(selector);input.value=value;input.dispatchEvent(new Event('input',{bubbles:true}))},selector,value);
 const disabled=async text=>page.evaluate(text=>[...document.querySelectorAll('button')].find(b=>b.textContent.trim()===text).disabled,text);
 const assert=(condition,name)=>{results.push({board,name,pass:!!condition});if(!condition)throw Error(board+': '+name)};
 assert(await page.$eval('x-dc',e=>!e.textContent.includes('{{')),'all template expressions rendered');
 assert(await page.$$eval('img',images=>images.every(i=>i.complete&&i.naturalWidth>0)),'local logos loaded');
 if(board==='cockpit'){
 await fill('input[placeholder="Rechercher une tâche ou un rôle"]','API');
 assert(await page.$eval('.task-map',e=>[...e.querySelectorAll('button')].filter(b=>b.style.opacity==='1').length===2),'search highlights two API nodes');
 await click('Décider');
 assert(await page.$eval('.task-inspector h2',e=>e.textContent.includes('Revue')),'decision opens review inspector');
 }
 if(board==='preparation'){
 await click('Je précise');
 await click('Les deux');
 assert(await disabled('Examiner le brief'),'freeform scope does not silently complete brief');
 await click('Import seul');
 assert(!(await disabled('Examiner le brief')),'explicit scope and acceptance enable brief review');
 }
 if(board==='admin'){
 await click('Définir ici');
 assert(await disabled('Examiner les modifications'),'missing reason prevents preview');
 await fill('textarea','Vérification de la maquette');
 await click('Examiner les modifications');
 await click('Simuler l’enregistrement');
 assert(await page.$eval('footer',e=>e.textContent.includes('Simulation terminée')),'save explicitly reports simulation');
 await fill('input[type=number]','0');
 assert(await page.$eval('footer',e=>e.textContent.includes('strictement positifs')),'invalid number shown without inherited fallback');
 }
 if(board==='lanes'){
 assert(await page.$$eval('a',links=>links.some(a=>a.getAttribute('href')==='Main.dc.html')),'lanes links back to graph');
 const before=await page.$eval('section[aria-label]',e=>e.textContent).catch(()=> '');
 await page.evaluate(()=>{const buttons=[...document.querySelectorAll('button[aria-pressed]')];if(buttons.length)buttons.at(-1).click()});
 assert(await page.$$eval('button[aria-pressed]',buttons=>buttons.some(b=>b.getAttribute('aria-pressed')==='true')),'lane task selection remains active');
 }
 if(board==='audit'){
 await click('Vérifier signatures et préfixe');
 assert(await page.$eval('aside',e=>e.textContent.includes('Exemple de verdict')),'audit verdict explicitly remains illustrative');
 await page.evaluate(()=>document.querySelector('button[aria-pressed]').click());
 assert(!(await page.$eval('aside',e=>e.textContent.includes('Exemple de verdict'))),'transaction change clears illustrative verdict');
 }
 for(const theme of ['sombre','clair']){
 if(theme==='clair')await page.$eval('.preview-bar button',b=>b.click());
 if(board==='security'){
 for(const label of ['Intégrité','Confidentialité','Conflits de rôles','Identités','Clés de preuve']){
 await click(label);
 assert(await page.$eval('[role=tabpanel]',e=>e.textContent.trim().length>0),theme+' security panel '+label);
 for(const width of [1440,320]){await page.setViewport({width,height:1100});assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),theme+' '+label+' no overflow at '+width)}
 }
 await click('Intégrité');
 }
 for(const width of [1440,1024,760,390,320]){
 await page.setViewport({width,height:1100});
 assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth),theme+' no page overflow at '+width);
 if(width===1440||width===390)await page.screenshot({path:path.join(output,board+'-'+theme+'-'+width+'.png'),fullPage:true});
 }
 }
 assert(errors.length===0,'no browser errors: '+errors.join('; '));await page.close();
 }
 }finally{fs.writeFileSync(path.join(output,'checks.json'),JSON.stringify(results,null,2));await browser.close()}
 console.log(JSON.stringify({passed:results.length,evidence:output}));
})().catch(e=>{console.error(e);process.exitCode=1});
