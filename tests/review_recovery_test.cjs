'use strict';
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
class Element {
 constructor(tag,text){this.tag=tag;this.textContent=text||'';this.childNodes=[];this.dataset={};}
 append(...items){this.childNodes.push(...items)}
 setAttribute(){}
 querySelectorAll(tag){return this.childNodes.flatMap(x=>[...(x.tag===tag?[x]:[]),...x.querySelectorAll(tag)])}
}
function render(status,changed,revalidating=false,archived=false){
 const task={id:'t1',title:'Result',status,independent_review:{state:revalidating?'passed':'changes_requested',reason:'Missing proof'}};
 const snapshot={work:{tasks:[task]},independent_reviews:{t1:{current:false,evidence_changed:changed,revalidation_available:revalidating}}};
 if(archived){ snapshot.independent_reviews.t1.archived_review=task.independent_review; delete task.independent_review; }
 const context=vm.createContext({snapshot,node:(tag,text)=>new Element(tag,text),Pilot:{command:(label,action)=>{const button=new Element('button',label);button.action=action;return button}},SwarmStatusContract:{review:()=>({message:'Review recorded',availability:'available',state:'completed'})}});
 vm.runInContext(fs.readFileSync('web/planning.js','utf8')+'\nthis.subject=Planning;',context);
 const panel=new Element('section');
 context.subject.roles(panel,{scopes:[],provider:'fixture',decisions:1,reviewer:{provider:'fixture',calls:1,max_calls:4}});
 return panel.querySelectorAll('button').map(b=>b.textContent);
}
assert(!render('blocked',false).includes('Reprendre la vérification'),'unchanged rejected result must not offer retry');
assert(render('blocked',true).includes('Reprendre la vérification'),'changed bound evidence must expose recovery from blocked');
assert(render('submitted',true).includes('Reprendre la vérification'));
assert(!render('accepted',true).includes('Reprendre la vérification'),'accepted tasks require explicit reopening');
assert(render('accepted',false,true).includes('Reprendre la vérification'),'engine-authorized stale acceptance must offer revalidation');
assert(render('blocked',true,false,true).includes('Reprendre la vérification'),'archived refusal with changed evidence remains recoverable');
console.log('PASS review recovery action follows engine freshness and task state');
