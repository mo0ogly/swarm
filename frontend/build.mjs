import {build} from 'esbuild';
import {readFile,writeFile,mkdir,copyFile,rm} from 'node:fs/promises';
const out='../web/lib/monaco';
await rm(out,{recursive:true,force:true});
await mkdir(out,{recursive:true});
// Version-pinned compatibility patch: nonce only on the two upstream stylesheet
// factories. Fail on drift instead of relaxing script/style CSP for the cockpit.
let patched=0;
const nonce={name:'swarm-style-nonce',setup(b){b.onLoad({filter:/\/(domStylesheets|contextview)\.js$/},async({path})=>{
 let contents=await readFile(path,'utf8');
 const source="const style = document.createElement('style');";
 if(!contents.includes(source))throw new Error('Monaco nonce patch needs review: '+path);
 contents=contents.replace(source,source+"\nstyle.nonce = document.querySelector('meta[name=swarm-style-nonce]')?.content || ''; ");patched++;
 return{contents,loader:'js'};
});}};
await build({entryPoints:{editor:'monaco.js',worker:'node_modules/monaco-editor/esm/vs/editor/editor.worker.js','json-worker':'node_modules/monaco-editor/esm/vs/language/json/json.worker.js'},bundle:true,format:'esm',splitting:true,outdir:out,minify:true,loader:{'.ttf':'file'},plugins:[nonce],metafile:true}).then(async r=>{await writeFile('bundle-meta.json',JSON.stringify(r.metafile,null,2)+'\n')});
if(patched!==2)throw new Error('Expected two nonce patches, got '+patched);
await copyFile('node_modules/monaco-editor/LICENSE',out+'/LICENSE.txt').catch(()=>copyFile('node_modules/monaco-editor/LICENSE.txt',out+'/LICENSE.txt'));
await copyFile('../../../static/js/security.js','../web/lib/preparation-security.js');

await copyFile('node_modules/monaco-editor/ThirdPartyNotices.txt',out+'/ThirdPartyNotices.txt');
