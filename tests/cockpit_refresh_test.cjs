const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../web/cockpit.js'),'utf8');
const fn=source.slice(source.indexOf('function startCockpitRefresh()'),source.indexOf('\nfunction showView'));
(async()=>{
 let reads=0,logs=0,seq=0;const timers=new Map(),listeners=new Map();
 const document={hidden:false,addEventListener:(k,v)=>listeners.set(k,v),removeEventListener:(k,v)=>{assert.equal(listeners.get(k),v);listeners.delete(k)}};
 const ctx=vm.createContext({document,follow:true,view:'logs',refresh:async()=>{reads++},loadLogs:async()=>{logs++},setTimeout:f=>{timers.set(++seq,f);return seq},clearTimeout:id=>timers.delete(id)});
 vm.runInContext(fn+';this.live=startCockpitRefresh()',ctx);
 async function step(){const [id,cb]=timers.entries().next().value;timers.delete(id);await cb()}
 await step();assert.equal(reads,1);assert.equal(logs,1);assert.equal(timers.size,1);
 document.hidden=true;await step();assert.equal(reads,1);
 document.hidden=false;listeners.get('visibilitychange')();await new Promise(r=>setImmediate(r));assert.equal(reads,2);assert.equal(timers.size,1);
 ctx.live.close();assert.equal(timers.size,0);assert.equal(listeners.size,0);
 console.log('PASS: visible refresh, hidden-tab suspension, immediate return, timer and listener cleanup');
})().catch(e=>{console.error(e);process.exit(1)});
