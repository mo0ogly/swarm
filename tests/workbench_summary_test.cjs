'use strict';
/** Run the real workbench module with a small DOM double and check the Conduite
 * summary cards: counts come only from recorded states, no model shows "—",
 * "Voir" selects locally and switches to lanes, mission change resets cards,
 * state chips filter lanes and dim graph cards without any write.
 * No browser, network, provider, storage write of Work data or mission operation. */
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element{
 constructor(tag){this.tagName=tag.toUpperCase();this.children=[];this.dataset={};this.className='';this._text='';this.attributes={};this.listeners={};this.parent=null;this.hidden=false}
 set textContent(v){this._text=String(v);this.children=[]}get textContent(){return this._text+this.children.map(c=>c.textContent).join('')}
 append(...els){for(const e of els){if(typeof e==='string'){const n=new Element('#text');n._text=e;this.children.push(n);continue}e.parent=this;this.children.push(e)}}
 appendChild(e){this.append(e);return e}
 replaceChildren(...els){this.children=[];this._text='';this.append(...els)}
 before(e){const s=this.parent.children;e.parent=this.parent;s.splice(s.indexOf(this),0,e)}
 after(e){const s=this.parent.children;e.parent=this.parent;s.splice(s.indexOf(this)+1,0,e)}
 setAttribute(k,v){this.attributes[k]=String(v)}getAttribute(k){return this.attributes[k]}
 addEventListener(k,f){(this.listeners[k]??=[]).push(f)}
 click(){for(const f of this.listeners.click||[])f()}
 focus(){focused=this}
 get classList(){const set=()=>new Set(this.className.split(/\s+/).filter(Boolean)),write=v=>{this.className=[...v].join(' ')};return{add:c=>{const v=set();v.add(c);write(v)},toggle:(c,on)=>{const v=set();if(on===undefined?!v.has(c):on)v.add(c);else v.delete(c);write(v)}}}
 matches(sel){if(sel.startsWith('.'))return this.className.split(/\s+/).includes(sel.slice(1));if(sel.startsWith('#'))return this.id===sel.slice(1);return this.tagName===sel.toUpperCase()}
 *walk(){for(const c of this.children){yield c;yield*c.walk()}}
 querySelector(sel){for(const n of this.walk())if(n.matches(sel))return n;return null}
 querySelectorAll(sel){return[...this.walk()].filter(n=>n.matches(sel))}
}
let focused=null;
const conduite=new Element('section'),host=new Element('div'),heading=new Element('div'),graph=new Element('div');
conduite.id='conduite';host.className='conduite-principal';heading.className='graph-heading';graph.id='graph';
conduite.append(host);host.append(heading,graph);
const body=new Element('body');body.append(conduite);
const document={body,createElement:tag=>new Element(tag),getElementById:id=>[...body.walk()].find(n=>n.id===id)||null,
 querySelector:sel=>sel==='#conduite .conduite-principal'?host:body.querySelector(sel),querySelectorAll:sel=>sel==='#graph .graph-noeud[data-task]'?graph.querySelectorAll('.graph-noeud').filter(n=>n.dataset.task):body.querySelectorAll(sel)};
class Option extends Element{constructor(text,value){super('option');this.textContent=text;this.value=value}}
const writes=[];const localStorage={getItem:()=>null,setItem:(k,v)=>writes.push([k,v])};
const ctx=vm.createContext({document,Option,localStorage,Error,Map,Set,String,Object,Array,Math});
vm.runInContext(fs.readFileSync(__dirname+'/../web/swarm-workbench.js','utf8'),ctx);
const W=ctx.SwarmWorkbench;
let count=0;function check(name,fn){fn();count++;console.log('PASS '+name)}
const cards=()=>host.querySelector('.workbench-summary').children;
const card=tone=>cards().find(c=>c.dataset.summary===tone);
check('no-model-shows-dashes-not-zero',()=>{W.mount();assert.equal(host.children[0].className,'workbench-summary');assert.equal(cards().length,3);for(const tone of ['warn','alert'])assert.equal(card(tone).querySelector('.workbench-count').textContent,'—');assert.match(card('progress').textContent,/—/);assert.equal(card('warn').querySelector('button'),null)});
const work={id:'w-1',title:'W',tasks:[
 {id:'a',title:'Accepted one',status:'accepted'},{id:'b',title:'Blocked one',status:'blocked'},
 {id:'c',title:'Submitted one',status:'submitted'},{id:'d',title:'Todo one',status:'todo'},{id:'e',title:'Failed one',status:'failed'}]};
