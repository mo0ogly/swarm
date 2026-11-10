// Generated web copies: edit frontend/signals, then run this script before building Go.
import {readFile,writeFile} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
import {createHash} from 'node:crypto';
const root = path.dirname(path.dirname(fileURLToPath(import.meta.url)));
const args = process.argv.slice(2);
if (args.length && (args.length !== 1 || args[0] !== '--check')) throw Error('Usage: node frontend/embed-signals.mjs [--check]');
const check = args[0] === '--check', assets = {};
for (const name of ['signals.js','signals.css']) {
 const source = await readFile(path.join(root,'frontend','signals',name));
 const destination = path.join(root,'web',name);
 if (check) {
  const copy = await readFile(destination);
  if (!source.equals(copy)) throw Error('Stale generated web/'+name+': run node frontend/embed-signals.mjs');
 } else await writeFile(destination,source);
 assets[name] = createHash('sha256').update(source).digest('hex');
}
console.log(JSON.stringify({mode:check?'check':'generate',assets}));
