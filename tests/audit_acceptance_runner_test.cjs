// Historical Go-runner unit fixtures; no Go execution or Rust acceptance claim.
'use strict';
const assert=require('node:assert/strict'),path=require('node:path');
const {runAcceptance}=require('./audit_acceptance.cjs');
const sink=()=>({text:'',write(value){this.text+=value}}),okGo={status:0,stdout:'{"Action":"run","Test":"TestStateContract"}\n{"Action":"pass","Test":"TestStateContract"}\n{"Action":"pass","Package":"swarm.local/companion"}\n',stderr:''};
function execute(overrides={}){const stdout=sink(),stderr=sink();const code=runAcceptance('states',{root:path.resolve(__dirname,'..'),stdout,stderr,runDOM:()=>{},exists:()=>true,spawnSync:()=>okGo,...overrides});return {code,stdout:stdout.text,stderr:stderr.text}}
assert.equal(execute({spawnSync:()=>({status:null,stdout:'',stderr:'',error:Object.assign(Error('spawn go EPERM'),{code:'EPERM'})})}).code,4,'Go absent/refusé doit échouer');
assert.notEqual(execute({spawnSync:()=>({status:1,stdout:'FAIL\n',stderr:'contract failed'})}).code,0,'Go défaillant doit échouer');
assert.equal(execute({spawnSync:()=>({status:0,stdout:'{"Action":"pass","Package":"swarm.local/companion"}\n',stderr:''})}).code,5,'zéro test Go doit échouer');
assert.equal(execute({runDOM:()=>{throw Error('assertion DOM')}}).code,6,'assertion DOM défaillante doit échouer');
assert.equal(execute().code,0,'DOM et Go réussis doivent réussir cumulativement');
// Real Rust checkout must refuse historical Go dependencies before any launch.
let actualLaunch=false;
assert.equal(runAcceptance('states',{root:path.resolve(__dirname,'..'),stdout:sink(),stderr:sink(),spawnSync:()=>{actualLaunch=true;throw Error('unexpected Go launch')}}),3,'historical Go inputs are unavailable in the Rust checkout');
assert.equal(actualLaunch,false,'missing historical dependencies must never launch Go');
assert.equal(runAcceptance('inconnu',{stdout:sink(),stderr:sink()}),2,'cas inconnu doit échouer explicitement');
console.log('PASS: historical audit runner unit fixtures fail closed for missing dependencies, DOM, Go, zero tests and unknown cases; no Go executed');
