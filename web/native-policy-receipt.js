'use strict';
/** Native compatible-file policy receipts: verify request subject and recorded
 * history before a UI confirms saving. No IO, business acceptance or rights. */
(()=>{
 const invalid=()=>{throw Error('policy-receipt-invalid')},object=v=>v!==null&&typeof v==='object'&&!Array.isArray(v);
 /** Canonical signed64 decimal, accepting only lossless JS integer input. This
  * is wire representation, never an operator duration/count/cap policy. */
 function integer(v){if(typeof v==='number'){if(!Number.isSafeInteger(v))invalid();v=String(v)}if(typeof v!=='string'||! /^-?(0|[1-9][0-9]*)$/.test(v)||v==='-0')invalid();const n=BigInt(v);if(n<-(1n<<63n)||n>(1n<<63n)-1n)invalid();return v}
 /** Fixed policy values contain only integer leaves, objects, arrays or nil.
  * Sort object keys without changing array order, preserving exact decimals. */
 function values(v){if(v===null)return null;if(Array.isArray(v))return v.map(values);if(object(v))return Object.fromEntries(Object.keys(v).sort().map(k=>[k,values(v[k])]));return integer(v)}
 /** Original Go Change fields only; token is deliberately not recorded. This
  * validates a matching subject, never unrelated historical metadata. Go JSON
  * replaces lone UTF16 surrogates with U+FFFD; match that wire spelling without
  * trimming/translating valid text. This is representation, not reason policy. */
 function change(v){if(!object(v)||typeof v.event_id!=='string'||typeof v.reason!=='string')invalid();return {schema_version:integer(v.schema_version),event_id:v.event_id,expected_revision:integer(v.expected_revision),values:values(v.values),reason:v.reason.toWellFormed()}}
 const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
 /** Require the producer's exact config projection and genuine history array.
  * No missing-key fallback to empty history/zero or invented current defaults. */
 function config(frame){const c=frame?.exact;if(!object(c)||integer(c.schema_version)!=='1'||BigInt(integer(c.revision))<0n||!Array.isArray(c.history))invalid();return c}
 /** Correlate a positive native ACK with the previewed intent. A new commit
  * must append expected+1 with requested values; exact replay may return newer
  * current state. Actor/time remain unverified. Errors convey no rollback. */
 function verifyMutation(reply,request){
  if(!object(reply)||reply.applied!==true||reply.preview!==false||typeof reply.replayed!=='boolean'||typeof reply.after_digest!=='string'||!reply.after_digest||typeof reply.before_digest!=='string'||!reply.before_digest)invalid();
  if(!reply.replayed&&(!request?.preview_token||reply.preview_token!==request.preview_token))invalid();
  const expected=change(request),c=config(reply),row=c.history.find(row=>row?.change?.event_id===expected.event_id);if(!row||!same(change(row.change),expected))invalid();
  const rowRevision=integer(row.revision),configRevision=integer(c.revision),recordedValues=values(row.values);
  if(BigInt(rowRevision)<1n||BigInt(configRevision)<BigInt(rowRevision))invalid();
  if(!reply.replayed&&(BigInt(rowRevision)!==BigInt(expected.expected_revision)+1n||configRevision!==rowRevision||!same(recordedValues,expected.values)||!same(values(c.values),expected.values)))invalid();
  return {change:expected,rowRevision,configRevision,recordedValues,afterDigest:reply.after_digest,replayed:reply.replayed};
 }
 /** Join actual readback to the acknowledged record, not just HTTP200/flags.
  * Later append-only changes are allowed and reported; an older/no-op/missing
  * record or same-revision wrong digest is a contradiction, never green success. */
 function verifyReadback(view,proof){
  const c=config(view),row=c.history.find(row=>row?.change?.event_id===proof.change.event_id);
  if(!row||integer(row.revision)!==proof.rowRevision||!same(change(row.change),proof.change)||!same(values(row.values),proof.recordedValues)||BigInt(integer(c.revision))<BigInt(proof.configRevision))invalid();
  if(integer(c.revision)===proof.configRevision&&view.digest!==proof.afterDigest)invalid();
  return {newer:BigInt(c.revision)>BigInt(proof.configRevision),replayed:proof.replayed};
 }
 /** SQL scope entries omit empty mission/key by Go protocol. Missing mandatory
  * fields never become a zero entry. Actor/time are data, not authentication. */
 function scopeEntry(e){if(!object(e)||typeof e.scope!=='string'||typeof e.reason!=='string'||typeof e.actor!=='string'||typeof e.updated!=='string')invalid();for(const k of ['mission_id','scope_key'])if(Object.hasOwn(e,k)&&typeof e[k]!=='string')invalid();return {scope:e.scope,mission_id:e.mission_id??'',scope_key:e.scope_key??'',revision:integer(e.revision),values:values(e.values),reason:e.reason,actor:e.actor,updated:e.updated}}
 const sameScope=(a,b)=>a.scope===b.scope&&a.mission_id===b.mission_id&&a.scope_key===b.scope_key;
 /** SQL apply/rollback ACK must bind the actual preview token and resolved
  * entry (including rollback reason/values). No engine historical read gate is
  * changed; UI uncertainty requires reload instead of claiming completion. */
 function verifyScopeMutation(reply,request,preview){
  if(!object(reply)||reply.applied!==true||reply.preview!==false||typeof reply.replayed!=='boolean'||!request?.preview_token||reply.preview_token!==request.preview_token)invalid();
  const entry=scopeEntry(reply.exact?.entry),expected=scopeEntry(preview?.exact?.entry),scope={scope:request.scope,mission_id:request.mission_id??'',scope_key:request.scope_key??''};
  if(!sameScope(entry,scope)||!sameScope(expected,scope))invalid();
  if(!reply.replayed&&(BigInt(entry.revision)!==BigInt(integer(request.expected_revision))+1n||entry.revision!==expected.revision||!same(entry.values,values(preview.exact.proposed))||entry.reason!==expected.reason))invalid();
  return {entry,replayed:reply.replayed,rollbackOf:Object.hasOwn(request,'to_revision')?integer(request.to_revision):integer(request.rollback_of??0)};
 }
 /** Join SQL current/history to the ACK, allowing a newer current revision.
  * A200 with unchanged/empty/wrong scope cannot establish the requested change.
  * Current/history metadata remain unverified identity, merely compared data. */
 function verifyScopeReadback(view,proof){
  const current=scopeEntry(view?.exact?.entry),e=proof.entry;if(!sameScope(current,e)||BigInt(current.revision)<BigInt(e.revision)||!Array.isArray(view.exact.history))invalid();
  const row=view.exact.history.find(row=>integer(row.revision)===e.revision);
  if(!row||!sameScope({scope:row.scope,mission_id:row.mission_id??'',scope_key:row.scope_key??''},e)||!same(values(row.values),e.values)||row.reason!==e.reason||row.actor!==e.actor||row.at!==e.updated||integer(row.rollback_of??0)!==proof.rollbackOf)invalid();
  if(current.revision===e.revision&&(!same(current.values,e.values)||current.reason!==e.reason||current.actor!==e.actor||current.updated!==e.updated))invalid();
  return {newer:BigInt(current.revision)>BigInt(e.revision),replayed:proof.replayed};
 }

 globalThis.SwarmPolicyReceipt=Object.freeze({verifyMutation,verifyReadback,verifyScopeMutation,verifyScopeReadback});
})();
