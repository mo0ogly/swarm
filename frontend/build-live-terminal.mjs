// Bundle the read-only live harness terminal (xterm + fit) into web/lib/terminal.
// Same CSP nonce patch as build-terminal.mjs: xterm's three <style> elements
// read the page nonce from <meta name="swarm-style-nonce">. Shares xterm.css.
import {build} from 'esbuild';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
const here=p=>fileURLToPath(new URL(p,import.meta.url));
let patched=0;
const nonce={name:'terminal-style-nonce',setup(b){b.onLoad({filter:/\/xterm\.mjs$/},async({path})=>{
 let contents=await readFile(path,'utf8');
 contents=contents.replace(/(this\._(?:styleElement|dimensionsStyleElement|themeStyleElement))=([\w.]+)\.createElement\("style"\)/g,(match,target)=>{patched++;return match+','+target+'.nonce=document.querySelector("meta[name=swarm-style-nonce]")?.content||""'});
 return{contents,loader:'js'};
});}};
await build({entryPoints:[here('live-terminal.js')],absWorkingDir:here('.'),bundle:true,format:'iife',outfile:here('../web/lib/terminal/live-terminal.js'),minify:true,plugins:[nonce],legalComments:'linked'});
if(patched!==3)throw Error('xterm nonce patch drift: '+patched);
const out=here('../web/lib/terminal/live-terminal.js');
console.log(JSON.stringify({bundle:'web/lib/terminal/live-terminal.js',sha256:createHash('sha256').update(await readFile(out)).digest('hex'),nonce_patches:patched}));
