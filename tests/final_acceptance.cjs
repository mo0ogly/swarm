#!/usr/bin/env node
'use strict';
const fs=require('node:fs'),path=require('node:path');
const {spawnSync}=require('node:child_process');
const root=path.resolve(__dirname,'..');
// "proofs" covers R1 — Fiabiliser les états des preuves: only evidence_projection.go
// and evidence_contract_test.go. It must refuse to report success on zero executed
// Go tests (a JSON test log with no run/pass record is not evidence of anything).
// "git" covers R2 — Attribuer les preuves à la révision Git: the managed
// integration receipt must carry the Git commit actually merged and tested
// (managed_integration.go's commit-tree result), never a value equal to the
// business revision counter, and it must stay "unknown" for a legacy receipt
// recorded before this field existed. Required tests span the unit-level
// evidence projection and the end-to-end managed integration path.
const cases={
 proofs:{dependencies:['go.mod','evidence_projection.go','evidence_contract_test.go'],pattern:'^TestEvidenceContract',
  required:['TestEvidenceContractControlAggregationIsOrderIndependent','TestEvidenceContractReviewerConfiguredWithoutVerdictIsNotNotConfigured'],
  message:'proofs acceptance: order-independent failed-priority aggregation, empty-list unknown, null exit_code on unknown history, and configured-reviewer-pending states verified\n'},
 git:{dependencies:['go.mod','model.go','managed_integration.go','evidence_projection.go','evidence_contract_test.go','managed_git_test.go'],
  pattern:'^(TestEvidenceContract.*|TestManagedIntegrationRecordsRealCandidateSHA|TestManagedIntegrationAtomicAndConflict|TestManagedFailedControlNeverPublishes)$',
  required:['TestEvidenceContractCandidateSHADistinctFromRevisionAndUnknownForLegacy','TestManagedIntegrationRecordsRealCandidateSHA'],
  message:'git acceptance: managed integration receipt carries the real tested candidate commit, distinct from the business revision and timestamps, unknown for legacy receipts, verified end to end\n'},
};
function runAcceptance(selected,options={}){
 const spec=cases[selected],projectRoot=options.root||root,exists=options.exists||fs.existsSync,out=options.stdout||process.stdout,err=options.stderr||process.stderr,spawn=options.spawnSync||spawnSync;
 if(!spec){err.write(`unknown final acceptance case: ${selected}\n`);return 2}
 for(const dependency of spec.dependencies)if(!exists(path.join(projectRoot,dependency))){err.write(`${selected} acceptance dependency missing: ${dependency}\n`);return 3}
 const env={...process.env,...options.env};if(!env.TMPDIR)env.TMPDIR='/dev/shm';if(!env.GOTMPDIR)env.GOTMPDIR='/dev/shm';if(!env.GOCACHE)env.GOCACHE='/dev/shm/swarm-final-go-cache';fs.mkdirSync(env.GOCACHE,{recursive:true});
 const go=spawn(options.goCommand||'go',['test','-json','-count=1','-run',spec.pattern,'.'],{cwd:projectRoot,env,encoding:'utf8',timeout:300000});out.write(go.stdout||'');err.write(go.stderr||'');
 if(go.error&&go.status===null){err.write(`${selected} acceptance could not execute Go: ${go.error.message}\n`);return 4}
 if(go.status!==0)return go.status??1;
 const records=(go.stdout||'').split('\n').filter(Boolean).map(line=>{try{return JSON.parse(line)}catch{return null}}).filter(Boolean);
 if(!records.some(record=>record.Action==='run'&&record.Test)||!records.some(record=>record.Action==='pass'&&!record.Test)){err.write(`${selected} acceptance produced no executed Go test success record\n`);return 5}
 const passedTests=records.filter(record=>record.Action==='pass'&&record.Test).map(record=>record.Test);
 for(const required of spec.required)
  if(!passedTests.includes(required)){err.write(`${selected} acceptance missing required passing test: ${required}\n`);return 6}
 out.write(spec.message);
 return 0;
}
function main(argv=process.argv.slice(2)){const index=argv.indexOf('--case');if(index<0||!argv[index+1]){console.error('usage: node tests/final_acceptance.cjs --case proofs|git');return 2}return runAcceptance(argv[index+1])}
module.exports={runAcceptance,main};if(require.main===module)process.exitCode=main();
