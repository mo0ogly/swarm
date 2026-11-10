'use strict';
/** Rendered-contract browser reproducer for the PAEA approval form:
 * 1. THEME-SINGLE-TOGGLE: paea-effects.html must switch theme with ONE click
 *    (etat -> sombre -> etat). Double-attached handlers make one click a
 *    no-op; the regression was paea-approval-form.js attaching a second
 *    #paea-theme listener beside paea-effects.js on the same page.
 * 2. DARK-COMPUTED: after the toggle the computed body/card/status colors
 *    must actually change theme (not just the data-theme attribute) and the
 *    measured foreground/background pairs must meet WCAG AA (4.5:1 text,
 *    3:1 large/UI) in BOTH themes; the sombre body background must be the
 *    shared dark token, not the historical white page shell.
 * 3. EN-RENDERED: the approval-section texts use curly apostrophes (repo
 *    convention) so the EN catalog keys apply; under ?lang=en the visible
 *    text must not remain the FR source string. The consent notice carries
 *    BOTH apostrophe variants as catalog keys (the curly one is what the
 *    page ships; the straight one is kept for the catalog's other uses).
 * 4. NO-EMPTY-LABEL-ROWS: the declared projection renders one dt/dd pair per
 *    present term value. The real presentation route derives recipe terms
 *    with deliberately EMPTY action_id/content_digest (the command identity
 *    is the request digest); a labeled empty row would present a private
 *    field shape the projection never serves. Reproduced against the exact
 *    served payload shape via an intercepted route on a static file server
 *    (no engine, no enrollment, no private data — the fixture terms are the
 *    public grammar: binding/resource/amount/currency/beneficiary/revision).
 * Scope limits: static asset qualification only (no live engine, no
 * enrollment, no Git effect, no identity or acceptance claim). */
