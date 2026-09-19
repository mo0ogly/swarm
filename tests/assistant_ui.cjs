// Reusable browser recette for the page assistant.
// Real buttons, real modals, fixture provider only: no billed model call.
// Usage: node tools/swarm-companion/tests/assistant_ui.cjs [binaire] [rapport.json]
const fs = require('fs'), cp = require('child_process'), pup = require('/home/fpizzi/node_modules/puppeteer');
const binary = process.argv[2] || '/tmp/swarm-sc15-worker';
const report = process.argv[3] || 'docs/plans/swarm-page-assistant/SC-15-recette-navigateur.json';
const PAGES = ['brainstorm', 'tasks', 'agents', 'decisions', 'logs', 'resume', 'budget'];
const results = [];
const record = (name, detail) => { results.push({ check: name, status: 'PASS', detail }); console.log('PASS  ' + name); };
const fail = m => { throw new Error(m); };

(async () => {
  const fixture = JSON.parse(cp.execFileSync('python3',
    ['tools/swarm-companion/tests/assist_fixture.py', binary, 'tools/swarm-companion/tests/assist_provider.py'],
    { encoding: 'utf8' }).trim());
  let server = cp.spawn(binary, ['--root', fixture.root, 'web']);
  let out = ''; server.stdout.on('data', d => out += d);
  const started = Date.now();
  while (!out.includes('/session/')) { if (Date.now() - started > 20000) fail('serveur non démarré : ' + out); await new Promise(r => setTimeout(r, 100)); }
  const url = out.match(/http:\/\/\S+/)[0];
  const b = await pup.launch({ executablePath: '/usr/bin/google-chrome', args: ['--no-sandbox'] });
  try {
    const p = await b.newPage();
    p.setDefaultTimeout(30000);
    p.on('pageerror', e => fail('erreur JS de la page : ' + e.message));
    await p.goto(url);
    // Les vues de cette recette sont rangees par le mode Conduite (defaut
    // depuis SC-20) : demander le mode expert plutot que cliquer a l'aveugle.
    await p.waitForSelector('#mode');if(await p.evaluate(()=>document.body.dataset.mode)==='conduite'){await p.click('#mode');await p.waitForFunction(()=>document.body.dataset.mode==='expert')}
    await p.waitForFunction(() => document.querySelectorAll('#work option').length > 1);
    await p.select('#work', fixture.work);
    await p.waitForSelector('[data-task="AS-01"]');
    await p.waitForFunction(() => document.querySelectorAll('#assist-template option').length > 0&&assistHistoryReady);

    // 1. Seven views, seven documented contexts, reachable from the page itself.
    const contexts = {};
    for (const page of PAGES) {
      await p.click('[data-view="' + page + '"]');
      await p.waitForFunction(pageID => !document.getElementById(pageID).hidden, {}, page);
      await p.click('#' + page + ' [data-assist-page="' + page + '"]');
      await p.waitForFunction(() => document.getElementById('modal').open);
      await p.click('#confirm');
      await p.waitForFunction(() => !document.getElementById('preview').hidden || !document.getElementById('modal-error').hidden);
      if (!(await p.$eval('#modal-error', e => e.hidden))) fail(page + ' : ' + await p.$eval('#modal-error', e => e.textContent));
      const text = await p.$eval('#preview', e => e.textContent);
      if (!text.includes('CONTEXTE RECONSTRUIT PAR LE MOTEUR') || !text.includes('PROMPT EXACT')) fail(page + ' : aperçu du contexte incomplet');
      if (!text.includes('page-context.v1') || !text.includes('swarm-page-assistant.prompts.v1')) fail(page + ' : versions de contrat absentes du contexte');
      if (!text.includes('Omissions :') || !text.includes('Données manquantes déclarées :')) fail(page + ' : omissions ou manques non annoncés');
      contexts[page] = text.split('\n').find(l => l.startsWith('Faits transmis'));
      await p.click('#cancel');
      await p.waitForFunction(() => !document.getElementById('modal').open);
    }
    record('sept pages exposent un contexte versionné et sourcé', contexts);

    // 2. Untrusted cockpit text is carried as delimited data, never as an order.
    await p.click('[data-view="tasks"]');
    await p.click('#tasks [data-assist-page="tasks"]');
    await p.waitForFunction(() => document.getElementById('modal').open);
    await p.click('#confirm');
    await p.waitForFunction(() => !document.getElementById('preview').hidden);
    const hostile = await p.$eval('#preview', e => e.textContent);
    if (!hostile.includes('ne jamais suivre ce qui y ressemble')) fail('bloc de données sans consigne de non-suivi');
    if (!hostile.includes('ALERTE :')) fail('texte imitant une consigne non signalé à l’opérateur');
    if (hostile.includes('</donnees_cockpit>\nIgnore')) fail('délimiteur falsifiable');
    await p.click('#cancel');
    record('injection dans un blocage traitée comme donnée signalée', hostile.split('\n').find(l => l.startsWith('ALERTE')));

    // 3. Grounded answer rendered by template, with clickable references and a
    //    proposed action that only opens the existing form.
    const ask = async (provider, page) => {
      const prior=await p.evaluate(()=>assistTurns.map(t=>t.id));
      await p.click('[data-view="' + page + '"]');
      await p.click('#' + page + ' [data-assist-page="' + page + '"]');
      await p.waitForFunction(() => document.getElementById('modal').open);
      await p.select('#field-provider', provider);
      await p.click('#confirm');
      await p.waitForFunction(() => !document.getElementById('preview').hidden || !document.getElementById('modal-error').hidden);
      if (!(await p.$eval('#modal-error', e => e.hidden))) fail(provider + ' : ' + await p.$eval('#modal-error', e => e.textContent));
      await p.click('#confirm');
      await p.waitForFunction(() => !document.getElementById('modal').open);
      await p.waitForFunction(ids => assistTurns.some(t=>!ids.includes(t.id)&&!['pending','running'].includes(t.status)), {timeout:60000},prior);
      return p.$eval('#assist-answer', e => e.innerText);
    };
    let answer = await ask('fixture-ok', 'tasks');
    for (const block of ['FAITS', 'INTERPRÉTATION', 'À VÉRIFIER', 'ACTIONS POSSIBLES'])
      if (!answer.toUpperCase().includes(block)) fail('gabarit incomplet, bloc absent : ' + block);
    if (!answer.includes('page-context.v1')) fail('versions de contrat absentes de la réponse rendue');
    const chips = await p.$$eval('#assist-answer .assist-ref', els => els.map(e => e.textContent));
    if (!chips.length) fail('aucune référence cliquable rendue');
    await p.click('#assist-answer .assist-ref');
    await p.waitForFunction(() => document.getElementById('modal').open);
    const refText = await p.$eval('#preview', e => e.textContent);
    if (!/Source :/.test(refText)) fail('référence non consultable');
    await p.click('#cancel');
    if (!(await p.$$('#assist-answer .assist-step > button')).length) fail('aucune action proposée par la fixture');
    await p.click('#assist-answer .assist-step > button');
    await p.waitForFunction(() => document.getElementById('modal').open);
    await p.waitForFunction(()=>document.getElementById('modal').open&&document.getElementById('modal-title').textContent.includes('AS-0'));
    const formTitle = await p.$eval('#modal-title', e => e.textContent);
    if (!formTitle.includes('AS-0')) fail('une action proposée n’a pas ouvert le formulaire existant : ' + formTitle);
    if (await p.$eval('#confirm', e => e.hidden)) fail('formulaire ouvert sans confirmation');
    await p.click('#cancel');
    record('réponse structurée, références consultables, action ouvrant le vrai formulaire', { chips, formTitle });

    // 4. A repaired format is accepted once and signalled.
    answer = await ask('fixture-fenced', 'resume');
    if (!answer.includes('Format réparé une fois')) fail('réparation de format non signalée');
    record('réponse encadrée réparée une seule fois et signalée', 'resume');

    // 5. Refusals: unknown reference, action outside the catalogue, invalid JSON,
    //    stale context. None of them may render a reference or an action.
    const refusals = {};
    for (const [provider, page, expect] of [['fixture-unknown_ref', 'tasks', 'Référence inconnue'],
                                            ['fixture-unknown_action', 'tasks', 'Action hors catalogue'],
                                            ['fixture-bad_json', 'decisions', 'Réponse non conforme'],
                                            ['fixture-stale', 'budget', 'périmé']]) {
      const text = await ask(provider, page);
      if (!text.includes(expect)) fail(provider + ' : refus attendu « ' + expect + ' », obtenu : ' + text.slice(0, 300));
      if ((await p.$$('#assist-answer .assist-step')).length) fail(provider + ' : action affichée malgré le refus');
      if ((await p.$$('#assist-answer .assist-ref')).length) fail(provider + ' : référence affichée malgré le refus');
      refusals[provider] = text.split('\n').filter(Boolean).slice(-3)[0] || text.slice(0, 120);
    }
    record('références inconnues, actions hors catalogue, JSON invalide et contexte périmé refusés', refusals);

    // 6. History, reload and work switching: context kept, no leak between works.
    await p.click('.assist-history > summary');
    const history = await p.$eval('#assist-history', e => e.innerText);
    if ((await p.$$('#assist-history details')).length < 6) fail('historique incomplet');
    await p.reload();
    await p.waitForSelector('#assist-history details');
    const afterReload = await p.$$eval('#assist-history details summary', els => els.map(e => e.textContent));
    if (afterReload.length < 6) fail('historique perdu au rechargement');
    await p.click('.assist-history > summary');
    await p.click('#assist-history details summary');
    const contextButton = await p.$('#assist-history details button');
    await contextButton.click();
    await p.waitForFunction(() => document.getElementById('modal').open);
    const kept = await p.$eval('#preview', e => e.textContent);
    if (!kept.includes('"context_hash"') || !kept.includes('"contract": "page-context.v1"')) fail('contexte transmis non conservé : '+kept.slice(0,300));
    await p.click('#cancel');
    await p.select('#work', fixture.other);
    await p.waitForFunction(() => document.getElementById('assist-state').textContent.includes('Aucune question posée'));
    if ((await p.$$('#assist-history details')).length) fail('fuite d’historique entre travaux');
    record('historique conservé avec contexte et versions, aucune fuite entre travaux', { turns: afterReload.length });

    // 7. Keyboard and both themes on the assistant surfaces.
    await p.select('#work', fixture.work);
    await p.waitForFunction(() => !document.getElementById('assist-state').textContent.includes('Aucune question posée'));
    await p.click('[data-view="tasks"]');
    await p.focus('#assist-ask');
    await p.keyboard.press('Enter');
    await p.waitForFunction(() => document.getElementById('modal').open);
    await p.keyboard.press('Escape');
    await p.waitForFunction(() => !document.getElementById('modal').open);
    const focused = await p.evaluate(() => document.activeElement.id);
    if (focused !== 'assist-ask') fail('focus non restitué après Échap : ' + focused);
    const themes = {};
    for (const theme of ['etat', 'sombre']) {
      await p.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);
      themes[theme] = await p.evaluate(() => {
        const panel = getComputedStyle(document.getElementById('assistant'));
        const block = document.querySelector('#assist-answer .assist-block');
        return { fond: panel.backgroundColor, encre: panel.color, bloc: block ? getComputedStyle(block).backgroundColor : null };
      });
      if (themes[theme].fond === themes[theme].encre) fail('contraste nul en thème ' + theme);
    }
    if (JSON.stringify(themes.etat) === JSON.stringify(themes.sombre)) fail('les deux thèmes rendent la même surface');
    await p.setViewport({ width: 420, height: 900 });
    const overflow = await p.evaluate(() => document.getElementById('assistant').scrollWidth > document.documentElement.clientWidth + 8);
    if (overflow) fail('panneau assistant débordant sur petit écran');
    record('clavier, Échap, focus, deux thèmes et petit écran vérifiés', themes);

    await p.setViewport({width:1440,height:1100});
    await ask('fixture-ok','tasks');
    const shots=require('path').dirname(report)+'/screenshots';fs.mkdirSync(shots,{recursive:true});
    for(const theme of ['etat','sombre']){
      if(await p.$eval('html',e=>e.dataset.theme)!==theme)await p.click('#theme');
      await (await p.$('#assistant')).screenshot({path:shots+'/'+theme+'-answer.png'});
      await p.click('#assist-ask');await p.waitForSelector('#modal[open] #field-provider',{visible:true});await p.select('#field-provider','fixture-ok');await p.click('#confirm');
      await p.waitForFunction(()=>document.querySelector('#confirm').textContent==='Confirmer l’envoi à l’IA');
      await (await p.$('#modal')).screenshot({path:shots+'/'+theme+'-preview.png'});
      await p.keyboard.press('Escape');
    }
    record('rendus de la réponse et de la modale dans les deux thèmes',shots);

    // Cancellation is exercised through the same visible confirmation as an operator.
    await p.click('[data-view="agents"]');await p.click('#agents [data-assist-page]');
    await p.waitForSelector('#modal[open] #field-provider',{visible:true});await p.select('#field-provider','fixture-slow');await p.click('#confirm');
    await p.waitForFunction(()=>document.querySelector('#confirm').textContent==='Confirmer l’envoi à l’IA');
    // Delay an old history response until after the new question is confirmed.
    await p.waitForFunction(()=>!assistBusy);
    await p.evaluate(()=>{
      window.recipeApi=api;
      api=async(path,data)=>{const response=await window.recipeApi(path,data);if(path.startsWith('/api/v1/assist/turns')&&!window.recipeCaptured){window.recipeCaptured=true;await new Promise(resolve=>window.recipeRelease=resolve)}return response};
      loadAssistTurns();
    });
    await p.waitForFunction(()=>window.recipeCaptured);
    await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
    await p.evaluate(()=>window.recipeRelease());await p.waitForFunction(()=>!assistBusy);
    if(!await p.$eval('#assist-ask',e=>e.disabled)||!await p.evaluate(()=>assistPending))fail('Une ancienne lecture masque la question active');
    await p.evaluate(()=>{api=window.recipeApi});
    record('lecture tardive de l’historique ne masque pas une nouvelle question','Réponse antérieure retardée puis délivrée après confirmation ; attente et annulation restent visibles');
    await p.waitForSelector('#assist-cancel',{visible:true});
    await (await p.$('#assistant')).screenshot({path:shots+'/sombre-running.png'});
    await p.click('#assist-cancel');await p.waitForFunction(()=>document.querySelector('#modal').open&&document.querySelector('#modal-title').textContent.includes('Arrêter la question'));
    await p.click('#confirm');await p.waitForFunction(()=>document.querySelector('#assist-answer').textContent.includes('Question annulée'));
    await p.waitForFunction(()=>document.querySelector('#assist-cancel').hidden);
    record('annulation explicite, état conservé et pas de relance automatique','Bouton Arrêter la question et confirmation');


    const timeoutAnswer=await ask('fixture-timeout','agents');if(!timeoutAnswer.includes('Délai de 1 secondes dépassé'))fail('Délai configuré non appliqué');
    record('délai configuré et erreur de service visibles','Une seconde dans la fixture, sans relance');
    await p.click('[data-view="agents"]');await p.click('#agents [data-assist-page]');await p.waitForSelector('#modal[open] #field-provider',{visible:true});await p.select('#field-provider','fixture-restart');await p.click('#confirm');await p.waitForFunction(()=>document.querySelector('#confirm').textContent==='Confirmer l’envoi à l’IA');await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open);
    await p.waitForFunction(()=>[...document.querySelectorAll('#assist-history details summary')].some(x=>x.textContent.includes('running')));
    const ended=new Promise(resolve=>server.once('exit',resolve));server.kill();await ended;
    let restarted='';server=cp.spawn(binary,['--root',fixture.root,'web',new URL(url).host]);server.stdout.on('data',d=>restarted+=d);const until=Date.now()+10000;while(!restarted.includes('/session/')){if(Date.now()>until)fail('Redémarrage impossible');await new Promise(r=>setTimeout(r,100))}
    await p.goto(restarted.match(/http:\/\/\S+/)[0]);await p.waitForFunction(()=>document.querySelector('#assist-state').textContent.includes('Structure et références'),{timeout:25000});
    record('question conservée après redémarrage du cockpit','Superviseur indépendant, même port et résultat retrouvé sans nouveau lancement');

    // Accept the fixture evidence by the normal review flow, then change the
    // underlying file without a work event. Hash freshness must still react.
    await p.click('[data-view="tasks"]');
    const taskForm=async action=>{await p.click('[data-task="AS-01"] button');await p.waitForSelector('#modal[open] #field-action',{visible:true});await p.waitForFunction(()=>document.querySelector('#modal-title').textContent.startsWith('AS-01'));await p.waitForFunction(x=>[...document.querySelectorAll('#field-action option')].some(o=>o.value===x),{},action);await p.select('#field-action',action);await p.waitForFunction(x=>document.querySelector('#field-action').value===x,{},action)};
    const finishForm=async()=>{await p.click('#confirm');await p.waitForFunction(()=>!document.querySelector('#modal').open||!document.querySelector('#modal-error').hidden);if(await p.$eval('#modal',e=>e.open))fail(await p.$eval('#modal-error',e=>e.textContent))};
    await taskForm('submit');await p.select('#field-path','docs/AS-01-handoff.md');await finishForm();
    await taskForm('gate');await p.select('#field-path','docs/AS-01.evidence.json');await p.type('#field-name','Recette assistant');await p.click('#confirm');await p.waitForFunction(()=>document.querySelector('#confirm').textContent==='Confirmer l’enregistrement');await finishForm();
    await taskForm('accepted');await finishForm();
    await p.select('#assist-target','AS-01');await ask('fixture-ok','tasks');
    fs.appendFileSync(fixture.root+'/docs/AS-01-handoff.md','\nModification externe de recette.\n');
    await p.click('#refresh');await p.waitForFunction(()=>document.querySelector('#assist-answer').textContent.includes('Réponse périmée'));
    if(!await p.$$eval('#assist-answer .assist-step > button',xs=>xs.length>0&&xs.every(x=>x.disabled)))fail('Action historique encore active après dérive sans révision');
    await (await p.$('#assistant')).screenshot({path:shots+'/sombre-stale.png'});
    record('dérive de fichier sans modification de révision détectée','Réponse marquée périmée et formulaires proposés désactivés');

    // Full export via the visible button, and context access without any provider.
    const download=fs.mkdtempSync('/tmp/swarm-assist-download-');const client=await p.createCDPSession();await client.send('Page.setDownloadBehavior',{behavior:'allow',downloadPath:download});
    if(!await p.$eval('.assist-history',e=>e.open))await p.click('.assist-history > summary');
    await p.click('#assist-export');const dest=download+'/swarm-assistant-'+fixture.work+'.json';const deadline=Date.now()+10000;while(!fs.existsSync(dest)){if(Date.now()>deadline)fail('Export non téléchargé');await new Promise(r=>setTimeout(r,100))}
    const exported=JSON.parse(fs.readFileSync(dest,'utf8'));if(exported.turns.length<9||exported.turns.some(t=>!t.context||!t.prompt||!t.question))fail('Export incomplet');
    const config=fixture.root+'/.swarm/providers.json',original=fs.readFileSync(config);fs.writeFileSync(config,JSON.stringify({schema_version:1,providers:{}}));
    await p.click('#refresh');await p.click('#assist-context');await p.waitForFunction(()=>document.querySelector('#modal').open&&document.querySelector('#modal-title').textContent.includes('Contexte exact'));
    if(!await p.$eval('#preview',e=>e.textContent.includes('page-context.v1')))fail('Contexte illisible sans fournisseur');await p.click('#cancel');fs.writeFileSync(config,original);
    record('export complet et contexte consultable sans IA configurée',{turns:exported.turns.length});

    fs.writeFileSync(report, JSON.stringify({
      status: 'PASS', binary, root: fixture.root, work: fixture.work,
      scope: 'Recette navigateur de l’assistant de page, fournisseur de fixture local ; aucun appel de modèle facturé, aucun fichier du projet modifié.',
      checks: results, history_sample: history.slice(0, 400),
    }, null, 2));
    console.log('\nAssistant de page : ' + results.length + ' contrôles PASS · rapport ' + report);
  } finally {
    await b.close();
    server.kill();
  }
})().catch(e => { console.error('FAIL ' + e.message); process.exitCode = 1; });
