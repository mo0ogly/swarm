'use strict';
// Extend the unchanged native product journey with observable diagnostic rows.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process');
const out=path.resolve(process.argv[3]);fs.mkdirSync(out,{recursive:true});
let source=fs.readFileSync('tests/graph_draft_ui.cjs','utf8');
const anchor="  const activityScreenshot=name+'-attempt.png';";
assert.equal(source.split(anchor).length,2);
const observation=String.raw`
  await page.locator('[data-mission-action="details"]').click();
  const facts=await page.evaluate(()=>{
   const state=snapshot.mission.tasks.find(t=>t.id==='t000'),task=snapshot.work.tasks.find(t=>t.id==='t000');
   const row=[...document.querySelectorAll('#mission-results .mission-task')].find(e=>e.querySelector('h4')?.textContent===task.title);
   const rendered=[...row.querySelectorAll('.mission-understanding > div')].map(e=>({label:e.querySelector('dt').textContent,value:e.querySelector('dd').textContent,visible:!!e.getClientRects().length}));
   return {task:task.id,attempt:snapshot.pilotage.tasks.t000.attempt.attempt_id,source:state.understanding,rendered,expected:[
    [SwarmI18n.t('Ce qui se passe'),missionFactText(state.understanding.what)],
    [SwarmI18n.t('Prochaine étape'),missionText(state.understanding.next_step)],
    [SwarmI18n.t('Qui agit'),missionText(state.understanding.actor)]
   ]};
  });
  assert.equal(facts.rendered.length,3);assert.equal(facts.source.actor_kind,'user');
  assert.equal(facts.source.what,'Configuration initiale à préparer avant le premier départ');
  assert.match(facts.source.next_step,/configur|prépar/i);
  for(let i=0;i<3;i++){assert.equal(facts.rendered[i].visible,true);assert.deepEqual([facts.rendered[i].label,facts.rendered[i].value],facts.expected[i]);assert.ok(facts.rendered[i].value.trim())}
  assert.equal(new Set(facts.rendered.map(r=>r.value)).size,3);
  await page.locator('[data-mission-action="journal"]').click();
  await page.waitForFunction(()=>document.querySelector('#fil-bloc')?.open&&document.querySelectorAll('#fil-entrees .fil-entree').length);
  const journal=await page.evaluate(()=>({open:document.querySelector('#fil-bloc').open,rows:[...document.querySelectorAll('#fil-entrees .fil-entree')].map(e=>({actor:e.querySelector('.fil-origine')?.textContent,origin:e.dataset.origine,action:e.querySelector('.fil-label')?.textContent,reason:e.querySelector('.fil-message')?.textContent,visible:!!e.getClientRects().length}))}));
  assert.ok(journal.rows.some(r=>r.visible&&r.origin==='vous'&&r.actor&&r.action&&r.reason&&/t000|Tâche 0/.test(r.reason)),'public task operation missing from visible journal');
  await page.screenshot({path:path.join(outDir,name+'-journal.png'),fullPage:true});
  fs.writeFileSync(path.join(outDir,name+'-facts.json'),JSON.stringify({variant:name,facts,journal},null,2));
`;
source=source.replace(anchor,observation+'\n'+anchor);
const load='const load_measurements=[];for(const cards of [50,200,500])load_measurements.push(await loadMeasurement(browser,cards));';
assert.equal(source.split(load).length,2);
source=source.replace(load,'const load_measurements=[];');
const generated=path.join(out,'native-journal.cjs');fs.writeFileSync(generated,source);
const env={...process.env,PUPPETEER_MODULE:require.resolve('puppeteer')};
const result=spawnSync(process.execPath,[generated,path.resolve(process.argv[2]),out],{env,encoding:'utf8',maxBuffer:8*1024*1024});
fs.writeFileSync(path.join(out,'run.log'),result.stdout+result.stderr);
assert.equal(result.status,0,result.stderr);
for(const variant of ['fr-sombre','fr-etat','en-sombre','en-etat']){
 const record=JSON.parse(fs.readFileSync(path.join(out,variant+'-facts.json')));
 console.log(JSON.stringify({variant,task:record.facts.task,attempt:record.facts.attempt,rows:record.facts.rendered,journal:record.journal.rows.filter(r=>/t000|Tâche 0/.test(r.reason)).slice(0,2)}));
}
