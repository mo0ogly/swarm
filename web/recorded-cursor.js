 'use strict';
/** Exact recorded SQL cursors through JSON numbers. These i64 representation
 * guards introduce no operator row ceiling, retention or execution authority. */
(()=>{
 const wireIntegers=new WeakMap();
 const refused=()=>Object.assign(new Error('recorded-cursor-invalid'),{code:'recorded-cursor-invalid'});
 /** Return canonical decimal i64. Native exact text is authoritative only when
  * consistent with the numeric wire value; safe legacy numbers remain usable.
  * Unsafe legacy numbers, malformed text/range/sign or contradictory values
  * refuse visibly instead of rounding a cursor and skipping/repeating history. */
 function integer(value,exact,signed){
  if(typeof signed!=='boolean'||typeof value!=='number'||!Number.isInteger(value))throw refused();
  if(exact===undefined){if(!Number.isSafeInteger(value))throw refused();exact=String(value)}
  if(typeof exact!=='string'||!/^(0|-[1-9][0-9]*|[1-9][0-9]*)$/.test(exact))throw refused();
  const n=BigInt(exact),top=(1n<<63n)-1n,bottom=signed?-(1n<<63n):0n;
  if(n<bottom||n>top||Number(exact)!==value)throw refused();
  return exact;
 }
 /** Native live pages accept nonnegative cursors. Root's numeric field remains
  * compatible; the decimal extension avoids JS precision loss without a cap. */
 function next(page){if(!page||typeof page!=='object')throw refused();return integer(page.next,page.next_exact,false)}
 /** Parse unchanged JSON values while retaining unsafe integer source text when
  * the browser exposes standard reviver context.source. Weak metadata never
  * mutates application objects or grants authority. Unsupported runtimes retain
  * legacy safe-number behavior; field() refuses unsafe unproven numbers. */
 function parse(raw){
  if(typeof raw!=='string')throw refused();
  return JSON.parse(raw,function(key,value,context){
   if(typeof value==='number'&&Number.isInteger(value)&&!Number.isSafeInteger(value)&&typeof context?.source==='string'&&/^-?(0|[1-9][0-9]*)$/.test(context.source)){
    let fields=wireIntegers.get(this);if(!fields){fields=new Map();wireIntegers.set(this,fields)}fields.set(key,context.source);
   }
   return value;
  });
 }
 /** Recover one actual parsed numeric field as canonical i64 text. Copies lose
  * weak metadata and unsafe copies must be read again, never rounded or inferred
  * from another snapshot. signed remains an explicit representation choice. */
 function field(object,key,signed){
  if(!object||typeof object!=='object'||typeof key!=='string')throw refused();
  return integer(object[key],wireIntegers.get(object)?.get(key),signed);
 }
 globalThis.SwarmRecordedCursor=Object.freeze({integer,next,parse,field});
})();
