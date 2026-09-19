#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
const index = args.indexOf('--case');
if (index < 0 || !args[index + 1]) {
  console.error('usage: node tests/audit_acceptance.cjs --case cursor|states|evidence');
  process.exit(2);
}
const selected = args[index + 1];
if (!['cursor', 'states', 'evidence'].includes(selected)) {
  console.error(`unknown audit acceptance case: ${selected}`);
  process.exit(2);
}

const dependencies=selected==='cursor'?['go.mod','cursor_contract_test.go']:
 selected==='states'?['go.mod','states_contract_test.go','tests/states_contract_test.cjs','web/status-contract.js']:
 ['go.mod','evidence_contract_test.go','tests/evidence_contract_test.cjs','web/evidence-contract.js','evidence_projection.go'];
for (const dependency of dependencies) {
  if (!fs.existsSync(path.join(root, dependency))) {
    console.error(`${selected} acceptance dependency missing: ${dependency}`);
    process.exit(3);
  }
}

const env = {...process.env};
if (!env.TMPDIR) env.TMPDIR = '/dev/shm';
if (!env.GOTMPDIR) env.GOTMPDIR = '/dev/shm';
if (!env.GOCACHE) env.GOCACHE = '/dev/shm/swarm-audit-go-cache';
fs.mkdirSync(env.GOCACHE, {recursive: true});
if(selected==='states'){
 // Run the Node assertions in-process: some supported sandboxes deny child
 // creation from Node. A thrown assertion still makes this case fail.
 require(path.join(root,'tests/states_contract_test.cjs'));
 console.log('states acceptance: DOM contract verified; run TestStateContract for the Go/JSON contract');
 process.exit(0);
}
if(selected==='evidence'){
 // DOM assertions fail before the Go contract: a missing/unknown presentation
 // can never be replaced by the mere existence of a report.
 require(path.join(root,'tests/evidence_contract_test.cjs'));
}
const pattern=selected==='cursor'?'^TestCursorContract':selected==='evidence'?'^TestEvidenceContract':'^TestStateContract';
const result = spawnSync('go', ['test', '-count=1', '-run', pattern, '.'], {
  cwd: root,
  env,
  encoding: 'utf8',
  timeout: 300000,
});
process.stdout.write(result.stdout || '');
process.stderr.write(result.stderr || '');
if (result.error && result.status === null) {
  console.error(`${selected} acceptance could not execute: ${result.error.message}`);
  process.exit(4);
}
if (result.status !== 0) process.exit(result.status ?? 1);
if (!/\bok\s+\S+/.test(result.stdout)) {
  console.error(`${selected} acceptance produced no Go test success record`);
  process.exit(5);
}
console.log(selected==='cursor'?'cursor acceptance: responsibilities and return flow verified':selected==='evidence'?'evidence acceptance: report review, executed controls, acceptance, freshness and CLI/web parity verified':'states acceptance: DOM and JSON contracts verified');
