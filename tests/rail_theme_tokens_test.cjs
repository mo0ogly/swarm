'use strict';
/** Static check of web/swarm-design.css: the cockpit rail takes every colour
 * from local --rail-* tokens, and the light "etat" theme redefines them, so the
 * rail follows the selected theme (user decision 10 Nov 2026).
 * Reads one file; no browser, network or write. Does not replace the visual
 * check of the served page. */
const assert = require('node:assert/strict');
const fs = require('node:fs');
const css = fs.readFileSync(__dirname + '/../web/swarm-design.css', 'utf8');
let count = 0;
function check(name, fn) { fn(); count++; console.log('PASS ' + name); }

/** Return the declaration body of the first rule whose selector is exactly `sel`. */
function ruleBody(sel) {
  const at = css.indexOf(sel + ' {');
  assert.notEqual(at, -1, 'missing rule ' + sel);
  return css.slice(css.indexOf('{', at) + 1, css.indexOf('}', at));
}
const tokens = (body) => new Set([...body.matchAll(/(--rail-[a-z-]+)\s*:/g)].map((m) => m[1]));

const dark = ruleBody('.cockpit-page .rail');
const light = ruleBody(':root[data-theme="etat"] .cockpit-page .rail');

check('rail-background-uses-token', () => {
  assert.match(dark, /background:\s*var\(--rail-fond\)/);
  assert.doesNotMatch(dark, /background:\s*#/);
});
check('light-theme-redefines-every-rail-token', () => {
  const missing = [...tokens(dark)].filter((t) => !tokens(light).has(t));
  assert.ok(tokens(dark).size >= 10);
  assert.deepEqual(missing, []);
  assert.match(light, /--rail-fond:\s*#eceae1/);
  assert.match(light, /color-scheme:\s*light/);
});
check('rail-rules-carry-no-literal-marine-colours', () => {
  const literals = [];
  for (const m of css.matchAll(/([^{}]*\.rail[^{}]*)\{([^}]*)\}/g)) {
    const [, sel, body] = m;
    if (sel.includes('data-theme') || /^\s*\.cockpit-page \.rail\s*$/.test(sel)) continue;
    for (const hex of body.match(/#(062338|375568|29485b|c5d8df|92b1c1|12384c|183e4f|adc3ce|628695)\b/gi) || []) literals.push(sel.trim() + ' ' + hex);
  }
  assert.deepEqual(literals, []);
});
console.log(`${count}/3 rail theme checks passed`);
