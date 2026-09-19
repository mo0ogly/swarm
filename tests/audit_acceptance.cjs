#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {spawnSync} = require('node:child_process');

const root = path.resolve(__dirname, '..');
const args = process.argv.slice(2);
const index = args.indexOf('--case');
if (index < 0 || !args[index + 1]) {
  console.error('usage: node tests/audit_acceptance.cjs --case cursor');
  process.exit(2);
}
const selected = args[index + 1];
if (selected !== 'cursor') {
  console.error(`unknown audit acceptance case: ${selected}`);
  process.exit(2);
}

for (const dependency of ['go.mod', 'cursor_contract_test.go']) {
  if (!fs.existsSync(path.join(root, dependency))) {
    console.error(`cursor acceptance dependency missing: ${dependency}`);
    process.exit(3);
  }
}

const env = {...process.env};
if (!env.TMPDIR) env.TMPDIR = '/dev/shm';
if (!env.GOTMPDIR) env.GOTMPDIR = '/dev/shm';
if (!env.GOCACHE) env.GOCACHE = '/dev/shm/swarm-audit-go-cache';
fs.mkdirSync(env.GOCACHE, {recursive: true});
const result = spawnSync('go', ['test', '-count=1', '-run', '^TestCursorContract', '.'], {
  cwd: root,
  env,
  encoding: 'utf8',
  timeout: 300000,
});
process.stdout.write(result.stdout || '');
process.stderr.write(result.stderr || '');
if (result.error && result.status === null) {
  console.error(`cursor acceptance could not execute: ${result.error.message}`);
  process.exit(4);
}
if (result.status !== 0) process.exit(result.status ?? 1);
if (!/\bok\s+\S+/.test(result.stdout)) {
  console.error('cursor acceptance produced no Go test success record');
  process.exit(5);
}
console.log('cursor acceptance: responsibilities and return flow verified');
