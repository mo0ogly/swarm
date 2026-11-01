'use strict';
/** Exercise analyst filtering and group placement through actual controls.
 * Called by the native isolated-Root browser recipe for every language/theme.
 * Only a labeled multi-hop projection fixture uses extra in-memory cards; real
 * Work readback restores it. No business mutation or provider request occurs.
 */
module.exports=async function analystJourney(page,label,cfg,check){
 const fs=require('node:fs'),path=require('node:path');
 const test=(name,value)=>check(label+'-analyst-'+name,value);
 const settle=()=>page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
 async function openPanel(){if(!await page.$eval('#pilot-analysis',e=>e.open))await page.click('#pilot-analysis-toggle')}
 async function reveal(selector){await openPanel();await page.$eval(selector,e=>{const section=e.closest('details');if(section&&!section.open)section.querySelector('summary').click()})}
 async function click(selector){await reveal(selector);await page.click(selector)}
 async function select(selector,value){await reveal(selector);return page.select(selector,value)}
 async function tools(){if(!await page.$eval('#pilot-tools',e=>e.open))await page.click('#pilot-tools-toggle')}
 const ids=()=>page.$$eval('.graph-unit:not(.graph-analysis-hidden)',nodes=>nodes.map(n=>n.dataset.task).sort());
 const placement=()=>page.$eval('#graph',e=>e.graphView().positions);
 const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
 const fullIds=await ids();
 test('control',!!await page.$('#pilot-analysis'));
 await openPanel();test('empty-selects-disable-actions',await page.$eval('#pilot-branch-apply',e=>e.disabled)&&await page.$eval('#pilot-path-apply',e=>e.disabled));
 await select('#pilot-branch','b');await click('#pilot-branch-apply');
 test('branch-excludes-sibling',same(await ids(),['a','b']));
 test('branch-count-visible',await page.$eval('#pilot-analysis-status',(e,total)=>e.textContent.includes('2 / '+total),fullIds.length));
 test('hidden-card-not-rendered',await page.$eval('.graph-unit[data-task="c"]',e=>getComputedStyle(e).display==='none'&&e.getAttribute('aria-hidden')==='true'));
 await page.$eval('.graph-port[data-task="b"][data-side="out"]',e=>e.focus());await page.keyboard.press('Tab');test('hidden-card-skipped-by-keyboard',await page.evaluate(()=>!document.activeElement.closest('.graph-analysis-hidden')));
 test('minimap-follows-filter',await page.$$eval('.graph-minimap-node',nodes=>nodes.length===2));
 await page.type('#pilot-search','cafe');test('search-excludes-hidden',await page.$eval('#pilot-search-next',e=>e.disabled));
 await openPanel();await click('#pilot-analysis-clear');test('clear-restores-all',same(await ids(),fullIds));test('clear-recomputes-search',!await page.$eval('#pilot-search-next',e=>e.disabled));await page.click('#pilot-search-clear');
 await openPanel();await select('#pilot-path-from','a');await select('#pilot-path-to','b');await click('#pilot-path-apply');test('directed-path',same(await ids(),['a','b']));
 await select('#pilot-path-from','b');await select('#pilot-path-to','c');await click('#pilot-path-apply');test('no-path-keeps-view',same(await ids(),['a','b'])&&await page.$eval('#pilot-analysis-status',e=>e.textContent.includes(document.documentElement.lang==='en'?'No directed path':'Aucun chemin orienté')));
 await select('#pilot-path-to','b');await click('#pilot-path-apply');test('same-endpoint-single-card',same(await ids(),['b']));
 await click('#pilot-analysis-clear');
 // Multi-hop and diamond fixture: all connecting routes must be retained,
 // including prototype-shaped IDs and a disconnected card. Never persisted.
 await page.evaluate(async work=>{const original=await(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json();window.analystOriginal=original;const seed=original.tasks[0];SwarmRustDependencyGraph.render({...original,tasks:[{...seed,id:'__proto__',depends:[]},{...seed,id:'b',depends:['__proto__']},{...seed,id:'c',depends:['__proto__']},{...seed,id:'d',depends:['b','c']},{...seed,id:'e',depends:['d']},{...seed,id:'isolated',depends:[]}]})},cfg.work);
 await openPanel();await select('#pilot-path-from','__proto__');await select('#pilot-path-to','e');await click('#pilot-path-apply');test('projection-fixture-all-diamond-paths',same(await ids(),['__proto__','b','c','d','e']));test('projection-fixture-all-path-edges',await page.$$eval('.graph-arete:not(.graph-analysis-hidden)',nodes=>nodes.length===5));
 await select('#pilot-branch','d');await click('#pilot-branch-apply');test('projection-fixture-transitive-branch',same(await ids(),['__proto__','b','c','d','e']));
 await page.evaluate(()=>{SwarmRustDependencyGraph.render(window.analystOriginal);delete window.analystOriginal});
 // Fullscreen gives real handles a stable visible gesture surface. Collapse
 // disclosures before measuring; no viewport-changing diagnostic connection.
 await openPanel();await click('#pilot-selection-mode');
 await page.$eval('.graph-task-open[data-task="a"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.$eval('.graph-task-open[data-task="b"]',e=>e.focus());await page.keyboard.press('Space');
 test('keyboard-selection-without-editor',await page.$$eval('.graph-multi-selected',nodes=>nodes.length===2)&&!await page.$('.native-graph-editor'));
 test('selection-pressed-and-text',await page.$eval('.graph-task-open[data-task="b"]',e=>e.getAttribute('aria-pressed')==='true')&&await page.$eval('#pilot-selection-status',e=>e.textContent.includes('a, b')));
 await click('#pilot-selection-mode');test('leave-mode-keeps-group',await page.$$eval('.graph-multi-selected',nodes=>nodes.length===2)&&await page.$eval('.graph-task-open[data-task="a"]',e=>!e.hasAttribute('aria-pressed')));
 await click('#pilot-analysis-close');await tools();await page.click('#pilot-layout-reset');const initial=await placement();
 await page.$eval('.graph-drag[data-task="a"]',e=>e.focus());await page.keyboard.press('ArrowRight');const moved=await placement();test('keyboard-group-common-translation',moved.a.x===16&&moved.b.x===16&&!moved.c);
 await tools();await page.click('#pilot-layout-undo');test('group-undo',same(await placement(),initial));await page.click('#pilot-layout-redo');test('group-redo',same(await placement(),moved));
 await page.$eval('.graph-drag[data-task="c"]',e=>e.focus());await page.keyboard.press('ArrowDown');const outside=await placement();test('unselected-handle-moves-alone',same(outside.a,moved.a)&&same(outside.b,moved.b)&&outside.c.y===16);
 await tools();await page.click('#pilot-layout-reset');await page.click('#pilot-layout-undo');const before=await placement();
 await page.click('#pilot-fullscreen');await page.click('#pilot-zoom-100');if(await page.$eval('#pilot-tools',e=>e.open))await page.click('#pilot-tools-toggle');await settle();await page.$eval('#pilot-canvas',e=>{e.scrollLeft=0;e.scrollTop=0});
 const handle=await(await page.$('.graph-drag[data-task="a"]')).boundingBox(),point={x:handle.x+handle.width/2,y:handle.y+handle.height/2};
 await page.mouse.move(point.x,point.y);await page.mouse.down();await page.mouse.move(point.x+45,point.y+30,{steps:4});
 test('pending-group-excluded-from-snapshot',same(await placement(),before));await page.keyboard.press('Escape');await page.mouse.up();test('cancel-rolls-back-entire-group',same(await placement(),before));
 await page.mouse.move(point.x,point.y);await page.mouse.down();await page.mouse.move(point.x+45,point.y+30,{steps:4});await page.mouse.up();const pointer=await placement();test('pointer-group-common-translation',Math.abs(pointer.a.x-before.a.x-45)<1&&Math.abs(pointer.b.x-before.b.x-45)<1&&Math.abs(pointer.a.y-before.a.y-30)<1&&Math.abs(pointer.b.y-before.b.y-30)<1&&same(pointer.c,before.c));
 await tools();await page.click('#pilot-layout-undo');test('pointer-group-undo',same(await placement(),before));
 // Huge negative keyboard movement is simulated by repeated real arrows until
 // the finite original positions hit the existing geometric origin clamp.
 await page.$eval('.graph-drag[data-task="a"]',e=>e.focus());for(let step=0;step<4;step++)await page.keyboard.press('ArrowLeft');const clamped=await placement();test('origin-clamp-retains-relative-offset',Math.abs((clamped.a.x-clamped.b.x)-(before.a.x-before.b.x))<.01&&await page.$$eval('.graph-unit',nodes=>nodes.every(n=>{const r=n.querySelector('.graph-cadre'),m=n.transform.baseVal.consolidate().matrix;return Number(r.getAttribute('x'))+m.e>=0&&Number(r.getAttribute('y'))+m.f>=0})));
 // Filtering drops group membership; showing all cannot unexpectedly move it.
 await openPanel();await select('#pilot-branch','b');await click('#pilot-branch-apply');test('filter-clears-group',await page.$$eval('.graph-multi-selected',nodes=>nodes.length===0));await click('#pilot-analysis-clear');
 await click('#pilot-selection-mode');await page.$eval('.graph-task-open[data-task="a"]',e=>e.focus());await page.keyboard.press('Enter');await page.keyboard.press('Escape');test('escape-leaves-selection-before-fullscreen',await page.$eval('.native-graph-dialog',e=>e.open)&&await page.$$eval('.graph-multi-selected',nodes=>nodes.length===0)&&await page.$eval('#pilot-selection-mode',e=>e.getAttribute('aria-pressed')==='false'));
 for(const width of [390,320]){await page.setViewport({width,height:844});await settle();await openPanel();await select('#pilot-branch','b');await click('#pilot-branch-apply');test('mobile-filter-'+width,same(await ids(),['a','b'])&&await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));await page.screenshot({path:path.join(cfg.output,label+'-analyst-'+width+'.png')});await click('#pilot-analysis-clear');await click('#pilot-analysis-close');}
 await page.setViewport({width:1440,height:1000});await page.click('#pilot-fullscreen');await tools();await page.click('#pilot-layout-reset');
 await openPanel();await page.focus('#pilot-analysis-close');await page.keyboard.press('Escape');test('disclosure-escape-focus',await page.$eval('#pilot-analysis',e=>!e.open&&document.activeElement.id==='pilot-analysis-toggle'));
 await select('#pilot-branch','b');await click('#pilot-branch-apply');await click('#pilot-selection-mode');await page.$eval('.graph-task-open[data-task="b"]',e=>e.focus());await page.keyboard.press('Enter');
 await page.evaluate(async work=>SwarmRustDependencyGraph.render(await(await fetch('/api/v1/planning?'+new URLSearchParams({work}))).json()),cfg.work);
 test('rerender-clears-filter-and-selection',same(await ids(),fullIds)&&await page.$$eval('.graph-multi-selected',nodes=>nodes.length===0)&&await page.$eval('#pilot-selection-mode',e=>e.getAttribute('aria-pressed')==='false')&&await page.$eval('#pilot-analysis-clear',e=>e.disabled));
 // Capture exact state assertions for the parent recipe; no fixture credentials.
 fs.writeFileSync(path.join(cfg.output,label+'-analyst.json'),JSON.stringify({projection_fixture:'local-only',work:cfg.work,placement:await placement()},null,2)+'\n');
};
