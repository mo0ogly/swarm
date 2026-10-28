'use strict';
/** Protocol-only UI correlation guards. Minimal DTO fixtures are not complete
 * Root ledgers or evidence of authority/durability. No DOM/server/provider used.
 */
const fs=require('node:fs'),vm=require('node:vm'),assert=require('node:assert/strict');
const context={};vm.runInNewContext(fs.readFileSync('frontend/engine-settings/admin.js','utf8'),context);const verify=context.SwarmEngineSettingsAdmin.verifyApplyReply;
/** Distinct wire values and exact revisions above JS Number precision. */
function fixture(){const pending={schema_version:20,event_id:'ack-fixture',expected_revision:'9007199254740993',reason:'Protocol fixture, no execution',preview_token:'fixture-preview',values:{count:'3',programs:['a','b'],nullable:null}};return{pending,reply:{replayed:false,document:{schema_version:20,revision:'9007199254740994',values:{nullable:null,programs:['a','b'],count:'3'},history:[{revision:'9007199254740994',change:JSON.parse(JSON.stringify(pending))}]}}}}
let pair=fixture(),before=JSON.stringify(pair);assert.strictEqual(verify(pair.reply,pair.pending),pair.reply);assert.equal(JSON.stringify(pair),before);console.log('PASS apply ACK canonical decimal, key order and no mutation');
pair=fixture();pair.reply.replayed=true;pair.reply.document.revision='9007199254740996';pair.reply.document.values.count='7';assert.strictEqual(verify(pair.reply,pair.pending),pair.reply);console.log('PASS apply ACK historical replay retains original event with newer document');
const cases={
 'empty reply':p=>p.reply={},'wrong document key':p=>{p.reply.saved=p.reply.document;delete p.reply.document},'missing replay flag':p=>delete p.reply.replayed,
 'unrelated event':p=>p.reply.document.history[0].change.event_id='other','duplicate event':p=>p.reply.document.history.push(p.reply.document.history[0]),
 'changed preview':p=>p.reply.document.history[0].change.preview_token='changed','changed reason':p=>p.reply.document.history[0].change.reason='changed',
 'changed value':p=>p.reply.document.history[0].change.values.count='4','missing value':p=>delete p.reply.document.history[0].change.values.nullable,
 'changed list order':p=>p.reply.document.history[0].change.values.programs.reverse(),'extra value':p=>p.reply.document.history[0].change.values.extra='x',
 'numeric revision':p=>p.reply.document.revision=9007199254740994,'noncanonical revision':p=>p.reply.document.revision='09007199254740994',
 'wrong entry revision':p=>p.reply.document.history[0].revision='9007199254740995','current revision predates event':p=>p.reply.document.revision='9007199254740993',
 'fresh current values differ':p=>p.reply.document.values.count='8','missing pending token':p=>{delete p.pending.preview_token;delete p.reply.document.history[0].change.preview_token},
 'invalid schema':p=>{p.pending.schema_version=0;p.reply.document.history[0].change.schema_version=0},'nested values':p=>{p.pending.values.count={nested:true};p.reply.document.history[0].change.values.count=p.pending.values.count;p.reply.document.values.count=p.pending.values.count}
};
for(const [name,change]of Object.entries(cases)){const p=fixture();change(p);const original=JSON.stringify(p);assert.throws(()=>verify(p.reply,p.pending),e=>e.code==='settings-apply-ack-invalid',name);assert.equal(JSON.stringify(p),original,name+' mutated input');console.log('PASS apply ACK refuses '+name)}

console.log('PASS apply ACK '+(Object.keys(cases).length+2)+' cases');
