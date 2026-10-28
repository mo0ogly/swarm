// Explicit, fresh output only. Does not replace embedded web assets.
import {readFile,writeFile,mkdir,copyFile,realpath} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {createHash} from 'node:crypto';
const source = path.dirname(fileURLToPath(import.meta.url)), root = path.dirname(source);
if (process.argv.length !== 3) throw Error('Usage: node frontend/build-signals.mjs NEW_OUTPUT_DIRECTORY');
const requested = path.resolve(process.argv[2]);
const out = path.join(await realpath(path.dirname(requested)),path.basename(requested));
const checkout = await realpath(root);
if (out===checkout || out.startsWith(checkout+path.sep) || checkout.startsWith(out+path.sep)) throw Error('Preview output must be outside the checkout');
await mkdir(out,{recursive:false});
const files = ['signals.js','signals.css','catalogue.html','catalogue.js','catalogue.css'];
const manifest = {schema_version:1,kind:'swarm-signals-presentation',entry:'catalogue.html',demonstration:true,authority:false,assets:{}};
for (const name of files) {const from=path.join(source,'signals',name);await copyFile(from,path.join(out,name));manifest.assets[name]=createHash('sha256').update(await readFile(from)).digest('hex');}
for (const name of ['wattson_themes.css','swarm-design.css','swarm-logo.png']) {const from=path.join(root,'web',name);await copyFile(from,path.join(out,name));manifest.assets[name]=createHash('sha256').update(await readFile(from)).digest('hex');}
await writeFile(path.join(out,'build-manifest.json'),JSON.stringify(manifest,null,2)+'\n');
console.log(JSON.stringify({output:out,assets:Object.keys(manifest.assets).length,entry:manifest.entry,authority:false}));
