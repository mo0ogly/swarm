import {build} from 'esbuild';
import {readFile,writeFile,mkdir,copyFile} from 'node:fs/promises';
const out='../web/lib/terminal';await mkdir(out,{recursive:true});
let patched=0;
const nonce={name:'terminal-style-nonce',setup(b){b.onLoad({filter:/\/xterm\.mjs$/},async({path})=>{
 let contents=await readFile(path,'utf8');
 contents=contents.replace(/(this\._(?:styleElement|dimensionsStyleElement|themeStyleElement))=([\w.]+)\.createElement\("style"\)/g,(match,target)=>{patched++;return match+','+target+'.nonce=document.querySelector("meta[name=swarm-style-nonce]")?.content||""'});
 return{contents,loader:'js'};
});}};
await build({entryPoints:['terminal.js'],bundle:true,format:'esm',outfile:out+'/terminal.js',minify:true,plugins:[nonce],legalComments:'linked'});
if(patched!==3)throw Error('xterm nonce patch drift: '+patched);
// Upstream geometry/selection stylesheet; color roles mapped to project tokens.
let css=await readFile('node_modules/@xterm/xterm/css/xterm.css','utf8');
css=css.replace(/#000(?:000)?\b/gi,'var(--wattson-champ)').replace(/#fff(?:fff)?\b/gi,'var(--wattson-texte)').replace(/rgba\(0,0,0,0\)/g,'transparent');
await writeFile(out+'/xterm.css',css);
await copyFile('node_modules/@xterm/xterm/LICENSE',out+'/LICENSE-xterm.txt');
await copyFile('node_modules/@xterm/addon-fit/LICENSE',out+'/LICENSE-fit.txt');
