// Recette authentifiée : sept questions explicites facturables ; exige le travail de recette indiqué.
// Pour des tests sans fournisseur réel, employer assistant_ui.cjs.
const fs=require('fs'),pup=require('/home/fpizzi/node_modules/puppeteer');
const url=process.env.SWARM_URL;if(!url)throw Error('SWARM_URL doit être le lien de session affiché par wattson.sh swarm web');
const wid='w-9aed29c86212f8f3c1e6ac8c',agent='717b896e-aa59-4b57-b819-4460b5bcb920';
const dir='docs/plans/swarm-page-assistant/evidence';const cases=[
 ['tasks','SC-14','explain_blocker.v1','Pourquoi cette tâche acceptée historiquement n’est-elle pas validée actuellement ? Cite les faits et les preuves connues, sans supposer que le livrable est faux.'],
 ['decisions','gate','next_action.v1','Quelle action permet de traiter cette décision ? Un acquittement suffirait-il à revalider la tâche ?'],
 ['agents',agent,'understand_page.v1','Cette tentative Claude travaille-t-elle encore ? Distingue son état observé, son dernier résultat et la validation de sa tâche.'],
 ['logs','', 'understand_page.v1','Que montrent ces lignes filtrées du journal ? Explique l’erreur si elle est visible ; si l’extrait ne donne pas sa cause précise, déclare-le explicitement.'],
 ['resume','', 'resume_session.v1','Le résumé historique dit-il que tout est terminé ? Quelle est la validation actuelle et par où reprendre sans inventer de résultat ?'],
 ['budget','', 'understand_page.v1','Combien ce travail a-t-il réellement coûté ? Distingue plafond estimatif, jetons déclarés, coût déclaré par le fournisseur et facture réelle.'],
 ['brainstorm','', 'understand_page.v1','Quel brief est actuellement adopté ? Peut-on considérer les tâches comme exécutées à partir d’un brief ? Signale les informations absentes.']
];
(async()=>{const b=await pup.launch({executablePath:'/usr/bin/google-chrome',args:['--no-sandbox']});try{
 const p=await b.newPage();await p.setViewport({width:1480,height:1100});p.setDefaultTimeout(45000);const errors=[];p.on('pageerror',e=>errors.push(e.message));
 await p.goto(url);await p.waitForSelector('#work option');await p.select('#work',wid);await p.waitForSelector('[data-task="SC-15"]');await p.waitForFunction(()=>assistWork===work&&!assistBusy);
 // The failed Claude attempt stays failed; only its task owner is transferred.
 const before=await p.evaluate(()=>({revision:snapshot.work.revision,tasks:JSON.stringify(snapshot.work.tasks),agents:snapshot.agents.length}));const results=[];
 for(const [page,target,template,question] of cases){
  const prior=await p.evaluate(()=>assistTurns.map(t=>t.id));
  await p.click('[data-view="'+page+'"]');
  if(page==='logs'){await p.select('#log-agent',agent);await p.type('#log-search','session limit');await p.type('#log-kind','output');await p.click('#log-form button');await p.waitForFunction(()=>logPage?.coordinates?.q==='session limit');}
  if(target==='gate'){const option=await p.$$eval('#assist-target option',xs=>xs.find(x=>x.textContent.includes('SC-14')&&x.textContent.includes('gate')&&x.textContent.includes('à traiter'))?.value);if(!option)throw Error('Décision SC-14 non trouvée');await p.select('#assist-target',option)}else if(target)await p.select('#assist-target',target);
  await p.select('#assist-template',template);await p.click('#assist-ask');await p.waitForSelector('#modal[open] #field-provider',{visible:true});await p.select('#field-provider','codex');await p.click('#field-question',{clickCount:3});await p.keyboard.press('Backspace');await p.type('#field-question',question);
  await p.click('#confirm');await p.waitForFunction(()=>document.querySelector('#confirm').textContent==='Confirmer l’envoi à l’IA'||!document.querySelector('#modal-error').hidden);
  if(!await p.$eval('#modal-error',e=>e.hidden))throw Error(await p.$eval('#modal-error',e=>e.textContent));
  const sent=p.waitForResponse(r=>r.url().endsWith('/api/v1/assist/ask')&&r.request().method()==='POST'&&JSON.parse(r.request().postData()).preview===false);
  await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open||!document.querySelector('#modal-error').hidden);if(await p.$eval('#modal',e=>e.open))throw Error(await p.$eval('#modal-error',e=>e.textContent));
  const sentTurn=await (await sent).json();const turnID=sentTurn.id;if(!turnID)throw Error('Identité de question absente');
  console.log('Question réelle envoyée : '+page+' '+turnID);
  await p.waitForFunction(id=>assistTurns.some(t=>t.id===id&&!['pending','running'].includes(t.status)),{timeout:330000},turnID);
  const turn=await p.evaluate(id=>assistTurns.find(t=>t.id===id),turnID);results.push(turn);fs.writeFileSync(dir+'/real-turns.json',JSON.stringify(results,null,2));
  await (await p.$('#assistant')).screenshot({path:dir+'/real-'+page+'.png'});
  console.log(page+' : '+turn.status+(turn.refusal?' — '+turn.refusal.message+' '+(turn.refusal.detail||''):''));
  if(turn.status!=='answered')throw Error('Question réelle non aboutie : '+page);
  if(page==='decisions'&&await p.$('#assist-answer .assist-step > button')){
   await p.click('#assist-answer .assist-step > button');await p.waitForSelector('#modal[open]',{visible:true});const title=await p.$eval('#modal-title',e=>e.textContent);if(!title.includes('SC-14'))throw Error('Mauvaise cible de formulaire : '+title);await p.click('#cancel');console.log('Action proposée vérifiée : formulaire SC-14 ouvert puis annulé');
  }
 }
 const after=await p.evaluate(()=>({revision:snapshot.work.revision,tasks:JSON.stringify(snapshot.work.tasks),agents:snapshot.agents.length}));
 if(JSON.stringify(before)!==JSON.stringify(after))throw Error('La recette de questions a modifié le workflow');if(errors.length)throw Error(errors.join('\n'));
 fs.writeFileSync(dir+'/real-ui.json',JSON.stringify({status:'PASS',provider:'codex',work:wid,questions:results.length,ui_errors:errors,workflow_unchanged:true,turn_ids:results.map(t=>t.id),limitation:'Structure et références contrôlées ; revue sémantique séparée requise.'},null,2));
 console.log('Sept questions réelles terminées ; aucun changement de tâche ni agent de réalisation.');
}finally{await b.close()}})().catch(e=>{console.error(e);process.exitCode=1});
