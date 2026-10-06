'use strict';
// The archived B03 recipe remains immutable. New performance runs use the
// operator's public configuration, frozen before the first browser action.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),{execFileSync,spawnSync}=require('node:child_process');
const binary=path.resolve(process.argv[2]||''),output=path.resolve(process.argv[3]||''),root=path.resolve(process.argv[4]||process.cwd());
if(!process.argv[2]||!process.argv[3])throw Error('node tests/graph_performance_ui.cjs BINARY OUTPUT [CONFIG_ROOT]');
fs.mkdirSync(output,{recursive:true});
const config=JSON.parse(execFileSync(binary,['--root',root,'--json','run-limits','performance','show'],{encoding:'utf8'})),policy=config.values;
const recipe=path.join(__dirname,'graph_draft_ui.cjs'),original=fs.readFileSync(recipe,'utf8'),sha=s=>crypto.createHash('sha256').update(s).digest('hex');
const snapshot={schema_version:1,revision:config.revision,values:policy,policy_sha256:sha(JSON.stringify(policy)),recipe_sha256:sha(original),scope:'frozen performance targets; not agent execution budgets'};
fs.writeFileSync(path.join(output,'performance-policy.json'),JSON.stringify(snapshot,null,2)+'\n');
let source=original;
function replace(before,after){if(source.split(before).length!==2)throw Error('Archived recipe changed; inspect adapter before running: '+before);source=source.replace(before,after)}
replace("const assert=require('node:assert/strict');","const assert=require('node:assert/strict');\nconst SWARM_POLICY="+JSON.stringify(snapshot)+';');
replace('const sampleCount=5;','const sampleCount=SWARM_POLICY.values.samples;');
replace('initialP95<=(cards===500?2000:1000)','initialP95<=SWARM_POLICY.values.loads.find(row=>row.cards===cards).render_p95_ms');
replace('keyboardP95<=100','keyboardP95<=SWARM_POLICY.values.keyboard_p95_ms');
replace('assert.ok([50,200,500].includes(cards))','assert.ok(SWARM_POLICY.values.loads.some(row=>row.cards===cards))');
replace('for(const cards of [50,200,500])','for(const {cards} of SWARM_POLICY.values.loads)');
replace("const results={surface:'swarm-product'","const results={performance_policy:SWARM_POLICY,surface:'swarm-product'");
const generated=path.join(output,'configured-graph-recipe.cjs');fs.writeFileSync(generated,source);
const result=spawnSync(process.execPath,[generated,binary,output],{stdio:'inherit',env:{...process.env,PUPPETEER_MODULE:process.env.PUPPETEER_MODULE||require.resolve('puppeteer')}});
if(result.error)throw result.error;process.exitCode=result.status??1;
