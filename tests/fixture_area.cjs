#!/usr/bin/env node
/**
 * Single canonical implementation of the repository fixture area (CommonJS).
 *
 * Operator interdiction (2026-10-31): project data must never live in `/tmp`.
 * tmpfs dies on reboot and retained evidence would point at a vanished root.
 * Every browser/UI test workspace in this repository is allocated under
 * `<repo>/.swarm-fixtures/` (0700, gitignored) instead, mirroring the Python
 * contract in `tests/fixture_area.py`.
 *
 * Usage in a test script:
 *   const fixtureArea = require('./fixture_area.cjs');
 *   const root = fixtureArea.fixtureRoot('swarm-skills-ui-');
 *
 * `fixtureRoot(prefix)` mirrors `fs.mkdtempSync(path.join(os.tmpdir(), prefix))`:
 * callers pass the exact same prefix they used before, so root-name assertions
 * keep holding, and get a durable absolute path string back. Path length stays
 * short enough for AF_UNIX sockets inside the root (kernel limit 108 bytes).
 * The caller owns retention and cleanup: this area holds fixture evidence,
 * not a disposable cache; regeneration follows the documented recipe path
 * (tools/verification/README.md).
 */
'use strict';
const fs = require('fs');
const os = require('os');
const path = require('path');

const REPO = path.resolve(__dirname, '..');

/** Resolve the fixture area from the Admin setting `runtime_temporary_directory`:
 * the value saved in `.swarm/engine-settings.json`, else the shipped installation
 * default in `rust/crates/swarm-engine/config/engine-settings.json`. Relative
 * values are relative to the repository. Throws on `/tmp` (wiped at reboot),
 * empty or traversal values. Read-only. Mirrors tests/fixture_area.py. */
function configuredArea() {
  const saved = path.join(REPO, '.swarm', 'engine-settings.json');
  let value;
  if (fs.existsSync(saved)) value = JSON.parse(fs.readFileSync(saved, 'utf8')).values?.runtime_temporary_directory;
  if (value === undefined) value = JSON.parse(fs.readFileSync(path.join(REPO, 'rust/crates/swarm-engine/config/engine-settings.json'), 'utf8')).runtime_temporary_directory;
  const parts = typeof value === 'string' ? value.replace(/^\/+|\/+$/g, '').split('/') : [];
  if (typeof value !== 'string' || !value || value.includes('\0') || value.replace(/\/+$/, '') === '/tmp' || value === '/' || value.startsWith('/tmp/') || parts.some(c => c === '' || c === '.' || c === '..'))
    throw Error('runtime_temporary_directory invalid for fixtures: ' + JSON.stringify(value));
  return path.isAbsolute(value) ? value : path.join(REPO, value);
}
const AREA = configuredArea();

function fixtureArea() {
  /** Return the repository fixture area directory, creating it 0700 if absent.
   *
   * Created on first use, never at require time (no artifact allocation on
   * require). Errors propagate: a read-only checkout cannot host fixtures.
   */
  fs.mkdirSync(AREA, { recursive: true, mode: 0o700 });  // parents too
  return AREA;
}

function fixtureRoot(prefix) {
  /** Create a private durable fixture root under the repository fixture area.
   *
   * Purpose: replace `fs.mkdtempSync(path.join(os.tmpdir(), prefix))` calls
   * that would place project data in `/tmp`. Inputs: `prefix` (same
   * convention as mkdtempSync, usually `swarm-...-`). Output: absolute path
   * string of a new directory named `<prefix><random>` (6 random chars, like
   * mkdtempSync). Errors: fs errors propagate. Side effects: creates the area
   * (0700) and the root. Limits: caller-owned retention; fixture evidence,
   * not a disposable cache.
   */
  fs.mkdirSync(AREA, { recursive: true, mode: 0o700 });  // parents too
  try {
    fs.chmodSync(AREA, 0o700);
  } catch (_err) {
    // best effort: an area created by another owner stays usable read-only
  }
  return fs.mkdtempSync(path.join(AREA, prefix));
}

module.exports = { fixtureArea, fixtureRoot, AREA, home: os.homedir() };
