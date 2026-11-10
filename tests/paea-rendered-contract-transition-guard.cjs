'use strict';
/** Targeted regression guard for the rendered-contract stable-state wait.
 *
 * Reproducer (observed 2026-10-31, supervisor-direct-rendered run): the etat
 * theme button keeps a background-color transition (paea.css `button` rule,
 * .15s — the last-loaded shorthand, so it also overrides swarm-design.css'
 * .14s background+color one) and the sampled pair landed mid-interpolation:
 * ink rgb(36,61,74) was ALREADY the final etat ink (the effective transition
 * covers only background/border, so ink snapped instantly) while background
 * rgb(113,130,137) ≈ sombre #153344 → etat #eeeee7 at t≈0.42 — a false 2.85
 * WCAG ratio although the static etat pair holds 9.79 and sombre 11.14.
 * The effective tokens come from the real cascade
 * wattson_themes.css → swarm-design.css → paea.css, measured in this checkout
 * (probe in .swarm-fixtures/glm-proof-contract-guard-20261031T19/): sombre
 * button rgb(21,51,68)/rgb(229,237,240), etat button rgb(238,238,231)/rgb(36,61,74).
 *
 * Diagnosis evidence, all kept in the same fixture directory:
 * - trace-frames.cjs: per-frame page truth after one toggle — the click is
 *   applied ~5 rAF frames AFTER pg.click resolves (pre-recalc window: old
 *   tokens, no animation visible yet), then the .15s transition runs and the
 *   page converges exactly at frame ~14. No reverse, no hang.
 * - trace-exact-loop.cjs: the token-equality settle loop, passed to
 *   page.evaluate as a REAL function, awaits the running CSSTransitions to
 *   their `finished` and exits exactly at the final static color.
 * - Guard runs 2–6 failed for a driver reason worth recording: passing a
 *   SOURCE STRING to page.evaluate (function.toString() wrapped in parens)
 *   evaluates it as an expression and never invokes it — the selector
 *   argument is silently dropped, so the "wait" was a no-op. Any stable-state
 *   wait must be passed to page.evaluate as a real function object.
 * - Gates: the data-theme attribute is NOT a valid change-begun gate (it
 *   flips synchronously in the click handler while used values still hold
 *   the old theme); the USED token value --wattson-carte-appuyee is, because
 *   it updates at the same style recalculation that registers the transition.
 *
 * The fix proven here is condition-based (no arbitrary sleep):
 * 1. CHANGE-BEGUN GATE: after a toggle, first wait (rAF polling) for the used
 *    token value to reach the expected theme color — a pre-change sample can
 *    no longer be mistaken for a settled one.
 * 2. TOKEN-SETTLED WAIT: the button's static contract is
 *    `background:var(--wattson-carte-appuyee)` (paea.css `button` rule), so
 *    the wait — a real function object passed to page.evaluate — ends only
 *    when no CSSTransition remains (each awaited to its real `finished`) AND
 *    the computed background EQUALS the element's own current token value at
 *    BOTH of two consecutive frames. A mid-flight blend and a not-yet-started
 *    old value both fail that test. The only bound is a loud
 *    test-coordination failure if the page never converges (never a pass).
 *
 * Scope: static page + CSS only; no engine, no enrollment, no Git effect,
 * no acceptance. Exit 0 only when every check holds; failures keep details.
 */
