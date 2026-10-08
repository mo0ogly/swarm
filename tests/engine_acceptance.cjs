#!/usr/bin/env node
'use strict';
// Every case requires newly executed behavioral tests for that engine lot.
// Report existence, cached output and an empty -run filter cannot grant success.
const {spawnSync}=require('node:child_process');
const fs=require('node:fs');
const path=require('node:path');
const names={launch:'Launch',ownership:'Ownership',revision:'Revision',recovery:'Recovery',truth:'Truth',real:'Real',history:'History',delivery:'Delivery'};
const args=process.argv.slice(2),selected=args[args.indexOf('--case')+1];
if(!args.includes('--case')||!names[selected]){console.error('usage: node tests/engine_acceptance.cjs --case '+Object.keys(names).join('|'));process.exit(2)}
const root=path.resolve(__dirname,'..');
if(!fs.existsSync(path.join(root,'go.mod'))){console.error('Go module missing');process.exit(3)}
const env={...process.env,TMPDIR:process.env.TMPDIR||'/dev/shm',GOTMPDIR:process.env.GOTMPDIR||'/dev/shm'};
if(selected==='truth')env.SWARM_ENGINE_TRUTH_BROWSER='1';
const prefix='TestEngineContract'+names[selected];
const result=spawnSync('go',['test','-json','-count=1','-run','^'+prefix,'.'],{cwd:root,env,encoding:'utf8',timeout:180000,maxBuffer:8*1024*1024});
process.stdout.write(result.stdout||'');process.stderr.write(result.stderr||'');
if(result.error){console.error(result.error.message);process.exit(4)}
if(result.status!==0)process.exit(result.status||1);
const records=(result.stdout||'').split('\n').map(l=>{try{return JSON.parse(l)}catch{return null}}).filter(Boolean);
const tests=records.filter(r=>r.Action==='run'&&r.Test?.startsWith(prefix));
if(!tests.length||!records.some(r=>r.Action==='pass'&&!r.Test)||!tests.every(t=>records.some(r=>r.Test===t.Test&&r.Action==='pass'))){console.error('No complete passing behavioral test run for '+prefix);process.exit(5)}
console.log(selected+': '+tests.length+' behavioral test runs passed; independent review remains required.');
