'use strict';
const assert=require('node:assert/strict'),path=require('node:path');
const {runAcceptance}=require('./audit_acceptance.cjs');
const sink=()=>({text:'',write(value){this.text+=value}}),okGo={status:0,stdout:'{"Action":"run","Test":"TestStateContract"}\n{"Action":"pass","Test":"TestStateContract"}\n{"Action":"pass","Package":"swarm.local/companion"}\n',stderr:''};
function execute(overrides={}){const stdout=sink(),stderr=sink();const code=runAcceptance('states',{root:path.resolve(__dirname,'..'),stdout,stderr,runDOM:()=>{},spawnSync:()=>okGo,...overrides});return {code,stdout:stdout.text,stderr:stderr.text}}
assert.equal(execute({spawnSync:()=>({status:null,stdout:'',stderr:'',error:Object.assign(Error('spawn go EPERM'),{code:'EPERM'})})}).code,4,'Go absent/refusé doit échouer');
assert.notEqual(execute({spawnSync:()=>({status:1,stdout:'FAIL\n',stderr:'contract failed'})}).code,0,'Go défaillant doit échouer');
assert.equal(execute({spawnSync:()=>({status:0,stdout:'{"Action":"pass","Package":"swarm.local/companion"}\n',stderr:''})}).code,5,'zéro test Go doit échouer');
assert.equal(execute({runDOM:()=>{throw Error('assertion DOM')}}).code,6,'assertion DOM défaillante doit échouer');
assert.equal(execute().code,0,'DOM et Go réussis doivent réussir cumulativement');
assert.equal(runAcceptance('inconnu',{stdout:sink(),stderr:sink()}),2,'cas inconnu doit échouer explicitement');
console.log('PASS: audit acceptance runner fails closed for DOM, Go, zero tests and unknown cases');
