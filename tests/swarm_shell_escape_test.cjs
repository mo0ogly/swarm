'use strict';
/** Execute the actual shell key handler with a minimal DOM double. These named
 * checks establish event ownership, not browser focus, rendering or delivery.
 * No network, timers, product state, provider or browser is used. */
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(__dirname+'/../web/swarm-shell.js','utf8');
/** Fresh presentation fixture: return shell state and the registered key handler.
 * modal and prevented model an active modal and an already consumed event. */
function fixture({modal=false,prevented=false,open=true}={}){
 const listeners={},toggle={attributes:{},setAttribute(k,v){this.attributes[k]=v},addEventListener(){},focus(){document.activeElement=this}};
 const document={activeElement:null,body:{dataset:{navOpen:String(open)}},getElementById:id=>id==='nav-toggle'?toggle:null,querySelector:s=>s==='dialog:modal'&&modal?{}:null,querySelectorAll:()=>[],addEventListener:(k,f)=>{listeners[k]=f}};
 vm.runInNewContext(source,{document,localStorage:{getItem:()=>null},MutationObserver:class{observe(){}}});
 return {document,toggle,press:key=>listeners.keydown({key,defaultPrevented:prevented})};
}
const checks=[
 ['navigation-alone-escape-closes-and-focuses-toggle',()=>{const f=fixture();f.press('Escape');assert.equal(f.document.body.dataset.navOpen,'false');assert.equal(f.document.activeElement,f.toggle);assert.equal(f.toggle.attributes['aria-expanded'],'false')}],
 ['modal-escape-preserves-navigation-and-focus-ownership',()=>{const f=fixture({modal:true});f.press('Escape');assert.equal(f.document.body.dataset.navOpen,'true');assert.equal(f.document.activeElement,null)}],
 ['consumed-escape-preserves-navigation',()=>{const f=fixture({prevented:true});f.press('Escape');assert.equal(f.document.body.dataset.navOpen,'true');assert.equal(f.document.activeElement,null)}],
 ['other-key-preserves-navigation',()=>{const f=fixture();f.press('Enter');assert.equal(f.document.body.dataset.navOpen,'true')}],
 ['closed-navigation-escape-has-no-focus-effect',()=>{const f=fixture({open:false});f.press('Escape');assert.equal(f.document.activeElement,null)}]
];
let failed=0;for(const [name,check] of checks){try{check();console.log('PASS '+name)}catch(error){failed++;console.error('FAIL '+name+'\n'+error.stack)}}
console.log(`${checks.length-failed}/${checks.length} shell Escape checks passed`);if(failed)process.exitCode=1;
