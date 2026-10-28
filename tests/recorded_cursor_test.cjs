 'use strict';
/** Pure consumer regression: actual JSON numeric parsing, no fabricated backend
 * execution. Numeric fields retain Go wire semantics; native exact text carries
 * the representation that JavaScript would otherwise round. */
const assert=require('node:assert/strict');require('../web/recorded-cursor.js');const C=globalThis.SwarmRecordedCursor;let checks=0;
const check=(name,fn)=>{fn();checks++;console.log('PASS '+name)};
check('safe native cursor',()=>assert.equal(C.next({next:11,next_exact:'11'}),'11'));
check('actual JSON above53 preserves native exact value',()=>{const page=JSON.parse('{"next":9007199254740993,"next_exact":"9007199254740993"}');assert.notEqual(String(page.next),'9007199254740993');assert.equal(C.next(page),'9007199254740993')});
check('unsafe legacy cursor refuses instead of rounding',()=>assert.throws(()=>C.next(JSON.parse('{"next":9007199254740993}'))));
check('safe legacy cursor remains compatible',()=>assert.equal(C.next({next:9007199254740991}),'9007199254740991'));
check('malformed exact spellings refuse',()=>{for(const text of ['','01','-0','+1','1.0','1e1',' 1'])assert.throws(()=>C.integer(Number(text),text,true))});
check('contradictory exact and numeric values refuse',()=>assert.throws(()=>C.next({next:11,next_exact:'12'})));
check('signed i64 endpoints accepted exactly',()=>{for(const text of ['-9223372036854775808','9223372036854775807'])assert.equal(C.integer(Number(text),text,true),text)});
check('outside storage range refuses',()=>{for(const text of ['-9223372036854775809','9223372036854775808'])assert.throws(()=>C.integer(Number(text),text,true))});
check('signed query and nonnegative live are distinct protocols',()=>{assert.equal(C.integer(-1,'-1',true),'-1');assert.throws(()=>C.next({next:-1,next_exact:'-1'}))});
check('wrong types and absent page refuse',()=>{for(const n of [undefined,null,true,'1',NaN,Infinity,1.5])assert.throws(()=>C.integer(n,'1',true));assert.throws(()=>C.next(null));assert.throws(()=>C.integer(1,'1',undefined))});
check('actual numeric wire source preserves unsafe Work revision',()=>{const value=C.parse('{"revision":9007199254740993}');assert.equal(C.field(value,'revision',false),'9007199254740993');assert.equal(JSON.stringify(value),'{"revision":9007199254740992}')});
check('actual nested endpoints remain exact without extra properties',()=>{const value=C.parse('{"a":[{"revision":9223372036854775807}],"b":{"revision":-9223372036854775808}}');assert.equal(C.field(value.a[0],'revision',false),'9223372036854775807');assert.equal(C.field(value.b,'revision',true),'-9223372036854775808');assert.deepEqual(Object.keys(value.a[0]),['revision'])});
check('metadata cannot be transferred to an unsafe copied snapshot',()=>{const value=C.parse('{"revision":9007199254740993}');assert.throws(()=>C.field({...value},'revision',false));assert.throws(()=>C.field({revision:value.revision},'revision',false))});
check('unsupported reviver context never invents precision',()=>{const original=JSON.parse;let unsafe,safe;try{JSON.parse=(text,reviver)=>original(text,function(key,value){return reviver.call(this,key,value)});unsafe=C.parse('{"revision":9007199254740993}');safe=C.parse('{"revision":12}')}finally{JSON.parse=original}assert.throws(()=>C.field(unsafe,'revision',false));assert.equal(C.field(safe,'revision',false),'12')});
check('parse is value compatible and unsafe out-of-i64 fields refuse',()=>{const raw='{"a":[1,true,null,"x"],"revision":9223372036854775808}';assert.deepEqual(C.parse(raw),JSON.parse(raw));assert.throws(()=>C.field(C.parse(raw),'revision',false))});
check('bad parse and missing or nonnumeric field refuse',()=>{assert.throws(()=>C.parse({}));assert.throws(()=>C.parse('{'));for(const value of [null,{}, {revision:'12'},{revision:true}])assert.throws(()=>C.field(value,'revision',false))});
assert.equal(checks,16);console.log('RECORDED_CURSOR_VERIFIED '+checks);