const puppeteer=require('/home/fpizzi/Documents/ChatGPT/swarm/node_modules/puppeteer');
const http=require('http'),fs=require('fs'),path=require('path');
const root='/home/fpizzi/Documents/ChatGPT/swarm/web';
const mime={'.css':'text/css','.js':'text/javascript','.html':'text/html','.png':'image/png'};
const srv=http.createServer((req,res)=>{
  let p=req.url.split('?')[0]; if(p==='/')p='/paea-effects.html';
  const f=path.join(root,p);
  if(!fs.existsSync(f)){res.writeHead(404);return res.end('not found')}
  res.writeHead(200,{'content-type':mime[path.extname(f)]||'application/octet-stream'});
  res.end(fs.readFileSync(f));
});
const checks=[],errors=[];
const check=(name,ok,detail)=>{checks.push({name,ok,detail});if(!ok)errors.push(name+': '+detail)};
// WCAG 2.x relative luminance / contrast ratio on 8-bit sRGB triples.
const lum=([r,g,b])=>{
  const f=c=>{c/=255;return c<=0.04045?c/12.92:((c+0.055)/1.055)**2.4};
  return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b);
};
const ratio=(a,b)=>{
  const [hi,lo]=[lum(a),lum(b)].sort((x,y)=>y-x);
  return (hi+0.05)/(lo+0.05);
};
const parse=s=>{
  const m=s.match(/rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/);
  return m?[+m[1],+m[2],+m[3]]:null;
};
(async()=>{
  const base='http://127.0.0.1:'+await new Promise(r=>srv.listen(0,'127.0.0.1',()=>r(srv.address().port)));
  const b=await puppeteer.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
  try{
    const pg=await b.newPage();
    // Control: one click must leave the page in "sombre".
    await pg.goto(base+'/paea-effects.html?theme=etat',{waitUntil:'networkidle0'});
    await pg.click('#paea-theme');
    const theme1=await pg.evaluate(()=>document.documentElement.dataset.theme);
    check('single-click-switches-theme',theme1==='sombre','after one click data-theme='+theme1);
    await pg.click('#paea-theme');
    const theme2=await pg.evaluate(()=>document.documentElement.dataset.theme);
    check('second-click-returns-etat',theme2==='etat','after second click data-theme='+theme2);
    // Stable-state measurements only: the theme button carries a .15s
    // background-color transition (paea.css `button` rule). Colors sampled
    // immediately after the clicks land mid-interpolation (observed 2026-10-31:
    // rgb(113,130,137) on the real cascade sombre #153344 → etat #eeeee7 at
    // t≈0.42, a false 2.85 contrast against the static pairs' 9.79/11.14), so
    // every awaited measurement first waits for the REAL end of the
    // transitions on the measured element. The wait is condition-based (no
    // arbitrary sleep): each running CSSTransition is awaited to its real
    // `finished`, and the exit additionally requires the computed background
    // to equal the element's own current --wattson-carte-appuyee token at
    // both of two consecutive frames — a mid-flight blend and a not-yet-started
    // old value both fail that test. The wait MUST be passed to
    // page.evaluate as a real function object: a source STRING is evaluated
    // as an expression and never invoked (silent no-op, diagnosed in
    // tests/paea-rendered-contract-transition-guard.cjs runs 2–6).
    const settledBody=async sel=>{
      const SETTLE_FRAME_BOUND=600; // ≈10 s at 60 fps; coordination bound only
      const node=document.querySelector(sel);
      const bg=()=>getComputedStyle(node).backgroundColor;
      const want=()=>{const t=getComputedStyle(node).getPropertyValue('--wattson-carte-appuyee').trim();
        const m=t.match(/^#([0-9a-f]{6})$/i);
        return m?'rgb('+parseInt(m[1].slice(0,2),16)+', '+parseInt(m[1].slice(2,4),16)+', '+parseInt(m[1].slice(4,6),16)+')':t};
      const nextFrame=()=>new Promise(r=>requestAnimationFrame(r));
      for(let frame=0;;frame++){
        if(frame>=SETTLE_FRAME_BOUND) throw new Error('settled-wait-no-convergence after '+frame+' frames (bg='+bg()+' want='+want()+')');
        const anims=node.getAnimations({subtree:true});
        if(anims.length) await Promise.all(anims.map(a=>a.finished.catch(()=>{})));
        const c1=bg(), w1=want();
        if(c1!==w1){ await nextFrame(); continue; }
        await nextFrame();
        if(bg()===c1 && want()===w1) return;
      }
    };
    await pg.evaluate(settledBody,'#paea-theme');
    // Colors must be computed per theme, not a dataset-only assertion: the
    // sombre body is the shared dark token; etat is its light counterpart.
    const computed=await pg.evaluate(()=>{
      const cs=e=>getComputedStyle(e);
      const pick=()=>({
        bodyBg:cs(document.body).backgroundColor,
        bodyColor:cs(document.body).color,
        cardBg:cs(document.querySelector('main section')).backgroundColor,
        cardColor:cs(document.querySelector('main h2')).color,
        noticeBg:cs(document.querySelector('.paea-notice')).backgroundColor,
        noticeColor:cs(document.querySelector('.paea-notice')).color,
        statusBg:cs(document.querySelector('#approval-status')).backgroundColor,
        statusColor:cs(document.querySelector('#approval-status')).color,
        btnBg:cs(document.querySelector('#paea-theme')).backgroundColor,
        btnColor:cs(document.querySelector('#paea-theme')).color,
      });
      return pick();
    });
    check('etat-body-light',computed.bodyBg==='rgb(245, 243, 236)','etat body background='+computed.bodyBg);
    // Re-enter sombre through the page's own URL param (same startup path a
    // toggled navigation link produces) and take the dark measurements there.
    await pg.goto(base+'/paea-effects.html?theme=sombre',{waitUntil:'networkidle0'});
    await pg.evaluate(settledBody,'#paea-theme');
    const sombre=await pg.evaluate(()=>{
      const cs=e=>getComputedStyle(e);
      return {
        bodyBg:cs(document.body).backgroundColor,
        bodyColor:cs(document.body).color,
        cardBg:cs(document.querySelector('main section')).backgroundColor,
        cardColor:cs(document.querySelector('main h2')).color,
        noticeBg:cs(document.querySelector('.paea-notice')).backgroundColor,
        noticeColor:cs(document.querySelector('.paea-notice')).color,
        statusBg:cs(document.querySelector('#approval-status')).backgroundColor,
        statusColor:cs(document.querySelector('#approval-status')).color,
        btnBg:cs(document.querySelector('#paea-theme')).backgroundColor,
        btnColor:cs(document.querySelector('#paea-theme')).color,
      };
    });
    check('sombre-body-dark',sombre.bodyBg==='rgb(8, 27, 41)','sombre body background='+sombre.bodyBg);
    for(const [name,a,b] of [
      ['etat body',computed.bodyColor,computed.bodyBg],
      ['etat card',computed.cardColor,computed.cardBg],
      ['etat notice',computed.noticeColor,computed.noticeBg],
      ['etat status',computed.statusColor,computed.statusBg],
      ['etat button',computed.btnColor,computed.btnBg],
      ['sombre body',sombre.bodyColor,sombre.bodyBg],
      ['sombre card',sombre.cardColor,sombre.cardBg],
      ['sombre notice',sombre.noticeColor,sombre.noticeBg],
      ['sombre status',sombre.statusColor,sombre.statusBg],
      ['sombre button',sombre.btnColor,sombre.btnBg],
    ]){
      const fa=parse(a),fb=parse(b);
      const r=fa&&fb?ratio(fa,fb):0;
      check('aa-contrast-'+name.toLowerCase().replace(/\s+/g,'-'),r>=4.5,name+' contrast='+r.toFixed(2)+' ('+a+' on '+b+')');
    }
    // Large text (WCAG 1.4.3 large-scale: ≥24px, or ≥18.66px bold) uses the
    // 3:1 threshold; smaller headings still need 4.5:1. The threshold follows
    // the COMPUTED size/weight, not an assumption about the stylesheet.
    for(const [theme,view] of [['etat',computed],['sombre',sombre]]){
      const dim=await pg.evaluate(()=>{
        const h=document.querySelector('main h2'),cs=getComputedStyle(h);
        return {size:parseFloat(cs.fontSize),weight:+cs.fontWeight,color:cs.color,bg:getComputedStyle(h.closest('section')).backgroundColor};
      });
      const large=dim.size>=24||(dim.size>=18.66&&dim.weight>=700);
      const fa=parse(dim.color),fb=parse(dim.bg);
      const r=fa&&fb?ratio(fa,fb):0;
      check('aa-heading-'+theme,r>=(large?3:4.5),theme+' h2 '+dim.size+'px/'+dim.weight+' threshold='+(large?3:4.5)+' contrast='+r.toFixed(2));
    }
    // EN rendering of the approval section (curly apostrophes in HTML).
    await pg.goto(base+'/paea-effects.html?lang=en',{waitUntil:'networkidle0'});
    const en=await pg.evaluate(()=>({
      title:document.querySelector('#approval-title')?.textContent.trim(),
      notice:document.querySelector('#approval-agreement')?.textContent.trim(),
      confirm:document.querySelector('label[for="approval-confirm"] span')?.textContent.trim(),
      password:document.querySelector('label[for="approval-password"] span')?.textContent.trim(),
      send:document.querySelector('#approval-send')?.textContent.trim()
    }));
    check('en-title',en.title==='Approval of the prepared action — explicit confirmation','EN title='+en.title);
    check('en-notice',en.notice==='No consent is captured by this display: the confirmation below is the explicit act that consumes the frozen command under the held enrolled authority.','EN notice='+en.notice);
    check('en-confirm',en.confirm==='I confirm the action presented above, for its exact terms.','EN confirm='+en.confirm);
    check('en-password',en.password==='Approver enrollment password','EN password='+en.password);
    check('en-send',en.send==='Approve the prepared action','EN send='+en.send);
    // Declared projection: the real route derives recipe terms whose
    // action_id/content_digest are EMPTY strings. No dt may be rendered for
    // them and no dd may be empty — every label shows a present value.
    await pg.setRequestInterception(true);
    const present=req=>{
      const u=req.url();
      const json=(body)=>req.respond({status:200,contentType:'application/json',body:JSON.stringify(body)});
      if(u.endsWith('/api/v1/native-session'))return json({schema_version:1,csrf_token:'0'.repeat(64)});
      if(u.endsWith('/api/v1/paea/git-call/approval'))return json({bound:true});
      if(u.endsWith('/api/v1/paea/git-call/approval/presentation'))return json({
        schema_version:1,event:'paea-git-call-approval-presentation',available:true,
        operation:'approve',request_digest:'d'.repeat(64),
        canonical:{schema:'paea-json-v1',action_digest:'d'.repeat(64),intent_id:'repo',policy_digest:'p'.repeat(64),digest_matches:true},
        agreement_terms:{binding:'recipe',action_id:'',content_digest:'',resource:'fixture/repo@refs/heads/main',amount_minor:1,currency:'OID',beneficiary:'old-oid',revision:7},
        current:{roots_bound:true,trust_revision:7,declared_trust_revision:7,trust_revision_current:true},
        limits:{consent:'none-captured',consumption:'none',human_presence:'not-established'}});
      req.continue();
    };
    pg.on('request',present);
    await pg.goto(base+'/paea-effects.html',{waitUntil:'networkidle0'});
    await pg.waitForFunction(()=>document.getElementById('approval-status').dataset.state!=='loading');
    const rows=await pg.evaluate(()=>[...document.querySelectorAll('#approval-declared dt,#approval-declared dd')].map(x=>x.textContent));
    check('no-empty-value-rows',rows.length>0&&rows.every(t=>t.trim()!==''),'declared rows='+JSON.stringify(rows));
    check('no-private-term-labels',!rows.some(t=>['Action','Mandat','Empreinte exacte du contenu'].includes(t.trim())),'private command labels must not render as term rows');
    // Visible refusals, FR and EN: a stale trust revision must show its typed
    // stale-approval message and disable the send button; the EN variant must
    // be the translated refusal, not the FR source string. The stale shape is
    // the real current witness grammar (trust_revision vs declared), fixture
    // values only — no live engine, no private command content.
    await pg.setRequestInterception(true);
    pg.off('request',present);
    const refuse=req=>{
      const u=req.url();
      const json=(body)=>req.respond({status:200,contentType:'application/json',body:JSON.stringify(body)});
      if(u.endsWith('/api/v1/native-session'))return json({schema_version:1,csrf_token:'0'.repeat(64)});
      if(u.endsWith('/api/v1/paea/git-call/approval'))return json({bound:true});
      if(u.endsWith('/api/v1/paea/git-call/approval/presentation'))return json({
        schema_version:1,event:'paea-git-call-approval-presentation',available:true,
        operation:'approve',request_digest:'d'.repeat(64),
        canonical:{schema:'paea-json-v1',action_digest:'d'.repeat(64),intent_id:'repo',policy_digest:'p'.repeat(64),digest_matches:true},
        agreement_terms:{binding:'recipe',action_id:'',content_digest:'',resource:'fixture/repo@refs/heads/main',amount_minor:1,currency:'OID',beneficiary:'old-oid',revision:7},
        current:{roots_bound:true,trust_revision:8,declared_trust_revision:7,trust_revision_current:false},
        limits:{consent:'none-captured',consumption:'none',human_presence:'not-established'}});
      req.continue();
    };
    pg.on('request',refuse);
    await pg.goto(base+'/paea-effects.html?lang=fr',{waitUntil:'networkidle0'});
    await pg.waitForFunction(()=>document.getElementById('approval-status').dataset.state==='error');
    const staleFr=await pg.evaluate(()=>({
      text:document.getElementById('approval-status').textContent,
      sendDisabled:document.getElementById('approval-send').disabled
    }));
    check('stale-refusal-visible-fr',staleFr.text.startsWith('Approbation périmée : la révision de confiance observée diffère de la révision déclarée.')&&staleFr.sendDisabled===true,'FR stale='+JSON.stringify(staleFr));
    await pg.goto(base+'/paea-effects.html?lang=en',{waitUntil:'networkidle0'});
    await pg.waitForFunction(()=>document.getElementById('approval-status').dataset.state==='error');
    const staleEn=await pg.evaluate(()=>({
      text:document.getElementById('approval-status').textContent,
      sendDisabled:document.getElementById('approval-send').disabled
    }));
    check('stale-refusal-visible-en',staleEn.text.startsWith('Stale approval: the observed trust revision differs from the declared one.')&&staleEn.sendDisabled===true,'EN stale='+JSON.stringify(staleEn));
    pg.off('request',refuse);
  }catch(error){errors.push('fatal: '+error.message)}
  finally{
    await b.close();srv.close();
    const out={checks,errors,verified:errors.length===0,utc:new Date().toISOString()};
    console.log(JSON.stringify(out,null,1));
    process.exit(out.verified?0:1);
  }
})();
