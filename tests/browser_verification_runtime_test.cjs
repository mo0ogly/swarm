'use strict';
const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),assert=require('node:assert/strict'),{tree,launchOptions}=require('./helpers/browser_verification_runtime.cjs');
const root=fs.mkdtempSync(path.join(os.tmpdir(),'swarm-runtime-guard-'));fs.chmodSync(root,0o700);let complete=false;
try{
 fs.writeFileSync(path.join(root,'module.cjs'),'valid original',{mode:0o600});const control=tree(root);assert.deepEqual(tree(root),control);
 fs.writeFileSync(path.join(root,'module.cjs'),'different code');assert.notDeepEqual(tree(root),control);fs.writeFileSync(path.join(root,'module.cjs'),'valid original');assert.deepEqual(tree(root),control);
 fs.writeFileSync(path.join(root,'added.cjs'),'new source');assert.notDeepEqual(tree(root),control);fs.unlinkSync(path.join(root,'added.cjs'));assert.deepEqual(tree(root),control);
 fs.renameSync(path.join(root,'module.cjs'),path.join(root,'renamed.cjs'));assert.notDeepEqual(tree(root),control);fs.renameSync(path.join(root,'renamed.cjs'),path.join(root,'module.cjs'));assert.deepEqual(tree(root),control);
 fs.symlinkSync('/dev/zero',path.join(root,'external'));assert.throws(()=>tree(root),/external dependency symlink/);fs.unlinkSync(path.join(root,'external'));assert.deepEqual(tree(root),control);
 assert.deepEqual(launchOptions,{timeout:0,protocolTimeout:0});complete=true;console.log('PASS runtime source control, change/add/rename/external-link refusal and recovery; explicit verification timeout settings');
}finally{if(complete)fs.rmSync(root,{recursive:true})}
