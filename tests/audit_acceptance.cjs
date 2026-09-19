#!/usr/bin/env node
'use strict';
const fs=require('node:fs'),path=require('node:path');
const {spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'..');
const cases={
 cursor:{dependencies:['go.mod','cursor_contract_test.go'],pattern:'^TestCursorContract'},
 states:{dependencies:['go.mod','states_contract_test.go','tests/states_contract_test.cjs','web/status-contract.js'],pattern:'^TestStateContract',dom:'tests/states_contract_test.cjs'},
 evidence:{dependencies:['go.mod','evidence_contract_test.go','tests/evidence_contract_test.cjs','web/evidence-contract.js','evidence_projection.go'],pattern:'^TestEvidenceContract',dom:'tests/evidence_contract_test.cjs'},
 journey:{dependencies:['go.mod','prephase_conversion_test.go','tests/journey_ui.cjs','web/prephase-conversion.js','web/prepare.html'],pattern:'^TestPreparationAuthorizePlanIsOneAtomicExplicitCommand$',journey:true},
};
function runAcceptance(selected,options={}){
 const spec=cases[selected],projectRoot=options.root||root,exists=options.exists||fs.existsSync,out=options.stdout||process.stdout,err=options.stderr||process.stderr,spawn=options.spawnSync||spawnSync;
 if(!spec){err.write(`unknown audit acceptance case: ${selected}\n`);return 2}
 for(const dependency of spec.dependencies)if(!exists(path.join(projectRoot,dependency))){err.write(`${selected} acceptance dependency missing: ${dependency}\n`);return 3}
 const env={...process.env,...options.env};if(!env.TMPDIR)env.TMPDIR='/dev/shm';if(!env.GOTMPDIR)env.GOTMPDIR='/dev/shm';if(!env.GOCACHE)env.GOCACHE='/dev/shm/swarm-audit-go-cache';fs.mkdirSync(env.GOCACHE,{recursive:true});
 if(spec.dom)try{(options.runDOM||((file)=>require(file)))(path.join(projectRoot,spec.dom))}catch(e){err.write(`${selected} acceptance DOM assertions failed: ${e?.stack||e}\n`);return 6}
 const go=spawn(options.goCommand||'go',['test','-json','-count=1','-run',spec.pattern,'.'],{cwd:projectRoot,env,encoding:'utf8',timeout:300000});out.write(go.stdout||'');err.write(go.stderr||'');
 if(go.error&&go.status===null){err.write(`${selected} acceptance could not execute Go: ${go.error.message}\n`);return 4}if(go.status!==0)return go.status??1;const records=(go.stdout||'').split('\n').filter(Boolean).map(line=>{try{return JSON.parse(line)}catch{return null}}).filter(Boolean);if(!records.some(record=>record.Action==='run'&&record.Test)||!records.some(record=>record.Action==='pass'&&!record.Test)){err.write(`${selected} acceptance produced no executed Go test success record\n`);return 5}
 if(spec.journey){
  const artifacts=path.join(projectRoot,'test-results');fs.mkdirSync(artifacts,{recursive:true});const binary=path.join(artifacts,'swarm-audit-journey-'+process.pid),report=path.join(artifacts,'audit-journey');
  const build=spawn(options.goCommand||'go',['build','-o',binary,'.'],{cwd:projectRoot,env,encoding:'utf8',timeout:300000});out.write(build.stdout||'');err.write(build.stderr||'');if(build.error&&build.status===null){err.write(`journey acceptance could not build: ${build.error.message}\n`);return 4}if(build.status!==0)return build.status??1;
  const ui=spawn(process.execPath,[path.join(projectRoot,'tests/journey_ui.cjs'),binary,report],{cwd:projectRoot,env,encoding:'utf8',timeout:300000});out.write(ui.stdout||'');err.write(ui.stderr||'');if(ui.error&&ui.status===null){err.write(`journey acceptance could not execute browser recipe: ${ui.error.message}\n`);return 4}if(ui.status!==0)return ui.status??1;if(!/journey acceptance: PASS/.test(ui.stdout||'')){err.write('journey acceptance produced no browser success record\n');return 5}
 }
 out.write(selected==='cursor'?'cursor acceptance: responsibilities and return flow verified\n':selected==='evidence'?'evidence acceptance: report review, executed controls, acceptance, freshness and CLI/web parity verified\n':selected==='states'?'states acceptance: cumulative DOM and Go/JSON contracts verified\n':'journey acceptance: need, proposed team, preflight and single authorization verified\n');return 0;
}
function main(argv=process.argv.slice(2)){const index=argv.indexOf('--case');if(index<0||!argv[index+1]){console.error('usage: node tests/audit_acceptance.cjs --case cursor|states|evidence|journey');return 2}return runAcceptance(argv[index+1])}
module.exports={runAcceptance,main};if(require.main===module)process.exitCode=main();
