'use strict';
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict'),{spawnSync}=require('node:child_process');
const digest=file=>crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const launchOptions=Object.freeze({timeout:0,protocolTimeout:0});
function tree(root){
 const values={};root=fs.realpathSync(root);
 function walk(directory){for(const name of fs.readdirSync(directory).sort()){const file=path.join(directory,name),stat=fs.lstatSync(file),key=path.relative(root,file);
  if(stat.isSymbolicLink()){const target=fs.realpathSync(file);assert(target===root||target.startsWith(root+path.sep),'external dependency symlink requires expanded collector');values[key]={symlink:fs.readlinkSync(file)}}
  else if(stat.isDirectory())walk(file);else if(stat.isFile())values[key]={sha256:digest(file)};else throw Error('unsupported dependency input kind');
 }}walk(root);return values;
}
/** Read back each archived source without extraction; preserve verifier diagnostics.
 * Failures refuse collection rather than falling back to an unverified archive. */
function verifyArchive(repo,output,archive,dependencies){
 const diagnostic=fs.openSync(path.join(output,'npm-readback.stderr'),'wx',0o600);let checked,closed;
 try{checked=spawnSync('python3',[path.join(repo,'tools/verification/verify_npm_archive.py'),archive],{input:JSON.stringify(dependencies),stdio:['pipe','ignore',diagnostic],timeout:0})}
 finally{try{fs.closeSync(diagnostic)}catch(error){closed=error}}
 const causes=[checked?.error,checked?.status!==0?Error('npm archive content readback failed'):undefined,closed].filter(Boolean);
 if(causes.length)throw new AggregateError(causes,'npm sources not verified');
}
/** Observe current npm/lock/Node inputs. An explicitly selected PASS baseline may
 * supply identical source bytes after full readback; it supplies no runtime proof.
 * Refuse drift/corruption, retain diagnostics, and never silently recollect. */
function begin(repo,output){
 const dependencies=tree(path.join(repo,'node_modules')),lock=digest(path.join(repo,'package-lock.json')),node=fs.realpathSync(process.execPath);
 assert(!process.env.NODE_OPTIONS,'Node loader/options require explicit effective-input collection');
 assert.equal(process.execArgv.length,0,'Node execution flags require explicit input collection');
 let archive=path.join(output,'npm-sources.tar.gz'),reference;
 const baseline=process.env.SWARM_VERIFICATION_NPM_BASELINE;
 if(baseline){
  const manifest=fs.realpathSync(baseline),manifestHash=digest(manifest),prior=JSON.parse(fs.readFileSync(manifest,'utf8'));
  assert.equal(prior.status,'PASS','npm baseline was not successful');
  assert.deepEqual(prior.dependencies,dependencies,'npm baseline tree differs');
  assert.equal(prior.lock,lock,'npm baseline lock differs');
  archive=prior.npm_source_reference?.archive||path.join(path.dirname(manifest),'npm-sources.tar.gz');
  archive=fs.realpathSync(archive);assert.equal(digest(archive),prior.npm_archive_sha256,'npm baseline archive digest differs');
  verifyArchive(repo,output,archive,dependencies);
  assert.equal(digest(archive),prior.npm_archive_sha256,'npm baseline archive changed during readback');
  assert.equal(digest(manifest),manifestHash,'npm baseline manifest changed');
  reference={manifest,manifest_sha256:manifestHash,archive};
 }else{
 const diagnostic=fs.openSync(path.join(output,'npm-sources.stderr'),'wx',0o600);let archived,closed;
 console.log('Archive actual installed npm inputs');
 try{archived=spawnSync('tar',['-I','gzip -1','-cf',archive,'-C',path.join(repo,'node_modules'),'.'],{stdio:['ignore','ignore',diagnostic]})}finally{try{fs.closeSync(diagnostic)}catch(error){closed=error}}
 const causes=[archived?.error,archived?.status!==0?Error('npm source archive failed'):undefined,closed].filter(Boolean);if(causes.length)throw new AggregateError(causes,'npm archive not verified');
 verifyArchive(repo,output,archive,dependencies);
 }
 assert.deepEqual(tree(path.join(repo,'node_modules')),dependencies,'npm inputs changed while collecting');
 assert.equal(digest(path.join(repo,'package-lock.json')),lock,'npm lock changed while collecting');
 return {dependencies,lock,node,node_sha256:digest(node),node_version:process.version,npm_archive_sha256:digest(archive),npm_source_reference:reference,output};
}
async function recordBrowser(record,browser){
 const pid=browser.process().pid,executable=fs.realpathSync('/proc/'+pid+'/exe');
 record.browser={version:await browser.version(),executable,sha256:digest(executable)};
 record.timeouts={launch_ms:launchOptions.timeout,cdp_protocol_ms:launchOptions.protocolTimeout,page_ms:0,navigation_ms:0};
}
function finish(record,repo){
 assert.deepEqual(tree(path.join(repo,'node_modules')),record.dependencies,'installed npm inputs changed');
 assert.equal(digest(path.join(repo,'package-lock.json')),record.lock,'npm lock changed');assert.equal(digest(record.node),record.node_sha256,'Node executable changed');
 assert.equal(digest(record.browser.executable),record.browser.sha256,'browser executable changed');
 const report={status:'PASS',scope:'Actual installed npm tree and lock, Node/Chrome binaries and explicit verification timeouts; OS/shared libraries are not fully snapshotted',...record};
 fs.writeFileSync(path.join(record.output,'runtime-inputs.json'),JSON.stringify(report,null,2)+'\n');return {node_version:record.node_version,browser:record.browser,timeouts:record.timeouts,npm_files:Object.keys(record.dependencies).length,runtime_inputs_sha256:digest(path.join(record.output,'runtime-inputs.json'))};
}
module.exports={launchOptions,tree,begin,recordBrowser,finish};