check('counts-come-from-recorded-states',()=>{assert.equal(W.render({work}),true);assert.equal(card('warn').querySelector('.workbench-count').textContent,'1');assert.equal(card('alert').querySelector('.workbench-count').textContent,'2');assert.match(card('alert').textContent,/Blocked one/);assert.match(card('progress').textContent,/1 \/ 5/);const bar=card('progress').querySelector('progress');assert.equal(bar.value,1);assert.equal(bar.max,5)});
check('projection-state-overrides-work-status',()=>{W.render({work,mission:{tasks:[{id:'d',state:'blocked'},{id:'a',state:'running'}]}});assert.equal(card('alert').querySelector('.workbench-count').textContent,'3');assert.match(card('progress').textContent,/0 \/ 5/)});
check('see-selects-locally-and-opens-lanes',()=>{W.render({work});writes.length=0;card('alert').querySelector('button').click();assert.equal(document.getElementById('graph').hidden,true);assert.equal(document.getElementById('workbench-lanes').hidden,false);assert.equal(focused?.textContent,'Blocked one');assert.deepEqual(writes,[['swarm-conduite:view:v1','lanes']])});
check('empty-state-has-no-see-button',()=>{W.render({work:{id:'w-2',title:'W2',tasks:[{id:'x',title:'Done',status:'accepted'}]}});assert.equal(card('warn').querySelector('.workbench-count').textContent,'0');assert.equal(card('warn').querySelector('button'),null);assert.match(card('warn').textContent,/Aucune tâche dans cet état\./)});
check('clear-resets-cards',()=>{W.clear('Lecture…');assert.equal(card('alert').querySelector('.workbench-count').textContent,'—')});
check('invalid-work-is-refused-and-cards-reset',()=>{W.render({work});assert.equal(W.render({work:{id:'w-3',tasks:[{id:'a'}]}}),false);assert.equal(card('warn').querySelector('.workbench-count').textContent,'—')});
for(const id of ['a','b','c','d','e']){const n=new Element('g');n.className='graph-noeud';n.dataset.task=id;graph.append(n)}
const chip=key=>host.querySelector('.workbench-filters').children.find(c=>c.dataset.filter===key);
const laneIds=()=>host.querySelectorAll('.workbench-task').map(b=>b.dataset.laneTask).sort().join(',');
const dimmed=()=>graph.querySelectorAll('.graph-noeud').filter(n=>n.className.includes('graph-state-dim')).map(n=>n.dataset.task).sort().join(',');
check('filter-chips-count-recorded-states',()=>{W.render({work});assert.equal(chip('').getAttribute('aria-pressed'),'true');assert.match(chip('').textContent,/5$/);assert.match(chip('act').textContent,/3$/);assert.match(chip('info').textContent,/0$/);assert.match(chip('success').textContent,/1$/);assert.equal(dimmed(),'')});
check('chip-filters-lanes-and-dims-graph-only',()=>{writes.length=0;chip('act').click();assert.equal(chip('act').getAttribute('aria-pressed'),'true');assert.equal(laneIds(),'b,c,e');assert.equal(dimmed(),'a,d');assert.deepEqual(writes,[]);chip('').click();assert.equal(laneIds(),'a,b,c,d,e');assert.equal(dimmed(),'')});
check('mission-change-resets-filter',()=>{W.render({work});chip('success').click();assert.equal(dimmed(),'b,c,d,e');W.render({work:{id:'w-9',title:'W9',tasks:[{id:'a',title:'A',status:'blocked'}]}});assert.equal(chip('').getAttribute('aria-pressed'),'true');assert.equal(dimmed(),'')});
check('clear-resets-filter-and-shows-dashes',()=>{W.render({work});chip('act').click();W.clear('Lecture…');assert.equal(chip('').getAttribute('aria-pressed'),'true');assert.match(chip('act').textContent,/—$/);assert.equal(dimmed(),'')});
console.log(`${count}/11 workbench summary checks passed`);
