'use strict';
/** Exercise source reuse and refusal on isolated npm fixtures, without Chrome.
 * Every fault follows a healthy control. Failed fixture evidence is preserved. */
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict'),crypto=require('node:crypto'),{spawnSync}=require('node:child_process');
const runtime=require('./helpers/browser_verification_runtime.cjs');
const hash=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-npm-baseline-')),repo=path.join(root,'repo'),modules=path.join(repo,'node_modules');
fs.chmodSync(root,0o700);fs.mkdirSync(modules,{recursive:true});fs.symlinkSync(path.resolve('tools'),path.join(repo,'tools'));
const moduleFile=path.join(modules,'module.cjs'),lock=path.join(repo,'package-lock.json');
fs.writeFileSync(moduleFile,'original source');fs.writeFileSync(lock,'{}');fs.symlinkSync('module.cjs',path.join(modules,'alias'));
fs.linkSync(moduleFile,path.join(modules,'hardlink.cjs'));
const previous=process.env.SWARM_VERIFICATION_NPM_BASELINE;delete process.env.SWARM_VERIFICATION_NPM_BASELINE;
let serial=0,complete=false;
const output=()=>{const dir=path.join(root,'run-'+(++serial));fs.mkdirSync(dir);return dir};
try{
 const initial=output(),record=runtime.begin(repo,initial),archive=path.join(initial,'npm-sources.tar.gz'),manifest=path.join(initial,'runtime-inputs.json');
 const good={...record,status:'PASS'};fs.writeFileSync(manifest,JSON.stringify(good));process.env.SWARM_VERIFICATION_NPM_BASELINE=manifest;
 const control=()=>{const dir=output(),observed=runtime.begin(repo,dir);assert.equal(observed.npm_archive_sha256,good.npm_archive_sha256);assert(!fs.existsSync(path.join(dir,'npm-sources.tar.gz')));assert.equal(observed.npm_source_reference.archive,archive)};
 const fault=(change,restore)=>{control();change();try{assert.throws(()=>runtime.begin(repo,output()))}finally{restore()}control()};
 control();
 fault(()=>fs.writeFileSync(moduleFile,'changed'),()=>fs.writeFileSync(moduleFile,'original source'));
 fault(()=>fs.writeFileSync(lock,'{"changed":true}'),()=>fs.writeFileSync(lock,'{}'));
 fault(()=>fs.writeFileSync(manifest,JSON.stringify({...good,status:'FAIL'})),()=>fs.writeFileSync(manifest,JSON.stringify(good)));
 fault(()=>fs.renameSync(archive,archive+'.held'),()=>fs.renameSync(archive+'.held',archive));
 const bytes=fs.readFileSync(archive);
 fault(()=>fs.writeFileSync(archive,'corrupt'),()=>fs.writeFileSync(archive,bytes));
 // Valid tar, updated hash, wrong source: hash-only reuse would incorrectly pass.
 fault(()=>{
  fs.writeFileSync(moduleFile,'wrong archived content');
  const packed=spawnSync('tar',['-czf',archive,'-C',modules,'.']);assert.equal(packed.status,0);
  fs.writeFileSync(moduleFile,'original source');
  fs.writeFileSync(manifest,JSON.stringify({...good,npm_archive_sha256:hash(archive)}));
 },()=>{fs.writeFileSync(archive,bytes);fs.writeFileSync(manifest,JSON.stringify(good))});
 assert.equal(hash(archive),good.npm_archive_sha256);complete=true;
 console.log('PASS initial archive/readback, explicit reuse without copy, six faults with healthy controls and recovery; regular/symbolic/hard links');
}finally{
 if(previous===undefined)delete process.env.SWARM_VERIFICATION_NPM_BASELINE;else process.env.SWARM_VERIFICATION_NPM_BASELINE=previous;
 if(complete)fs.rmSync(root,{recursive:true});else console.error('Retained failed fixture:',root);
}
