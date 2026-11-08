'use strict';
/** Run actual workbench badge producer and native row consumer with a minimal
 * DOM double. Named checks cover status semantics, safe text and refresh; no
 * browser, provider, network or mission operation is performed. */
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element{
 constructor(tag){this.tagName=tag;this.children=[];this.dataset={};this.className='';this._text=''}
 set textContent(v){this._text=String(v);this.children=[]}get textContent(){return this._text+this.children.map(c=>c.textContent).join('')}
 appendChild(e){this.children.push(e);return e}append(...els){this.children.push(...els)}
}
const tbody=new Element('tbody');const document={createElement:tag=>new Element(tag),getElementById:id=>id==='tasks-body'?tbody:null,querySelector:()=>null};
const ctx=vm.createContext({document,SwarmRustDependencyGraph:{render(){},clear(){}}});
for(const name of ['swarm-workbench.js','cockpit-native-read.js'])vm.runInContext(fs.readFileSync(__dirname+'/../web/'+name,'utf8'),ctx);
let count=0;function check(name,fn){fn();count++;console.log('PASS '+name)}
check('native-table-uses-colored-recorded-status-badges',()=>{
 const states=['accepted','running','blocked','failed','submitted','stale','waived','todo','unknown'];
 ctx.SwarmRustCockpitRead.render({work:{id:'fixture',title:'Fixture',tasks:states.map((status,i)=>({id:'t'+i,title:'Task',status}))}});
 const tones=['success','info','alert','alert','warn','warn','warn','neutral','neutral'];
 for(let i=0;i<states.length;i++){const badge=tbody.children[i].children[1].children[0];assert(badge,'missing badge '+states[i]);assert.equal(badge.className,'workbench-badge '+tones[i]);assert.equal(badge.dataset.taskState,states[i]);assert.equal(tbody.children[i].children.length,5)}
});
check('unknown-state-is-plain-text-and-neutral',()=>{const value='<img src=x onerror=alert(1)>';const badge=ctx.SwarmWorkbench.statusBadge(value);assert.equal(badge.textContent,value);assert.equal(badge.className,'workbench-badge neutral');assert.equal(badge.children.length,0)});
check('label-does-not-grant-success-tone',()=>{const badge=ctx.SwarmWorkbench.statusBadge('unknown','accepted');assert.equal(badge.className,'workbench-badge neutral');assert.equal(badge.textContent,'Acceptée')});
check('missing-state-remains-explicit',()=>{const badge=ctx.SwarmWorkbench.statusBadge(undefined);assert.equal(badge.textContent,'État non disponible.');assert.equal(badge.className,'workbench-badge neutral')});
check('table-refresh-replaces-old-status-and-empty-state',()=>{ctx.SwarmRustCockpitRead.render({work:{id:'fixture',title:'Fixture',tasks:[{id:'t0',title:'Task',status:'blocked'}]}});assert.equal(tbody.children.length,1);assert.equal(tbody.children[0].children[1].children[0].className,'workbench-badge alert');ctx.SwarmRustCockpitRead.render({work:{id:'fixture',title:'Fixture',tasks:[]}});assert.equal(tbody.children.length,1);assert.equal(tbody.children[0].children[0].textContent,'Aucune tâche enregistrée.')});
console.log(`${count}/5 task status badge checks passed`);