const puppeteer=require('/home/fpizzi/Documents/ChatGPT/swarm/node_modules/puppeteer');
const http=require('http'),fs=require('fs'),path=require('path');
const root='/home/fpizzi/Documents/ChatGPT/swarm/web';
const mime={'.css':'text/css','.js':'text/javascript','.html':'text/html'};
const srv=http.createServer((req,res)=>{
  let p=req.url.split('?')[0]; if(p==='/')p='/paea-effects.html';
  const f=path.join(root,p);
  if(!fs.existsSync(f)){res.writeHead(404);return res.end('not found')}
  res.writeHead(200,{'content-type':mime[path.extname(f)]||'application/octet-stream'});
  res.end(fs.readFileSync(f));
});
const checks=[],errors=[];
const check=(name,ok,detail)=>{checks.push({name,ok,detail});if(!ok)errors.push(name+': '+detail)};
const parse=s=>{const m=s.match(/rgba?\(([\d.]+)[,\s]+([\d.]+)[,\s]+([\d.]+)/);return m?[+m[1],+m[2],+m[3]]:null};
const lum=([r,g,b])=>{const f=c=>{c/=255;return c<=0.04045?c/12.92:((c+0.055)/1.055)**2.4};return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)};
const ratio=(a,b)=>{const[hi,lo]=[lum(a),lum(b)].sort((x,y)=>y-x);return (hi+0.05)/(lo+0.05)};
// Token-settled wait, run in the page. MUST be passed to page.evaluate as a
// real function object (see header note on the string-expression no-op).
const SETTLE_FRAME_BOUND=600; // ≈10 s at 60 fps, exceeds any shipped transition
const settledBody=async sel=>{
  const SETTLE_FRAME_BOUND=600; // serialized into the page with the function
  const node=document.querySelector(sel);
  const bg=()=>getComputedStyle(node).backgroundColor;
  // Static contract of the measured control: background:var(--wattson-carte-appuyee).
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
// Change-begun gate: poll the USED VALUE of the token until it reaches the
// expected theme color (see header: the data-theme attribute is not a valid
// gate — it flips before the used values do).
const SETTLE_MS=10000; // same coordination bound as SETTLE_FRAME_BOUND
const themeGate=(page,expectedHex)=>page.waitForFunction(
  hex=>getComputedStyle(document.documentElement).getPropertyValue('--wattson-carte-appuyee').trim()===hex,
  {polling:'raf',timeout:SETTLE_MS},expectedHex);
(async()=>{
  const base='http://127.0.0.1:'+await new Promise(r=>srv.listen(0,'127.0.0.1',()=>r(srv.address().port)));
  const b=await puppeteer.launch({headless:'new',executablePath:process.env.CHROME_BIN||'/usr/bin/google-chrome',args:['--no-sandbox','--disable-dev-shm-usage']});
  try{
    const pg=await b.newPage();
    // Page's own startup path: sombre tokens, then one toggle to etat — the
    // same gesture that raced the button transition in the failure.
    await pg.goto(base+'/paea-effects.html?theme=sombre',{waitUntil:'networkidle0'});
    await pg.click('#paea-theme');
    await themeGate(pg,'#eeeee7');
    // CONTROL: sample IMMEDIATELY after the change has begun. Any color is
    // legal here; the assertion is only that the observation did not depend
    // on timing luck.
    const early=await pg.evaluate(()=>getComputedStyle(document.querySelector('#paea-theme')).backgroundColor);
    const earlyState=early==='rgb(238, 238, 231)'?'already-final':early==='rgb(21, 51, 68)'?'old-value-t0':'mid-transition';
    check('control-immediate-sample-observed',true,'immediately after the gated toggle, button background='+early+' ('+earlyState+')');
    // GUARD-STABLE (first pass): wait real transition end, then measure.
    await pg.evaluate(settledBody,'#paea-theme');
    const stable=await pg.evaluate(()=>getComputedStyle(document.querySelector('#paea-theme')).backgroundColor);
    check('guard-reaches-static-etat-token',stable==='rgb(238, 238, 231)','after settled wait, button background='+stable+' (early sample was '+earlyState+')');
    const pair=await pg.evaluate(()=>({color:getComputedStyle(document.querySelector('#paea-theme')).color,bg:getComputedStyle(document.querySelector('#paea-theme')).backgroundColor}));
    const r=parse(pair.color)&&parse(pair.bg)?ratio(parse(pair.color),parse(pair.bg)):0;
    check('guard-static-pair-holds-aa',r>=4.5,'static etat button pair contrast='+r.toFixed(2)+' ('+pair.color+' on '+pair.bg+')');
    // A second toggle + settle must land on the sombre token: the wait works
    // in both directions, not once by luck.
    await pg.click('#paea-theme');
    await themeGate(pg,'#153344');
    await pg.evaluate(settledBody,'#paea-theme');
    const stableSombre=await pg.evaluate(()=>getComputedStyle(document.querySelector('#paea-theme')).backgroundColor);
    check('guard-reaches-static-sombre-token',stableSombre==='rgb(21, 51, 68)','after second settled wait, button background='+stableSombre);
    // REDUCED-MOTION: no transition ships at all; the same gated wait must
    // resolve (bounded test-coordination window, not a product deadline) and
    // the static etat token must still be reached after a toggle.
    const reduced=await b.createBrowserContext();
    const rp=await reduced.newPage();
    // Reduced-motion is emulated per context: page.emulateMediaFeatures
    // honors prefers-reduced-motion.
    await rp.emulateMediaFeatures([{name:'prefers-reduced-motion',value:'reduce'}]);
    await rp.goto(base+'/paea-effects.html?theme=etat',{waitUntil:'networkidle0'});
    const reducedStart=Date.now();
    await rp.evaluate(settledBody,'#paea-theme');
    const reducedWait=Date.now()-reducedStart;
    check('reduced-motion-wait-resolves-immediately',reducedWait<1000,'reduced-motion settled wait took '+reducedWait+'ms');
    await rp.click('#paea-theme');
    await themeGate(rp,'#153344');
    await rp.evaluate(settledBody,'#paea-theme');
    const reducedColor=await rp.evaluate(()=>getComputedStyle(document.querySelector('#paea-theme')).backgroundColor);
    check('reduced-motion-reaches-static-sombre',reducedColor==='rgb(21, 51, 68)','reduced-motion after toggle, button background='+reducedColor);
    await reduced.close();
  }catch(error){errors.push('fatal: '+error.message)}
  finally{
    await b.close();srv.close();
    console.log(JSON.stringify({checks,errors,verified:errors.length===0,utc:new Date().toISOString()},null,1));
    process.exit(errors.length===0?0:1);
  }
})();
